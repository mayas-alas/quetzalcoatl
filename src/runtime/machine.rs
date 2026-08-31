use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::{ControllerUrl, MACHINE_NAME};
use crate::error::GnxError;
use crate::process::CommandSpec;

const REMOTE_TOFU_ARCHIVE: &str = "/var/tmp/gnx-opentofu.tar.gz";
const PROXMOX_ENV: &str = "/etc/gnx/proxmox.env";
const OPENTOFU_ENV: &str = "/etc/gnx/opentofu.env";

pub fn ensure(controller: &ControllerUrl) -> Result<(), GnxError> {
    let podman = podman_executable();
    let inspect = CommandSpec::new(&podman)
        .args(["machine", "inspect", MACHINE_NAME])
        .timeout(Duration::from_secs(60))
        .run("machine_inspect")?;
    if !inspect.success() {
        let provider = if cfg!(target_os = "windows") {
            "wsl"
        } else {
            "qemu"
        };
        CommandSpec::new(&podman)
            .args([
                "machine",
                "init",
                "--provider",
                provider,
                "--cpus",
                "4",
                "--memory",
                "8192",
                "--disk-size",
                "100",
                "--rootful",
                MACHINE_NAME,
            ])
            .timeout(Duration::from_secs(1800))
            .run_checked("machine_init")?;
    }

    let start = CommandSpec::new(&podman)
        .args(["machine", "start", MACHINE_NAME])
        .timeout(Duration::from_secs(600))
        .run("machine_start")?;
    if !start.success()
        && !start
            .stderr
            .to_ascii_lowercase()
            .contains("already running")
    {
        return Err(GnxError::process(
            "machine_start",
            &podman,
            start.stderr,
            true,
        ));
    }
    CommandSpec::new(&podman)
        .args(["info", "--format", "json"])
        .timeout(Duration::from_secs(60))
        .run_checked("machine_health")?;
    deploy_runtime(&podman, controller)?;
    Ok(())
}

fn deploy_runtime(podman: &Path, controller: &ControllerUrl) -> Result<(), GnxError> {
    remote_checked(
        podman,
        &[
            "sudo",
            "install",
            "-d",
            "-m",
            "0755",
            "/etc/containers/systemd",
            "/etc/systemd/system",
            "/opt/gnx/infra",
            "/opt/gnx/guest/units",
            "/var/lib/gnx/opentofu",
            "/var/lib/gnx/proxmox/data",
            "/var/lib/gnx/proxmox/config",
            "/var/lib/gnx/tailscale",
            "/run/gnx/tailscale",
        ],
        "runtime_directories",
        Duration::from_secs(60),
    )?;
    remote_checked(
        podman,
        &["sudo", "install", "-d", "-m", "0700", "/etc/gnx"],
        "runtime_secret_directory",
        Duration::from_secs(30),
    )?;

    ensure_opentofu(podman)?;
    ensure_proxmox_environment(podman)?;
    ensure_tailscale_environment(podman, controller)?;

    for (content, destination, mode) in [
        (
            crate::runtime::tailscale::QUADLET,
            "/etc/containers/systemd/tailscale.container",
            "0644",
        ),
        (
            crate::runtime::docktail::QUADLET,
            "/etc/containers/systemd/docktail.container",
            "0644",
        ),
        (
            crate::runtime::proxmox::QUADLET,
            "/etc/containers/systemd/proxmox.container",
            "0644",
        ),
        (
            crate::runtime::proxmox::NETWORK,
            "/etc/containers/systemd/gnx-runtime.network",
            "0644",
        ),
        (
            crate::runtime::opentofu::SYSTEMD_UNIT,
            "/etc/systemd/system/gnx-opentofu.service",
            "0644",
        ),
        (
            crate::runtime::opentofu::VERSIONS_TF,
            "/opt/gnx/infra/versions.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::VARIABLES_TF,
            "/opt/gnx/infra/variables.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::MAIN_TF,
            "/opt/gnx/infra/main.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::OUTPUTS_TF,
            "/opt/gnx/infra/outputs.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::PROVIDER_LOCK,
            "/opt/gnx/infra/.terraform.lock.hcl",
            "0644",
        ),
        (
            include_str!("../../guest/bootstrap.sh"),
            "/opt/gnx/guest/bootstrap.sh",
            "0755",
        ),
        (
            include_str!("../../guest/units/tailscale.container"),
            "/opt/gnx/guest/units/tailscale.container",
            "0644",
        ),
        (
            include_str!("../../guest/units/docktail.container"),
            "/opt/gnx/guest/units/docktail.container",
            "0644",
        ),
        (
            &tailscale_environment(controller, "gnx-cell-01"),
            "/opt/gnx/guest/tailscale-controller.env",
            "0600",
        ),
    ] {
        install_text(podman, content, destination, mode)?;
    }

    remote_checked(
        podman,
        &["sudo", "systemctl", "daemon-reload"],
        "runtime_daemon_reload",
        Duration::from_secs(60),
    )?;
    remote_checked(
        podman,
        &[
            "sudo",
            "systemctl",
            "enable",
            "--now",
            "podman.socket",
            "tailscale.service",
            "docktail.service",
            "proxmox.service",
            "gnx-opentofu.service",
        ],
        "runtime_enable",
        Duration::from_secs(2400),
    )?;
    Ok(())
}

fn ensure_tailscale_environment(podman: &Path, controller: &ControllerUrl) -> Result<(), GnxError> {
    install_text(
        podman,
        &tailscale_environment(controller, "gnx-runtime"),
        "/etc/gnx/tailscale-controller.env",
        "0600",
    )?;
    let auth = remote(
        podman,
        &["sudo", "test", "-e", "/etc/gnx/tailscale-auth.env"],
        "tailscale_auth_check",
        Duration::from_secs(30),
    )?;
    if !auth.success() {
        install_text(podman, "", "/etc/gnx/tailscale-auth.env", "0600")?;
    }
    Ok(())
}

fn tailscale_environment(controller: &ControllerUrl, hostname: &str) -> String {
    format!(
        "TS_HOSTNAME={hostname}\nTS_EXTRA_ARGS=--login-server={} --accept-dns=true\n",
        controller.canonical()
    )
}

fn ensure_opentofu(podman: &Path) -> Result<(), GnxError> {
    let dependency = crate::runtime::opentofu::dependency()?;
    let version = remote(
        podman,
        &["/usr/local/bin/tofu", "version"],
        "opentofu_version",
        Duration::from_secs(30),
    );
    if version.is_ok_and(|output| output.success() && output.stdout.contains(&dependency.version)) {
        return Ok(());
    }

    let (_, archive) = crate::runtime::opentofu::download()?;
    install_blob(podman, &archive, REMOTE_TOFU_ARCHIVE, "0600")?;
    remote_checked(
        podman,
        &[
            "sudo",
            "tar",
            "-xzf",
            REMOTE_TOFU_ARCHIVE,
            "-C",
            "/usr/local/bin",
            "tofu",
        ],
        "opentofu_extract",
        Duration::from_secs(120),
    )?;
    remote_checked(
        podman,
        &["sudo", "chmod", "0755", "/usr/local/bin/tofu"],
        "opentofu_permissions",
        Duration::from_secs(30),
    )?;
    remote_checked(
        podman,
        &["sudo", "rm", "-f", REMOTE_TOFU_ARCHIVE],
        "opentofu_cleanup",
        Duration::from_secs(30),
    )?;
    let installed = remote_checked(
        podman,
        &["/usr/local/bin/tofu", "version"],
        "opentofu_verify",
        Duration::from_secs(30),
    )?;
    if !installed.stdout.contains(&dependency.version) {
        return Err(GnxError::new(
            "OPENTOFU_VERSION_MISMATCH",
            "opentofu",
            "install",
            installed.stdout,
            "Conserve el cache verificado y ejecute gnx repair.",
            true,
            18,
        ));
    }
    Ok(())
}

fn ensure_proxmox_environment(podman: &Path) -> Result<(), GnxError> {
    let proxmox_existing = remote(
        podman,
        &["sudo", "test", "-s", PROXMOX_ENV],
        "proxmox_secret_check",
        Duration::from_secs(30),
    )?;
    let opentofu_existing = remote(
        podman,
        &["sudo", "test", "-s", OPENTOFU_ENV],
        "opentofu_secret_check",
        Duration::from_secs(30),
    )?;
    if proxmox_existing.success() && opentofu_existing.success() {
        return Ok(());
    }
    let proxmox_password = crate::secrets::random_hex(32)?;
    let guest_password = crate::secrets::random_hex(32)?;
    let opentofu = format!(
        concat!(
            "PROXMOX_VE_ENDPOINT=https://127.0.0.1:8006/\n",
            "PROXMOX_VE_USERNAME=root@pam\n",
            "PROXMOX_VE_PASSWORD={}\n",
            "PROXMOX_VE_INSECURE=true\n",
            "TF_VAR_guest_password={}\n"
        ),
        proxmox_password, guest_password
    );
    install_text(
        podman,
        &format!("PASSWORD={proxmox_password}\n"),
        PROXMOX_ENV,
        "0600",
    )?;
    install_text(podman, &opentofu, OPENTOFU_ENV, "0600")
}

fn install_text(
    podman: &Path,
    content: &str,
    destination: &str,
    mode: &str,
) -> Result<(), GnxError> {
    install_bytes(podman, content.as_bytes().to_vec(), destination, mode)
}

fn install_blob(
    podman: &Path,
    source: &Path,
    destination: &str,
    mode: &str,
) -> Result<(), GnxError> {
    let bytes = std::fs::read(source)
        .map_err(|error| GnxError::io("runtime_blob_read", error.to_string()))?;
    install_bytes(podman, bytes, destination, mode)
}

fn install_bytes(
    podman: &Path,
    bytes: Vec<u8>,
    destination: &str,
    mode: &str,
) -> Result<(), GnxError> {
    let output_argument = format!("of={destination}");
    CommandSpec::new(podman)
        .args([
            "machine",
            "ssh",
            MACHINE_NAME,
            "sudo",
            "dd",
            &output_argument,
            "status=none",
        ])
        .stdin(bytes)
        .timeout(Duration::from_secs(300))
        .run_checked("runtime_file_install")?;
    remote_checked(
        podman,
        &["sudo", "chmod", mode, destination],
        "runtime_file_permissions",
        Duration::from_secs(30),
    )?;
    Ok(())
}

fn remote(
    podman: &Path,
    arguments: &[&str],
    operation: &'static str,
    timeout: Duration,
) -> Result<crate::process::ProcessOutput, GnxError> {
    CommandSpec::new(podman)
        .args(["machine", "ssh", MACHINE_NAME])
        .args(arguments)
        .timeout(timeout)
        .run(operation)
}

fn remote_checked(
    podman: &Path,
    arguments: &[&str],
    operation: &'static str,
    timeout: Duration,
) -> Result<crate::process::ProcessOutput, GnxError> {
    CommandSpec::new(podman)
        .args(["machine", "ssh", MACHINE_NAME])
        .args(arguments)
        .timeout(timeout)
        .run_checked(operation)
}

pub fn podman_executable() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let program_files = std::env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
        program_files.join("Podman").join("podman.exe")
    }

    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("podman")
    }
}
