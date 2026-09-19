use crate::{
    domain::setup::BundleInput,
    port::host::{SetupHost, SetupObservation, SetupVerification},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
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

    fn verify_setup(&self) -> Result<SetupVerification, String> {
        verify_installed()
    }

    fn recover_setup(&self, rollback: bool) -> Result<(), String> {
        recover(rollback)
    }
}

fn preflight() -> SetupObservation {
    let legacy_program = Path::new(r"C:\Program Files\QuetzalcoatlNext");
    let legacy_state = Path::new(r"C:\ProgramData\QuetzalcoatlNext");
    let legacy_gnx_program = Path::new(r"C:\Program Files\GNX");
    let legacy_gnx_state = Path::new(r"C:\ProgramData\GNX");
    // Legacy roots are never adopted, even when they contain files that look
    // like a current service. Presence alone is an honest preflight conflict.
    let legacy_present = [
        legacy_program,
        legacy_state,
        legacy_gnx_program,
        legacy_gnx_state,
    ]
    .iter()
    .any(|path| path.exists());

    SetupObservation {
        legacy_present,
        target_present: Path::new(TARGET_PROGRAM).exists() || Path::new(TARGET_DATA).exists(),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema: u32,
    operation: String,
    phase: String,
    #[serde(rename = "code")]
    _code: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    version: String,
    platform: String,
    status: String,
    rootfs_sha256: String,
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
    if manifest.schema != 1
        || manifest.version != "0.3.1"
        || manifest.platform != "windows+linux-amd64"
    {
        return Err("MANIFEST_SCHEMA_INVALID".into());
    }
    if manifest.status != "sealed" {
        return Err("RELEASE_UNAUTHENTICATED".into());
    }
    verify_manifest_signature(&bytes, &bundle.join("manifest.sig"))?;
    if !valid_hash(&manifest.rootfs_sha256)
        || !manifest
            .rootfs_sha256
            .eq_ignore_ascii_case(&input.rootfs_sha256)
    {
        return Err("ROOTFS_NOT_COVERED_BY_MANIFEST".into());
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

fn verify_manifest_signature(manifest: &[u8], signature_path: &Path) -> Result<(), String> {
    let signature = read_bounded(signature_path, 128)?;
    let signature = Signature::from_slice(&signature).map_err(|_| "RELEASE_SIGNATURE_INVALID")?;
    trusted_release_key()?
        .verify(manifest, &signature)
        .map_err(|_| "RELEASE_SIGNATURE_INVALID".into())
}

fn trusted_release_key() -> Result<VerifyingKey, String> {
    #[cfg(test)]
    {
        use ed25519_dalek::SigningKey;
        Ok(SigningKey::from_bytes(&[
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
            0x2c, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c,
            0xae, 0x7f, 0x60, 0x00,
        ])
        .verifying_key())
    }
    #[cfg(not(test))]
    {
        let key_bytes = hex::decode(include_str!("../../../config/release-public-key.hex").trim())
            .map_err(|_| "RELEASE_TRUST_ROOT_INVALID")?;
        let key_bytes: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| "RELEASE_TRUST_ROOT_INVALID")?;
        VerifyingKey::from_bytes(&key_bytes).map_err(|_| "RELEASE_TRUST_ROOT_INVALID".into())
    }
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
    // Keep the large streaming buffer on the heap; Windows setup has a small
    // default thread stack and a stack allocation here overflows on rootfs validation.
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|_| "ARTIFACT_READ_FAILED")?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(hex::encode(digest.finalize()))
}

// State is deliberately kept beside the protected runtime data so lock,
// snapshot, journal, and setup-state share one ACL boundary.
pub const SETUP_ROOT: &str = r"C:\ProgramData\GNX-Setup-0.3.1";
pub const TARGET_PROGRAM: &str = r"C:\Program Files\GNX-0.3.1";
pub const TARGET_DATA: &str = r"C:\ProgramData\GNX-0.3.1";

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
    tx.state(&crate::adapter::setup_state::SetupState {
        schema: 1,
        phase: "PRECHECK".into(),
        outcome: "IN_PROGRESS".into(),
        code: None,
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
            "manifest.sig",
            "gnx.exe",
            "gnx-service.exe",
            "gnx-setup.exe",
            "gnx-linux",
            "gnx-linux-bundle.tar",
            "gnx-linux.run",
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
        for name in ["gnx.exe", "gnx-service.exe", "gnx-setup.exe"] {
            copy_new(&stage.join(name), &Path::new(TARGET_PROGRAM).join(name))?;
        }
        for name in ["gnx-linux", "gnx-linux.run"] {
            copy_new(&stage.join(name), &Path::new(TARGET_DATA).join(name))?;
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
        tx.state(&crate::adapter::setup_state::SetupState {
            schema: 1,
            phase: "VERIFY".into(),
            outcome: "PENDING".into(),
            code: None,
        })?;
        Ok(())
    })();
    if let Err(code) = &result {
        // Preserve the last attempted phase and all partial artifacts for explicit recovery.
        if tx.phase(phase, Some(code)).is_err() {
            return Err("SETUP_FAILURE_JOURNAL_FAILED".into());
        }
        if tx
            .state(&crate::adapter::setup_state::SetupState {
                schema: 1,
                phase: phase.into(),
                outcome: "FAILED".into(),
                code: Some(code.clone()),
            })
            .is_err()
        {
            return Err("SETUP_FAILURE_STATE_FAILED".into());
        }
    }
    result
}

fn verify_installed() -> Result<SetupVerification, String> {
    let required = [
        Path::new(TARGET_PROGRAM).join("gnx.exe"),
        Path::new(TARGET_PROGRAM).join("gnx-service.exe"),
        Path::new(TARGET_PROGRAM).join("gnx-setup.exe"),
        Path::new(TARGET_DATA).join("gnx-linux"),
        Path::new(TARGET_DATA).join("gnx-linux.run"),
        Path::new(TARGET_DATA).join("bundle.tar"),
        Path::new(TARGET_DATA).join("rootfs.tar"),
    ];
    if required.iter().any(|path| !path.is_file()) {
        return Err("SETUP_VERIFY_ARTIFACTS_MISSING".into());
    }
    // Provisioning alone is never readiness. After bootstrap/reboot, use the
    // same Linux core gates that the operator sees: doctor first, then status.
    // Missing config means the bootstrap boundary has not completed yet.
    let config_path = Path::new(TARGET_DATA).join("gnx.toml");
    if !config_path.is_file() {
        return Ok(SetupVerification::RebootRequired);
    }
    let intent = fs::read_to_string(&config_path).map_err(|_| "SETUP_CONFIG_READ_FAILED")?;
    let doctor = super::runtime::invoke("doctor", &intent);
    if doctor.state != crate::report::State::Ready {
        return Ok(SetupVerification::RebootRequired);
    }
    let status = super::runtime::invoke("status", &intent);
    if status.state == crate::report::State::Ready {
        Ok(SetupVerification::Ready)
    } else {
        Ok(SetupVerification::RebootRequired)
    }
}

fn recover(rollback: bool) -> Result<(), String> {
    use crate::adapter::setup_state::{SetupState, SetupTransaction};
    let root = Path::new(SETUP_ROOT);
    if !root.is_dir() {
        return Err("SETUP_STATE_NOT_FOUND".into());
    }
    let tx = SetupTransaction::acquire_recovery(root)?;
    if rollback {
        super::account::rollback_owned_resources(rollback_account_owned(root)?)?;
        // Only remove artifacts owned by this transaction; legacy locations
        // and unrelated files are never traversed or deleted.
        // Remove only files created by this transaction. Never recursively
        // delete a target root: a partial or concurrent target must remain
        // inspectable rather than turning rollback into an unsafe wipe.
        for path in [
            Path::new(TARGET_PROGRAM).join("gnx.exe"),
            Path::new(TARGET_PROGRAM).join("gnx-service.exe"),
            Path::new(TARGET_PROGRAM).join("gnx-setup.exe"),
            Path::new(TARGET_DATA).join("gnx-linux"),
            Path::new(TARGET_DATA).join("gnx-linux.run"),
            Path::new(TARGET_DATA).join("bundle.tar"),
            Path::new(TARGET_DATA).join("rootfs.tar"),
            Path::new(TARGET_DATA).join("operator.sid"),
            root.join("staged/manifest.json"),
            root.join("staged/manifest.sig"),
            root.join("staged/gnx.exe"),
            root.join("staged/gnx-service.exe"),
            root.join("staged/gnx-setup.exe"),
            root.join("staged/gnx-linux"),
            root.join("staged/gnx-linux-bundle.tar"),
            root.join("staged/gnx-linux.run"),
            root.join("staged/rootfs.tar"),
            root.join("snapshot.json"),
            root.join("journal.json"),
        ] {
            crate::adapter::setup_state::reject_link(&path)?;
            if path.exists() {
                fs::remove_file(path).map_err(|_| "SETUP_ROLLBACK_FAILED")?;
            }
        }
        // A successful rollback must make a later apply possible. Remove only
        // empty directories owned by this transaction; unrelated content keeps
        // the directory non-empty and therefore causes an honest failure.
        for path in [
            root.join("staged"),
            PathBuf::from(TARGET_PROGRAM),
            PathBuf::from(TARGET_DATA),
        ] {
            crate::adapter::setup_state::reject_link(&path)?;
            if path.exists() {
                fs::remove_dir(&path).map_err(|_| "SETUP_ROLLBACK_NOT_EMPTY")?;
            }
        }
    }
    tx.state(&SetupState {
        schema: 1,
        phase: "RECOVERY".into(),
        outcome: if rollback { "ROLLED_BACK" } else { "RECOVERED" }.into(),
        code: None,
    })?;
    Ok(())
}

fn rollback_account_owned(root: &Path) -> Result<bool, String> {
    let path = root.join("journal.json");
    crate::adapter::setup_state::reject_link(&path)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err("SETUP_STATE_READ_FAILED".into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 4096 {
        return Err("SETUP_STATE_READ_FAILED".into());
    }
    let journal: Journal =
        serde_json::from_slice(&fs::read(path).map_err(|_| "SETUP_STATE_READ_FAILED")?)
            .map_err(|_| "SETUP_STATE_READ_FAILED")?;
    if journal.schema != 1 || journal.operation != "PROVISION" {
        return Err("SETUP_STATE_READ_FAILED".into());
    }
    // The account is first mutated in REGISTERING. Earlier failures include
    // account/service conflicts observed by preflight and are never ownership
    // evidence for deleting an existing gnx-runtime account.
    Ok(matches!(
        journal.phase.as_str(),
        "REGISTERING" | "SECURING" | "PROVISIONED"
    ))
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
            for name in [
                "gnx.exe",
                "gnx-service.exe",
                "gnx-setup.exe",
                "gnx-linux",
                "gnx-linux-bundle.tar",
                "gnx-linux.run",
            ] {
                fs::write(bundle.join(name), name).unwrap();
                artifacts.insert(
                    name.into(),
                    hex::encode(Sha256::digest(name.as_bytes())).into(),
                );
            }
            let manifest = serde_json::to_vec(&serde_json::json!({
                "schema": 1,
                "version": "0.3.1",
                "platform": "windows+linux-amd64",
                "status": "sealed",
                "rootfs_sha256": hex::encode(Sha256::digest(b"rootfs")),
                "artifacts": artifacts
            }))
            .unwrap();
            fs::write(bundle.join("manifest.json"), &manifest).unwrap();
            use ed25519_dalek::{Signer, SigningKey};
            let signing = SigningKey::from_bytes(&[
                0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
                0x2c, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c,
                0xae, 0x7f, 0x60, 0x00,
            ]);
            fs::write(
                bundle.join("manifest.sig"),
                signing.sign(&manifest).to_bytes(),
            )
            .unwrap();
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
    fn manifest_signature_is_required_and_bound_to_exact_bytes() {
        let fixture = Fixture::new();
        assert_eq!(validate_bundle(&fixture.0), Ok(()));
        let signature = fixture.0.bundle.join("manifest.sig");
        let mut bytes = fs::read(&signature).unwrap();
        bytes[0] ^= 1;
        fs::write(&signature, bytes).unwrap();
        assert_eq!(
            validate_bundle(&fixture.0).unwrap_err(),
            "RELEASE_SIGNATURE_INVALID"
        );
    }

    #[test]
    fn unsealed_manifest_is_refused_before_artifact_use() {
        let mut fixture = Fixture::new();
        let path = fixture.0.bundle.join("manifest.json");
        let bytes = fs::read_to_string(&path)
            .unwrap()
            .replace("sealed", "unsealed");
        fs::write(&path, &bytes).unwrap();
        fixture.0.manifest_sha256 = hex::encode(Sha256::digest(bytes.as_bytes()));
        assert_eq!(
            validate_bundle(&fixture.0).unwrap_err(),
            "RELEASE_UNAUTHENTICATED"
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

    #[test]
    fn target_and_transaction_roots_are_versioned_and_disjoint_from_legacy() {
        assert_eq!(TARGET_PROGRAM, r"C:\Program Files\GNX-0.3.1");
        assert_eq!(TARGET_DATA, r"C:\ProgramData\GNX-0.3.1");
        assert_eq!(SETUP_ROOT, r"C:\ProgramData\GNX-Setup-0.3.1");
        for root in [TARGET_PROGRAM, TARGET_DATA, SETUP_ROOT] {
            assert!(!root.ends_with(r"\GNX"));
            assert!(!root.ends_with(r"\GNX-Setup"));
        }
        assert_ne!(TARGET_DATA, SETUP_ROOT);
    }

    #[test]
    fn rollback_account_ownership_requires_registering_or_later_journal() {
        let root = std::env::temp_dir().join(format!(
            "gnx-recovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        for (phase, owned) in [
            ("PREFLIGHT", false),
            ("STAGING", false),
            ("PUBLISHING", false),
            ("REGISTERING", true),
            ("SECURING", true),
            ("PROVISIONED", true),
        ] {
            fs::write(
                root.join("journal.json"),
                serde_json::json!({"schema":1,"operation":"PROVISION","phase":phase,"code":null})
                    .to_string(),
            )
            .unwrap();
            assert_eq!(rollback_account_owned(&root).unwrap(), owned, "{phase}");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
