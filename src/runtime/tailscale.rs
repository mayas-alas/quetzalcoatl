use crate::config::ControllerUrl;
use crate::error::GnxError;

pub const QUADLET: &str = include_str!("../../runtime/tailscale.container");
pub const IMAGE: &str = "docker.io/tailscale/tailscale@sha256:51fec4863144d6ba0a22504cfb455d020ee0307d0c04eb48f5afee63390bdba0";

pub fn enrollment_arguments(
    controller: &ControllerUrl,
    hostname: &str,
    tag: &str,
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
    if !tag.starts_with("tag:")
        || !tag
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, ':' | '-'))
    {
        return Err(GnxError::config_invalid("El tag mesh es inválido."));
    }
    Ok(vec![
        "up".to_string(),
        format!("--login-server={}", controller.canonical()),
        format!("--hostname={hostname}"),
        format!("--advertise-tags={tag}"),
        "--ssh=false".to_string(),
    ])
}

pub fn validate_quadlet() -> bool {
    QUADLET.contains(IMAGE)
        && QUADLET.contains("AddDevice=/dev/net/tun")
        && QUADLET.contains("TS_SOCKET=/var/run/tailscale/tailscaled.sock")
        && !QUADLET.contains(":latest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_keep_exact_controller() {
        let controller = ControllerUrl::parse("https://headscale.node.gnx").unwrap();
        let args = enrollment_arguments(&controller, "gnx-runtime", "tag:gnx-runtime").unwrap();
        assert!(args.contains(&"--login-server=https://headscale.node.gnx".to_string()));
        assert!(!args.iter().any(|argument| argument.contains("auth")));
    }

    #[test]
    fn quadlet_is_digest_pinned_and_owns_local_socket() {
        assert!(validate_quadlet());
    }
}
