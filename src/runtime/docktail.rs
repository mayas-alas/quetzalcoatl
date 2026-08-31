pub const QUADLET: &str = include_str!("../../runtime/docktail.container");
pub const IMAGE: &str = "ghcr.io/marvinvr/docktail@sha256:dc1ceec533df5c13d56754e41173767e2cef78da21b9d4d1e58372f73392f668";
pub const ALLOWED_LABELS: &[&str] = &[
    "docktail.service.enable",
    "docktail.service.name",
    "docktail.service.port",
    "docktail.service.direct",
];

pub fn validate_quadlet() -> bool {
    QUADLET.contains(IMAGE)
        && QUADLET.contains("/run/podman/podman.sock:/var/run/docker.sock:ro")
        && QUADLET.contains("/run/gnx/tailscale:/var/run/tailscale")
        && !QUADLET.contains(":latest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadlet_is_digest_pinned_and_local() {
        assert!(validate_quadlet());
    }
}
