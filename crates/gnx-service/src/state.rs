use std::fs;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

const STATE_NAME: &str = "state.json";
const SCHEMA_VERSION: u8 = 2;
const PREVIOUS_SCHEMA_VERSION: u8 = 1;

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

#[derive(Deserialize)]
struct SchemaOneState {
    schema_version: u8,
    stage: String,
    role: PersistedRole,
    self_id: String,
    self_ip: IpAddr,
    controller: ControllerIdentity,
    tailnet: String,
    #[serde(default)]
    member: Option<MemberIdentity>,
    #[serde(default)]
    cluster_join: ClusterJoinState,
}

impl PersistedState {
    pub fn controller(self_id: String, self_ip: IpAddr, hostname: String, tailnet: String) -> Self {
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
    let verified = load_current_from(&path)?;
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
        Ok(bytes) => {
            let (state, migrated) = decode(&bytes)?;
            if migrated {
                store(&state)?;
            }
            Ok(Some(state))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StateError::io("cannot read persisted state", &error)),
    }
}

pub fn reset_runtime_checkpoint() -> Result<(), StateError> {
    let Some(mut state) = load_optional()? else {
        return Ok(());
    };
    normalize_runtime_checkpoint(&mut state);
    store(&state)
}

fn normalize_runtime_checkpoint(state: &mut PersistedState) {
    state.stage = "ROLE_RESOLVED".into();
    state.cluster_join = if state.role.is_controller() {
        ClusterJoinState::NotApplicable
    } else {
        ClusterJoinState::NotStarted
    };
}

fn load_current_from(path: &std::path::Path) -> Result<PersistedState, StateError> {
    let bytes =
        fs::read(path).map_err(|error| StateError::io("cannot read persisted state", &error))?;
    let state: PersistedState = serde_json::from_slice(&bytes)
        .map_err(|_| StateError::new("state.json has invalid current data"))?;
    validate(&state)?;
    Ok(state)
}

fn decode(bytes: &[u8]) -> Result<(PersistedState, bool), StateError> {
    if let Ok(state) = serde_json::from_slice::<PersistedState>(bytes) {
        validate(&state)?;
        return Ok((state, false));
    }

    let previous: SchemaOneState = serde_json::from_slice(bytes)
        .map_err(|_| StateError::new("state.json has invalid data"))?;
    if previous.schema_version != PREVIOUS_SCHEMA_VERSION {
        return Err(StateError::new("state.json schema version is unsupported"));
    }

    let stage = if previous.role.is_controller() {
        if previous.stage == "ROLE_RESOLVED" {
            previous.stage
        } else {
            "CONTROLLER_CLUSTER_READY".into()
        }
    } else {
        previous.stage
    };
    let state = PersistedState {
        schema_version: SCHEMA_VERSION,
        stage,
        role: previous.role,
        self_id: previous.self_id,
        self_ip: previous.self_ip,
        controller: previous.controller,
        tailnet: previous.tailnet,
        member: previous.member,
        cluster_join: previous.cluster_join,
    };
    validate(&state)?;
    Ok((state, true))
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
                && state.cluster_join == ClusterJoinState::NotApplicable
                && valid_controller_stage(&state.stage) => {}
        (PersistedRole::Member, Some(member))
            if member.id == state.self_id
                && member.ip == state.self_ip
                && valid_node_hostname(&member.hostname)
                && state.controller.id != state.self_id
                && state.controller.ip != state.self_ip
                && valid_member_stage(&state.stage, &state.cluster_join) => {}
        _ => {
            return Err(StateError::new(
                "state.json does not satisfy the persisted role contract",
            ));
        }
    }
    Ok(())
}

fn valid_controller_stage(stage: &str) -> bool {
    matches!(
        stage,
        "ROLE_RESOLVED" | "CONTROLLER_CLUSTER_READY" | "READY"
    )
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
        )
    }

    #[test]
    fn controller_state_round_trips_without_secrets() {
        let bytes = serde_json::to_vec(&controller_state()).expect("serialize state");
        let (state, migrated) = decode(&bytes).expect("parse state");
        assert!(!migrated);
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
    fn schema_one_state_is_normalized_to_the_current_cluster_contract() {
        let schema_one = r#"{
            "schema_version": 1,
            "stage": "READY",
            "role": "controller",
            "self_id": "node-id-123",
            "self_ip": "100.100.10.20",
            "controller": {
                "id": "node-id-123",
                "hostname": "gnx-controller-node-id-123",
                "ip": "100.100.10.20"
            },
            "tailnet": "tetra-balance.ts.net",
            "supplemental_metadata": {"enabled": true}
        }"#;
        let (state, migrated) = decode(schema_one.as_bytes()).expect("normalize schema one state");
        assert!(migrated);
        assert_eq!(state.schema_version, SCHEMA_VERSION);
        assert_eq!(state.stage, "CONTROLLER_CLUSTER_READY");
        assert_eq!(state.member, None);
        assert_eq!(state.cluster_join, ClusterJoinState::NotApplicable);
    }

    #[test]
    fn runtime_checkpoint_reset_preserves_controller_identity() {
        let mut state = controller_state();
        state.stage = "READY".into();
        normalize_runtime_checkpoint(&mut state);

        assert_eq!(state.stage, "ROLE_RESOLVED");
        assert_eq!(state.cluster_join, ClusterJoinState::NotApplicable);
        assert_eq!(state.self_id, "node-id-123");
        assert_eq!(state.controller.hostname, "gnx-controller-node-id-123");
    }

    #[test]
    fn runtime_checkpoint_reset_requires_member_rejoin() {
        let controller = ControllerIdentity {
            id: "controller-id".into(),
            hostname: "gnx-controller-controller-id".into(),
            ip: "100.100.10.20".parse().expect("IP"),
        };
        let mut state = PersistedState::member(
            "member-id".into(),
            "100.100.10.21".parse().expect("IP"),
            "gnx-member-member-id".into(),
            controller,
            "tetra-balance.ts.net".into(),
        );
        state.stage = "READY".into();
        state.cluster_join = ClusterJoinState::Joined;
        normalize_runtime_checkpoint(&mut state);

        assert_eq!(state.stage, "ROLE_RESOLVED");
        assert_eq!(state.cluster_join, ClusterJoinState::NotStarted);
        assert_eq!(state.self_id, "member-id");
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
        assert_eq!(decode(&bytes).expect("parse member state").0, state);
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
