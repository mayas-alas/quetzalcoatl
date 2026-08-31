pub const QUADLET: &str = include_str!("../../runtime/proxmox.container");
pub const NETWORK: &str = include_str!("../../runtime/gnx-runtime.network");
pub const IMAGE: &str = "docker.io/dockurr/proxmox@sha256:99e86ebf6b78b4e551bacabd414ec801a8fc1c3082e76e16ea7fab0f15adb8cc";

pub fn validate_quadlet() -> bool {
    QUADLET.contains(IMAGE)
        && QUADLET.contains("ConditionPathExists=/dev/kvm")
        && QUADLET.contains("PodmanArgs=--privileged")
        && QUADLET.contains("/var/lib/gnx/proxmox/data:/var/lib/vz")
        && QUADLET.contains("127.0.0.1:8006:8006")
        && QUADLET.contains("HealthCmd=/usr/bin/pvesh get /version")
        && !QUADLET.contains(":latest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dockur_quadlet_is_pinned_and_persistent() {
        assert!(validate_quadlet());
    }
}
