use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u8 = 1;
const MAX_ATTEMPTS: u8 = 3;

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
    pub phase: String,
    pub attempt: u8,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

impl InstallJournal {
    pub fn load() -> Result<Self, JournalError> {
        let path = journal_path()?;
        match fs::read(&path) {
            Ok(bytes) => {
                let journal: Self = serde_json::from_slice(&bytes)
                    .map_err(|_| error("install-state.json contains invalid data"))?;
                if journal.schema_version != SCHEMA_VERSION
                    || journal.product_version != env!("CARGO_PKG_VERSION")
                {
                    return Ok(Self::new());
                }
                Ok(journal)
            }
            Err(read_error) if read_error.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
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

    fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            product_version: env!("CARGO_PKG_VERSION").into(),
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
    let root =
        crate::staging::installer_root().map_err(|stage_error| error(stage_error.message()))?;
    let logs = root.join("logs");
    fs::create_dir_all(&logs).map_err(|create_error| {
        error(format!(
            "cannot create installer log directory: {create_error}"
        ))
    })?;
    Ok(logs.join(file_name))
}

fn journal_path() -> Result<PathBuf, JournalError> {
    crate::staging::installer_root()
        .map(|root| root.join("install-state.json"))
        .map_err(|stage_error| error(stage_error.message()))
}

fn error(message: impl Into<String>) -> JournalError {
    JournalError {
        message: message.into(),
    }
}
