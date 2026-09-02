use std::{
    fs,
    io::Write,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, &'static str>;
const ACCESS: &str = include_str!("../../../runtime/access/gnx-access.container");
const DNS: &str = include_str!("../../../runtime/access/gnx-dns.container");
const RESOLVER: &str = include_str!("../../../runtime/access/dns.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    distribution: String,
    zone: String,
    wildcard: bool,
    state_dir: String,
}

fn config(source: &str) -> Result<Config> {
    let c: Config = toml::from_str(source).map_err(|_| "CONFIG")?;
    if c.distribution != "Ubuntu-24.04"
        || c.zone != "mesh.gnx"
        || !c.wildcard
        || c.state_dir != "/var/lib/gnx/access"
    {
        return Err("CONFIG_SCOPE");
    }
    Ok(c)
}

fn arguments(args: &[String]) -> Result<(PathBuf, Option<PathBuf>)> {
    if !matches!(args.len(), 3 | 5) || args[0] != "apply" || args[1] != "--config" {
        return Err("ARGUMENTS");
    }
    let key = if args.len() == 5 && args[3] == "--key-file" {
        Some(PathBuf::from(&args[4]))
    } else if args.len() == 3 {
        None
    } else {
        return Err("ARGUMENTS");
    };
    Ok((PathBuf::from(&args[2]), key))
}

// Never forward subprocess output: authentication can return credentials or URLs.
fn wsl(c: &Config, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut child = Command::new("wsl.exe")
        .args(["-d", &c.distribution, "--user", "root", "--exec"])
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "WSL_START")?;
    if let Some(bytes) = input {
        let result = child.stdin.take().ok_or("WSL_INPUT")?.write_all(bytes);
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return Err("WSL_INPUT");
        }
    }
    let output = child.wait_with_output().map_err(|_| "WSL_WAIT")?;
    if !output.status.success() {
        return Err("WSL_COMMAND");
    }
    Ok(output.stdout)
}

fn render(template: &str, c: &Config, ip: Ipv4Addr) -> String {
    template
        .replace("@STATE@", &c.state_dir)
        .replace("@ZONE@", &c.zone)
        .replace("@IP@", &ip.to_string())
}

fn read(c: &Config, path: &str) -> Result<Vec<u8>> {
    wsl(
        c,
        &[
            "sh",
            "-ec",
            "if test -f \"$1\"; then cat -- \"$1\"; fi",
            "gnx",
            path,
        ],
        None,
    )
}

fn write(c: &Config, path: &str, content: &[u8], mode: &str) -> Result<bool> {
    let previous = read(c, path)?;
    if previous == content {
        return Ok(false);
    }
    // Only our managed unit names may be replaced. Never overwrite foreign units.
    if path.starts_with("/etc/containers/")
        && !previous.is_empty()
        && !previous.starts_with(b"# Managed by GNX access")
    {
        return Err("UNIT_OWNERSHIP");
    }
    wsl(
        c,
        &[
            "sh",
            "-ec",
            "umask 077; cat > \"$1.new\"; chmod \"$2\" \"$1.new\"; mv -- \"$1.new\" \"$1\"",
            "gnx",
            path,
            mode,
        ],
        Some(content),
    )?;
    Ok(true)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Status {
    backend_state: String,
    #[serde(rename = "Self")]
    node: Option<Node>,
}

#[derive(Deserialize)]
struct Node {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "TailscaleIPs")]
    ips: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct Identity {
    id: String,
    ipv4: Ipv4Addr,
}

fn identity(status: Status) -> Result<Identity> {
    if status.backend_state != "Running" {
        return Err("ACCESS_NOT_CONNECTED");
    }
    let node = status.node.ok_or("ACCESS_IDENTITY")?;
    if node.id.is_empty() || !node.id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("ACCESS_IDENTITY");
    }
    let ip = node
        .ips
        .unwrap_or_default()
        .iter()
        .filter_map(|s| s.parse::<Ipv4Addr>().ok())
        .find(|ip| ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        .ok_or("ACCESS_ADDRESS")?;
    Ok(Identity {
        id: node.id,
        ipv4: ip,
    })
}

fn client(c: &Config, args: &[&str]) -> Result<Vec<u8>> {
    let mut command = vec![
        "podman",
        "exec",
        "gnx-access",
        "/usr/local/bin/tailscale",
        "--socket=/run/gnx/access.sock",
    ];
    command.extend_from_slice(args);
    wsl(c, &command, None)
}

fn status(c: &Config) -> Result<Status> {
    serde_json::from_slice(&client(c, &["status", "--json"])?).map_err(|_| "ACCESS_STATUS")
}

fn enrollment(path: &Path) -> Result<String> {
    if fs::metadata(path).map_err(|_| "KEY_FILE")?.len() > 512 {
        return Err("KEY_FORMAT");
    }
    let text = fs::read_to_string(path).map_err(|_| "KEY_FILE")?;
    let key = text.trim_start_matches('\u{feff}').trim();
    if !key.starts_with("tskey-auth-")
        || key.len() < 24
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("KEY_FORMAT");
    }
    Ok(key.to_owned())
}

fn enroll(c: &Config, key: Option<&str>) -> Result<()> {
    let path = format!("{}/bootstrap/enrollment.key", c.state_dir);
    let mut args = vec![
        "up",
        "--reset",
        "--accept-dns=false",
        "--accept-routes=false",
        "--hostname=gnx-access",
        "--ssh=false",
        "--timeout=45s",
    ];
    if let Some(key) = key {
        write(c, &path, key.as_bytes(), "600")?;
        args.push("--auth-key=file:/run/gnx/bootstrap/enrollment.key");
    }
    let result = client(c, &args);
    // Cleanup is attempted even when the provider rejects the key.
    wsl(c, &["rm", "-f", "--", &path], None).map_err(|_| "KEY_CLEANUP")?;
    result.map(|_| ()).map_err(|_| "ACCESS_ENROLLMENT")
}

fn service(c: &Config, name: &str, changed: bool) -> Result<()> {
    wsl(
        c,
        &["systemctl", if changed { "restart" } else { "start" }, name],
        None,
    )?;
    wsl(c, &["systemctl", "is-active", "--quiet", name], None)?;
    Ok(())
}

fn verify(c: &Config, ip: Ipv4Addr) -> Result<()> {
    let address = format!("@{ip}");
    for transport in ["+notcp", "+tcp"] {
        for name in ["mesh.gnx", "proxmox.mesh.gnx", "wildcard-check.mesh.gnx"] {
            let answer = wsl(
                c,
                &[
                    "podman", "exec", "gnx-dns", "dig", &address, name, "A", "+short", "+time=2",
                    "+tries=1", transport,
                ],
                None,
            )?;
            if String::from_utf8_lossy(&answer).trim() != ip.to_string() {
                return Err("DNS_ANSWER");
            }
        }
    }
    for name in ["mesh.gnx", "proxmox.mesh.gnx"] {
        let mapping = format!("{name}:443:{ip}");
        let url = format!("https://{name}/");
        let code = wsl(
            c,
            &[
                "curl",
                "--silent",
                "--show-error",
                "--noproxy",
                "*",
                "--max-time",
                "15",
                "--cacert",
                "/var/lib/gnx/control/tls/root.crt",
                "--resolve",
                &mapping,
                "--output",
                "/dev/null",
                "--write-out",
                "%{http_code}",
                &url,
            ],
            None,
        )?;
        if code != b"200" {
            return Err("HTTPS_ENDPOINT");
        }
    }
    Ok(())
}

fn apply(path: &Path, key_path: Option<&Path>) -> Result<()> {
    let c = config(&fs::read_to_string(path).map_err(|_| "CONFIG_FILE")?)?;
    wsl(&c, &["test", "-c", "/dev/net/tun"], None).map_err(|_| "TUN_REQUIRED")?;
    for unit in [
        "gnx-entry.service",
        "gnx-control.service",
        "gnx-compute.service",
    ] {
        wsl(&c, &["systemctl", "is-active", "--quiet", unit], None)
            .map_err(|_| "INFRASTRUCTURE_REQUIRED")?;
    }
    // No shell substitution of configuration or credentials; arguments stay separate.
    for suffix in ["", "/state", "/bootstrap"] {
        wsl(
            &c,
            &[
                "install",
                "-d",
                "-m",
                "700",
                &format!("{}{suffix}", c.state_dir),
            ],
            None,
        )?;
    }
    let key = key_path.map(enrollment).transpose()?;
    let unit = render(ACCESS, &c, Ipv4Addr::UNSPECIFIED);
    let changed = write(
        &c,
        "/etc/containers/systemd/gnx-access.container",
        unit.as_bytes(),
        "644",
    )?;
    wsl(&c, &["systemctl", "daemon-reload"], None)?;
    service(&c, "gnx-access.service", changed).map_err(|_| "ACCESS_SERVICE")?;
    let mut observed = status(&c);
    for _ in 0..20 {
        if observed.is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(500));
        observed = status(&c);
    }
    let observed = observed?;
    if observed.backend_state != "Running" && observed.backend_state != "Stopped" && key.is_none() {
        return Err("ACCESS_ENROLLMENT_REQUIRED");
    }
    enroll(
        &c,
        if observed.backend_state == "Running" {
            None
        } else {
            key.as_deref()
        },
    )?;
    let current = identity(status(&c)?)?;
    let identity_path = format!("{}/identity.json", c.state_dir);
    let previous = read(&c, &identity_path)?;
    if !previous.is_empty() {
        let previous: Identity = serde_json::from_slice(&previous).map_err(|_| "SAVED_IDENTITY")?;
        if previous != current {
            return Err("ACCESS_IDENTITY_CHANGED");
        }
    }
    write(
        &c,
        &identity_path,
        &serde_json::to_vec(&current).map_err(|_| "SAVED_IDENTITY")?,
        "600",
    )?;
    let dns = render(RESOLVER, &c, current.ipv4);
    let changed_config = write(
        &c,
        &format!("{}/dns.toml", c.state_dir),
        dns.as_bytes(),
        "644",
    )?;
    let unit = render(DNS, &c, current.ipv4);
    let changed_unit = write(
        &c,
        "/etc/containers/systemd/gnx-dns.container",
        unit.as_bytes(),
        "644",
    )?;
    wsl(&c, &["systemctl", "daemon-reload"], None)?;
    service(&c, "gnx-dns.service", changed_config || changed_unit).map_err(|_| "DNS_SERVICE")?;
    let mut checks = verify(&c, current.ipv4);
    for _ in 0..5 {
        if checks.is_ok() {
            break;
        }
        thread::sleep(Duration::from_secs(1));
        checks = verify(&c, current.ipv4);
    }
    checks?;
    println!(
        "READY access-local ip={} zone={}\nPENDING saas-dns-policy android-cellular reboot backup",
        current.ipv4, c.zone
    );
    Ok(())
}

fn main() {
    let result = arguments(&std::env::args().skip(1).collect::<Vec<_>>())
        .and_then(|(config, key)| apply(&config, key.as_deref()));
    if let Err(gate) = result {
        eprintln!("FAILED {gate}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const CONFIG: &str = include_str!("../../../runtime/access/access.toml");

    #[test]
    fn bounds_configuration_before_host_changes() {
        assert!(config(CONFIG).is_ok());
        for invalid in [
            CONFIG.replace("mesh.gnx", "gnx"),
            CONFIG.replace("true", "false"),
            CONFIG.replace("/var/lib/gnx/access", "/"),
            format!("{CONFIG}\nsecret = 'forbidden'"),
            CONFIG.replace("Ubuntu-24.04", "another-host"),
        ] {
            assert!(config(&invalid).is_err());
        }
    }

    #[test]
    fn accepts_only_key_file_arguments() {
        let parse = |a: &[&str]| arguments(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(parse(&["apply", "--config", "access.toml"]).is_ok());
        assert!(
            parse(&[
                "apply",
                "--config",
                "access.toml",
                "--key-file",
                "enrollment.key"
            ])
            .is_ok()
        );
        for args in [
            vec![],
            vec!["apply"],
            vec!["apply", "--config", "a", "--key", "value"],
            vec!["render", "--config", "a"],
        ] {
            assert!(parse(&args).is_err());
        }
    }

    #[test]
    fn renders_only_private_dns_without_global_forwarding() {
        let c = config(CONFIG).unwrap();
        let ip = "100.100.100.10".parse().unwrap();
        let dns = render(RESOLVER, &c, ip);
        let parsed: toml::Value = toml::from_str(&dns).unwrap();
        assert!(parsed["dns"]["upstreams"].as_array().unwrap().is_empty());
        assert_eq!(parsed["webserver"]["port"].as_str(), Some(""));
        assert!(dns.contains("address=/mesh.gnx/100.100.100.10"));
        assert!(dns.contains("local=/mesh.gnx/"));
        for template in [ACCESS, DNS, RESOLVER] {
            for marker in ["@STATE@", "@ZONE@", "@IP@"] {
                assert!(!render(template, &c, ip).contains(marker));
            }
        }
        let unit = render(DNS, &c, ip);
        assert!(unit.contains("PublishPort=100.100.100.10:53:53/udp"));
        assert!(unit.contains("PublishPort=100.100.100.10:53:53/tcp"));
        assert!(!unit.contains("8006"));
    }

    #[test]
    fn requires_a_running_stable_overlay_identity() {
        let parse = |state: &str, ip: &str| {
            identity(Status {
                backend_state: state.into(),
                node: Some(Node {
                    id: "n123".into(),
                    ips: Some(vec![ip.into()]),
                }),
            })
        };
        assert!(parse("Running", "100.68.68.10").is_ok());
        for (state, ip) in [
            ("NeedsLogin", "100.68.68.10"),
            ("Stopped", "100.68.68.10"),
            ("Running", "192.168.1.1"),
            ("Running", "100.128.0.1"),
        ] {
            assert!(parse(state, ip).is_err());
        }
    }

    #[test]
    fn unenrolled_status_allows_null_addresses_without_exposing_login_data() {
        let status: Status = serde_json::from_str(r#"{"BackendState":"NeedsLogin","Self":{"ID":"","TailscaleIPs":null},"AuthURL":"not-retained"}"#).unwrap();
        assert_eq!(status.backend_state, "NeedsLogin");
        assert!(identity(status).is_err());
    }
}
