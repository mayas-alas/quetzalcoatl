use std::net::IpAddr;

use gnx_contracts::{ComponentHealth, StatusResponse};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Component {
    None,
    Wsl,
    PodmanMachine,
    Kvm,
    Proxmox,
    Tailscale,
    TailscaleServe,
}

impl Component {
    pub(crate) fn set(self, status: &mut StatusResponse, value: ComponentHealth) {
        match self {
            Self::None => {}
            Self::Wsl => status.components.wsl = value,
            Self::PodmanMachine => status.components.podman_machine = value,
            Self::Kvm => status.components.kvm = value,
            Self::Proxmox => status.components.proxmox = value,
            Self::Tailscale => status.components.tailscale = value,
            Self::TailscaleServe => status.components.tailscale_serve = value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TailscaleIdentity {
    pub(crate) self_id: String,
    pub(crate) self_ip: IpAddr,
    pub(crate) hostname: String,
    pub(crate) host_peers: Vec<HostPeer>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HostPeer {
    pub(crate) id: String,
    pub(crate) hostname: String,
    pub(crate) ip: IpAddr,
    pub(crate) online: bool,
    pub(crate) direct: bool,
}

pub(crate) fn valid_discovered_hostname(value: &str) -> bool {
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
