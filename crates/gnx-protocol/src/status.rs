use serde::{Deserialize, Serialize};

use crate::version::PROTOCOL_SCHEMA_VERSION;

#[derive(Clone, Deserialize, Serialize)]
pub struct StatusResponse {
    pub schema_version: u8,
    pub overall: String,
    pub stage: String,
    pub role: Option<String>,
    pub controller: Option<String>,
    pub components: Components,
    pub cluster: Cluster,
    pub last_error: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Components {
    pub service: String,
    pub wsl: String,
    pub podman_machine: String,
    pub kvm: String,
    pub tailscale: String,
    pub tailscale_serve: String,
    pub proxmox: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Cluster {
    pub joined: bool,
    pub quorate: bool,
}

impl StatusResponse {
    pub fn service_ready() -> Self {
        Self {
            schema_version: PROTOCOL_SCHEMA_VERSION,
            overall: "pending".into(),
            stage: "SERVICE_READY".into(),
            role: None,
            controller: None,
            components: Components {
                service: "ready".into(),
                wsl: "pending".into(),
                podman_machine: "pending".into(),
                kvm: "pending".into(),
                tailscale: "pending".into(),
                tailscale_serve: "pending".into(),
                proxmox: "pending".into(),
            },
            cluster: Cluster {
                joined: false,
                quorate: false,
            },
            last_error: None,
        }
    }

    pub fn member_ready(controller: String) -> Self {
        let mut status = Self::service_ready();
        status.overall = "ready".into();
        status.stage = "READY".into();
        status.role = Some("member".into());
        status.controller = Some(controller);
        status.components.wsl = "ready".into();
        status.components.podman_machine = "ready".into();
        status.components.kvm = "ready".into();
        status.components.tailscale = "ready".into();
        status.components.tailscale_serve = "ready".into();
        status.components.proxmox = "ready".into();
        status.cluster.joined = true;
        status.cluster.quorate = true;
        status
    }
}
