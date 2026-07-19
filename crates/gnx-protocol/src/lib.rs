use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const PIPE_NAME: &str = r"\\.\pipe\Quetzalcoatl";
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct InstallerConfiguration {
    pub tailnet: String,
    pub auth_key: String,
    pub pve_root_password: String,
    pub install_garage: bool,
    pub install_forgejo: bool,
}

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
            schema_version: 1,
            accepted: true,
            stage: stage.into(),
            error_code: None,
            message: None,
        }
    }

    pub fn rejected(error_code: &str, message: &str) -> Self {
        Self {
            schema_version: 1,
            accepted: false,
            stage: "CONFIGURATION_REJECTED".into(),
            error_code: Some(error_code.into()),
            message: Some(message.into()),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct StatusResponse {
    pub schema_version: u8,
    pub overall: String,
    pub stage: String,
    pub role: Option<String>,
    pub controller: Option<String>,
    pub components: Components,
    pub cluster: Cluster,
    pub services: Services,
    pub last_error: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Components {
    pub service: String,
    pub wsl: String,
    pub podman_machine: String,
    pub kvm: String,
    pub tailscale: String,
    pub tailscale_serve: String,
    pub proxmox: String,
    pub opentofu: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Cluster {
    pub joined: bool,
    pub quorate: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Services {
    pub garage: String,
    pub forgejo: String,
}

impl StatusResponse {
    pub fn service_ready() -> Self {
        Self {
            schema_version: 1,
            overall: "pending".into(),
            stage: "SERVICE_READY".into(),
            role: None,
            controller: None,
            components: Components {
                service: "ready".into(),
                wsl: "pending".into(),
                podman_machine: "pending".into(),
                kvm: "pending".into(),
                tailscale: "pending".into(),
                tailscale_serve: "pending".into(),
                proxmox: "pending".into(),
                opentofu: "pending".into(),
            },
            cluster: Cluster {
                joined: false,
                quorate: false,
            },
            services: Services {
                garage: "pending".into(),
                forgejo: "pending".into(),
            },
            last_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_ready_does_not_claim_increment_ready() {
        let status = StatusResponse::service_ready();
        assert_eq!(status.overall, "pending");
        assert_eq!(status.stage, "SERVICE_READY");
        assert_eq!(status.components.service, "ready");
        assert_eq!(status.components.kvm, "pending");
    }

    #[test]
    fn status_request_has_no_configuration_payload() {
        let request = Request {
            command: Command::Status,
            configuration: None,
        };
        assert_eq!(
            serde_json::to_string(&request).expect("serialize status request"),
            r#"{"command":"status"}"#
        );
    }
}
