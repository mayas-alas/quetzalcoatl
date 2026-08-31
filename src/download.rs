use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use ureq::tls::{RootCerts, TlsConfig};

use crate::error::GnxError;

#[derive(Debug, Clone, Copy)]
pub struct Artifact<'a> {
    pub id: &'a str,
    pub url: &'a str,
    pub sha256: &'a str,
    pub size: u64,
}

pub fn download_verified(artifact: Artifact<'_>, directory: &Path) -> Result<PathBuf, GnxError> {
    fs::create_dir_all(directory)
        .map_err(|error| GnxError::io("download_prepare", error.to_string()))?;
    let filename = artifact.url.rsplit('/').next().ok_or_else(|| {
        GnxError::new(
            "DEPENDENCY_LOCK_INVALID",
            "download",
            "url_filename",
            format!("{} no contiene un nombre de archivo", artifact.id),
            "Corrija dependencies.lock.toml.",
            false,
            10,
        )
    })?;
    let destination = directory.join(filename);
    if destination.exists()
        && fs::metadata(&destination)
            .map(|metadata| metadata.len() == artifact.size)
            .unwrap_or(false)
        && sha256_file(&destination)? == artifact.sha256
    {
        return Ok(destination);
    }

    let temporary = destination.with_extension("download");
    let _ = fs::remove_file(&temporary);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let response = agent.get(artifact.url).call().map_err(|error| {
        GnxError::new(
            "DOWNLOAD_FAILED",
            "download",
            "https_get",
            format!("{}: {error}", artifact.id),
            "Compruebe HTTPS y vuelva a intentar; el archivo parcial no se ejecutará.",
            true,
            11,
        )
    })?;
    let mut reader = response.into_body().into_reader();
    let mut file = File::create(&temporary)
        .map_err(|error| GnxError::io("download_stage", error.to_string()))?;
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| GnxError::io("download_read", error.to_string()))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > artifact.size {
            let _ = fs::remove_file(&temporary);
            return Err(integrity_error(artifact, "el tamaño excede el lock"));
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| GnxError::io("download_write", error.to_string()))?;
    }
    file.sync_all()
        .map_err(|error| GnxError::io("download_sync", error.to_string()))?;

    let digest = hex(hasher.finalize());
    if total != artifact.size || digest != artifact.sha256 {
        let _ = fs::remove_file(&temporary);
        return Err(integrity_error(
            artifact,
            &format!("size={total}, sha256={digest}"),
        ));
    }
    let _ = fs::remove_file(&destination);
    fs::rename(&temporary, &destination)
        .map_err(|error| GnxError::io("download_activate", error.to_string()))?;
    Ok(destination)
}

pub fn sha256_file(path: &Path) -> Result<String, GnxError> {
    let mut file = File::open(path).map_err(|error| GnxError::io("sha256", error.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| GnxError::io("sha256", error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(hasher.finalize()))
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.as_ref().len() * 2);
    for byte in bytes.as_ref() {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn integrity_error(artifact: Artifact<'_>, observed: &str) -> GnxError {
    GnxError::new(
        "DOWNLOAD_INTEGRITY_INVALID",
        "download",
        "verify",
        format!("{} no coincide con el lock: {observed}", artifact.id),
        "No ejecute el artefacto; revise la fuente oficial y actualice el lock explícitamente.",
        false,
        12,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        let path = std::env::temp_dir().join(format!("gnx-sha-{}", std::process::id()));
        fs::write(&path, b"gnx").unwrap();
        assert_eq!(
            sha256_file(&path).unwrap(),
            "cb21ba691d7c3ab29c9cb48ca26068ccf17e934111f66ed6688bd75cf5bfc473"
        );
        let _ = fs::remove_file(path);
    }
}
