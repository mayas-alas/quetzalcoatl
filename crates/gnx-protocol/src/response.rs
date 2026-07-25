use serde::{Deserialize, Serialize};

use crate::version::PROTOCOL_SCHEMA_VERSION;

#[derive(Deserialize, Serialize)]
pub struct OperationResponse {
    pub schema_version: u8,
    pub accepted: bool,
    pub stage: String,
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
