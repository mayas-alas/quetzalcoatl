use std::env;
use std::net::IpAddr;
use std::path::Path;
use std::thread;
use std::time::Duration;

use gnx_contracts::PveUrl;
use zeroize::Zeroize;

use crate::domain::errors::GateError;
use crate::domain::lifecycle::{Component, HostPeer, TailscaleIdentity, valid_discovered_hostname};
use crate::infrastructure::models::{TailscaleServeStatus, TailscaleStatus};
use crate::infrastructure::remote::{RuntimeOperation, bounded_text, machine_stdin, runtime_agent};
use crate::infrastructure::runtime_assets::{
    START_TAILSCALE, TAILSCALE_DIAGNOSTICS, TAILSCALE_SECRET_CLEANUP_PROBE,
};

pub(crate) fn candidate_hostname() -> Result<String, GateError> {
    let computer_name = env::var("COMPUTERNAME").map_err(|_| {
        GateError::new(
            "TAILSCALE_ENROLL_FAILED",
            Component::Tailscale,
            "COMPUTERNAME is unavailable in the service environment",
        )
    })?;
    let computer_name = computer_name.to_ascii_lowercase();
    if computer_name.is_empty()
        || computer_name.len() > 32
        || computer_name.starts_with('-')
        || computer_name.ends_with('-')
        || !computer_name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(GateError::new(
            "TAILSCALE_ENROLL_FAILED",
            Component::Tailscale,
            "Windows computer name cannot form a Tailscale hostname",
        ));
    }
    Ok(format!("gnx-candidate-{computer_name}"))
}

pub(crate) fn prepare_tailscale(
    podman: &Path,
    hostname: &str,
    auth_key: &str,
) -> Result<(), GateError> {
    let mut input = Vec::with_capacity(auth_key.len() + hostname.len() + 2);
    input.extend_from_slice(auth_key.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(hostname.as_bytes());
    input.push(b'\n');
    let result = runtime_agent(podman, RuntimeOperation::TailscalePrepare, &input);
    input.zeroize();
    result
        .map(|_| ())
        .map_err(|error| error.with_code("TAILSCALE_ENROLL_FAILED", Component::Tailscale))
}

pub(crate) fn start_tailscale(podman: &Path) -> Result<(), GateError> {
    let output = match machine_stdin(podman, ["sh", "-s"], START_TAILSCALE.as_bytes()) {
        Ok(output) => output,
        Err(error) => {
            verify_tailscale_secret_cleanup(podman)?;
            return Err(error.with_code("TAILSCALE_ENROLL_FAILED", Component::Tailscale));
        }
    };
    if String::from_utf8_lossy(&output.stdout).trim() != "TAILSCALE_SERVICE=active" {
        return Err(GateError::new(
            "TAILSCALE_ENROLL_FAILED",
            Component::Tailscale,
            "systemd did not confirm the permanent Tailscale sidecar",
        ));
    }
    Ok(())
}

pub(crate) fn disable_tailscale_ssh(podman: &Path) -> Result<(), GateError> {
    machine_stdin(
        podman,
        [
            "podman",
            "exec",
            "gnx-tailscaled",
            "tailscale",
            "set",
            "--ssh=false",
        ],
        &[],
    )
    .map(|_| ())
    .map_err(|error| error.with_code("TAILSCALE_SSH_DISABLE_FAILED", Component::Tailscale))
}

pub(crate) fn verify_tailscale_secret_cleanup(podman: &Path) -> Result<(), GateError> {
    let output = machine_stdin(
        podman,
        ["sh", "-s"],
        TAILSCALE_SECRET_CLEANUP_PROBE.as_bytes(),
    )
    .map_err(|error| error.with_code("TAILSCALE_SECRET_CLEANUP_FAILED", Component::Tailscale))?;
    if String::from_utf8_lossy(&output.stdout).trim() != "TAILSCALE_SECRET_CLEAN=ready" {
        return Err(GateError::new(
            "TAILSCALE_SECRET_CLEANUP_FAILED",
            Component::Tailscale,
            "Tailscale enrollment did not confirm secret cleanup",
        ));
    }
    Ok(())
}

pub(crate) fn wait_for_tailscale(
    podman: &Path,
    hostname: &str,
    tailnet: &str,
) -> Result<TailscaleIdentity, GateError> {
    let mut last_error = String::from("Tailscale sidecar is not ready");
    for attempt in 0..90 {
        match read_tailscale_status(podman, hostname, tailnet) {
            Ok(identity) => return Ok(identity),
            Err(error) => last_error = error,
        }
        if attempt + 1 < 90 {
            thread::sleep(Duration::from_secs(2));
        }
    }
    Err(GateError::new(
        "TAILSCALE_LOCAL_NOT_READY",
        Component::Tailscale,
        format!(
            "Local Tailscale identity did not become ready: {last_error}; {}",
            tailscale_diagnostics(podman)
        ),
    ))
}

pub(crate) fn stabilize_host_inventory(
    podman: &Path,
    initial: TailscaleIdentity,
    tailnet: &str,
) -> Result<TailscaleIdentity, GateError> {
    let hostname = initial.hostname.clone();
    let expected_self_id = initial.self_id.clone();
    let mut previous = Some(initial);
    let mut last_error = String::from("Tailscale host inventory has not stabilized");
    for attempt in 0..30 {
        thread::sleep(Duration::from_secs(2));
        match read_tailscale_status(podman, &hostname, tailnet) {
            Ok(current) => {
                if current.self_id != expected_self_id {
                    return Err(GateError::new(
                        "TAILSCALE_IDENTITY_CHANGED",
                        Component::Tailscale,
                        "Tailscale Self.ID changed during role discovery",
                    ));
                }
                if previous
                    .as_ref()
                    .is_some_and(|value| value.host_peers == current.host_peers)
                {
                    return Ok(current);
                }
                last_error = "consecutive tagged host inventories differ".into();
                previous = Some(current);
            }
            Err(error) => {
                last_error = error;
                previous = None;
            }
        }
        if attempt + 1 == 30 {
            break;
        }
    }
    Err(GateError::new(
        "TAILSCALE_DISCOVERY_UNSTABLE",
        Component::Tailscale,
        format!("Tailscale host inventory did not stabilize within 60 seconds: {last_error}"),
    ))
}

pub(crate) fn read_tailscale_status(
    podman: &Path,
    hostname: &str,
    tailnet: &str,
) -> Result<TailscaleIdentity, String> {
    let output = machine_stdin(
        podman,
        [
            "podman",
            "exec",
            "gnx-tailscaled",
            "tailscale",
            "status",
            "--json",
        ],
        &[],
    )
    .map_err(|error| error.message)?;
    parse_tailscale_status(&output.stdout, hostname, tailnet)
}

pub(crate) fn parse_tailscale_status(
    bytes: &[u8],
    hostname: &str,
    tailnet: &str,
) -> Result<TailscaleIdentity, String> {
    let status: TailscaleStatus = serde_json::from_slice(bytes)
        .map_err(|_| "tailscale status returned invalid JSON".to_string())?;
    let current_tailnet = status
        .current_tailnet
        .ok_or_else(|| "tailscale status has no current tailnet".to_string())?;
    let self_node = status
        .self_node
        .ok_or_else(|| "tailscale status has no self node".to_string())?;
    let domain = format!("{hostname}.{tailnet}");
    let expected_dns_name = format!("{domain}.");
    let cgnat_ipv4 = status
        .tailscale_ips
        .iter()
        .filter_map(|value| value.parse::<IpAddr>().ok())
        .filter(|address| match address {
            IpAddr::V4(address) => {
                let octets = address.octets();
                octets[0] == 100 && (64..=127).contains(&octets[1])
            }
            IpAddr::V6(_) => false,
        })
        .collect::<Vec<_>>();
    if status.backend_state != "Running"
        || !status.health.is_empty()
        || !status.tun
        || current_tailnet.magic_dns_suffix != tailnet
        || !current_tailnet.magic_dns_enabled
        || self_node.host_name != hostname
        || self_node.dns_name != expected_dns_name
        || !self_node
            .tags
            .iter()
            .any(|tag| tag == "tag:quetzalcoatl-node")
        || !status.cert_domains.iter().any(|value| value == &domain)
        || cgnat_ipv4.len() != 1
        || self_node.id.is_empty()
        || !self_node.id.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err("tailscale status does not match tailnet, hostname, stable ID, tag, TUN, IP and HTTPS requirements".into());
    }

    let mut host_peers = Vec::new();
    for peer in status.peers.values() {
        if peer.expired
            || peer.id == self_node.id
            || !peer.tags.iter().any(|tag| tag == "tag:quetzalcoatl-node")
            || !peer.host_name.starts_with("gnx-controller-")
        {
            continue;
        }
        let peer_ips = peer
            .tailscale_ips
            .iter()
            .filter_map(|value| value.parse::<IpAddr>().ok())
            .filter(|address| matches!(address, IpAddr::V4(address) if address.octets()[0] == 100 && (64..=127).contains(&address.octets()[1])))
            .collect::<Vec<_>>();
        if peer.id.is_empty()
            || !peer.id.bytes().all(|byte| byte.is_ascii_graphic())
            || peer_ips.len() != 1
            || !valid_discovered_hostname(&peer.host_name)
            || !peer.online
        {
            // Discovery is intentionally controller-only and tolerant. A stale,
            // malformed or offline tagged peer must not block a healthy node.
            continue;
        }
        host_peers.push(HostPeer {
            id: peer.id.clone(),
            hostname: peer.host_name.clone(),
            ip: peer_ips[0],
            online: peer.online,
            // Tailscale reports the peer's DERP region even when CurAddr is the
            // active direct endpoint. Its own status renderer treats a
            // non-empty CurAddr as direct and consults Relay only otherwise.
            direct: !peer.cur_addr.is_empty(),
        });
    }
    host_peers.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(TailscaleIdentity {
        self_id: self_node.id,
        self_ip: cgnat_ipv4[0],
        hostname: self_node.host_name,
        host_peers,
    })
}

pub(crate) fn pve_https_url(hostname: &str, tailnet: &str) -> Result<String, GateError> {
    PveUrl::parse(format!("https://{hostname}.{tailnet}/"))
        .map(|url| url.to_string())
        .map_err(|message| GateError::new("PVE_URL_INVALID", Component::TailscaleServe, message))
}

pub(crate) fn tailscale_serve_config(hostname: &str, tailnet: &str) -> Result<Vec<u8>, GateError> {
    let host_port = format!("{hostname}.{tailnet}:443");

    let mut web = serde_json::Map::new();
    web.insert(
        host_port.clone(),
        serde_json::json!({
            "Handlers": {
                "/": {
                    "Proxy": "https+insecure://127.0.0.1:8006"
                }
            }
        }),
    );

    let mut allow_funnel = serde_json::Map::new();
    allow_funnel.insert(host_port, serde_json::Value::Bool(false));

    let mut root = serde_json::Map::new();
    root.insert(
        "TCP".into(),
        serde_json::json!({
            "443": {
                "HTTPS": true
            }
        }),
    );
    root.insert("Web".into(), serde_json::Value::Object(web));
    root.insert(
        "AllowFunnel".into(),
        serde_json::Value::Object(allow_funnel),
    );

    serde_json::to_vec(&serde_json::Value::Object(root)).map_err(|error| {
        GateError::new(
            "TAILSCALE_SERVE_CONFIG_INVALID",
            Component::TailscaleServe,
            format!("cannot serialize the fixed Tailscale Serve config: {error}"),
        )
    })
}

pub(crate) fn apply_tailscale_serve(
    podman: &Path,
    hostname: &str,
    tailnet: &str,
) -> Result<(), GateError> {
    let config = tailscale_serve_config(hostname, tailnet)?;
    machine_stdin(
        podman,
        [
            "podman",
            "exec",
            "-i",
            "gnx-tailscaled",
            "tailscale",
            "serve",
            "set-raw",
        ],
        &config,
    )
    .map(|_| ())
    .map_err(|error| error.with_code("TAILSCALE_SERVE_APPLY_FAILED", Component::TailscaleServe))
}

pub(crate) fn wait_for_tailscale_serve(
    podman: &Path,
    hostname: &str,
    tailnet: &str,
) -> Result<(), GateError> {
    let domain = format!("{hostname}.{tailnet}");
    let host_port = format!("{domain}:443");
    let mut last_error = String::from("Tailscale Serve config is not ready");
    for attempt in 0..60 {
        match machine_stdin(
            podman,
            [
                "podman",
                "exec",
                "gnx-tailscaled",
                "tailscale",
                "serve",
                "status",
                "--json",
            ],
            &[],
        ) {
            Ok(output) => match parse_serve_status(&output.stdout, &host_port) {
                Ok(()) => return Ok(()),
                Err(error) => last_error = error,
            },
            Err(error) => last_error = error.message,
        }
        if attempt + 1 < 60 {
            thread::sleep(Duration::from_secs(2));
        }
    }
    Err(GateError::new(
        "TAILSCALE_SERVE_FAILED",
        Component::TailscaleServe,
        format!(
            "Tailscale Serve did not expose only the approved PVE UI: {last_error}; {}",
            tailscale_diagnostics(podman)
        ),
    ))
}

pub(crate) fn parse_serve_status(bytes: &[u8], host_port: &str) -> Result<(), String> {
    let status: TailscaleServeStatus = serde_json::from_slice(bytes)
        .map_err(|_| "tailscale serve status returned invalid JSON".to_string())?;
    let https = status.tcp.get("443").is_some_and(|entry| entry.https);
    let proxy = status
        .web
        .get(host_port)
        .and_then(|entry| entry.handlers.get("/"))
        .map(|handler| handler.proxy.as_str());
    let funnel_disabled = !status.allow_funnel.get(host_port).copied().unwrap_or(false);
    if !https || proxy != Some("https+insecure://127.0.0.1:8006") || !funnel_disabled {
        return Err("serve status is missing the fixed HTTPS PVE proxy or enables Funnel".into());
    }
    Ok(())
}

pub(crate) fn tailscale_diagnostics(podman: &Path) -> String {
    match machine_stdin(podman, ["sh", "-s"], TAILSCALE_DIAGNOSTICS.as_bytes()) {
        Ok(output) => bounded_text(&output.stdout),
        Err(error) => format!("diagnostics unavailable: {}", error.message),
    }
}
