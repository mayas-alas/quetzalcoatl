use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use gnx_contracts::{
    FORGEJO_ADMIN_USERNAME, ForgejoAdminResponse, NodeRole, PlatformConfiguration, PlatformUrl,
    StatusResponse,
};
use zeroize::Zeroize;

use super::status::{
    set_platform_failed, set_platform_ready, set_platform_reconciling, set_platform_waiting,
};
use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;
use crate::domain::topology::load_protected_configuration;
use crate::infrastructure::platform_bundle::apply_platform_bundle;
use crate::infrastructure::remote::{RuntimeOperation, podman_binary, runtime_agent};

const CONFIGURATION_POLL: Duration = Duration::from_millis(500);
const FAILURE_RETRY: Duration = Duration::from_secs(60);
const DEPLOYMENT_POLL: Duration = Duration::from_secs(60);

pub(super) fn run(status: &Arc<RwLock<StatusResponse>>) {
    let role = status.read().ok().and_then(|value| value.role);
    if role != Some(NodeRole::Controller) {
        return;
    }

    set_platform_waiting(status);
    loop {
        if crate::infrastructure::service_shutdown::requested() {
            return;
        }
        let mut platform = match load_platform_configuration() {
            Ok(Some(configuration)) => configuration,
            Ok(None) => {
                thread::sleep(CONFIGURATION_POLL);
                continue;
            }
            Err(error) => {
                set_platform_failed(status, &error);
                sleep_until_retry();
                continue;
            }
        };

        set_platform_reconciling(status);
        match reconcile(status, &mut platform) {
            Ok(url) => {
                set_platform_ready(status, url.clone());
                monitor_deployments(status, &url);
                return;
            }
            Err(error) => {
                set_platform_failed(status, &error);
                sleep_until_retry();
            }
        }
    }
}

fn reconcile(
    status: &Arc<RwLock<StatusResponse>>,
    platform: &mut PlatformConfiguration,
) -> Result<PlatformUrl, GateError> {
    let podman = podman_binary()?;
    apply_platform_bundle(&podman)?;
    let tailnet = invoke_platform_operation(
        status,
        platform,
        &podman,
        RuntimeOperation::PlatformReconcile,
        "PLATFORM_RECONCILE=ready",
    )?;

    PlatformUrl::parse(format!("https://gnx-forgejo.{tailnet}/")).map_err(|message| {
        GateError::new("PLATFORM_URL_INVALID", Component::TailscaleServe, message)
    })
}

fn monitor_deployments(status: &Arc<RwLock<StatusResponse>>, forgejo_url: &PlatformUrl) {
    loop {
        if crate::infrastructure::service_shutdown::requested() {
            return;
        }
        let mut platform = match load_platform_configuration() {
            Ok(Some(configuration)) => configuration,
            Ok(None) => {
                set_platform_waiting(status);
                sleep_interval(DEPLOYMENT_POLL);
                continue;
            }
            Err(error) => {
                set_platform_failed(status, &error);
                sleep_interval(FAILURE_RETRY);
                continue;
            }
        };
        match deploy(status, &mut platform) {
            Ok(()) => set_platform_ready(status, forgejo_url.clone()),
            Err(error) => set_platform_failed(status, &error),
        }
        sleep_interval(DEPLOYMENT_POLL);
    }
}

fn deploy(
    status: &Arc<RwLock<StatusResponse>>,
    platform: &mut PlatformConfiguration,
) -> Result<(), GateError> {
    let podman = podman_binary()?;
    invoke_platform_operation(
        status,
        platform,
        &podman,
        RuntimeOperation::PlatformDeploy,
        "PLATFORM_DEPLOY=ready",
    )
    .map(|_| ())
}

fn invoke_platform_operation(
    status: &Arc<RwLock<StatusResponse>>,
    platform: &mut PlatformConfiguration,
    podman: &std::path::Path,
    operation: RuntimeOperation,
    completion_marker: &'static str,
) -> Result<String, GateError> {
    let mut node = load_protected_configuration()?;
    let controller = status
        .read()
        .map_err(|_| {
            GateError::new(
                "PLATFORM_STATE_FAILED",
                Component::None,
                "runtime status lock is poisoned",
            )
        })?
        .controller
        .clone()
        .ok_or_else(|| {
            GateError::new(
                "PLATFORM_STATE_FAILED",
                Component::None,
                "READY controller status has no controller identity",
            )
        })?;

    let mut input = Vec::with_capacity(
        controller.len()
            + node.tailnet.len()
            + node.pve_root_password.len()
            + platform.tailscale_auth_key.len()
            + 16,
    );
    input.extend_from_slice(b"1\n");
    input.extend_from_slice(controller.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(node.tailnet.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(node.pve_root_password.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(platform.tailscale_auth_key.as_bytes());
    input.push(b'\n');

    let tailnet = node.tailnet.to_string();
    let error_code = if operation == RuntimeOperation::PlatformDeploy {
        "PLATFORM_DEPLOY_FAILED"
    } else {
        "PLATFORM_RECONCILE_FAILED"
    };
    let result = runtime_agent(podman, operation, &input)
        .map_err(|error| error.with_code(error_code, Component::Proxmox));
    input.zeroize();
    node.pve_root_password.zeroize();
    platform.tailscale_auth_key.zeroize();
    let output = result?;
    if String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| *line == completion_marker)
        .count()
        != 1
    {
        return Err(GateError::new(
            "PLATFORM_RECONCILE_FAILED",
            Component::Proxmox,
            "platform runner did not return its closed completion marker",
        ));
    }
    Ok(tailnet)
}

fn load_platform_configuration() -> Result<Option<PlatformConfiguration>, GateError> {
    crate::infrastructure::secrets::load_platform_optional().map_err(|error| {
        GateError::new(
            "PLATFORM_CONFIGURATION_INVALID",
            Component::None,
            error.message(),
        )
    })
}

pub(crate) fn forgejo_admin(
    status: &Arc<RwLock<StatusResponse>>,
    reset: bool,
) -> Result<ForgejoAdminResponse, GateError> {
    let is_ready_controller = status
        .read()
        .map_err(|_| {
            GateError::new(
                "FORGEJO_ADMIN_FAILED",
                Component::None,
                "runtime status lock is poisoned",
            )
        })?
        .role
        == Some(NodeRole::Controller);
    if !is_ready_controller {
        return Err(GateError::new(
            "FORGEJO_ADMIN_UNAVAILABLE",
            Component::None,
            "Forgejo administration is available only on the active controller",
        ));
    }

    let podman = podman_binary()?;
    apply_platform_bundle(&podman)?;
    let operation = if reset {
        RuntimeOperation::ForgejoAdminReset
    } else {
        RuntimeOperation::ForgejoAdminShow
    };
    let mut output = runtime_agent(&podman, operation, &[])
        .map_err(|error| error.with_code("FORGEJO_ADMIN_FAILED", Component::Proxmox))?;
    let parsed = parse_forgejo_admin_output(&output.stdout, reset);
    output.stdout.zeroize();
    output.stderr.zeroize();
    parsed
}

fn parse_forgejo_admin_output(
    bytes: &[u8],
    reset: bool,
) -> Result<ForgejoAdminResponse, GateError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        GateError::new(
            "FORGEJO_ADMIN_FAILED",
            Component::Proxmox,
            "Forgejo admin operation returned non-UTF-8 output",
        )
    })?;
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 2 || lines[0] != format!("FORGEJO_ADMIN_USERNAME={FORGEJO_ADMIN_USERNAME}") {
        return Err(GateError::new(
            "FORGEJO_ADMIN_FAILED",
            Component::Proxmox,
            "Forgejo admin operation returned an invalid identity contract",
        ));
    }
    let password = lines[1]
        .strip_prefix("FORGEJO_ADMIN_PASSWORD=")
        .filter(|value| {
            value.len() == 48
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or_else(|| {
            GateError::new(
                "FORGEJO_ADMIN_FAILED",
                Component::Proxmox,
                "Forgejo admin operation returned an invalid credential contract",
            )
        })?;
    Ok(ForgejoAdminResponse::accepted(password.into(), reset))
}

fn sleep_until_retry() {
    sleep_interval(FAILURE_RETRY);
}

fn sleep_interval(interval: Duration) {
    let mut elapsed = Duration::ZERO;
    while elapsed < interval && !crate::infrastructure::service_shutdown::requested() {
        thread::sleep(CONFIGURATION_POLL);
        elapsed += CONFIGURATION_POLL;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forgejo_admin_output_accepts_only_the_fixed_identity_and_secret_shape() {
        let shown = parse_forgejo_admin_output(
            format!(
                "FORGEJO_ADMIN_USERNAME=gnx-admin\nFORGEJO_ADMIN_PASSWORD={}\n",
                "a".repeat(48)
            )
            .as_bytes(),
            false,
        )
        .expect("valid credential");
        assert_eq!(shown.username.as_deref(), Some(FORGEJO_ADMIN_USERNAME));
        assert_eq!(shown.stage, gnx_contracts::ForgejoAdminStage::Shown);

        for invalid in [
            "FORGEJO_ADMIN_USERNAME=admin\nFORGEJO_ADMIN_PASSWORD=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            "FORGEJO_ADMIN_USERNAME=gnx-admin\nFORGEJO_ADMIN_PASSWORD=secret\n",
            "FORGEJO_ADMIN_USERNAME=gnx-admin\nFORGEJO_ADMIN_PASSWORD=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n",
        ] {
            assert!(parse_forgejo_admin_output(invalid.as_bytes(), false).is_err());
        }
    }
}
