//! Installed admission boundary for generic workload imports.
//!
//! The first shipped policy profile is AI. The PowerShell helper only admits
//! the coordinator-owned files and returns a bounded receipt; Rust never
//! interprets or executes files from the user's source tree.

use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct Admission {
    pub workload: String,
    pub revision: String,
    pub bundle_dir: PathBuf,
}

fn stable_field(value: Option<&serde_json::Value>, max: usize) -> Option<String> {
    let value = value?.as_str()?;
    if value.is_empty()
        || value.len() > max
        || value.chars().any(char::is_control)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        None
    } else {
        Some(value.to_owned())
    }
}

fn stable_workload(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?.as_str()?;
    crate::linux_node::validate_workload_id(value).ok()?;
    Some(value.to_owned())
}

fn stable_error(code: &'static str) -> io::Error {
    io::Error::other(code)
}

fn validate_source(source: &str) -> io::Result<PathBuf> {
    let extended_local = source.strip_prefix(r"\\?\").is_some_and(|value| {
        let bytes = value.as_bytes();
        bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\')
    });
    if source.is_empty()
        || source.len() > 4096
        || source.chars().any(char::is_control)
        || (source.starts_with("\\\\") && !extended_local)
        || source.starts_with("//")
        || source.starts_with(r"\\.\")
        || source.split(['/', '\\']).any(|part| part == "..")
    {
        return Err(stable_error("WORKLOAD_SOURCE_INVALID"));
    }
    let path = Path::new(source);
    if !path.is_absolute() {
        return Err(stable_error("WORKLOAD_SOURCE_INVALID"));
    }
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| stable_error("WORKLOAD_SOURCE_UNAVAILABLE"))?;
    if !metadata.file_type().is_file() && !metadata.file_type().is_dir() {
        return Err(stable_error("WORKLOAD_SOURCE_INVALID"));
    }
    #[cfg(windows)]
    if path.ancestors().any(crate::linux_node::is_reparse_point_for_import) {
        return Err(stable_error("WORKLOAD_SOURCE_INVALID"));
    }
    std::fs::canonicalize(path).map_err(|_| stable_error("WORKLOAD_SOURCE_UNAVAILABLE"))
}

// Windows canonicalization adds the extended-device prefix; PowerShell 5.1
// providers require the ordinary local-drive spelling of this verified path.
#[cfg(windows)]
fn powershell_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}

#[cfg(windows)]
pub fn import(source: &str, _state_dir: &Path) -> io::Result<Admission> {
    let source = validate_source(source)?;
    let script = crate::linux_node::trusted_installed_payload_dir("payload-host")?
        .join("workload-bundle.ps1");
    let node_payload = crate::linux_node::trusted_installed_payload_dir("payload-node")?;
    let script_meta = std::fs::symlink_metadata(&script)
        .map_err(|_| stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))?;
    if !script_meta.file_type().is_file() || crate::linux_node::is_reparse_point_for_import(&script)
    {
        return Err(stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"));
    }
    let fixed_state_dir = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join(r"GnX\Node\state");
    let powershell = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut child = std::process::Command::new(powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(powershell_path(&script))
        .args(["-Mode", "Import", "-SourcePath"])
        .arg(powershell_path(&source))
        .args(["-NodePayload"])
        .arg(powershell_path(&node_payload))
        .args(["-StateDir"])
        .arg(fixed_state_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))?;
    let stdout_reader = std::thread::spawn(move || read_bounded(stdout, 64 * 1024));
    let stderr_reader = std::thread::spawn(move || read_bounded(stderr, 64 * 1024));
    let status = child
        .wait()
        .map_err(|_| stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))?;
    let (stdout, stdout_too_large) = stdout_reader
        .join()
        .map_err(|_| stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))??;
    let (stderr, stderr_too_large) = stderr_reader
        .join()
        .map_err(|_| stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))??;
    if stdout_too_large || stderr_too_large {
        return Err(stable_error("WORKLOAD_ADMISSION_OUTPUT_TOO_LARGE"));
    }
    if !status.success() {
        return Err(admission_error(&stderr));
    }
    let value: serde_json::Value = serde_json::from_slice(&stdout)
        .map_err(|_| stable_error("WORKLOAD_ADMISSION_INVALID"))?;
    let workload = stable_workload(value.get("workload"))
        .ok_or_else(|| stable_error("WORKLOAD_ADMISSION_INVALID"))?;
    let revision = stable_field(value.get("revision"), 128)
        .ok_or_else(|| stable_error("WORKLOAD_ADMISSION_INVALID"))?;
    crate::linux_node::validate_revision(&revision)?;
    let bundle_dir = value
        .get("bundle_dir")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| stable_error("WORKLOAD_ADMISSION_INVALID"))
        .and_then(validate_source)?;
    let files = value
        .get("files")
        .and_then(serde_json::Value::as_array)
        .map(|files| files.len() as u64)
        .ok_or_else(|| stable_error("WORKLOAD_ADMISSION_INVALID"))?;
    let bytes = value
        .get("bytes")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| stable_error("WORKLOAD_ADMISSION_INVALID"))?;
    validate_receipt(&bundle_dir, &workload, &revision, files, bytes)?;
    Ok(Admission {
        workload,
        revision,
        bundle_dir,
    })
}

fn validate_receipt(
    bundle_dir: &Path,
    workload: &str,
    revision: &str,
    files: u64,
    bytes: u64,
) -> io::Result<()> {
    let path = bundle_dir.join("receipt.json");
    let text = std::fs::read(&path).map_err(|_| stable_error("WORKLOAD_RECEIPT_MISSING"))?;
    if text.len() > 1024 * 1024 {
        return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
    }
    let value: serde_json::Value = serde_json::from_slice(&text)
        .map_err(|_| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
    if value.get("schema").and_then(serde_json::Value::as_u64) != Some(1)
        || value.get("workload").and_then(serde_json::Value::as_str) != Some(workload)
        || value.get("revision").and_then(serde_json::Value::as_str) != Some(revision)
    {
        return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
    }
    let entries = value
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
    if entries.len() as u64 != files {
        return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
    }
    let mut total = 0u64;
    let canonical_root = std::fs::canonicalize(bundle_dir)
        .map_err(|_| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
    for entry in entries {
        let relative = entry
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
        if relative.is_empty()
            || relative.len() > 512
            || relative.chars().any(char::is_control)
            || relative.starts_with('/')
            || relative.starts_with('\\')
            || relative
                .split(['/', '\\'])
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
        }
        let digest = entry
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
        }
        let length = entry
            .get("bytes")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
        let file = bundle_dir.join(relative);
        let metadata = std::fs::symlink_metadata(&file)
            .map_err(|_| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
        if !metadata.file_type().is_file() {
            return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
        }
        #[cfg(windows)]
        if crate::linux_node::is_reparse_point_for_import(&file) {
            return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
        }
        if metadata.len() != length
            || std::fs::canonicalize(&file)
                .map_err(|_| stable_error("WORKLOAD_RECEIPT_INVALID"))?
                .strip_prefix(&canonical_root)
                .is_err()
        {
            return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
        }
        let mut input = std::fs::File::open(&file)
            .map_err(|_| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut read_total = 0u64;
        loop {
            let read = input
                .read(&mut buffer)
                .map_err(|_| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
            if read == 0 {
                break;
            }
            read_total = read_total
                .checked_add(read as u64)
                .ok_or_else(|| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
            hasher.update(&buffer[..read]);
        }
        if read_total != length
            || format!("{:x}", hasher.finalize()) != digest.to_ascii_lowercase()
        {
            return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
        }
        total = total
            .checked_add(length)
            .ok_or_else(|| stable_error("WORKLOAD_RECEIPT_INVALID"))?;
    }
    if total != bytes {
        return Err(stable_error("WORKLOAD_RECEIPT_INVALID"));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn import(_source: &str, _state_dir: &Path) -> io::Result<Admission> {
    Err(stable_error("WORKLOAD_ADMISSION_UNAVAILABLE"))
}

#[cfg(windows)]
fn admission_error(stderr: &[u8]) -> io::Error {
    let text = String::from_utf8_lossy(stderr);
    let code = text.lines().find_map(|line| {
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let code = value
            .get("error")
            .and_then(|error| error.get("code"))
            .or_else(|| value.get("code"))
            .and_then(serde_json::Value::as_str)?;
        (code.len() <= 80
            && !code.is_empty()
            && code
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'))
            .then(|| code.to_owned())
    });
    io::Error::other(code.as_deref().unwrap_or("WORKLOAD_ADMISSION_FAILED"))
}

#[cfg(windows)]
fn read_bounded(mut reader: impl Read, limit: usize) -> io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0u8; 8192];
    let mut too_large = false;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(output.len());
        let keep = remaining.min(count);
        output.extend_from_slice(&buffer[..keep]);
        if keep != count {
            too_large = true;
        }
    }
    Ok((output, too_large))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_validation_rejects_remote_and_traversal_paths() {
        for source in [
            r"\\server\share\bundle.zip",
            r"C:\Users\maya\..\bundle.zip",
            "C:\\Users\\maya\\bundle\n.zip",
            r"relative\bundle.zip",
        ] {
            assert!(validate_source(source).is_err());
        }
    }

    #[cfg(windows)]
    #[test]
    fn admission_error_reads_nested_stable_code() {
        let error = admission_error(br#"{"error":{"code":"AI_API_UNAVAILABLE","message":"safe"}}"#);
        assert_eq!(error.to_string(), "AI_API_UNAVAILABLE");
    }
}
