use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, RwLock};

use gnx_protocol::StatusResponse;
use serde::Deserialize;

const EXPECTED_SERVICE_SID: &str = "S-1-5-80-1414281857-1943412974-186110390-2486725240-2230548587";
const MACHINE_NAME: &str = "quetzalcoatl";
const MACHINE_CPUS: u64 = 6;
const MACHINE_MEMORY_MIB: u64 = 8192;
const MACHINE_DISK_GIB: u64 = 100;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MACHINE_IMAGE_INDEX: &str =
    "sha256:6dec5eadc84f41e55c3b6fc67264ed6c985e5f61a1d4ba243056dc0efc234bec";
const MACHINE_IMAGE_MANIFEST: &str =
    "sha256:c1b05f0f5f5cdbbfb2be4e23fccfbd0436f3aa6bfa6d4705daed00a251b03943";
const MACHINE_IMAGE_LAYER: &str =
    "sha256:0d828beef16a031a50a7cee594fd79ade36c3d3972b590cb01c32a987bd88bc3";
const MACHINE_IMAGE_COMMIT: &str = "137982aea62947e436bfb58408676e246414ea47";

const WSL_CONFIG: &str = "[wsl2]\nprocessors=6\nmemory=8GB\nswap=2GB\nnestedVirtualization=true\n";

const FEDORA_PROBE: &str = r#"set -eu
test "$(ps -p 1 -o comm= | tr -d ' ')" = systemd
test "$(stat -fc %T /sys/fs/cgroup)" = cgroup2fs
systemctl is-system-running --wait >/dev/null 2>&1 || test "$(systemctl is-system-running)" = degraded
printf 'SYSTEMD=ready;CGROUP=ready\n'
"#;

const DEVICE_PROBE: &str = r#"import array
import fcntl
import os
import stat

for path in ("/dev/kvm", "/dev/net/tun", "/dev/fuse"):
    mode = os.stat(path).st_mode
    if not stat.S_ISCHR(mode):
        raise SystemExit(f"{path} is not a character device")

kvm = os.open("/dev/kvm", os.O_RDWR | os.O_CLOEXEC)
try:
    api = fcntl.ioctl(kvm, 0xAE00, 0)
finally:
    os.close(kvm)
if api != 12:
    raise SystemExit(f"KVM API version is {api}, expected 12")

tun = os.open("/dev/net/tun", os.O_RDWR | os.O_CLOEXEC)
try:
    features = array.array("I", [0])
    fcntl.ioctl(tun, 0x800454CF, features, True)
finally:
    os.close(tun)

fuse = os.open("/dev/fuse", os.O_RDWR | os.O_CLOEXEC)
os.close(fuse)
print("KVM_API_VERSION=12;TUN=ready;FUSE=ready")
"#;

pub fn run(status: Arc<RwLock<StatusResponse>>) {
    set_stage(&status, "RUNTIME_IDENTITY");
    if let Err(error) = run_inner(&status) {
        fail(&status, error);
    }
}

fn run_inner(status: &Arc<RwLock<StatusResponse>>) -> Result<(), GateError> {
    let profile = validate_identity()?;

    set_stage(status, "WSL_PREPARING");
    configure_wsl(&profile)?;
    set_component(status, Component::Wsl, "ready");

    set_stage(status, "MACHINE_PREPARING");
    let image = load_machine_image()?;
    let podman = podman_binary()?;
    ensure_machine(&podman, &image)?;
    set_component(status, Component::PodmanMachine, "ready");
    set_stage(status, "MACHINE_READY");

    validate_fedora(&podman)?;

    set_stage(status, "KVM_CHECKING");
    validate_devices(&podman)?;
    set_component(status, Component::Kvm, "ready");
    set_stage(status, "KVM_READY");
    Ok(())
}

fn validate_identity() -> Result<PathBuf, GateError> {
    let whoami = system_binary("whoami.exe")
        .map_err(|error| error.with_code("RUNTIME_IDENTITY_INVALID", Component::None))?;
    let output = run_command(&whoami, ["/user", "/fo", "csv", "/nh"])
        .map_err(|error| error.with_code("RUNTIME_IDENTITY_INVALID", Component::None))?;
    let identity = String::from_utf8_lossy(&output.stdout);
    if !identity.contains(EXPECTED_SERVICE_SID) {
        return Err(GateError::new(
            "RUNTIME_IDENTITY_INVALID",
            Component::None,
            "service process is not running under NT SERVICE\\Quetzalcoatl",
        ));
    }

    let profile = env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_dir())
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_IDENTITY_INVALID",
                Component::None,
                "SCM did not load an absolute service profile",
            )
        })?;
    Ok(profile)
}

fn configure_wsl(profile: &Path) -> Result<(), GateError> {
    let wsl = system_binary("wsl.exe")
        .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;
    run_command(&wsl, ["--version"])
        .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;

    let config = profile.join(".wslconfig");
    let changed = fs::read(&config).map_or(true, |current| current != WSL_CONFIG.as_bytes());
    if changed {
        fs::write(&config, WSL_CONFIG).map_err(|error| {
            GateError::new(
                "WSL_NESTED_VIRT_FAILED",
                Component::Wsl,
                format!("cannot write managed .wslconfig: {error}"),
            )
        })?;
        if fs::read(&config).ok().as_deref() != Some(WSL_CONFIG.as_bytes()) {
            return Err(GateError::new(
                "WSL_NESTED_VIRT_FAILED",
                Component::Wsl,
                "managed .wslconfig did not round-trip",
            ));
        }
        run_command(&wsl, ["--shutdown"])
            .map_err(|error| error.with_code("WSL_NESTED_VIRT_FAILED", Component::Wsl))?;
    }
    Ok(())
}

fn ensure_machine(podman: &Path, image: &MachineImage) -> Result<(), GateError> {
    let list = run_command(podman, ["machine", "list", "--format", "json"])
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
    let machines: Vec<MachineListEntry> =
        serde_json::from_slice(&list.stdout).map_err(|error| {
            GateError::new(
                "MACHINE_CREATE_FAILED",
                Component::PodmanMachine,
                format!("podman machine list returned invalid JSON: {error}"),
            )
        })?;

    if machines.iter().any(|machine| machine.name != MACHINE_NAME) {
        return Err(GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "dedicated runtime identity owns an unexpected Podman machine",
        ));
    }

    match machines.iter().find(|machine| machine.name == MACHINE_NAME) {
        Some(machine) => {
            if machine.vm_type != "wsl" {
                return Err(GateError::new(
                    "MACHINE_CREATE_FAILED",
                    Component::PodmanMachine,
                    "managed machine exists with a provider other than WSL",
                ));
            }
        }
        None => {
            let image_reference = format!("docker://{}@{}", image.image, image.index_digest);
            let cpus = MACHINE_CPUS.to_string();
            let memory = MACHINE_MEMORY_MIB.to_string();
            let disk = MACHINE_DISK_GIB.to_string();
            run_command(
                podman,
                [
                    "machine",
                    "init",
                    "--provider",
                    "wsl",
                    "--image",
                    &image_reference,
                    "--cpus",
                    &cpus,
                    "--memory",
                    &memory,
                    "--disk-size",
                    &disk,
                    "--rootful",
                    "--update-connection",
                    "--now",
                    MACHINE_NAME,
                ],
            )
            .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
        }
    }

    let inspect = inspect_machine(podman)?;
    if inspect.name != MACHINE_NAME
        || !inspect.rootful
        || inspect.resources.cpus != MACHINE_CPUS
        || inspect.resources.memory != MACHINE_MEMORY_MIB
        || inspect.resources.disk_size != MACHINE_DISK_GIB
    {
        return Err(GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "managed machine configuration does not match the fixed runtime profile",
        ));
    }
    if inspect.state != "running" {
        run_command(podman, ["machine", "start", MACHINE_NAME])
            .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
    }
    Ok(())
}

fn inspect_machine(podman: &Path) -> Result<MachineInspect, GateError> {
    let output = run_command(podman, ["machine", "inspect", MACHINE_NAME])
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
    let mut machines: Vec<MachineInspect> =
        serde_json::from_slice(&output.stdout).map_err(|error| {
            GateError::new(
                "MACHINE_CREATE_FAILED",
                Component::PodmanMachine,
                format!("podman machine inspect returned invalid JSON: {error}"),
            )
        })?;
    if machines.len() != 1 {
        return Err(GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "podman machine inspect did not return exactly one managed machine",
        ));
    }
    Ok(machines.remove(0))
}

fn validate_fedora(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, "sh", "-s", FEDORA_PROBE)
        .map_err(|error| error.with_code("FEDORA_RUNTIME_UNSUPPORTED", Component::PodmanMachine))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "SYSTEMD=ready;CGROUP=ready" {
        return Err(GateError::new(
            "FEDORA_RUNTIME_UNSUPPORTED",
            Component::PodmanMachine,
            "Fedora probe did not confirm systemd and cgroup v2",
        ));
    }
    Ok(())
}

fn validate_devices(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, "python3", "-", DEVICE_PROBE)
        .map_err(|error| error.with_code("REQUIRED_DEVICE_MISSING", Component::Kvm))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "KVM_API_VERSION=12;TUN=ready;FUSE=ready" {
        return Err(GateError::new(
            "REQUIRED_DEVICE_MISSING",
            Component::Kvm,
            "device probe did not confirm KVM API 12, TUN and FUSE",
        ));
    }
    Ok(())
}

fn machine_stdin(
    podman: &Path,
    interpreter: &str,
    input_argument: &str,
    input: &str,
) -> Result<Output, GateError> {
    let mut command = Command::new(podman);
    command
        .args([
            "machine",
            "ssh",
            "--username",
            "root",
            MACHINE_NAME,
            interpreter,
            input_argument,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|error| {
        GateError::command(format!("cannot start podman machine probe: {error}"))
    })?;
    child
        .stdin
        .take()
        .ok_or_else(|| GateError::command("podman machine probe stdin is unavailable"))?
        .write_all(input.as_bytes())
        .map_err(|error| GateError::command(format!("cannot write machine probe: {error}")))?;
    let output = child
        .wait_with_output()
        .map_err(|error| GateError::command(format!("cannot wait for machine probe: {error}")))?;
    check_output(output, "podman machine probe")
}

fn load_machine_image() -> Result<MachineImage, GateError> {
    let executable = env::current_exe().map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot locate gnx-service executable: {error}"),
        )
    })?;
    let manifest = executable
        .parent()
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                "gnx-service executable has no parent directory",
            )
        })?
        .join("runtime")
        .join("manifest.json");
    let bytes = fs::read(&manifest).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot read runtime manifest: {error}"),
        )
    })?;
    parse_machine_image(&bytes)
}

fn parse_machine_image(bytes: &[u8]) -> Result<MachineImage, GateError> {
    let manifest: RuntimeManifest = serde_json::from_slice(bytes).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("runtime manifest is invalid JSON: {error}"),
        )
    })?;
    let image = manifest
        .components
        .into_iter()
        .find(|component| component.id == "podman-machine-os")
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                "runtime manifest has no podman-machine-os component",
            )
        })?;
    let platform = image.platform.as_ref();
    if image.kind.as_deref() != Some("oci_machine_image")
        || image.version.as_deref() != Some("6.0.1")
        || image.source_ref.as_deref() != Some("v6.0.1")
        || image.source_commit.as_deref() != Some(MACHINE_IMAGE_COMMIT)
        || image.image.as_deref() != Some("quay.io/podman/machine-os")
        || image.index_digest.as_deref() != Some(MACHINE_IMAGE_INDEX)
        || image.manifest_digest.as_deref() != Some(MACHINE_IMAGE_MANIFEST)
        || image.layer_digest.as_deref() != Some(MACHINE_IMAGE_LAYER)
        || platform.map(|value| value.os.as_str()) != Some("linux")
        || platform.map(|value| value.architecture.as_str()) != Some("x86_64")
        || platform.map(|value| value.disk_type.as_str()) != Some("wsl")
    {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            "podman-machine-os pin is incomplete or incompatible",
        ));
    }
    Ok(MachineImage {
        image: image.image.expect("validated image"),
        index_digest: image.index_digest.expect("validated index digest"),
    })
}

#[cfg(test)]
fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn system_binary(name: &str) -> Result<PathBuf, GateError> {
    let root = env::var_os("SystemRoot").ok_or_else(|| {
        GateError::command("SystemRoot is not available in the service environment")
    })?;
    let path = PathBuf::from(root).join("System32").join(name);
    existing_binary(path)
}

fn podman_binary() -> Result<PathBuf, GateError> {
    let root = env::var_os("ProgramFiles").ok_or_else(|| {
        GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            "ProgramFiles is not available in the service environment",
        )
    })?;
    existing_binary(PathBuf::from(root).join("Podman").join("podman.exe"))
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))
}

fn existing_binary(path: PathBuf) -> Result<PathBuf, GateError> {
    if path.is_file() {
        Ok(path)
    } else {
        Err(GateError::command(format!(
            "required executable is absent: {}",
            path.display()
        )))
    }
}

fn run_command<I, S>(program: &Path, args: I) -> Result<Output, GateError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| {
            GateError::command(format!("cannot execute {}: {error}", program.display()))
        })?;
    check_output(output, &program.display().to_string())
}

fn check_output(output: Output, operation: &str) -> Result<Output, GateError> {
    if output.status.success() {
        return Ok(output);
    }
    let detail = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    Err(GateError::command(format!(
        "{operation} failed with exit {}: {}",
        output.status.code().unwrap_or(-1),
        bounded_text(detail)
    )))
}

fn bounded_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes).replace(['\r', '\n'], " ");
    text.chars().take(400).collect::<String>().trim().to_owned()
}

fn set_stage(status: &Arc<RwLock<StatusResponse>>, stage: &str) {
    if let Ok(mut status) = status.write() {
        status.stage = stage.to_owned();
        status.overall = "pending".into();
        status.last_error = None;
    }
}

fn set_component(status: &Arc<RwLock<StatusResponse>>, component: Component, value: &str) {
    if let Ok(mut status) = status.write() {
        component.set(&mut status, value);
    }
}

fn fail(status: &Arc<RwLock<StatusResponse>>, error: GateError) {
    if let Ok(mut status) = status.write() {
        status.overall = "failed".into();
        status.stage = "FAILED".into();
        error.component.set(&mut status, "failed");
        status.last_error = Some(format!("{}: {}", error.code, error.message));
    }
}

#[derive(Clone, Copy, Debug)]
enum Component {
    None,
    Wsl,
    PodmanMachine,
    Kvm,
}

impl Component {
    fn set(self, status: &mut StatusResponse, value: &str) {
        match self {
            Self::None => {}
            Self::Wsl => status.components.wsl = value.into(),
            Self::PodmanMachine => status.components.podman_machine = value.into(),
            Self::Kvm => status.components.kvm = value.into(),
        }
    }
}

#[derive(Debug)]
struct GateError {
    code: &'static str,
    component: Component,
    message: String,
}

impl GateError {
    fn new(code: &'static str, component: Component, message: impl Into<String>) -> Self {
        Self {
            code,
            component,
            message: message.into(),
        }
    }

    fn command(message: impl Into<String>) -> Self {
        Self::new("RUNTIME_GATE_FAILED", Component::None, message)
    }

    fn with_code(mut self, code: &'static str, component: Component) -> Self {
        self.code = code;
        self.component = component;
        self
    }
}

#[derive(Deserialize)]
struct RuntimeManifest {
    components: Vec<RuntimeComponent>,
}

#[derive(Deserialize)]
struct RuntimeComponent {
    id: String,
    kind: Option<String>,
    version: Option<String>,
    source_ref: Option<String>,
    source_commit: Option<String>,
    image: Option<String>,
    index_digest: Option<String>,
    manifest_digest: Option<String>,
    layer_digest: Option<String>,
    platform: Option<RuntimePlatform>,
}

#[derive(Deserialize)]
struct RuntimePlatform {
    os: String,
    architecture: String,
    disk_type: String,
}

struct MachineImage {
    image: String,
    index_digest: String,
}

#[derive(Deserialize)]
struct MachineListEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "VMType")]
    vm_type: String,
}

#[derive(Deserialize)]
struct MachineInspect {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Rootful")]
    rootful: bool,
    #[serde(rename = "Resources")]
    resources: MachineResources,
}

#[derive(Deserialize)]
struct MachineResources {
    #[serde(rename = "CPUs")]
    cpus: u64,
    #[serde(rename = "Memory")]
    memory: u64,
    #[serde(rename = "DiskSize")]
    disk_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locked_wsl_machine_image() {
        let manifest = include_bytes!("../../../runtime/payload-v1/manifest.json");
        let image = parse_machine_image(manifest).expect("machine image pin");
        assert_eq!(image.image, "quay.io/podman/machine-os");
        assert_eq!(image.index_digest, MACHINE_IMAGE_INDEX);
    }

    #[test]
    fn rejects_mutable_or_malformed_digest() {
        assert!(!valid_sha256("quay.io/podman/machine-os:6.0"));
        assert!(!valid_sha256("sha256:ABC"));
    }

    #[test]
    fn parses_podman_inspect_contract() {
        let json = br#"[{"Name":"quetzalcoatl","State":"running","Rootful":true,"Resources":{"CPUs":6,"Memory":8192,"DiskSize":100}}]"#;
        let inspect: Vec<MachineInspect> = serde_json::from_slice(json).expect("inspect JSON");
        assert_eq!(inspect[0].name, MACHINE_NAME);
        assert!(inspect[0].rootful);
        assert_eq!(inspect[0].resources.memory, MACHINE_MEMORY_MIB);
    }
}
