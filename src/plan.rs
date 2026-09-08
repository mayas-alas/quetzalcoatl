use crate::{Result, config::Config, release, report::Failure};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct Artifact {
    pub path: PathBuf,
    pub content: String,
    pub capability: String,
}
#[derive(Debug, Serialize)]
pub struct Change {
    pub path: PathBuf,
    pub operation: &'static str,
}

pub fn render(config: &Config, scope: &str) -> Result<Vec<Artifact>> {
    config.validate()?;
    let mut result = Vec::new();
    let state = &config.state_dir;
    let unit =
        |name: &str| Path::new(&config.units_dir).join(format!("{}.{name}", config.instance));
    // Keep the major/minor ownership marker stable across 0.3.x patch releases.
    let marker = format!("# Managed by GNX 0.3: {}\n", config.instance);
    let mut add = |path: PathBuf, content: String, capability: &str| {
        result.push(Artifact {
            path,
            content: marker.clone() + &content,
            capability: capability.into(),
        });
    };
    let network = config.unit("backplane");
    add(
        unit("network"),
        format!(
            "[Network]\nNetworkName={network}\nSubnet={}\n",
            config.network.subnet
        ),
        "network",
    );
    let network_ref = format!("{}.network", config.instance);
    let mount = |name: &str| format!("RequiresMountsFor={state}/{name}\n");
    let lifecycle = "[Service]\nRestart=on-failure\nRestartSec=5\nTimeoutStartSec=900\nTimeoutStopSec=150\n\n[Install]\nWantedBy=multi-user.target\n";
    if scope == "all" || scope == "compute" {
        add(
            Path::new(state).join("runtime/compute-entry.sh"),
            include_str!("../runtime/compute-entry.sh").into(),
            "compute",
        );
        let name = config.unit("compute");
        add(
            Path::new(&config.units_dir).join(format!("{name}.container")),
            format!(
                "[Unit]\nDescription=GNX persistent compute\n{}StartLimitIntervalSec=120\nStartLimitBurst=5\n\n[Container]\nImage={}\nContainerName={name}\nHostName={}\nNetwork={network_ref}\nIP={}\nPodmanArgs=--privileged --systemd=always --stop-timeout=120 --entrypoint=/bin/sh\nExec=/run/gnx/entry.sh /sbin/init --log-target=console --log-level=warning\nVolume={state}/runtime/compute-entry.sh:/run/gnx/entry.sh:ro\nVolume={state}/compute/private/password:/run/gnx/password:ro\nVolume={state}/compute/config:/var/lib/pve-cluster\nVolume={state}/compute/storage:/var/lib/vz\nShmSize=1g\nPidsLimit=2048\n\n{lifecycle}",
                mount("compute"),
                release::COMPUTE_IMAGE,
                config.node,
                config.network.compute_ip
            ),
            "compute",
        );
    }
    if scope == "all" || scope == "access" {
        let name = config.unit("access");
        add(
            Path::new(&config.units_dir).join(format!("{name}.container")),
            format!(
                "[Unit]\nDescription=GNX private access\n{}StartLimitIntervalSec=120\nStartLimitBurst=5\n\n[Container]\nImage={}\nContainerName={name}\nNetwork={network_ref}\nIP={}\nAddDevice=/dev/net/tun\nAddCapability=NET_ADMIN NET_RAW\nPodmanArgs=--entrypoint=/usr/local/bin/tailscaled\nExec=--state=/var/lib/tailscale/node.state --socket=/run/tailscale/tailscaled.sock --tun=tailscale0\nVolume={state}/access/state:/var/lib/tailscale\n\n{lifecycle}",
                mount("access"),
                release::ACCESS_IMAGE,
                config.network.access_ip
            ),
            "access",
        );
    }
    if let Some(ip) = config.network.identity_ip {
        if scope == "all" || scope == "access" {
            // Content-derived serial: reuse the current serial on an identical plan;
            // otherwise strictly increment, including when reverting to older records.
            let records = format!(
                "@ IN NS ns.gnx.\nns IN A {ip}\ncompute IN A {ip}\n{}",
                config
                    .routes
                    .iter()
                    .map(|r| format!("{} IN A {ip}\n", r.hostname.trim_end_matches(".gnx")))
                    .collect::<String>()
            );
            let zone_path = Path::new(state).join("access/dns/gnx.db");
            let previous = std::fs::read_to_string(&zone_path).unwrap_or_default();
            let old_serial = previous
                .lines()
                .find_map(|line| line.strip_prefix("@ IN SOA ns.gnx. hostmaster.gnx. "))
                .and_then(|line| line.split_whitespace().next())
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0);
            let serial = if previous.ends_with(&records) {
                old_serial.max(1)
            } else {
                old_serial
                    .checked_add(1)
                    .ok_or_else(|| Failure::new("DNS_SERIAL", "SOA serial exhausted"))?
            };
            add(
                zone_path,
                format!(
                    "$ORIGIN gnx.\n$TTL 60\n@ IN SOA ns.gnx. hostmaster.gnx. {serial} 60 30 86400 60\n{records}"
                ),
                "dns",
            );
            add(
                Path::new(state).join("access/dns/Corefile"),
                format!(
                    "gnx:53 {{\n bind {ip}\n errors\n file /etc/coredns/gnx.db gnx {{\n  reload 5s\n }}\n}}\n.:53 {{\n bind {ip}\n template ANY ANY {{\n  rcode REFUSED\n }}\n}}\n"
                ),
                "dns",
            );
            let access = config.unit("access");
            let name = config.unit("dns");
            add(
                Path::new(&config.units_dir).join(format!("{name}.container")),
                format!(
                    "[Unit]\nDescription=GNX authoritative DNS\nAfter={access}.service\nBindsTo={access}.service\nPartOf={access}.service\n\n[Container]\nImage={}\nContainerName={name}\nNetwork={access}.container\nVolume={state}/access/dns:/etc/coredns:ro\nExec=-conf /etc/coredns/Corefile\n\n{lifecycle}",
                    release::DNS_IMAGE
                ),
                "dns",
            );
        }
        if scope == "all" || scope == "control" {
            let mut caddy = format!(
                "{{\n admin 127.0.0.1:2019\n skip_install_trust\n auto_https disable_redirects\n servers {{\n  protocols h1 h2\n }}\n}}\nhttps://compute.gnx {{\n bind {ip}\n tls internal\n reverse_proxy https://{}:8006 {{\n  transport http {{\n   tls_server_name {}\n   tls_trust_pool file /etc/gnx/upstream-ca.crt\n  }}\n }}\n}}\n",
                config.network.compute_ip, config.node
            );
            for route in &config.routes {
                caddy.push_str(&format!(
                    "https://{} {{\n bind {ip}\n tls internal\n reverse_proxy {}\n}}\n",
                    route.hostname, route.upstream
                ));
            }
            // An explicit rejection also covers an unknown Host over a known TLS SNI.
            caddy.push_str(&format!("https://:443 {{\n bind {ip}\n respond 404\n}}\n"));
            add(Path::new(state).join("control/Caddyfile"), caddy, "control");
            let name = config.unit("control");
            let access = config.unit("access");
            add(
                Path::new(&config.units_dir).join(format!("{name}.container")),
                format!(
                    "[Unit]\nDescription=GNX HTTPS control\nAfter={access}.service\nBindsTo={access}.service\nPartOf={access}.service\n\n[Container]\nImage={}\nContainerName={name}\nNetwork={access}.container\nVolume={state}/control/Caddyfile:/etc/caddy/Caddyfile:ro\nVolume={state}/compute/public/upstream-ca.crt:/etc/gnx/upstream-ca.crt:ro\nVolume={state}/control/data:/data\nVolume={state}/control/config:/config\n\n{lifecycle}",
                    release::CONTROL_IMAGE
                ),
                "control",
            );
        }
    } else if scope == "control" {
        return Err(Failure::action(
            "ACCESS_IP_REQUIRED",
            "Enroll Access and provide network.identity_ip before publishing Control",
        ));
    }
    Ok(result)
}

pub fn changes(config: &Config, artifacts: &[Artifact]) -> Result<Vec<Change>> {
    let marker = format!("# Managed by GNX 0.3: {}\n", config.instance);
    let mut changes = Vec::new();
    for artifact in artifacts {
        match std::fs::symlink_metadata(&artifact.path) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
                return Err(Failure::new(
                    "FILE_OWNERSHIP",
                    format!("Not a regular managed file: {}", artifact.path.display()),
                ));
            }
            Ok(_) => {
                let old = std::fs::read_to_string(&artifact.path)?;
                if !old.starts_with(&marker) {
                    return Err(Failure::new(
                        "FILE_OWNERSHIP",
                        format!("Preserving existing file: {}", artifact.path.display()),
                    ));
                }
                if old != artifact.content {
                    changes.push(Change {
                        path: artifact.path.clone(),
                        operation: "update",
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => changes.push(Change {
                path: artifact.path.clone(),
                operation: "create",
            }),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(changes)
}
