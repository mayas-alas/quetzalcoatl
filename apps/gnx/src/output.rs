use gnx_contracts::StatusResponse;

pub(crate) fn format_human(status: &StatusResponse) -> String {
    let mut lines = vec![
        format!("overall: {}", status.overall),
        format!("stage: {}", status.stage),
        format!(
            "role: {}",
<<<<<<< HEAD
            status.role.map(|role| role.as_str()).unwrap_or("ready")
        ),
        format!(
            "controller: {}",
            status.controller.as_deref().unwrap_or("ready")
        ),
        format!("pve_url: {}", status.pve_url.as_deref().unwrap_or("ready")),
=======
            status
                .role
                .map(|role| role.as_str())
                .unwrap_or("not_resolved")
        ),
        format!(
            "controller: {}",
            status.controller.as_deref().unwrap_or("not_resolved")
        ),
>>>>>>> origin/master
        format!("service: {}", status.components.service),
        format!("wsl: {}", status.components.wsl),
        format!("podman_machine: {}", status.components.podman_machine),
        format!("kvm: {}", status.components.kvm),
        format!("tailscale: {}", status.components.tailscale),
        format!("tailscale_serve: {}", status.components.tailscale_serve),
        format!("proxmox: {}", status.components.proxmox),
        format!("cluster_joined: {}", status.cluster.joined),
        format!("cluster_quorate: {}", status.cluster.quorate),
    ];
    if let Some(error) = &status.last_error {
        lines.push(format!("last_error: {error}"));
    }
    if let Some(platform) = &status.platform {
        lines.push(format!("platform: {}", platform.health));
        if let Some(url) = &platform.forgejo_url {
            lines.push(format!("forgejo_url: {url}"));
        }
        if let Some(error) = &platform.last_error {
            lines.push(format!("platform_error: {error}"));
        }
    }
    lines.join("\n")
}

pub(crate) fn print_human(status: &StatusResponse) {
    println!("{}", format_human(status));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_status_exposes_the_complete_mvp_contract() {
        let status = StatusResponse::member_ready("gnx-controller-a".into());
        let output = format_human(&status);
        for expected in [
            "overall: ready",
            "stage: READY",
            "role: member",
            "controller: gnx-controller-a",
            "service: ready",
            "wsl: ready",
            "podman_machine: ready",
            "kvm: ready",
            "tailscale: ready",
            "tailscale_serve: ready",
            "proxmox: ready",
            "cluster_joined: true",
            "cluster_quorate: true",
        ] {
            assert!(output.lines().any(|line| line == expected));
        }
    }
}
