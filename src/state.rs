use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::GnxError;

pub const STATE_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Pending,
    Installing,
    RebootRequired,
    Installed,
    Working,
    Ready,
    Failed,
    Uninstalled,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Installing => "installing",
            Self::RebootRequired => "reboot_required",
            Self::Installed => "installed",
            Self::Working => "working",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Uninstalled => "uninstalled",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalState {
    pub schema: u32,
    pub product_version: String,
    pub stage: Stage,
    pub machine: String,
    pub mesh: String,
    pub docktail: String,
    pub proxmox: String,
    pub infra: String,
    pub last_error: Option<String>,
}

impl Default for OperationalState {
    fn default() -> Self {
        Self {
            schema: STATE_SCHEMA,
            product_version: env!("CARGO_PKG_VERSION").to_string(),
            stage: Stage::Pending,
            machine: "pending".to_string(),
            mesh: "pending".to_string(),
            docktail: "pending".to_string(),
            proxmox: "pending".to_string(),
            infra: "pending".to_string(),
            last_error: None,
        }
    }
}

impl OperationalState {
    pub fn load(path: &Path) -> Result<Option<Self>, GnxError> {
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content)
                .map(Some)
                .map_err(|error| GnxError::io("state_load", format!("State inválido: {error}"))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(GnxError::io("state_load", error.to_string())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), GnxError> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| GnxError::io("state_serialize", error.to_string()))?;
        atomic_write(path, &bytes)
    }
}

pub fn default_state_path() -> PathBuf {
    crate::config::data_root().join("state.json")
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), GnxError> {
    let parent = path.parent().ok_or_else(|| {
        GnxError::io(
            "atomic_write",
            format!("Ruta sin directorio: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| GnxError::io("atomic_write", error.to_string()))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));

    let mut file = File::create(&temporary)
        .map_err(|error| GnxError::io("atomic_write", error.to_string()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| GnxError::io("atomic_write", error.to_string()))?;
    replace_file(&temporary, path)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), GnxError> {
    fs::rename(temporary, destination)
        .map_err(|error| GnxError::io("atomic_write", error.to_string()))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), GnxError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source: Vec<u16> = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let target: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: both paths are NUL-terminated UTF-16 buffers that live through the call.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(GnxError::io(
            "atomic_write",
            std::io::Error::last_os_error().to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trip() {
        let path = std::env::temp_dir().join(format!("gnx-state-{}.json", std::process::id()));
        let state = OperationalState::default();
        state.save(&path).unwrap();
        let loaded = OperationalState::load(&path).unwrap().unwrap();
        assert_eq!(loaded.schema, STATE_SCHEMA);
        let _ = fs::remove_file(path);
    }
}
