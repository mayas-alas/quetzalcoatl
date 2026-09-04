use std::path::Path;

use crate::{Error, Result};

#[cfg(windows)]
pub fn forward(config: &Path, action: &[&str]) -> Result<String> {
    use std::io::{Read, Write};
    use zeroize::Zeroizing;

    if action.len() != 2 {
        return Err(Error::Arguments);
    }
    let config = std::fs::read(config).map_err(Error::ConfigRead)?;
    let action = [action[0], action[1]];
    let mut response = crate::windows::broker::request(action, &config, None)?;

    if action == ["access", "configure"]
        && String::from_utf8_lossy(&response.stderr).contains("FAILED ACCESS_SECRET_REQUIRED")
    {
        let secret = Zeroizing::new(
            rpassword::prompt_password("Tailscale auth key (hidden; Enter cancels): ")
                .map_err(|_| Error::Operation("ACCESS_SECRET_INPUT"))?,
        );
        if secret.trim().is_empty() {
            return Err(Error::Operation("ACCESS_SECRET_INPUT"));
        }
        response = crate::windows::broker::request(action, &config, Some(secret.as_bytes()))?;
    }

    if action == ["compute", "credentials"] && response.exit_code == 0 {
        let output = Zeroizing::new(String::from_utf8_lossy(&response.stdout).into_owned());
        if let Some(payload) = output.strip_prefix("READY broker-credentials\n") {
            print!(
                "\x1b[?1049h\x1b[2J\x1b[HGNX compute\n{payload}\n\nEnter hides this screen."
            );
            std::io::stdout().flush().map_err(Error::Spawn)?;
            let mut input = [0_u8; 1];
            let _ = std::io::stdin().read(&mut input);
            println!("\x1b[2J\x1b[H\x1b[?1049lREADY credentials-hidden");
            std::io::stdout().flush().map_err(Error::Spawn)?;
            std::process::exit(0);
        }
    }

    std::io::stdout()
        .write_all(&response.stdout)
        .map_err(Error::Spawn)?;
    std::io::stderr()
        .write_all(&response.stderr)
        .map_err(Error::Spawn)?;
    std::process::exit(response.exit_code as i32);
}

#[cfg(target_os = "linux")]
pub fn linux_command(args: &[&str]) -> std::process::Command {
    let mut command = std::process::Command::new(args.first().copied().unwrap_or("false"));
    command.args(args.get(1..).unwrap_or_default());
    command.env_remove("TS_AUTHKEY");
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

    let mut child = linux_command(args)
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn linux_commands_are_native() {
        let command = linux_command(&["sh", "-c", "exit 0"]);
        assert_eq!(command.get_program(), "sh");
    }
}
