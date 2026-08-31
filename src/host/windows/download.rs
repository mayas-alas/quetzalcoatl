use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::download::Artifact;
use crate::error::GnxError;

const DEPENDENCY_LOCK: &str = include_str!("../../../dependencies.lock.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub id: String,
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub publisher: String,
}

#[derive(Debug, Deserialize)]
struct LockFile {
    schema: u32,
    windows: WindowsDependencies,
}

#[derive(Debug, Deserialize)]
struct WindowsDependencies {
    podman: Dependency,
}

pub fn podman_dependency() -> Result<Dependency, GnxError> {
    let lock: LockFile = toml::from_str(DEPENDENCY_LOCK).map_err(|error| {
        GnxError::new(
            "DEPENDENCY_LOCK_INVALID",
            "download",
            "lock_parse",
            error.to_string(),
            "Corrija dependencies.lock.toml antes de publicar.",
            false,
            10,
        )
    })?;
    if lock.schema != 1 {
        return Err(GnxError::new(
            "DEPENDENCY_LOCK_INVALID",
            "download",
            "lock_parse",
            format!("Schema {} no soportado", lock.schema),
            "Use schema 1.",
            false,
            10,
        ));
    }
    Ok(lock.windows.podman)
}

pub fn download_verified(dependency: &Dependency, directory: &Path) -> Result<PathBuf, GnxError> {
    crate::download::download_verified(
        Artifact {
            id: &dependency.id,
            url: &dependency.url,
            sha256: &dependency.sha256,
            size: dependency.size,
        },
        directory,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_podman_lock_is_complete() {
        let dependency = podman_dependency().unwrap();
        assert_eq!(dependency.version, "6.0.1");
        assert_eq!(dependency.sha256.len(), 64);
        assert_eq!(dependency.publisher, "Podman");
    }
}
