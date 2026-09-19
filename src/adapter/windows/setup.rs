use crate::{
    domain::setup::BundleInput,
    port::host::{SetupHost, SetupObservation},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub struct WindowsSetupHost;

impl SetupHost for WindowsSetupHost {
    fn preflight_setup(&self) -> SetupObservation {
        preflight()
    }

    fn validate_bundle(&self, input: &BundleInput) -> Result<(), String> {
        validate_bundle(input)
    }

    fn provision_setup(&self, input: &BundleInput) -> Result<(), String> {
        provision(input)
    }
}

fn preflight() -> SetupObservation {
    let legacy_program = Path::new(r"C:\Program Files\QuetzalcoatlNext");
    let current_program = Path::new(r"C:\Program Files\GNX");
    let legacy_state = Path::new(r"C:\ProgramData\QuetzalcoatlNext");
    let service_binary = current_program.join("gnx-service.exe");
    let versioned_service = Path::new(TARGET_PROGRAM).join("gnx-service.exe");
    let old_tray_binary = current_program.join("gnx-tray.exe");

    // rama-mvp already installs under C:\Program Files\GNX. The tray binary
    // and absence of the 0.3.1 service distinguish that layout from the target.
    let legacy_present = legacy_program.exists()
        || legacy_state.exists()
        || (old_tray_binary.exists() && !service_binary.exists());

    SetupObservation {
        legacy_present,
        target_present: service_binary.exists() || versioned_service.exists(),
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
    let bytes = read_bounded(&manifest_path, 1024 * 1024)?;
    let manifest_hash = hex::encode(Sha256::digest(&bytes));
    if !valid_hash(&input.manifest_sha256)
        || manifest_hash != input.manifest_sha256.to_ascii_lowercase()
    {
        return Err("MANIFEST_HASH_MISMATCH".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| "MANIFEST_INVALID")?;
    if manifest.schema != 1 || manifest.version != "0.3.1" {
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
    super::setup_security::check_ancestors(
        &fs::canonicalize(path).map_err(|_| "BUNDLE_NOT_FOUND")?,
    )?;
    for ancestor in path.ancestors() {
        crate::adapter::setup_state::reject_link(ancestor)?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "BUNDLE_NOT_FOUND")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("BUNDLE_PATH_INVALID".into());
    }
    Ok(path.to_path_buf())
}

fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    for ancestor in path.ancestors() {
        crate::adapter::setup_state::reject_link(ancestor)?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "MANIFEST_NOT_FOUND")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > max as u64 {
        return Err("MANIFEST_PATH_INVALID".into());
    }
    let mut file = open_source(path)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "MANIFEST_READ_FAILED")?;
    if bytes.len() > max {
        return Err("MANIFEST_PATH_INVALID".into());
    }
    Ok(bytes)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    for ancestor in path.ancestors() {
        crate::adapter::setup_state::reject_link(ancestor)?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "ARTIFACT_NOT_FOUND")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("ARTIFACT_PATH_INVALID".into());
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err("ARTIFACT_TOO_LARGE".into());
    }
    let mut file = open_source(path)?;
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

const SETUP_ROOT: &str = r"C:\Program Files\GNX-Setup";
pub const TARGET_PROGRAM: &str = r"C:\Program Files\GNX-0.3.1";
const TARGET_DATA: &str = r"C:\ProgramData\GNX";

fn provision(input: &BundleInput) -> Result<(), String> {
    use super::setup_security::{protected_dir, require_elevation};
    use crate::adapter::setup_state::{SetupTransaction, Snapshot};
    require_elevation()?;
    // Reject untrusted input before creating any persistent setup state.
    validate_bundle(input)?;
    protected_dir(Path::new(SETUP_ROOT), true)?;
    let tx = SetupTransaction::acquire(Path::new(SETUP_ROOT))?;
    let observed = preflight();
    tx.snapshot(&Snapshot {
        schema: 1,
        legacy_present: observed.legacy_present,
        target_present: observed.target_present,
        manifest_sha256: input.manifest_sha256.to_ascii_lowercase(),
        rootfs_sha256: input.rootfs_sha256.to_ascii_lowercase(),
    })?;
    let mut phase = "PREFLIGHT";
    let result: Result<(), String> = (|| {
        tx.phase(phase, None)?;
        super::account::require_absent()?;
        for path in [TARGET_PROGRAM, TARGET_DATA] {
            super::setup_security::check_ancestors(Path::new(path))?;
            if Path::new(path)
                .try_exists()
                .map_err(|_| "SETUP_PATH_READ_FAILED")?
            {
                return Err("SETUP_TARGET_CONFLICT".into());
            }
        }
        phase = "STAGING";
        tx.phase(phase, None)?;
        let stage = Path::new(SETUP_ROOT).join("staged");
        protected_dir(&stage, false)?;
        for name in [
            "manifest.json",
            "gnx.exe",
            "gnx-service.exe",
            "gnx-linux-bundle.tar",
        ] {
            copy_new(&input.bundle.join(name), &stage.join(name))?;
        }
        copy_new(&input.rootfs, &stage.join("rootfs.tar"))?;
        // Reauthenticate the private copies, closing the source hash/copy race.
        validate_bundle(&BundleInput {
            bundle: stage.clone(),
            rootfs: stage.join("rootfs.tar"),
            manifest_sha256: input.manifest_sha256.clone(),
            rootfs_sha256: input.rootfs_sha256.clone(),
        })?;
        phase = "PUBLISHING";
        tx.phase(phase, None)?;
        // A sibling of legacy GNX avoids depending on its potentially weaker parent ACL.
        protected_dir(Path::new(TARGET_PROGRAM), false)?;
        protected_dir(Path::new(TARGET_DATA), false)?;
        for name in ["gnx.exe", "gnx-service.exe"] {
            copy_new(&stage.join(name), &Path::new(TARGET_PROGRAM).join(name))?;
        }
        copy_new(
            &stage.join("rootfs.tar"),
            &Path::new(TARGET_DATA).join("rootfs.tar"),
        )?;
        copy_new(
            &stage.join("gnx-linux-bundle.tar"),
            &Path::new(TARGET_DATA).join("bundle.tar"),
        )?;
        let sid = super::setup_security::operator_sid()?;
        crate::adapter::filesystem::atomic_write(
            &Path::new(TARGET_DATA).join("operator.sid"),
            sid.as_bytes(),
            0o600,
        )?;
        phase = "REGISTERING";
        tx.phase(phase, None)?;
        let runtime_sid = super::account::install()?;
        phase = "SECURING";
        tx.phase(phase, None)?;
        super::setup_security::grant_runtime(Path::new(TARGET_PROGRAM), &runtime_sid, false)?;
        super::setup_security::grant_runtime(Path::new(TARGET_DATA), &runtime_sid, true)?;
        tx.phase("PROVISIONED", None)?;
        Ok(())
    })();
    if let Err(code) = &result {
        // Preserve the last attempted phase and all partial artifacts for explicit recovery.
        if tx.phase(phase, Some(code)).is_err() {
            return Err("SETUP_FAILURE_JOURNAL_FAILED".into());
        }
    }
    result
}

fn copy_new(from: &Path, to: &Path) -> Result<(), String> {
    for ancestor in from.ancestors() {
        crate::adapter::setup_state::reject_link(ancestor)?;
    }
    if !fs::metadata(from)
        .map_err(|_| "ARTIFACT_NOT_FOUND")?
        .is_file()
    {
        return Err("ARTIFACT_PATH_INVALID".into());
    }
    let source = open_source(from)?;
    let mut source = source.take(MAX_ARTIFACT_BYTES + 1);
    let mut target = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)
        .map_err(|_| "SETUP_COPY_FAILED")?;
    let copied = std::io::copy(&mut source, &mut target).map_err(|_| "SETUP_COPY_FAILED")?;
    if copied > MAX_ARTIFACT_BYTES {
        return Err("ARTIFACT_TOO_LARGE".into());
    }
    target.sync_all().map_err(|_| "SETUP_COPY_FAILED".into())
}

const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024 * 1024;

fn open_source(path: &Path) -> Result<fs::File, String> {
    use std::os::windows::fs::OpenOptionsExt;
    // Deny simultaneous source writes/deletion while a handle is being read.
    let file = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .custom_flags(0x00200000)
        .open(path)
        .map_err(|_| "ARTIFACT_READ_FAILED")?;
    use std::os::windows::fs::MetadataExt;
    let metadata = file.metadata().map_err(|_| "ARTIFACT_READ_FAILED")?;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
        return Err("ARTIFACT_PATH_INVALID".into());
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(BundleInput);
    impl Fixture {
        fn new() -> Self {
            let bundle = std::env::temp_dir().join(format!(
                "gnx-bundle-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&bundle).unwrap();
            let mut artifacts = serde_json::Map::new();
            for name in ["gnx.exe", "gnx-service.exe", "gnx-linux-bundle.tar"] {
                fs::write(bundle.join(name), name).unwrap();
                artifacts.insert(
                    name.into(),
                    hex::encode(Sha256::digest(name.as_bytes())).into(),
                );
            }
            let manifest = serde_json::to_vec(
                &serde_json::json!({"schema":1,"version":"0.3.1","artifacts":artifacts}),
            )
            .unwrap();
            fs::write(bundle.join("manifest.json"), &manifest).unwrap();
            let rootfs = bundle.join("rootfs.tar");
            fs::write(&rootfs, b"rootfs").unwrap();
            Self(BundleInput {
                bundle,
                manifest_sha256: hex::encode(Sha256::digest(&manifest)),
                rootfs,
                rootfs_sha256: hex::encode(Sha256::digest(b"rootfs")),
            })
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0.bundle).unwrap();
        }
    }
    #[test]
    fn trusted_bundle_rejects_tampered_artifact_and_rootfs() {
        let fixture = Fixture::new();
        assert_eq!(validate_bundle(&fixture.0), Ok(()));
        fs::write(fixture.0.bundle.join("gnx.exe"), b"tampered").unwrap();
        assert_eq!(
            validate_bundle(&fixture.0).unwrap_err(),
            "ARTIFACT_HASH_MISMATCH"
        );
        fs::write(fixture.0.bundle.join("gnx.exe"), b"gnx.exe").unwrap();
        fs::write(&fixture.0.rootfs, b"tampered").unwrap();
        assert_eq!(
            validate_bundle(&fixture.0).unwrap_err(),
            "ROOTFS_HASH_MISMATCH"
        );
    }
    #[test]
    fn authenticated_manifest_requires_exact_release() {
        let mut fixture = Fixture::new();
        let path = fixture.0.bundle.join("manifest.json");
        let bytes = fs::read_to_string(&path)
            .unwrap()
            .replace("0.3.1", "0.3.10");
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            validate_bundle(&fixture.0).unwrap_err(),
            "MANIFEST_HASH_MISMATCH"
        );
        fixture.0.manifest_sha256 = hex::encode(Sha256::digest(bytes.as_bytes()));
        assert_eq!(
            validate_bundle(&fixture.0).unwrap_err(),
            "MANIFEST_SCHEMA_INVALID"
        );
    }
    #[test]
    fn copied_inputs_are_reauthenticated_and_never_overwritten() {
        let fixture = Fixture::new();
        let copy = fixture.0.bundle.join("copy.exe");
        copy_new(&fixture.0.bundle.join("gnx.exe"), &copy).unwrap();
        assert!(copy_new(&fixture.0.bundle.join("gnx.exe"), &copy).is_err());
        assert_eq!(fs::read(copy).unwrap(), b"gnx.exe");
    }
}
