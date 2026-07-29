mod configuration;
mod host_profile;
mod identity;
mod ipc;
mod migration;
mod status;

pub use configuration::{InstallerConfiguration, TailnetName};
pub use host_profile::{DetectedResources, HostProfile, MachineProfile};
pub use identity::WINDOWS_SERVICE_SID;
pub use ipc::{Command, MAX_MESSAGE_BYTES, OperationResponse, PIPE_NAME, Request};
pub use migration::{
    HOST_PROFILE_SCHEMA_VERSION, PERSISTED_STATE_SCHEMA_VERSION, PROTOCOL_SCHEMA_VERSION,
    RUNTIME_GENERATION, RUNTIME_PAYLOAD_CONTRACT,
};
pub use status::{
    Cluster, ComponentHealth, Components, LifecycleStage, NodeRole, OverallHealth, PveUrl,
    StatusResponse,
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_two_status_remains_wire_compatible() {
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/contracts/fixtures/status-member-ready-schema-2.json"
        ))
        .unwrap();
        let value =
            serde_json::to_value(StatusResponse::member_ready("gnx-controller-a".into())).unwrap();
        assert_eq!(value, expected);
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["overall"], "ready");
        assert_eq!(value["stage"], "READY");
        assert_eq!(value["role"], "member");
    }
    #[test]
    fn named_pipe_command_set_is_unchanged() {
        let expected =
            include_str!("../../../tests/contracts/fixtures/request-status-schema-2.json").trim();
        let status = serde_json::to_string(&Request {
            command: Command::Status,
            configuration: None,
        })
        .unwrap();
        assert_eq!(status, expected);
    }

    #[test]
    fn invalid_closed_status_values_fail_deserialization() {
        assert!(serde_json::from_str::<LifecycleStage>(r#""UNKNOWN""#).is_err());
        assert!(serde_json::from_str::<PveUrl>(r#""http://localhost:8006/""#).is_err());
        assert!(PveUrl::parse("https://gnx-controller-a.example.com/").is_err());
    }
}
