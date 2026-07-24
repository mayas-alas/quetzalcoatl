use super::*;

pub(super) fn run(status: &Arc<RwLock<StatusResponse>>) -> Result<(), GateError> {
    let profile = validate_identity()?;

    set_stage(status, "WSL_PREPARING");
    configure_wsl(&profile)?;
    set_component(status, Component::Wsl, "ready");

    set_stage(status, "MACHINE_PREPARING");
    let image = load_machine_image()?;
    let podman = podman_binary()?;
    ensure_machine(&podman, &image)?;
    set_stage(status, "MACHINE_NETWORK_PREPARING");
    configure_machine_outer_mtu(&podman)?;
    set_component(status, Component::PodmanMachine, "ready");
    set_stage(status, "MACHINE_READY");

    validate_fedora(&podman)?;

    set_stage(status, "KVM_CHECKING");
    validate_devices(&podman)?;
    set_component(status, Component::Kvm, "ready");
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
    set_component(status, Component::Tailscale, "ready");
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
    set_component(status, Component::Proxmox, "ready");
    set_stage(status, "PROXMOX_READY");
    verify_tailscale_secret_cleanup(&podman)?;

    set_stage(status, "PVE_CREDENTIAL_APPLYING");
    configure_pve_password(&podman, &configuration.pve_root_password)?;
    configuration.pve_root_password.zeroize();

    set_stage(status, "TAILSCALE_SERVE_CHECKING");
    wait_for_tailscale_serve(
        &podman,
        persisted_local_hostname(&controller)?,
        &configuration.tailnet,
    )?;
    set_component(status, Component::TailscaleServe, "ready");
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
