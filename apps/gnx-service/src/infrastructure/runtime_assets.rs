use crate::infrastructure::models::PayloadSpec;

pub(crate) const EXPECTED_SERVICE_SID: &str =
    "S-1-5-80-1414281857-1943412974-186110390-2486725240-2230548587";
pub(crate) const MACHINE_NAME: &str = "quetzalcoatl";
pub(crate) const MACHINE_NETWORK_MTU: u32 = 1500;
pub(crate) const RUNTIME_GENERATION: &str = gnx_contracts::RUNTIME_GENERATION;
pub(crate) const RUNTIME_GENERATION_PATH: &str = "/etc/quetzalcoatl/runtime-generation";
pub(crate) const TAILSCALE_STATE_PATH: &str =
    "/var/lib/quetzalcoatl/tailscale/host/tailscaled.state";
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub(crate) const RUNTIME_AGENT_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-runtime-agent";

pub(crate) const PAYLOAD_FILES: [PayloadSpec; 12] = [
    PayloadSpec::new(
        "commands/gnx-runtime-agent",
        "/usr/libexec/quetzalcoatl/gnx-runtime-agent",
        "0755",
    ),
    PayloadSpec::new(
        "commands/gnx-proxmox-entrypoint",
        "/usr/libexec/quetzalcoatl/gnx-proxmox-entrypoint",
        "0755",
    ),
    PayloadSpec::new(
        "commands/gnx-pve-configure",
        "/usr/libexec/quetzalcoatl/gnx-pve-configure",
        "0755",
    ),
    PayloadSpec::new(
        "commands/gnx-pve-cluster-create",
        "/usr/libexec/quetzalcoatl/gnx-pve-cluster-create",
        "0755",
    ),
    PayloadSpec::new(
        "commands/gnx-tailscale-prepare",
        "/usr/libexec/quetzalcoatl/gnx-tailscale-prepare",
        "0755",
    ),
    PayloadSpec::new(
        "commands/gnx-tailscale-rename",
        "/usr/libexec/quetzalcoatl/gnx-tailscale-rename",
        "0755",
    ),
    PayloadSpec::new(
        "commands/gnx-tailscale-enroll",
        "/usr/libexec/quetzalcoatl/gnx-tailscale-enroll",
        "0755",
    ),
    PayloadSpec::new(
        "configuration/serve.json",
        "/etc/quetzalcoatl/node/serve.json",
        "0644",
    ),
    PayloadSpec::new(
        "containers/gnx-node.pod",
        "/etc/containers/systemd/gnx-node.pod",
        "0644",
    ),
    PayloadSpec::new(
        "containers/proxmox.container",
        "/etc/containers/systemd/proxmox.container",
        "0644",
    ),
    PayloadSpec::new(
        "containers/tailscaled.container",
        "/etc/containers/systemd/tailscaled.container",
        "0644",
    ),
    PayloadSpec::new(
        "services/gnx-tailscale-enroll.service",
        "/etc/systemd/system/gnx-tailscale-enroll.service",
        "0644",
    ),
];

pub(crate) const FEDORA_PROBE: &str =
    include_str!("../../../../runtime/operations/probes/fedora.sh");
pub(crate) const MACHINE_OUTER_MTU: &str =
    include_str!("../../../../runtime/operations/configure-machine-mtu.sh");
pub(crate) const POD_NETWORK_MTU: &str =
    include_str!("../../../../runtime/operations/configure-pod-network-mtu.sh");
pub(crate) const DEVICE_PROBE: &str =
    include_str!("../../../../runtime/operations/probes/devices.py");
pub(crate) const PAYLOAD_HEREDOC: &str = "__GNX_PAYLOAD_EOF__";
pub(crate) const START_PROXMOX: &str =
    include_str!("../../../../runtime/operations/start-proxmox.sh");
pub(crate) const PVE_READY_PROBE: &str =
    include_str!("../../../../runtime/operations/probes/pve-ready.sh");
pub(crate) const PROXMOX_DIAGNOSTICS: &str =
    include_str!("../../../../runtime/operations/probes/proxmox-diagnostics.sh");
pub(crate) const START_TAILSCALE: &str =
    include_str!("../../../../runtime/operations/start-tailscale.sh");
pub(crate) const TAILSCALE_DIAGNOSTICS: &str =
    include_str!("../../../../runtime/operations/probes/tailscale-diagnostics.sh");
pub(crate) const TAILSCALE_SECRET_CLEANUP_PROBE: &str =
    include_str!("../../../../runtime/operations/probes/tailscale-secret-cleanup.sh");
