use std::sync::{Arc, RwLock};

use gnx_contracts::{ComponentHealth, StatusResponse};
use zeroize::Zeroize;

use super::status::{
    complete, set_cluster_ready, set_component, set_controller, set_pve_url, set_stage,
};
use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;
use crate::domain::topology::{
    controller_stage_rank, join_member_cluster, load_persisted_state, persisted_local_hostname,
    resolve_controller, store_persisted_state, validate_state_configuration,
    wait_for_configuration,
};
use crate::infrastructure::host::{configure_wsl, validate_identity};
use crate::infrastructure::host_profile::load_host_profile;
use crate::infrastructure::payload::{apply_runtime_payload, load_machine_image};
use crate::infrastructure::podman::{
    configure_machine_outer_mtu, configure_pod_network_mtu, ensure_machine, validate_devices,
    validate_fedora,
};
use crate::infrastructure::proxmox::{
    configure_pve_password, confirm_empty_controller_inventory, create_controller_cluster,
    prepare_pve_identity, start_proxmox, validate_proxmox_devices, verify_controller_cluster,
    verify_pve_identity, wait_for_proxmox,
};
use crate::infrastructure::remote::{podman_binary, verify_runtime_agent};
use crate::infrastructure::tailscale::{
    apply_tailscale_serve, candidate_hostname, disable_tailscale_ssh, prepare_tailscale,
    pve_https_url, start_tailscale, verify_tailscale_secret_cleanup, wait_for_tailscale,
    wait_for_tailscale_serve,
};

pub(super) fn run(status: &Arc<RwLock<StatusResponse>>) -> Result<(), GateError> {
    let service_profile = validate_identity()?;

    set_stage(status, "HOST_PROFILE_LOADING");
    let host_profile = load_host_profile()?;

    set_stage(status, "WSL_PREPARING");
    configure_wsl(&service_profile, &host_profile)?;
    set_component(status, Component::Wsl, ComponentHealth::Ready);

    set_stage(status, "MACHINE_PREPARING");
    let image = load_machine_image()?;
    let podman = podman_binary()?;
    ensure_machine(&podman, &image, &host_profile.selected)?;
    set_stage(status, "MACHINE_NETWORK_PREPARING");
    configure_machine_outer_mtu(&podman)?;
    set_component(status, Component::PodmanMachine, ComponentHealth::Ready);
    set_stage(status, "MACHINE_READY");

    validate_fedora(&podman)?;

    set_stage(status, "KVM_CHECKING");
    validate_devices(&podman)?;
    set_component(status, Component::Kvm, ComponentHealth::Ready);
    set_stage(status, "KVM_READY");

    set_stage(status, "PAYLOAD_APPLYING");
    apply_runtime_payload(&podman)?;
    verify_runtime_agent(&podman)?;

    set_stage(status, "CONFIGURATION_WAITING");
    let mut configuration = wait_for_configuration()?;
    let persisted_state = load_persisted_state()?;
    if let Some(state) = persisted_state.as_ref() {
        validate_state_configuration(state, &configuration)?;
    }

    let hostname = match persisted_state.as_ref() {
        Some(state) => persisted_local_hostname(state)?.to_owned(),
        None => candidate_hostname()?,
    };
    set_stage(status, "TAILSCALE_ENROLLING");
    prepare_tailscale(&podman, &hostname, &configuration.auth_key)?;
    configuration.auth_key.zeroize();
    start_tailscale(&podman)?;

    set_stage(status, "TAILSCALE_CHECKING");
    let identity = wait_for_tailscale(&podman, &hostname, &configuration.tailnet)?;
    disable_tailscale_ssh(&podman)?;
    set_stage(status, "ROLE_DISCOVERING");
    let mut controller =
        resolve_controller(&podman, persisted_state, identity, &configuration.tailnet)?;
    if controller.role.is_controller() {
        set_controller(status, &controller.controller.hostname);
    }
    set_component(status, Component::Tailscale, ComponentHealth::Ready);
    set_stage(status, "ROLE_RESOLVED");

    set_stage(status, "PVE_IDENTITY_PREPARING");
    prepare_pve_identity(
        &podman,
        controller.self_ip,
        persisted_local_hostname(&controller)?,
    )?;

    set_stage(status, "PROXMOX_STARTING");
    start_proxmox(&podman)?;
    set_stage(status, "POD_NETWORK_PREPARING");
    configure_pod_network_mtu(&podman)?;
    set_stage(status, "PROXMOX_CHECKING");
    validate_proxmox_devices(&podman)?;
    wait_for_proxmox(&podman)?;
    set_component(status, Component::Proxmox, ComponentHealth::Ready);
    set_stage(status, "PROXMOX_READY");
    verify_tailscale_secret_cleanup(&podman)?;

    set_stage(status, "PVE_CREDENTIAL_APPLYING");
    configure_pve_password(&podman, &configuration.pve_root_password)?;
    configuration.pve_root_password.zeroize();

    let local_hostname = persisted_local_hostname(&controller)?;
    set_stage(status, "TAILSCALE_SERVE_APPLYING");
    apply_tailscale_serve(&podman, local_hostname, &configuration.tailnet)?;
    set_stage(status, "TAILSCALE_SERVE_CHECKING");
    wait_for_tailscale_serve(&podman, local_hostname, &configuration.tailnet)?;
    let pve_url = pve_https_url(local_hostname, &configuration.tailnet)?;
    set_component(status, Component::TailscaleServe, ComponentHealth::Ready);
    set_pve_url(status, &pve_url)?;
    set_stage(status, "TAILSCALE_READY");

    if controller.role.is_member() {
        join_member_cluster(status, &podman, &mut controller)?;
        return Ok(());
    }

    let stage_rank = controller_stage_rank(&controller.stage).ok_or_else(|| {
        GateError::new(
            "STATE_STAGE_UNSUPPORTED",
            Component::None,
            "persisted controller state has an unsupported platform stage",
        )
    })?;
    if stage_rank == 0 {
        set_stage(status, "CONTROLLER_CLUSTER_PRECHECK");
        confirm_empty_controller_inventory(&podman, &controller)?;
    }

    if stage_rank == 0 {
        set_stage(status, "CONTROLLER_CLUSTER_CREATING");
        create_controller_cluster(&podman, controller.self_ip, &controller.controller.hostname)?;
        controller.stage = "CONTROLLER_CLUSTER_READY".into();
        store_persisted_state(&controller)?;
    } else {
        set_stage(status, "CONTROLLER_CLUSTER_CHECKING");
        verify_controller_cluster(&podman, controller.self_ip, &controller.controller.hostname)?;
    }
    set_stage(status, "PVE_IDENTITY_CHECKING");
    verify_pve_identity(&podman, controller.self_ip, &controller.controller.hostname)?;
    set_cluster_ready(status);
    set_stage(status, "CONTROLLER_CLUSTER_READY");

    controller.stage = "READY".into();
    store_persisted_state(&controller)?;
    complete(status);
    Ok(())
}
