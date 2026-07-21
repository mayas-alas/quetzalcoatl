use std::fs;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

const STATE_NAME: &str = "state.json";
const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedState {
    pub schema_version: u8,
    pub stage: String,
    pub role: PersistedRole,
    pub self_id: String,
    pub self_ip: IpAddr,
    pub controller: ControllerIdentity,
    pub tailnet: String,
    pub install_garage: bool,
    pub install_forgejo: bool,
    #[serde(default)]
    pub member: Option<MemberIdentity>,
    #[serde(default)]
    pub cluster_join: ClusterJoinState,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PersistedRole {
    Controller,
    Member,
}

impl PersistedRole {
    pub fn is_controller(&self) -> bool {
        matches!(self, Self::Controller)
    }

    pub fn is_member(&self) -> bool {
        matches!(self, Self::Member)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerIdentity {
    pub id: String,
    pub hostname: String,
    pub ip: IpAddr,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemberIdentity {
    pub id: String,
    pub hostname: String,
    pub ip: IpAddr,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterJoinState {
    #[default]
    NotApplicable,
    NotStarted,
    Joining,
    Joined,
}

impl PersistedState {
    pub fn controller(
        self_id: String,
        self_ip: IpAddr,
        hostname: String,
        tailnet: String,
        install_garage: bool,
        install_forgejo: bool,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            stage: "ROLE_RESOLVED".into(),
            role: PersistedRole::Controller,
            controller: ControllerIdentity {
                id: self_id.clone(),
                hostname,
                ip: self_ip,
            },
            self_id,
            self_ip,
            tailnet,
            install_garage,
            install_forgejo,
            member: None,
            cluster_join: ClusterJoinState::NotApplicable,
        }
    }

    pub fn member(
        self_id: String,
        self_ip: IpAddr,
        hostname: String,
        controller: ControllerIdentity,
        tailnet: String,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            stage: "ROLE_RESOLVED".into(),
            role: PersistedRole::Member,
            member: Some(MemberIdentity {
                id: self_id.clone(),
                hostname,
                ip: self_ip,
            }),
            self_id,
            self_ip,
            controller,
            tailnet,
            install_garage: false,
            install_forgejo: false,
            cluster_join: ClusterJoinState::NotStarted,
        }
    }
}

pub fn store(state: &PersistedState) -> Result<(), StateError> {
    validate(state)?;
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|_| StateError::new("cannot encode persisted state"))?;
    let path = state_path()?;
    crate::secrets::atomic_write(&path, &bytes).map_err(StateError::from_storage)?;
    let verified = load_from(&path)?;
    if &verified != state {
        return Err(StateError::new(
            "state read-after-write verification failed",
        ));
    }
    Ok(())
}

pub fn load_optional() -> Result<Option<PersistedState>, StateError> {
    let path = state_path()?;
    match fs::read(&path) {
        Ok(bytes) => parse(&bytes).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StateError::io("cannot read persisted state", &error)),
    }
}

fn load_from(path: &std::path::Path) -> Result<PersistedState, StateError> {
    let bytes =
        fs::read(path).map_err(|error| StateError::io("cannot read persisted state", &error))?;
    parse(&bytes)
}

fn parse(bytes: &[u8]) -> Result<PersistedState, StateError> {
    let state = serde_json::from_slice(bytes)
        .map_err(|_| StateError::new("state.json has invalid data"))?;
    validate(&state)?;
    Ok(state)
}

fn validate(state: &PersistedState) -> Result<(), StateError> {
    if state.schema_version != SCHEMA_VERSION
        || state.stage.is_empty()
        || state.stage.len() > 64
        || !state
            .stage
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        || state.self_id.is_empty()
        || state.self_id.len() > 128
        || !state.self_id.bytes().all(|byte| byte.is_ascii_graphic())
        || !valid_identity_id(&state.controller.id)
        || !valid_hostname(&state.controller.hostname)
        || !valid_tailnet(&state.tailnet)
    {
        return Err(StateError::new(
            "state.json does not satisfy the controller identity contract",
        ));
    }
    match (&state.role, &state.member) {
        (PersistedRole::Controller, None)
            if state.controller.id == state.self_id
                && state.controller.ip == state.self_ip
                && state.cluster_join == ClusterJoinState::NotApplicable => {}
        (PersistedRole::Member, Some(member))
            if member.id == state.self_id
                && member.ip == state.self_ip
                && valid_node_hostname(&member.hostname)
                && state.controller.id != state.self_id
                && state.controller.ip != state.self_ip
                && !state.install_garage
                && !state.install_forgejo
                && valid_member_stage(&state.stage, &state.cluster_join) => {}
        _ => {
            return Err(StateError::new(
                "state.json does not satisfy the persisted role contract",
            ));
        }
    }
    Ok(())
}

fn valid_member_stage(stage: &str, cluster_join: &ClusterJoinState) -> bool {
    matches!(
        (stage, cluster_join),
        ("ROLE_RESOLVED", ClusterJoinState::NotStarted)
            | ("MEMBER_JOINING", ClusterJoinState::Joining)
            | ("READY", ClusterJoinState::Joined)
    )
}

fn valid_hostname(value: &str) -> bool {
    value.len() <= 63
        && value.starts_with("gnx-controller-")
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_node_hostname(value: &str) -> bool {
    value.len() <= 63
        && value.starts_with("gnx-member-")
        && value.len() > "gnx-member-".len()
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_identity_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_tailnet(value: &str) -> bool {
    value.len() >= 7
        && value.len() <= 253
        && value.ends_with(".ts.net")
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn state_path() -> Result<std::path::PathBuf, StateError> {
    crate::secrets::product_root()
        .map(|root| root.join(STATE_NAME))
        .map_err(StateError::from_storage)
}

#[derive(Debug)]
pub struct StateError {
    message: String,
}

impl StateError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn io(operation: &'static str, error: &std::io::Error) -> Self {
        Self::new(format!(
            "{operation} (OS {})",
            error.raw_os_error().unwrap_or_default()
        ))
    }

    fn from_storage(error: crate::secrets::ConfigurationError) -> Self {
        Self::new(error.message())
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller_state() -> PersistedState {
        PersistedState::controller(
            "node-id-123".into(),
            "100.100.10.20".parse().expect("IP"),
            "gnx-controller-node-id-123".into(),
            "tetra-balance.ts.net".into(),
            true,
            true,
        )
    }

    #[test]
    fn controller_state_round_trips_without_secrets() {
        let bytes = serde_json::to_vec(&controller_state()).expect("serialize state");
        let state = parse(&bytes).expect("parse state");
        assert_eq!(state, controller_state());
        let json = String::from_utf8(bytes).expect("UTF-8");
        assert!(!json.contains("auth_key"));
        assert!(!json.contains("password"));
    }

    #[test]
    fn state_rejects_identity_drift() {
        let mut state = controller_state();
        state.controller.id = "different-id".into();
        assert!(validate(&state).is_err());
    }

    #[test]
    fn legacy_controller_state_deserializes_with_member_defaults() {
        let legacy = r#"{
            "schema_version": 1,
            "stage": "ROLE_RESOLVED",
            "role": "controller",
            "self_id": "node-id-123",
            "self_ip": "100.100.10.20",
            "controller": {
                "id": "node-id-123",
                "hostname": "gnx-controller-node-id-123",
                "ip": "100.100.10.20"
            },
            "tailnet": "tetra-balance.ts.net",
            "install_garage": true,
            "install_forgejo": true
        }"#;
        let state = parse(legacy.as_bytes()).expect("parse legacy state");
        assert_eq!(state.member, None);
        assert_eq!(state.cluster_join, ClusterJoinState::NotApplicable);
    }

    #[test]
    fn member_state_round_trips_and_keeps_its_controller() {
        let controller = ControllerIdentity {
            id: "controller-id".into(),
            hostname: "gnx-controller-controller-id".into(),
            ip: "100.100.10.20".parse().expect("IP"),
        };
        let state = PersistedState::member(
            "member-id".into(),
            "100.100.10.21".parse().expect("IP"),
            "gnx-member-member-id".into(),
            controller,
            "tetra-balance.ts.net".into(),
        );
        let bytes = serde_json::to_vec(&state).expect("serialize state");
        assert_eq!(parse(&bytes).expect("parse member state"), state);
    }

    #[test]
    fn member_state_rejects_invalid_identity_data() {
        let controller = ControllerIdentity {
            id: "controller-id".into(),
            hostname: "gnx-controller-controller-id".into(),
            ip: "100.100.10.20".parse().expect("IP"),
        };
        let member = || {
            PersistedState::member(
                "member-id".into(),
                "100.100.10.21".parse().expect("IP"),
                "gnx-member-member-id".into(),
                controller.clone(),
                "tetra-balance.ts.net".into(),
            )
        };

        for controller_id in [
            String::new(),
            "controller id".into(),
            "controller\n-id".into(),
            "a".repeat(129),
        ] {
            let mut state = member();
            state.controller.id = controller_id.clone();
            assert!(validate(&state).is_err(), "accepted {controller_id:?}");
        }

        for hostname in ["member-id", "gnx-member-", "gnx-member-member_id"] {
            let mut state = member();
            state.member.as_mut().expect("member identity").hostname = hostname.into();
            assert!(validate(&state).is_err(), "accepted {hostname:?}");
        }
    }

    #[test]
    fn role_constructors_distinguish_controller_and_member() {
        let controller = controller_state();
        let member = PersistedState::member(
            "member-id".into(),
            "100.100.10.21".parse().expect("IP"),
            "gnx-member-member-id".into(),
            controller.controller.clone(),
            controller.tailnet.clone(),
        );
        assert!(controller.role.is_controller());
        assert!(member.role.is_member());
    }

    #[test]
    fn member_stage_must_match_its_join_checkpoint() {
        let controller = controller_state();
        let mut member = PersistedState::member(
            "member-id".into(),
            "100.100.10.21".parse().expect("IP"),
            "gnx-member-member-id".into(),
            controller.controller,
            controller.tailnet,
        );
        assert!(validate(&member).is_ok());

        member.stage = "MEMBER_JOINING".into();
        assert!(validate(&member).is_err());
        member.cluster_join = ClusterJoinState::Joining;
        assert!(validate(&member).is_ok());

        member.stage = "READY".into();
        assert!(validate(&member).is_err());
        member.cluster_join = ClusterJoinState::Joined;
        assert!(validate(&member).is_ok());
    }
}
