use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Deserialize, Serialize)]
pub struct Request {
    pub command: Command,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<InstallerConfiguration>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Status,
    Configure,
}

#[derive(Deserialize, PartialEq, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct InstallerConfiguration {
    pub tailnet: String,
    pub auth_key: String,
    pub pve_root_password: String,
}
