use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config::{ControllerUrl, MACHINE_NAME};
use crate::error::GnxError;
use crate::process::CommandSpec;

const REMOTE_TOFU_ARCHIVE: &str = "/opt/gnx/guest/opentofu.tar.gz";
const PROXMOX_ENV: &str = "/etc/gnx/proxmox.env";
const OWNERSHIP_SCHEMA: u32 = 2;
const OWNERSHIP_FILE: &str = "machine-ownership.json";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum OwnershipPhase {
    Provisioning,
    Ready,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MachineOwnership {
    schema: u32,
    product: String,
    machine_name: String,
    phase: OwnershipPhase,
}

impl MachineOwnership {
    fn current(phase: OwnershipPhase) -> Self {
        Self {
            schema: OWNERSHIP_SCHEMA,
            product: "Quetzalcoatl Next".to_string(),
            machine_name: MACHINE_NAME.to_string(),
            phase,
        }
    }

    fn is_current(&self) -> bool {
        self.schema == OWNERSHIP_SCHEMA
            && self.product == "Quetzalcoatl Next"
            && self.machine_name == MACHINE_NAME
    }
}

pub fn ensure(controller: &ControllerUrl) -> Result<(), GnxError> {
    prepare()?;
    deploy(controller)
}

pub fn prepare() -> Result<(), GnxError> {
    let podman = podman_executable();
    prepare_podman_environment()?;
    crate::logs::event(
        "info",
        "runtime",
        "machine_prepare",
        format!("Inspeccionando Podman Machine {MACHINE_NAME}"),
    );
    let inspect = podman_command(&podman)
        .args(["machine", "inspect", MACHINE_NAME])
        .timeout(Duration::from_secs(60))
        .run("machine_inspect")?;
    let mut ownership = load_ownership()?;
    if inspect.success() {
        let Some(marker) = ownership.as_ref() else {
            return Err(machine_name_conflict(
                "Existe una Podman Machine llamada quetzalcoatl sin marcador de propiedad GNX.",
            ));
        };
        if marker.phase == OwnershipPhase::Provisioning {
            save_ownership(OwnershipPhase::Ready)?;
            ownership = Some(MachineOwnership::current(OwnershipPhase::Ready));
        }
    }
    if !inspect.success() {
        match ownership.as_ref().map(|marker| marker.phase) {
            None => save_ownership(OwnershipPhase::Provisioning)?,
            Some(OwnershipPhase::Provisioning) => {}
            Some(OwnershipPhase::Ready) => {
                return Err(machine_name_conflict(
                    "El marcador GNX indica una máquina lista, pero Podman no puede inspeccionarla. GNX no elimina automáticamente una máquina que pudo contener datos.",
                ));
            }
        }
        let mut initialized = initialize_machine(&podman)?;
        if !initialized.success() && is_partial_hypervisor_conflict(&initialized.stderr) {
            recover_partial_machine(&podman)?;
            initialized = initialize_machine(&podman)?;
        }
        if !initialized.success() {
            return Err(GnxError::process(
                "machine_init",
                &podman,
                initialized.stderr,
                true,
            ));
        }
        save_ownership(OwnershipPhase::Ready)?;
        crate::logs::event(
            "info",
            "runtime",
            "machine_init",
            format!("Podman Machine {MACHINE_NAME} creada"),
        );
    }

    let start = podman_command(&podman)
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
    podman_command(&podman)
        .args(["info", "--format", "json"])
        .timeout(Duration::from_secs(60))
        .run_checked("machine_health")?;
    crate::logs::event(
        "info",
        "runtime",
        "machine_ready",
        format!("Podman Machine {MACHINE_NAME} disponible"),
    );
    Ok(())
}

pub fn deploy(controller: &ControllerUrl) -> Result<(), GnxError> {
    install_runtime(controller)?;
    converge_proxmox()?;
    converge_control_plane(controller)?;
    converge_mesh(controller)?;
    converge_docktail()?;
    converge_infra()?;
    Ok(())
}

pub fn install_runtime(controller: &ControllerUrl) -> Result<(), GnxError> {
    deploy_runtime(&podman_executable(), controller)
}

pub fn converge_mesh(controller: &ControllerUrl) -> Result<(), GnxError> {
    let podman = podman_executable();
    ensure_mesh_credentials(&podman)?;
    start_units(
        &podman,
        &["podman.socket", "mesh-agent.service"],
        "mesh_start",
        Duration::from_secs(1200),
    )
    .map_err(|error| component_error("MESH_RUNTIME_START_FAILED", "mesh", "start", error, 16))?;
    verify_mesh(&podman, controller)?;
    clear_runtime_mesh_key(&podman)
}

pub fn converge_control_plane(controller: &ControllerUrl) -> Result<(), GnxError> {
    let podman = podman_executable();
    remote_checked(
        &podman,
        &[
            "sudo",
            "podman",
            "exec",
            "gnx-proxmox",
            "/opt/gnx/guest/control-plane-bootstrap.sh",
            "bootstrap",
        ],
        "control_plane_bootstrap",
        Duration::from_secs(2400),
    )
    .map_err(|error| {
        component_error(
            "CONTROL_PLANE_BOOTSTRAP_FAILED",
            "control_plane",
            "bootstrap",
            error,
            17,
        )
    })?;
    verify_control_plane(&podman, controller)
}

pub fn converge_docktail() -> Result<(), GnxError> {
    let podman = podman_executable();
    start_units(
        &podman,
        &["docktail.service"],
        "docktail_start",
        Duration::from_secs(1200),
    )
    .map_err(|error| component_error("DOCKTAIL_START_FAILED", "docktail", "start", error, 19))?;
    remote_checked(
        &podman,
        &[
            "sudo",
            "systemctl",
            "is-active",
            "--quiet",
            "docktail.service",
        ],
        "docktail_active",
        Duration::from_secs(30),
    )
    .map_err(|error| component_error("DOCKTAIL_UNHEALTHY", "docktail", "health", error, 19))?;
    remote_checked(
        &podman,
        &[
            "sudo",
            "podman",
            "inspect",
            "--format",
            "{{.State.Status}}",
            "gnx-docktail",
        ],
        "docktail_container",
        Duration::from_secs(30),
    )
    .map_err(|error| component_error("DOCKTAIL_UNHEALTHY", "docktail", "container", error, 19))?;
    Ok(())
}

pub fn converge_proxmox() -> Result<(), GnxError> {
    let podman = podman_executable();
    for device in ["/dev/kvm", "/dev/fuse"] {
        let probe = remote(
            &podman,
            &["sudo", "test", "-e", device],
            "proxmox_device",
            Duration::from_secs(30),
        )?;
        if !probe.success() {
            return Err(GnxError::new(
                "PROXMOX_ACCELERATION_UNAVAILABLE",
                "proxmox",
                "device_preflight",
                format!("{device} no está disponible dentro de Podman Machine quetzalcoatl."),
                "Habilite virtualización anidada/KVM y reinicie Windows antes de ejecutar gnx init.",
                false,
                20,
            ));
        }
    }
    start_units(
        &podman,
        &["proxmox.service"],
        "proxmox_start",
        Duration::from_secs(1200),
    )
    .map_err(|error| component_error("PROXMOX_START_FAILED", "proxmox", "start", error, 20))?;
    remote_checked(
        &podman,
        &[
            "sudo",
            "podman",
            "wait",
            "--condition=healthy",
            "--interval=5s",
            "gnx-proxmox",
        ],
        "proxmox_health",
        Duration::from_secs(1200),
    )
    .map_err(|error| component_error("PROXMOX_UNHEALTHY", "proxmox", "health", error, 20))?;
    Ok(())
}

pub fn converge_infra() -> Result<(), GnxError> {
    let podman = podman_executable();
    remote_checked(
        &podman,
        &["sudo", "systemctl", "enable", "gnx-opentofu.service"],
        "opentofu_enable",
        Duration::from_secs(60),
    )
    .map_err(|error| component_error("INFRA_APPLY_FAILED", "infra", "enable", error, 18))?;
    remote_checked(
        &podman,
        &["sudo", "systemctl", "restart", "gnx-opentofu.service"],
        "opentofu_apply",
        Duration::from_secs(2400),
    )
    .map_err(|error| component_error("INFRA_APPLY_FAILED", "infra", "apply", error, 18))?;
    for (operation, arguments) in [
        (
            "opentofu_runner_health",
            vec![
                "sudo",
                "podman",
                "exec",
                "gnx-proxmox",
                "pct",
                "exec",
                "200",
                "--",
                "/usr/local/bin/tofu",
                "version",
            ],
        ),
        (
            "workload_lxc_health",
            vec![
                "sudo",
                "podman",
                "exec",
                "gnx-proxmox",
                "pct",
                "exec",
                "201",
                "--",
                "systemctl",
                "is-active",
                "--quiet",
                "mesh-agent.service",
                "docktail.service",
            ],
        ),
    ] {
        remote_checked(&podman, &arguments, operation, Duration::from_secs(60)).map_err(
            |error| component_error("INFRA_HEALTH_FAILED", "infra", operation, error, 18),
        )?;
    }
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

fn save_ownership(phase: OwnershipPhase) -> Result<(), GnxError> {
    let bytes = serde_json::to_vec_pretty(&MachineOwnership::current(phase))
        .map_err(|error| GnxError::io("machine_ownership_encode", error.to_string()))?;
    crate::state::atomic_write(&ownership_path(), &bytes)
}

fn initialize_machine(podman: &Path) -> Result<crate::process::ProcessOutput, GnxError> {
    let command = podman_command(podman).args(["machine", "init"]);
    #[cfg(target_os = "windows")]
    let command = command.args(["--provider", "wsl"]);
    command
        .args([
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
        .run("machine_init")
}

fn is_partial_hypervisor_conflict(stderr: &str) -> bool {
    stderr
        .to_ascii_lowercase()
        .contains("already exists on hypervisor")
}

#[cfg(target_os = "windows")]
fn recover_partial_machine(podman: &Path) -> Result<(), GnxError> {
    crate::logs::event(
        "warn",
        "runtime",
        "machine_partial_recovery",
        "Retirando exclusivamente la distribución WSL parcial de GNX antes de reintentar",
    );
    let wsl = Path::new(r"C:\Windows\System32\wsl.exe");
    let distribution = format!("podman-{MACHINE_NAME}");
    podman_command(wsl)
        .args(["--unregister", &distribution])
        .timeout(Duration::from_secs(120))
        .run_checked("machine_partial_unregister")?;

    // Podman puede haber dejado metadatos locales aunque no haya podido crear
    // la conexión. Su propio remove es seguro aquí: el marcador sigue en fase
    // provisioning y la cuenta dedicada pertenece exclusivamente a GNX.
    let _ = podman_command(podman)
        .args(["machine", "rm", "--force", MACHINE_NAME])
        .timeout(Duration::from_secs(120))
        .run("machine_partial_metadata_remove");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn recover_partial_machine(_podman: &Path) -> Result<(), GnxError> {
    Err(machine_name_conflict(
        "El hipervisor reportó una máquina parcial; la recuperación automática sólo está habilitada para la distribución WSL exclusiva de GNX.",
    ))
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

fn deploy_runtime(
    podman: &Path,
    controller: &ControllerUrl,
) -> Result<(), GnxError> {
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
            "/run/gnx/control-plane",
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
    remote_checked(
        podman,
        &["sudo", "install", "-d", "-m", "0700", "/run/gnx/mesh"],
        "runtime_ephemeral_secret_directory",
        Duration::from_secs(30),
    )?;

    ensure_proxmox_environment(podman)?;
    ensure_mesh_environment(podman, controller)?;
    ensure_opentofu_payload(podman)?;
    let headscale_config = crate::runtime::headscale::config(controller)?;

    for (content, destination, mode) in [
        (
            crate::runtime::mesh_agent::QUADLET,
            "/etc/containers/systemd/mesh-agent.container",
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
            crate::runtime::mesh_agent::GUEST_QUADLET,
            "/opt/gnx/guest/units/mesh-agent.container",
            "0644",
        ),
        (
            include_str!("../../guest/units/docktail.container"),
            "/opt/gnx/guest/units/docktail.container",
            "0644",
        ),
        (
            &crate::runtime::mesh_agent::environment(controller, "gnx-cell-01")?,
            "/opt/gnx/guest/mesh-agent.env",
            "0600",
        ),
        (
            crate::runtime::headscale::BOOTSTRAP,
            "/opt/gnx/guest/control-plane-bootstrap.sh",
            "0755",
        ),
        (
            crate::runtime::headscale::QUADLET,
            "/opt/gnx/guest/units/headscale.container",
            "0644",
        ),
        (
            headscale_config.as_str(),
            "/opt/gnx/guest/headscale-config.yaml",
            "0644",
        ),
        (
            crate::runtime::headscale::POLICY,
            "/opt/gnx/guest/headscale-policy.hujson",
            "0644",
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
    Ok(())
}

fn start_units(
    podman: &Path,
    units: &[&str],
    operation: &'static str,
    timeout: Duration,
) -> Result<(), GnxError> {
    let mut arguments = vec!["sudo", "systemctl", "enable", "--now"];
    arguments.extend_from_slice(units);
    remote_checked(podman, &arguments, operation, timeout)?;
    Ok(())
}

fn verify_mesh(podman: &Path, controller: &ControllerUrl) -> Result<(), GnxError> {
    let started = std::time::Instant::now();
    let timeout = Duration::from_secs(120);
    let mut last_state = "unavailable".to_string();
    while started.elapsed() < timeout {
        let status = remote(
            podman,
            &[
                "sudo",
                "podman",
                "exec",
                "gnx-tailscale",
                "tailscale",
                "--socket=/var/run/tailscale/tailscaled.sock",
                "status",
                "--json",
            ],
            "mesh_status",
            Duration::from_secs(30),
        )?;
        if status.success()
            && let Ok(document) = serde_json::from_str::<serde_json::Value>(&status.stdout)
        {
            last_state = document
                .get("BackendState")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let has_ip = document
                .get("TailscaleIPs")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|addresses| !addresses.is_empty());
            if last_state == "Running" && has_ip {
                verify_mesh_controller(podman, controller)?;
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_secs(5));
    }
    let key_present = remote(
        podman,
        &["sudo", "test", "-s", "/run/gnx/mesh/auth.key"],
        "mesh_auth_presence",
        Duration::from_secs(30),
    )?
    .success();
    Err(GnxError::new(
        if key_present {
            "MESH_ENROLLMENT_FAILED"
        } else {
            "MESH_ENROLLMENT_REQUIRED"
        },
        "mesh",
        "enrollment",
        format!("tailscaled no obtuvo identidad; BackendState={last_state}."),
        if key_present {
            "Verifique que la pre-auth key de Headscale sea válida, reutilizable y etiquetada; consulte gnx logs."
        } else {
            "Entregue una pre-auth key reutilizable por stdin: Get-Content <archivo-seguro> | gnx init --mesh-auth-stdin"
        },
        true,
        16,
    ))
}

fn verify_mesh_controller(podman: &Path, controller: &ControllerUrl) -> Result<(), GnxError> {
    let prefs = remote_checked(
        podman,
        &[
            "sudo",
            "podman",
            "exec",
            "gnx-tailscale",
            "tailscale",
            "--socket=/var/run/tailscale/tailscaled.sock",
            "debug",
            "prefs",
        ],
        "mesh_controller_observed",
        Duration::from_secs(30),
    )?;
    let document: serde_json::Value = serde_json::from_str(&prefs.stdout).map_err(|error| {
        GnxError::new(
            "MESH_STATE_INVALID",
            "mesh",
            "controller_observed",
            error.to_string(),
            "Conserve los logs y ejecute gnx repair.",
            true,
            16,
        )
    })?;
    let observed = document
        .get("ControlURL")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim_end_matches('/');
    if observed != controller.canonical() {
        return Err(GnxError::new(
            "MESH_CONTROLLER_MISMATCH",
            "mesh",
            "controller_observed",
            format!(
                "tailscaled usa '{}', pero config exige '{}'.",
                if observed.is_empty() {
                    "unknown"
                } else {
                    observed
                },
                controller.canonical()
            ),
            "No se aplicó fallback. Re-enrole la identidad contra el controller configurado.",
            false,
            16,
        ));
    }
    Ok(())
}

fn component_error(
    code: &'static str,
    component: &'static str,
    operation: &'static str,
    source: GnxError,
    exit_code: u8,
) -> GnxError {
    GnxError::new(
        code,
        component,
        operation,
        source.message,
        "Consulte gnx logs --tail 100; GNX conservará los recursos y reintentará.",
        true,
        exit_code,
    )
}

fn ensure_tailscale_environment(podman: &Path, controller: &ControllerUrl) -> Result<(), GnxError> {
    install_text(
        podman,
        &tailscale_environment(controller, "gnx-runtime"),
        "/etc/gnx/tailscale-controller.env",
        "0600",
    )?;
    #[cfg(target_os = "windows")]
    let mut pending = crate::host::windows::ipc::load_pending_mesh_auth()?;
    #[cfg(not(target_os = "windows"))]
    let mut pending: Option<Vec<u8>> = None;

    if let Some(secret) = pending.as_ref() {
        install_bytes(podman, secret.clone(), "/run/gnx/mesh/auth.key", "0400")?;
        #[cfg(target_os = "windows")]
        crate::host::windows::ipc::discard_pending_mesh_auth()?;
    }
    if let Some(secret) = pending.as_mut() {
        secret.fill(0);
    }

    let auth = remote(
        podman,
        &["sudo", "test", "-s", "/run/gnx/mesh/auth.key"],
        "tailscale_auth_check",
        Duration::from_secs(30),
    )?;
    let environment = if auth.success() {
        "TS_AUTHKEY=file:/run/secrets/gnx/auth.key\n"
    } else {
        ""
    };
    install_text(
        podman,
        environment,
        "/run/gnx/mesh/tailscale-auth.env",
        "0600",
    )?;
    Ok(())
}

fn tailscale_environment(controller: &ControllerUrl, hostname: &str) -> String {
    format!(
        "TS_HOSTNAME={hostname}\nTS_AUTH_ONCE=true\nTS_ACCEPT_DNS=true\nTS_EXTRA_ARGS=--login-server={} --accept-dns=true --ssh=false\n",
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
    podman_command(podman)
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
    podman_command(podman)
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
    podman_command(podman)
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

fn podman_command(podman: &Path) -> CommandSpec {
    let command = CommandSpec::new(podman);
    #[cfg(target_os = "windows")]
    {
        let profile = crate::host::windows::account::runtime_profile_path();
        command
            .cwd(&profile)
            .env("HOME", &profile)
            .env("USERPROFILE", &profile)
            .env("XDG_CONFIG_HOME", profile.join(".config"))
            .env("XDG_DATA_HOME", profile.join(".local").join("share"))
            .env("APPDATA", profile.join("AppData").join("Roaming"))
            .env("LOCALAPPDATA", profile.join("AppData").join("Local"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        command
    }
}

#[cfg(target_os = "windows")]
fn prepare_podman_environment() -> Result<(), GnxError> {
    let profile = crate::host::windows::account::runtime_profile_path();
    for path in [
        profile.join(".config").join("containers"),
        profile.join(".local").join("share").join("containers"),
        profile.join("AppData").join("Roaming"),
        profile.join("AppData").join("Local"),
    ] {
        std::fs::create_dir_all(&path).map_err(|error| {
            GnxError::io(
                "machine_profile_prepare",
                format!("{}: {error}", path.display()),
            )
        })?;
    }
    crate::logs::event(
        "info",
        "runtime",
        "machine_profile",
        format!("Podman aislado en {}", profile.display()),
    );
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn prepare_podman_environment() -> Result<(), GnxError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ownership_marker_is_strictly_greenfield() {
        assert!(MachineOwnership::current(OwnershipPhase::Ready).is_current());
        assert!(
            !MachineOwnership {
                schema: OWNERSHIP_SCHEMA,
                product: "legacy".to_string(),
                machine_name: MACHINE_NAME.to_string(),
                phase: OwnershipPhase::Ready,
            }
            .is_current()
        );
    }

    #[test]
    fn only_exact_partial_hypervisor_conflict_is_recoverable() {
        assert!(is_partial_hypervisor_conflict(
            "Error: quetzalcoatl already exists on hypervisor"
        ));
        assert!(!is_partial_hypervisor_conflict(
            "Error: existing machine contains user data"
        ));
    }
}
