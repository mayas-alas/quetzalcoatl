//! Protected setup transaction metadata. Callers establish the directory ACL first.
use crate::adapter::filesystem::atomic_write;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u32,
    pub legacy_present: bool,
    pub target_present: bool,
    pub manifest_sha256: String,
    pub rootfs_sha256: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupState {
    pub schema: u32,
    pub phase: String,
    pub outcome: String,
    pub code: Option<String>,
}

pub struct SetupTransaction {
    root: PathBuf,
    _lock: File,
}

impl SetupTransaction {
    pub fn acquire(root: &Path) -> Result<Self, String> {
        Self::acquire_inner(root, false)
    }

    /// Open an interrupted transaction for explicit recovery/rollback. Normal
    /// apply calls must continue to refuse blind retries.
    pub fn acquire_recovery(root: &Path) -> Result<Self, String> {
        Self::acquire_inner(root, true)
    }

    fn acquire_inner(root: &Path, recovery: bool) -> Result<Self, String> {
        reject_link(root)?;
        let path = root.join("setup.lock");
        reject_link(&path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let lock = options.open(path).map_err(|_| "SETUP_LOCK_FAILED")?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| "SETUP_BUSY")?;
        let result = Self {
            root: root.into(),
            _lock: lock,
        };
        for name in ["journal.json", "snapshot.json"] {
            reject_link(&root.join(name))?;
            if !recovery
                && root
                    .join(name)
                    .try_exists()
                    .map_err(|_| "SETUP_STATE_READ_FAILED")?
            {
                return Err("SETUP_RECOVERY_REQUIRED".into());
            }
        }
        Ok(result)
    }

    pub fn snapshot(&self, snapshot: &Snapshot) -> Result<(), String> {
        self.write("snapshot.json", snapshot)
    }

    /// Written before each mutation, so an interrupted phase is never mistaken for success.
    pub fn phase(&self, phase: &str, code: Option<&str>) -> Result<(), String> {
        self.write(
            "journal.json",
            &serde_json::json!({"schema":1,"operation":"PROVISION","phase":phase,"code":code}),
        )
    }

    pub fn state(&self, state: &SetupState) -> Result<(), String> {
        self.write("setup-state.json", state)
    }

    fn write(&self, name: &str, value: &impl Serialize) -> Result<(), String> {
        let path = self.root.join(name);
        reject_link(&path)?;
        reject_link(&path.with_extension("gnx-new"))?;
        atomic_write(
            &path,
            &serde_json::to_vec(value).map_err(|_| "SETUP_STATE_ENCODE_FAILED")?,
            0o600,
        )
    }
}

pub fn reject_link(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err("SETUP_REPARSE_REFUSED".into());
                }
            }
            if metadata.file_type().is_symlink() {
                return Err("SETUP_REPARSE_REFUSED".into());
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("SETUP_PATH_READ_FAILED".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "gnx-setup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn lock_excludes_concurrency_and_releases_on_drop() {
        let p = root();
        let first = SetupTransaction::acquire(&p).unwrap();
        assert!(matches!(SetupTransaction::acquire(&p), Err(e) if e == "SETUP_BUSY"));
        drop(first);
        drop(SetupTransaction::acquire(&p).unwrap());
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn snapshot_or_journal_blocks_blind_retry() {
        for name in ["snapshot.json", "journal.json"] {
            let p = root();
            std::fs::write(p.join(name), "interrupted").unwrap();
            assert!(
                matches!(SetupTransaction::acquire(&p), Err(e) if e == "SETUP_RECOVERY_REQUIRED")
            );
            std::fs::remove_dir_all(p).unwrap();
        }
    }
    #[test]
    fn journal_records_failure_without_input_paths() {
        let p = root();
        let tx = SetupTransaction::acquire(&p).unwrap();
        tx.phase("STAGING", None).unwrap();
        tx.phase("FAILED", Some("ARTIFACT_HASH_MISMATCH")).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(p.join("journal.json")).unwrap()).unwrap();
        assert_eq!(value["phase"], "FAILED");
        assert_eq!(value["code"], "ARTIFACT_HASH_MISMATCH");
        drop(tx);
        std::fs::remove_dir_all(p).unwrap();
    }
}
