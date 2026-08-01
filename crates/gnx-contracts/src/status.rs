use crate::migration::PROTOCOL_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    Controller,
    Member,
}

impl NodeRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Controller => "controller",
            Self::Member => "member",
        }
    }
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for NodeRole {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentHealth {
    Pending,
    Ready,
    Failed,
}

impl ComponentHealth {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

impl From<&str> for ComponentHealth {
    fn from(value: &str) -> Self {
        match value {
            "pending" => Self::Pending,
            "ready" => Self::Ready,
            "failed" => Self::Failed,
            _ => panic!("invalid component health: {value}"),
        }
    }
}
impl From<String> for ComponentHealth {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
impl std::fmt::Display for ComponentHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl PartialEq<&str> for ComponentHealth {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OverallHealth {
    Pending,
    Ready,
    Failed,
}
impl OverallHealth {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}
impl From<&str> for OverallHealth {
    fn from(value: &str) -> Self {
        match value {
            "pending" => Self::Pending,
            "ready" => Self::Ready,
            "failed" => Self::Failed,
            _ => panic!("invalid overall health: {value}"),
        }
    }
}
impl From<String> for OverallHealth {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
impl std::fmt::Display for OverallHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl PartialEq<&str> for OverallHealth {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformHealth {
    WaitingConfiguration,
    Reconciling,
    Ready,
    Failed,
}

impl PlatformHealth {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WaitingConfiguration => "waiting_configuration",
            Self::Reconciling => "reconciling",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for PlatformHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LifecycleStage(pub String);
impl LifecycleStage {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        const VALID: &[&str] = &[
            "SERVICE_READY",
            "RUNTIME_IDENTITY",
            "HOST_PROFILE_LOADING",
            "WSL_PREPARING",
            "MACHINE_PREPARING",
            "MACHINE_NETWORK_PREPARING",
            "MACHINE_READY",
            "KVM_CHECKING",
            "KVM_READY",
            "PAYLOAD_APPLYING",
            "CONFIGURATION_WAITING",
            "TAILSCALE_ENROLLING",
            "TAILSCALE_CHECKING",
            "ROLE_DISCOVERING",
            "ROLE_RESOLVED",
            "PVE_IDENTITY_PREPARING",
            "PROXMOX_STARTING",
            "POD_NETWORK_PREPARING",
            "PROXMOX_CHECKING",
            "PROXMOX_READY",
            "PVE_CREDENTIAL_APPLYING",
            "TAILSCALE_SERVE_APPLYING",
            "TAILSCALE_SERVE_CHECKING",
            "TAILSCALE_READY",
            "CONTROLLER_CLUSTER_PRECHECK",
            "CONTROLLER_CLUSTER_CREATING",
            "CONTROLLER_CLUSTER_CHECKING",
            "CONTROLLER_CLUSTER_READY",
            "PVE_IDENTITY_CHECKING",
            "MEMBER_PREPARING",
            "MEMBER_AUTHORIZING",
            "MEMBER_JOINING",
            "MEMBER_VERIFYING",
            "MEMBER_CONFIRMING",
            "READY",
            "FAILED",
        ];
        VALID
            .contains(&value.as_str())
            .then_some(Self(value))
            .ok_or("unknown lifecycle stage")
    }
}
impl<'de> Deserialize<'de> for LifecycleStage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}
impl From<&str> for LifecycleStage {
    fn from(value: &str) -> Self {
        Self::parse(value).expect("invalid lifecycle stage")
    }
}
impl From<String> for LifecycleStage {
    fn from(value: String) -> Self {
        Self::parse(value).expect("invalid lifecycle stage")
    }
}
impl std::fmt::Display for LifecycleStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl PartialEq<&str> for LifecycleStage {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PveUrl(pub String);
impl PveUrl {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        let host = value
            .strip_prefix("https://")
            .and_then(|value| value.strip_suffix('/'))
            .ok_or("PVE URL must use HTTPS and end with a slash")?;
        if host.is_empty()
            || host.contains(['/', '\\', '@', ':', '[', ']'])
            || host.eq_ignore_ascii_case("localhost")
            || host.parse::<std::net::IpAddr>().is_ok()
            || !host.ends_with(".ts.net")
        {
            return Err("PVE URL host is not an allowed tailnet DNS name");
        }
        let local = host.split('.').next().ok_or("PVE URL host is absent")?;
        if !(local.starts_with("gnx-controller-") || local.starts_with("gnx-member-"))
            || !host.split('.').all(valid_dns_label)
        {
            return Err("PVE URL host is not a GNX node");
        }
        Ok(Self(value))
    }
}
fn valid_dns_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
impl<'de> Deserialize<'de> for PveUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}
impl std::ops::Deref for PveUrl {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for PveUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PlatformUrl(pub String);

impl PlatformUrl {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        let host = value
            .strip_prefix("https://")
            .and_then(|value| value.strip_suffix('/'))
            .ok_or("platform URL must use HTTPS and end with a slash")?;
        if host.is_empty()
            || host.contains(['/', '\\', '@', ':', '[', ']'])
            || !host.ends_with(".ts.net")
            || !host.split('.').all(valid_dns_label)
        {
            return Err("platform URL host is not an allowed tailnet DNS name");
        }
        let local = host
            .split('.')
            .next()
            .ok_or("platform URL host is absent")?;
        if !local.starts_with("gnx-") || local.len() <= 4 {
            return Err("platform URL host is not a GNX service");
        }
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for PlatformUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for PlatformUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct StatusResponse {
    pub schema_version: u8,
    pub overall: OverallHealth,
    pub stage: LifecycleStage,
    pub role: Option<NodeRole>,
    pub controller: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pve_url: Option<PveUrl>,
    pub components: Components,
    pub cluster: Cluster,
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<PlatformStatus>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Components {
    pub service: ComponentHealth,
    pub wsl: ComponentHealth,
    pub podman_machine: ComponentHealth,
    pub kvm: ComponentHealth,
    pub tailscale: ComponentHealth,
    pub tailscale_serve: ComponentHealth,
    pub proxmox: ComponentHealth,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Cluster {
    pub joined: bool,
    pub quorate: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PlatformStatus {
    pub health: PlatformHealth,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forgejo_url: Option<PlatformUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl PlatformStatus {
    pub fn waiting_configuration() -> Self {
        Self {
            health: PlatformHealth::WaitingConfiguration,
            forgejo_url: None,
            last_error: None,
        }
    }
}

impl StatusResponse {
    pub fn service_ready() -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            overall: OverallHealth::Pending,
            stage: "SERVICE_READY".into(),
            role: None,
            controller: None,
            pve_url: None,
            components: Components {
                service: ComponentHealth::Ready,
                wsl: ComponentHealth::Pending,
                podman_machine: ComponentHealth::Pending,
                kvm: ComponentHealth::Pending,
                tailscale: ComponentHealth::Pending,
                tailscale_serve: ComponentHealth::Pending,
                proxmox: ComponentHealth::Pending,
            },
            cluster: Cluster {
                joined: false,
                quorate: false,
            },
            last_error: None,
            platform: None,
        }
    }

    pub fn member_ready(controller: String) -> Self {
        let mut status = Self::service_ready();
        status.overall = OverallHealth::Ready;
        status.stage = "READY".into();
        status.role = Some(NodeRole::Member);
        status.controller = Some(controller);
        status.components.wsl = ComponentHealth::Ready;
        status.components.podman_machine = ComponentHealth::Ready;
        status.components.kvm = ComponentHealth::Ready;
        status.components.tailscale = ComponentHealth::Ready;
        status.components.tailscale_serve = ComponentHealth::Ready;
        status.components.proxmox = ComponentHealth::Ready;
        status.cluster.joined = true;
        status.cluster.quorate = true;
        status
    }
}
