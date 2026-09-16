use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub enum Operation { Check, Status }
impl Operation {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self { Self::Check => b"check", Self::Status => b"status" }
    }

    pub fn from_bytes(value: &[u8]) -> Option<Self> {
        match value { b"check" => Some(Self::Check), b"status" => Some(Self::Status), _ => None }
    }
}

pub fn parse_operation(args: &[String]) -> Result<Operation, String> {
    let operation = match args { [one] => Operation::from_bytes(one.as_bytes()), _ => None };
    operation.ok_or_else(|| "Only check and status are supported; no additional arguments are accepted.".into())
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
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
        Self { protocol: 1, state: state.into(), observed_unix: unix_now(), detail: detail.into(), verified }
    }
    pub fn validate_at(&mut self, now: u64) -> Result<(), String> {
        if self.protocol != 1 || !matches!(self.state.as_str(), "ready" | "degraded" | "stopped" | "unknown" | "downloading" | "configuring" | "verifying") || self.verified != (self.state == "ready") {
            return Err("protocol_mismatch: unsupported or inconsistent health state".into());
        }
        if self.observed_unix > now.saturating_add(5) { return Err("protocol_mismatch: observation is in the future".into()); }
        if now.saturating_sub(self.observed_unix) > 45 {
            self.state = "unknown".into(); self.verified = false;
            self.detail = "Observation expired; current health is unknown.".into();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn accepts_only_queries() {
        assert_eq!(parse_operation(&["check".into()]).unwrap(), Operation::Check);
        assert_eq!(parse_operation(&["status".into()]).unwrap(), Operation::Status);
        for args in [vec![], vec!["install".into()], vec!["status".into(), "--exec".into()], vec!["status;cmd".into()]] { assert!(parse_operation(&args).is_err()); }
    }
    #[test] fn wire_operations_roundtrip_and_reject_trailing_data() {
        for op in [Operation::Check, Operation::Status] {
            assert_eq!(Operation::from_bytes(op.as_bytes()), Some(op));
        }
        for data in [b"check\0".as_slice(), b"status\n", b"STATUS", b"install", &[255]] {
            assert!(Operation::from_bytes(data).is_none());
        }
    }
    #[test] fn rejects_extra_response_fields() {
        let text = r#"{"protocol":1,"state":"ready","observed_unix":0,"detail":"x","verified":true,"exec":"cmd"}"#;
        assert!(serde_json::from_str::<Report>(text).is_err());
    }
    #[test] fn progress_is_valid_but_not_ready() {
        for state in ["downloading", "configuring", "verifying"] {
            let mut report = Report::new(state, "initializing", false);
            assert!(report.validate_at(report.observed_unix).is_ok());
            report.verified = true;
            assert!(report.validate_at(report.observed_unix).is_err());
        }
    }
    #[test] fn stale_health_is_never_ready() {
        let mut report = Report::new("ready", "healthy", true);
        report.observed_unix = 100; report.validate_at(146).unwrap();
        assert_eq!(report.state, "unknown"); assert!(!report.verified);
    }
    #[test] fn reject_inconsistent_and_future_health() {
        let mut report = Report::new("degraded", "failure", true);
        assert!(report.validate_at(report.observed_unix).is_err());
        report = Report::new("ready", "healthy", true);
        assert!(report.validate_at(report.observed_unix - 6).is_err());
        report.protocol = 2; assert!(report.validate_at(report.observed_unix).is_err());
    }
}
