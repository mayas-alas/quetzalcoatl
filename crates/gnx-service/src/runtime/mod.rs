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

pub(crate) mod control;
mod error;
mod host;
mod machine;
mod model;
mod payload;
mod proxmox;
mod reconciler;
mod remote;
mod status;
mod tailscale;
mod topology;

use error::*;
use host::*;
use machine::*;
use model::*;
use payload::*;
use proxmox::*;
use remote::*;
use status::*;
use tailscale::*;
use topology::*;

const EXPECTED_SERVICE_SID: &str = "S-1-5-80-1414281857-1943412974-186110390-2486725240-2230548587";
const MACHINE_NAME: &str = "quetzalcoatl";
const MACHINE_CPUS: u64 = 6;
const MACHINE_MEMORY_MIB: u64 = 8192;
const MACHINE_DISK_GIB: u64 = 100;
const MACHINE_NETWORK_MTU: u32 = 1500;
const RUNTIME_GENERATION: &str = "proxmox-cluster-v2";
const RUNTIME_GENERATION_PATH: &str = "/etc/quetzalcoatl/runtime-generation";
const TAILSCALE_STATE_PATH: &str = "/var/lib/quetzalcoatl/tailscale/host/tailscaled.state";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const RUNTIME_AGENT_BIN: &str = "/usr/libexec/quetzalcoatl/gnx-runtime-agent";
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

const PAYLOAD_FILES: [PayloadSpec; 12] = [
    PayloadSpec::new(
        "bin/gnx-runtime-agent",
        "/usr/libexec/quetzalcoatl/gnx-runtime-agent",
        "0755",
    ),
    PayloadSpec::new(
        "bin/gnx-proxmox-entrypoint",
        "/usr/libexec/quetzalcoatl/gnx-proxmox-entrypoint",
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

const PAYLOAD_HEREDOC: &str = "__GNX_PAYLOAD_EOF__";

const START_PROXMOX: &str = r#"set -eu
install -d -m 0755 \
  /var/lib/quetzalcoatl/proxmox/vz \
  /var/lib/quetzalcoatl/proxmox/cluster
install -d -m 0755 /run/gnx
date --iso-8601=seconds > /run/gnx/proxmox-started-at
systemctl daemon-reload
systemctl stop proxmox.service >/dev/null 2>&1 || true
systemctl reset-failed gnx-node-pod.service proxmox.service >/dev/null 2>&1 || true
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
test "$(podman exec gnx-proxmox ps -p 1 -o comm= | tr -d ' ')" = systemd
test "$(podman exec gnx-proxmox stat -fc %T /sys/fs/cgroup)" = cgroup2fs
podman exec gnx-proxmox systemctl is-active --quiet pve-cluster.service
podman exec gnx-proxmox systemctl is-active --quiet pvedaemon.service
podman exec gnx-proxmox systemctl is-active --quiet pveproxy.service
podman exec gnx-proxmox pvesh get /version --output-format json >/dev/null
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

pub(crate) fn run(status: Arc<RwLock<StatusResponse>>) {
    set_stage(&status, "RUNTIME_IDENTITY");
    if let Err(error) = reconciler::run(&status) {
        fail(&status, error);
    }
}

#[cfg(test)]
mod tests;
