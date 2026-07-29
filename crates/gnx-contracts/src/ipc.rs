use crate::{InstallerConfiguration, LifecycleStage, PROTOCOL_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};

pub const PIPE_NAME: &str = r"\\.\pipe\Quetzalcoatl";
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024;

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

#[derive(Deserialize, Serialize)]
pub struct OperationResponse {
    pub schema_version: u8,
    pub accepted: bool,
    pub stage: LifecycleStage,
    pub error_code: Option<String>,
    pub message: Option<String>,
}
impl OperationResponse {
    pub fn accepted(stage: &str) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            accepted: true,
            stage: stage.into(),
            error_code: None,
            message: None,
        }
    }
    pub fn rejected(error_code: &str, message: &str) -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            accepted: false,
            stage: "CONFIGURATION_REJECTED".into(),
            error_code: Some(error_code.into()),
            message: Some(message.into()),
        }
    }
}
