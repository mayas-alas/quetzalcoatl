use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Component as PathComponent, Path, PathBuf};

use serde::Deserialize;

use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;
use crate::infrastructure::models::PayloadFile;
use crate::infrastructure::payload::{payload_install_script, sha256_bytes};
use crate::infrastructure::remote::machine_stdin;

const PLATFORM_BUNDLE_CONTRACT: u8 = 1;
const PLATFORM_INSTALL_ROOT: &str = "/usr/share/quetzalcoatl/platform";
const PLATFORM_STAGING_ROOT: &str = "/usr/share/quetzalcoatl/platform.gnx-new";
const MAX_PLATFORM_FILES: usize = 128;
const MAX_PLATFORM_LOCK_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct PlatformLock {
    schema_version: u8,
    bundle_contract: u8,
    policy: PlatformPolicy,
    files: Vec<PlatformFile>,
}

#[derive(Deserialize)]
struct PlatformPolicy {
    mutable_image_tags_allowed: bool,
    embedded_secrets_allowed: bool,
    repository_commands_allowed: bool,
}

#[derive(Deserialize)]
struct PlatformFile {
    path: String,
    mode: String,
    sha256: String,
}

pub(crate) fn apply_platform_bundle(podman: &Path) -> Result<(), GateError> {
    let files = load_platform_files()?;
    machine_stdin(
        podman,
        ["sh", "-s"],
        format!(
            "set -eu\nroot='{PLATFORM_STAGING_ROOT}'\ntest \"$root\" = '/usr/share/quetzalcoatl/platform.gnx-new'\nrm -rf -- \"$root\"\ninstall -d -m 0755 \"$root\"\n"
        )
        .as_bytes(),
    )
    .map_err(|error| error.with_code("PLATFORM_BUNDLE_INVALID", Component::Proxmox))?;

    for file in files {
        let script = payload_install_script(&file)
            .map_err(|error| error.with_code("PLATFORM_BUNDLE_INVALID", Component::Proxmox))?;
        machine_stdin(podman, ["sh", "-s"], &script)
            .map_err(|error| error.with_code("PLATFORM_BUNDLE_INVALID", Component::Proxmox))?;
    }

    let activation = format!(
        "set -eu\nroot='{PLATFORM_INSTALL_ROOT}'\nstaging='{PLATFORM_STAGING_ROOT}'\nold='/usr/share/quetzalcoatl/platform.gnx-old'\ntest -d \"$staging\"\ntest \"$root\" = '/usr/share/quetzalcoatl/platform'\ntest \"$old\" = '/usr/share/quetzalcoatl/platform.gnx-old'\nrm -rf -- \"$old\"\nif [ -d \"$root\" ]; then mv \"$root\" \"$old\"; fi\nif ! mv \"$staging\" \"$root\"; then\n  if [ -d \"$old\" ]; then mv \"$old\" \"$root\"; fi\n  exit 1\nfi\nrm -rf -- \"$old\"\n"
    );
    machine_stdin(podman, ["sh", "-s"], activation.as_bytes())
        .map_err(|error| error.with_code("PLATFORM_BUNDLE_INVALID", Component::Proxmox))?;
    Ok(())
}

pub(crate) fn validate_platform_bundle() -> Result<(), GateError> {
    load_platform_files().map(|_| ())
}

fn load_platform_files() -> Result<Vec<PayloadFile>, GateError> {
    let root = platform_root()?;
    let lock_path = root.join("platform.lock.json");
    let metadata = fs::metadata(&lock_path).map_err(bundle_io("cannot inspect platform lock"))?;
    if metadata.len() == 0 || metadata.len() > MAX_PLATFORM_LOCK_BYTES {
        return Err(bundle_error("platform lock size is invalid"));
    }
    let lock_bytes = fs::read(&lock_path).map_err(bundle_io("cannot read platform lock"))?;
    let lock: PlatformLock = serde_json::from_slice(&lock_bytes)
        .map_err(|_| bundle_error("platform lock is invalid JSON"))?;
    if lock.schema_version != 1
        || lock.bundle_contract != PLATFORM_BUNDLE_CONTRACT
        || lock.policy.mutable_image_tags_allowed
        || lock.policy.embedded_secrets_allowed
        || lock.policy.repository_commands_allowed
        || lock.files.is_empty()
        || lock.files.len() > MAX_PLATFORM_FILES
    {
        return Err(bundle_error("platform lock contract or policy differs"));
    }

    let mut seen = HashSet::with_capacity(lock.files.len());
    let mut files = Vec::with_capacity(lock.files.len());
    for entry in lock.files {
        let relative = normalized_relative_path(&entry.path)?;
        if !seen.insert(entry.path.clone())
            || !matches!(entry.mode.as_str(), "0644" | "0755")
            || !valid_sha256(&entry.sha256)
        {
            return Err(bundle_error("platform lock contains invalid file metadata"));
        }
        let source = root.join(&relative);
        let contents = fs::read(&source).map_err(bundle_io("cannot read locked platform file"))?;
        if sha256_bytes(&contents) != entry.sha256 {
            return Err(bundle_error(
                "locked platform file does not match its SHA-256",
            ));
        }
        files.push(PayloadFile {
            destination: format!("{PLATFORM_STAGING_ROOT}/{}", entry.path),
            mode: entry.mode,
            sha256: entry.sha256,
            contents,
        });
    }

    let actual = enumerate_files(&root)?;
    if actual != seen {
        return Err(bundle_error(
            "platform source and platform lock inventories differ",
        ));
    }
    Ok(files)
}

fn platform_root() -> Result<PathBuf, GateError> {
    let executable = env::current_exe().map_err(bundle_io("cannot locate service executable"))?;
    executable
        .parent()
        .map(|parent| parent.join("platform"))
        .ok_or_else(|| bundle_error("service executable has no parent directory"))
}

fn normalized_relative_path(value: &str) -> Result<PathBuf, GateError> {
    if value.is_empty()
        || value.len() > 240
        || value.contains('\\')
        || value.starts_with('/')
        || value.ends_with('/')
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'/' | b'-' | b'_' | b'.')
        })
    {
        return Err(bundle_error("platform lock path is not normalized"));
    }
    let path = PathBuf::from(value);
    if path.components().any(|component| {
        !matches!(component, PathComponent::Normal(_))
            || component.as_os_str().to_string_lossy() == ".."
    }) {
        return Err(bundle_error("platform lock path escapes its root"));
    }
    Ok(path)
}

fn enumerate_files(root: &Path) -> Result<HashSet<String>, GateError> {
    fn visit(root: &Path, directory: &Path, files: &mut HashSet<String>) -> Result<(), GateError> {
        for entry in
            fs::read_dir(directory).map_err(bundle_io("cannot enumerate platform bundle"))?
        {
            let entry = entry.map_err(bundle_io("cannot inspect platform bundle entry"))?;
            let file_type = entry
                .file_type()
                .map_err(bundle_io("cannot inspect platform bundle type"))?;
            if file_type.is_symlink() {
                return Err(bundle_error("platform bundle contains a symbolic link"));
            }
            if file_type.is_dir() {
                visit(root, &entry.path(), files)?;
            } else if file_type.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| bundle_error("platform file escapes its root"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                if relative != "platform.lock.json" {
                    normalized_relative_path(&relative)?;
                    files.insert(relative);
                }
            } else {
                return Err(bundle_error("platform bundle contains a special file"));
            }
        }
        Ok(())
    }

    let mut files = HashSet::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn bundle_error(message: impl Into<String>) -> GateError {
    GateError::new(
        "PLATFORM_BUNDLE_INVALID",
        Component::Proxmox,
        message.into(),
    )
}

fn bundle_io(operation: &'static str) -> impl FnOnce(std::io::Error) -> GateError {
    move |error| bundle_error(format!("{operation}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_paths_are_relative_and_semantic() {
        assert!(normalized_relative_path("tofu/foundation/main.tf").is_ok());
        for invalid in [
            "",
            "/absolute",
            "../escape",
            "tofu/../escape",
            "Uppercase",
            "path\\file",
            "path/",
        ] {
            assert!(normalized_relative_path(invalid).is_err(), "{invalid}");
        }
    }
}
