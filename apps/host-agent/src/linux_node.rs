//! Narrow adapter for the external Linux runtime contract.
//!
//! Rust owns orchestration and checkpoints; `install.run` and `verify.run` own
//! Podman, Quadlets, networking and runtime configuration.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};

pub const DISTRO_NAME: &str = "gnx-node";
pub const ROOTFS_ARTIFACT_NAME: &str = "gnx-node-rootfs.tar";
pub const ROOTFS_MANIFEST_NAME: &str = "gnx-node-rootfs.tar.sha256";
pub const PAYLOAD_MARKER_PATH: &str = "/opt/gnx/payload/node/.gnx-payload.sha256";
pub const WSL_CONFIG_NAME: &str = ".wslconfig";

// Set before the final client connection is terminated. The service keepalive
// and status monitor consult this gate so neither can reopen WSL in the small
// window between a successful termination and persistence of RuntimeStopped.
static RUNTIME_STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn runtime_stop_requested() -> bool {
    RUNTIME_STOP_REQUESTED.load(Ordering::Acquire)
}

pub fn payload_dir() -> PathBuf {
    std::env::var_os("GNX_NODE_PAYLOAD")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            installed_payload_dir("payload-node").unwrap_or_else(|| PathBuf::from("payload/node"))
        })
}

pub fn host_payload_dir() -> PathBuf {
    std::env::var_os("GNX_HOST_PAYLOAD")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            installed_payload_dir("payload-host").unwrap_or_else(|| PathBuf::from("payload/host"))
        })
}

#[cfg(windows)]
fn installed_payload_dir(directory: &str) -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|executable| installed_payload_path(&executable, directory))
}

#[cfg(not(windows))]
fn installed_payload_dir(_directory: &str) -> Option<PathBuf> {
    None
}

fn installed_payload_path(executable: &Path, directory: &str) -> Option<PathBuf> {
    executable.parent().map(|parent| parent.join(directory))
}

fn payload_copy_source(root: &Path) -> String {
    format!("{}/.", windows_path_to_wsl(&root.to_string_lossy()))
}

pub fn rootfs_artifact() -> PathBuf {
    host_payload_dir().join(ROOTFS_ARTIFACT_NAME)
}

/// Validate the immutable WSL image contract before any WSL command is run.
/// The artifact and its detached SHA-256 manifest are intentionally required;
/// a distro must never be imported from an arbitrary or stale local file.
pub fn validate_rootfs_artifact(root: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("MISSING_ROOTFS_ARTIFACT: {}", root.display()),
        )
    })?;
    if !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("INVALID_ROOTFS_ARTIFACT: {}", root.display()),
        ));
    }
    #[cfg(windows)]
    if is_reparse_point(root) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "INVALID_ROOTFS_ARTIFACT: reparse point",
        ));
    }

    let expected_name = Path::new(ROOTFS_ARTIFACT_NAME);
    if root.file_name() != expected_name.file_name() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "INVALID_ROOTFS_ARTIFACT: unexpected filename",
        ));
    }
    let manifest = root.with_file_name(ROOTFS_MANIFEST_NAME);
    let manifest_text = std::fs::read_to_string(&manifest).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("MISSING_ROOTFS_MANIFEST: {}", manifest.display()),
        )
    })?;
    let mut fields = manifest_text.split_whitespace();
    let digest = fields.next().unwrap_or_default();
    let name = fields.next().unwrap_or_default();
    if fields.next().is_some()
        || name != ROOTFS_ARTIFACT_NAME
        || digest.len() != 64
        || digest != digest.to_ascii_lowercase()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "INVALID_ROOTFS_MANIFEST",
        ));
    }
    let expected = (0..32)
        .map(|index| u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "INVALID_ROOTFS_MANIFEST"))?;
    let mut file = std::fs::File::open(root)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut DigestWriter(&mut hasher))?;
    if hasher.finalize().as_slice() != expected.as_slice() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ROOTFS_DIGEST_MISMATCH",
        ));
    }
    Ok(())
}

struct DigestWriter<'a>(&'a mut Sha256);

impl io::Write for DigestWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn validate_payload(root: &Path) -> io::Result<()> {
    for required in ["install.run", "verify.run"] {
        if !root.join(required).is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("runtime payload missing {required}"),
            ));
        }
    }
    if !root.join("services").is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "runtime payload missing services directory",
        ));
    }
    Ok(())
}

// Add a stage code only to errors that do not already carry one.  The
// provisioning layer uses the first token as its public error code; keeping
// an existing contract code here preserves useful distinctions such as a
// missing artifact or an invalid profile instead of collapsing them into
// ACCESS_DENIED.  The original OS error remains in the local diagnostic.
fn stage_error(stage: &str, error: io::Error) -> io::Error {
    let text = error.to_string();
    let code = text.split_once(':').map(|(code, _)| code).unwrap_or(&text);
    let classified = !code.is_empty()
        && code != "ACCESS_DENIED"
        && code.len() <= 80
        && code.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        });
    if classified {
        error
    } else {
        io::Error::new(error.kind(), format!("{stage}: {error}"))
    }
}

#[cfg(windows)]
fn copy_wsl_config(host_root: &Path, profile: &Path) -> io::Result<()> {
    let source = host_root.join(WSL_CONFIG_NAME);
    let metadata = match std::fs::symlink_metadata(&source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "WSL_CONFIG_MISSING: payload-host/.wslconfig",
            ));
        }
        Err(error) => return Err(stage_error("WSL_CONFIG_SOURCE_ACCESS_FAILED", error)),
    };
    if !metadata.file_type().is_file() || is_reparse_point(&source) || metadata.len() > 64 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WSL_CONFIG_INVALID: expected a bounded regular file",
        ));
    }
    let content = std::fs::read_to_string(&source).map_err(|error| {
        if error.kind() == io::ErrorKind::InvalidData {
            io::Error::new(io::ErrorKind::InvalidData, "WSL_CONFIG_INVALID: not UTF-8")
        } else {
            stage_error("WSL_CONFIG_SOURCE_READ_FAILED", error)
        }
    })?;
    if !content
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("[wsl2]"))
        || content.chars().any(|character| {
            character.is_control() && character != '\n' && character != '\r' && character != '\t'
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WSL_CONFIG_INVALID: missing [wsl2] section",
        ));
    }
    // managed_profile_dir() resolves the exact token-derived profile path and
    // validates its allowed owner and gnxnodesvc FullControl ACE before copy.
    let destination = wsl_config_path(profile);
    std::fs::write(&destination, content.as_bytes())
        .map_err(|error| stage_error("WSL_CONFIG_DESTINATION_WRITE_FAILED", error))?;
    crate::dedicated_account::restrict_read_file(&destination)
        .map_err(|error| stage_error("WSL_CONFIG_ACL_FAILED", error))?;
    if std::fs::read(&destination)
        .map_err(|error| stage_error("WSL_CONFIG_VERIFY_READ_FAILED", error))?
        != content.as_bytes()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WSL_CONFIG_COPY_FAILED",
        ));
    }
    Ok(())
}

fn wsl_config_path(profile: &Path) -> PathBuf {
    profile.join(WSL_CONFIG_NAME)
}

#[cfg(windows)]
fn materialize_payload(node_root: &Path) -> io::Result<()> {
    validate_payload(node_root)?;
    let source = payload_copy_source(node_root);
    let commands = [
        ("/usr/bin/mkdir", vec!["-p", "/opt/gnx/payload/node"]),
        (
            "/usr/bin/cp",
            vec!["-a", source.as_str(), "/opt/gnx/payload/node"],
        ),
    ];
    for (program, args) in commands {
        let output = run_wsl(program, &args)?;
        if !output.status.success() {
            return Err(io::Error::other("DISTRO_PAYLOAD_MATERIALIZE_FAILED"));
        }
    }
    let hash = run_wsl(
        "/usr/bin/sha256sum",
        &["--strict", "-c", PAYLOAD_MARKER_PATH],
    )?;
    if !hash.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DISTRO_PAYLOAD_HASH_MISMATCH",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeResult {
    Installed,
    Ready,
}

#[cfg(not(windows))]
pub fn verify(_state_dir: &Path) -> io::Result<RuntimeResult> {
    validate_payload(&payload_dir())?;
    Ok(RuntimeResult::Ready)
}

/// Stop the managed runtime and release the shared WSL VM memory. The command
/// is fixed to `wsl --shutdown`: GnX owns the dedicated service identity and
/// intentionally treats all WSL work under this host operation as disposable.
#[cfg(windows)]
pub fn stop_runtime() -> io::Result<()> {
    RUNTIME_STOP_REQUESTED.store(true, Ordering::Release);
    let result = (|| {
        let output = run_wsl(
            "/usr/bin/systemctl",
            &[
                "stop",
                "platform.target",
                "mesh.service",
                "gnx-ingress.service",
            ],
        )?;
        if !output.status.success() {
            return Err(io::Error::other("RUNTIME_STOP_FAILED"));
        }
        let output = std::process::Command::new("wsl.exe")
            .arg("--shutdown")
            .output()
            .map_err(|error| {
                io::Error::new(error.kind(), format!("WSL_SHUTDOWN_FAILED: {error}"))
            })?;
        // WSL can return a nonzero status from the LocalSystem service context
        // after it has already shut down the shared VM (that account owns no
        // registered distro). Launch failure above is actionable; the fixed
        // shutdown request itself is therefore treated as complete here.
        let _ = output;
        Ok(())
    })();
    // A failed stop leaves the durable checkpoint at NodeReady, so the
    // existing keepalive must remain available to preserve that runtime.
    if result.is_err() {
        RUNTIME_STOP_REQUESTED.store(false, Ordering::Release);
    }
    result
}

/// Start the fixed managed distro and its allowlisted platform units.
#[cfg(windows)]
pub fn start_runtime() -> io::Result<()> {
    ensure_distro()?;
    let output = run_wsl(
        "/usr/bin/systemctl",
        &[
            "start",
            "gnx-ingress.service",
            "mesh.service",
            "platform.target",
        ],
    )?;
    if output.status.success() {
        RUNTIME_STOP_REQUESTED.store(false, Ordering::Release);
        Ok(())
    } else {
        Err(io::Error::other("RUNTIME_START_FAILED"))
    }
}

#[cfg(windows)]
fn run_wsl(script: &str, args: &[&str]) -> io::Result<std::process::Output> {
    let mut command = vec![
        "--distribution".to_string(),
        DISTRO_NAME.to_string(),
        "--".to_string(),
        script.to_string(),
    ];
    command.extend(args.iter().map(|arg| (*arg).to_string()));
    crate::dedicated_account::run_wsl_as_windows_identity(&command)
        .map_err(|error| io::Error::new(error.kind(), format!("WSL_IDENTITY_FAILED: {error}")))
}

fn wsl_reports_no_distributions(status: Option<i32>, stdout: &[u8], stderr: &[u8]) -> bool {
    status == Some(-1) && stderr.is_empty() && !decode_wsl_text(stdout).trim().is_empty()
}

fn decode_wsl_text(bytes: &[u8]) -> String {
    let utf16le = bytes.starts_with(&[0xff, 0xfe])
        || (bytes.len().is_multiple_of(2)
            && (0..bytes.len() / 2)
                .filter(|index| bytes[index * 2 + 1] == 0)
                .count()
                >= bytes.len() / 4);
    if utf16le {
        let start = if bytes.starts_with(&[0xff, 0xfe]) {
            2
        } else {
            0
        };
        let units = (start..bytes.len().saturating_sub(1))
            .step_by(2)
            .map(|index| u16::from_le_bytes([bytes[index], bytes[index + 1]]));
        String::from_utf16_lossy(&units.collect::<Vec<_>>())
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(windows)]
fn distro_is_registered() -> io::Result<bool> {
    let output =
        crate::dedicated_account::run_wsl_as_windows_identity(&["--list".into(), "--quiet".into()])
            .map_err(|error| stage_error("WSL_LIST_LAUNCH_FAILED", error))?;
    if !output.status.success() {
        // `wsl.exe --list` returns unsigned exit code 0xffffffff and a
        // localized informational message when this Windows identity has no
        // registered distributions yet. That is the expected pre-import
        // state, not a failure to inspect WSL.
        if wsl_reports_no_distributions(output.status.code(), &output.stdout, &output.stderr) {
            return Ok(false);
        }
        return Err(io::Error::other(
            "DISTRO_LIST_FAILED: cannot inspect WSL registrations",
        ));
    }
    Ok(decode_wsl_text(&output.stdout)
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim())
        .any(|line| line == DISTRO_NAME))
}

fn distro_import_args(install_dir: &Path, artifact: &Path) -> [String; 6] {
    [
        "--import".into(),
        DISTRO_NAME.into(),
        install_dir.to_string_lossy().into_owned(),
        artifact.to_string_lossy().into_owned(),
        "--version".into(),
        "2".into(),
    ]
}

#[cfg(windows)]
fn import_distro(artifact: &Path) -> io::Result<()> {
    let profile = crate::dedicated_account::managed_profile_dir()
        .map_err(|error| stage_error("WSL_PROFILE_VALIDATION_FAILED", error))?;
    let install_dir = profile
        .join("AppData")
        .join("Local")
        .join("GnX")
        .join("wsl")
        .join(DISTRO_NAME);
    if let Some(parent) = install_dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| stage_error("DISTRO_IMPORT_PARENT_FAILED", error))?;
    }
    match std::fs::symlink_metadata(&install_dir) {
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "DISTRO_INSTALL_PATH_OCCUPIED: refusing to adopt an existing path",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(stage_error("DISTRO_IMPORT_PATH_CHECK_FAILED", error)),
    }
    let args = distro_import_args(&install_dir, artifact);
    let output = crate::dedicated_account::run_wsl_as_windows_identity(&args)
        .map_err(|error| stage_error("DISTRO_IMPORT_LAUNCH_FAILED", error))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(
            "DISTRO_IMPORT_FAILED: register gnx-node as gnxnodesvc",
        ))
    }
}

#[cfg(windows)]
pub fn ensure_distro() -> io::Result<()> {
    let artifact = rootfs_artifact();
    validate_rootfs_artifact(&artifact)
        .map_err(|error| stage_error("ROOTFS_VALIDATION_FAILED", error))?;
    let host_root = host_payload_dir();
    let node_root = payload_dir();
    validate_payload(&node_root)
        .map_err(|error| stage_error("PAYLOAD_VALIDATION_FAILED", error))?;
    let profile = crate::dedicated_account::managed_profile_dir()
        .map_err(|error| stage_error("WSL_PROFILE_VALIDATION_FAILED", error))?;
    // This is deliberately the first WSL-facing stage: no registration query
    // or import may start until the managed profile has its validated config.
    copy_wsl_config(&host_root, &profile)
        .map_err(|error| stage_error("WSL_CONFIG_COPY_FAILED", error))?;
    if !distro_is_registered()? {
        import_distro(&artifact)?;
    }
    materialize_payload(&node_root)?;
    let installer = run_wsl(
        "/usr/bin/test",
        &["-x", "/opt/gnx/payload/node/install.run"],
    )?;
    if !installer.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "DISTRO_PAYLOAD_MISSING: /opt/gnx/payload/node/install.run",
        ));
    }
    Ok(())
}

#[cfg(windows)]
pub fn install_with_secrets(
    state_dir: &Path,
    mesh_secret_file: Option<&Path>,
    compute_secret_file: Option<&Path>,
) -> io::Result<RuntimeResult> {
    validate_payload(&payload_dir())?;
    let state = path_arg(state_dir);
    let mut args = vec![
        "install",
        "--payload-dir",
        "/opt/gnx/payload/node",
        "--state-dir",
        state.as_str(),
    ];
    let secret = mesh_secret_file.map(path_arg);
    if let Some(secret) = secret.as_deref() {
        args.extend(["--mesh-secret-file", secret]);
    }
    let compute = compute_secret_file.map(path_arg);
    if let Some(compute) = compute.as_deref() {
        args.extend(["--compute-secret-file", compute]);
    }
    let output = run_wsl("/opt/gnx/payload/node/install.run", &args)?;
    if !output.status.success() {
        return Err(runtime_error(&output));
    }
    Ok(RuntimeResult::Installed)
}

#[cfg(windows)]
pub fn reconcile_ingress() -> io::Result<()> {
    let node_root = payload_dir();
    validate_payload(&node_root)?;
    materialize_payload(&node_root)?;
    // The external installer owns shared DNS/CA and workload-adapter upgrades.
    // Retry must not publish new gateway routes over stale DNS or script copies.
    let access = run_wsl(
        "/opt/gnx/payload/node/install.run",
        &[
            "reconcile",
            "--payload-dir",
            "/opt/gnx/payload/node",
            "--state-dir",
            &path_arg(&crate::state::state_dir()),
        ],
    )?;
    if !access.status.success() {
        return Err(runtime_error(&access));
    }
    for (source, destination, code) in [
        (
            "/opt/gnx/payload/node/network/ingress.nft",
            "/var/lib/gnx-node/ingress.nft",
            "INGRESS_COPY_FAILED",
        ),
        (
            "/opt/gnx/payload/node/gateway/routes.conf",
            "/var/lib/gnx-node/routes.conf",
            "GATEWAY_ROUTES_COPY_FAILED",
        ),
    ] {
        let copy = run_wsl("/usr/bin/install", &["-m", "0644", source, destination])?;
        if !copy.status.success() {
            return Err(io::Error::other(code));
        }
    }
    // Preserve the generated status.json while refreshing only the reviewed
    // PWA assets. Reconciliation is the update path after an MSI repair, so
    // routes alone are insufficient to make the new frontend live.
    let web = run_wsl(
        "/usr/bin/cp",
        &[
            "-a",
            "/opt/gnx/payload/node/web-app/.",
            "/var/lib/gnx-node/web-app/",
        ],
    )?;
    if !web.status.success() {
        return Err(io::Error::other("WEB_APP_COPY_FAILED"));
    }
    // Replace the named table atomically from the fixed runtime policy. The
    // oneshot unit uses the same two commands on every subsequent WSL boot.
    let _ = run_wsl("/usr/sbin/nft", &["delete", "table", "inet", "gnx_ingress"]);
    let apply = run_wsl("/usr/sbin/nft", &["-f", "/var/lib/gnx-node/ingress.nft"])?;
    if !apply.status.success() {
        return Err(io::Error::other("INGRESS_APPLY_FAILED"));
    }
    let _ = run_wsl(
        "/usr/bin/systemctl",
        &["reset-failed", "gnx-ingress.service"],
    );
    let start = run_wsl("/usr/bin/systemctl", &["start", "gnx-ingress.service"])?;
    if !start.status.success() {
        return Err(io::Error::other("INGRESS_START_FAILED"));
    }
    let compute_changed = !run_wsl(
        "/usr/bin/cmp",
        &[
            "-s",
            "/etc/containers/systemd/compute.container",
            "/opt/gnx/payload/node/services/compute.container",
        ],
    )?
    .status
    .success();
    let network = run_wsl(
        "/bin/bash",
        &["/opt/gnx/payload/node/compute/install-network.run"],
    )?;
    if !network.status.success() {
        return Err(io::Error::other("COMPUTE_NETWORK_INSTALL_FAILED"));
    }
    // Apply changed Quadlets, but do not recreate an unchanged PVE container
    // on every retry. Native Notify=healthy still gates a cold start.
    let compute_action = if compute_changed { "restart" } else { "start" };
    let compute = run_wsl("/usr/bin/systemctl", &[compute_action, "compute.service"])?;
    if !compute.status.success() {
        return Err(io::Error::other("COMPUTE_RESTART_FAILED"));
    }
    let gateway = run_wsl("/usr/bin/systemctl", &["restart", "gateway.service"])?;
    if !gateway.status.success() {
        return Err(io::Error::other("GATEWAY_RESTART_FAILED"));
    }
    let platform = run_wsl("/usr/bin/systemctl", &["start", "platform.target"])?;
    if !platform.status.success() {
        return Err(io::Error::other("PLATFORM_START_FAILED"));
    }
    Ok(())
}

#[cfg(windows)]
pub fn wsl_nat_ipv4() -> io::Result<std::net::Ipv4Addr> {
    let output = run_wsl("/usr/bin/hostname", &["-I"])?;
    if !output.status.success() {
        return Err(runtime_error(&output));
    }
    let output_text = String::from_utf8_lossy(&output.stdout);
    output_text
        .split_whitespace()
        .find_map(|value| validate_wsl_nat_ipv4(value).ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "WSL_NAT_IPV4_UNAVAILABLE"))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkloadStatus {
    pub workload: String,
    pub state: String,
    pub service: String,
    pub version: String,
}

#[cfg(windows)]
fn parse_workload_status(output: &std::process::Output) -> io::Result<WorkloadStatus> {
    if !output.status.success() {
        return Err(runtime_error(output));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<WorkloadStatus>(line).ok())
        .filter(|status| status.workload == "rustfs")
        .ok_or_else(|| io::Error::other("WORKLOAD_STATUS_INVALID"))
}

#[cfg(windows)]
pub fn workload_status() -> io::Result<WorkloadStatus> {
    let available = run_wsl("/usr/bin/test", &["-x", "/var/lib/gnx-node/lxc.run"])?;
    if !available.status.success() {
        return Ok(WorkloadStatus {
            workload: "rustfs".into(),
            state: "off".into(),
            service: "available".into(),
            version: "1.0.0-gnx.1".into(),
        });
    }
    parse_workload_status(&run_wsl(
        "/var/lib/gnx-node/lxc.run",
        &["status", "rustfs"],
    )?)
}

#[cfg(not(windows))]
pub fn workload_status() -> io::Result<WorkloadStatus> {
    Ok(WorkloadStatus {
        workload: "rustfs".into(),
        state: "off".into(),
        service: "available".into(),
        version: "1.0.0-gnx.1".into(),
    })
}

#[cfg(windows)]
pub fn control_workload(state_dir: &Path, start: bool) -> io::Result<WorkloadStatus> {
    ensure_distro()?;
    let refresh = run_wsl(
        "/usr/bin/install",
        &[
            "-m",
            "0755",
            "/opt/gnx/payload/node/compute/lxc.run",
            "/var/lib/gnx-node/lxc.run",
        ],
    )?;
    if !refresh.status.success() {
        install_with_secrets(state_dir, None, None)?;
    }
    let action = if start { "start" } else { "shutdown" };
    parse_workload_status(&run_wsl("/var/lib/gnx-node/lxc.run", &[action, "rustfs"])?)
}

#[cfg(not(windows))]
pub fn control_workload(_state_dir: &Path, start: bool) -> io::Result<WorkloadStatus> {
    Ok(WorkloadStatus {
        workload: "rustfs".into(),
        state: if start { "running" } else { "off" }.into(),
        service: if start { "healthy" } else { "stopped" }.into(),
        version: "1.0.0-gnx.1".into(),
    })
}

#[cfg(windows)]
pub fn private_ip() -> io::Result<String> {
    let output = run_wsl(
        "/usr/bin/podman",
        &["exec", "gnx-mesh", "tailscale", "ip", "-4"],
    )?;
    if !output.status.success() {
        return Err(runtime_error(&output));
    }
    let output_text = String::from_utf8_lossy(&output.stdout);
    let value = output_text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "mesh IP unavailable"))?;
    validate_private_ip(value)
}

fn validate_private_ip(value: &str) -> io::Result<String> {
    let parsed = value
        .parse::<std::net::IpAddr>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid mesh IP"))?;
    if parsed.is_unspecified() || parsed.is_loopback() || parsed.is_multicast() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid mesh IP",
        ));
    }
    Ok(parsed.to_string())
}

fn validate_wsl_nat_ipv4(value: &str) -> io::Result<std::net::Ipv4Addr> {
    let parsed = value
        .parse::<std::net::Ipv4Addr>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid WSL NAT IPv4"))?;
    if parsed.is_unspecified()
        || parsed.is_loopback()
        || parsed.is_multicast()
        || parsed.is_link_local()
        || parsed.is_broadcast()
        || !parsed.is_private()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid WSL NAT IPv4",
        ));
    }
    Ok(parsed)
}

#[cfg(windows)]
pub fn verify(state_dir: &Path) -> io::Result<RuntimeResult> {
    let output = run_wsl(
        "/opt/gnx/payload/node/verify.run",
        &["--state-dir", &path_arg(state_dir)],
    )?;
    if !output.status.success() {
        return Err(runtime_error(&output));
    }
    Ok(RuntimeResult::Ready)
}

/// Read only the fixed pass/fail check names emitted by the canonical runtime
/// verifier. Raw output is intentionally not retained or sent to the browser.
#[cfg(windows)]
pub fn runtime_checks(state_dir: &Path) -> io::Result<Vec<crate::state::RuntimeCheck>> {
    const ALLOWED: &[&str] = &[
        "podman",
        "tun",
        "fuse",
        "kvm",
        "platform_target",
        "mesh",
        "mesh_authenticated",
        "gateway",
        "app_https",
        "compute",
        "compute_gateway_https",
        "compute_https",
        "dns_rustfs",
        "rustfs_gateway_https",
    ];
    let output = run_wsl(
        "/opt/gnx/payload/node/verify.run",
        &["--state-dir", &path_arg(state_dir)],
    )?;
    let checks = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|line| {
            let name = line.get("check")?.as_str()?;
            let result = line.get("result")?.as_str()?;
            (ALLOWED.contains(&name) && matches!(result, "pass" | "fail")).then(|| {
                crate::state::RuntimeCheck {
                    name: name.into(),
                    result: result.into(),
                }
            })
        })
        .collect::<Vec<_>>();
    if checks.is_empty() {
        Err(io::Error::other("RUNTIME_CHECKS_UNAVAILABLE"))
    } else {
        Ok(checks)
    }
}

/// Copy the intentionally small, sanitized state projection into the existing
/// Caddy document root. The host owns the source; the browser gets no pipe or
/// privileged endpoint.
#[cfg(windows)]
pub fn publish_app_status(source: &Path) -> io::Result<()> {
    let source = path_arg(source);
    let output = run_wsl(
        "/usr/bin/install",
        &[
            "-m",
            "0644",
            &source,
            "/var/lib/gnx-node/web-app/status.json",
        ],
    )?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other("APP_STATUS_PUBLISH_FAILED"))
    }
}

#[cfg(windows)]
fn path_arg(path: &Path) -> String {
    windows_path_to_wsl(path.to_string_lossy().as_ref())
}

fn windows_path_to_wsl(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        return format!("/mnt/{drive}/{}", normalized[2..].trim_start_matches('/'));
    }
    normalized
}

#[cfg(windows)]
fn is_reparse_point(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, FILE_ATTRIBUTE_REPARSE_POINT, INVALID_FILE_ATTRIBUTES,
    };
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
    attributes != INVALID_FILE_ATTRIBUTES && attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(windows)]
fn compute_diagnostic() -> io::Result<String> {
    use std::fmt::Write as _;
    ensure_distro()?;
    let mut report = String::new();
    for (label, program, args) in [
        (
            "unit",
            "/usr/bin/systemctl",
            vec!["status", "compute.service", "--no-pager", "--lines=30"],
        ),
        (
            "container",
            "/usr/bin/podman",
            vec![
                "inspect",
                "--format",
                "{{.State.Status}} health={{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}} error={{.State.Error}}",
                "gnx-compute",
            ],
        ),
        (
            "pve",
            "/usr/bin/podman",
            vec![
                "exec",
                "gnx-compute",
                "systemctl",
                "status",
                "pve-cluster.service",
                "pveproxy.service",
                "--no-pager",
                "--lines=20",
            ],
        ),
        (
            "kernel-firewall",
            "/usr/bin/podman",
            vec!["exec", "gnx-compute", "/bin/bash", "-c", "pve-firewall status; ebtables --version; ip6tables --version; ip6tables-restore --test -T filter -n < /var/lib/pve-firewall/ip6cmdlist; journalctl -u pve-firewall --no-pager -n 5"],
        ),
        (
            "pve-network",
            "/usr/bin/podman",
            vec!["exec", "gnx-compute", "/bin/bash", "-c", "timeout 15 pct exec 100 -- /bin/bash -c 'cat /etc/network/interfaces; cat /etc/systemd/network/*.network; networkctl status eth0 --no-pager; journalctl -u systemd-networkd --no-pager -n 15'; journalctl -u dnsmasq --no-pager -n 15; iptables -t nat -S; iptables -S INPUT; systemctl status gnx-lxc-network pve-firewall --no-pager --lines=8; journalctl -u pve-firewall --no-pager -n 5"],
        ),
        (
            "templates",
            "/usr/bin/podman",
            vec!["exec", "gnx-compute", "/usr/bin/pveam", "available", "--section", "system"],
        ),
        (
            "storage",
            "/usr/bin/podman",
            vec!["exec", "gnx-compute", "/usr/sbin/pvesm", "status"],
        ),
        (
            "rustfs",
            "/opt/gnx/payload/node/compute/lxc.run",
            vec!["diagnose", "rustfs"],
        ),
    ] {
        let mut bounded_args = vec!["--kill-after=5", "25", program];
        bounded_args.extend(args);
        let output = run_wsl("/usr/bin/timeout", &bounded_args)?;
        let _ = writeln!(report, "--- {label} (exit={}) ---", output.status);

        let combined = [output.stdout, output.stderr].concat();
        let text = String::from_utf8_lossy(&combined);
        for line in text.lines().take(120) {
            let _ = writeln!(report, "{}", line.chars().take(500).collect::<String>());
        }
    }
    Ok(report)
}

#[cfg(windows)]
pub fn diagnose_compute() -> io::Result<()> {
    print!("{}", compute_diagnostic()?);
    Ok(())
}

#[cfg(windows)]
pub fn diagnose_compute_file(state_dir: &Path) -> io::Result<()> {
    let report = compute_diagnostic()?;
    std::fs::write(state_dir.join("compute-diagnostic.txt"), report)
}

#[cfg(windows)]
fn runtime_error(output: &std::process::Output) -> io::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Runtime diagnostics are JSONL and must never leak credentials into the
    // progress store. Keep only a short, non-secret classification here.
    let code = stderr
        .lines()
        .find_map(|line| line.split("\"code\":\"").nth(1))
        .and_then(|v| v.split('\"').next())
        .unwrap_or("RUNTIME_FAILED");
    io::Error::other(code.chars().take(80).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_validation_rejects_missing_contract() {
        let root = std::env::temp_dir().join(format!("gnx-node-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(validate_payload(&root).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn private_ip_validation_rejects_non_node_addresses() {
        assert!(validate_private_ip("127.0.0.1").is_err());
        assert_eq!(validate_private_ip("100.64.0.12").unwrap(), "100.64.0.12");
    }

    #[test]
    fn windows_paths_translate_to_wsl_mounts() {
        assert_eq!(
            windows_path_to_wsl(r"C:\ProgramData\GNX\state"),
            "/mnt/c/ProgramData/GNX/state"
        );
        assert_eq!(windows_path_to_wsl(r"D:/staged/env"), "/mnt/d/staged/env");
        assert_eq!(windows_path_to_wsl("/var/lib/gnx"), "/var/lib/gnx");
    }

    #[test]
    fn wsl_list_output_decodes_utf16_and_utf8() {
        let utf16: Vec<u8> = "* gnx-builder\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(decode_wsl_text(&utf16), "* gnx-builder\r\n");
        assert_eq!(decode_wsl_text(b"gnx-node\r\n"), "gnx-node\r\n");
        assert!(wsl_reports_no_distributions(Some(-1), &utf16, b""));
        assert!(!wsl_reports_no_distributions(Some(-1), b"", b"error"));
        assert!(!wsl_reports_no_distributions(Some(1), &utf16, b""));
    }

    #[test]
    fn missing_rootfs_reports_the_exact_artifact_contract() {
        let root = std::env::temp_dir()
            .join(format!("gnx-rootfs-missing-{}", std::process::id()))
            .join(ROOTFS_ARTIFACT_NAME);
        let error = validate_rootfs_artifact(&root).unwrap_err();
        assert!(error.to_string().starts_with("MISSING_ROOTFS_ARTIFACT:"));
        assert!(error.to_string().contains(ROOTFS_ARTIFACT_NAME));
    }

    #[test]
    fn rootfs_manifest_hash_is_verified() {
        let root = std::env::temp_dir().join(format!("gnx-rootfs-fixture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let artifact = root.join(ROOTFS_ARTIFACT_NAME);
        let bytes = b"sealed-rootfs-fixture";
        std::fs::write(&artifact, bytes).unwrap();
        let digest = Sha256::digest(bytes);
        std::fs::write(
            artifact.with_file_name(ROOTFS_MANIFEST_NAME),
            format!("{digest:x}  {ROOTFS_ARTIFACT_NAME}\n"),
        )
        .unwrap();
        validate_rootfs_artifact(&artifact).unwrap();
        std::fs::write(
            artifact.with_file_name(ROOTFS_MANIFEST_NAME),
            format!("{}  {ROOTFS_ARTIFACT_NAME}\n", "0".repeat(64)),
        )
        .unwrap();
        assert_eq!(
            validate_rootfs_artifact(&artifact).unwrap_err().to_string(),
            "ROOTFS_DIGEST_MISMATCH"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn distro_import_is_fixed_to_contract_identity() {
        let args = distro_import_args(
            Path::new(r"C:\ProgramData\GNX\wsl\gnx-node"),
            Path::new(r"C:\ProgramData\GNX\gnx-node-rootfs.tar"),
        );
        assert_eq!(args[0], "--import");
        assert_eq!(args[1], DISTRO_NAME);
        assert_eq!(args[4], "--version");
        assert_eq!(args[5], "2");
        assert!(!args.iter().any(|arg| arg == "--user"));
    }

    #[test]
    fn installed_service_payloads_resolve_beside_executable() {
        let executable = Path::new("install").join("gnx-host-agent.exe");
        assert_eq!(
            installed_payload_path(&executable, "payload-node").unwrap(),
            Path::new("install").join("payload-node")
        );
        assert_eq!(
            installed_payload_path(&executable, "payload-host").unwrap(),
            Path::new("install").join("payload-host")
        );
    }

    #[test]
    fn payload_materialization_uses_drvfs_source_and_fixed_destination() {
        assert_eq!(
            payload_copy_source(Path::new(r"C:\Program Files\GnX Node\payload-node")),
            "/mnt/c/Program Files/GnX Node/payload-node/."
        );
        assert_eq!(
            PAYLOAD_MARKER_PATH,
            "/opt/gnx/payload/node/.gnx-payload.sha256"
        );
    }

    #[test]
    fn wsl_config_is_in_managed_profile_and_acl_staged_for_managed_identity() {
        let profile = Path::new("gnxnodesvc");
        assert_eq!(wsl_config_path(profile), profile.join(WSL_CONFIG_NAME));
        assert_eq!(WSL_CONFIG_NAME, ".wslconfig");
        assert_eq!(
            profile.file_name().and_then(|name| name.to_str()),
            Some("gnxnodesvc")
        );
    }
}
