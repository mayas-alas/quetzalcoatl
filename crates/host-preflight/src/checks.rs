use std::io::{self, ErrorKind};
use std::process::Command;

use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, RegType};

use crate::windows;

pub enum Verdict {
    Pass(String),
    Fail(String),
    Error(String),
    Reboot(String),
}

const CBS_REBOOT: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Component Based Servicing\\RebootPending";
const UPDATE_REBOOT: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired";
const SESSION_MANAGER: &str = "SYSTEM\\CurrentControlSet\\Control\\Session Manager";
const PODMAN_PRODUCT: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{661EDED1-C5BC-430C-8802-015B34A382FA}";
const PODMAN_NAME: &str = "Podman CLI";
const PODMAN_VERSION: &str = "6.0.1";

pub fn windows_host() -> Verdict {
    match windows::windows_11_x64() {
        Ok(true) => Verdict::Pass("Windows 11 x64 compatible".into()),
        Ok(false) => Verdict::Fail("requires Windows 11 x64 build 22000 or newer".into()),
        Err(e) => Verdict::Error(e),
    }
}

pub fn elevation() -> Verdict {
    match windows::is_elevated() {
        Ok(true) => Verdict::Pass("process token is elevated".into()),
        Ok(false) => Verdict::Fail("process token is not elevated".into()),
        Err(e) => Verdict::Error(e),
    }
}

pub fn virtualization() -> Verdict {
    match windows::virtualization_available() {
        Ok(true) => Verdict::Pass("firmware virtualization and hypervisor are active".into()),
        Ok(false) => Verdict::Fail("firmware virtualization or hypervisor is inactive".into()),
        Err(e) => Verdict::Error(e),
    }
}

pub fn windows_features() -> Verdict {
    match feature_enabled("Microsoft-Windows-Subsystem-Linux") {
        Ok(false) => Verdict::Fail("Microsoft-Windows-Subsystem-Linux is not enabled".into()),
        Err(e) => Verdict::Error(e),
        Ok(true) => match feature_enabled("VirtualMachinePlatform") {
            Ok(true) => Verdict::Pass("WSL and VirtualMachinePlatform are enabled".into()),
            Ok(false) => Verdict::Fail("VirtualMachinePlatform is not enabled".into()),
            Err(e) => Verdict::Error(e),
        },
    }
}

pub fn prepare_windows_features() -> Verdict {
    let mut missing = Vec::new();
    for feature in [
        "Microsoft-Windows-Subsystem-Linux",
        "VirtualMachinePlatform",
    ] {
        match feature_enabled(feature) {
            Ok(true) => {}
            Ok(false) => missing.push(feature),
            Err(e) => return Verdict::Error(e),
        }
    }

    if missing.is_empty() {
        return Verdict::Pass("WSL and VirtualMachinePlatform are already enabled".into());
    }

    for feature in &missing {
        if let Err(message) = enable_feature(feature) {
            return Verdict::Fail(message);
        }
    }

    Verdict::Reboot(format!(
        "enabled {}; Windows must restart before setup resumes",
        missing.join(" and ")
    ))
}

pub fn pending_reboot() -> Verdict {
    match reboot_pending() {
        Ok(Some(source)) => Verdict::Fail(format!("pending reboot detected: {source}")),
        Ok(None) => Verdict::Pass("no pending reboot detected".into()),
        Err(e) => Verdict::Error(e),
    }
}

pub fn wsl() -> Verdict {
    let wsl = match windows::system32_file("wsl.exe") {
        Ok(path) => path,
        Err(e) => return Verdict::Error(e),
    };
    let version = match run_text(&wsl, &["--version"]) {
        Ok(output) => output,
        Err(e) => return Verdict::Error(e),
    };
    if !version.status.success() {
        return Verdict::Fail("wsl --version did not confirm Store-distributed WSL".into());
    }
    let version_text = match decode_wsl(&version.stdout) {
        Ok(text) => text,
        Err(e) => return Verdict::Error(e),
    };
    if !version_text.to_ascii_lowercase().contains("wsl")
        || !version_text.chars().any(|c| c.is_ascii_digit())
    {
        return Verdict::Fail("wsl --version did not report a Store WSL version".into());
    }
    let status = match run_text(&wsl, &["--status"]) {
        Ok(output) => output,
        Err(e) => return Verdict::Error(e),
    };
    if !status.status.success() {
        return Verdict::Fail("wsl --status did not confirm a WSL2 provider".into());
    }
    let status_text = match decode_wsl(&status.stdout) {
        Ok(text) => text,
        Err(e) => return Verdict::Error(e),
    };
    let wsl2 = status_text.lines().any(|line| {
        let line = line.trim().to_ascii_lowercase();
        (line.contains("default version") || line.contains("versi\u{00f3}n predeterminada"))
            && line.ends_with('2')
    });
    if wsl2 {
        Verdict::Pass("Store WSL with WSL2 provider is available".into())
    } else {
        Verdict::Fail("wsl --status did not report default WSL version 2".into())
    }
}

pub fn podman_msi() -> Verdict {
    match podman_installed() {
        Ok(true) => Verdict::Pass("pinned Podman CLI 6.0.1 MSI is installed".into()),
        Ok(false) => Verdict::Fail("pinned Podman CLI 6.0.1 MSI is absent or incompatible".into()),
        Err(e) => Verdict::Error(e),
    }
}

fn feature_enabled(feature: &str) -> Result<bool, String> {
    let dism = windows::system32_file("dism.exe")?;
    let feature_arg = format!("/FeatureName:{feature}");
    let output = Command::new(dism)
        .args(["/Online", "/English", "/Get-FeatureInfo", &feature_arg])
        .output()
        .map_err(|e| format!("cannot query {feature} with DISM: {e}"))?;
    if !output.status.success() {
        return Err(format!("DISM could not query {feature}"));
    }
    // DISM writes human-readable text using the active Windows OEM code page. The
    // `/English` state marker is ASCII, so lossy decoding preserves the contract
    // while avoiding locale-specific failures on otherwise compatible hosts.
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().any(|line| line.trim() == "State : Enabled"))
}

fn enable_feature(feature: &str) -> Result<(), String> {
    let dism = windows::system32_file("dism.exe")?;
    let feature_arg = format!("/FeatureName:{feature}");
    let output = Command::new(dism)
        .args([
            "/Online",
            "/English",
            "/Enable-Feature",
            &feature_arg,
            "/All",
            "/NoRestart",
        ])
        .output()
        .map_err(|e| format!("cannot enable {feature} with DISM: {e}"))?;
    match output.status.code() {
        Some(0 | 3010) => Ok(()),
        Some(code) => Err(format!("DISM could not enable {feature} (exit {code})")),
        None => Err(format!("DISM was terminated while enabling {feature}")),
    }
}

fn run_text(path: &std::path::Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new(path)
        .args(args)
        .output()
        .map_err(|e| format!("cannot execute {}: {e}", path.display()))
}

fn decode_wsl(bytes: &[u8]) -> Result<String, String> {
    let pairs_look_utf16le = bytes.len() >= 2 && bytes.chunks_exact(2).all(|pair| pair[1] == 0);
    let utf16le = bytes.starts_with(&[0xff, 0xfe]) || pairs_look_utf16le;
    if utf16le {
        if bytes.len() % 2 != 0 {
            return Err("wsl returned malformed UTF-16LE output".into());
        }
        let content = if bytes.starts_with(&[0xff, 0xfe]) {
            &bytes[2..]
        } else {
            bytes
        };
        let units: Vec<u16> = content
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        String::from_utf16(&units).map_err(|_| "wsl returned invalid UTF-16LE output".into())
    } else {
        String::from_utf8(bytes.to_vec()).map_err(|_| "wsl returned non-UTF-8 output".into())
    }
}

fn root() -> RegKey {
    RegKey::predef(HKEY_LOCAL_MACHINE)
}

fn open_64(path: &str) -> io::Result<RegKey> {
    root().open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
}

fn key_exists(path: &str) -> Result<bool, String> {
    match open_64(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("cannot query {path}: {e}")),
    }
}

fn reboot_pending() -> Result<Option<&'static str>, String> {
    if key_exists(CBS_REBOOT)? {
        return Ok(Some("CBS RebootPending"));
    }
    if key_exists(UPDATE_REBOOT)? {
        return Ok(Some("Windows Update RebootRequired"));
    }
    let key =
        open_64(SESSION_MANAGER).map_err(|e| format!("cannot query {SESSION_MANAGER}: {e}"))?;
    match key.get_raw_value("PendingFileRenameOperations") {
        Ok(value) => {
            if value.vtype != RegType::REG_MULTI_SZ || value.bytes.len() % 2 != 0 {
                return Err("PendingFileRenameOperations has an ambiguous registry value".into());
            }
            let values = String::from_utf16(
                &value
                    .bytes
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| "PendingFileRenameOperations is not valid UTF-16LE".to_string())?;
            if values.split('\0').any(|item| !item.is_empty()) {
                Ok(Some("PendingFileRenameOperations"))
            } else {
                Ok(None)
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("cannot query PendingFileRenameOperations: {e}")),
    }
}

fn podman_installed() -> Result<bool, String> {
    let key = match open_64(PODMAN_PRODUCT) {
        Ok(key) => key,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("cannot query {PODMAN_PRODUCT}: {e}")),
    };
    let installer: u32 = match key.get_value("WindowsInstaller") {
        Ok(value) => value,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("cannot query Podman WindowsInstaller: {e}")),
    };
    let name: String = match key.get_value("DisplayName") {
        Ok(value) => value,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("cannot query Podman DisplayName: {e}")),
    };
    let version: String = match key.get_value("DisplayVersion") {
        Ok(value) => value,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(format!("cannot query Podman DisplayVersion: {e}")),
    };
    Ok(installer == 1 && name == PODMAN_NAME && version == PODMAN_VERSION)
}
