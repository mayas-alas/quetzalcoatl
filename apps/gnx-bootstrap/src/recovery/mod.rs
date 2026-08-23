use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u8 = 2;
const MAX_ATTEMPTS: u8 = 3;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum InstallOperation {
    Install,
    Upgrade,
    Repair,
}

#[derive(Debug)]
pub struct JournalError {
    message: String,
}

impl JournalError {
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstallJournal {
    schema_version: u8,
    product_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_product_version: Option<String>,
    operation: InstallOperation,
    pub phase: String,
    pub attempt: u8,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

impl InstallJournal {
    pub fn load(requested_operation: crate::RequestedOperation) -> Result<Self, JournalError> {
        let path = journal_path()?;
        match fs::read(&path) {
            Ok(bytes) => Self::decode(&bytes, requested_operation),
            Err(read_error) if read_error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Self::new(requested_operation))
            }
            Err(read_error) => Err(error(format!(
                "cannot read install-state.json: {read_error}"
            ))),
        }
    }

    pub fn begin(&mut self, phase: &str) -> Result<(), JournalError> {
        self.attempt = if self.phase == phase {
            self.attempt.saturating_add(1)
        } else {
            1
        };
        self.phase = phase.to_owned();
        self.last_error_code = None;
        self.last_error_message = None;
        if self.attempt > MAX_ATTEMPTS {
            self.last_error_code = Some("INSTALL_RESUME_LIMIT_REACHED".into());
            self.last_error_message =
                Some(format!("phase {phase} exceeded {MAX_ATTEMPTS} attempts"));
            self.store()?;
            return Err(error(format!(
                "phase {phase} exceeded {MAX_ATTEMPTS} attempts"
            )));
        }
        self.store()
    }

    pub fn complete(&mut self, phase: &str) -> Result<(), JournalError> {
        self.phase = phase.to_owned();
        self.attempt = 0;
        self.last_error_code = None;
        self.last_error_message = None;
        self.store()
    }

    pub fn record_error(&mut self, code: &str, message: &str) -> Result<(), JournalError> {
        self.last_error_code = Some(code.to_owned());
        self.last_error_message = Some(message.to_owned());
        self.store()
    }

    fn decode(
        bytes: &[u8],
        requested_operation: crate::RequestedOperation,
    ) -> Result<Self, JournalError> {
        let value: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|_| error("install-state.json contains invalid data"))?;
        let stored_schema = value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| error("install-state.json omits schema_version"))?;
        let stored_version = value
            .get("product_version")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| error("install-state.json omits product_version"))?;

        if stored_schema > u64::from(SCHEMA_VERSION) {
            return Err(error("install-state.json uses a newer schema"));
        }
        if stored_schema == u64::from(SCHEMA_VERSION) && stored_version == env!("CARGO_PKG_VERSION")
        {
            let mut journal: Self = serde_json::from_value(value)
                .map_err(|_| error("install-state.json contains invalid data"))?;
            if requested_operation == crate::RequestedOperation::Repair {
                journal.operation = InstallOperation::Repair;
            }
            return Ok(journal);
        }

        let operation = if requested_operation == crate::RequestedOperation::Repair {
            InstallOperation::Repair
        } else {
            InstallOperation::Upgrade
        };
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            product_version: env!("CARGO_PKG_VERSION").into(),
            previous_product_version: Some(stored_version.to_owned()),
            operation,
            phase: "STARTED".into(),
            attempt: 0,
            last_error_code: None,
            last_error_message: None,
        })
    }

    fn new(requested_operation: crate::RequestedOperation) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            product_version: env!("CARGO_PKG_VERSION").into(),
            previous_product_version: None,
            operation: match requested_operation {
                crate::RequestedOperation::Install => InstallOperation::Install,
                crate::RequestedOperation::Repair => InstallOperation::Repair,
            },
            phase: "STARTED".into(),
            attempt: 0,
            last_error_code: None,
            last_error_message: None,
        }
    }

    fn store(&self) -> Result<(), JournalError> {
        let path = journal_path()?;
        let parent = path
            .parent()
            .ok_or_else(|| error("install-state.json has no parent directory"))?;
        fs::create_dir_all(parent).map_err(|store_error| {
            error(format!(
                "cannot create installer state directory: {store_error}"
            ))
        })?;
        let temporary = path.with_extension("json.next");
        let _ = fs::remove_file(&temporary);
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|_| error("cannot encode install-state.json"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|store_error| {
                error(format!(
                    "cannot create install-state.json.next: {store_error}"
                ))
            })?;
        file.write_all(&bytes)
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|store_error| {
                error(format!(
                    "cannot persist install-state.json.next: {store_error}"
                ))
            })?;
        if path.exists() {
            fs::remove_file(&path).map_err(|store_error| {
                error(format!("cannot replace install-state.json: {store_error}"))
            })?;
        }
        fs::rename(&temporary, &path).map_err(|store_error| {
            error(format!("cannot activate install-state.json: {store_error}"))
        })?;
        Ok(())
    }
}

pub fn log_path(file_name: &str) -> Result<PathBuf, JournalError> {
    let root = crate::dependencies::staging::installer_root()
        .map_err(|stage_error| error(stage_error.message()))?;
    let logs = root.join("logs");
    fs::create_dir_all(&logs).map_err(|create_error| {
        error(format!(
            "cannot create installer log directory: {create_error}"
        ))
    })?;
    Ok(logs.join(file_name))
}

fn journal_path() -> Result<PathBuf, JournalError> {
    crate::dependencies::staging::installer_root()
        .map(|root| root.join("install-state.json"))
        .map_err(|stage_error| error(stage_error.message()))
}

fn error(message: impl Into<String>) -> JournalError {
    JournalError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_a_previous_release_journal_into_upgrade_operation() {
        let previous = br#"{
            "schema_version": 1,
            "product_version": "0.1.17",
            "phase": "WSL_INSTALLED",
            "attempt": 0,
            "last_error_code": null,
            "last_error_message": null
        }"#;
        let journal = InstallJournal::decode(previous, crate::RequestedOperation::Install).unwrap();
        assert_eq!(journal.schema_version, 2);
        assert_eq!(journal.product_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(journal.previous_product_version.as_deref(), Some("0.1.17"));
        assert_eq!(journal.operation, InstallOperation::Upgrade);
        assert_eq!(journal.phase, "STARTED");
    }

    #[test]
    fn migrates_the_incomplete_0_2_0_journal_into_upgrade_operation() {
        let previous = br#"{
            "schema_version": 2,
            "product_version": "0.2.0",
            "operation": "repair",
            "phase": "PODMAN_INSTALLED",
            "attempt": 1,
            "last_error_code": "RUNTIME_PAYLOAD_INVALID",
            "last_error_message": "runtime manifest is absent"
        }"#;
        let journal = InstallJournal::decode(previous, crate::RequestedOperation::Install).unwrap();
        assert_eq!(journal.product_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(journal.previous_product_version.as_deref(), Some("0.2.0"));
        assert_eq!(journal.operation, InstallOperation::Upgrade);
        assert_eq!(journal.phase, "STARTED");
        assert_eq!(journal.attempt, 0);
        assert_eq!(journal.last_error_code, None);
    }

    #[test]
    fn migrates_the_0_2_1_journal_into_upgrade_operation() {
        let previous = br#"{
            "schema_version": 2,
            "product_version": "0.2.1",
            "operation": "repair",
            "phase": "PODMAN_INSTALLED",
            "attempt": 1,
            "last_error_code": null,
            "last_error_message": null
        }"#;
        let journal = InstallJournal::decode(previous, crate::RequestedOperation::Install).unwrap();
        assert_eq!(journal.product_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(journal.previous_product_version.as_deref(), Some("0.2.1"));
        assert_eq!(journal.operation, InstallOperation::Upgrade);
        assert_eq!(journal.phase, "STARTED");
        assert_eq!(journal.attempt, 0);
    }

    #[test]
    fn migrates_the_0_2_4_journal_into_upgrade_operation() {
        let previous = br#"{
            "schema_version": 2,
            "product_version": "0.2.4",
            "operation": "repair",
            "phase": "PODMAN_INSTALLED",
            "attempt": 0,
            "last_error_code": null,
            "last_error_message": null
        }"#;
        let journal = InstallJournal::decode(previous, crate::RequestedOperation::Install).unwrap();
        assert_eq!(journal.product_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(journal.previous_product_version.as_deref(), Some("0.2.4"));
        assert_eq!(journal.operation, InstallOperation::Upgrade);
        assert_eq!(journal.phase, "STARTED");
        assert_eq!(journal.attempt, 0);
    }

    #[test]
    fn a_repair_request_is_explicit_and_keeps_the_current_checkpoint() {
        let current = format!(
            r#"{{
            "schema_version": 2,
            "product_version": "{}",
            "operation": "install",
            "phase": "PODMAN_INSTALLED",
            "attempt": 0,
            "last_error_code": null,
            "last_error_message": null
        }}"#,
            env!("CARGO_PKG_VERSION")
        );
        let journal =
            InstallJournal::decode(current.as_bytes(), crate::RequestedOperation::Repair).unwrap();
        assert_eq!(journal.operation, InstallOperation::Repair);
        assert_eq!(journal.phase, "PODMAN_INSTALLED");
        assert_eq!(journal.attempt, 0);
    }
}
