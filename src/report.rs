use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Ready,
    Failed,
    ActionRequired,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub healthy: bool,
    pub code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema: u32,
    pub operation: String,
    pub state: State,
    pub code: String,
    pub revision: Option<String>,
    pub capabilities: Vec<Capability>,
    pub next_action: Option<String>,
    pub changes: Vec<String>,
    #[serde(default)]
    pub routes: Vec<Capability>,
    #[serde(default)]
    pub secret_kind: Option<crate::domain::secret::SecretKind>,
    #[serde(default)]
    pub public_root: Option<String>,
    #[serde(default)]
    pub access_ip: Option<String>,
}
impl Report {
    pub fn new(op: &str, state: State, code: &str, action: Option<&str>) -> Self {
        Self {
            schema: 1,
            operation: op.into(),
            state,
            code: code.into(),
            revision: None,
            capabilities: vec![],
            next_action: action.map(str::to_string),
            changes: vec![],
            routes: vec![],
            secret_kind: None,
            public_root: None,
            access_ip: None,
        }
    }
    pub fn exit(&self) -> i32 {
        match self.state {
            State::Ready => 0,
            State::Failed => 1,
            State::ActionRequired => 2,
        }
    }
}
