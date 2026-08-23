use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use gnx_contracts::{InstallerConfiguration, StatusResponse};
use zeroize::{Zeroize, Zeroizing};

use super::cluster::{
    authorize_member, confirm_membership, persist_member_stage, prepare_member, verify_member,
};
use super::errors::GateError;
use super::lifecycle::{Component, HostPeer, TailscaleIdentity};
use crate::application::status::set_member_ready_status;
use crate::infrastructure::remote::{RuntimeOperation, runtime_agent, runtime_agent_output};
use crate::infrastructure::tailscale::{stabilize_host_inventory, wait_for_tailscale};

pub(crate) fn wait_for_configuration() -> Result<InstallerConfiguration, GateError> {
    loop {
        crate::infrastructure::service_shutdown::ensure_running()?;
        match crate::infrastructure::secrets::load_optional() {
            Ok(Some(configuration)) => return Ok(configuration),
            Ok(None) => thread::sleep(Duration::from_millis(500)),
            Err(error) => {
                return Err(GateError::new(
                    error.code(),
                    Component::None,
                    error.message(),
                ));
            }
        }
    }
}

pub(crate) fn load_protected_configuration() -> Result<InstallerConfiguration, GateError> {
    match crate::infrastructure::secrets::load_optional() {
        Ok(Some(configuration)) => Ok(configuration),
        Ok(None) => Err(GateError::new(
            "CONFIGURATION_MISSING",
            Component::None,
            "protected installer inputs disappeared after runtime configuration",
        )),
        Err(error) => Err(GateError::new(
            error.code(),
            Component::None,
            error.message(),
        )),
    }
}

pub(crate) fn load_persisted_state()
-> Result<Option<crate::infrastructure::state::PersistedState>, GateError> {
    crate::infrastructure::state::load_optional()
        .map_err(|error| GateError::new("STATE_STORAGE_FAILED", Component::None, error.message()))
}

pub(crate) fn store_persisted_state(
    state: &crate::infrastructure::state::PersistedState,
) -> Result<(), GateError> {
    crate::infrastructure::state::store(state)
        .map_err(|error| GateError::new("STATE_STORAGE_FAILED", Component::None, error.message()))
}

pub(crate) fn controller_stage_rank(stage: &str) -> Option<u8> {
    match stage {
        "ROLE_RESOLVED" => Some(0),
        "CONTROLLER_CLUSTER_READY" | "READY" => Some(1),
        _ => None,
    }
}

pub(crate) fn validate_state_configuration(
    state: &crate::infrastructure::state::PersistedState,
    configuration: &InstallerConfiguration,
) -> Result<(), GateError> {
    let matches = state.tailnet.as_str() == configuration.tailnet.as_ref();
    if !matches {
        return Err(GateError::new(
            "STATE_CONFIGURATION_MISMATCH",
            Component::None,
            "persisted state does not match the protected installer inputs",
        ));
    }
    Ok(())
}

pub(crate) fn join_member_cluster(
    status: &Arc<RwLock<StatusResponse>>,
    podman: &Path,
    member: &mut crate::infrastructure::state::PersistedState,
) -> Result<(), GateError> {
    if !member.role.is_member() {
        return Err(GateError::new(
            "STATE_IDENTITY_INVALID",
            Component::None,
            "member join was requested for a controller state",
        ));
    }
    if begin_member_join(member)? {
        store_persisted_state(member)?;
    }

    prepare_member(status, member)?;
    let mut configuration = load_protected_configuration()?;
    configuration.auth_key.zeroize();
    let configuration_matches = validate_state_configuration(member, &configuration);
    if configuration_matches.is_err() {
        configuration.pve_root_password.zeroize();
        return configuration_matches;
    }
    authorize_member(status, member, &configuration.pve_root_password)?;
    persist_member_stage(status, member, "MEMBER_JOINING")?;
    let join_result = join_member(podman, member, &configuration.pve_root_password);
    configuration.pve_root_password.zeroize();
    join_result?;

    verify_member(status, podman, member)?;
    confirm_membership(status, podman, member)?;

    member.cluster_join = crate::infrastructure::state::ClusterJoinState::Joined;
    member.stage = "READY".into();
    store_persisted_state(member)?;
    set_member_ready_status(status, &member.controller.hostname);
    Ok(())
}

pub(crate) fn begin_member_join(
    member: &mut crate::infrastructure::state::PersistedState,
) -> Result<bool, GateError> {
    match member.cluster_join {
        crate::infrastructure::state::ClusterJoinState::NotStarted => {
            member.cluster_join = crate::infrastructure::state::ClusterJoinState::Joining;
            member.stage = "MEMBER_PREPARING".into();
            Ok(true)
        }
        crate::infrastructure::state::ClusterJoinState::Joining => Ok(false),
        crate::infrastructure::state::ClusterJoinState::Joined => {
            member.cluster_join = crate::infrastructure::state::ClusterJoinState::Joining;
            member.stage = "MEMBER_PREPARING".into();
            Ok(true)
        }
        crate::infrastructure::state::ClusterJoinState::NotApplicable => Err(GateError::new(
            "STATE_IDENTITY_INVALID",
            Component::None,
            "member state has no join checkpoint",
        )),
    }
}

pub(crate) fn join_member(
    podman: &Path,
    member: &crate::infrastructure::state::PersistedState,
    password: &str,
) -> Result<(), GateError> {
    let hostname = persisted_local_hostname(member)?;
    let input = member_join_input(member, hostname, password);
    let result = runtime_agent_output(podman, RuntimeOperation::PveClusterJoin, &input);
    drop(input);
    let mut output = match result {
        Ok(output) => output,
        Err(_) => {
            return Err(GateError::new(
                "PVE_JOIN_FAILED",
                Component::Proxmox,
                "PVE member join did not complete",
            ));
        }
    };
    if !output.status.success() {
        let error = map_member_join_error(&output.stderr);
        output.stdout.zeroize();
        output.stderr.zeroize();
        return Err(error);
    }
    let ready = output.stdout.as_slice() == b"PVE_JOIN=ready\n";
    output.stdout.zeroize();
    output.stderr.zeroize();
    if ready {
        Ok(())
    } else {
        Err(GateError::new(
            "PVE_JOIN_FAILED",
            Component::Proxmox,
            "PVE join did not confirm the fixed output contract",
        ))
    }
}

pub(crate) fn member_join_input(
    member: &crate::infrastructure::state::PersistedState,
    member_hostname: &str,
    password: &str,
) -> Zeroizing<Vec<u8>> {
    let controller_ip = member.controller.ip.to_string();
    let member_ip = member.self_ip.to_string();
    let mut input = Zeroizing::new(Vec::with_capacity(
        controller_ip.len()
            + member.controller.hostname.len()
            + member_ip.len()
            + member_hostname.len()
            + password.len()
            + 5,
    ));
    for value in [
        controller_ip.as_str(),
        member.controller.hostname.as_str(),
        member_ip.as_str(),
        member_hostname,
        password,
    ] {
        input.extend_from_slice(value.as_bytes());
        input.push(b'\n');
    }
    input
}

pub(crate) fn map_member_join_error(output: &[u8]) -> GateError {
    let network_preflight = [
        "PVE_JOIN_CLOCK_UNSYNCED",
        "PVE_JOIN_CONTROLLER_DNS",
        "PVE_JOIN_API_UNREACHABLE",
        "PVE_JOIN_SSH_UNREACHABLE",
        "PVE_JOIN_COROSYNC_UNREACHABLE",
        "PVE_JOIN_MTU_UNUSABLE",
        "PVE_JOIN_MEMBER_NETWORK",
    ];
    let contains = |code: &&str| {
        output
            .windows(code.len())
            .any(|candidate| candidate == code.as_bytes())
    };
    let code = if network_preflight.iter().any(contains) {
        "CLUSTER_NETWORK_PREFLIGHT_FAILED"
    } else if [
        "PVE_JOIN_TAILSCALE_UNREACHABLE",
        "PVE_JOIN_TAILSCALE_RELAYED",
        "PVE_JOIN_TAILSCALE_LATENCY",
    ]
    .iter()
    .any(contains)
    {
        "TAILSCALE_DIRECT_PATH_REQUIRED"
    } else {
        "PVE_JOIN_FAILED"
    };
    GateError::new(code, Component::Proxmox, "PVE member join did not complete")
}

pub(crate) fn resolve_controller(
    podman: &Path,
    persisted: Option<crate::infrastructure::state::PersistedState>,
    identity: TailscaleIdentity,
    tailnet: &str,
) -> Result<crate::infrastructure::state::PersistedState, GateError> {
    if let Some(state) = persisted {
        let (state, node_id_rotated) = reconcile_persisted_identity(state, &identity)?;
        if state.role.is_member() {
            validate_persisted_member_controller(
                &state,
                &stabilize_host_inventory(podman, identity, tailnet)?,
            )?;
        }
        if node_id_rotated {
            store_persisted_state(&state)?;
        }
        return Ok(state);
    }

    // Lean discovery only needs to know whether an online controller exists.
    // Members and transient candidates never affect role selection.
    let decision = select_topology(&identity)?;
    let (state, hostname) = match decision {
        TopologyDecision::Controller => {
            let hostname = controller_hostname(&identity.self_id)?;
            (
                crate::infrastructure::state::PersistedState::controller(
                    identity.self_id.clone(),
                    identity.self_ip,
                    hostname.clone(),
                    tailnet.to_owned(),
                ),
                hostname,
            )
        }
        TopologyDecision::Member(controller) => {
            let hostname = member_hostname(&identity.self_id)?;
            (
                crate::infrastructure::state::PersistedState::member(
                    identity.self_id.clone(),
                    identity.self_ip,
                    hostname.clone(),
                    crate::infrastructure::state::ControllerIdentity {
                        id: controller.id,
                        hostname: controller.hostname,
                        ip: controller.ip,
                    },
                    tailnet.to_owned(),
                ),
                hostname,
            )
        }
    };
    rename_tailscale(podman, &hostname)?;
    let renamed = wait_for_tailscale(podman, &hostname, tailnet)?;
    validate_state_identity(&state, &renamed)?;
    if state.role.is_member() {
        validate_persisted_member_controller(&state, &renamed)?;
    }
    // Commit only after Tailscale confirms the final hostname and identity.
    store_persisted_state(&state)?;
    Ok(state)
}

pub(crate) fn validate_persisted_member_controller(
    state: &crate::infrastructure::state::PersistedState,
    identity: &TailscaleIdentity,
) -> Result<(), GateError> {
    let peer = identity
        .host_peers
        .iter()
        .find(|peer| peer.id == state.controller.id);
    let Some(peer) = peer else {
        return Err(GateError::new(
            "CONTROLLER_UNAVAILABLE",
            Component::Tailscale,
            "the persisted controller is absent from GNX discovery",
        ));
    };
    if peer.hostname != state.controller.hostname || peer.ip != state.controller.ip {
        return Err(GateError::new(
            "TAILSCALE_IDENTITY_CHANGED",
            Component::Tailscale,
            "the persisted controller identity changed",
        ));
    }
    if !peer.online {
        return Err(GateError::new(
            "CONTROLLER_UNAVAILABLE",
            Component::Tailscale,
            "the persisted controller is offline",
        ));
    }
    if !peer.direct {
        return Err(GateError::new(
            "TAILSCALE_DIRECT_PATH_REQUIRED",
            Component::Tailscale,
            "the persisted controller path is relayed or unavailable",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TopologyDecision {
    Controller,
    Member(HostPeer),
}

pub(crate) fn select_topology(identity: &TailscaleIdentity) -> Result<TopologyDecision, GateError> {
    // host_peers is controller-only and sorted by stable Tailscale node ID.
    // Any controller means this node is a member; the number of members is irrelevant.
    match identity
        .host_peers
        .iter()
        .filter(|peer| peer.online && peer.hostname.starts_with("gnx-controller-"))
        .min_by(|left, right| left.id.cmp(&right.id))
    {
        Some(controller) => Ok(TopologyDecision::Member(controller.clone())),
        None => Ok(TopologyDecision::Controller),
    }
}

pub(crate) fn reconcile_persisted_identity(
    mut state: crate::infrastructure::state::PersistedState,
    identity: &TailscaleIdentity,
) -> Result<(crate::infrastructure::state::PersistedState, bool), GateError> {
    if state.self_ip != identity.self_ip || persisted_local_hostname(&state)? != identity.hostname {
        return Err(GateError::new(
            "TAILSCALE_IDENTITY_CHANGED",
            Component::Tailscale,
            "current Tailscale IP or hostname does not match persisted local state",
        ));
    }

    let node_id_rotated = state.self_id != identity.self_id;
    if node_id_rotated {
        state.self_id.clone_from(&identity.self_id);
        match &state.role {
            crate::infrastructure::state::PersistedRole::Controller => {
                state.controller.id.clone_from(&identity.self_id);
            }
            crate::infrastructure::state::PersistedRole::Member => {
                let member = state.member.as_mut().ok_or_else(|| {
                    GateError::new(
                        "STATE_IDENTITY_INVALID",
                        Component::Tailscale,
                        "persisted member state has no local member identity",
                    )
                })?;
                member.id.clone_from(&identity.self_id);
            }
        }
    }
    Ok((state, node_id_rotated))
}

pub(crate) fn validate_state_identity(
    state: &crate::infrastructure::state::PersistedState,
    identity: &TailscaleIdentity,
) -> Result<(), GateError> {
    if state.self_id != identity.self_id
        || state.self_ip != identity.self_ip
        || persisted_local_hostname(state)? != identity.hostname
    {
        return Err(GateError::new(
            "TAILSCALE_IDENTITY_CHANGED",
            Component::Tailscale,
            "current Tailscale identity does not match persisted local state",
        ));
    }
    Ok(())
}

pub(crate) fn persisted_local_hostname(
    state: &crate::infrastructure::state::PersistedState,
) -> Result<&str, GateError> {
    match &state.role {
        crate::infrastructure::state::PersistedRole::Controller => Ok(&state.controller.hostname),
        crate::infrastructure::state::PersistedRole::Member => state
            .member
            .as_ref()
            .map(|member| member.hostname.as_str())
            .ok_or_else(|| {
                GateError::new(
                    "STATE_IDENTITY_INVALID",
                    Component::Tailscale,
                    "persisted member state has no local member identity",
                )
            }),
    }
}

pub(crate) fn controller_hostname(self_id: &str) -> Result<String, GateError> {
    let suffix = self_id.to_ascii_lowercase();
    if suffix.is_empty()
        || suffix.len() > 47
        || suffix.starts_with('-')
        || suffix.ends_with('-')
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(GateError::new(
            "TAILSCALE_IDENTITY_INVALID",
            Component::Tailscale,
            "Tailscale Self.ID cannot form the fixed controller hostname",
        ));
    }
    Ok(format!("gnx-controller-{suffix}"))
}

pub(crate) fn member_hostname(self_id: &str) -> Result<String, GateError> {
    controller_hostname(self_id)
        .map(|hostname| hostname.replacen("gnx-controller-", "gnx-member-", 1))
}

pub(crate) fn rename_tailscale(podman: &Path, hostname: &str) -> Result<(), GateError> {
    let input = format!("{hostname}\n");
    let output = runtime_agent(podman, RuntimeOperation::TailscaleRename, input.as_bytes())
        .map_err(|error| error.with_code("TAILSCALE_RENAME_FAILED", Component::Tailscale))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "TAILSCALE_HOSTNAME=updated" {
        return Err(GateError::new(
            "TAILSCALE_RENAME_FAILED",
            Component::Tailscale,
            "Tailscale sidecar did not confirm the persistent hostname transition",
        ));
    }
    Ok(())
}
