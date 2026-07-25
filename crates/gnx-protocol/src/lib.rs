mod request;
mod response;
mod status;
mod version;

pub use request::{Command, InstallerConfiguration, Request};
pub use response::OperationResponse;
pub use status::{Cluster, Components, StatusResponse};
pub use version::{MAX_MESSAGE_BYTES, PIPE_NAME, PROTOCOL_SCHEMA_VERSION};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_ready_does_not_claim_platform_ready() {
        let status = StatusResponse::service_ready();
        assert_eq!(status.schema_version, PROTOCOL_SCHEMA_VERSION);
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

    #[test]
    fn member_ready_serializes_the_final_platform_contract() {
        let status = StatusResponse::member_ready("gnx-controller-a".into());
        let json = serde_json::to_value(&status).expect("serialize status");
        assert_eq!(json["overall"], "ready");
        assert_eq!(json["stage"], "READY");
        assert_eq!(json["role"], "member");
        assert_eq!(json["cluster"]["joined"], true);
        assert_eq!(json["cluster"]["quorate"], true);
        assert_eq!(json["components"]["service"], "ready");
        assert_eq!(json["components"]["wsl"], "ready");
        assert_eq!(json["components"]["podman_machine"], "ready");
        assert_eq!(json["components"]["kvm"], "ready");
        assert_eq!(json["components"]["tailscale"], "ready");
        assert_eq!(json["components"]["tailscale_serve"], "ready");
        assert_eq!(json["components"]["proxmox"], "ready");
        assert!(json.get("services").is_none());
    }
}
