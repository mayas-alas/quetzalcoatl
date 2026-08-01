mod configuration;
mod host_profile;
mod identity;
mod ipc;
mod migration;
mod status;

pub use configuration::{
    InstallerConfiguration, PLATFORM_CONFIGURATION_SCHEMA_VERSION, PlatformConfiguration,
    TailnetName,
};
pub use host_profile::{DetectedResources, HostProfile, MachineProfile};
pub use identity::WINDOWS_SERVICE_SID;
pub use ipc::{
    Command, FORGEJO_ADMIN_USERNAME, ForgejoAdminResponse, ForgejoAdminStage, MAX_MESSAGE_BYTES,
    OperationResponse, OperationStage, PIPE_NAME, Request,
};
pub use migration::{
    HOST_PROFILE_SCHEMA_VERSION, PERSISTED_STATE_SCHEMA_VERSION, PROTOCOL_SCHEMA_VERSION,
    RUNTIME_GENERATION, RUNTIME_PAYLOAD_CONTRACT,
};
pub use status::{
    Cluster, ComponentHealth, Components, LifecycleStage, NodeRole, OverallHealth, PlatformHealth,
    PlatformStatus, PlatformUrl, PveUrl, StatusResponse,
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
    fn protocol_two_status_request_remains_wire_compatible() {
        let expected =
            include_str!("../../../tests/contracts/fixtures/request-status-schema-2.json").trim();
        let status = serde_json::to_string(&Request {
            command: Command::Status,
            configuration: None,
            platform_configuration: None,
        })
        .unwrap();
        assert_eq!(status, expected);
    }

    #[test]
    fn platform_configuration_is_a_closed_protocol_two_operation() {
        let value = serde_json::to_value(Request {
            command: Command::ConfigurePlatform,
            configuration: None,
            platform_configuration: Some(PlatformConfiguration::new(
                "tskey-auth-k-example-not-a-real-key".into(),
            )),
        })
        .unwrap();
        assert_eq!(value["command"], "configure_platform");
        assert_eq!(value["platform_configuration"]["schema_version"], 1);
        assert!(value.get("configuration").is_none());
    }

    #[test]
    fn invalid_closed_status_values_fail_deserialization() {
        assert!(serde_json::from_str::<LifecycleStage>(r#""UNKNOWN""#).is_err());
        assert!(serde_json::from_str::<OperationStage>(r#""unknown""#).is_err());
        assert!(serde_json::from_str::<PveUrl>(r#""http://localhost:8006/""#).is_err());
        assert!(PveUrl::parse("https://gnx-controller-a.example.com/").is_err());
        assert!(PlatformUrl::parse("https://forgejo.example.com/").is_err());
        assert!(PlatformUrl::parse("https://gnx-forgejo.tetra-balance.ts.net/").is_ok());
    }

    #[test]
    fn operation_responses_use_their_own_closed_stage_taxonomy() {
        let accepted = OperationResponse::accepted(OperationStage::PlatformConfigurationStored);
        let rejected = OperationResponse::rejected("INVALID", "invalid");
        assert_eq!(
            serde_json::to_value(accepted).unwrap()["stage"],
            "platform_configuration_stored"
        );
        assert_eq!(serde_json::to_value(rejected).unwrap()["stage"], "rejected");
    }

    #[test]
    fn forgejo_admin_responses_use_a_distinct_secret_contract() {
        let shown = ForgejoAdminResponse::accepted("a".repeat(48), false);
        let reset = ForgejoAdminResponse::accepted("b".repeat(48), true);
        let rejected = ForgejoAdminResponse::rejected("DENIED", "denied");
        assert_eq!(shown.stage, ForgejoAdminStage::Shown);
        assert_eq!(shown.username.as_deref(), Some(FORGEJO_ADMIN_USERNAME));
        assert_eq!(reset.stage, ForgejoAdminStage::Reset);
        assert_eq!(rejected.stage, ForgejoAdminStage::Rejected);
        assert!(rejected.password.is_none());
    }

    #[test]
    fn forgejo_admin_requests_are_closed_protocol_two_operations() {
        for (command, expected) in [
            (Command::ForgejoAdminShow, "forgejo_admin_show"),
            (Command::ForgejoAdminReset, "forgejo_admin_reset"),
        ] {
            let value = serde_json::to_value(Request {
                command,
                configuration: None,
                platform_configuration: None,
            })
            .unwrap();
            assert_eq!(value["command"], expected);
            assert!(value.get("configuration").is_none());
            assert!(value.get("platform_configuration").is_none());
        }
    }
}
