use crate::{InstallerConfiguration, PROTOCOL_SCHEMA_VERSION, PlatformConfiguration};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const PIPE_NAME: &str = r"\\.\pipe\Quetzalcoatl";
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Deserialize, Serialize)]
pub struct Request {
    pub command: Command,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<InstallerConfiguration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_configuration: Option<PlatformConfiguration>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Status,
    Configure,
    ConfigurePlatform,
    ForgejoAdminShow,
    ForgejoAdminReset,
}

pub const FORGEJO_ADMIN_USERNAME: &str = "gnx-admin";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Zeroize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoAdminStage {
    Shown,
    Reset,
    Rejected,
}

#[derive(Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct ForgejoAdminResponse {
    pub schema_version: u8,
    pub accepted: bool,
    pub stage: ForgejoAdminStage,
    pub username: Option<String>,
    pub password: Option<String>,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

impl ForgejoAdminResponse {
    pub fn accepted(password: String, reset: bool) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            accepted: true,
            stage: if reset {
                ForgejoAdminStage::Reset
            } else {
                ForgejoAdminStage::Shown
            },
            username: Some(FORGEJO_ADMIN_USERNAME.into()),
            password: Some(password),
            error_code: None,
            message: None,
        }
    }

    pub fn rejected(error_code: &str, message: &str) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            accepted: false,
            stage: ForgejoAdminStage::Rejected,
            username: None,
            password: None,
            error_code: Some(error_code.into()),
            message: Some(message.into()),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct OperationResponse {
    pub schema_version: u8,
    pub accepted: bool,
    pub stage: OperationStage,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStage {
    ConfigurationStored,
    PlatformConfigurationStored,
    Rejected,
}

impl std::fmt::Display for OperationStage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ConfigurationStored => "configuration_stored",
            Self::PlatformConfigurationStored => "platform_configuration_stored",
            Self::Rejected => "rejected",
        })
    }
}

impl OperationResponse {
    pub fn accepted(stage: OperationStage) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            accepted: true,
            stage,
            error_code: None,
            message: None,
        }
    }
    pub fn rejected(error_code: &str, message: &str) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            accepted: false,
            stage: OperationStage::Rejected,
            error_code: Some(error_code.into()),
            message: Some(message.into()),
        }
    }
}
