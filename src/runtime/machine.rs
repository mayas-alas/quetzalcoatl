use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::{ControllerUrl, MACHINE_NAME};
use crate::error::GnxError;
use crate::process::CommandSpec;

const REMOTE_TOFU_ARCHIVE: &str = "/opt/gnx/guest/opentofu.tar.gz";
const PROXMOX_ENV: &str = "/etc/gnx/proxmox.env";
const OWNERSHIP_SCHEMA: u32 = 1;
const OWNERSHIP_FILE: &str = "machine-ownership.json";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MachineOwnership {
    schema: u32,
    product: String,
    machine_name: String,
}

impl MachineOwnership {
    fn current() -> Self {
        Self {
            schema: OWNERSHIP_SCHEMA,
            product: "Quetzalcoatl Next".to_string(),
            machine_name: MACHINE_NAME.to_string(),
        }
    }

    fn is_current(&self) -> bool {
        self.schema == OWNERSHIP_SCHEMA
            && self.product == "Quetzalcoatl Next"
            && self.machine_name == MACHINE_NAME
    }
}

pub fn ensure(controller: &ControllerUrl) -> Result<(), GnxError> {
    let podman = podman_executable();
    let inspect = CommandSpec::new(&podman)
        .args(["machine", "inspect", MACHINE_NAME])
        .timeout(Duration::from_secs(60))
        .run("machine_inspect")?;
    let ownership = load_ownership()?;
    if inspect.success() && ownership.is_none() {
        return Err(machine_name_conflict(
            "Existe una Podman Machine llamada quetzalcoatl sin marcador de propiedad GNX.",
        ));
    }
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
        save_ownership()?;
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

pub fn verify_local_ownership() -> Result<(), GnxError> {
    if load_ownership()?.is_some() {
        Ok(())
    } else {
        Err(machine_name_conflict(
            "Falta el marcador de propiedad de Podman Machine quetzalcoatl.",
        ))
    }
}

fn ownership_path() -> PathBuf {
    crate::config::data_root().join(OWNERSHIP_FILE)
}

fn load_ownership() -> Result<Option<MachineOwnership>, GnxError> {
    let path = ownership_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(GnxError::io("machine_ownership_read", error.to_string())),
    };
    let ownership: MachineOwnership = serde_json::from_str(&content).map_err(|error| {
        machine_name_conflict(format!(
            "El marcador {} no es válido: {error}.",
            path.display()
        ))
    })?;
    if ownership.is_current() {
        Ok(Some(ownership))
    } else {
        Err(machine_name_conflict(format!(
            "El marcador {} no pertenece a esta versión de GNX.",
            path.display()
        )))
    }
}

fn save_ownership() -> Result<(), GnxError> {
    let bytes = serde_json::to_vec_pretty(&MachineOwnership::current())
        .map_err(|error| GnxError::io("machine_ownership_encode", error.to_string()))?;
    crate::state::atomic_write(&ownership_path(), &bytes)
}

fn machine_name_conflict(detail: impl Into<String>) -> GnxError {
    GnxError::new(
        "MACHINE_NAME_CONFLICT",
        "runtime",
        "machine_ownership",
        detail,
        concat!(
            "GNX no adopta máquinas existentes. Use un host limpio o retire/renombre ",
            "esa máquina fuera de GNX antes de reintentar."
        ),
        false,
        21,
    )
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
            "/opt/gnx/guest/opentofu",
            "/opt/gnx/guest/units",
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

    ensure_proxmox_environment(podman)?;
    ensure_tailscale_environment(podman, controller)?;
    ensure_opentofu_payload(podman)?;

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
            "/opt/gnx/guest/opentofu/versions.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::VARIABLES_TF,
            "/opt/gnx/guest/opentofu/variables.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::MAIN_TF,
            "/opt/gnx/guest/opentofu/main.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::OUTPUTS_TF,
            "/opt/gnx/guest/opentofu/outputs.tf",
            "0644",
        ),
        (
            crate::runtime::opentofu::PROVIDER_LOCK,
            "/opt/gnx/guest/opentofu/.terraform.lock.hcl",
            "0644",
        ),
        (
            crate::runtime::opentofu::RUNNER_BOOTSTRAP,
            "/opt/gnx/guest/infra-runner-bootstrap.sh",
            "0755",
        ),
        (
            crate::runtime::opentofu::RUNNER_COMMAND,
            "/opt/gnx/guest/infra-runner-run.sh",
            "0755",
        ),
        (
            crate::runtime::opentofu::RUNNER_SYSTEMD_UNIT,
            "/opt/gnx/guest/units/gnx-opentofu.service",
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

fn ensure_opentofu_payload(podman: &Path) -> Result<(), GnxError> {
    let dependency = crate::runtime::opentofu::dependency()?;
    let checksum = remote(
        podman,
        &["sha256sum", REMOTE_TOFU_ARCHIVE],
        "opentofu_payload_checksum",
        Duration::from_secs(30),
    )?;
    if checksum.success()
        && checksum
            .stdout
            .split_whitespace()
            .next()
            .is_some_and(|actual| actual.eq_ignore_ascii_case(&dependency.sha256))
    {
        return Ok(());
    }

    let (_, archive) = crate::runtime::opentofu::download()?;
    install_blob(podman, &archive, REMOTE_TOFU_ARCHIVE, "0644")?;
    let installed = remote_checked(
        podman,
        &["sha256sum", REMOTE_TOFU_ARCHIVE],
        "opentofu_payload_verify",
        Duration::from_secs(30),
    )?;
    if !installed
        .stdout
        .split_whitespace()
        .next()
        .is_some_and(|actual| actual.eq_ignore_ascii_case(&dependency.sha256))
    {
        return Err(GnxError::new(
            "OPENTOFU_PAYLOAD_MISMATCH",
            "opentofu",
            "payload",
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
    if proxmox_existing.success() {
        return Ok(());
    }
    let proxmox_password = crate::secrets::random_hex(32)?;
    install_text(
        podman,
        &format!("PASSWORD={proxmox_password}\n"),
        PROXMOX_ENV,
        "0600",
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ownership_marker_is_strictly_greenfield() {
        assert!(MachineOwnership::current().is_current());
        assert!(
            !MachineOwnership {
                schema: OWNERSHIP_SCHEMA,
                product: "legacy".to_string(),
                machine_name: MACHINE_NAME.to_string(),
            }
            .is_current()
        );
    }
}
