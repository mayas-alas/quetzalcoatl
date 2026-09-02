// Shared operational core for the GNX CLI and standalone host helper.
use std::{
    fs,
    io::{IsTerminal, Write},
    net::Ipv4Addr,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub type Result<T> = std::result::Result<T, &'static str>;
const ACCESS: &str = include_str!("../../../runtime/access/gnx-access.container");
const DNS: &str = include_str!("../../../runtime/access/gnx-dns.container");
const RESOLVER: &str = include_str!("../../../runtime/access/dns.toml");
const ENROLL: &str = include_str!("../../../runtime/access/enroll.sh");
const DNS_PROBE: &str = include_str!("../../../runtime/access/probe-dns.sh");
const NETWORK: &str = include_str!("../../../runtime/access/gnx-access-network.service");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    distribution: String,
    zone: String,
    wildcard: bool,
    state_dir: String,
    uplink: String,
    uplink_mtu: u16,
}

fn config(source: &str) -> Result<Config> {
    let c: Config = toml::from_str(source).map_err(|_| "CONFIG")?;
    if c.distribution != "Ubuntu-24.04"
        || c.zone != "mesh.gnx"
        || !c.wildcard
        || c.state_dir != "/var/lib/gnx/access"
        || c.uplink != "eth0"
        || c.uplink_mtu != 1500
    {
        return Err("CONFIG_SCOPE");
    }
    Ok(c)
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
        .replace("@UPLINK@", &c.uplink)
        .replace("@MTU@", &c.uplink_mtu.to_string())
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
    if (path.starts_with("/etc/containers/") || path.starts_with("/etc/systemd/system/"))
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

fn enrollment(text: &str) -> Result<&str> {
    let key = text.trim_start_matches('\u{feff}').trim();
    if !key.starts_with("tskey-auth-")
        || !(24..=512).contains(&key.len())
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("KEY_FORMAT");
    }
    Ok(key)
}

fn enroll(c: &Config, key: Option<&str>) -> Result<()> {
    let args = [
        "up",
        "--reset",
        "--accept-dns=false",
        "--accept-routes=false",
        "--hostname=gnx-access",
        "--ssh=false",
        "--timeout=45s",
    ];
    let result = if let Some(key) = key {
        let mut command = vec![
            "podman",
            "exec",
            "-i",
            "gnx-access",
            "sh",
            "-ec",
            ENROLL,
            "gnx",
            "/usr/local/bin/tailscale",
            "--socket=/run/gnx/access.sock",
        ];
        command.extend_from_slice(&args);
        wsl(c, &command, Some(key.as_bytes()))
    } else {
        client(c, &args)
    };
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

fn dns_probe_args(ip: &str) -> Vec<&str> {
    let image = DNS
        .lines()
        .find_map(|line| line.strip_prefix("Image="))
        .expect("pinned DNS image in compiled template");
    vec![
        "podman",
        "run",
        "--rm",
        "--pull=never",
        "--network=host",
        "--log-driver=none",
        "--entrypoint=/bin/sh",
        image,
        "-ec",
        DNS_PROBE,
        "gnx",
        ip,
        "53",
        ip,
    ]
}

fn verify(c: &Config, ip: Ipv4Addr) -> Result<()> {
    let links =
        wsl(c, &["ip", "-json", "link", "show", &c.uplink], None).map_err(|_| "UPLINK_READ")?;
    check_uplink(&links, c)?;
    // A bridge container cannot reliably reach its own published port (hairpin).
    wsl(c, &dns_probe_args(&ip.to_string()), None).map_err(|_| "DNS_QUERY")?;
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
        )
        .map_err(|_| "HTTPS_TRANSPORT")?;
        if code != b"200" {
            return Err("HTTPS_ENDPOINT");
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct Link {
    ifname: String,
    mtu: u16,
}

fn check_uplink(source: &[u8], c: &Config) -> Result<()> {
    let links: Vec<Link> = serde_json::from_slice(source).map_err(|_| "UPLINK_READ")?;
    match links.as_slice() {
        [link] if link.ifname == c.uplink && link.mtu == c.uplink_mtu => Ok(()),
        _ => Err("UPLINK_MTU"),
    }
}

#[derive(Deserialize)]
struct AccessPolicy {
    #[serde(rename = "Cached")]
    cached: bool,
    #[serde(rename = "PacketFilter")]
    matches: Option<Vec<serde::de::IgnoredAny>>,
}

fn check_policy(source: &[u8]) -> Result<()> {
    let policy: AccessPolicy = serde_json::from_slice(source).map_err(|_| "ACCESS_POLICY_READ")?;
    if policy.cached {
        return Err("ACCESS_POLICY_STALE");
    }
    match policy.matches {
        Some(matches) if matches.is_empty() => Err("ACCESS_POLICY_EMPTY"),
        Some(_) => Ok(()), // Nonempty is necessary, not proof of remote port access.
        None => Err("ACCESS_POLICY_READ"),
    }
}

fn verify_policy(c: &Config) -> Result<()> {
    let data = client(c, &["debug", "netmap"]).map_err(|_| "ACCESS_POLICY_READ")?;
    check_policy(&data)
}

pub fn apply(path: &Path) -> Result<String> {
    apply_with(path, || Err("ACCESS_ENROLLMENT_REQUIRED"))
}

pub fn configure(path: &Path) -> Result<String> {
    // Never read credentials from redirected input or a shell argument.
    config(&fs::read_to_string(path).map_err(|_| "CONFIG_FILE")?)?;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err("ACCESS_TERMINAL_REQUIRED");
    }
    apply_with(path, || {
        let secret = Zeroizing::new(
            rpassword::prompt_password("GNX enrollment key (hidden; Enter cancels): ")
                .map_err(|_| "ACCESS_SECRET_INPUT")?,
        );
        if secret.trim().is_empty() {
            return Err("ACCESS_CANCELLED");
        }
        enrollment(&secret)?;
        Ok(secret)
    })
}

fn apply_with(path: &Path, prompt: impl FnOnce() -> Result<Zeroizing<String>>) -> Result<String> {
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
    let network = render(NETWORK, &c, Ipv4Addr::UNSPECIFIED);
    let network_changed = write(
        &c,
        "/etc/systemd/system/gnx-access-network.service",
        network.as_bytes(),
        "644",
    )?;
    wsl(&c, &["systemctl", "daemon-reload"], None)?;
    wsl(
        &c,
        &["systemctl", "enable", "gnx-access-network.service"],
        None,
    )
    .map_err(|_| "UPLINK_ENABLE")?;
    // Reapply the oneshot if the host's MTU drifted, even with unchanged config.
    let links = wsl(&c, &["ip", "-json", "link", "show", &c.uplink], None)?;
    service(
        &c,
        "gnx-access-network.service",
        network_changed || check_uplink(&links, &c).is_err(),
    )
    .map_err(|_| "UPLINK_SERVICE")?;
    // No shell substitution of configuration or credentials; arguments stay separate.
    for suffix in ["", "/state"] {
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
        if observed.as_ref().is_ok_and(|state| {
            matches!(
                state.backend_state.as_str(),
                "Running" | "Stopped" | "NeedsLogin" | "NeedsMachineAuth"
            )
        }) {
            break;
        }
        thread::sleep(Duration::from_millis(500));
        observed = status(&c);
    }
    let observed = observed?;
    let key = match observed.backend_state.as_str() {
        "Running" | "Stopped" => None,
        "NeedsLogin" => Some(prompt()?),
        "NeedsMachineAuth" => return Err("ACCESS_DEVICE_APPROVAL_REQUIRED"),
        _ => return Err("ACCESS_NOT_CONNECTED"),
    };
    enroll(&c, key.as_ref().map(|key| enrollment(key)).transpose()?)?;
    drop(key);
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
    verify_policy(&c)?;
    Ok(format!(
        "access-local\n{}\nPENDING remote-client-check reboot backup",
        dns_fields(&c, Some(current.ipv4))
    ))
}

fn dns_fields(c: &Config, ip: Option<Ipv4Addr>) -> String {
    let address = ip
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "PENDING (gnx access configure)".into());
    format!(
        "Nameserver: {address}\nRestrict to domain (Split DNS): ON\nDomain: {}\nUse with exit node: OFF\nSearch domain (optional): {}\nUplink target: {} MTU {}",
        c.zone, c.zone, c.uplink, c.uplink_mtu
    )
}

pub struct DnsReport {
    pub fields: String,
    pub checks: Result<()>,
}

pub fn dns(path: &Path) -> Result<DnsReport> {
    let c = config(&fs::read_to_string(path).map_err(|_| "CONFIG_FILE")?)?;
    let current = status(&c).and_then(|status| {
        if status.backend_state == "NeedsLogin" {
            Err("ACCESS_ENROLLMENT_REQUIRED")
        } else {
            identity(status)
        }
    });
    let mut fields = dns_fields(&c, current.as_ref().ok().map(|node| node.ipv4));
    let mut checks = current.and_then(|current| {
        let saved = read(&c, &format!("{}/identity.json", c.state_dir))?;
        let saved: Identity = serde_json::from_slice(&saved).map_err(|_| "SAVED_IDENTITY")?;
        if saved != current {
            return Err("ACCESS_IDENTITY_CHANGED");
        }
        wsl(
            &c,
            &["systemctl", "is-active", "--quiet", "gnx-dns.service"],
            None,
        )
        .map_err(|_| "DNS_SERVICE")?;
        verify(&c, current.ipv4)
    });
    if checks.is_ok() {
        fields.push_str("\nLocal DNS/HTTPS: PASS");
        checks = verify_policy(&c);
    }
    Ok(DnsReport { fields, checks })
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
            CONFIG.replace("1500", "1280"),
            CONFIG.replace("eth0", "gnx-access"),
        ] {
            assert!(config(&invalid).is_err());
        }
    }

    #[test]
    fn validates_human_input_without_reflecting_it() {
        let example = "tskey-auth-example-not-a-real-key";
        assert_eq!(enrollment(&format!("\u{feff} {example}\r\n")), Ok(example));
        for input in [
            "",
            "wrong-kind",
            "tskey-auth-not valid",
            "tskey-api-example-not-a-real-key",
        ] {
            assert_eq!(enrollment(input), Err("KEY_FORMAT"));
        }
        assert_eq!(
            enrollment(&format!("tskey-auth-{}", "x".repeat(513))),
            Err("KEY_FORMAT")
        );
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
        for template in [ACCESS, DNS, RESOLVER, NETWORK] {
            for marker in ["@STATE@", "@ZONE@", "@IP@", "@UPLINK@", "@MTU@"] {
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

    #[test]
    fn dns_form_never_invents_a_nameserver() {
        let c = config(CONFIG).unwrap();
        let pending = dns_fields(&c, None);
        assert!(pending.contains("Nameserver: PENDING"));
        assert!(!pending.contains("100."));
        let ready = dns_fields(&c, Some("100.100.100.10".parse().unwrap()));
        for field in [
            "Nameserver: 100.100.100.10",
            "Restrict to domain (Split DNS): ON",
            "Domain: mesh.gnx",
            "Use with exit node: OFF",
        ] {
            assert!(ready.contains(field));
        }
        assert!(!ready.contains("*.mesh.gnx"));
    }

    #[test]
    fn dns_probe_uses_host_namespace_and_the_pinned_image() {
        let args = dns_probe_args("100.91.239.31");
        assert!(args.contains(&"--network=host"));
        assert!(args.contains(&"--pull=never"));
        assert!(!args.contains(&"exec"));
        assert!(args.iter().any(|arg| arg.contains("@sha256:")));
        assert_eq!(
            &args[args.len() - 3..],
            &["100.91.239.31", "53", "100.91.239.31"]
        );
    }

    #[test]
    fn uplink_is_persistent_and_drift_never_passes_as_ready() {
        let c = config(CONFIG).unwrap();
        assert!(check_uplink(br#"[{"ifname":"eth0","mtu":1500}]"#, &c).is_ok());
        for source in [
            br#"[{"ifname":"eth0","mtu":1280}]"#.as_slice(),
            br#"[{"ifname":"gnx-access","mtu":1500}]"#,
            b"[]",
        ] {
            assert_eq!(check_uplink(source, &c), Err("UPLINK_MTU"));
        }
        assert_eq!(check_uplink(b"invalid", &c), Err("UPLINK_READ"));
        let unit = render(NETWORK, &c, Ipv4Addr::UNSPECIFIED);
        assert!(unit.contains("ExecStart=/usr/sbin/ip link set dev eth0 mtu 1500"));
        assert!(unit.contains("WantedBy=multi-user.target"));
        assert!(ACCESS.contains("Requires=gnx-access-network.service"));
    }

    #[test]
    fn an_empty_or_unavailable_policy_never_passes_as_ready() {
        assert_eq!(
            check_policy(br#"{"Cached":false,"PacketFilter":[]}"#),
            Err("ACCESS_POLICY_EMPTY")
        );
        assert_eq!(
            check_policy(br#"{"Cached":true,"PacketFilter":[{}]}"#),
            Err("ACCESS_POLICY_STALE")
        );
        assert_eq!(
            check_policy(br#"{"Cached":false,"PacketFilter":null}"#),
            Err("ACCESS_POLICY_READ")
        );
        assert_eq!(check_policy(b"not-json"), Err("ACCESS_POLICY_READ"));
        assert!(
            check_policy(br#"{"Cached":false,"PacketFilter":[{}],"ignored":"not-retained"}"#)
                .is_ok()
        );
    }
}
