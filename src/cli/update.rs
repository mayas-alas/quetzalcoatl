use std::path::PathBuf;
use std::time::Duration;

use crate::error::GnxError;
use crate::process::CommandSpec;

pub fn run(artifact: PathBuf, expected_sha256: String) -> Result<(), GnxError> {
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|value| value.is_ascii_hexdigit())
    {
        return Err(update_error(
            "El SHA-256 debe contener exactamente 64 caracteres hexadecimales.",
        ));
    }
    if !artifact.is_file() {
        return Err(update_error(format!(
            "No existe el artefacto {}.",
            artifact.display()
        )));
    }
    let observed = crate::download::sha256_file(&artifact)?;
    if observed != expected_sha256.to_ascii_lowercase() {
        return Err(update_error(format!(
            "SHA-256 observado {observed}; el artefacto no fue ejecutado."
        )));
    }

    let version = version_command(&artifact)
        .timeout(Duration::from_secs(60))
        .run_checked("update_version")?;
    if !version.stdout.trim().starts_with("gnx ") {
        return Err(update_error(
            "El artefacto verificado no se identifica como GNX.",
        ));
    }

    CommandSpec::new(&artifact)
        .arg("__install")
        .timeout(Duration::from_secs(2700))
        .run_checked("update_handoff")?;
    println!(
        "Release {} verificado; el nuevo instalador tomó control.",
        version.stdout.trim()
    );
    Ok(())
}

fn version_command(artifact: &std::path::Path) -> CommandSpec {
    let command = CommandSpec::new(artifact);
    if cfg!(target_os = "linux")
        && artifact
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".AppImage"))
    {
        command.args(["--appimage-extract-and-run", "version"])
    } else {
        command.arg("version")
    }
}

fn update_error(message: impl Into<String>) -> GnxError {
    GnxError::new(
        "UPDATE_ARTIFACT_INVALID",
        "update",
        "verify",
        message,
        "Use un EXE o AppImage GNX y el SHA-256 publicado en SHA256SUMS.",
        false,
        19,
    )
}
