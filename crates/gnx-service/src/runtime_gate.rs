use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::IpAddr;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use gnx_protocol::{InstallerConfiguration, StatusResponse};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

const EXPECTED_SERVICE_SID: &str = "S-1-5-80-1414281857-1943412974-186110390-2486725240-2230548587";
const MACHINE_NAME: &str = "quetzalcoatl";
const MACHINE_CPUS: u64 = 6;
const MACHINE_MEMORY_MIB: u64 = 8192;
const MACHINE_DISK_GIB: u64 = 100;
const MACHINE_NETWORK_MTU: u32 = 1500;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const LXC_PREPARE_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-lxc-prepare";
const LXC_SERVICE_PREPARE_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-lxc-service-prepare";
const FORGEJO_CONFIGURE_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-forgejo-configure";
const OPENTOFU_PREPARE_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-opentofu-prepare";
const PVE_CLUSTER_CREATE_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-pve-cluster-create";
const PVE_CONFIGURE_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-pve-configure";
const TAILSCALE_PREPARE_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-tailscale-prepare";
const TAILSCALE_RENAME_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-tailscale-rename";
const MACHINE_IMAGE_INDEX: &str =
    "sha256:6dec5eadc84f41e55c3b6fc67264ed6c985e5f61a1d4ba243056dc0efc234bec";
const MACHINE_IMAGE_MANIFEST: &str =
    "sha256:c1b05f0f5f5cdbbfb2be4e23fccfbd0436f3aa6bfa6d4705daed00a251b03943";
const MACHINE_IMAGE_LAYER: &str =
    "sha256:0d828beef16a031a50a7cee594fd79ade36c3d3972b590cb01c32a987bd88bc3";
const MACHINE_IMAGE_COMMIT: &str = "137982aea62947e436bfb58408676e246414ea47";
const MACHINE_IMAGE_ARTIFACT: &str = "podman-machine.x86_64.wsl.tar.zst";
const MACHINE_IMAGE_URL: &str = "https://github.com/podman-container-tools/podman-machine-os/releases/download/v6.0.1/podman-machine.x86_64.wsl.tar.zst";
const MACHINE_IMAGE_SIZE: u64 = 249_510_008;

const PAYLOAD_FILES: [PayloadSpec; 32] = [
    PayloadSpec::new(
        "bin/gnx-proxmox-entrypoint",
        "/usr/libexec/quetzalcoatl/gnx-proxmox-entrypoint",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-lxc-docker-bootstrap",
        "/usr/libexec/quetzalcoatl/gnx-lxc-docker-bootstrap",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-lxc-prepare",
        "/usr/libexec/quetzalcoatl/gnx-lxc-prepare",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-lxc-service-bootstrap",
        "/usr/libexec/quetzalcoatl/gnx-lxc-service-bootstrap",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-lxc-service-prepare",
        "/usr/libexec/quetzalcoatl/gnx-lxc-service-prepare",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-forgejo-configure",
        "/usr/libexec/quetzalcoatl/gnx-forgejo-configure",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-forgejo-configure-guest",
        "/usr/libexec/quetzalcoatl/gnx-forgejo-configure-guest",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-pve-configure",
        "/usr/libexec/quetzalcoatl/gnx-pve-configure",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-pve-cluster-create",
        "/usr/libexec/quetzalcoatl/gnx-pve-cluster-create",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-opentofu-entrypoint",
        "/usr/libexec/quetzalcoatl/gnx-opentofu-entrypoint",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-opentofu-prepare",
        "/usr/libexec/quetzalcoatl/gnx-opentofu-prepare",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-opentofu-runner",
        "/usr/libexec/quetzalcoatl/gnx-opentofu-runner",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-tailscale-prepare",
        "/usr/libexec/quetzalcoatl/gnx-tailscale-prepare",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-tailscale-rename",
        "/usr/libexec/quetzalcoatl/gnx-tailscale-rename",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-tailscale-enroll",
        "/usr/libexec/quetzalcoatl/gnx-tailscale-enroll",
        "0755",
    ),
    PayloadSpec::new(
        "config/node/serve.json",
        "/etc/quetzalcoatl/node/serve.json",
        "0644",
    ),
    PayloadSpec::new(
        "quadlet/gnx-node.pod",
        "/etc/containers/systemd/gnx-node.pod",
        "0644",
    ),
    PayloadSpec::new(
        "quadlet/opentofu.image",
        "/etc/containers/systemd/opentofu.image",
        "0644",
    ),
    PayloadSpec::new(
        "quadlet/proxmox.container",
        "/etc/containers/systemd/proxmox.container",
        "0644",
    ),
    PayloadSpec::new(
        "quadlet/tailscaled.container",
        "/etc/containers/systemd/tailscaled.container",
        "0644",
    ),
    PayloadSpec::new(
        "opentofu/controller/.terraform.lock.hcl",
        "/usr/share/quetzalcoatl/opentofu/controller/.terraform.lock.hcl",
        "0600",
    ),
    PayloadSpec::new(
        "opentofu/controller/main.tf",
        "/usr/share/quetzalcoatl/opentofu/controller/main.tf",
        "0600",
    ),
    PayloadSpec::new(
        "opentofu/controller/versions.tf",
        "/usr/share/quetzalcoatl/opentofu/controller/versions.tf",
        "0600",
    ),
    PayloadSpec::new(
        "services/forgejo/compose.yaml",
        "/usr/share/quetzalcoatl/services/forgejo/compose.yaml",
        "0644",
    ),
    PayloadSpec::new(
        "services/forgejo/serve/serve.json",
        "/usr/share/quetzalcoatl/services/forgejo/serve/serve.json",
        "0644",
    ),
    PayloadSpec::new(
        "services/forgejo/git_probe.sh",
        "/usr/share/quetzalcoatl/services/forgejo/git_probe.sh",
        "0755",
    ),
    PayloadSpec::new(
        "services/garage/compose.yaml",
        "/usr/share/quetzalcoatl/services/garage/compose.yaml",
        "0644",
    ),
    PayloadSpec::new(
        "services/garage/garage.toml.template",
        "/usr/share/quetzalcoatl/services/garage/garage.toml.template",
        "0644",
    ),
    PayloadSpec::new(
        "services/garage/s3_probe.py",
        "/usr/share/quetzalcoatl/services/garage/s3_probe.py",
        "0755",
    ),
    PayloadSpec::new(
        "services/garage/serve/serve.json",
        "/usr/share/quetzalcoatl/services/garage/serve/serve.json",
        "0644",
    ),
    PayloadSpec::new(
        "systemd/gnx-opentofu.service",
        "/etc/systemd/system/gnx-opentofu.service",
        "0644",
    ),
    PayloadSpec::new(
        "systemd/gnx-tailscale-enroll.service",
        "/etc/systemd/system/gnx-tailscale-enroll.service",
        "0644",
    ),
];

const WSL_CONFIG: &str = "[wsl2]\nprocessors=6\nmemory=8GB\nswap=2GB\nnestedVirtualization=true\n";

const FEDORA_PROBE: &str = r#"set -eu
test "$(ps -p 1 -o comm= | tr -d ' ')" = systemd
test "$(stat -fc %T /sys/fs/cgroup)" = cgroup2fs
systemctl is-system-running --wait >/dev/null 2>&1 || test "$(systemctl is-system-running)" = degraded
printf 'SYSTEMD=ready;CGROUP=ready\n'
"#;

const MACHINE_OUTER_MTU: &str = r#"set -eu
test -e /sys/class/net/eth0/mtu
ip link set dev eth0 mtu 1500
test "$(cat /sys/class/net/eth0/mtu)" = 1500
printf 'MACHINE_OUTER_MTU=1500\n'
"#;

const POD_NETWORK_MTU: &str = r#"set -eu
bridge=podman0
test -d "/sys/class/net/$bridge/brif"
ip link set dev "$bridge" mtu 1500
members=0
for member_path in "/sys/class/net/$bridge/brif/"*; do
  test -e "$member_path"
  member=${member_path##*/}
  ip link set dev "$member" mtu 1500
  test "$(cat "/sys/class/net/$member/mtu")" = 1500
  members=$((members + 1))
done
test "$members" -ge 1
test "$(cat "/sys/class/net/$bridge/mtu")" = 1500
test "$(podman exec gnx-proxmox cat /sys/class/net/eth0/mtu)" = 1500
printf 'POD_NETWORK_MTU=1500;MEMBERS=%s\n' "$members"
"#;

const DEVICE_PROBE: &str = r#"import array
import fcntl
import os
import stat

for path in ("/dev/kvm", "/dev/net/tun", "/dev/fuse"):
    mode = os.stat(path).st_mode
    if not stat.S_ISCHR(mode):
        raise SystemExit(f"{path} is not a character device")

kvm = os.open("/dev/kvm", os.O_RDWR | os.O_CLOEXEC)
try:
    api = fcntl.ioctl(kvm, 0xAE00, 0)
finally:
    os.close(kvm)
if api != 12:
    raise SystemExit(f"KVM API version is {api}, expected 12")

tun = os.open("/dev/net/tun", os.O_RDWR | os.O_CLOEXEC)
try:
    features = array.array("I", [0])
    fcntl.ioctl(tun, 0x800454CF, features, True)
finally:
    os.close(tun)

fuse = os.open("/dev/fuse", os.O_RDWR | os.O_CLOEXEC)
os.close(fuse)
print("KVM_API_VERSION=12;TUN=ready;FUSE=ready")
"#;

const PAYLOAD_HEREDOC: &str = "__GNX_PAYLOAD_V1_EOF__";

const START_PROXMOX: &str = r#"set -eu
install -d -m 0755 \
  /var/lib/quetzalcoatl/proxmox/vz \
  /var/lib/quetzalcoatl/proxmox/cluster
install -d -m 0755 /run/gnx
date --iso-8601=seconds > /run/gnx/proxmox-started-at
systemctl daemon-reload
systemctl stop \
  tailscaled.service \
  gnx-tailscale-enroll.service \
  proxmox.service \
  gnx-node-pod.service >/dev/null 2>&1 || true
systemctl reset-failed \
  gnx-node-pod.service \
  proxmox.service \
  gnx-tailscale-enroll.service \
  tailscaled.service >/dev/null 2>&1 || true
if ! systemctl start gnx-node-pod.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 30 -u gnx-node-pod.service >&2 || true
  exit 1
fi
if ! systemctl start proxmox.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 30 -u proxmox.service >&2 || true
  exit 1
fi
systemctl is-active --quiet proxmox.service
printf 'PROXMOX_SERVICE=active\n'
"#;

const PVE_READY_PROBE: &str = r#"set -eu
test "$(podman inspect --format '{{.State.Status}}' gnx-proxmox)" = running
podman exec gnx-proxmox sh -eu -c '
  test "$(ps -p 1 -o comm= | tr -d " ")" = systemd
  test "$(stat -fc %T /sys/fs/cgroup)" = cgroup2fs
  systemctl is-active --quiet pve-cluster.service
  systemctl is-active --quiet pvedaemon.service
  systemctl is-active --quiet pveproxy.service
  pvesh get /version --output-format json >/dev/null
'
printf 'PVE=ready;SYSTEMD=ready;CGROUP=ready\n'
"#;

const PROXMOX_DIAGNOSTICS: &str = r#"set -eu
since="$(cat /run/gnx/proxmox-started-at)"
journalctl --no-pager -o cat --since "$since" -u proxmox.service \
  | grep -avE 'image pull|container (create|init|start|died|remove|cleanup)|pod (create|start|stop)' \
  | head -n 60
"#;

const START_TAILSCALE: &str = r#"set -eu
systemctl daemon-reload
systemctl reset-failed gnx-tailscale-enroll.service tailscaled.service >/dev/null 2>&1 || true
if [ ! -s /var/lib/quetzalcoatl/tailscale/host/tailscaled.state ]; then
  systemctl stop gnx-tailscale-enroll.service >/dev/null 2>&1 || true
fi
if ! systemctl start gnx-tailscale-enroll.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 40 -u gnx-tailscale-enroll.service >&2 || true
  exit 1
fi
test ! -e /run/gnx/ts-authkey
if ! systemctl restart tailscaled.service >/dev/null 2>&1; then
  journalctl --no-pager -o cat -r -n 40 -u tailscaled.service >&2 || true
  exit 1
fi
systemctl is-active --quiet tailscaled.service
printf 'TAILSCALE_SERVICE=active\n'
"#;

const TAILSCALE_DIAGNOSTICS: &str = r#"set -eu
journalctl --no-pager -o cat -r -n 30 \
  -u gnx-tailscale-enroll.service -u tailscaled.service 2>/dev/null \
  | head -n 60
"#;

const TAILSCALE_SECRET_CLEANUP_PROBE: &str = r#"set -eu
test ! -e /run/gnx/ts-authkey
test -z "$(podman ps -aq --filter name='^gnx-host-enroll$')"
printf 'TAILSCALE_SECRET_CLEAN=ready\n'
"#;

const OPENTOFU_SECRET_CLEANUP_PROBE: &str = r#"set -eu
test ! -e /run/gnx/opentofu-password
test ! -e /run/gnx/opentofu.env
test -z "$(podman ps -aq --filter name='^gnx-opentofu$')"
printf 'OPENTOFU_SECRET_CLEAN=ready\n'
"#;

pub fn run(status: Arc<RwLock<StatusResponse>>) {
    set_stage(&status, "RUNTIME_IDENTITY");
    if let Err(error) = run_inner(&status) {
        fail(&status, error);
    }
}

pub fn configure_forgejo(username: &str, password: &str) -> Result<(), (String, String)> {
    configure_forgejo_inner(username, password)
        .map_err(|error| (error.code.to_owned(), error.message))
}

fn configure_forgejo_inner(username: &str, password: &str) -> Result<(), GateError> {
    let state = load_persisted_state()?.ok_or_else(|| {
        GateError::new(
            "FORGEJO_CONFIGURATION_UNAVAILABLE",
            Component::Forgejo,
            "the persisted role is not available",
        )
    })?;
    if !state.role.is_controller() || state.stage != "READY" || !state.install_forgejo {
        return Err(GateError::new(
            "FORGEJO_CONFIGURATION_UNAVAILABLE",
            Component::Forgejo,
            "Forgejo configuration requires a ready controller with Forgejo selected",
        ));
    }
    let podman = podman_binary()?;
    let mut secrets =
        crate::service_secrets::load_required(state.install_garage, state.install_forgejo)
            .map_err(|error| {
                GateError::new("FORGEJO_SECRET_FAILED", Component::Forgejo, error.message())
            })?;
    crate::service_secrets::begin_forgejo_update(&mut secrets, username, password).map_err(
        |error| {
            GateError::new(
                "FORGEJO_CONFIGURATION_INVALID",
                Component::Forgejo,
                error.message(),
            )
        },
    )?;
    let forgejo = secrets.forgejo.as_ref().ok_or_else(|| {
        GateError::new(
            "FORGEJO_CONFIGURATION_UNAVAILABLE",
            Component::Forgejo,
            "Forgejo protected credentials are unavailable",
        )
    })?;
    let pending = forgejo.pending_admin.as_ref().ok_or_else(|| {
        GateError::new(
            "FORGEJO_SECRET_FAILED",
            Component::Forgejo,
            "Forgejo pending credential was not persisted",
        )
    })?;
    let mut input = Zeroizing::new(format!(
        "{}\n{}\n{}\n{}\n",
        forgejo.admin_username, forgejo.admin_password, pending.username, pending.password
    ));
    let output = machine_stdin(&podman, [FORGEJO_CONFIGURE_BIN], input.as_bytes())
        .map_err(|error| error.with_code("FORGEJO_CONFIGURATION_FAILED", Component::Forgejo));
    input.zeroize();
    let output = output?;
    if output.stdout != b"FORGEJO_CONFIGURATION=ready\n" {
        return Err(GateError::new(
            "FORGEJO_CONFIGURATION_FAILED",
            Component::Forgejo,
            "Forgejo credential rotation did not confirm the fixed output contract",
        ));
    }
    crate::service_secrets::commit_forgejo_update(&mut secrets).map_err(|error| {
        GateError::new("FORGEJO_SECRET_FAILED", Component::Forgejo, error.message())
    })
}

fn run_inner(status: &Arc<RwLock<StatusResponse>>) -> Result<(), GateError> {
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

    set_stage(status, "CONFIGURATION_WAITING");
    let mut configuration = wait_for_configuration()?;
    let persisted_state = load_persisted_state()?;
    if let Some(state) = persisted_state.as_ref() {
        validate_state_configuration(state, &configuration)?;
    }

    set_stage(status, "PVE_CREDENTIAL_APPLYING");
    configure_pve_password(&podman, &configuration.pve_root_password)?;
    configuration.pve_root_password.zeroize();

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
    let mut controller = resolve_controller(
        &podman,
        persisted_state,
        identity,
        &configuration.tailnet,
        configuration.install_garage,
        configuration.install_forgejo,
    )?;
    if controller.role.is_controller() {
        set_controller(status, &controller.controller.hostname);
    }
    set_component(status, Component::Tailscale, "ready");
    set_stage(status, "ROLE_RESOLVED");

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

    let mut stage_rank = controller_stage_rank(&controller.stage).ok_or_else(|| {
        GateError::new(
            "STATE_STAGE_UNSUPPORTED",
            Component::None,
            "persisted controller state has an unsupported I1 stage",
        )
    })?;
    if stage_rank == 0 {
        set_stage(status, "CONTROLLER_CLUSTER_PRECHECK");
        confirm_empty_controller_inventory(&podman, &controller)?;
    }

    set_stage(status, "CONTROLLER_CLUSTER_CREATING");
    create_controller_cluster(&podman, controller.self_ip, &controller.controller.hostname)?;
    if stage_rank < 1 {
        controller.stage = "CONTROLLER_CLUSTER_READY".into();
        store_persisted_state(&controller)?;
        stage_rank = 1;
    }
    set_cluster_ready(status);
    set_stage(status, "CONTROLLER_CLUSTER_READY");

    set_stage(status, "OPENTOFU_PREPARING");
    let mut infrastructure_configuration = load_protected_configuration()?;
    infrastructure_configuration.auth_key.zeroize();
    validate_state_configuration(&controller, &infrastructure_configuration)?;
    let opentofu_result = apply_opentofu(
        &podman,
        &controller.role,
        &infrastructure_configuration.pve_root_password,
        &controller.controller.hostname,
        controller.install_garage,
        controller.install_forgejo,
    );
    infrastructure_configuration.pve_root_password.zeroize();
    opentofu_result?;
    set_component(status, Component::OpenTofu, "ready");
    if stage_rank < 2 {
        controller.stage = "OPENTOFU_READY".into();
        store_persisted_state(&controller)?;
        stage_rank = 2;
    }
    set_stage(status, "OPENTOFU_READY");

    if controller.install_garage {
        set_stage(status, "GARAGE_LXC_DOCKER_PREPARING");
        prepare_lxc_docker(&podman, ServiceKind::Garage)?;
    }
    if controller.install_forgejo {
        set_stage(status, "FORGEJO_LXC_DOCKER_PREPARING");
        prepare_lxc_docker(&podman, ServiceKind::Forgejo)?;
    }
    if stage_rank < 3 {
        controller.stage = "LXC_DOCKER_READY".into();
        store_persisted_state(&controller)?;
        stage_rank = 3;
    }
    set_stage(status, "LXC_DOCKER_READY");

    set_stage(status, "SERVICE_SECRETS_PREPARING");
    let load_service_secrets = if stage_rank >= 4 {
        crate::service_secrets::load_required
    } else {
        crate::service_secrets::load_or_create
    };
    let mut service_secrets =
        load_service_secrets(controller.install_garage, controller.install_forgejo).map_err(
            |error| GateError::new("SERVICE_SECRETS_FAILED", Component::None, error.message()),
        )?;
    let mut service_configuration = load_protected_configuration()?;
    service_configuration.pve_root_password.zeroize();
    validate_state_configuration(&controller, &service_configuration)?;

    if controller.install_garage {
        set_stage(status, "GARAGE_PREPARING");
        let hostname = service_hostname(ServiceKind::Garage, &controller.controller.hostname)?;
        let credential = prepare_lxc_service(
            &podman,
            ServiceKind::Garage,
            &hostname,
            &controller.tailnet,
            &service_configuration.auth_key,
            &service_secrets,
        )?
        .ok_or_else(|| {
            GateError::new(
                "GARAGE_BOOTSTRAP_FAILED",
                Component::Garage,
                "Garage bootstrap did not return its S3 credential",
            )
        })?;
        crate::service_secrets::record_garage_s3(
            &mut service_secrets,
            &credential.access_key,
            &credential.secret_key,
        )
        .map_err(|error| {
            GateError::new("GARAGE_SECRET_FAILED", Component::Garage, error.message())
        })?;
        set_component(status, Component::Garage, "ready");
    } else {
        set_component(status, Component::Garage, "not_selected");
    }

    if controller.install_forgejo {
        set_stage(status, "FORGEJO_PREPARING");
        let hostname = service_hostname(ServiceKind::Forgejo, &controller.controller.hostname)?;
        let credential = prepare_lxc_service(
            &podman,
            ServiceKind::Forgejo,
            &hostname,
            &controller.tailnet,
            &service_configuration.auth_key,
            &service_secrets,
        )?;
        if credential.is_some() {
            return Err(GateError::new(
                "FORGEJO_BOOTSTRAP_FAILED",
                Component::Forgejo,
                "Forgejo bootstrap returned an unexpected credential",
            ));
        }
        set_component(status, Component::Forgejo, "ready");
    } else {
        set_component(status, Component::Forgejo, "not_selected");
    }
    service_configuration.auth_key.zeroize();

    set_stage(status, "SERVICES_READY");
    if stage_rank < 4 {
        controller.stage = "READY".into();
        store_persisted_state(&controller)?;
    }
    complete(status);
    Ok(())
}

fn validate_identity() -> Result<PathBuf, GateError> {
    let whoami = system_binary("whoami.exe")
        .map_err(|error| error.with_code("RUNTIME_IDENTITY_INVALID", Component::None))?;
    let output = run_command(&whoami, ["/user", "/fo", "csv", "/nh"])
        .map_err(|error| error.with_code("RUNTIME_IDENTITY_INVALID", Component::None))?;
    let identity = String::from_utf8_lossy(&output.stdout);
    if !identity.contains(EXPECTED_SERVICE_SID) {
        return Err(GateError::new(
            "RUNTIME_IDENTITY_INVALID",
            Component::None,
            "service process is not running under NT SERVICE\\Quetzalcoatl",
        ));
    }

    let profile = env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_dir())
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_IDENTITY_INVALID",
                Component::None,
                "SCM did not load an absolute service profile",
            )
        })?;
    Ok(profile)
}

fn configure_wsl(profile: &Path) -> Result<(), GateError> {
    let wsl = system_binary("wsl.exe")
        .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;
    run_command(&wsl, ["--version"])
        .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;

    let config = profile.join(".wslconfig");
    let changed = fs::read(&config).map_or(true, |current| current != WSL_CONFIG.as_bytes());
    if changed {
        fs::write(&config, WSL_CONFIG).map_err(|error| {
            GateError::new(
                "WSL_NESTED_VIRT_FAILED",
                Component::Wsl,
                format!("cannot write managed .wslconfig: {error}"),
            )
        })?;
        if fs::read(&config).ok().as_deref() != Some(WSL_CONFIG.as_bytes()) {
            return Err(GateError::new(
                "WSL_NESTED_VIRT_FAILED",
                Component::Wsl,
                "managed .wslconfig did not round-trip",
            ));
        }
        run_command(&wsl, ["--shutdown"])
            .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;
    }
    Ok(())
}

fn ensure_machine(podman: &Path, image: &MachineImage) -> Result<(), GateError> {
    let list = run_command(podman, ["machine", "list", "--format", "json"])
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
    let machines: Vec<MachineListEntry> =
        serde_json::from_slice(&list.stdout).map_err(|error| {
            GateError::new(
                "MACHINE_CREATE_FAILED",
                Component::PodmanMachine,
                format!("podman machine list returned invalid JSON: {error}"),
            )
        })?;

    if machines.iter().any(|machine| machine.name != MACHINE_NAME) {
        return Err(GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "dedicated runtime identity owns an unexpected Podman machine",
        ));
    }

    match machines.iter().find(|machine| machine.name == MACHINE_NAME) {
        Some(machine) => {
            if machine.vm_type != "wsl" {
                return Err(GateError::new(
                    "MACHINE_CREATE_FAILED",
                    Component::PodmanMachine,
                    "managed machine exists with a provider other than WSL",
                ));
            }
        }
        None => {
            let image_path = installed_machine_image(image)?;
            let cpus = MACHINE_CPUS.to_string();
            let memory = MACHINE_MEMORY_MIB.to_string();
            let disk = MACHINE_DISK_GIB.to_string();
            let args = vec![
                OsString::from("machine"),
                OsString::from("init"),
                OsString::from("--provider"),
                OsString::from("wsl"),
                OsString::from("--image"),
                image_path.into_os_string(),
                OsString::from("--cpus"),
                OsString::from(cpus),
                OsString::from("--memory"),
                OsString::from(memory),
                OsString::from("--disk-size"),
                OsString::from(disk),
                OsString::from("--rootful"),
                OsString::from("--update-connection"),
                OsString::from("--now"),
                OsString::from(MACHINE_NAME),
            ];
            run_command(podman, args).map_err(|error| {
                error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine)
            })?;
        }
    }

    let inspect = inspect_machine(podman)?;
    if inspect.name != MACHINE_NAME
        || !inspect.rootful
        || inspect.resources.cpus != MACHINE_CPUS
        || inspect.resources.memory != MACHINE_MEMORY_MIB
        || inspect.resources.disk_size != MACHINE_DISK_GIB
    {
        return Err(GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "managed machine configuration does not match the fixed runtime profile",
        ));
    }
    if inspect.state != "running" {
        run_command(podman, ["machine", "start", MACHINE_NAME])
            .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
    }
    Ok(())
}

fn installed_machine_image(image: &MachineImage) -> Result<PathBuf, GateError> {
    let executable = env::current_exe().map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot locate gnx-service executable: {error}"),
        )
    })?;
    let path = executable
        .parent()
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                "gnx-service executable has no parent directory",
            )
        })?
        .join("machine-images")
        .join(&image.artifact);
    if !path.is_file() || !verify_artifact(&path, image)? {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            "installed Podman Machine image does not match its locked size and SHA-256",
        ));
    }
    Ok(path)
}

fn verify_artifact(path: &Path, image: &MachineImage) -> Result<bool, GateError> {
    let metadata = fs::metadata(path).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot inspect installed machine image: {error}"),
        )
    })?;
    if metadata.len() != image.size {
        return Ok(false);
    }
    let mut file = File::open(path).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot open installed machine image: {error}"),
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                format!("cannot hash installed machine image: {error}"),
            )
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", digest.finalize()) == image.sha256)
}

fn inspect_machine(podman: &Path) -> Result<MachineInspect, GateError> {
    let output = run_command(podman, ["machine", "inspect", MACHINE_NAME])
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
    let mut machines: Vec<MachineInspect> =
        serde_json::from_slice(&output.stdout).map_err(|error| {
            GateError::new(
                "MACHINE_CREATE_FAILED",
                Component::PodmanMachine,
                format!("podman machine inspect returned invalid JSON: {error}"),
            )
        })?;
    if machines.len() != 1 {
        return Err(GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "podman machine inspect did not return exactly one managed machine",
        ));
    }
    Ok(machines.remove(0))
}

fn validate_fedora(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["sh", "-s"], FEDORA_PROBE.as_bytes())
        .map_err(|error| error.with_code("FEDORA_RUNTIME_UNSUPPORTED", Component::PodmanMachine))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "SYSTEMD=ready;CGROUP=ready" {
        return Err(GateError::new(
            "FEDORA_RUNTIME_UNSUPPORTED",
            Component::PodmanMachine,
            "Fedora probe did not confirm systemd and cgroup v2",
        ));
    }
    Ok(())
}

fn configure_machine_outer_mtu(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["sh", "-s"], MACHINE_OUTER_MTU.as_bytes())
        .map_err(|error| error.with_code("MACHINE_MTU_FAILED", Component::PodmanMachine))?;
    let expected = format!("MACHINE_OUTER_MTU={MACHINE_NETWORK_MTU}");
    if String::from_utf8_lossy(&output.stdout).trim() != expected {
        return Err(GateError::new(
            "MACHINE_MTU_FAILED",
            Component::PodmanMachine,
            "Podman Machine did not confirm the fixed outer MTU",
        ));
    }
    Ok(())
}

fn configure_pod_network_mtu(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["sh", "-s"], POD_NETWORK_MTU.as_bytes())
        .map_err(|error| error.with_code("POD_NETWORK_MTU_FAILED", Component::PodmanMachine))?;
    let confirmation = String::from_utf8_lossy(&output.stdout);
    if !confirmation
        .trim()
        .starts_with(&format!("POD_NETWORK_MTU={MACHINE_NETWORK_MTU};MEMBERS="))
    {
        return Err(GateError::new(
            "POD_NETWORK_MTU_FAILED",
            Component::PodmanMachine,
            "Podman bridge and pod veth did not confirm the fixed MTU",
        ));
    }
    Ok(())
}

fn validate_devices(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["python3", "-"], DEVICE_PROBE.as_bytes())
        .map_err(|error| error.with_code("REQUIRED_DEVICE_MISSING", Component::Kvm))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "KVM_API_VERSION=12;TUN=ready;FUSE=ready" {
        return Err(GateError::new(
            "REQUIRED_DEVICE_MISSING",
            Component::Kvm,
            "device probe did not confirm KVM API 12, TUN and FUSE",
        ));
    }
    Ok(())
}

fn apply_runtime_payload(podman: &Path) -> Result<(), GateError> {
    let files = load_payload_files()
        .map_err(|error| error.with_code("RUNTIME_PAYLOAD_INVALID", Component::Proxmox))?;
    for file in files {
        let script = payload_install_script(&file)?;
        machine_stdin(podman, ["sh", "-s"], &script)
            .map_err(|error| error.with_code("RUNTIME_PAYLOAD_INVALID", Component::Proxmox))?;
    }
    Ok(())
}

fn payload_install_script(file: &PayloadFile) -> Result<Vec<u8>, GateError> {
    let delimiter_present = file
        .contents
        .split(|byte| *byte == b'\n')
        .any(|line| line == PAYLOAD_HEREDOC.as_bytes());
    if file.destination.contains('\'')
        || file.mode.contains('\'')
        || file.sha256.contains('\'')
        || file.contents.contains(&b'\r')
        || file.contents.contains(&0)
        || !file.contents.ends_with(b"\n")
        || delimiter_present
    {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            "payload file cannot be represented by the fixed LF text transport",
        ));
    }

    let mut script = format!(
        "set -eu\ndestination='{}'\nmode='{}'\nexpected='{}'\ndirectory=\"$(dirname \"$destination\")\"\ntemporary=\"${{destination}}.gnx-new\"\ninstall -d -m 0755 \"$directory\"\numask 077\ncat > \"$temporary\" <<'{}'\n",
        file.destination, file.mode, file.sha256, PAYLOAD_HEREDOC
    )
    .into_bytes();
    script.extend_from_slice(&file.contents);
    script.extend_from_slice(
        format!(
            "{}\nchmod \"$mode\" \"$temporary\"\nactual=\"$(sha256sum \"$temporary\" | cut -d ' ' -f 1)\"\ntest \"$actual\" = \"$expected\"\nmv -f \"$temporary\" \"$destination\"\n",
            PAYLOAD_HEREDOC
        )
        .as_bytes(),
    );
    Ok(script)
}

fn start_proxmox(podman: &Path) -> Result<(), GateError> {
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

fn validate_proxmox_devices(podman: &Path) -> Result<(), GateError> {
    let mut last_error = String::from("Proxmox container did not become executable");
    for attempt in 0..30 {
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

fn wait_for_proxmox(podman: &Path) -> Result<(), GateError> {
    let mut last_error = String::from("Proxmox services are not ready");
    for attempt in 0..120 {
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

fn proxmox_diagnostics(podman: &Path) -> String {
    match machine_stdin(podman, ["sh", "-s"], PROXMOX_DIAGNOSTICS.as_bytes()) {
        Ok(output) => bounded_text(&output.stdout),
        Err(error) => format!("diagnostics unavailable: {}", error.message),
    }
}

fn wait_for_configuration() -> Result<InstallerConfiguration, GateError> {
    loop {
        match crate::secrets::load_optional() {
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

fn load_protected_configuration() -> Result<InstallerConfiguration, GateError> {
    match crate::secrets::load_optional() {
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

fn load_persisted_state() -> Result<Option<crate::state::PersistedState>, GateError> {
    crate::state::load_optional()
        .map_err(|error| GateError::new("STATE_STORAGE_FAILED", Component::None, error.message()))
}

fn store_persisted_state(state: &crate::state::PersistedState) -> Result<(), GateError> {
    crate::state::store(state)
        .map_err(|error| GateError::new("STATE_STORAGE_FAILED", Component::None, error.message()))
}

fn controller_stage_rank(stage: &str) -> Option<u8> {
    match stage {
        "ROLE_RESOLVED" => Some(0),
        "CONTROLLER_CLUSTER_READY" => Some(1),
        "OPENTOFU_READY" => Some(2),
        "LXC_DOCKER_READY" => Some(3),
        "READY" => Some(4),
        _ => None,
    }
}

fn validate_state_configuration(
    state: &crate::state::PersistedState,
    configuration: &InstallerConfiguration,
) -> Result<(), GateError> {
    let matches = state.tailnet == configuration.tailnet
        && (state.role.is_member()
            || (state.install_garage == configuration.install_garage
                && state.install_forgejo == configuration.install_forgejo));
    if !matches {
        return Err(GateError::new(
            "STATE_CONFIGURATION_MISMATCH",
            Component::None,
            "persisted state does not match the protected installer inputs",
        ));
    }
    Ok(())
}

fn join_member_cluster(
    status: &Arc<RwLock<StatusResponse>>,
    podman: &Path,
    member: &mut crate::state::PersistedState,
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
    set_member_joining_status(status, &member.controller.hostname);

    let mut configuration = load_protected_configuration()?;
    configuration.auth_key.zeroize();
    let configuration_matches = validate_state_configuration(member, &configuration);
    if configuration_matches.is_err() {
        configuration.pve_root_password.zeroize();
        return configuration_matches;
    }
    let join_result = join_member(podman, member, &configuration.pve_root_password);
    configuration.pve_root_password.zeroize();
    join_result?;

    member.cluster_join = crate::state::ClusterJoinState::Joined;
    member.stage = "READY".into();
    store_persisted_state(member)?;
    set_member_ready_status(status, &member.controller.hostname);
    Ok(())
}

fn begin_member_join(member: &mut crate::state::PersistedState) -> Result<bool, GateError> {
    match member.cluster_join {
        crate::state::ClusterJoinState::NotStarted => {
            member.cluster_join = crate::state::ClusterJoinState::Joining;
            member.stage = "MEMBER_JOINING".into();
            Ok(true)
        }
        crate::state::ClusterJoinState::Joining | crate::state::ClusterJoinState::Joined => {
            Ok(false)
        }
        crate::state::ClusterJoinState::NotApplicable => Err(GateError::new(
            "STATE_IDENTITY_INVALID",
            Component::None,
            "member state has no join checkpoint",
        )),
    }
}

fn join_member(
    podman: &Path,
    member: &crate::state::PersistedState,
    password: &str,
) -> Result<(), GateError> {
    let hostname = persisted_local_hostname(member)?;
    let input = member_join_input(member, hostname, password);
    let result = machine_stdin_output(podman, [PVE_CLUSTER_CREATE_BIN, "join"], &input);
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

fn member_join_input(
    member: &crate::state::PersistedState,
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

fn map_member_join_error(output: &[u8]) -> GateError {
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

fn resolve_controller(
    podman: &Path,
    persisted: Option<crate::state::PersistedState>,
    identity: TailscaleIdentity,
    tailnet: &str,
    install_garage: bool,
    install_forgejo: bool,
) -> Result<crate::state::PersistedState, GateError> {
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

    let identity = stabilize_host_inventory(podman, identity, tailnet)?;
    let decision = select_topology(&identity)?;
    let (state, hostname) = match decision {
        TopologyDecision::Controller => {
            let hostname = controller_hostname(&identity.self_id)?;
            (
                crate::state::PersistedState::controller(
                    identity.self_id.clone(),
                    identity.self_ip,
                    hostname.clone(),
                    tailnet.to_owned(),
                    install_garage,
                    install_forgejo,
                ),
                hostname,
            )
        }
        TopologyDecision::Member(controller) => {
            let hostname = member_hostname(&identity.self_id)?;
            (
                crate::state::PersistedState::member(
                    identity.self_id.clone(),
                    identity.self_ip,
                    hostname.clone(),
                    crate::state::ControllerIdentity {
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
    store_persisted_state(&state)?;

    rename_tailscale(podman, &hostname)?;
    let renamed = wait_for_tailscale(podman, &hostname, tailnet)?;
    validate_state_identity(&state, &renamed)?;
    if state.role.is_member() {
        validate_persisted_member_controller(&state, &renamed)?;
    }
    Ok(state)
}

fn validate_persisted_member_controller(
    state: &crate::state::PersistedState,
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
enum TopologyDecision {
    Controller,
    Member(HostPeer),
}

fn select_topology(identity: &TailscaleIdentity) -> Result<TopologyDecision, GateError> {
    match identity.host_peers.len() {
        0 => Ok(TopologyDecision::Controller),
        1 | 2 => {
            let controllers = identity
                .host_peers
                .iter()
                .filter(|peer| peer.hostname.starts_with("gnx-controller-"))
                .cloned()
                .collect::<Vec<_>>();
            match controllers.as_slice() {
                [] => Err(GateError::new(
                    "TOPOLOGY_UNSUPPORTED",
                    Component::Tailscale,
                    "bounded topology has no identifiable controller",
                )),
                [controller] => Ok(TopologyDecision::Member(controller.clone())),
                _ => Err(GateError::new(
                    "TOPOLOGY_UNSUPPORTED",
                    Component::Tailscale,
                    "bounded topology has multiple identifiable controllers",
                )),
            }
        }
        _ => Err(GateError::new(
            "TOPOLOGY_UNSUPPORTED",
            Component::Tailscale,
            "bounded topology has more than two existing GNX hosts",
        )),
    }
}

fn reconcile_persisted_identity(
    mut state: crate::state::PersistedState,
    identity: &TailscaleIdentity,
) -> Result<(crate::state::PersistedState, bool), GateError> {
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
            crate::state::PersistedRole::Controller => {
                state.controller.id.clone_from(&identity.self_id);
            }
            crate::state::PersistedRole::Member => {
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

fn validate_state_identity(
    state: &crate::state::PersistedState,
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

fn persisted_local_hostname(state: &crate::state::PersistedState) -> Result<&str, GateError> {
    match &state.role {
        crate::state::PersistedRole::Controller => Ok(&state.controller.hostname),
        crate::state::PersistedRole::Member => state
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

fn controller_hostname(self_id: &str) -> Result<String, GateError> {
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

fn member_hostname(self_id: &str) -> Result<String, GateError> {
    controller_hostname(self_id)
        .map(|hostname| hostname.replacen("gnx-controller-", "gnx-member-", 1))
}

fn rename_tailscale(podman: &Path, hostname: &str) -> Result<(), GateError> {
    let input = format!("{hostname}\n");
    let output = machine_stdin(podman, [TAILSCALE_RENAME_BIN], input.as_bytes())
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

fn confirm_empty_controller_inventory(
    podman: &Path,
    state: &crate::state::PersistedState,
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

fn create_controller_cluster(
    podman: &Path,
    self_ip: IpAddr,
    hostname: &str,
) -> Result<(), GateError> {
    let input = format!("{self_ip}\n{hostname}\n");
    let output = machine_stdin(podman, [PVE_CLUSTER_CREATE_BIN], input.as_bytes())
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

fn apply_opentofu(
    podman: &Path,
    role: &crate::state::PersistedRole,
    password: &str,
    hostname: &str,
    install_garage: bool,
    install_forgejo: bool,
) -> Result<(), GateError> {
    if role.is_member() {
        return Err(GateError::new(
            "MEMBER_OPENTOFU_DENIED",
            Component::OpenTofu,
            "OpenTofu execution is denied for members",
        ));
    }
    let mut input = Vec::with_capacity(password.len() + hostname.len() + 8);
    input.extend_from_slice(password.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(hostname.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(if install_garage { b"1\n" } else { b"0\n" });
    input.extend_from_slice(if install_forgejo { b"1\n" } else { b"0\n" });
    let result = machine_stdin(podman, [OPENTOFU_PREPARE_BIN], &input);
    input.zeroize();

    let output = match result {
        Ok(output) => output,
        Err(error) => {
            verify_opentofu_secret_cleanup(podman)?;
            return Err(error.with_code("OPENTOFU_APPLY_FAILED", Component::OpenTofu));
        }
    };
    verify_opentofu_secret_cleanup(podman)?;
    if String::from_utf8_lossy(&output.stdout).trim() != "OPENTOFU=ready" {
        return Err(GateError::new(
            "OPENTOFU_APPLY_FAILED",
            Component::OpenTofu,
            "OpenTofu one-shot did not confirm the controller workspace",
        ));
    }
    Ok(())
}

fn verify_opentofu_secret_cleanup(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(
        podman,
        ["sh", "-s"],
        OPENTOFU_SECRET_CLEANUP_PROBE.as_bytes(),
    )
    .map_err(|error| error.with_code("OPENTOFU_SECRET_CLEANUP_FAILED", Component::OpenTofu))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "OPENTOFU_SECRET_CLEAN=ready" {
        return Err(GateError::new(
            "OPENTOFU_SECRET_CLEANUP_FAILED",
            Component::OpenTofu,
            "OpenTofu did not confirm transient credential cleanup",
        ));
    }
    Ok(())
}

fn prepare_lxc_docker(podman: &Path, service: ServiceKind) -> Result<(), GateError> {
    let input = format!("{}\n", service.name());
    let output = machine_stdin(
        podman,
        ["podman", "exec", "-i", "gnx-proxmox", LXC_PREPARE_BIN],
        input.as_bytes(),
    )
    .map_err(|error| error.with_code("LXC_DOCKER_FAILED", service.component()))?;
    let expected = format!(
        "LXC_DOCKER=ready;SERVICE={};VMID={}",
        service.name(),
        service.vmid()
    );
    if String::from_utf8_lossy(&output.stdout).trim() != expected {
        return Err(GateError::new(
            "LXC_DOCKER_FAILED",
            service.component(),
            "LXC did not confirm the fixed Docker runtime contract",
        ));
    }
    Ok(())
}

fn service_hostname(service: ServiceKind, controller_hostname: &str) -> Result<String, GateError> {
    let suffix = controller_hostname
        .strip_prefix("gnx-controller-")
        .filter(|suffix| {
            !suffix.is_empty()
                && !suffix.starts_with('-')
                && !suffix.ends_with('-')
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
        .ok_or_else(|| {
            GateError::new(
                service.bootstrap_error_code(),
                service.component(),
                "controller identity cannot form a service hostname",
            )
        })?;
    let hostname = format!("gnx-{}-{suffix}", service.name());
    if hostname.len() > 63 {
        return Err(GateError::new(
            service.bootstrap_error_code(),
            service.component(),
            "service hostname exceeds the DNS label limit",
        ));
    }
    Ok(hostname)
}

fn prepare_lxc_service(
    podman: &Path,
    service: ServiceKind,
    hostname: &str,
    tailnet: &str,
    auth_key: &str,
    secrets: &crate::service_secrets::ServiceSecrets,
) -> Result<Option<GarageCredential>, GateError> {
    let mut input = Vec::with_capacity(1024);
    for value in [service.name(), hostname, tailnet, auth_key] {
        input.extend_from_slice(value.as_bytes());
        input.push(b'\n');
    }
    match service {
        ServiceKind::Garage => {
            let garage = secrets.garage.as_ref().ok_or_else(|| {
                GateError::new(
                    service.bootstrap_error_code(),
                    service.component(),
                    "protected Garage secrets are missing",
                )
            })?;
            for value in [
                garage.rpc_secret.as_str(),
                garage.admin_token.as_str(),
                garage.s3_access_key.as_deref().unwrap_or_default(),
                garage.s3_secret_key.as_deref().unwrap_or_default(),
            ] {
                input.extend_from_slice(value.as_bytes());
                input.push(b'\n');
            }
        }
        ServiceKind::Forgejo => {
            let forgejo = secrets.forgejo.as_ref().ok_or_else(|| {
                GateError::new(
                    service.bootstrap_error_code(),
                    service.component(),
                    "protected Forgejo secrets are missing",
                )
            })?;
            for value in [
                forgejo.secret_key.as_str(),
                forgejo.internal_token.as_str(),
                forgejo.admin_password.as_str(),
                "",
            ] {
                input.extend_from_slice(value.as_bytes());
                input.push(b'\n');
            }
        }
    }

    let result = machine_stdin(
        podman,
        [
            "podman",
            "exec",
            "-i",
            "gnx-proxmox",
            LXC_SERVICE_PREPARE_BIN,
        ],
        &input,
    );
    input.zeroize();
    let mut output = result
        .map_err(|error| error.with_code(service.bootstrap_error_code(), service.component()))?;
    let parsed = parse_service_bootstrap_output(&output.stdout, service);
    output.stdout.zeroize();
    output.stderr.zeroize();
    parsed
}

fn parse_service_bootstrap_output(
    bytes: &[u8],
    service: ServiceKind,
) -> Result<Option<GarageCredential>, GateError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        GateError::new(
            service.bootstrap_error_code(),
            service.component(),
            "LXC service bootstrap returned non-UTF-8 output",
        )
    })?;
    let lines = text.lines().collect::<Vec<_>>();
    let status = format!(
        "LXC_SERVICE=ready;SERVICE={};VMID={}",
        service.name(),
        service.vmid()
    );
    match service {
        ServiceKind::Garage if lines.len() == 3 && lines[2] == status => {
            let access_key = lines[0]
                .strip_prefix("GARAGE_ACCESS_KEY=")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    GateError::new(
                        service.bootstrap_error_code(),
                        service.component(),
                        "Garage bootstrap omitted the S3 access key",
                    )
                })?;
            let secret_key = lines[1]
                .strip_prefix("GARAGE_SECRET_KEY=")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    GateError::new(
                        service.bootstrap_error_code(),
                        service.component(),
                        "Garage bootstrap omitted the S3 secret key",
                    )
                })?;
            Ok(Some(GarageCredential {
                access_key: access_key.to_owned(),
                secret_key: secret_key.to_owned(),
            }))
        }
        ServiceKind::Forgejo if lines.as_slice() == [status.as_str()] => Ok(None),
        _ => Err(GateError::new(
            service.bootstrap_error_code(),
            service.component(),
            "LXC service bootstrap did not confirm the fixed output contract",
        )),
    }
}

fn configure_pve_password(podman: &Path, password: &str) -> Result<(), GateError> {
    let mut input = password.as_bytes().to_vec();
    let result = machine_stdin(podman, [PVE_CONFIGURE_BIN], &input);
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

fn candidate_hostname() -> Result<String, GateError> {
    let computer_name = env::var("COMPUTERNAME").map_err(|_| {
        GateError::new(
            "TAILSCALE_ENROLL_FAILED",
            Component::Tailscale,
            "COMPUTERNAME is unavailable in the service environment",
        )
    })?;
    let computer_name = computer_name.to_ascii_lowercase();
    if computer_name.is_empty()
        || computer_name.len() > 32
        || computer_name.starts_with('-')
        || computer_name.ends_with('-')
        || !computer_name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(GateError::new(
            "TAILSCALE_ENROLL_FAILED",
            Component::Tailscale,
            "Windows computer name cannot form a Tailscale hostname",
        ));
    }
    Ok(format!("gnx-candidate-{computer_name}"))
}

fn prepare_tailscale(podman: &Path, hostname: &str, auth_key: &str) -> Result<(), GateError> {
    let mut input = Vec::with_capacity(auth_key.len() + hostname.len() + 2);
    input.extend_from_slice(auth_key.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(hostname.as_bytes());
    input.push(b'\n');
    let result = machine_stdin(podman, [TAILSCALE_PREPARE_BIN], &input);
    input.zeroize();
    result
        .map(|_| ())
        .map_err(|error| error.with_code("TAILSCALE_ENROLL_FAILED", Component::Tailscale))
}

fn start_tailscale(podman: &Path) -> Result<(), GateError> {
    let output = match machine_stdin(podman, ["sh", "-s"], START_TAILSCALE.as_bytes()) {
        Ok(output) => output,
        Err(error) => {
            verify_tailscale_secret_cleanup(podman)?;
            return Err(error.with_code("TAILSCALE_ENROLL_FAILED", Component::Tailscale));
        }
    };
    if String::from_utf8_lossy(&output.stdout).trim() != "TAILSCALE_SERVICE=active" {
        return Err(GateError::new(
            "TAILSCALE_ENROLL_FAILED",
            Component::Tailscale,
            "systemd did not confirm the permanent Tailscale sidecar",
        ));
    }
    Ok(())
}

fn disable_tailscale_ssh(podman: &Path) -> Result<(), GateError> {
    machine_stdin(
        podman,
        [
            "podman",
            "exec",
            "gnx-tailscaled",
            "tailscale",
            "set",
            "--ssh=false",
        ],
        &[],
    )
    .map(|_| ())
    .map_err(|error| error.with_code("TAILSCALE_SSH_DISABLE_FAILED", Component::Tailscale))
}

fn verify_tailscale_secret_cleanup(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(
        podman,
        ["sh", "-s"],
        TAILSCALE_SECRET_CLEANUP_PROBE.as_bytes(),
    )
    .map_err(|error| error.with_code("TAILSCALE_SECRET_CLEANUP_FAILED", Component::Tailscale))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "TAILSCALE_SECRET_CLEAN=ready" {
        return Err(GateError::new(
            "TAILSCALE_SECRET_CLEANUP_FAILED",
            Component::Tailscale,
            "Tailscale enrollment did not confirm secret cleanup",
        ));
    }
    Ok(())
}

fn wait_for_tailscale(
    podman: &Path,
    hostname: &str,
    tailnet: &str,
) -> Result<TailscaleIdentity, GateError> {
    let mut last_error = String::from("Tailscale sidecar is not ready");
    for attempt in 0..90 {
        match read_tailscale_status(podman, hostname, tailnet) {
            Ok(identity) => return Ok(identity),
            Err(error) => last_error = error,
        }
        if attempt + 1 < 90 {
            thread::sleep(Duration::from_secs(2));
        }
    }
    Err(GateError::new(
        "TAILSCALE_ENROLL_FAILED",
        Component::Tailscale,
        format!(
            "Tailscale did not satisfy the pinned identity contract: {last_error}; {}",
            tailscale_diagnostics(podman)
        ),
    ))
}

fn stabilize_host_inventory(
    podman: &Path,
    initial: TailscaleIdentity,
    tailnet: &str,
) -> Result<TailscaleIdentity, GateError> {
    let hostname = initial.hostname.clone();
    let expected_self_id = initial.self_id.clone();
    let mut previous = Some(initial);
    let mut last_error = String::from("Tailscale host inventory has not stabilized");
    for attempt in 0..30 {
        thread::sleep(Duration::from_secs(2));
        match read_tailscale_status(podman, &hostname, tailnet) {
            Ok(current) => {
                if current.self_id != expected_self_id {
                    return Err(GateError::new(
                        "TAILSCALE_IDENTITY_CHANGED",
                        Component::Tailscale,
                        "Tailscale Self.ID changed during role discovery",
                    ));
                }
                if previous
                    .as_ref()
                    .is_some_and(|value| value.host_peers == current.host_peers)
                {
                    return Ok(current);
                }
                last_error = "consecutive tagged host inventories differ".into();
                previous = Some(current);
            }
            Err(error) => {
                last_error = error;
                previous = None;
            }
        }
        if attempt + 1 == 30 {
            break;
        }
    }
    Err(GateError::new(
        "TAILSCALE_DISCOVERY_UNSTABLE",
        Component::Tailscale,
        format!("Tailscale host inventory did not stabilize within 60 seconds: {last_error}"),
    ))
}

fn read_tailscale_status(
    podman: &Path,
    hostname: &str,
    tailnet: &str,
) -> Result<TailscaleIdentity, String> {
    let output = machine_stdin(
        podman,
        [
            "podman",
            "exec",
            "gnx-tailscaled",
            "tailscale",
            "status",
            "--json",
        ],
        &[],
    )
    .map_err(|error| error.message)?;
    parse_tailscale_status(&output.stdout, hostname, tailnet)
}

fn parse_tailscale_status(
    bytes: &[u8],
    hostname: &str,
    tailnet: &str,
) -> Result<TailscaleIdentity, String> {
    let status: TailscaleStatus = serde_json::from_slice(bytes)
        .map_err(|_| "tailscale status returned invalid JSON".to_string())?;
    let current_tailnet = status
        .current_tailnet
        .ok_or_else(|| "tailscale status has no current tailnet".to_string())?;
    let self_node = status
        .self_node
        .ok_or_else(|| "tailscale status has no self node".to_string())?;
    let domain = format!("{hostname}.{tailnet}");
    let expected_dns_name = format!("{domain}.");
    let cgnat_ipv4 = status
        .tailscale_ips
        .iter()
        .filter_map(|value| value.parse::<IpAddr>().ok())
        .filter(|address| match address {
            IpAddr::V4(address) => {
                let octets = address.octets();
                octets[0] == 100 && (64..=127).contains(&octets[1])
            }
            IpAddr::V6(_) => false,
        })
        .collect::<Vec<_>>();
    if status.backend_state != "Running"
        || !status.health.is_empty()
        || !status.tun
        || current_tailnet.magic_dns_suffix != tailnet
        || !current_tailnet.magic_dns_enabled
        || self_node.host_name != hostname
        || self_node.dns_name != expected_dns_name
        || self_node.tags.as_slice() != ["tag:quetzalcoatl-node"]
        || !status.cert_domains.iter().any(|value| value == &domain)
        || cgnat_ipv4.len() != 1
        || self_node.id.is_empty()
        || !self_node.id.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err("tailscale status does not match tailnet, hostname, stable ID, tag, TUN, IP and HTTPS requirements".into());
    }

    let mut host_peers = Vec::new();
    for peer in status.peers.values() {
        if peer.expired
            || peer.id == self_node.id
            || peer.tags.as_slice() != ["tag:quetzalcoatl-node"]
        {
            continue;
        }
        let peer_ips = peer
            .tailscale_ips
            .iter()
            .filter_map(|value| value.parse::<IpAddr>().ok())
            .filter(|address| matches!(address, IpAddr::V4(address) if address.octets()[0] == 100 && (64..=127).contains(&address.octets()[1])))
            .collect::<Vec<_>>();
        if peer.id.is_empty()
            || !peer.id.bytes().all(|byte| byte.is_ascii_graphic())
            || peer_ips.len() != 1
            || !valid_discovered_hostname(&peer.host_name)
        {
            return Err(
                "tagged Tailscale host peer does not satisfy the GNX host identity contract".into(),
            );
        }
        host_peers.push(HostPeer {
            id: peer.id.clone(),
            hostname: peer.host_name.clone(),
            ip: peer_ips[0],
            online: peer.online,
            // Tailscale reports the peer's DERP region even when CurAddr is the
            // active direct endpoint. Its own status renderer treats a
            // non-empty CurAddr as direct and consults Relay only otherwise.
            direct: !peer.cur_addr.is_empty(),
        });
    }
    host_peers.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(TailscaleIdentity {
        self_id: self_node.id,
        self_ip: cgnat_ipv4[0],
        hostname: self_node.host_name,
        host_peers,
    })
}

fn wait_for_tailscale_serve(podman: &Path, hostname: &str, tailnet: &str) -> Result<(), GateError> {
    let domain = format!("{hostname}.{tailnet}");
    let host_port = format!("{domain}:443");
    let mut last_error = String::from("Tailscale Serve config is not ready");
    for attempt in 0..60 {
        match machine_stdin(
            podman,
            [
                "podman",
                "exec",
                "gnx-tailscaled",
                "tailscale",
                "serve",
                "status",
                "--json",
            ],
            &[],
        ) {
            Ok(output) => match parse_serve_status(&output.stdout, &host_port) {
                Ok(()) => return Ok(()),
                Err(error) => last_error = error,
            },
            Err(error) => last_error = error.message,
        }
        if attempt + 1 < 60 {
            thread::sleep(Duration::from_secs(2));
        }
    }
    Err(GateError::new(
        "TAILSCALE_SERVE_FAILED",
        Component::TailscaleServe,
        format!(
            "Tailscale Serve did not expose only the approved PVE UI: {last_error}; {}",
            tailscale_diagnostics(podman)
        ),
    ))
}

fn parse_serve_status(bytes: &[u8], host_port: &str) -> Result<(), String> {
    let status: TailscaleServeStatus = serde_json::from_slice(bytes)
        .map_err(|_| "tailscale serve status returned invalid JSON".to_string())?;
    let https = status.tcp.get("443").is_some_and(|entry| entry.https);
    let proxy = status
        .web
        .get(host_port)
        .and_then(|entry| entry.handlers.get("/"))
        .map(|handler| handler.proxy.as_str());
    let funnel_disabled = !status.allow_funnel.get(host_port).copied().unwrap_or(false);
    if !https || proxy != Some("https+insecure://127.0.0.1:8006") || !funnel_disabled {
        return Err("serve status is missing the fixed HTTPS PVE proxy or enables Funnel".into());
    }
    Ok(())
}

fn tailscale_diagnostics(podman: &Path) -> String {
    match machine_stdin(podman, ["sh", "-s"], TAILSCALE_DIAGNOSTICS.as_bytes()) {
        Ok(output) => bounded_text(&output.stdout),
        Err(error) => format!("diagnostics unavailable: {}", error.message),
    }
}

fn machine_stdin<I, S>(podman: &Path, args: I, input: &[u8]) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = machine_stdin_output(podman, args, input)?;
    check_output(output, "podman machine probe")
}

fn machine_stdin_output<I, S>(podman: &Path, args: I, input: &[u8]) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(podman);
    command
        .args(["machine", "ssh", "--username", "root", MACHINE_NAME])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|error| {
        GateError::command(format!("cannot start podman machine probe: {error}"))
    })?;
    child
        .stdin
        .take()
        .ok_or_else(|| GateError::command("podman machine probe stdin is unavailable"))?
        .write_all(input)
        .map_err(|error| GateError::command(format!("cannot write machine probe: {error}")))?;
    let output = child
        .wait_with_output()
        .map_err(|error| GateError::command(format!("cannot wait for machine probe: {error}")))?;
    Ok(output)
}

fn load_machine_image() -> Result<MachineImage, GateError> {
    let manifest = runtime_root()?.join("manifest.json");
    let bytes = fs::read(&manifest).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot read runtime manifest: {error}"),
        )
    })?;
    parse_machine_image(&bytes)
}

fn runtime_root() -> Result<PathBuf, GateError> {
    let executable = env::current_exe().map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot locate gnx-service executable: {error}"),
        )
    })?;
    executable
        .parent()
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                "gnx-service executable has no parent directory",
            )
        })
        .map(|parent| parent.join("runtime"))
}

fn load_payload_files() -> Result<Vec<PayloadFile>, GateError> {
    let root = runtime_root()?;
    let manifest_path = root.join("manifest.json");
    let bytes = fs::read(&manifest_path).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            format!("cannot read runtime manifest: {error}"),
        )
    })?;
    let entries = parse_payload_manifest(&bytes)?;
    let mut files = Vec::with_capacity(entries.len());
    for entry in entries {
        let source = root.join(&entry.relative_path);
        let contents = fs::read(&source).map_err(|error| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!("cannot read payload file {}: {error}", entry.relative_path),
            )
        })?;
        if sha256_bytes(&contents) != entry.sha256 {
            return Err(GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!(
                    "payload file {} does not match its locked SHA-256",
                    entry.relative_path
                ),
            ));
        }
        files.push(PayloadFile {
            destination: entry.destination,
            mode: entry.mode,
            sha256: entry.sha256,
            contents,
        });
    }
    Ok(files)
}

fn parse_payload_manifest(bytes: &[u8]) -> Result<Vec<LockedPayloadFile>, GateError> {
    let manifest: RuntimeManifest = serde_json::from_slice(bytes).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            format!("runtime manifest is invalid JSON: {error}"),
        )
    })?;
    if manifest.payload_version != 1 || manifest.files.len() != PAYLOAD_FILES.len() {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            "runtime manifest does not contain the exact payload v1 file set",
        ));
    }

    let mut locked = Vec::with_capacity(PAYLOAD_FILES.len());
    for spec in PAYLOAD_FILES {
        let matches = manifest
            .files
            .iter()
            .filter(|file| file.path == spec.relative_path)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!(
                    "runtime manifest must contain {} exactly once",
                    spec.relative_path
                ),
            ));
        }
        let file = matches[0];
        if file.mode != spec.mode || !valid_file_sha256(&file.sha256) {
            return Err(GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!("runtime manifest metadata is invalid for {}", file.path),
            ));
        }
        locked.push(LockedPayloadFile {
            relative_path: file.path.clone(),
            destination: spec.destination.to_owned(),
            mode: file.mode.clone(),
            sha256: file.sha256.clone(),
        });
    }
    Ok(locked)
}

fn valid_file_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_machine_image(bytes: &[u8]) -> Result<MachineImage, GateError> {
    let manifest: RuntimeManifest = serde_json::from_slice(bytes).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("runtime manifest is invalid JSON: {error}"),
        )
    })?;
    let image = manifest
        .components
        .into_iter()
        .find(|component| component.id == "podman-machine-os")
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                "runtime manifest has no podman-machine-os component",
            )
        })?;
    let platform = image.platform.as_ref();
    if image.kind.as_deref() != Some("oci_machine_image")
        || image.version.as_deref() != Some("6.0.1")
        || image.source_ref.as_deref() != Some("v6.0.1")
        || image.source_commit.as_deref() != Some(MACHINE_IMAGE_COMMIT)
        || image.image.as_deref() != Some("quay.io/podman/machine-os")
        || image.index_digest.as_deref() != Some(MACHINE_IMAGE_INDEX)
        || image.manifest_digest.as_deref() != Some(MACHINE_IMAGE_MANIFEST)
        || image.layer_digest.as_deref() != Some(MACHINE_IMAGE_LAYER)
        || image.artifact.as_deref() != Some(MACHINE_IMAGE_ARTIFACT)
        || image.artifact_url.as_deref() != Some(MACHINE_IMAGE_URL)
        || image.artifact_size != Some(MACHINE_IMAGE_SIZE)
        || platform.map(|value| value.os.as_str()) != Some("linux")
        || platform.map(|value| value.architecture.as_str()) != Some("x86_64")
        || platform.map(|value| value.disk_type.as_str()) != Some("wsl")
    {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            "podman-machine-os pin is incomplete or incompatible",
        ));
    }
    Ok(MachineImage {
        artifact: image.artifact.expect("validated artifact"),
        size: image.artifact_size.expect("validated artifact size"),
        sha256: image.layer_digest.expect("validated layer digest"),
    })
}

#[cfg(test)]
fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn system_binary(name: &str) -> Result<PathBuf, GateError> {
    let root = env::var_os("SystemRoot").ok_or_else(|| {
        GateError::command("SystemRoot is not available in the service environment")
    })?;
    let path = PathBuf::from(root).join("System32").join(name);
    existing_binary(path)
}

fn podman_binary() -> Result<PathBuf, GateError> {
    let root = env::var_os("ProgramFiles").ok_or_else(|| {
        GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "ProgramFiles is not available in the service environment",
        )
    })?;
    existing_binary(PathBuf::from(root).join("Podman").join("podman.exe"))
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))
}

fn existing_binary(path: PathBuf) -> Result<PathBuf, GateError> {
    if path.is_file() {
        Ok(path)
    } else {
        Err(GateError::command(format!(
            "required executable is absent: {}",
            path.display()
        )))
    }
}

fn run_command<I, S>(program: &Path, args: I) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| {
            GateError::command(format!("cannot execute {}: {error}", program.display()))
        })?;
    check_output(output, &program.display().to_string())
}

fn check_output(output: Output, operation: &str) -> Result<Output, GateError> {
    if output.status.success() {
        return Ok(output);
    }
    let detail = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    Err(GateError::command(format!(
        "{operation} failed with exit {}: {}",
        output.status.code().unwrap_or(-1),
        bounded_text(detail)
    )))
}

fn bounded_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes).replace(['\r', '\n'], " ");
    text.chars()
        .take(1600)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn set_stage(status: &Arc<RwLock<StatusResponse>>, stage: &str) {
    if let Ok(mut status) = status.write() {
        status.stage = stage.to_owned();
        status.overall = "pending".into();
        status.last_error = None;
    }
}

fn set_component(status: &Arc<RwLock<StatusResponse>>, component: Component, value: &str) {
    if let Ok(mut status) = status.write() {
        component.set(&mut status, value);
    }
}

fn set_controller(status: &Arc<RwLock<StatusResponse>>, hostname: &str) {
    if let Ok(mut status) = status.write() {
        status.role = Some("controller".into());
        status.controller = Some(hostname.into());
    }
}

fn set_member_joining_status(status: &Arc<RwLock<StatusResponse>>, controller: &str) {
    if let Ok(mut status) = status.write() {
        status.role = Some("member".into());
        status.controller = Some(controller.into());
        status.components.opentofu = "not_applicable".into();
        status.services.garage = "not_applicable".into();
        status.services.forgejo = "not_applicable".into();
        status.stage = "MEMBER_JOINING".into();
        status.overall = "pending".into();
    }
}

fn set_member_ready_status(status: &Arc<RwLock<StatusResponse>>, controller: &str) {
    if let Ok(mut value) = status.write() {
        *value = StatusResponse::member_ready(controller.to_owned());
    }
}

fn set_cluster_ready(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.cluster.joined = true;
        status.cluster.quorate = true;
    }
}

fn complete(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.overall = "ready".into();
        status.stage = "READY".into();
        status.last_error = None;
    }
}

fn fail(status: &Arc<RwLock<StatusResponse>>, error: GateError) {
    if let Ok(mut status) = status.write() {
        status.overall = "failed".into();
        status.stage = "FAILED".into();
        error.component.set(&mut status, "failed");
        status.last_error = Some(format!("{}: {}", error.code, error.message));
    }
}

#[derive(Clone, Copy)]
enum ServiceKind {
    Garage,
    Forgejo,
}

impl ServiceKind {
    fn name(self) -> &'static str {
        match self {
            Self::Garage => "garage",
            Self::Forgejo => "forgejo",
        }
    }

    fn vmid(self) -> u16 {
        match self {
            Self::Garage => 200,
            Self::Forgejo => 201,
        }
    }

    fn component(self) -> Component {
        match self {
            Self::Garage => Component::Garage,
            Self::Forgejo => Component::Forgejo,
        }
    }

    fn bootstrap_error_code(self) -> &'static str {
        match self {
            Self::Garage => "GARAGE_BOOTSTRAP_FAILED",
            Self::Forgejo => "FORGEJO_BOOTSTRAP_FAILED",
        }
    }
}

#[derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
struct GarageCredential {
    access_key: String,
    secret_key: String,
}

#[derive(Clone, Copy, Debug)]
enum Component {
    None,
    Wsl,
    PodmanMachine,
    Kvm,
    Proxmox,
    Tailscale,
    TailscaleServe,
    OpenTofu,
    Garage,
    Forgejo,
}

impl Component {
    fn set(self, status: &mut StatusResponse, value: &str) {
        match self {
            Self::None => {}
            Self::Wsl => status.components.wsl = value.into(),
            Self::PodmanMachine => status.components.podman_machine = value.into(),
            Self::Kvm => status.components.kvm = value.into(),
            Self::Proxmox => status.components.proxmox = value.into(),
            Self::Tailscale => status.components.tailscale = value.into(),
            Self::TailscaleServe => status.components.tailscale_serve = value.into(),
            Self::OpenTofu => status.components.opentofu = value.into(),
            Self::Garage => status.services.garage = value.into(),
            Self::Forgejo => status.services.forgejo = value.into(),
        }
    }
}

#[derive(Debug)]
struct GateError {
    code: &'static str,
    component: Component,
    message: String,
}

impl GateError {
    fn new(code: &'static str, component: Component, message: impl Into<String>) -> Self {
        Self {
            code,
            component,
            message: message.into(),
        }
    }

    fn command(message: impl Into<String>) -> Self {
        Self::new("RUNTIME_GATE_FAILED", Component::None, message)
    }

    fn with_code(mut self, code: &'static str, component: Component) -> Self {
        self.code = code;
        self.component = component;
        self
    }
}

#[derive(Deserialize)]
struct RuntimeManifest {
    payload_version: u64,
    components: Vec<RuntimeComponent>,
    files: Vec<RuntimeFile>,
}

#[derive(Deserialize)]
struct RuntimeFile {
    path: String,
    mode: String,
    sha256: String,
}

#[derive(Deserialize)]
struct RuntimeComponent {
    id: String,
    kind: Option<String>,
    version: Option<String>,
    source_ref: Option<String>,
    source_commit: Option<String>,
    image: Option<String>,
    index_digest: Option<String>,
    manifest_digest: Option<String>,
    layer_digest: Option<String>,
    artifact: Option<String>,
    artifact_url: Option<String>,
    artifact_size: Option<u64>,
    platform: Option<RuntimePlatform>,
}

#[derive(Deserialize)]
struct RuntimePlatform {
    os: String,
    architecture: String,
    disk_type: String,
}

struct MachineImage {
    artifact: String,
    size: u64,
    sha256: String,
}

#[derive(Clone, Copy)]
struct PayloadSpec {
    relative_path: &'static str,
    destination: &'static str,
    mode: &'static str,
}

impl PayloadSpec {
    const fn new(
        relative_path: &'static str,
        destination: &'static str,
        mode: &'static str,
    ) -> Self {
        Self {
            relative_path,
            destination,
            mode,
        }
    }
}

struct LockedPayloadFile {
    relative_path: String,
    destination: String,
    mode: String,
    sha256: String,
}

struct PayloadFile {
    destination: String,
    mode: String,
    sha256: String,
    contents: Vec<u8>,
}

#[derive(Deserialize)]
struct MachineListEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "VMType")]
    vm_type: String,
}

#[derive(Deserialize)]
struct MachineInspect {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Rootful")]
    rootful: bool,
    #[serde(rename = "Resources")]
    resources: MachineResources,
}

#[derive(Deserialize)]
struct MachineResources {
    #[serde(rename = "CPUs")]
    cpus: u64,
    #[serde(rename = "Memory")]
    memory: u64,
    #[serde(rename = "DiskSize")]
    disk_size: u64,
}

#[derive(Deserialize)]
struct TailscaleStatus {
    #[serde(rename = "BackendState")]
    backend_state: String,
    #[serde(rename = "Health", default)]
    health: Vec<String>,
    #[serde(rename = "TUN")]
    tun: bool,
    #[serde(rename = "TailscaleIPs")]
    tailscale_ips: Vec<String>,
    #[serde(rename = "Self")]
    self_node: Option<TailscalePeer>,
    #[serde(rename = "CurrentTailnet")]
    current_tailnet: Option<TailscaleTailnet>,
    #[serde(rename = "CertDomains")]
    cert_domains: Vec<String>,
    #[serde(rename = "Peer", default)]
    peers: HashMap<String, TailscalePeer>,
}

#[derive(Deserialize)]
struct TailscaleTailnet {
    #[serde(rename = "MagicDNSSuffix")]
    magic_dns_suffix: String,
    #[serde(rename = "MagicDNSEnabled")]
    magic_dns_enabled: bool,
}

#[derive(Deserialize)]
struct TailscalePeer {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "HostName")]
    host_name: String,
    #[serde(rename = "DNSName")]
    dns_name: String,
    #[serde(rename = "Tags", default)]
    tags: Vec<String>,
    #[serde(rename = "Expired", default)]
    expired: bool,
    #[serde(rename = "Online", default)]
    online: bool,
    #[serde(rename = "TailscaleIPs", default)]
    tailscale_ips: Vec<String>,
    #[serde(rename = "CurAddr", default)]
    cur_addr: String,
}

#[derive(Clone, Debug, PartialEq)]
struct TailscaleIdentity {
    self_id: String,
    self_ip: IpAddr,
    hostname: String,
    host_peers: Vec<HostPeer>,
}

#[derive(Clone, Debug, PartialEq)]
struct HostPeer {
    id: String,
    hostname: String,
    ip: IpAddr,
    online: bool,
    direct: bool,
}

fn valid_discovered_hostname(value: &str) -> bool {
    let suffix = value
        .strip_prefix("gnx-controller-")
        .or_else(|| value.strip_prefix("gnx-member-"));
    suffix.is_some_and(|suffix| {
        !suffix.is_empty()
            && !suffix.starts_with('-')
            && !suffix.ends_with('-')
            && value.len() <= 63
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    })
}

#[derive(Deserialize)]
struct TailscaleServeStatus {
    #[serde(rename = "TCP", default)]
    tcp: HashMap<String, TailscaleTcpHandler>,
    #[serde(rename = "Web", default)]
    web: HashMap<String, TailscaleWebHandler>,
    #[serde(rename = "AllowFunnel", default)]
    allow_funnel: HashMap<String, bool>,
}

#[derive(Deserialize)]
struct TailscaleTcpHandler {
    #[serde(rename = "HTTPS", default)]
    https: bool,
}

#[derive(Deserialize)]
struct TailscaleWebHandler {
    #[serde(rename = "Handlers", default)]
    handlers: HashMap<String, TailscalePathHandler>,
}

#[derive(Deserialize)]
struct TailscalePathHandler {
    #[serde(rename = "Proxy", default)]
    proxy: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locked_wsl_machine_image() {
        let manifest = include_bytes!("../../../runtime/payload-v1/manifest.json");
        let image = parse_machine_image(manifest).expect("machine image pin");
        assert_eq!(image.artifact, MACHINE_IMAGE_ARTIFACT);
        assert_eq!(image.size, MACHINE_IMAGE_SIZE);
        assert_eq!(image.sha256, MACHINE_IMAGE_LAYER);
    }

    #[test]
    fn payload_manifest_matches_all_installed_files() {
        let manifest = include_bytes!("../../../runtime/payload-v1/manifest.json");
        let files = parse_payload_manifest(manifest).expect("payload v1 file set");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/payload-v1");

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
        assert_eq!(inspect[0].resources.memory, MACHINE_MEMORY_MIB);
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
    fn discovery_counts_offline_hosts_and_excludes_expired_peers_and_sidecars() {
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
              "HostName":"gnx-garage-existing",
              "DNSName":"gnx-garage-existing.tetra-balance.ts.net.",
              "Tags":["tag:quetzalcoatl-service"],
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
        assert_eq!(
            identity
                .host_peers
                .iter()
                .map(|peer| peer.id.clone())
                .collect::<Vec<_>>(),
            vec!["node-id-host".to_string()]
        );
        assert!(identity.host_peers[0].direct);
    }

    #[test]
    fn topology_matrix_is_bounded_and_selects_exactly_one_controller() {
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
        assert!(matches!(
            select_topology(&identity(vec![host("member", "gnx-member-a")])),
            Err(error) if error.code == "TOPOLOGY_UNSUPPORTED"
        ));
        assert!(matches!(
            select_topology(&identity(vec![host("a", "gnx-controller-a"), host("b", "gnx-controller-b")])),
            Err(error) if error.code == "TOPOLOGY_UNSUPPORTED"
        ));
        assert!(matches!(
            select_topology(&identity(vec![host("a", "gnx-controller-a"), host("b", "gnx-member-b"), host("c", "gnx-member-c")])),
            Err(error) if error.code == "TOPOLOGY_UNSUPPORTED"
        ));
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
            "gnx-garage-node",
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
    fn initial_member_decision_pins_an_offline_controller_before_availability_fails() {
        let controller = HostPeer {
            id: "controller".into(),
            hostname: "gnx-controller-controller".into(),
            ip: "100.100.10.21".parse().expect("IP"),
            online: false,
            direct: false,
        };
        let identity = TailscaleIdentity {
            self_id: "member".into(),
            self_ip: "100.100.10.22".parse().expect("IP"),
            hostname: "gnx-candidate".into(),
            host_peers: vec![controller.clone()],
        };
        let TopologyDecision::Member(selected) = select_topology(&identity).expect("member role")
        else {
            panic!("expected member role");
        };
        let state = crate::state::PersistedState::member(
            identity.self_id.clone(),
            identity.self_ip,
            member_hostname(&identity.self_id).expect("member hostname"),
            crate::state::ControllerIdentity {
                id: selected.id,
                hostname: selected.hostname,
                ip: selected.ip,
            },
            "tetra-balance.ts.net".into(),
        );

        assert!(state.role.is_member());
        assert_eq!(state.controller.id, controller.id);
        assert!(matches!(
            validate_persisted_member_controller(&state, &identity),
            Err(error) if error.code == "CONTROLLER_UNAVAILABLE"
        ));
    }

    #[test]
    fn member_configuration_uses_tailnet_without_controller_workload_selection() {
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
            install_garage: true,
            install_forgejo: true,
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
    fn member_joining_status_keeps_local_components_ready_and_disables_controller_workloads() {
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

        set_member_joining_status(&status, "gnx-controller-controller");

        let value = status.read().expect("status lock");
        assert_eq!(value.role.as_deref(), Some("member"));
        assert_eq!(
            value.controller.as_deref(),
            Some("gnx-controller-controller")
        );
        assert_eq!(value.stage, "MEMBER_JOINING");
        assert_eq!(value.components.proxmox, "ready");
        assert_eq!(value.components.opentofu, "not_applicable");
        assert_eq!(value.services.garage, "not_applicable");
        assert_eq!(value.services.forgejo, "not_applicable");
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
        assert_eq!(member.stage, "MEMBER_JOINING");
        assert!(!begin_member_join(&mut member).expect("resume joining"));
        assert_eq!(member.controller, controller);

        member.cluster_join = crate::state::ClusterJoinState::Joined;
        member.stage = "READY".into();
        assert!(!begin_member_join(&mut member).expect("resume joined verification"));
        assert_eq!(member.controller, controller);
    }

    #[test]
    fn member_opentofu_guard_returns_before_starting_a_process() {
        let error = apply_opentofu(
            Path::new("not-launched"),
            &crate::state::PersistedRole::Member,
            "test-credential",
            "gnx-controller-controller",
            false,
            false,
        )
        .expect_err("members must not invoke OpenTofu");
        assert_eq!(error.code, "MEMBER_OPENTOFU_DENIED");
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
        assert_eq!(value.components.opentofu, "not_applicable");
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
    fn service_hostnames_share_only_the_stable_logical_suffix() {
        assert_eq!(
            service_hostname(ServiceKind::Garage, "gnx-controller-nabc123")
                .expect("Garage hostname"),
            "gnx-garage-nabc123"
        );
        assert_eq!(
            service_hostname(ServiceKind::Forgejo, "gnx-controller-nabc123")
                .expect("Forgejo hostname"),
            "gnx-forgejo-nabc123"
        );
        assert!(service_hostname(ServiceKind::Garage, "gnx-controller-").is_err());
    }

    #[test]
    fn reconciles_reauthenticated_node_id_only_with_logical_identity_continuity() {
        let state = crate::state::PersistedState::controller(
            "node-id-before".into(),
            "100.100.10.20".parse().expect("IP"),
            "gnx-controller-node-id-before".into(),
            "tetra-balance.ts.net".into(),
            true,
            true,
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
        let error = reconcile_persisted_identity(state.clone(), &changed_ip)
            .expect_err("IP drift must fail");
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

        let (reconciled, changed) = reconcile_persisted_identity(state.clone(), &identity)
            .expect("member re-authentication");
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
    fn service_bootstrap_output_has_no_alternate_shape() {
        let garage = b"GARAGE_ACCESS_KEY=GK0123456789abcdef01234567\nGARAGE_SECRET_KEY=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nLXC_SERVICE=ready;SERVICE=garage;VMID=200\n";
        let credential = parse_service_bootstrap_output(garage, ServiceKind::Garage)
            .expect("Garage output")
            .expect("Garage credential");
        assert_eq!(credential.access_key, "GK0123456789abcdef01234567");
        assert_eq!(credential.secret_key.len(), 64);

        let forgejo = b"LXC_SERVICE=ready;SERVICE=forgejo;VMID=201\n";
        assert!(
            parse_service_bootstrap_output(forgejo, ServiceKind::Forgejo)
                .expect("Forgejo output")
                .is_none()
        );
        assert!(
            parse_service_bootstrap_output(
                b"diagnostic\nLXC_SERVICE=ready;SERVICE=forgejo;VMID=201\n",
                ServiceKind::Forgejo,
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_only_the_fixed_pve_serve_route() {
        let json = br#"{
          "TCP":{"443":{"HTTPS":true}},
          "Web":{
            "gnx-candidate-nitro.tetra-balance.ts.net:443":{
              "Handlers":{"/":{"Proxy":"https+insecure://127.0.0.1:8006"}}
            }
          },
          "AllowFunnel":{"gnx-candidate-nitro.tetra-balance.ts.net:443":false}
        }"#;
        assert!(parse_serve_status(json, "gnx-candidate-nitro.tetra-balance.ts.net:443").is_ok());
    }
}
