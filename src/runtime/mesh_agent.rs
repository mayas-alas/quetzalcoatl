use crate::config::ControllerUrl;
use crate::error::GnxError;

pub const QUADLET: &str = include_str!("../../runtime/mesh-agent.container");
pub const GUEST_QUADLET: &str = include_str!("../../guest/units/mesh-agent.container");
pub const IMAGE: &str = "docker.io/tailscale/tailscale@sha256:51fec4863144d6ba0a22504cfb455d020ee0307d0c04eb48f5afee63390bdba0";

pub fn enrollment_arguments(
    controller: &ControllerUrl,
    hostname: &str,
) -> Result<Vec<String>, GnxError> {
    if hostname.is_empty()
        || !hostname
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(GnxError::config_invalid(
            "El hostname mesh debe ser alfanumérico con guiones.",
        ));
    }
    Ok(vec![
        "up".to_string(),
        format!("--login-server={}", controller.canonical()),
        format!("--hostname={hostname}"),
        "--ssh=false".to_string(),
    ])
}

pub fn environment(controller: &ControllerUrl, hostname: &str) -> Result<String, GnxError> {
    let arguments = enrollment_arguments(controller, hostname)?;
    Ok(format!(
        "TS_HOSTNAME={hostname}\nTS_AUTH_ONCE=true\nTS_ACCEPT_DNS=true\nTS_EXTRA_ARGS={} --accept-dns=true\n",
        arguments[1..].join(" ")
    ))
}

pub fn validate_quadlets() -> bool {
    [QUADLET, GUEST_QUADLET].iter().all(|quadlet| {
        quadlet.contains(IMAGE)
            && quadlet.contains("ContainerName=gnx-mesh-agent")
            && quadlet.contains("TS_SOCKET=/var/run/tailscale/tailscaled.sock")
            && quadlet.contains("SSL_CERT_DIR=/etc/ssl/certs:/run/gnx/control-plane")
            && !quadlet.contains(":latest")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrollment_targets_the_sovereign_controller_without_human_flags() {
        let controller = ControllerUrl::parse("https://headscale.node.gnx").unwrap();
        let args = enrollment_arguments(&controller, "gnx-runtime").unwrap();
        assert!(args.contains(&"--login-server=https://headscale.node.gnx".to_string()));
        assert!(!args.iter().any(|argument| argument.contains("auth")));
    }

    #[test]
    fn both_cells_use_one_digest_pinned_shared_agent() {
        assert!(validate_quadlets());
    }
}
