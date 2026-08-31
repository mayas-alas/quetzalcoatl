use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::GnxError;
use crate::state::atomic_write;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum InstallCheckpoint {
    Started,
    Elevated,
    FilesInstalled,
    WslEnabled,
    PodmanInstalled,
    ServiceRegistered,
    MachineRequested,
    Completed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationJournal {
    pub schema: u32,
    pub operation_id: String,
    pub operation: String,
    pub checkpoint: InstallCheckpoint,
    pub target_version: String,
    pub reboot_required: bool,
    pub last_error: Option<String>,
}

impl OperationJournal {
    pub fn new_install() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            schema: 1,
            operation_id: format!("{:x}-{:x}", std::process::id(), nanos),
            operation: "install".to_string(),
            checkpoint: InstallCheckpoint::Started,
            target_version: env!("CARGO_PKG_VERSION").to_string(),
            reboot_required: false,
            last_error: None,
        }
    }

    pub fn load(path: &Path) -> Result<Option<Self>, GnxError> {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).map(Some).map_err(|error| {
                GnxError::io("journal_load", format!("Journal inválido: {error}"))
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(GnxError::io("journal_load", error.to_string())),
        }
    }

    pub fn advance(&mut self, checkpoint: InstallCheckpoint) -> Result<(), GnxError> {
        if checkpoint < self.checkpoint {
            return Err(GnxError::new(
                "INSTALL_JOURNAL_REGRESSION",
                "install",
                "journal_advance",
                format!("{:?} no puede regresar a {:?}", self.checkpoint, checkpoint),
                "Conserve el journal y ejecute gnx doctor.",
                false,
                8,
            ));
        }
        self.checkpoint = checkpoint;
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), GnxError> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| GnxError::io("journal_serialize", error.to_string()))?;
        atomic_write(path, &bytes)
    }
}

pub fn default_journal_path() -> PathBuf {
    crate::config::data_root().join("journal.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_is_monotonic() {
        let mut journal = OperationJournal::new_install();
        journal.advance(InstallCheckpoint::WslEnabled).unwrap();
        let error = journal
            .advance(InstallCheckpoint::FilesInstalled)
            .unwrap_err();
        assert_eq!(error.code, "INSTALL_JOURNAL_REGRESSION");
    }
}
