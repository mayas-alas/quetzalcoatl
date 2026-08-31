use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{data_root, default_config_path};
use crate::error::GnxError;
use crate::host::{InstallOptions, InstallOutcome, UninstallOutcome};
use crate::process::CommandSpec;
use crate::state::{OperationalState, Stage, default_state_path};

const DEFAULT_CONFIG: &str = include_str!("../../config.example.toml");
const HOST_SERVICE: &str = include_str!("../../runtime/gnx-host.service");
const INSTALLED_BINARY: &str = "/usr/local/bin/gnx";
const SERVICE_PATH: &str = "/etc/systemd/system/gnx-host.service";

pub fn install(options: InstallOptions) -> Result<InstallOutcome, GnxError> {
    if !is_root()? {
        if options.elevated {
            return Err(GnxError::new(
                "HOST_ELEVATION_REQUIRED",
                "host",
                "install",
                "La instalación Linux requiere root.",
                "Acepte sudo y vuelva a intentar.",
                false,
                9,
            ));
        }
        return relaunch_with_sudo(&[
            "__install",
            "--elevated",
            if options.resume { "--resume" } else { "" },
        ]);
    }

    let mut state = OperationalState {
        stage: Stage::Installing,
        ..OperationalState::default()
    };
    state.save(&default_state_path())?;

    install_packages()?;
    install_binary()?;
    install_default_config()?;
    install_host_service()?;

    fs::create_dir_all(data_root())
        .map_err(|error| GnxError::io("linux_install", error.to_string()))?;
    state.stage = Stage::Installed;
    state.save(&default_state_path())?;
    Ok(InstallOutcome::Installed)
}

pub fn uninstall(elevated: bool) -> Result<UninstallOutcome, GnxError> {
    if !is_root()? {
        if elevated {
            return Err(GnxError::new(
                "HOST_ELEVATION_REQUIRED",
                "host",
                "uninstall",
                "La desinstalación Linux requiere root.",
                "Acepte sudo y vuelva a intentar.",
                false,
                9,
            ));
        }
        relaunch_with_sudo(&["uninstall", "--elevated"])?;
        return Ok(UninstallOutcome::RelaunchedElevated);
    }

    let _ = CommandSpec::new("systemctl")
        .args(["disable", "--now", "gnx-host.service"])
        .timeout(Duration::from_secs(300))
        .run("linux_service_disable");
    remove_if_exists(Path::new(SERVICE_PATH))?;
    CommandSpec::new("systemctl")
        .arg("daemon-reload")
        .run_checked("linux_daemon_reload")?;
    remove_podman_package()?;
    remove_if_exists(Path::new(INSTALLED_BINARY))?;
    OperationalState {
        stage: Stage::Uninstalled,
        ..OperationalState::default()
    }
    .save(&default_state_path())?;
    Ok(UninstallOutcome::Removed)
}

pub fn running_from_installed_path() -> bool {
    let source = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok());
    source.is_some_and(|path| path == Path::new(INSTALLED_BINARY))
}

pub fn start_service() -> Result<(), GnxError> {
    let command = if is_root()? {
        CommandSpec::new("systemctl")
    } else {
        CommandSpec::new("sudo").arg("systemctl")
    };
    command
        .args(["restart", "--no-block", "gnx-host.service"])
        .timeout(Duration::from_secs(60))
        .run_checked("linux_service_start")?;
    Ok(())
}

pub fn run_service() -> Result<(), GnxError> {
    let mut state = OperationalState {
        stage: Stage::Working,
        ..OperationalState::default()
    };
    state.save(&default_state_path())?;
    let convergence = crate::config::Config::load(&default_config_path())
        .and_then(|config| config.validate())
        .and_then(|controller| {
            crate::runtime::headscale::verify_controller(&controller)?;
            Ok(controller)
        })
        .and_then(|controller| crate::runtime::machine::ensure(&controller));
    match convergence {
        Ok(()) => {
            state.stage = Stage::Installed;
            state.machine = "ready".to_string();
            state.docktail = "deployed".to_string();
            state.proxmox = "ready".to_string();
            state.infra = "applied".to_string();
            state.last_error = None;
            state.save(&default_state_path())?;
            Ok(())
        }
        Err(error) => {
            state.stage = Stage::Failed;
            state.machine = "failed".to_string();
            state.last_error = Some(error.code.to_string());
            state.save(&default_state_path())?;
            Err(error)
        }
    }
}

fn is_root() -> Result<bool, GnxError> {
    let output = CommandSpec::new("id").arg("-u").run("linux_uid")?;
    Ok(output.success() && output.stdout.trim() == "0")
}

fn relaunch_with_sudo(arguments: &[&str]) -> Result<InstallOutcome, GnxError> {
    let executable = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| std::env::current_exe())
        .map_err(|error| GnxError::io("linux_elevate", error.to_string()))?;
    let arguments = arguments.iter().copied().filter(|value| !value.is_empty());
    CommandSpec::new("sudo")
        .arg(executable)
        .args(arguments)
        .timeout(Duration::from_secs(2700))
        .run_checked("linux_elevate")?;
    Ok(InstallOutcome::RelaunchedElevated)
}

fn install_packages() -> Result<(), GnxError> {
    if command_succeeds("podman", &["--version"])
        && (command_succeeds("qemu-system-x86_64", &["--version"])
            || command_succeeds("qemu-system-x86_64", &["-version"]))
    {
        return Ok(());
    }

    if Path::new("/usr/bin/apt-get").exists() {
        CommandSpec::new("apt-get")
            .arg("update")
            .timeout(Duration::from_secs(1800))
            .run_checked("linux_packages_update")?;
        CommandSpec::new("apt-get")
            .args([
                "install",
                "-y",
                "podman",
                "qemu-system-x86",
                "qemu-utils",
                "fuse3",
            ])
            .timeout(Duration::from_secs(1800))
            .run_checked("linux_packages_install")?;
    } else if Path::new("/usr/bin/dnf").exists() {
        CommandSpec::new("dnf")
            .args(["install", "-y", "podman", "qemu-kvm", "fuse3"])
            .timeout(Duration::from_secs(1800))
            .run_checked("linux_packages_install")?;
    } else if Path::new("/usr/bin/pacman").exists() {
        CommandSpec::new("pacman")
            .args([
                "-Sy",
                "--needed",
                "--noconfirm",
                "podman",
                "qemu-desktop",
                "fuse3",
            ])
            .timeout(Duration::from_secs(1800))
            .run_checked("linux_packages_install")?;
    } else {
        return Err(GnxError::unsupported_host(
            "No se encontró apt-get, dnf o pacman para instalar Podman/QEMU.",
        ));
    }
    Ok(())
}

fn remove_podman_package() -> Result<(), GnxError> {
    if Path::new("/usr/bin/apt-get").exists() {
        CommandSpec::new("apt-get")
            .args(["remove", "-y", "podman"])
            .timeout(Duration::from_secs(1800))
            .run_checked("linux_podman_remove")?;
    } else if Path::new("/usr/bin/dnf").exists() {
        CommandSpec::new("dnf")
            .args(["remove", "-y", "podman"])
            .timeout(Duration::from_secs(1800))
            .run_checked("linux_podman_remove")?;
    } else if Path::new("/usr/bin/pacman").exists() {
        CommandSpec::new("pacman")
            .args(["-R", "--noconfirm", "podman"])
            .timeout(Duration::from_secs(1800))
            .run_checked("linux_podman_remove")?;
    } else if command_succeeds("podman", &["--version"]) {
        return Err(GnxError::unsupported_host(
            "No se encontró el gestor de paquetes que instaló Podman.",
        ));
    }
    Ok(())
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    CommandSpec::new(program)
        .args(args)
        .timeout(Duration::from_secs(20))
        .run("linux_preflight")
        .is_ok_and(|output| output.success())
}

fn install_binary() -> Result<(), GnxError> {
    let source = std::env::var_os("APPIMAGE").map(PathBuf::from).unwrap_or(
        std::env::current_exe()
            .map_err(|error| GnxError::io("linux_binary_install", error.to_string()))?,
    );
    let destination = Path::new(INSTALLED_BINARY);
    if source != destination {
        fs::copy(&source, destination)
            .map_err(|error| GnxError::io("linux_binary_install", error.to_string()))?;
    }
    let mut permissions = fs::metadata(destination)
        .map_err(|error| GnxError::io("linux_binary_install", error.to_string()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(destination, permissions)
        .map_err(|error| GnxError::io("linux_binary_install", error.to_string()))?;
    Ok(())
}

fn install_default_config() -> Result<(), GnxError> {
    let path = default_config_path();
    if path.exists() {
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| GnxError::io("linux_config_install", "Ruta de config inválida"))?;
    fs::create_dir_all(parent)
        .map_err(|error| GnxError::io("linux_config_install", error.to_string()))?;
    fs::write(path, DEFAULT_CONFIG)
        .map_err(|error| GnxError::io("linux_config_install", error.to_string()))
}

fn install_host_service() -> Result<(), GnxError> {
    fs::write(SERVICE_PATH, HOST_SERVICE)
        .map_err(|error| GnxError::io("linux_service_install", error.to_string()))?;
    CommandSpec::new("systemctl")
        .arg("daemon-reload")
        .run_checked("linux_daemon_reload")?;
    CommandSpec::new("systemctl")
        .args(["enable", "gnx-host.service"])
        .run_checked("linux_service_enable")?;
    CommandSpec::new("systemctl")
        .args(["start", "--no-block", "gnx-host.service"])
        .run_checked("linux_service_start")?;
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<(), GnxError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(GnxError::io("linux_remove", error.to_string())),
    }
}
