use std::{fs, io::IsTerminal, path::Path, thread, time::Duration};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{blocking::Client, redirect::Policy};
use serde::Deserialize;
use zeroize::Zeroizing;

use crate::{
    Error, Result,
    config::Config,
    platform::systemctl,
};

const ENTRYPOINT: &str = include_str!("../runtime/compute/entrypoint.sh");
const UNIT: &str = include_str!("../runtime/compute/gnx-compute.container");

pub fn apply(config: &Config) -> Result<String> {
    crate::platform::root()?;
    let state = Path::new(&config.compute.state_dir);
    for directory in [
        state.to_path_buf(),
        state.join("storage"),
        state.join("config"),
    ] {
        crate::platform::private_dir(&directory)?;
    }
    let secret = state.join("root.password");
    if !secret.exists() {
        crate::platform::write_new(&secret, password()?.as_bytes())?;
    }
    let runtime = Path::new(&config.host.runtime_dir).join("compute/entrypoint.sh");
    crate::platform::install(&runtime, ENTRYPOINT, 0o755)?;
    crate::platform::install(
        Path::new("/etc/containers/systemd/gnx-compute.container"),
        UNIT,
        0o644,
    )?;
    systemctl(&["daemon-reload"], "COMPUTE_INSTALL")?;
    systemctl(
        &["enable", "--now", "gnx-compute.service"],
        "COMPUTE_SERVICE",
    )?;

    let mut ready = false;
    for _ in 0..60 {
        ready = crate::platform::run(
            &[
                "podman",
                "exec",
                "gnx-compute",
                "test",
                "-s",
                "/etc/pve/pve-root-ca.pem",
            ],
            None,
            "COMPUTE_STARTING",
        )
        .is_ok();
        if ready {
            break;
        }
        thread::sleep(Duration::from_secs(3));
    }
    if !ready {
        return Err(Error::Operation("COMPUTE_UPSTREAM_TLS"));
    }
    let certificate = state.join("upstream-ca.crt");
    let temporary = state.join("upstream-ca.crt.gnx-new");
    let temporary_text = temporary.to_str().ok_or(Error::ConfigInvalid)?;
    crate::platform::run(
        &[
            "podman",
            "cp",
            "gnx-compute:/etc/pve/pve-root-ca.pem",
            temporary_text,
        ],
        None,
        "COMPUTE_CA_COPY",
    )?;
    if fs::metadata(&temporary).map_err(Error::ConfigRead)?.len() == 0 {
        let _ = fs::remove_file(&temporary);
        return Err(Error::Operation("COMPUTE_CA_EMPTY"));
    }
    fs::rename(temporary, certificate).map_err(Error::ConfigRead)?;
    Ok("compute".into())
}

pub fn status(config: &Config) -> Result<String> {
    systemctl(
        &["is-active", "--quiet", "gnx-compute.service"],
        "COMPUTE_SERVICE",
    )?;
    let password = Zeroizing::new(read_secret(
        &Path::new(&config.compute.state_dir).join("root.password"),
    )?);
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(Policy::none())
        .user_agent("GNX/0.2")
        .build()
        .map_err(|_| Error::Operation("COMPUTE_CLIENT"))?;
    let login: Api<Login> = client
        .post(format!(
            "{}/api2/json/access/ticket",
            config.compute.verify_endpoint
        ))
        .form(&[
            ("username", config.compute.username.as_str()),
            ("password", password.trim()),
        ])
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|_| Error::Operation("COMPUTE_LOGIN"))?;
    if login.data.username != config.compute.username {
        return Err(Error::Operation("COMPUTE_IDENTITY"));
    }
    let cookie = format!("PVEAuthCookie={}", login.data.ticket);
    let node: Api<Node> = client
        .get(format!(
            "{}/api2/json/nodes/{}/status",
            config.compute.verify_endpoint, config.compute.node
        ))
        .header("Cookie", cookie)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|_| Error::Operation("COMPUTE_HEALTH"))?;
    if node.data.uptime == 0 {
        return Err(Error::Operation("COMPUTE_HEALTH"));
    }
    Ok(format!(
        "compute\nNode: {}\nUptime: {}s",
        config.compute.node, node.data.uptime
    ))
}

pub fn credentials(config: &Config) -> Result<String> {
    let broker = std::env::var_os("GNX_BROKER_STDIN").is_some();
    if !broker && (!std::io::stdin().is_terminal() || !std::io::stdout().is_terminal()) {
        return Err(Error::Operation("CREDENTIAL_TERMINAL_REQUIRED"));
    }
    let secret = Zeroizing::new(read_secret(
        &Path::new(&config.compute.state_dir).join("root.password"),
    )?);
    if broker {
        return Ok(format!(
            "broker-credentials\nUser: {}\nPassword: {}",
            config.compute.username,
            secret.trim()
        ));
    }
    crate::platform::show_secret(&format!(
        "GNX compute\nUser: {}\nPassword: {}",
        config.compute.username,
        secret.trim()
    ))?;
    Ok("credentials-hidden".into())
}

#[derive(Deserialize)]
struct Api<T> {
    data: T,
}

#[derive(Deserialize)]
struct Login {
    username: String,
    ticket: String,
}

#[derive(Deserialize)]
struct Node {
    uptime: u64,
}

fn password() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Operation("SYSTEM_ENTROPY"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn read_secret(path: &Path) -> Result<String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let metadata = fs::symlink_metadata(path).map_err(Error::ConfigRead)?;
    // SAFETY: geteuid has no preconditions.
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.uid() != unsafe { libc::geteuid() }
    {
        return Err(Error::Operation("CREDENTIAL_PERMISSIONS"));
    }
    fs::read_to_string(path).map_err(Error::ConfigRead)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_passwords_are_independent_and_file_safe() {
        let first = password().unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(&first).unwrap().len(), 32);
        assert_ne!(first, password().unwrap());
    }
}
