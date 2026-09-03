use std::{io::IsTerminal, net::Ipv4Addr, path::Path, thread, time::Duration};

use serde::Deserialize;
use zeroize::Zeroizing;

use crate::{
    Error, Result,
    config::{Access, Config},
};

const ACCESS_UNIT: &str = include_str!("../runtime/access/gnx-access.container");
const DNS_UNIT: &str = include_str!("../runtime/access/gnx-dns.container");
const DNS_CONFIG: &str = include_str!("../runtime/access/dns.toml");
const ENROLL: &str = include_str!("../runtime/access/enroll.sh");
const NETWORK_UNIT: &str = include_str!("../runtime/access/gnx-access-network.service");
const DNS_IMAGE: &str = "docker.io/pihole/pihole@sha256:7c96327ecfb96dbc74b0a47d073dbef7d60526e0aa87519b2a2f7a0007cb5c88";

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Status {
    backend_state: String,
    #[serde(rename = "Self")]
    node: Option<Node>,
}

#[derive(Deserialize)]
struct Node {
    #[serde(rename = "TailscaleIPs")]
    ips: Option<Vec<String>>,
}

pub struct DnsReport {
    pub fields: String,
    pub checks: Result<()>,
}

pub fn configure(config: &Config) -> Result<String> {
    if !std::io::stdin().is_terminal()
        || !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        return Err(Error::Operation("ACCESS_TERMINAL_REQUIRED"));
    }
    foundation(config)?;
    if status().and_then(identity).is_err() {
        let secret = Zeroizing::new(
            rpassword::prompt_password("Tailscale auth key (hidden; Enter cancels): ")
                .map_err(|_| Error::Operation("ACCESS_SECRET_INPUT"))?,
        );
        let key = enrollment(&secret)?;
        let hostname = format!("--hostname={}", config.access.hostname);
        let tag = format!("--advertise-tags={}", config.access.tag);
        crate::platform::run(
            &[
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
                "up",
                "--reset",
                "--accept-dns=false",
                "--accept-routes=false",
                "--ssh=false",
                "--timeout=45s",
                &hostname,
                &tag,
            ],
            Some(key.as_bytes()),
            "ACCESS_ENROLLMENT",
        )?;
    }
    finish(config)
}

pub fn apply(config: &Config) -> Result<String> {
    foundation(config)?;
    status()
        .and_then(identity)
        .map_err(|_| Error::Operation("ACCESS_ENROLLMENT_REQUIRED"))?;
    finish(config)
}

pub fn dns(config: &Config) -> Result<DnsReport> {
    crate::platform::root()?;
    let access_ip = status().and_then(identity)?;
    let services = match service_addresses(&config.access) {
        Ok(services) => services,
        Err(error) => {
            return Err(Error::AccessReport {
                operation: error.label(),
                fields: format!(
                    "Split DNS: {} -> {access_ip}\nTailscale nameserver: {access_ip}\nServices: pending approval",
                    config.access.zone
                ),
            });
        }
    };
    let fields = fields(config, access_ip, &services);
    let checks = dns_checks(config, access_ip);
    Ok(DnsReport { fields, checks })
}

fn foundation(config: &Config) -> Result<()> {
    crate::platform::root()?;
    crate::platform::private_dir(Path::new(&config.access.state_dir))?;
    let network = NETWORK_UNIT
        .replace("@UPLINK@", &config.access.uplink)
        .replace("@MTU@", &config.access.uplink_mtu.to_string());
    let access = ACCESS_UNIT.replace("@STATE@", &config.access.state_dir);
    crate::platform::install(
        Path::new("/etc/systemd/system/gnx-access-network.service"),
        &network,
        0o644,
    )?;
    crate::platform::install(
        Path::new("/etc/containers/systemd/gnx-access.container"),
        &access,
        0o644,
    )?;
    systemctl(&["daemon-reload"], "ACCESS_INSTALL")?;
    systemctl(
        &["enable", "--now", "gnx-access-network.service"],
        "ACCESS_UPLINK",
    )?;
    systemctl(&["enable", "--now", "gnx-access.service"], "ACCESS_SERVICE")
}

fn finish(config: &Config) -> Result<String> {
    let mut access_ip = status().and_then(identity);
    for _ in 0..20 {
        if access_ip.is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(500));
        access_ip = status().and_then(identity);
    }
    let access_ip = access_ip?;
    for service in &config.access.services {
        let identity = format!("svc:{}", service.name);
        client(&[
            "serve",
            "--yes",
            &format!("--service={identity}"),
            "--https=443",
            &service.target,
        ])?;
    }
    let services = match service_addresses(&config.access) {
        Ok(services) => services,
        Err(error) => {
            return Err(Error::AccessReport {
                operation: error.label(),
                fields: format!(
                    "Split DNS: {} -> {access_ip}\nTailscale nameserver: {access_ip}\nServices: pending approval",
                    config.access.zone
                ),
            });
        }
    };
    let records = records(config, access_ip);
    let dns = DNS_CONFIG.replace("@RECORDS@", &records);
    let unit = DNS_UNIT
        .replace("@STATE@", &config.access.state_dir)
        .replace("@IP@", &access_ip.to_string());
    crate::platform::install(
        &Path::new(&config.access.state_dir).join("dns.toml"),
        &dns,
        0o644,
    )?;
    crate::platform::install(
        Path::new("/etc/containers/systemd/gnx-dns.container"),
        &unit,
        0o644,
    )?;
    systemctl(&["daemon-reload"], "DNS_INSTALL")?;
    systemctl(&["enable", "--now", "gnx-dns.service"], "DNS_SERVICE")?;
    dns_checks(config, access_ip)?;
    Ok(format!("access\n{}", fields(config, access_ip, &services)))
}

fn client(args: &[&str]) -> Result<Vec<u8>> {
    let mut command = vec![
        "podman",
        "exec",
        "gnx-access",
        "/usr/local/bin/tailscale",
        "--socket=/run/gnx/access.sock",
    ];
    command.extend_from_slice(args);
    crate::platform::run(&command, None, "TAILSCALE")
}

fn status() -> Result<Status> {
    serde_json::from_slice(&client(&["status", "--json"])?)
        .map_err(|_| Error::Operation("ACCESS_STATUS"))
}

fn identity(status: Status) -> Result<Ipv4Addr> {
    if status.backend_state != "Running" {
        return Err(Error::Operation("ACCESS_NOT_CONNECTED"));
    }
    status
        .node
        .and_then(|node| node.ips)
        .unwrap_or_default()
        .iter()
        .filter_map(|value| value.parse().ok())
        .find(is_tailnet_ip)
        .ok_or(Error::Operation("ACCESS_ADDRESS"))
}

fn service_addresses(access: &Access) -> Result<Vec<Ipv4Addr>> {
    access
        .services
        .iter()
        .map(|service| {
            let answer = client(&["dns", "query", &service.fqdn])?;
            String::from_utf8_lossy(&answer)
                .split_whitespace()
                .filter_map(|value| {
                    value
                        .trim_matches(|character: char| {
                            !character.is_ascii_digit() && character != '.'
                        })
                        .parse()
                        .ok()
                })
                .find(is_tailnet_ip)
                .ok_or(Error::Operation("TAILSCALE_SERVICE_APPROVAL"))
        })
        .collect()
}

fn is_tailnet_ip(ip: &Ipv4Addr) -> bool {
    let bytes = ip.octets();
    bytes[0] == 100 && (64..=127).contains(&bytes[1])
}

fn records(config: &Config, access_ip: Ipv4Addr) -> String {
    let mut records = config
        .access
        .services
        .iter()
        .map(|service| format!("\"address=/{}/{access_ip}\"", service.alias))
        .collect::<Vec<_>>();
    if config.controller.autonomous_ca {
        records.push(format!("\"address=/pki.gnx/{access_ip}\""));
    }
    records.push("\"local=/gnx/\"".into());
    records.join(", ")
}

fn fields(config: &Config, access_ip: Ipv4Addr, services: &[Ipv4Addr]) -> String {
    let aliases = config
        .access
        .services
        .iter()
        .zip(services)
        .map(|(service, ip)| {
            format!(
                "{} -> {access_ip}\n{} -> {ip} (Tailscale Service)",
                service.alias, service.fqdn
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Split DNS: {} -> {access_ip}\nTailscale nameserver: {access_ip}\n{aliases}",
        config.access.zone
    )
}

fn dns_checks(config: &Config, access_ip: Ipv4Addr) -> Result<()> {
    systemctl(&["is-active", "--quiet", "gnx-dns.service"], "DNS_SERVICE")?;
    for service in &config.access.services {
        for transport in ["+notcp", "+tcp"] {
            let server = format!("@{access_ip}");
            let answer = crate::platform::run(
                &[
                    "podman",
                    "run",
                    "--rm",
                    "--pull=never",
                    "--network=host",
                    "--log-driver=none",
                    "--entrypoint=dig",
                    DNS_IMAGE,
                    &server,
                    &service.alias,
                    "A",
                    "+short",
                    "+time=2",
                    "+tries=1",
                    transport,
                ],
                None,
                "DNS_QUERY",
            )?;
            if String::from_utf8_lossy(&answer).trim() != access_ip.to_string() {
                return Err(Error::Operation("DNS_ANSWER"));
            }
        }
        let url = format!("https://{}/", service.fqdn);
        crate::platform::run(
            &[
                "curl",
                "--fail",
                "--silent",
                "--max-time",
                "15",
                "--output",
                "/dev/null",
                &url,
            ],
            None,
            "TAILSCALE_SERVICE_TLS",
        )?;
    }
    if config.controller.autonomous_ca {
        let server = format!("@{access_ip}");
        let answer = crate::platform::run(
            &[
                "podman",
                "run",
                "--rm",
                "--pull=never",
                "--network=host",
                "--log-driver=none",
                "--entrypoint=dig",
                DNS_IMAGE,
                &server,
                "pki.gnx",
                "A",
                "+short",
                "+time=2",
                "+tries=1",
            ],
            None,
            "DNS_QUERY",
        )?;
        if String::from_utf8_lossy(&answer).trim() != access_ip.to_string() {
            return Err(Error::Operation("DNS_ANSWER"));
        }
    }
    Ok(())
}

fn systemctl(args: &[&str], operation: &'static str) -> Result<()> {
    let mut command = vec!["systemctl"];
    command.extend_from_slice(args);
    crate::platform::run(&command, None, operation).map(|_| ())
}

fn enrollment(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.starts_with("tskey-auth-")
        && (24..=512).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        Ok(value)
    } else {
        Err(Error::Operation("ACCESS_KEY_FORMAT"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_tailnet_ipv4_addresses() {
        assert!(is_tailnet_ip(&"100.64.0.1".parse().unwrap()));
        assert!(is_tailnet_ip(&"100.127.255.254".parse().unwrap()));
        assert!(!is_tailnet_ip(&"192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn enrollment_never_accepts_a_shell_expression() {
        assert!(enrollment("tskey-auth-GNX_TEST_1234567890").is_ok());
        assert!(enrollment("tskey-auth-$(forbidden)").is_err());
    }

    #[test]
    fn private_names_resolve_to_the_controller_not_the_service_vip() {
        let config: Config = toml::from_str(include_str!("../config/gnx.example.toml")).unwrap();
        let access_ip = "100.64.0.1".parse().unwrap();
        let rendered = records(&config, access_ip);
        assert!(rendered.contains("address=/compute.gnx/100.64.0.1"));
        assert!(rendered.contains("address=/pki.gnx/100.64.0.1"));
    }
}
