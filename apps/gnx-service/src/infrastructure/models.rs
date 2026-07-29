use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct RuntimeManifest {
    pub(crate) payload_version: u64,
    pub(crate) components: Vec<RuntimeComponent>,
    pub(crate) files: Vec<RuntimeFile>,
}

#[derive(Deserialize)]
pub(crate) struct RuntimeFile {
    pub(crate) path: String,
    pub(crate) mode: String,
    pub(crate) sha256: String,
}

#[derive(Deserialize)]
pub(crate) struct RuntimeComponent {
    pub(crate) id: String,
    pub(crate) kind: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) source_ref: Option<String>,
    pub(crate) source_commit: Option<String>,
    pub(crate) image: Option<String>,
    pub(crate) index_digest: Option<String>,
    pub(crate) manifest_digest: Option<String>,
    pub(crate) layer_digest: Option<String>,
    pub(crate) artifact: Option<String>,
    pub(crate) artifact_url: Option<String>,
    pub(crate) artifact_size: Option<u64>,
    pub(crate) platform: Option<RuntimePlatform>,
}

#[derive(Deserialize)]
pub(crate) struct RuntimePlatform {
    pub(crate) os: String,
    pub(crate) architecture: String,
    pub(crate) disk_type: String,
}

pub(crate) struct MachineImage {
    pub(crate) artifact: String,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

#[derive(Clone, Copy)]
pub(crate) struct PayloadSpec {
    pub(crate) relative_path: &'static str,
    pub(crate) destination: &'static str,
    pub(crate) mode: &'static str,
}

impl PayloadSpec {
    pub(crate) const fn new(
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

pub(crate) struct LockedPayloadFile {
    pub(crate) relative_path: String,
    pub(crate) destination: String,
    pub(crate) mode: String,
    pub(crate) sha256: String,
}

pub(crate) struct PayloadFile {
    pub(crate) destination: String,
    pub(crate) mode: String,
    pub(crate) sha256: String,
    pub(crate) contents: Vec<u8>,
}

#[derive(Deserialize)]
pub(crate) struct MachineListEntry {
    #[serde(rename = "Name")]
    pub(crate) name: String,
    #[serde(rename = "VMType")]
    pub(crate) vm_type: String,
}

#[derive(Deserialize)]
pub(crate) struct MachineInspect {
    #[serde(rename = "Name")]
    pub(crate) name: String,
    #[serde(rename = "State")]
    pub(crate) state: String,
    #[serde(rename = "Rootful")]
    pub(crate) rootful: bool,
    #[serde(rename = "Resources")]
    pub(crate) resources: MachineResources,
}

#[derive(Deserialize)]
pub(crate) struct MachineResources {
    #[serde(rename = "CPUs")]
    pub(crate) cpus: u64,
    #[serde(rename = "Memory")]
    pub(crate) memory: u64,
    #[serde(rename = "DiskSize")]
    pub(crate) disk_size: u64,
}

#[derive(Deserialize)]
pub(crate) struct TailscaleStatus {
    #[serde(rename = "BackendState")]
    pub(crate) backend_state: String,
    #[serde(rename = "Health", default)]
    pub(crate) health: Vec<String>,
    #[serde(rename = "TUN")]
    pub(crate) tun: bool,
    #[serde(rename = "TailscaleIPs")]
    pub(crate) tailscale_ips: Vec<String>,
    #[serde(rename = "Self")]
    pub(crate) self_node: Option<TailscalePeer>,
    #[serde(rename = "CurrentTailnet")]
    pub(crate) current_tailnet: Option<TailscaleTailnet>,
    #[serde(rename = "CertDomains")]
    pub(crate) cert_domains: Vec<String>,
    #[serde(rename = "Peer", default)]
    pub(crate) peers: HashMap<String, TailscalePeer>,
}

#[derive(Deserialize)]
pub(crate) struct TailscaleTailnet {
    #[serde(rename = "MagicDNSSuffix")]
    pub(crate) magic_dns_suffix: String,
    #[serde(rename = "MagicDNSEnabled")]
    pub(crate) magic_dns_enabled: bool,
}

#[derive(Deserialize)]
pub(crate) struct TailscalePeer {
    #[serde(rename = "ID", default)]
    pub(crate) id: String,
    #[serde(rename = "HostName")]
    pub(crate) host_name: String,
    #[serde(rename = "DNSName")]
    pub(crate) dns_name: String,
    #[serde(rename = "Tags", default)]
    pub(crate) tags: Vec<String>,
    #[serde(rename = "Expired", default)]
    pub(crate) expired: bool,
    #[serde(rename = "Online", default)]
    pub(crate) online: bool,
    #[serde(rename = "TailscaleIPs", default)]
    pub(crate) tailscale_ips: Vec<String>,
    #[serde(rename = "CurAddr", default)]
    pub(crate) cur_addr: String,
}

#[derive(Deserialize)]
pub(crate) struct TailscaleServeStatus {
    #[serde(rename = "TCP", default)]
    pub(crate) tcp: HashMap<String, TailscaleTcpHandler>,
    #[serde(rename = "Web", default)]
    pub(crate) web: HashMap<String, TailscaleWebHandler>,
    #[serde(rename = "AllowFunnel", default)]
    pub(crate) allow_funnel: HashMap<String, bool>,
}

#[derive(Deserialize)]
pub(crate) struct TailscaleTcpHandler {
    #[serde(rename = "HTTPS", default)]
    pub(crate) https: bool,
}

#[derive(Deserialize)]
pub(crate) struct TailscaleWebHandler {
    #[serde(rename = "Handlers", default)]
    pub(crate) handlers: HashMap<String, TailscalePathHandler>,
}

#[derive(Deserialize)]
pub(crate) struct TailscalePathHandler {
    #[serde(rename = "Proxy", default)]
    pub(crate) proxy: String,
}
