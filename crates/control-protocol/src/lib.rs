//! Versioned, deliberately small contract between Setup and the Windows host agent.
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Operation {
    GetProgress,
    Provision,
    JoinMesh,
    Retry,
    Cancel,
    StopRuntime,
    StartRuntime,
    StartWorkload,
    StopWorkload,
    DiagnoseCompute,
    /// Start the native GitHub Actions integration (device OAuth or gh import).
    Connect,
    /// Dispatch the fixed GnX workflow with a typed start command.
    Start,
    /// Cancel the latest GnX workflow run.
    Stop,
    /// Read the sanitized GitHub integration and workflow projection.
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeState {
    New,
    WindowsReady,
    RebootRequired,
    WslReady,
    RuntimeReady,
    MeshPending,
    NodeReady,
    RuntimeStopped,
    Blocked,
    Failed,
    RecoveryRequired,
}

/// A bounded action which must be completed by the operator for external
/// tailnet clients. The local Windows host is reconciled separately by the
/// privileged agent: it receives CA trust and 127.0.0.1 HTTPS access
/// automatically before `NodeReady` is reported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionKind {
    ConfigureSplitDns,
    TrustPrivateCa,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequiredAction {
    pub kind: ActionKind,
    pub target: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitHubConnectionState {
    Disconnected,
    AwaitingAuthorization,
    Connected,
    Error,
}

/// Safe-to-display GitHub projection. It intentionally has no token, API
/// response body, headers, or arbitrary diagnostic text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubStatus {
    pub connection: GitHubConnectionState,
    #[serde(default)]
    pub auth_method: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub run_id: Option<u64>,
    #[serde(default)]
    pub run_state: Option<String>,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub verification_uri: Option<String>,
    #[serde(default)]
    pub user_code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Typed workflow target. Paths and URLs are never accepted from callers;
/// the host agent constructs requests against GitHub's fixed API authorities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GitHubWorkflowRequest {
    pub owner: String,
    pub repository: String,
    #[serde(default = "default_github_workflow")]
    pub workflow: String,
    #[serde(default = "default_github_ref")]
    pub reference: String,
}

fn default_github_workflow() -> String { "gnx-ephemeral-exit-node.yml".into() }
fn default_github_ref() -> String { "main".into() }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Progress {
    #[serde(default)]
    pub request_id: Option<Uuid>,
    pub operation_id: Uuid,
    pub sequence: u64,
    pub phase: String,
    pub state: NodeState,
    pub message: String,
    pub error_code: Option<String>,
    pub retryable: bool,
    pub requires_restart: bool,
    pub percent: Option<u8>,
    #[serde(default)]
    pub node_private_ip: Option<String>,
    #[serde(default)]
    pub pihole_private_ip: Option<String>,
    /// Runtime-compatible aliases retained for the JSONL verifier contract.
    #[serde(default)]
    pub tailscale_ip: Option<String>,
    #[serde(default)]
    pub pihole_ip: Option<String>,
    #[serde(default)]
    pub required_actions: Vec<RequiredAction>,
    /// Public certificate path only; the private key is never returned.
    #[serde(default)]
    pub private_ca_certificate: Option<String>,
    /// Sanitized native GitHub Actions status; never populated with secrets.
    #[serde(default)]
    pub github: Option<GitHubStatus>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u16,
    pub request_id: Uuid,
    pub operation: Operation,
    /// Inline values are accepted only in-process. Native Setup uses the
    /// one-shot secret_file reference so values never cross the JSON pipe.
    #[serde(default, skip_serializing)]
    pub tailscale_auth_key: Option<String>,
    #[serde(default, skip_serializing)]
    pub proxmox_password: Option<String>,
    /// A bounded local path to an ACL-protected, one-shot secret bundle.
    #[serde(default)]
    pub secret_file: Option<String>,
    /// Native GitHub action target; credentials stay in the host agent.
    #[serde(default)]
    pub github: Option<GitHubWorkflowRequest>,
    /// Fixed product workload selected by lifecycle operations. It is never a
    /// shell fragment, VMID, path, or user-defined Proxmox identifier.
    #[serde(default)]
    pub workload: Option<String>,
}

impl fmt::Debug for Request {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Request")
            .field("version", &self.version)
            .field("request_id", &self.request_id)
            .field("operation", &self.operation)
            .field(
                "tailscale_auth_key",
                &self.tailscale_auth_key.as_ref().map(|_| "[redacted]"),
            )
            .field(
                "proxmox_password",
                &self.proxmox_password.as_ref().map(|_| "[redacted]"),
            )
            .field("secret_file", &self.secret_file)
            .field("github", &self.github)
            .field("workload", &self.workload)
            .finish()
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        if let Some(value) = &mut self.tailscale_auth_key {
            unsafe { value.as_mut_vec().fill(0) };
        }
        if let Some(value) = &mut self.proxmox_password {
            unsafe { value.as_mut_vec().fill(0) };
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    pub version: u16,
    pub request_id: Uuid,
    pub operation_id: Uuid,
    pub progress: Progress,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("unsupported protocol version")]
    UnsupportedVersion,
    #[error("frame exceeds maximum size")]
    FrameTooLarge,
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("malformed JSON: {0}")]
    Json(String),
}

impl Request {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion);
        }
        let github_operation = matches!(
            self.operation,
            Operation::Connect | Operation::Start | Operation::Stop | Operation::Status
        );
        if let Some(target) = &self.github {
            validate_github_target(target)?;
        }
        if !github_operation && self.github.is_some() {
            return Err(ProtocolError::Invalid(
                "GitHub target is only accepted by GitHub actions".into(),
            ));
        }
        if self.operation != Operation::JoinMesh && self.tailscale_auth_key.is_some() {
            return Err(ProtocolError::Invalid(
                "secret is only accepted by JoinMesh".into(),
            ));
        }
        if !matches!(self.operation, Operation::Provision | Operation::JoinMesh)
            && self.proxmox_password.is_some()
        {
            return Err(ProtocolError::Invalid(
                "Proxmox credential is only accepted by Provision or JoinMesh".into(),
            ));
        }
        let workload_operation = matches!(
            self.operation,
            Operation::StartWorkload | Operation::StopWorkload
        );
        if workload_operation != self.workload.is_some() {
            return Err(ProtocolError::Invalid(
                "workload is required only by workload lifecycle operations".into(),
            ));
        }
        if let Some(workload) = &self.workload {
            if workload != "rustfs" {
                return Err(ProtocolError::Invalid("unsupported workload".into()));
            }
        }
        if self.secret_file.is_some()
            && !matches!(
                self.operation,
                Operation::Provision | Operation::JoinMesh | Operation::Connect
            )
        {
            return Err(ProtocolError::Invalid(
                "secret reference is only accepted by onboarding or GitHub connect".into(),
            ));
        }
        if self.operation == Operation::Connect
            && self.secret_file.is_none()
            && self.tailscale_auth_key.is_none()
        {
            return Err(ProtocolError::Invalid(
                "Tailscale auth key is required by GitHub connect".into(),
            ));
        }
        if let Some(path) = &self.secret_file {
            if !valid_secret_reference(path) {
                return Err(ProtocolError::Invalid("invalid secret reference".into()));
            }
        }
        if self.operation == Operation::JoinMesh
            && self.tailscale_auth_key.is_none()
            && self.secret_file.is_none()
        {
            return Err(ProtocolError::Invalid(
                "mesh credential is required for JoinMesh".into(),
            ));
        }
        if let Some(key) = &self.tailscale_auth_key {
            if !key.starts_with("tskey-auth-")
                || key.len() > 512
                || key.chars().any(|c| c.is_control() || c.is_whitespace())
            {
                return Err(ProtocolError::Invalid(
                    "invalid mesh credential format".into(),
                ));
            }
        }
        if let Some(password) = &self.proxmox_password {
            if password.is_empty()
                || password.len() > 1024
                || password.chars().any(|c| c.is_control())
            {
                return Err(ProtocolError::Invalid(
                    "invalid Proxmox credential format".into(),
                ));
            }
        }
        Ok(())
    }
    pub fn from_frame(frame: &[u8]) -> Result<Self, ProtocolError> {
        if frame.is_empty() || frame.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::FrameTooLarge);
        }
        let req: Self =
            serde_json::from_slice(frame).map_err(|e| ProtocolError::Json(e.to_string()))?;
        if req.tailscale_auth_key.is_some() || req.proxmox_password.is_some() {
            return Err(ProtocolError::Invalid(
                "inline secrets are not accepted on the control pipe".into(),
            ));
        }
        req.validate()?;
        Ok(req)
    }
    pub fn to_frame(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|e| ProtocolError::Json(e.to_string()))
    }

    pub fn with_secret_file(mut self, secret_file: impl Into<String>) -> Self {
        self.secret_file = Some(secret_file.into());
        self.tailscale_auth_key = None;
        self.proxmox_password = None;
        self
    }
}

fn validate_github_target(target: &GitHubWorkflowRequest) -> Result<(), ProtocolError> {
    for (label, value, max) in [
        ("owner", target.owner.as_str(), 100usize),
        ("repository", target.repository.as_str(), 100usize),
        ("workflow", target.workflow.as_str(), 128usize),
        ("reference", target.reference.as_str(), 256usize),
    ] {
        if value.is_empty()
            || value.len() > max
            || value.chars().any(|c| c.is_control() || c.is_whitespace())
            || (label != "reference" && value.contains('/'))
            || value.contains('\\')
            || value.contains("..")
            || value.chars().any(|c| {
                if label == "reference" { !(c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.')) }
                else { !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) }
            })
        {
            return Err(ProtocolError::Invalid(format!("invalid GitHub {label}")));
        }
    }
    if target.workflow != "gnx-ephemeral-exit-node.yml" {
        return Err(ProtocolError::Invalid("unsupported GitHub workflow".into()));
    }
    if target.reference.starts_with('-') {
        return Err(ProtocolError::Invalid("invalid GitHub reference".into()));
    }
    Ok(())
}

fn valid_secret_reference(path: &str) -> bool {
    if path.is_empty()
        || path.len() > 512
        || path.chars().any(|c| c.is_control())
        || path.starts_with("\\\\")
        || path.starts_with("//")
        || path.split(['/', '\\']).any(|part| part == "..")
    {
        return false;
    }
    let absolute = path.starts_with('/')
        || (path.len() >= 3
            && path.as_bytes()[1] == b':'
            && matches!(path.as_bytes()[2], b'/' | b'\\'));
    if !absolute {
        return false;
    }
    let Some(name) = path.rsplit(['/', '\\']).next() else {
        return false;
    };
    let Some(uuid) = name
        .strip_prefix("gnx-onboarding-")
        .and_then(|value| value.strip_suffix(".secret"))
    else {
        return false;
    };
    Uuid::parse_str(uuid).is_ok() && uuid == Uuid::parse_str(uuid).unwrap().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_secret_on_other_operation() {
        let r = Request {
            version: 1,
            request_id: Uuid::new_v4(),
            operation: Operation::Provision,
            tailscale_auth_key: Some("tskey-auth-a".into()),
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
        };
        assert!(matches!(r.validate(), Err(ProtocolError::Invalid(_))));
    }
    #[test]
    fn rejects_old_version() {
        let r = Request {
            version: 99,
            request_id: Uuid::new_v4(),
            operation: Operation::GetProgress,
            tailscale_auth_key: None,
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
        };
        assert_eq!(r.validate(), Err(ProtocolError::UnsupportedVersion));
    }
    #[test]
    fn secret_is_not_serialized() {
        let r = Request {
            version: 1,
            request_id: Uuid::new_v4(),
            operation: Operation::JoinMesh,
            tailscale_auth_key: Some("tskey-auth-a".into()),
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
        };
        assert!(!String::from_utf8(r.to_frame().unwrap())
            .unwrap()
            .contains("tskey"));
    }

    #[test]
    fn rejects_secret_with_control_characters() {
        let r = Request {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            operation: Operation::JoinMesh,
            tailscale_auth_key: Some("tskey-auth-a\n--bad".into()),
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
        };
        assert!(matches!(r.validate(), Err(ProtocolError::Invalid(_))));
    }

    #[test]
    fn rejects_join_without_credential() {
        let r = Request {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            operation: Operation::JoinMesh,
            tailscale_auth_key: None,
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
        };
        assert!(matches!(r.validate(), Err(ProtocolError::Invalid(_))));
    }

    #[test]
    fn rejects_empty_frames() {
        assert_eq!(Request::from_frame(b""), Err(ProtocolError::FrameTooLarge));
    }

    #[test]
    fn accepts_get_progress_control_frame() {
        let frame = serde_json::json!({
            "version": PROTOCOL_VERSION,
            "request_id": Uuid::new_v4(),
            "operation": "GetProgress"
        });
        let encoded = serde_json::to_vec(&frame).unwrap();
        assert!(Request::from_frame(&encoded).is_ok());
        assert!(Request::from_frame(&[encoded, b"\r".to_vec()].concat()).is_ok());
    }

    #[test]
    fn rejects_inline_secrets_from_control_frames() {
        let request = Request {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            operation: Operation::Provision,
            tailscale_auth_key: None,
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
        };
        let mut value = serde_json::to_value(request).unwrap();
        value["proxmox_password"] = serde_json::json!("should-not-cross-the-pipe");
        let frame = serde_json::to_vec(&value).unwrap();
        assert!(matches!(
            Request::from_frame(&frame),
            Err(ProtocolError::Invalid(message)) if message.contains("inline secrets")
        ));
    }

    #[test]
    fn native_secret_reference_serializes_but_values_do_not() {
        let r = Request {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            operation: Operation::Provision,
            tailscale_auth_key: None,
            proxmox_password: Some("correct horse battery staple".into()),
            secret_file: Some(
                r"C:\Users\mayas\AppData\Local\Temp\gnx-onboarding-550e8400-e29b-41d4-a716-446655440000.secret".into(),
            ),
            github: None,
            workload: None,
        };
        let frame = String::from_utf8(r.to_frame().unwrap()).unwrap();
        assert!(frame.contains("secret_file"));
        assert!(!frame.contains("correct horse"));
        let debug = format!("{r:?}");
        assert!(!debug.contains("correct horse"));
        assert!(debug.contains("redacted"));
    }

    #[test]
    fn github_actions_are_typed_and_have_no_credential_field() {
        let request = Request {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            operation: Operation::Start,
            tailscale_auth_key: None,
            proxmox_password: None,
            secret_file: None,
            github: Some(GitHubWorkflowRequest {
                owner: "gnx".into(),
                repository: "node".into(),
                workflow: "gnx-ephemeral-exit-node.yml".into(),
                reference: "main".into(),
            }),
            workload: None,
        };
        let frame = String::from_utf8(request.to_frame().unwrap()).unwrap();
        assert!(frame.contains("gnx-ephemeral-exit-node.yml"));
        assert!(!frame.to_ascii_lowercase().contains("token"));
    }

    #[test]
    fn rejects_arbitrary_github_workflows() {
        let target = GitHubWorkflowRequest {
            owner: "gnx".into(), repository: "node".into(),
            workflow: "https://evil.invalid/run".into(), reference: "main".into(),
        };
        assert!(matches!(validate_github_target(&target), Err(ProtocolError::Invalid(_))));
    }

    #[test]
    fn rejects_arbitrary_secret_paths() {
        for path in [
            r"C:\ProgramData\GnX\state\other.secret",
            r"C:\Users\mayas\AppData\Local\Temp\gnx-onboarding-550e8400-e29b-41d4-a716-446655440000.txt",
            r"C:\Users\mayas\AppData\Local\Temp\gnx-onboarding-550e8400-e29b-41d4-a716-446655440000.secret\..\other",
        ] {
            assert!(!valid_secret_reference(path));
        }
    }
}
