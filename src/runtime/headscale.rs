use crate::config::ControllerUrl;
use crate::error::GnxError;

pub const VMID: &str = "190";
pub const ADDRESS: &str = "172.30.70.10";
pub const QUADLET: &str = include_str!("../../guest/units/headscale.container");
pub const BOOTSTRAP: &str = include_str!("../../guest/control-plane-bootstrap.sh");
pub const IMAGE: &str = "ghcr.io/juanfont/headscale@sha256:9a4d883997cbe9de8ff58e8e11c37c204a25bd76d4c0920d16e0d944bbe05fb8";

pub const POLICY: &str = r#"{
  "tagOwners": {
    "tag:server": [],
    "tag:container": [],
    "tag:gnx-runtime": [],
    "tag:gnx-cell": []
  },
  "acls": [
    { "action": "accept", "src": ["*"], "dst": ["*:*" ] }
  ]
}
"#;

pub fn config(controller: &ControllerUrl) -> Result<String, GnxError> {
    if !matches!(
        controller.host(),
        "controlplane.node.gnx" | "headscale.node.gnx"
    ) {
        return Err(GnxError::controller_invalid(
            "El control plane administrado debe usar controlplane.node.gnx o headscale.node.gnx.",
        ));
    }
    Ok(format!(
        r#"server_url: {controller}
listen_addr: 0.0.0.0:443
metrics_listen_addr: 127.0.0.1:9090
grpc_listen_addr: 127.0.0.1:50443
grpc_allow_insecure: false
trusted_proxies: []
noise:
  private_key_path: /var/lib/headscale/noise_private.key
prefixes:
  v4: 100.64.0.0/10
  v6: fd7a:115c:a1e0::/48
  allocation: sequential
derp:
  server:
    enabled: false
  urls:
    - https://controlplane.tailscale.com/derpmap/default
  paths: []
  auto_update_enabled: true
  update_frequency: 24h
disable_check_updates: true
node:
  expiry: 0
  ephemeral:
    inactivity_timeout: 30m
database:
  type: sqlite
  debug: false
  sqlite:
    path: /var/lib/headscale/db.sqlite
    write_ahead_log: true
tls_cert_path: /var/lib/headscale/tls/server.crt
tls_key_path: /var/lib/headscale/tls/server.key
log:
  level: info
  format: json
policy:
  mode: file
  path: /etc/headscale/policy.hujson
dns:
  magic_dns: true
  base_domain: mesh.node.gnx
  override_local_dns: false
  nameservers:
    global: []
    split: {{}}
  search_domains: []
  extra_records: []
unix_socket: /var/run/headscale/headscale.sock
unix_socket_permission: "0770"
logtail:
  enabled: false
taildrop:
  enabled: true
auto_update:
  enabled: false
"#,
        controller = controller.canonical()
    ))
}

pub fn validate_assets() -> bool {
    QUADLET.contains(IMAGE)
        && QUADLET.contains("ContainerName=gnx-headscale")
        && QUADLET.contains("Exec=serve")
        && BOOTSTRAP.contains("control_vmid=190")
        && BOOTSTRAP.contains("podman exec gnx-headscale headscale preauthkeys create")
        && BOOTSTRAP.contains("ip=${control_address}/24")
        && !QUADLET.contains(":latest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_plane_is_a_pinned_quadlet_in_a_fixed_lxc() {
        assert!(validate_assets());
    }

    #[test]
    fn both_sovereign_names_are_supported() {
        for name in ["controlplane.node.gnx", "headscale.node.gnx"] {
            let controller = ControllerUrl::parse(&format!("https://{name}")).unwrap();
            assert!(
                config(&controller)
                    .unwrap()
                    .contains(&format!("server_url: https://{name}"))
            );
        }
    }

    #[test]
    fn external_controller_is_not_silently_adopted() {
        let controller = ControllerUrl::parse("https://login.tailscale.com").unwrap();
        assert_eq!(
            config(&controller).unwrap_err().code,
            "MESH_CONTROLLER_URL_INVALID"
        );
    }
}
