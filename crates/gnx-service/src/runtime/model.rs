use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) enum Component {
    None,
    Wsl,
    PodmanMachine,
    Kvm,
    Proxmox,
    Tailscale,
    TailscaleServe,
}

impl Component {
    pub(super) fn set(self, status: &mut StatusResponse, value: &str) {
        match self {
            Self::None => {}
            Self::Wsl => status.components.wsl = value.into(),
            Self::PodmanMachine => status.components.podman_machine = value.into(),
            Self::Kvm => status.components.kvm = value.into(),
            Self::Proxmox => status.components.proxmox = value.into(),
            Self::Tailscale => status.components.tailscale = value.into(),
            Self::TailscaleServe => status.components.tailscale_serve = value.into(),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct RuntimeManifest {
    pub(super) payload_version: u64,
    pub(super) components: Vec<RuntimeComponent>,
    pub(super) files: Vec<RuntimeFile>,
}

#[derive(Deserialize)]
pub(super) struct RuntimeFile {
    pub(super) path: String,
    pub(super) mode: String,
    pub(super) sha256: String,
}

#[derive(Deserialize)]
pub(super) struct RuntimeComponent {
    pub(super) id: String,
    pub(super) kind: Option<String>,
    pub(super) version: Option<String>,
    pub(super) source_ref: Option<String>,
    pub(super) source_commit: Option<String>,
    pub(super) image: Option<String>,
    pub(super) index_digest: Option<String>,
    pub(super) manifest_digest: Option<String>,
    pub(super) layer_digest: Option<String>,
    pub(super) artifact: Option<String>,
    pub(super) artifact_url: Option<String>,
    pub(super) artifact_size: Option<u64>,
    pub(super) platform: Option<RuntimePlatform>,
}

#[derive(Deserialize)]
pub(super) struct RuntimePlatform {
    pub(super) os: String,
    pub(super) architecture: String,
    pub(super) disk_type: String,
}

pub(super) struct MachineImage {
    pub(super) artifact: String,
    pub(super) size: u64,
    pub(super) sha256: String,
}

#[derive(Clone, Copy)]
pub(super) struct PayloadSpec {
    pub(super) relative_path: &'static str,
    pub(super) destination: &'static str,
    pub(super) mode: &'static str,
}

impl PayloadSpec {
    pub(super) const fn new(
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

pub(super) struct LockedPayloadFile {
    pub(super) relative_path: String,
    pub(super) destination: String,
    pub(super) mode: String,
    pub(super) sha256: String,
}

pub(super) struct PayloadFile {
    pub(super) destination: String,
    pub(super) mode: String,
    pub(super) sha256: String,
    pub(super) contents: Vec<u8>,
}

#[derive(Deserialize)]
pub(super) struct MachineListEntry {
    #[serde(rename = "Name")]
    pub(super) name: String,
    #[serde(rename = "VMType")]
    pub(super) vm_type: String,
}

#[derive(Deserialize)]
pub(super) struct MachineInspect {
    #[serde(rename = "Name")]
    pub(super) name: String,
    #[serde(rename = "State")]
    pub(super) state: String,
    #[serde(rename = "Rootful")]
    pub(super) rootful: bool,
    #[serde(rename = "Resources")]
    pub(super) resources: MachineResources,
}

#[derive(Deserialize)]
pub(super) struct MachineResources {
    #[serde(rename = "CPUs")]
    pub(super) cpus: u64,
    #[serde(rename = "Memory")]
    pub(super) memory: u64,
    #[serde(rename = "DiskSize")]
    pub(super) disk_size: u64,
}

#[derive(Deserialize)]
pub(super) struct TailscaleStatus {
    #[serde(rename = "BackendState")]
    pub(super) backend_state: String,
    #[serde(rename = "Health", default)]
    pub(super) health: Vec<String>,
    #[serde(rename = "TUN")]
    pub(super) tun: bool,
    #[serde(rename = "TailscaleIPs")]
    pub(super) tailscale_ips: Vec<String>,
    #[serde(rename = "Self")]
    pub(super) self_node: Option<TailscalePeer>,
    #[serde(rename = "CurrentTailnet")]
    pub(super) current_tailnet: Option<TailscaleTailnet>,
    #[serde(rename = "CertDomains")]
    pub(super) cert_domains: Vec<String>,
    #[serde(rename = "Peer", default)]
    pub(super) peers: HashMap<String, TailscalePeer>,
}

#[derive(Deserialize)]
pub(super) struct TailscaleTailnet {
    #[serde(rename = "MagicDNSSuffix")]
    pub(super) magic_dns_suffix: String,
    #[serde(rename = "MagicDNSEnabled")]
    pub(super) magic_dns_enabled: bool,
}

#[derive(Deserialize)]
pub(super) struct TailscalePeer {
    #[serde(rename = "ID", default)]
    pub(super) id: String,
    #[serde(rename = "HostName")]
    pub(super) host_name: String,
    #[serde(rename = "DNSName")]
    pub(super) dns_name: String,
    #[serde(rename = "Tags", default)]
    pub(super) tags: Vec<String>,
    #[serde(rename = "Expired", default)]
    pub(super) expired: bool,
    #[serde(rename = "Online", default)]
    pub(super) online: bool,
    #[serde(rename = "TailscaleIPs", default)]
    pub(super) tailscale_ips: Vec<String>,
    #[serde(rename = "CurAddr", default)]
    pub(super) cur_addr: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TailscaleIdentity {
    pub(super) self_id: String,
    pub(super) self_ip: IpAddr,
    pub(super) hostname: String,
    pub(super) host_peers: Vec<HostPeer>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct HostPeer {
    pub(super) id: String,
    pub(super) hostname: String,
    pub(super) ip: IpAddr,
    pub(super) online: bool,
    pub(super) direct: bool,
}

pub(super) fn valid_discovered_hostname(value: &str) -> bool {
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
pub(super) struct TailscaleServeStatus {
    #[serde(rename = "TCP", default)]
    pub(super) tcp: HashMap<String, TailscaleTcpHandler>,
    #[serde(rename = "Web", default)]
    pub(super) web: HashMap<String, TailscaleWebHandler>,
    #[serde(rename = "AllowFunnel", default)]
    pub(super) allow_funnel: HashMap<String, bool>,
}

#[derive(Deserialize)]
pub(super) struct TailscaleTcpHandler {
    #[serde(rename = "HTTPS", default)]
    pub(super) https: bool,
}

#[derive(Deserialize)]
pub(super) struct TailscaleWebHandler {
    #[serde(rename = "Handlers", default)]
    pub(super) handlers: HashMap<String, TailscalePathHandler>,
}

#[derive(Deserialize)]
pub(super) struct TailscalePathHandler {
    #[serde(rename = "Proxy", default)]
    pub(super) proxy: String,
}
