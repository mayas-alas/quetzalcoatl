use serde::{Deserialize, Serialize};

pub const SERVICE: &str = "IsolatedRuntime";
pub const ACCOUNT: &str = "svc_runtime";
pub const DISTRO: &str = "isolated-runtime";
pub const PIPE: &str = r"\\.\pipe\isolated-runtime-control-v1";
pub const MAX_MESSAGE: usize = 8192;

#[derive(Debug, PartialEq)]
pub enum Operation { Check, Status }
pub fn parse_operation(args: &[String]) -> Result<Operation, String> {
    match args {
        [one] if one == "check" => Ok(Operation::Check),
        [one] if one == "status" => Ok(Operation::Status),
        _ => Err("Only check and status are supported; no additional arguments are accepted.".into()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub protocol: u32,
    pub state: String,
    pub observed_unix: u64,
    pub detail: String,
    pub verified: bool,
}
impl Report {
    pub fn new(state: &str, detail: &str, verified: bool) -> Self {
        Self { protocol: 1, state: state.into(), observed_unix: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(), detail: detail.into(), verified }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config { pub client_sid: String, pub account_sid: String, pub rootfs_sha256: String }

pub fn valid_sid(s: &str) -> bool {
    s.starts_with("S-1-5-21-") && s.len() < 190 && s.split('-').skip(1).all(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit())) && s.split('-').count() == 8
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn accepts_only_queries() {
        assert_eq!(parse_operation(&["check".into()]).unwrap(), Operation::Check);
        assert_eq!(parse_operation(&["status".into()]).unwrap(), Operation::Status);
        for args in [vec![], vec!["install".into()], vec!["status".into(), "--exec".into()], vec!["status;cmd".into()]] { assert!(parse_operation(&args).is_err()); }
    }
    #[test] fn sid_cannot_inject_acl() {
        assert!(valid_sid("S-1-5-21-1-2-3-1001"));
        for sid in ["S-1-5-18", "S-1-5-21-1-2-3-1001)(A;;GA;;;WD)", "S-1-5-21-1-2-3-", "S-1-5-21-1-2-3-1001-extra"] { assert!(!valid_sid(sid)); }
    }
    #[test] fn rejects_extra_response_fields() {
        let text = r#"{"protocol":1,"state":"ready","observed_unix":0,"detail":"x","verified":true,"exec":"cmd"}"#;
        assert!(serde_json::from_str::<Report>(text).is_err());
    }
}
