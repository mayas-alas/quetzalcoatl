use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use gnx_contracts::MachineProfile;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;
use crate::infrastructure::models::{MachineImage, MachineInspect, MachineListEntry};
use crate::infrastructure::remote::{machine_stdin, machine_stdin_output, run_command};
use crate::infrastructure::runtime_assets::{
    DEVICE_PROBE, FEDORA_PROBE, MACHINE_NAME, MACHINE_NETWORK_MTU, MACHINE_OUTER_MTU,
    POD_NETWORK_MTU, RUNTIME_GENERATION, RUNTIME_GENERATION_PATH, TAILSCALE_STATE_PATH,
};

pub(crate) fn ensure_machine(
    podman: &Path,
    image: &MachineImage,
    profile: &MachineProfile,
) -> Result<(), GateError> {
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

    let mut recreated = false;
    if let Some(machine) = machines.iter().find(|machine| machine.name == MACHINE_NAME) {
        if machine.vm_type != "wsl" {
            return Err(GateError::new(
                "MACHINE_CREATE_FAILED",
                Component::PodmanMachine,
                "managed machine exists with a provider other than WSL",
            ));
        }
        ensure_machine_running(podman)?;
        wait_for_machine_ssh(podman)?;
        if read_runtime_generation(podman)?.as_deref() != Some(RUNTIME_GENERATION) {
            let tailscale_state = read_managed_tailscale_state(podman)?;
            remove_managed_machine(podman)?;
            create_managed_machine(podman, image, profile)?;
            wait_for_machine_ssh(podman)?;
            if let Some(state) = tailscale_state.as_deref() {
                restore_managed_tailscale_state(podman, state)?;
            }
            recreated = true;
        }
    } else {
        create_managed_machine(podman, image, profile)?;
        wait_for_machine_ssh(podman)?;
        recreated = true;
    }

    let inspect = inspect_machine(podman)?;
    if inspect.name != MACHINE_NAME
        || !inspect.rootful
        || inspect.resources.cpus != profile.machine_cpus
        || inspect.resources.memory != profile.machine_memory_mib
        || inspect.resources.disk_size < profile.machine_disk_gib
    {
        return Err(GateError::new(
            "MACHINE_CREATE_FAILED",
            Component::PodmanMachine,
            format!(
                "managed machine configuration does not match host profile: expected {} CPU, {} MiB RAM and {} GiB disk",
                profile.machine_cpus, profile.machine_memory_mib, profile.machine_disk_gib
            ),
        ));
    }
    if inspect.state != "running" {
        ensure_machine_running(podman)?;
        wait_for_machine_ssh(podman)?;
    }

    if recreated {
        crate::infrastructure::state::reset_runtime_checkpoint().map_err(|error| {
            GateError::new("STATE_STORAGE_FAILED", Component::None, error.message())
        })?;
        write_runtime_generation(podman)?;
    }
    Ok(())
}

pub(crate) fn create_managed_machine(
    podman: &Path,
    image: &MachineImage,
    profile: &MachineProfile,
) -> Result<(), GateError> {
    let image_path = installed_machine_image(image)?;
    let cpus = profile.machine_cpus.to_string();
    let memory = profile.machine_memory_mib.to_string();
    let disk = profile.machine_disk_gib.to_string();
    let args = vec![
        OsString::from("machine"),
        OsString::from("init"),
        OsString::from("--provider"),
        OsString::from("wsl"),
        OsString::from("--image"),
        image_path.into_os_string(),
        OsString::from("--cpus"),
        OsString::from(cpus),
        OsString::from("--memory"),
        OsString::from(memory),
        OsString::from("--disk-size"),
        OsString::from(disk),
        OsString::from("--rootful"),
        OsString::from("--update-connection"),
        OsString::from("--now"),
        OsString::from(MACHINE_NAME),
    ];
    run_command(podman, args)
        .map(|_| ())
        .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))
}

pub(crate) fn ensure_machine_running(podman: &Path) -> Result<(), GateError> {
    let inspect = inspect_machine(podman)?;
    if inspect.state != "running" {
        run_command(podman, ["machine", "start", MACHINE_NAME])
            .map_err(|error| error.with_code("MACHINE_CREATE_FAILED", Component::PodmanMachine))?;
    }
    Ok(())
}

pub(crate) fn wait_for_machine_ssh(podman: &Path) -> Result<(), GateError> {
    let mut last_error = String::from("machine SSH is not ready");
    for attempt in 0..30 {
        match machine_stdin(podman, ["true"], &[]) {
            Ok(_) => return Ok(()),
            Err(error) => last_error = error.message,
        }
        if attempt + 1 < 30 {
            thread::sleep(Duration::from_secs(1));
        }
    }
    Err(GateError::new(
        "MACHINE_CREATE_FAILED",
        Component::PodmanMachine,
        format!("managed machine SSH did not become ready: {last_error}"),
    ))
}

pub(crate) fn read_runtime_generation(podman: &Path) -> Result<Option<String>, GateError> {
    let script =
        format!("if test -f {RUNTIME_GENERATION_PATH}; then cat {RUNTIME_GENERATION_PATH}; fi\n");
    let output = machine_stdin(podman, ["sh", "-s"], script.as_bytes())
        .map_err(|error| error.with_code("MACHINE_GENERATION_FAILED", Component::PodmanMachine))?;
    let generation = String::from_utf8(output.stdout).map_err(|_| {
        GateError::new(
            "MACHINE_GENERATION_FAILED",
            Component::PodmanMachine,
            "managed machine generation marker is not UTF-8",
        )
    })?;
    let generation = generation.trim();
    if generation.is_empty() {
        Ok(None)
    } else {
        Ok(Some(generation.to_owned()))
    }
}

pub(crate) fn write_runtime_generation(podman: &Path) -> Result<(), GateError> {
    let script = format!(
        "set -eu\ninstall -d -m 0755 /etc/quetzalcoatl\nprintf '%s\\n' \"$1\" > {RUNTIME_GENERATION_PATH}.new\nchmod 0644 {RUNTIME_GENERATION_PATH}.new\nmv -f {RUNTIME_GENERATION_PATH}.new {RUNTIME_GENERATION_PATH}\ntest \"$(cat {RUNTIME_GENERATION_PATH})\" = \"$1\"\n",
    );
    machine_stdin(
        podman,
        ["sh", "-s", "--", RUNTIME_GENERATION],
        script.as_bytes(),
    )
    .map(|_| ())
    .map_err(|error| error.with_code("MACHINE_GENERATION_FAILED", Component::PodmanMachine))
}

pub(crate) fn read_managed_tailscale_state(
    podman: &Path,
) -> Result<Option<Zeroizing<Vec<u8>>>, GateError> {
    let script =
        format!("if test -s {TAILSCALE_STATE_PATH}; then cat {TAILSCALE_STATE_PATH}; fi\n");
    let output = machine_stdin(podman, ["sh", "-s"], script.as_bytes())
        .map_err(|error| error.with_code("MACHINE_GENERATION_FAILED", Component::Tailscale))?;
    if output.stdout.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Zeroizing::new(output.stdout)))
    }
}

pub(crate) fn restore_managed_tailscale_state(
    podman: &Path,
    state: &[u8],
) -> Result<(), GateError> {
    const STATE_DIRECTORY: &str = "/var/lib/quetzalcoatl/tailscale/host";
    let pending_state = format!("{TAILSCALE_STATE_PATH}.new");

    let result = (|| -> Result<(), GateError> {
        machine_stdin(
            podman,
            ["install", "-d", "-m", "0700", STATE_DIRECTORY],
            &[],
        )?;
        machine_stdin(
            podman,
            [
                OsString::from("dd"),
                OsString::from(format!("of={pending_state}")),
                OsString::from("status=none"),
            ],
            state,
        )?;
        machine_stdin(podman, ["test", "-s", pending_state.as_str()], &[])?;
        machine_stdin(
            podman,
            ["mv", "-f", pending_state.as_str(), TAILSCALE_STATE_PATH],
            &[],
        )?;
        machine_stdin(podman, ["chmod", "0600", TAILSCALE_STATE_PATH], &[])?;
        Ok(())
    })();

    if result.is_err() {
        let _ = machine_stdin_output(podman, ["rm", "-f", pending_state.as_str()], &[]);
    }

    result.map_err(|error| error.with_code("MACHINE_GENERATION_FAILED", Component::Tailscale))
}

pub(crate) fn remove_managed_machine(podman: &Path) -> Result<(), GateError> {
    let inspect = inspect_machine(podman)?;
    if inspect.state == "running" {
        run_command(podman, ["machine", "stop", MACHINE_NAME]).map_err(|error| {
            error.with_code("MACHINE_GENERATION_FAILED", Component::PodmanMachine)
        })?;
    }
    run_command(podman, ["machine", "rm", "--force", MACHINE_NAME])
        .map(|_| ())
        .map_err(|error| error.with_code("MACHINE_GENERATION_FAILED", Component::PodmanMachine))
}

pub(crate) fn installed_machine_image(image: &MachineImage) -> Result<PathBuf, GateError> {
    let executable = env::current_exe().map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot locate gnx-service executable: {error}"),
        )
    })?;
    let path = executable
        .parent()
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                "gnx-service executable has no parent directory",
            )
        })?
        .join("machine-images")
        .join(&image.artifact);
    if !path.is_file() || !verify_artifact(&path, image)? {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            "installed Podman Machine image does not match its locked size and SHA-256",
        ));
    }
    Ok(path)
}

pub(crate) fn verify_artifact(path: &Path, image: &MachineImage) -> Result<bool, GateError> {
    let metadata = fs::metadata(path).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot inspect installed machine image: {error}"),
        )
    })?;
    if metadata.len() != image.size {
        return Ok(false);
    }
    let mut file = File::open(path).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot open installed machine image: {error}"),
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                format!("cannot hash installed machine image: {error}"),
            )
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", digest.finalize()) == image.sha256)
}

pub(crate) fn inspect_machine(podman: &Path) -> Result<MachineInspect, GateError> {
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

pub(crate) fn validate_fedora(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["sh", "-s"], FEDORA_PROBE.as_bytes())
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

pub(crate) fn configure_machine_outer_mtu(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["sh", "-s"], MACHINE_OUTER_MTU.as_bytes())
        .map_err(|error| error.with_code("MACHINE_MTU_FAILED", Component::PodmanMachine))?;
    let expected = format!("MACHINE_OUTER_MTU={MACHINE_NETWORK_MTU}");
    if String::from_utf8_lossy(&output.stdout).trim() != expected {
        return Err(GateError::new(
            "MACHINE_MTU_FAILED",
            Component::PodmanMachine,
            "Podman Machine did not confirm the fixed outer MTU",
        ));
    }
    Ok(())
}

pub(crate) fn configure_pod_network_mtu(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["sh", "-s"], POD_NETWORK_MTU.as_bytes())
        .map_err(|error| error.with_code("POD_NETWORK_MTU_FAILED", Component::PodmanMachine))?;
    let confirmation = String::from_utf8_lossy(&output.stdout);
    if !confirmation
        .trim()
        .starts_with(&format!("POD_NETWORK_MTU={MACHINE_NETWORK_MTU};MEMBERS="))
    {
        return Err(GateError::new(
            "POD_NETWORK_MTU_FAILED",
            Component::PodmanMachine,
            "Podman bridge and pod veth did not confirm the fixed MTU",
        ));
    }
    Ok(())
}

pub(crate) fn validate_devices(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(podman, ["python3", "-"], DEVICE_PROBE.as_bytes())
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
