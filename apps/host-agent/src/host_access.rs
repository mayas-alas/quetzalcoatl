//! Windows host access adapter for local browser entry points.
//!
//! The adapter trusts only the product public CA, maps fixed product hostnames
//! to loopback, and forwards loopback HTTPS to the current WSL NAT address.

use std::io;
use std::path::Path;

#[cfg(windows)]
const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";
#[cfg(windows)]
const STATE_DIR_NAME: &str = "host-access";
#[cfg(windows)]
const CERT_THUMBPRINT_FILE: &str = "root-ca-thumbprint.txt";
#[cfg(windows)]
const HOSTS_BEGIN: &str = "# BEGIN GNX HOST ACCESS";
#[cfg(windows)]
const HOSTS_END: &str = "# END GNX HOST ACCESS";

#[cfg(windows)]
pub fn reconcile(certificate_path: &Path) -> io::Result<()> {
    validate_public_file(certificate_path, "HOST_ACCESS_CA_INVALID")?;
    let state_dir = crate::state::state_dir().join(STATE_DIR_NAME);
    std::fs::create_dir_all(&state_dir)
        .map_err(|error| stage_error("HOST_ACCESS_STATE_FAILED", error))?;
    let thumbprint_path = state_dir.join(CERT_THUMBPRINT_FILE);
    install_root_ca(certificate_path, &thumbprint_path)?;
    reconcile_hosts()?;
    reconcile_portproxy(crate::linux_node::wsl_nat_ipv4()?)
}

#[cfg(not(windows))]
pub fn reconcile(_certificate_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
pub fn cleanup(state_dir: &Path) -> io::Result<()> {
    let access_dir = state_dir.join(STATE_DIR_NAME);
    let thumbprint_path = access_dir.join(CERT_THUMBPRINT_FILE);
    let mut first_error = None;
    if let Err(error) = remove_portproxy() {
        first_error = Some(error);
    }
    if let Err(error) = cleanup_hosts() {
        first_error.get_or_insert(error);
    }
    if let Ok(thumbprint) = std::fs::read_to_string(&thumbprint_path) {
        if let Err(error) = remove_root_ca(thumbprint.trim()) {
            first_error.get_or_insert(error);
        }
    }
    let _ = std::fs::remove_file(&thumbprint_path);
    let _ = std::fs::remove_dir(&access_dir);
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(not(windows))]
pub fn cleanup(_state_dir: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn install_root_ca(certificate_path: &Path, thumbprint_path: &Path) -> io::Result<()> {
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$certificatePath = {}
$thumbprintPath = {}
$loaded = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($certificatePath)
$cert = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($loaded.RawData)
$store = [System.Security.Cryptography.X509Certificates.X509Store]::new('Root', 'LocalMachine')
$store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
try {{
  $existing = $store.Certificates.Find([System.Security.Cryptography.X509Certificates.X509FindType]::FindByThumbprint, $cert.Thumbprint, $false)
  if ($existing.Count -eq 0) {{ $store.Add($cert) }}
}} finally {{ $store.Close() }}
[System.IO.File]::WriteAllText($thumbprintPath, $cert.Thumbprint, [System.Text.Encoding]::ASCII)
"#,
        powershell_literal(certificate_path.as_os_str().to_string_lossy().as_ref()),
        powershell_literal(thumbprint_path.as_os_str().to_string_lossy().as_ref())
    );
    run_powershell(&script, "HOST_ACCESS_CA_INSTALL_FAILED")
}

#[cfg(windows)]
fn remove_root_ca(thumbprint: &str) -> io::Result<()> {
    if !valid_thumbprint(thumbprint) {
        return Ok(());
    }
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$thumbprint = {}
$store = [System.Security.Cryptography.X509Certificates.X509Store]::new('Root', 'LocalMachine')
$store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
try {{
  $matches = $store.Certificates.Find([System.Security.Cryptography.X509Certificates.X509FindType]::FindByThumbprint, $thumbprint, $false)
  foreach ($cert in $matches) {{ $store.Remove($cert) }}
}} finally {{ $store.Close() }}
"#,
        powershell_literal(thumbprint)
    );
    run_powershell(&script, "HOST_ACCESS_CA_REMOVE_FAILED")
}

#[cfg(windows)]
fn reconcile_hosts() -> io::Result<()> {
    let path = Path::new(HOSTS_PATH);
    let current = std::fs::read_to_string(path).unwrap_or_default();
    let cleaned = filter_hosts(&current);
    let block =
        format!("{HOSTS_BEGIN}\r\n127.0.0.1 app.gnx\r\n127.0.0.1 compute.gnx\r\n127.0.0.1 rustfs.gnx\r\n127.0.0.1 pocket.gnx\r\n127.0.0.1 netbird.gnx\r\n{HOSTS_END}\r\n");
    let next = if cleaned.trim().is_empty() {
        block
    } else {
        format!("{}\r\n{}", cleaned.trim_end(), block)
    };
    if current != next {
        std::fs::write(path, next)
            .map_err(|error| stage_error("HOST_ACCESS_HOSTS_WRITE_FAILED", error))?;
    }
    Ok(())
}

#[cfg(windows)]
fn cleanup_hosts() -> io::Result<()> {
    let path = Path::new(HOSTS_PATH);
    let current = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(stage_error("HOST_ACCESS_HOSTS_READ_FAILED", error)),
    };
    let next = filter_hosts(&current);
    if current != next {
        std::fs::write(path, next)
            .map_err(|error| stage_error("HOST_ACCESS_HOSTS_WRITE_FAILED", error))?;
    }
    Ok(())
}

#[cfg(windows)]
fn filter_hosts(input: &str) -> String {
    let mut output = Vec::new();
    let mut in_block = false;
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed == HOSTS_BEGIN {
            in_block = true;
            continue;
        }
        if trimmed == HOSTS_END {
            in_block = false;
            continue;
        }
        if in_block || hosts_line_mentions_product_name(trimmed) {
            continue;
        }
        output.push(line);
    }
    if output.is_empty() {
        String::new()
    } else {
        format!("{}\r\n", output.join("\r\n"))
    }
}

#[cfg(windows)]
fn hosts_line_mentions_product_name(line: &str) -> bool {
    let body = line.split('#').next().unwrap_or_default();
    body.split_whitespace().skip(1).any(|name| {
        name.eq_ignore_ascii_case("app.gnx")
            || name.eq_ignore_ascii_case("compute.gnx")
            || name.eq_ignore_ascii_case("rustfs.gnx")
            || name.eq_ignore_ascii_case("pocket.gnx")
            || name.eq_ignore_ascii_case("netbird.gnx")
    })
}

#[cfg(windows)]
fn reconcile_portproxy(target: std::net::Ipv4Addr) -> io::Result<()> {
    remove_portproxy()?;
    run_netsh(
        &[
            "interface",
            "portproxy",
            "add",
            "v4tov4",
            "listenaddress=127.0.0.1",
            "listenport=443",
            &format!("connectaddress={target}"),
            "connectport=443",
        ],
        "HOST_ACCESS_PORTPROXY_ADD_FAILED",
    )
}

#[cfg(windows)]
fn remove_portproxy() -> io::Result<()> {
    let _ = run_netsh(
        &[
            "interface",
            "portproxy",
            "delete",
            "v4tov4",
            "listenaddress=127.0.0.1",
            "listenport=443",
        ],
        "HOST_ACCESS_PORTPROXY_DELETE_FAILED",
    );
    Ok(())
}

#[cfg(windows)]
fn run_netsh(args: &[&str], code: &str) -> io::Result<()> {
    let status = std::process::Command::new("netsh.exe")
        .args(args)
        .status()
        .map_err(|error| stage_error(code, error))?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(code))
    }
}

#[cfg(windows)]
fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(windows)]
fn run_powershell(script: &str, code: &str) -> io::Result<()> {
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    run_command(command, code)
}

#[cfg(windows)]
fn run_command(mut command: std::process::Command, code: &str) -> io::Result<()> {
    let output = command.output().map_err(|error| stage_error(code, error))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(code))
    }
}

#[cfg(windows)]
fn validate_public_file(path: &Path, code: &str) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| stage_error(code, error))?;
    if !metadata.file_type().is_file() || metadata.len() == 0 || is_reparse_point(path) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, code));
    }
    Ok(())
}

#[cfg(windows)]
fn valid_thumbprint(value: &str) -> bool {
    value.len() >= 40
        && value.len() <= 128
        && value.chars().all(|character| character.is_ascii_hexdigit())
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
fn stage_error(stage: &str, _error: io::Error) -> io::Error {
    io::Error::other(stage)
}
