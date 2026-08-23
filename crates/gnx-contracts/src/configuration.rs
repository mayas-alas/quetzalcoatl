use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const PLATFORM_CONFIGURATION_SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Zeroize)]
#[serde(transparent)]
pub struct TailnetName(pub String);

impl AsRef<str> for TailnetName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::ops::Deref for TailnetName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::fmt::Display for TailnetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl From<String> for TailnetName {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for TailnetName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Deserialize, PartialEq, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct InstallerConfiguration {
    pub tailnet: TailnetName,
    pub auth_key: String,
    pub pve_root_password: String,
}

#[derive(Deserialize, PartialEq, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct PlatformConfiguration {
    pub schema_version: u8,
    pub tailscale_auth_key: String,
}

impl PlatformConfiguration {
    pub fn new(tailscale_auth_key: String) -> Self {
        Self {
            schema_version: PLATFORM_CONFIGURATION_SCHEMA_VERSION,
            tailscale_auth_key,
        }
    }
}
