use std::{error::Error, fs, io::Write, path::Path, time::Duration};

use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::{Method, blocking::Client, redirect::Policy};
use serde::Deserialize;
use serde_json::{Value, json};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    endpoint: String,
    owner_email: String,
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("FAILED CONTROL_ARGUMENTS");
        std::process::exit(2);
    }
    if run(&args[1], Path::new(&args[2]), Path::new(&args[3])).is_err() {
        // Never render upstream errors: they may contain authentication material.
        eprintln!("FAILED CONTROL_{}", args[1].to_ascii_uppercase());
        std::process::exit(1);
    }
    println!("READY control={}", args[1]);
}

fn run(action: &str, config: &Path, state: &Path) -> Result<()> {
    let config: Config = toml::from_str(&fs::read_to_string(config)?)?;
    if config.endpoint != "https://mesh.gnx"
        || !config.owner_email.contains('@')
        || config.owner_email.chars().any(char::is_control)
        || !state.is_dir()
    {
        return Err("invalid local control configuration".into());
    }
    if action == "render" {
        let server = state.join("server.yaml");
        if !server.exists() {
            let template = include_str!("../../../runtime/control/server.yaml.in");
            let rendered = template
                .replace("__RELAY_SECRET__", &secret()?)
                .replace("__STORE_KEY__", &secret()?)
                .replace("__COOKIE_KEY__", &secret()?);
            write_new(&server, rendered.as_bytes())?;
        }
        let owner = state.join("owner.json");
        if !owner.exists() && !state.join("bootstrap.json").exists() {
            write_new(
                &owner,
                &serde_json::to_vec(&json!({
                    "email": config.owner_email, "name": "GNX Owner", "password": secret()?,
                    "create_pat": true, "pat_expire_in": 1
                }))?,
            )?;
        }
        return Ok(());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(Policy::none())
        .user_agent("GNX-Control/0.1")
        .build()?;
    let api = |method, route: &str, token: Option<&str>, body: Option<&Value>| -> Result<Value> {
        let mut request = client.request(method, format!("{}/api{route}", config.endpoint));
        if let Some(token) = token {
            request = request.header("Authorization", format!("Token {token}"));
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send()?.error_for_status()?;
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        let text = response.text()?;
        Ok(if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text)?
        })
    };
    if action == "bootstrap" && !state.join("bootstrap.json").exists() {
        let instance = api(Method::GET, "/instance", None, None)?;
        if instance["setup_required"] != true {
            return Err("existing owner requires explicit recovery".into());
        }
        let owner: Value = serde_json::from_slice(&fs::read(state.join("owner.json"))?)?;
        let response = api(Method::POST, "/setup", None, Some(&owner))?;
        write_new(
            &state.join("bootstrap.json"),
            &serde_json::to_vec(&response)?,
        )?;
    }
    let auth: Value = serde_json::from_slice(&fs::read(state.join("bootstrap.json"))?)?;
    let token = auth["personal_access_token"]
        .as_str()
        .ok_or("no bootstrap token")?;
    let user = auth["user_id"].as_str().ok_or("no bootstrap owner")?;
    match action {
        "bootstrap" => {
            if !state.join("token-id").exists() {
                let tokens = api(
                    Method::GET,
                    &format!("/users/{user}/tokens"),
                    Some(token),
                    None,
                )?;
                let tokens = tokens.as_array().ok_or("invalid token list")?;
                if tokens.len() != 1 {
                    return Err("ambiguous bootstrap token".into());
                }
                write_new(
                    &state.join("token-id"),
                    tokens[0]["id"]
                        .as_str()
                        .ok_or("missing token id")?
                        .as_bytes(),
                )?;
            }
            if !state.join("join.json").exists() {
                let key = api(
                    Method::POST,
                    "/setup-keys",
                    Some(token),
                    Some(&json!({
                        "name": "gnx-windows", "type": "one-off", "expires_in": 3600,
                        "auto_groups": [], "usage_limit": 1, "ephemeral": false
                    })),
                )?;
                write_new(&state.join("join.json"), &serde_json::to_vec(&key)?)?;
            }
            if !state.join("join.key").exists() {
                let key: Value = serde_json::from_slice(&fs::read(state.join("join.json"))?)?;
                write_new(
                    &state.join("join.key"),
                    key["key"].as_str().ok_or("missing join key")?.as_bytes(),
                )?;
            }
        }
        "verify" => {
            let peers = api(Method::GET, "/peers", Some(token), None)?;
            let peers = peers.as_array().ok_or("invalid peer list")?;
            if peers.len() != 1 || peers[0]["connected"] != true {
                return Err("expected exactly one connected peer".into());
            }
            let id = peers[0]["id"].as_str().ok_or("missing peer identity")?;
            let identity = state.join("peer-id");
            if identity.exists() {
                if fs::read_to_string(identity)? != id {
                    return Err("peer identity changed".into());
                }
            } else {
                write_new(&identity, id.as_bytes())?;
            }
        }
        "finalize" => {
            let key: Value = serde_json::from_slice(&fs::read(state.join("join.json"))?)?;
            let key_id = key["id"].as_str().ok_or("missing join id")?;
            api(
                Method::DELETE,
                &format!("/setup-keys/{key_id}"),
                Some(token),
                None,
            )?;
            let id = fs::read_to_string(state.join("token-id"))?;
            api(
                Method::DELETE,
                &format!("/users/{user}/tokens/{id}"),
                Some(token),
                None,
            )?;
            for name in ["join.key", "join.json", "bootstrap.json", "token-id"] {
                fs::remove_file(state.join(name))?;
            }
        }
        _ => return Err("unknown action".into()),
    }
    Ok(())
}

fn secret() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "system entropy unavailable")?;
    Ok(STANDARD.encode(bytes))
}

fn write_new(path: &Path, value: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(value)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("gnx-control-test-{}-{suffix}", std::process::id()));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn secrets_are_independent_256_bit_values() {
        let first = secret().unwrap();
        assert_eq!(STANDARD.decode(&first).unwrap().len(), 32);
        assert_ne!(first, secret().unwrap());
    }

    #[test]
    fn render_preserves_existing_identity_and_rejects_overwrite() {
        let state = test_state();
        let config = state.join("control.toml");
        fs::write(
            &config,
            "endpoint='https://mesh.gnx'\nowner_email='operator@email.gnx'",
        )
        .unwrap();
        run("render", &config, &state).unwrap();
        let server = fs::read(state.join("server.yaml")).unwrap();
        let owner = fs::read(state.join("owner.json")).unwrap();
        run("render", &config, &state).unwrap();
        assert!(fs::read(state.join("server.yaml")).unwrap() == server);
        assert!(fs::read(state.join("owner.json")).unwrap() == owner);
        assert!(write_new(&state.join("server.yaml"), b"overwrite").is_err());
        for name in ["control.toml", "server.yaml", "owner.json"] {
            fs::remove_file(state.join(name)).unwrap();
        }
        fs::remove_dir(state).unwrap();
    }

    #[test]
    fn rejects_a_different_endpoint_before_writing_credentials() {
        let state = test_state();
        let config = state.join("control.toml");
        fs::write(
            &config,
            "endpoint='http://elsewhere.invalid'\nowner_email='operator@email.gnx'",
        )
        .unwrap();
        assert!(run("render", &config, &state).is_err());
        assert!(!state.join("owner.json").exists());
        fs::remove_file(config).unwrap();
        fs::remove_dir(state).unwrap();
    }
}
