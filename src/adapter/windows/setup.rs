use crate::{
    domain::setup::BundleInput,
    port::host::{SetupHost, SetupObservation},
};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::{Path, PathBuf}};
use serde::Deserialize;

pub struct WindowsSetupHost;

impl SetupHost for WindowsSetupHost {
    fn preflight_setup(&self) -> SetupObservation {
        preflight()
    }

    fn validate_bundle(&self, input: &BundleInput) -> Result<(), String> {
        validate_bundle(input)
    }
}

fn preflight() -> SetupObservation {
    let legacy_program = Path::new(r"C:\Program Files\QuetzalcoatlNext");
    let current_program = Path::new(r"C:\Program Files\GNX");
    let legacy_state = Path::new(r"C:\ProgramData\QuetzalcoatlNext");
    let service_binary = current_program.join("gnx-service.exe");
    let old_tray_binary = current_program.join("gnx-tray.exe");

    // rama-mvp already installs under C:\Program Files\GNX. The tray binary
    // and absence of the 0.3.1 service distinguish that layout from the target.
    let legacy_present = legacy_program.exists()
        || legacy_state.exists()
        || (old_tray_binary.exists() && !service_binary.exists());

    SetupObservation {
        legacy_present,
        target_present: service_binary.exists(),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    version: String,
    artifacts: Artifacts,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifacts {
    #[serde(rename = "gnx.exe")]
    cli: String,
    #[serde(rename = "gnx-service.exe")]
    service: String,
    #[serde(rename = "gnx-setup.exe")]
    setup: String,
    #[serde(rename = "gnx-linux")]
    linux: String,
    #[serde(rename = "gnx-linux-bundle.tar")]
    linux_bundle: String,
    #[serde(rename = "gnx-linux.run")]
    linux_run: String,
}

fn validate_bundle(input: &BundleInput) -> Result<(), String> {
    let bundle = plain_dir(&input.bundle)?;
    let manifest_path = bundle.join("manifest.json");
    let manifest_hash = sha256_file(&manifest_path)?;
    if !valid_hash(&input.manifest_sha256) || manifest_hash != input.manifest_sha256.to_ascii_lowercase() {
        return Err("MANIFEST_HASH_MISMATCH".into());
    }
    let bytes = read_bounded(&manifest_path, 1024 * 1024)?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| "MANIFEST_INVALID")?;
    if manifest.schema != 1 || !manifest.version.starts_with("0.3.1") {
        return Err("MANIFEST_SCHEMA_INVALID".into());
    }
    for (name, expected) in [
        ("gnx.exe", manifest.artifacts.cli),
        ("gnx-service.exe", manifest.artifacts.service),
        ("gnx-setup.exe", manifest.artifacts.setup),
        ("gnx-linux", manifest.artifacts.linux),
        ("gnx-linux-bundle.tar", manifest.artifacts.linux_bundle),
        ("gnx-linux.run", manifest.artifacts.linux_run),
    ] {
        if !valid_hash(&expected) {
            return Err("ARTIFACT_HASH_INVALID".into());
        }
        if sha256_file(&bundle.join(name))? != expected.to_ascii_lowercase() {
            return Err("ARTIFACT_HASH_MISMATCH".into());
        }
    }
    if !valid_hash(&input.rootfs_sha256) {
        return Err("ROOTFS_HASH_INVALID".into());
    }
    if sha256_file(&input.rootfs)? != input.rootfs_sha256.to_ascii_lowercase() {
        return Err("ROOTFS_HASH_MISMATCH".into());
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn plain_dir(path: &Path) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "BUNDLE_NOT_FOUND")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("BUNDLE_PATH_INVALID".into());
    }
    Ok(path.to_path_buf())
}

fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "MANIFEST_NOT_FOUND")?;
    if metadata.file_type().is_symlink() || metadata.len() > max as u64 {
        return Err("MANIFEST_PATH_INVALID".into());
    }
    let mut file = fs::File::open(path).map_err(|_| "MANIFEST_READ_FAILED")?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes).map_err(|_| "MANIFEST_READ_FAILED")?;
    Ok(bytes)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "ARTIFACT_NOT_FOUND")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("ARTIFACT_PATH_INVALID".into());
    }
    let mut file = fs::File::open(path).map_err(|_| "ARTIFACT_READ_FAILED")?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|_| "ARTIFACT_READ_FAILED")?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(hex::encode(digest.finalize()))
}
