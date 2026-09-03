use std::{path::Path, process::Command};

use crate::{Error, Result};

#[cfg(windows)]
pub fn forward(distribution: &str, config: &Path, action: &[&str]) -> Result<String> {
    use std::process::Stdio;

    let config = wsl_path(distribution, config)?;
    let status = linux_command(
        Some(distribution),
        &[
            "/usr/local/bin/gnx",
            "--config",
            &config,
            action[0],
            action[1],
        ],
    )
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()
    .map_err(Error::Spawn)?;
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(windows)]
fn wsl_path(distribution: &str, path: &Path) -> Result<String> {
    use std::process::Stdio;

    let path = path.canonicalize().map_err(Error::ConfigRead)?;
    let value = path.to_str().ok_or(Error::ConfigInvalid)?;
    let output = linux_command(Some(distribution), &["wslpath", "-u", value])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(Error::Spawn)?;
    if !output.status.success() {
        return Err(Error::Operation("WSL_PATH"));
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|text| text.starts_with('/'))
        .ok_or(Error::Operation("WSL_PATH"))
}

pub fn linux_command(distribution: Option<&str>, args: &[&str]) -> Command {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("wsl.exe");
        if let Some(name) = distribution {
            command.args(["-d", name]);
        }
        command.args(["--user", "root", "--exec"]).args(args);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let _ = distribution;
        let mut command = Command::new(args.first().copied().unwrap_or("false"));
        command.args(args.get(1..).unwrap_or_default());
        command
    };
    for name in ["NB_SETUP_KEY", "NB_SETUP_KEY_FILE", "TS_AUTHKEY"] {
        command.env_remove(name);
    }
    command
}

#[cfg(target_os = "linux")]
pub fn root() -> Result<()> {
    // SAFETY: geteuid has no preconditions.
    if unsafe { libc::geteuid() } == 0 {
        Ok(())
    } else {
        Err(Error::Operation("ROOT_REQUIRED"))
    }
}

#[cfg(target_os = "linux")]
pub fn private_dir(path: &Path) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    std::fs::create_dir_all(path).map_err(Error::ConfigRead)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(Error::ConfigRead)?;
    let data = std::fs::symlink_metadata(path).map_err(Error::ConfigRead)?;
    // SAFETY: geteuid has no preconditions.
    if !data.is_dir()
        || data.file_type().is_symlink()
        || data.mode() & 0o077 != 0
        || data.uid() != unsafe { libc::geteuid() }
    {
        return Err(Error::Operation("STATE_PERMISSIONS"));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn write_new(path: &Path, data: &[u8]) -> Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(Error::ConfigRead)?;
    file.write_all(data).map_err(Error::ConfigRead)?;
    file.sync_all().map_err(Error::ConfigRead)
}

#[cfg(target_os = "linux")]
pub fn install(path: &Path, data: &str, mode: u32) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    const MARKER: &str = "# Managed by GNX";
    if let Ok(previous) = std::fs::read_to_string(path) {
        if previous == data {
            return Ok(false);
        }
        if !previous.starts_with(MARKER) {
            return Err(Error::Operation("FILE_OWNERSHIP"));
        }
    }
    let parent = path.parent().ok_or(Error::Operation("INSTALL_PATH"))?;
    std::fs::create_dir_all(parent).map_err(Error::ConfigRead)?;
    let temporary = path.with_extension("gnx-new");
    std::fs::write(&temporary, data).map_err(Error::ConfigRead)?;
    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(mode))
        .map_err(Error::ConfigRead)?;
    std::fs::rename(temporary, path).map_err(Error::ConfigRead)?;
    Ok(true)
}

#[cfg(target_os = "linux")]
pub fn run(args: &[&str], input: Option<&[u8]>, operation: &'static str) -> Result<Vec<u8>> {
    use std::{io::Write, process::Stdio};

    let mut child = linux_command(None, args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(Error::Spawn)?;
    if let Some(data) = input {
        child
            .stdin
            .take()
            .ok_or(Error::Operation(operation))?
            .write_all(data)
            .map_err(|_| Error::Operation(operation))?;
    }
    let output = child.wait_with_output().map_err(Error::Spawn)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(Error::Operation(operation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_local_or_an_explicit_wsl_bridge() {
        let command = linux_command(Some("GNX-Test"), &["sh", "-c", "exit 0"]);
        #[cfg(windows)]
        assert_eq!(command.get_program(), "wsl.exe");
        #[cfg(target_os = "linux")]
        assert_eq!(command.get_program(), "sh");
    }
}
