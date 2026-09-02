use std::{error::Error, fs, io::Write, path::Path, time::Duration};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{blocking::Client, redirect::Policy};
use serde::Deserialize;
use serde_json::Value;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    endpoint: String,
    node: String,
    username: String,
}

fn config(source: &str) -> Result<Config> {
    let config: Config = toml::from_str(source)?;
    if config.endpoint != "https://proxmox.mesh.gnx"
        || config.node != "gnx-compute"
        || config.username != "root@pam"
    {
        return Err("unsupported compute configuration".into());
    }
    Ok(config)
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = if args.len() == 4 {
        run(&args[1], Path::new(&args[2]), Path::new(&args[3]))
    } else {
        Err("invalid compute arguments".into())
    };
    if result.is_err() {
        // Upstream responses and authentication errors must never reach evidence.
        eprintln!("FAILED COMPUTE_OPERATION");
        std::process::exit(1);
    }
    println!("READY compute-operation");
}

fn password() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "system entropy unavailable")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn run(action: &str, config_path: &Path, state: &Path) -> Result<()> {
    let config = config(&fs::read_to_string(config_path)?)?;
    if !state.is_dir() {
        return Err("protected state directory required".into());
    }
    let secret_path = state.join("root.password");
    match action {
        "render" => {
            if !secret_path.exists() {
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(secret_path)?;
                file.write_all(password()?.as_bytes())?;
                file.sync_all()?;
            }
        }
        "verify" => {
            let password = fs::read_to_string(secret_path)?;
            let client = Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(Policy::none())
                .user_agent("GNX-Compute/0.1")
                .build()?;
            let login: Value = client
                .post(format!("{}/api2/json/access/ticket", config.endpoint))
                .form(&[
                    ("username", config.username.as_str()),
                    ("password", password.trim()),
                ])
                .send()?
                .error_for_status()?
                .json()?;
            if login["data"]["username"] != config.username {
                return Err("unexpected account".into());
            }
            let ticket = login["data"]["ticket"].as_str().ok_or("missing ticket")?;
            let cookie = format!("PVEAuthCookie={ticket}");
            let node: Value = client
                .get(format!(
                    "{}/api2/json/nodes/{}/status",
                    config.endpoint, config.node
                ))
                .header("Cookie", cookie)
                .send()?
                .error_for_status()?
                .json()?;
            if !node["data"]["uptime"].is_number() {
                return Err("node health unavailable".into());
            }
        }
        _ => return Err("unsupported compute action".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_the_first_service_to_its_declared_endpoint() {
        let source = include_str!("../../../runtime/compute/compute.toml");
        assert!(config(source).is_ok());
        for invalid in [
            source.replace("https:", "http:"),
            source.replace("proxmox.mesh.gnx", "elsewhere.invalid"),
            format!("{source}\nunknown = true"),
        ] {
            assert!(config(&invalid).is_err());
        }
    }

    #[test]
    fn generates_independent_file_safe_credentials() {
        let first = password().unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(&first).unwrap().len(), 32);
        assert!(
            first
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        );
        assert_ne!(first, password().unwrap());
    }
}
