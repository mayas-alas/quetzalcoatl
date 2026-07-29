use std::net::IpAddr;
use std::path::Path;
use std::thread;
use std::time::Duration;

use zeroize::Zeroize;

use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;
use crate::domain::topology::validate_state_identity;
use crate::infrastructure::remote::{RuntimeOperation, bounded_text, machine_stdin, runtime_agent};
use crate::infrastructure::runtime_assets::{
    DEVICE_PROBE, PROXMOX_DIAGNOSTICS, PVE_READY_PROBE, START_PROXMOX,
};
use crate::infrastructure::tailscale::{stabilize_host_inventory, wait_for_tailscale};

pub(crate) fn start_proxmox(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["sh", "-s"], START_PROXMOX.as_bytes())
        .map_err(|error| error.with_code("NESTED_RUNTIME_FAILED", Component::Proxmox))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "PROXMOX_SERVICE=active" {
        return Err(GateError::new(
            "NESTED_RUNTIME_FAILED",
            Component::Proxmox,
            "systemd did not confirm the generated Proxmox Quadlet service",
        ));
    }
    Ok(())
}

pub(crate) fn validate_proxmox_devices(podman: &Path) -> Result<(), GateError> {
    let mut last_error = String::from("Proxmox container did not become executable");
    for attempt in 0..30 {
        crate::infrastructure::service_shutdown::ensure_running()?;
        match machine_stdin(
            podman,
            ["podman", "exec", "-i", "gnx-proxmox", "python3", "-"],
            DEVICE_PROBE.as_bytes(),
        ) {
            Ok(output)
                if String::from_utf8_lossy(&output.stdout).trim()
                    == "KVM_API_VERSION=12;TUN=ready;FUSE=ready" =>
            {
                return Ok(());
            }
            Ok(output) => {
                last_error = format!(
                    "unexpected device probe output: {}",
                    bounded_text(&output.stdout)
                );
            }
            Err(error) => last_error = error.message,
        }
        if machine_stdin(
            podman,
            ["sh", "-s"],
            b"systemctl is-failed --quiet proxmox.service\n",
        )
        .is_ok()
        {
            break;
        }
        if attempt + 1 < 30 {
            thread::sleep(Duration::from_secs(2));
        }
    }
    let diagnostics = proxmox_diagnostics(podman);
    Err(GateError::new(
        "NESTED_RUNTIME_FAILED",
        Component::Proxmox,
        format!("container did not confirm KVM API 12, TUN and FUSE: {last_error}; {diagnostics}"),
    ))
}

pub(crate) fn wait_for_proxmox(podman: &Path) -> Result<(), GateError> {
    let mut last_error = String::from("Proxmox services are not ready");
    for attempt in 0..120 {
        crate::infrastructure::service_shutdown::ensure_running()?;
        match machine_stdin(podman, ["sh", "-s"], PVE_READY_PROBE.as_bytes()) {
            Ok(output)
                if String::from_utf8_lossy(&output.stdout).trim()
                    == "PVE=ready;SYSTEMD=ready;CGROUP=ready" =>
            {
                return Ok(());
            }
            Ok(output) => {
                last_error = format!(
                    "unexpected PVE probe output: {}",
                    bounded_text(&output.stdout)
                );
            }
            Err(error) => last_error = error.message,
        }
        if attempt + 1 < 120 {
            thread::sleep(Duration::from_secs(5));
        }
    }
    Err(GateError::new(
        "NESTED_RUNTIME_FAILED",
        Component::Proxmox,
        format!(
            "PVE did not become healthy within 10 minutes: {last_error}; {}",
            proxmox_diagnostics(podman)
        ),
    ))
}

pub(crate) fn proxmox_diagnostics(podman: &Path) -> String {
    match machine_stdin(podman, ["sh", "-s"], PROXMOX_DIAGNOSTICS.as_bytes()) {
        Ok(output) => bounded_text(&output.stdout),
        Err(error) => format!("diagnostics unavailable: {}", error.message),
    }
}

pub(crate) fn confirm_empty_controller_inventory(
    podman: &Path,
    state: &crate::infrastructure::state::PersistedState,
) -> Result<(), GateError> {
    let identity = wait_for_tailscale(podman, &state.controller.hostname, &state.tailnet)?;
    validate_state_identity(state, &identity)?;
    let identity = stabilize_host_inventory(podman, identity, &state.tailnet)?;
    if !identity.host_peers.is_empty() {
        return Err(GateError::new(
            "TOPOLOGY_CHANGED",
            Component::Tailscale,
            format!(
                "tagged host inventory changed before cluster creation; observed {} other nodes",
                identity.host_peers.len()
            ),
        ));
    }
    Ok(())
}

pub(crate) fn prepare_pve_identity(
    podman: &Path,
    self_ip: IpAddr,
    hostname: &str,
) -> Result<(), GateError> {
    let input = format!("{self_ip}\n{hostname}\n");
    let output = runtime_agent(
        podman,
        RuntimeOperation::PveClusterPrepare,
        input.as_bytes(),
    )
    .map_err(|error| error.with_code("PVE_IDENTITY_PREPARE_FAILED", Component::Proxmox))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "PVE_IDENTITY=ready" {
        return Err(GateError::new(
            "PVE_IDENTITY_PREPARE_FAILED",
            Component::Proxmox,
            "PVE identity was not persisted before the first Proxmox start",
        ));
    }
    Ok(())
}

pub(crate) fn verify_pve_identity(
    podman: &Path,
    self_ip: IpAddr,
    hostname: &str,
) -> Result<(), GateError> {
    let input = format!("{self_ip}\n{hostname}\n");
    let mut last_error = String::from("PVE node identity is not coherent");
    for attempt in 0..60 {
        crate::infrastructure::service_shutdown::ensure_running()?;
        match runtime_agent(
            podman,
            RuntimeOperation::PveClusterVerifyNode,
            input.as_bytes(),
        ) {
            Ok(output) if String::from_utf8_lossy(&output.stdout).trim() == "PVE_NODE=ready" => {
                return Ok(());
            }
            Ok(output) => {
                last_error = format!("unexpected output: {}", bounded_text(&output.stdout))
            }
            Err(error) => last_error = error.message,
        }
        if attempt + 1 < 60 {
            thread::sleep(Duration::from_secs(2));
        }
    }
    Err(GateError::new(
        "PVE_NODE_IDENTITY_MISMATCH",
        Component::Proxmox,
        last_error,
    ))
}

pub(crate) fn create_controller_cluster(
    podman: &Path,
    self_ip: IpAddr,
    hostname: &str,
) -> Result<(), GateError> {
    let input = format!("{self_ip}\n{hostname}\n");
    let output = runtime_agent(podman, RuntimeOperation::PveClusterCreate, input.as_bytes())
        .map_err(|error| error.with_code("PVE_CLUSTER_CREATE_FAILED", Component::Proxmox))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "PVE_CLUSTER=ready" {
        return Err(GateError::new(
            "PVE_CLUSTER_CREATE_FAILED",
            Component::Proxmox,
            "PVE did not confirm the controller cluster contract",
        ));
    }
    Ok(())
}

pub(crate) fn verify_controller_cluster(
    podman: &Path,
    self_ip: IpAddr,
    hostname: &str,
) -> Result<(), GateError> {
    let input = format!("{self_ip}\n{hostname}\n");
    let output = runtime_agent(podman, RuntimeOperation::PveClusterVerify, input.as_bytes())
        .map_err(|error| error.with_code("PVE_CLUSTER_VERIFY_FAILED", Component::Proxmox))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "PVE_CLUSTER=ready" {
        return Err(GateError::new(
            "PVE_CLUSTER_VERIFY_FAILED",
            Component::Proxmox,
            "PVE did not confirm the persisted controller cluster contract",
        ));
    }
    Ok(())
}

pub(crate) fn configure_pve_password(podman: &Path, password: &str) -> Result<(), GateError> {
    let mut input = password.as_bytes().to_vec();
    let result = runtime_agent(podman, RuntimeOperation::PveConfigure, &input);
    input.zeroize();
    let output =
        result.map_err(|error| error.with_code("PVE_CREDENTIAL_FAILED", Component::Proxmox))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "PVE_PASSWORD=ready" {
        return Err(GateError::new(
            "PVE_CREDENTIAL_FAILED",
            Component::Proxmox,
            format!(
                "PVE did not confirm credential replacement; output: {}",
                bounded_text(&output.stdout)
            ),
        ));
    }
    Ok(())
}
