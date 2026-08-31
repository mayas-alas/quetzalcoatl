use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::Duration;

use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW};
use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::config::{data_root, default_config_path};
use crate::error::GnxError;
use crate::host::windows::{account, download, reboot, service, wsl};
use crate::host::{InstallOptions, InstallOutcome, UninstallOutcome};
use crate::journal::{InstallCheckpoint, OperationJournal, default_journal_path};
use crate::process::CommandSpec;
use crate::state::{OperationalState, Stage, default_state_path};

const DEFAULT_CONFIG: &str = include_str!("../../../config.example.toml");
const REGISTRY_ENVIRONMENT: &str =
    r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

pub fn install(options: InstallOptions) -> Result<InstallOutcome, GnxError> {
    if !is_elevated() {
        if options.elevated {
            return Err(GnxError::new(
                "HOST_ELEVATION_REQUIRED",
                "host",
                "install",
                "La instalación no recibió un token elevado.",
                "Acepte el diálogo UAC y vuelva a intentar.",
                false,
                9,
            ));
        }
        let parameters = if options.resume {
            "__install --elevated --resume"
        } else {
            "__install --elevated"
        };
        elevate(parameters, "instalar WSL, Podman y el servicio")?;
        return Ok(InstallOutcome::RelaunchedElevated);
    }

    let journal_path = default_journal_path();
    let mut journal = match OperationJournal::load(&journal_path)? {
        Some(journal)
            if journal.target_version != env!("CARGO_PKG_VERSION")
                && journal.checkpoint != InstallCheckpoint::Completed =>
        {
            return Err(GnxError::new(
                "INSTALL_VERSION_CONFLICT",
                "install",
                "resume",
                format!(
                    "Journal {} pertenece a GNX {}.",
                    journal.operation_id, journal.target_version
                ),
                "Reanude con el mismo binario antes de activar otro release.",
                false,
                15,
            ));
        }
        Some(journal) if journal.target_version == env!("CARGO_PKG_VERSION") => journal,
        Some(_) | None => OperationJournal::new_install(),
    };
    advance(&mut journal, InstallCheckpoint::Elevated, &journal_path)?;

    let installed_executable = install_files()?;
    advance(
        &mut journal,
        InstallCheckpoint::FilesInstalled,
        &journal_path,
    )?;

    let wsl_outcome = wsl::ensure()?;
    advance(&mut journal, InstallCheckpoint::WslEnabled, &journal_path)?;

    let podman_reboot = install_podman()?;
    advance(
        &mut journal,
        InstallCheckpoint::PodmanInstalled,
        &journal_path,
    )?;

    if wsl_outcome == wsl::WslOutcome::RebootRequired || podman_reboot {
        reboot::register_resume(&installed_executable)?;
        journal.reboot_required = true;
        journal.save(&journal_path)?;
        OperationalState {
            stage: Stage::RebootRequired,
            ..OperationalState::default()
        }
        .save(&default_state_path())?;
        println!("GNX instaló sus archivos y Podman. Windows debe reiniciarse para continuar.");
        return Ok(InstallOutcome::RebootRequired);
    }

    service::register(&installed_executable)?;
    account::grant_data_access(&data_root())?;
    advance(
        &mut journal,
        InstallCheckpoint::ServiceRegistered,
        &journal_path,
    )?;
    service::start()?;
    advance(
        &mut journal,
        InstallCheckpoint::MachineRequested,
        &journal_path,
    )?;
    journal.reboot_required = false;
    advance(&mut journal, InstallCheckpoint::Completed, &journal_path)?;
    reboot::unregister_resume()?;

    OperationalState {
        stage: Stage::Installed,
        ..OperationalState::default()
    }
    .save(&default_state_path())?;
    println!("GNX quedó en PATH. Abra una shell nueva y ejecute: gnx status");
    Ok(InstallOutcome::Installed)
}

pub fn uninstall(elevated: bool) -> Result<UninstallOutcome, GnxError> {
    if !is_elevated() {
        if elevated {
            return Err(GnxError::new(
                "HOST_ELEVATION_REQUIRED",
                "host",
                "uninstall",
                "La desinstalación no recibió un token elevado.",
                "Acepte el diálogo UAC y vuelva a intentar.",
                false,
                9,
            ));
        }
        elevate("uninstall --elevated", "retirar GNX y Podman CLI")?;
        return Ok(UninstallOutcome::RelaunchedElevated);
    }

    service::remove()?;
    let podman_reboot = uninstall_podman()?;
    reboot::unregister_resume()?;
    let install_directory = install_directory();
    remove_from_machine_path(&install_directory)?;
    let binary_reboot = delete_or_schedule(&installed_executable())?;
    let _ = fs::remove_dir(&install_directory);
    OperationalState {
        stage: Stage::Uninstalled,
        ..OperationalState::default()
    }
    .save(&default_state_path())?;

    if podman_reboot || binary_reboot {
        Ok(UninstallOutcome::RebootRequired)
    } else {
        Ok(UninstallOutcome::Removed)
    }
}

pub fn installed_executable() -> PathBuf {
    install_directory().join("gnx.exe")
}

pub fn running_from_installed_path() -> bool {
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    current
        .display()
        .to_string()
        .eq_ignore_ascii_case(&installed_executable().display().to_string())
}

fn install_directory() -> PathBuf {
    std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
        .join("QuetzalcoatlNext")
}

fn advance(
    journal: &mut OperationJournal,
    checkpoint: InstallCheckpoint,
    path: &Path,
) -> Result<(), GnxError> {
    if journal.checkpoint < checkpoint {
        journal.advance(checkpoint)?;
        journal.save(path)?;
    }
    Ok(())
}

fn is_elevated() -> bool {
    // SAFETY: IsUserAnAdmin has no pointer arguments and only inspects the current token.
    unsafe { IsUserAnAdmin() != 0 }
}

fn elevate(parameters: &str, purpose: &str) -> Result<(), GnxError> {
    let executable = std::env::current_exe()
        .map_err(|error| GnxError::io("windows_elevate", error.to_string()))?;
    let verb = wide("runas");
    let file = wide(executable.as_os_str());
    let parameters = wide(parameters);
    // SAFETY: all string buffers are NUL-terminated and remain alive through ShellExecuteW.
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        Err(GnxError::new(
            "HOST_ELEVATION_CANCELLED",
            "host",
            "windows_elevate",
            format!("ShellExecuteW devolvió {result}."),
            format!("Acepte UAC para que GNX pueda {purpose}."),
            true,
            9,
        ))
    } else {
        Ok(())
    }
}

fn install_files() -> Result<PathBuf, GnxError> {
    let install_directory = install_directory();
    let destination = installed_executable();
    fs::create_dir_all(&install_directory)
        .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    let source = std::env::current_exe()
        .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    if !paths_equal(&source, &destination) {
        if destination.exists() {
            service::stop()?;
        }
        fs::copy(&source, &destination)
            .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    }

    fs::create_dir_all(data_root())
        .map_err(|error| GnxError::io("windows_files", error.to_string()))?;
    let config_path = default_config_path();
    if !config_path.exists() {
        fs::write(&config_path, DEFAULT_CONFIG)
            .map_err(|error| GnxError::io("windows_config", error.to_string()))?;
    }
    add_to_machine_path(&install_directory)?;
    Ok(destination)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left.display()
        .to_string()
        .eq_ignore_ascii_case(&right.display().to_string())
}

fn query_machine_path() -> Result<String, GnxError> {
    let query = CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args(["QUERY", REGISTRY_ENVIRONMENT, "/v", "Path"])
        .run_checked("path_query")?;
    query
        .stdout
        .lines()
        .find_map(|line| {
            line.split_once("REG_EXPAND_SZ")
                .map(|(_, value)| value.trim().to_string())
        })
        .or_else(|| {
            query.stdout.lines().find_map(|line| {
                line.split_once("REG_SZ")
                    .map(|(_, value)| value.trim().to_string())
            })
        })
        .ok_or_else(|| GnxError::io("path_query", "No se pudo interpretar PATH de máquina"))
}

fn write_machine_path(value: String) -> Result<(), GnxError> {
    CommandSpec::new(r"C:\Windows\System32\reg.exe")
        .args([
            "ADD",
            REGISTRY_ENVIRONMENT,
            "/v",
            "Path",
            "/t",
            "REG_EXPAND_SZ",
            "/d",
        ])
        .arg(value)
        .arg("/f")
        .run_checked("path_update")?;
    Ok(())
}

fn add_to_machine_path(install_directory: &Path) -> Result<(), GnxError> {
    let current_path = query_machine_path()?;
    if current_path.split(';').any(|entry| {
        entry
            .trim()
            .eq_ignore_ascii_case(&install_directory.display().to_string())
    }) {
        return Ok(());
    }
    write_machine_path(format!("{current_path};{}", install_directory.display()))
}

fn remove_from_machine_path(install_directory: &Path) -> Result<(), GnxError> {
    let current_path = query_machine_path()?;
    let filtered = current_path
        .split(';')
        .filter(|entry| {
            !entry
                .trim()
                .eq_ignore_ascii_case(&install_directory.display().to_string())
        })
        .collect::<Vec<_>>()
        .join(";");
    if filtered != current_path {
        write_machine_path(filtered)?;
    }
    Ok(())
}

fn install_podman() -> Result<bool, GnxError> {
    let podman = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
        .join("Podman")
        .join("podman.exe");
    if CommandSpec::new(&podman)
        .arg("--version")
        .timeout(Duration::from_secs(30))
        .run("podman_version")
        .is_ok_and(|output| output.success())
    {
        return Ok(false);
    }

    let dependency = download::podman_dependency()?;
    println!(
        "Descargando {} {} desde el release oficial…",
        dependency.publisher, dependency.version
    );
    let installer = download::download_verified(&dependency, &data_root().join("cache"))?;
    let log_path = data_root().join("podman-msi.log");
    let output = CommandSpec::new(r"C:\Windows\System32\msiexec.exe")
        .arg("/package")
        .arg(installer)
        .arg("/quiet")
        .arg("/norestart")
        .arg("ALLUSERS=1")
        .arg("MACHINE_PROVIDER=wsl")
        .arg("/l*v")
        .arg(log_path)
        .timeout(Duration::from_secs(1800))
        .run("podman_install")?;
    match output.exit_code {
        Some(0) => Ok(false),
        Some(1641 | 3010) => Ok(true),
        _ => Err(GnxError::process(
            "podman_install",
            Path::new(r"C:\Windows\System32\msiexec.exe"),
            output.stderr,
            true,
        )),
    }
}

fn uninstall_podman() -> Result<bool, GnxError> {
    let dependency = download::podman_dependency()?;
    let installer = download::download_verified(&dependency, &data_root().join("cache"))?;
    let log_path = data_root().join("podman-msi-uninstall.log");
    let output = CommandSpec::new(r"C:\Windows\System32\msiexec.exe")
        .arg("/uninstall")
        .arg(installer)
        .arg("/quiet")
        .arg("/norestart")
        .arg("/l*v")
        .arg(log_path)
        .timeout(Duration::from_secs(1800))
        .run("podman_uninstall")?;
    match output.exit_code {
        Some(0 | 1605) => Ok(false),
        Some(1641 | 3010) => Ok(true),
        _ => Err(GnxError::process(
            "podman_uninstall",
            Path::new(r"C:\Windows\System32\msiexec.exe"),
            output.stderr,
            true,
        )),
    }
}

fn delete_or_schedule(path: &Path) -> Result<bool, GnxError> {
    match fs::remove_file(path) {
        Ok(()) => return Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => {}
    }
    let encoded = wide(path.as_os_str());
    // SAFETY: path is a NUL-terminated UTF-16 buffer and NULL destination requests deletion.
    let result = unsafe { MoveFileExW(encoded.as_ptr(), ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) };
    if result == 0 {
        Err(GnxError::io(
            "windows_binary_remove",
            std::io::Error::last_os_error().to_string(),
        ))
    } else {
        Ok(true)
    }
}

fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
