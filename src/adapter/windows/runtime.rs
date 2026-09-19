use crate::report::{Report, State};
use std::{path::Path, time::Duration};

const WSL: &str = "C:\\Windows\\System32\\wsl.exe";
const ROOT: &str = "C:\\ProgramData\\GNX";
const DISTRO: &str = "GNX";
const MAX_BOOTSTRAP_BUNDLE_BYTES: u64 = 1024 * 1024 * 1024;

pub fn invoke(op: &str, intent: &str) -> Report { invoke_secret(op, intent, None) }
pub fn invoke_secret(op: &str, intent: &str, secret: Option<&crate::domain::secret::Secret>) -> Report {
    let fail = || Report::new(op, State::ActionRequired, "WSL_RUNTIME_UNAVAILABLE", Some("Verify the service-owned GNX runtime and installed Linux release."));
    let Ok(frame) = crate::wire::encode(op, intent, secret) else { return fail() };
    let result = super::super::process::run(WSL, &["--distribution", DISTRO, "--user", "root", "--exec", "/usr/bin/timeout", "600", "/usr/local/bin/gnx", op, "--broker"], Some(&frame), Duration::from_secs(620), 1024 * 1024);
    match result { Ok(o) => match serde_json::from_slice::<Report>(&o.stdout) { Ok(r) if r.schema == 1 && r.operation == op && r.exit() == o.code => r, _ => fail() }, _ => fail() }
}
fn command(args: &[&str], input: Option<&[u8]>, timeout: Duration, failure: &'static str) -> Result<Vec<u8>, String> {
    let output = super::super::process::run(WSL, args, input, timeout, 1024 * 1024).map_err(|e| if e == "PROCESS_TIMEOUT" { "WSL_COMMAND_TIMEOUT" } else { failure })?;
    match output.code { 0 => Ok(output.stdout.to_vec()), 3010 => Err("REBOOT_REQUIRED".into()), _ => Err(failure.into()) }
}
fn has_distro(bytes: &[u8], wanted: &str) -> bool {
    let text = if bytes.len() >= 2 && bytes.iter().step_by(2).any(|b| *b == 0) {
        let units = bytes.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else { String::from_utf8_lossy(bytes).into_owned() };
    text.lines().map(|line| line.trim_matches('\0').trim()).any(|line| line == wanted || line.trim_start_matches('*').trim() == wanted)
}
/// Import and install the authenticated runtime staged by setup. Every WSL operation has a deadline and fixed arguments.
pub fn bootstrap() -> Result<(), String> {
    let root = Path::new(ROOT); let bundle = root.join("bundle.tar"); let rootfs = root.join("rootfs.tar");
    if !bundle.exists() && !rootfs.exists() { return Ok(()) }
    if !bundle.is_file() { return Err("BUNDLE_MISSING".into()) }
    if std::fs::metadata(&bundle).map_err(|_| "BUNDLE_READ_FAILED")?.len() > MAX_BOOTSTRAP_BUNDLE_BYTES { return Err("BUNDLE_TOO_LARGE".into()) }
    command(&["--status"], None, Duration::from_secs(15), "WSL_UNAVAILABLE")?;
    let listed = command(&["--list", "--quiet"], None, Duration::from_secs(15), "WSL_LIST_FAILED")?;
    if !has_distro(&listed, DISTRO) {
        if !rootfs.is_file() { return Err("WSL_RUNTIME_UNAVAILABLE".into()) }
        command(&["--import", DISTRO, "C:\\ProgramData\\GNX\\wsl", "C:\\ProgramData\\GNX\\rootfs.tar", "--version", "2"], None, Duration::from_secs(300), "WSL_IMPORT_FAILED")?;
    }
    let tar = std::fs::read(&bundle).map_err(|_| "BUNDLE_READ_FAILED")?;
    command(&["-d", DISTRO, "-u", "root", "--exec", "/bin/tar", "-xf", "-", "-C", "/"], Some(&tar), Duration::from_secs(600), "BUNDLE_INSTALL_FAILED")?;
    command(&["-d", DISTRO, "-u", "root", "--exec", "/bin/sh", "-c", "set -eu; umask 077; printf '[boot]\\nsystemd=true\\n[automount]\\nenabled=false\\n[interop]\\nenabled=false\\nappendWindowsPath=false\\n' > /etc/wsl.conf; test -x /usr/local/bin/gnx; test -f /etc/gnx/gnx.toml || cp /etc/gnx/gnx.example.toml /etc/gnx/gnx.toml; chmod 600 /etc/gnx/gnx.toml"], None, Duration::from_secs(30), "ISOLATION_FAILED")?;
    command(&["--terminate", DISTRO], None, Duration::from_secs(30), "WSL_RESTART_FAILED")?;
    std::fs::remove_file(&bundle).map_err(|_| "BOOTSTRAP_CLEANUP_FAILED")?;
    if rootfs.exists() { std::fs::remove_file(&rootfs).map_err(|_| "BOOTSTRAP_CLEANUP_FAILED")?; }
    Ok(())
}
#[cfg(test)]
mod tests { use super::*; #[test] fn distro_matching_is_exact_and_handles_wsl_markers() { assert!(has_distro(b"Ubuntu\n* GNX\n", "GNX")); assert!(!has_distro(b"GNX-dev\n", "GNX")); } }
