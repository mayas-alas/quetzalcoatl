use super::*;

#[test]
fn parses_locked_wsl_machine_image() {
    let manifest = include_bytes!("../../../../runtime/payload/manifest.json");
    let image = parse_machine_image(manifest).expect("machine image pin");
    assert_eq!(image.artifact, MACHINE_IMAGE_ARTIFACT);
    assert_eq!(image.size, MACHINE_IMAGE_SIZE);
    assert_eq!(image.sha256, MACHINE_IMAGE_LAYER);
}

#[test]
fn payload_manifest_matches_all_installed_files() {
    let manifest = include_bytes!("../../../../runtime/payload/manifest.json");
    let files = parse_payload_manifest(manifest).expect("payload file set");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/payload");

    assert_eq!(files.len(), PAYLOAD_FILES.len());
    for file in files {
        let contents = fs::read(root.join(&file.relative_path)).expect("payload file");
        assert_eq!(
            sha256_bytes(&contents),
            file.sha256,
            "{}",
            file.relative_path
        );
    }
}

#[test]
fn payload_contract_mismatch_reports_both_sides() {
    let manifest = include_bytes!("../../../../runtime/payload/manifest.json");
    let mut value: serde_json::Value = serde_json::from_slice(manifest).expect("manifest JSON");
    value["files"].as_array_mut().expect("files array").pop();
    let bytes = serde_json::to_vec(&value).expect("manifest bytes");
    let error = match parse_payload_manifest(&bytes) {
        Ok(_) => panic!("truncated manifest must fail"),
        Err(error) => error,
    };

    assert_eq!(error.code, "RUNTIME_PAYLOAD_INVALID");
    assert!(error.message.contains("service_version=0.1.17"));
    assert!(error.message.contains("expected_files=12"));
    assert!(error.message.contains("manifest_files=11"));
}

#[test]
fn rejects_mutable_or_malformed_digest() {
    assert!(!valid_sha256("quay.io/podman/machine-os:6.0"));
    assert!(!valid_sha256("sha256:ABC"));
}

#[test]
fn parses_podman_inspect_contract() {
    let json = br#"[{"Name":"quetzalcoatl","State":"running","Rootful":true,"Resources":{"CPUs":6,"Memory":8192,"DiskSize":100}}]"#;
    let inspect: Vec<MachineInspect> = serde_json::from_slice(json).expect("inspect JSON");
    assert_eq!(inspect[0].name, MACHINE_NAME);
    assert!(inspect[0].rootful);
    assert_eq!(inspect[0].resources.cpus, 6);
    assert_eq!(inspect[0].resources.memory, 8192);
    assert_eq!(inspect[0].resources.disk_size, 100);
}

#[test]
fn accepts_pinned_tailscale_identity_contract() {
    let json = br#"{
      "BackendState":"Running",
      "TUN":true,
      "TailscaleIPs":["100.100.10.20","fd7a:115c:a1e0::1"],
      "Self":{
        "ID":"node-id-123",
        "HostName":"gnx-candidate-nitro",
        "DNSName":"gnx-candidate-nitro.tetra-balance.ts.net.",
        "Tags":["tag:quetzalcoatl-node"]
      },
      "CurrentTailnet":{
        "MagicDNSSuffix":"tetra-balance.ts.net",
        "MagicDNSEnabled":true
      },
      "CertDomains":["gnx-candidate-nitro.tetra-balance.ts.net"],
      "Peer":{}
    }"#;
    let identity = parse_tailscale_status(json, "gnx-candidate-nitro", "tetra-balance.ts.net")
        .expect("valid identity");
    assert_eq!(identity.self_id, "node-id-123");
    assert_eq!(
        identity.self_ip,
        "100.100.10.20".parse::<IpAddr>().expect("IP")
    );
    assert!(identity.host_peers.is_empty());

    let unhealthy = String::from_utf8(json.to_vec())
        .expect("status JSON")
        .replace(
            "\"BackendState\":\"Running\"",
            "\"BackendState\":\"Running\",\"Health\":[\"warning\"]",
        );
    assert!(
        parse_tailscale_status(
            unhealthy.as_bytes(),
            "gnx-candidate-nitro",
            "tetra-balance.ts.net"
        )
        .is_err()
    );
}

#[test]
fn discovery_excludes_offline_expired_member_and_unmanaged_peers() {
    let json = br#"{
      "BackendState":"Running",
      "TUN":true,
      "TailscaleIPs":["100.100.10.20"],
      "Self":{
        "ID":"node-id-self",
        "HostName":"gnx-candidate-nitro",
        "DNSName":"gnx-candidate-nitro.tetra-balance.ts.net.",
        "Tags":["tag:quetzalcoatl-node"]
      },
      "CurrentTailnet":{
        "MagicDNSSuffix":"tetra-balance.ts.net",
        "MagicDNSEnabled":true
      },
      "CertDomains":["gnx-candidate-nitro.tetra-balance.ts.net"],
        "Peer":{
          "peer-key-self":{
            "ID":"node-id-self",
            "HostName":"gnx-member-self",
            "DNSName":"gnx-member-self.tetra-balance.ts.net.",
            "Tags":["tag:quetzalcoatl-node"],
            "Online":true,
            "TailscaleIPs":["100.100.10.20"],
            "Expired":false
          },
          "peer-key-host":{
          "ID":"node-id-host",
          "HostName":"gnx-controller-existing",
          "DNSName":"gnx-controller-existing.tetra-balance.ts.net.",
          "Tags":["tag:quetzalcoatl-node"],
          "Online":false,
          "TailscaleIPs":["100.100.10.21"],
          "CurAddr":"100.100.10.21:41641",
          "Relay":"dfw",
          "Expired":false
        },
          "peer-key-service":{
          "ID":"node-id-service",
          "HostName":"gnx-unmanaged-existing",
          "DNSName":"gnx-unmanaged-existing.tetra-balance.ts.net.",
          "Tags":["tag:other"],
          "Online":true,
            "Expired":false
          },
          "peer-key-extra-tag":{
            "ID":"node-id-extra-tag",
            "HostName":"gnx-member-extra-tag",
            "DNSName":"gnx-member-extra-tag.tetra-balance.ts.net.",
            "Tags":["tag:quetzalcoatl-node","tag:other"],
            "Online":true,
            "TailscaleIPs":["100.100.10.23"],
            "Expired":false
          },
        "peer-key-expired":{
          "ID":"node-id-expired",
          "HostName":"gnx-controller-expired",
          "DNSName":"gnx-controller-expired.tetra-balance.ts.net.",
          "Tags":["tag:quetzalcoatl-node"],
          "Online":false,
          "Expired":true
        }
      }
    }"#;
    let identity = parse_tailscale_status(json, "gnx-candidate-nitro", "tetra-balance.ts.net")
        .expect("valid discovery status");
    assert!(
        identity.host_peers.is_empty(),
        "offline, expired, member and unmanaged peers must not participate in controller discovery"
    );
}

#[test]
fn topology_matrix_selects_exactly_one_controller_without_a_member_count_limit() {
    let host = |id: &str, hostname: &str| HostPeer {
        id: id.into(),
        hostname: hostname.into(),
        ip: "100.100.10.21".parse().expect("IP"),
        online: true,
        direct: true,
    };
    let identity = |host_peers| TailscaleIdentity {
        self_id: "self".into(),
        self_ip: "100.100.10.20".parse().expect("IP"),
        hostname: "gnx-candidate".into(),
        host_peers,
    };

    assert_eq!(
        select_topology(&identity(vec![])).expect("first host"),
        TopologyDecision::Controller
    );
    assert_eq!(
        select_topology(&identity(vec![host("controller", "gnx-controller-a")]))
            .expect("second host"),
        TopologyDecision::Member(host("controller", "gnx-controller-a"))
    );
    assert_eq!(
        select_topology(&identity(vec![host("member", "gnx-member-a")]))
            .expect("members do not block controller promotion"),
        TopologyDecision::Controller
    );
    assert_eq!(
        select_topology(&identity(vec![
            host("a", "gnx-controller-a"),
            host("b", "gnx-controller-b")
        ]))
        .expect("existing controller deterministically selects member role"),
        TopologyDecision::Member(host("a", "gnx-controller-a"))
    );
    assert_eq!(
        select_topology(&identity(vec![
            host("a", "gnx-controller-a"),
            host("b", "gnx-member-b"),
            host("c", "gnx-member-c"),
            host("d", "gnx-member-d"),
        ]))
        .expect("additional member"),
        TopologyDecision::Member(host("a", "gnx-controller-a"))
    );
}

#[test]
fn member_hostname_is_stable_and_never_uses_an_ordinal() {
    assert_eq!(
        member_hostname("nAbC123").expect("member hostname"),
        "gnx-member-nabc123"
    );
    assert!(member_hostname("invalid/id").is_err());
}

#[test]
fn discovered_hostnames_require_an_exact_controller_or_member_shape() {
    assert!(valid_discovered_hostname("gnx-controller-node-123"));
    assert!(valid_discovered_hostname("gnx-member-node-123"));
    for hostname in [
        "gnx-controller-",
        "gnx-member-",
        "gnx-controller--node",
        "gnx-worker-node",
        "controller-node",
        "gnx-member-node_123",
    ] {
        assert!(
            !valid_discovered_hostname(hostname),
            "accepted invalid hostname {hostname:?}"
        );
    }
}

#[test]
fn member_retry_requires_the_pinned_controller_to_remain_direct_and_available() {
    let controller = crate::state::ControllerIdentity {
        id: "controller".into(),
        hostname: "gnx-controller-a".into(),
        ip: "100.100.10.21".parse().expect("IP"),
    };
    let state = crate::state::PersistedState::member(
        "member".into(),
        "100.100.10.22".parse().expect("IP"),
        "gnx-member-member".into(),
        controller,
        "tetra-balance.ts.net".into(),
    );
    let inventory = |online, direct| TailscaleIdentity {
        self_id: "member".into(),
        self_ip: "100.100.10.22".parse().expect("IP"),
        hostname: "gnx-member-member".into(),
        host_peers: vec![
            HostPeer {
                id: "controller".into(),
                hostname: "gnx-controller-a".into(),
                ip: "100.100.10.21".parse().expect("IP"),
                online,
                direct,
            },
            HostPeer {
                id: "replacement".into(),
                hostname: "gnx-controller-replacement".into(),
                ip: "100.100.10.23".parse().expect("IP"),
                online: true,
                direct: true,
            },
        ],
    };
    assert!(validate_persisted_member_controller(&state, &inventory(true, true)).is_ok());
    let mut missing = inventory(true, true);
    missing
        .host_peers
        .retain(|peer| peer.id != state.controller.id);
    assert!(
        matches!(validate_persisted_member_controller(&state, &missing), Err(error) if error.code == "CONTROLLER_UNAVAILABLE")
    );
    assert!(
        matches!(validate_persisted_member_controller(&state, &inventory(false, true)), Err(error) if error.code == "CONTROLLER_UNAVAILABLE")
    );
    assert!(
        matches!(validate_persisted_member_controller(&state, &inventory(true, false)), Err(error) if error.code == "TAILSCALE_DIRECT_PATH_REQUIRED")
    );
}

#[test]
fn initial_decision_ignores_an_offline_controller() {
    let controller = HostPeer {
        id: "controller".into(),
        hostname: "gnx-controller-controller".into(),
        ip: "100.100.10.21".parse().expect("IP"),
        online: false,
        direct: false,
    };
    let identity = TailscaleIdentity {
        self_id: "new-node".into(),
        self_ip: "100.100.10.22".parse().expect("IP"),
        hostname: "gnx-candidate".into(),
        host_peers: vec![controller],
    };

    assert!(matches!(
        select_topology(&identity).expect("topology decision"),
        TopologyDecision::Controller
    ));
}

#[test]
fn member_configuration_matches_only_the_persisted_tailnet() {
    let state = crate::state::PersistedState::member(
        "member".into(),
        "100.100.10.22".parse().expect("IP"),
        "gnx-member-member".into(),
        crate::state::ControllerIdentity {
            id: "controller".into(),
            hostname: "gnx-controller-controller".into(),
            ip: "100.100.10.21".parse().expect("IP"),
        },
        "tetra-balance.ts.net".into(),
    );
    let configuration = InstallerConfiguration {
        tailnet: "tetra-balance.ts.net".into(),
        auth_key: "unused".into(),
        pve_root_password: "unused".into(),
    };

    assert!(validate_state_configuration(&state, &configuration).is_ok());
    let mut wrong_tailnet = configuration;
    wrong_tailnet.tailnet = "other-tailnet.ts.net".into();
    assert!(matches!(
        validate_state_configuration(&state, &wrong_tailnet),
        Err(error) if error.code == "STATE_CONFIGURATION_MISMATCH"
    ));
}

#[test]
fn member_stage_status_keeps_local_components_ready() {
    let status = Arc::new(RwLock::new(StatusResponse::service_ready()));
    {
        let mut value = status.write().expect("status lock");
        value.components.wsl = "ready".into();
        value.components.podman_machine = "ready".into();
        value.components.kvm = "ready".into();
        value.components.tailscale = "ready".into();
        value.components.tailscale_serve = "ready".into();
        value.components.proxmox = "ready".into();
    }

    set_member_stage_status(&status, "gnx-controller-controller", "MEMBER_JOINING");

    let value = status.read().expect("status lock");
    assert_eq!(value.role.as_deref(), Some("member"));
    assert_eq!(
        value.controller.as_deref(),
        Some("gnx-controller-controller")
    );
    assert_eq!(value.stage, "MEMBER_JOINING");
    assert_eq!(value.components.proxmox, "ready");
}

#[test]
fn member_join_input_is_exactly_five_newline_delimited_values_and_zeroizable() {
    let member = crate::state::PersistedState::member(
        "member".into(),
        "100.100.10.22".parse().expect("IP"),
        "gnx-member-member".into(),
        crate::state::ControllerIdentity {
            id: "controller".into(),
            hostname: "gnx-controller-controller".into(),
            ip: "100.100.10.21".parse().expect("IP"),
        },
        "tetra-balance.ts.net".into(),
    );
    let mut input = member_join_input(&member, "gnx-member-member", "test-credential");
    assert_eq!(
        input.as_slice(),
        b"100.100.10.21\ngnx-controller-controller\n100.100.10.22\ngnx-member-member\ntest-credential\n"
    );
    input.zeroize();
    assert!(input.iter().all(|byte| *byte == 0));
}

#[test]
fn member_join_maps_payload_errors_without_retaining_payload_output() {
    assert_eq!(
        map_member_join_error(b"PVE_JOIN_MTU_UNUSABLE").code,
        "CLUSTER_NETWORK_PREFLIGHT_FAILED"
    );
    assert_eq!(
        map_member_join_error(b"PVE_JOIN_TAILSCALE_RELAYED").code,
        "TAILSCALE_DIRECT_PATH_REQUIRED"
    );
    assert_eq!(
        map_member_join_error(b"PVE_JOIN_TOOL_MISSING").code,
        "PVE_JOIN_FAILED"
    );
    assert_eq!(map_member_join_error(b"unexpected").code, "PVE_JOIN_FAILED");
}

#[test]
fn member_join_checkpoint_resumes_without_rediscovery_or_controller_change() {
    let controller = crate::state::ControllerIdentity {
        id: "controller".into(),
        hostname: "gnx-controller-controller".into(),
        ip: "100.100.10.21".parse().expect("IP"),
    };
    let mut member = crate::state::PersistedState::member(
        "member".into(),
        "100.100.10.22".parse().expect("IP"),
        "gnx-member-member".into(),
        controller.clone(),
        "tetra-balance.ts.net".into(),
    );
    assert!(begin_member_join(&mut member).expect("transition to joining"));
    assert_eq!(member.cluster_join, crate::state::ClusterJoinState::Joining);
    assert_eq!(member.stage, "MEMBER_PREPARING");
    assert!(!begin_member_join(&mut member).expect("resume joining"));
    assert_eq!(member.controller, controller);

    member.cluster_join = crate::state::ClusterJoinState::Joined;
    member.stage = "READY".into();
    assert!(begin_member_join(&mut member).expect("resume joined verification"));
    assert_eq!(member.cluster_join, crate::state::ClusterJoinState::Joining);
    assert_eq!(member.stage, "MEMBER_PREPARING");
    assert_eq!(member.controller, controller);
}

#[test]
fn member_ready_status_uses_the_final_member_contract() {
    let status = Arc::new(RwLock::new(StatusResponse::service_ready()));
    set_member_ready_status(&status, "gnx-controller-controller");
    let value = status.read().expect("status lock");
    assert_eq!(value.stage, "READY");
    assert_eq!(value.overall, "ready");
    assert_eq!(value.role.as_deref(), Some("member"));
    assert!(value.cluster.joined);
    assert!(value.cluster.quorate);
}

#[test]
fn controller_hostname_is_derived_from_the_stable_node_id() {
    assert_eq!(
        controller_hostname("nAbC123").expect("controller hostname"),
        "gnx-controller-nabc123"
    );
    assert!(controller_hostname("invalid/id").is_err());
}

#[test]
fn reconciles_reauthenticated_node_id_only_with_logical_identity_continuity() {
    let state = crate::state::PersistedState::controller(
        "node-id-before".into(),
        "100.100.10.20".parse().expect("IP"),
        "gnx-controller-node-id-before".into(),
        "tetra-balance.ts.net".into(),
    );
    let identity = TailscaleIdentity {
        self_id: "node-id-after".into(),
        self_ip: "100.100.10.20".parse().expect("IP"),
        hostname: "gnx-controller-node-id-before".into(),
        host_peers: Vec::new(),
    };

    let (rotated, changed) =
        reconcile_persisted_identity(state.clone(), &identity).expect("node ID rotation");
    assert!(changed);
    assert_eq!(rotated.self_id, "node-id-after");
    assert_eq!(rotated.controller.id, "node-id-after");
    assert_eq!(rotated.controller.hostname, state.controller.hostname);
    assert_eq!(rotated.self_ip, state.self_ip);

    let mut changed_ip = identity.clone();
    changed_ip.self_ip = "100.100.10.21".parse().expect("IP");
    let error =
        reconcile_persisted_identity(state.clone(), &changed_ip).expect_err("IP drift must fail");
    assert_eq!(error.code, "TAILSCALE_IDENTITY_CHANGED");

    let mut changed_hostname = identity;
    changed_hostname.hostname = "gnx-controller-other".into();
    let error = reconcile_persisted_identity(state, &changed_hostname)
        .expect_err("hostname drift must fail");
    assert_eq!(error.code, "TAILSCALE_IDENTITY_CHANGED");
}

#[test]
fn reconciles_reauthenticated_member_without_mutating_the_pinned_controller() {
    let controller = crate::state::ControllerIdentity {
        id: "controller-id".into(),
        hostname: "gnx-controller-controller-id".into(),
        ip: "100.100.10.20".parse().expect("IP"),
    };
    let state = crate::state::PersistedState::member(
        "member-id-before".into(),
        "100.100.10.21".parse().expect("IP"),
        "gnx-member-member-id-before".into(),
        controller.clone(),
        "tetra-balance.ts.net".into(),
    );
    let identity = TailscaleIdentity {
        self_id: "member-id-after".into(),
        self_ip: "100.100.10.21".parse().expect("IP"),
        hostname: "gnx-member-member-id-before".into(),
        host_peers: Vec::new(),
    };

    let (reconciled, changed) =
        reconcile_persisted_identity(state.clone(), &identity).expect("member re-authentication");
    assert!(changed);
    assert!(reconciled.role.is_member());
    assert_eq!(reconciled.stage, state.stage);
    assert_eq!(reconciled.self_id, "member-id-after");
    assert_eq!(
        reconciled.member.as_ref().expect("member identity").id,
        "member-id-after"
    );
    assert_eq!(reconciled.controller, controller);
    assert_eq!(reconciled.self_ip, state.self_ip);
    assert_eq!(
        persisted_local_hostname(&reconciled).expect("local hostname"),
        "gnx-member-member-id-before"
    );
}

#[test]
fn builds_and_accepts_only_the_fixed_pve_serve_route() {
    let host_port = "gnx-controller-nitro.tetra-balance.ts.net:443";
    let json = tailscale_serve_config("gnx-controller-nitro", "tetra-balance.ts.net")
        .expect("fixed Serve config");
    assert!(parse_serve_status(&json, host_port).is_ok());
    assert!(!String::from_utf8_lossy(&json).contains("TS_CERT_DOMAIN"));
}

#[test]
fn runtime_agent_operations_have_fixed_argument_vectors() {
    assert_eq!(RuntimeOperation::Ping.argv(), &["ping"]);
    assert_eq!(
        RuntimeOperation::PveClusterPrepare.argv(),
        &["pve-cluster-create", "prepare"]
    );
    assert_eq!(
        RuntimeOperation::PveClusterVerifyNode.argv(),
        &["pve-cluster-create", "verify-node"]
    );
    assert_eq!(
        RuntimeOperation::PveClusterCreate.argv(),
        &["pve-cluster-create", "create"]
    );
    assert_eq!(
        RuntimeOperation::PveClusterVerify.argv(),
        &["pve-cluster-create", "verify"]
    );
    assert_eq!(
        RuntimeOperation::PveClusterJoin.argv(),
        &["pve-cluster-create", "join"]
    );
    assert_eq!(
        RuntimeOperation::PveClusterConfirmMember.argv(),
        &["pve-cluster-create", "confirm-member"]
    );
    assert_eq!(RuntimeOperation::PveConfigure.argv(), &["pve-configure"]);
    assert_eq!(
        RuntimeOperation::TailscalePrepare.argv(),
        &["tailscale-prepare"]
    );
    assert_eq!(
        RuntimeOperation::TailscaleRename.argv(),
        &["tailscale-rename"]
    );
}
