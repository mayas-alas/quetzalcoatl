use super::*;

pub(super) fn apply_runtime_payload(podman: &Path) -> Result<(), GateError> {
    let files = load_payload_files()
        .map_err(|error| error.with_code("RUNTIME_PAYLOAD_INVALID", Component::Proxmox))?;
    for file in files {
        let script = payload_install_script(&file)?;
        machine_stdin(podman, ["sh", "-s"], &script)
            .map_err(|error| error.with_code("RUNTIME_PAYLOAD_INVALID", Component::Proxmox))?;
    }
    Ok(())
}

pub(super) fn payload_install_script(file: &PayloadFile) -> Result<Vec<u8>, GateError> {
    let delimiter_present = file
        .contents
        .split(|byte| *byte == b'\n')
        .any(|line| line == PAYLOAD_HEREDOC.as_bytes());
    if file.destination.contains('\'')
        || file.mode.contains('\'')
        || file.sha256.contains('\'')
        || file.contents.contains(&b'\r')
        || file.contents.contains(&0)
        || !file.contents.ends_with(b"\n")
        || delimiter_present
    {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            "payload file cannot be represented by the fixed LF text transport",
        ));
    }

    let mut script = format!(
        "set -eu\ndestination='{}'\nmode='{}'\nexpected='{}'\ndirectory=\"$(dirname \"$destination\")\"\ntemporary=\"${{destination}}.gnx-new\"\ninstall -d -m 0755 \"$directory\"\numask 077\ncat > \"$temporary\" <<'{}'\n",
        file.destination, file.mode, file.sha256, PAYLOAD_HEREDOC
    )
    .into_bytes();
    script.extend_from_slice(&file.contents);
    script.extend_from_slice(
        format!(
            "{}\nchmod \"$mode\" \"$temporary\"\nactual=\"$(sha256sum \"$temporary\" | cut -d ' ' -f 1)\"\ntest \"$actual\" = \"$expected\"\nmv -f \"$temporary\" \"$destination\"\n",
            PAYLOAD_HEREDOC
        )
        .as_bytes(),
    );
    Ok(script)
}

pub(super) fn load_machine_image() -> Result<MachineImage, GateError> {
    let manifest = runtime_root()?.join("manifest.json");
    let bytes = fs::read(&manifest).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot read runtime manifest: {error}"),
        )
    })?;
    parse_machine_image(&bytes)
}

pub(super) fn runtime_root() -> Result<PathBuf, GateError> {
    let executable = env::current_exe().map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::PodmanMachine,
            format!("cannot locate gnx-service executable: {error}"),
        )
    })?;
    executable
        .parent()
        .ok_or_else(|| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::PodmanMachine,
                "gnx-service executable has no parent directory",
            )
        })
        .map(|parent| parent.join("runtime"))
}

pub(super) fn load_payload_files() -> Result<Vec<PayloadFile>, GateError> {
    let root = runtime_root()?;
    let manifest_path = root.join("manifest.json");
    let bytes = fs::read(&manifest_path).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            format!("cannot read runtime manifest: {error}"),
        )
    })?;
    let entries = parse_payload_manifest(&bytes)?;
    let mut files = Vec::with_capacity(entries.len());
    for entry in entries {
        let source = root.join(&entry.relative_path);
        let contents = fs::read(&source).map_err(|error| {
            GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!("cannot read payload file {}: {error}", entry.relative_path),
            )
        })?;
        if sha256_bytes(&contents) != entry.sha256 {
            return Err(GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!(
                    "payload file {} does not match its locked SHA-256",
                    entry.relative_path
                ),
            ));
        }
        files.push(PayloadFile {
            destination: entry.destination,
            mode: entry.mode,
            sha256: entry.sha256,
            contents,
        });
    }
    Ok(files)
}

pub(super) fn parse_payload_manifest(bytes: &[u8]) -> Result<Vec<LockedPayloadFile>, GateError> {
    let manifest: RuntimeManifest = serde_json::from_slice(bytes).map_err(|error| {
        GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            format!("runtime manifest is invalid JSON: {error}"),
        )
    })?;
    if manifest.payload_version != 4 || manifest.files.len() != PAYLOAD_FILES.len() {
        return Err(GateError::new(
            "RUNTIME_PAYLOAD_INVALID",
            Component::Proxmox,
            format!(
                "runtime/service payload contract mismatch: service_version={} expected_payload_version=4 expected_files={} manifest_payload_version={} manifest_files={}",
                env!("CARGO_PKG_VERSION"),
                PAYLOAD_FILES.len(),
                manifest.payload_version,
                manifest.files.len(),
            ),
        ));
    }

    let mut locked = Vec::with_capacity(PAYLOAD_FILES.len());
    for spec in PAYLOAD_FILES {
        let matches = manifest
            .files
            .iter()
            .filter(|file| file.path == spec.relative_path)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!(
                    "runtime manifest must contain {} exactly once",
                    spec.relative_path
                ),
            ));
        }
        let file = matches[0];
        if file.mode != spec.mode || !valid_file_sha256(&file.sha256) {
            return Err(GateError::new(
                "RUNTIME_PAYLOAD_INVALID",
                Component::Proxmox,
                format!("runtime manifest metadata is invalid for {}", file.path),
            ));
        }
        locked.push(LockedPayloadFile {
            relative_path: file.path.clone(),
            destination: spec.destination.to_owned(),
            mode: file.mode.clone(),
            sha256: file.sha256.clone(),
        });
    }
    Ok(locked)
}

pub(super) fn valid_file_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn parse_machine_image(bytes: &[u8]) -> Result<MachineImage, GateError> {
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
        || image.artifact.as_deref() != Some(MACHINE_IMAGE_ARTIFACT)
        || image.artifact_url.as_deref() != Some(MACHINE_IMAGE_URL)
        || image.artifact_size != Some(MACHINE_IMAGE_SIZE)
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
        artifact: image.artifact.expect("validated artifact"),
        size: image.artifact_size.expect("validated artifact size"),
        sha256: image.layer_digest.expect("validated layer digest"),
    })
}

#[cfg(test)]
pub(super) fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
