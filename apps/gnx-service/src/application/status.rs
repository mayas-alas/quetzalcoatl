use std::sync::{Arc, RwLock};

use gnx_contracts::{
    ComponentHealth, NodeRole, PlatformHealth, PlatformStatus, PlatformUrl, PveUrl, StatusResponse,
};

use crate::domain::errors::GateError;
use crate::domain::lifecycle::Component;

pub(crate) fn set_stage(status: &Arc<RwLock<StatusResponse>>, stage: &str) {
    if let Ok(mut status) = status.write() {
        status.stage = stage.into();
        status.overall = "pending".into();
        status.last_error = None;
    }
}

pub(crate) fn set_component(
    status: &Arc<RwLock<StatusResponse>>,
    component: Component,
    value: ComponentHealth,
) {
    if let Ok(mut status) = status.write() {
        component.set(&mut status, value);
    }
}

pub(crate) fn set_controller(status: &Arc<RwLock<StatusResponse>>, hostname: &str) {
    if let Ok(mut status) = status.write() {
        status.role = Some(NodeRole::Controller);
        status.controller = Some(hostname.into());
    }
}

pub(crate) fn set_member_stage_status(
    status: &Arc<RwLock<StatusResponse>>,
    controller: &str,
    stage: &str,
) {
    if let Ok(mut status) = status.write() {
        status.role = Some(NodeRole::Member);
        status.controller = Some(controller.into());
        status.stage = stage.into();
        status.overall = "pending".into();
        status.last_error = None;
    }
}

pub(crate) fn set_member_ready_status(status: &Arc<RwLock<StatusResponse>>, controller: &str) {
    if let Ok(mut value) = status.write() {
        let pve_url = value.pve_url.clone();
        *value = StatusResponse::member_ready(controller.to_owned());
        value.pve_url = pve_url;
    }
}

pub(crate) fn set_pve_url(
    status: &Arc<RwLock<StatusResponse>>,
    pve_url: &str,
) -> Result<(), GateError> {
    let pve_url = PveUrl::parse(pve_url)
        .map_err(|message| GateError::new("PVE_URL_INVALID", Component::TailscaleServe, message))?;
    if let Ok(mut status) = status.write() {
        status.pve_url = Some(pve_url);
    }
    Ok(())
}

pub(crate) fn set_cluster_ready(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.cluster.joined = true;
        status.cluster.quorate = true;
    }
}

pub(crate) fn complete(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.overall = "ready".into();
        status.stage = "READY".into();
        status.last_error = None;
    }
}

pub(crate) fn fail(status: &Arc<RwLock<StatusResponse>>, error: GateError) {
    if let Ok(mut status) = status.write() {
        status.overall = "failed".into();
        status.stage = "FAILED".into();
        error.component.set(&mut status, ComponentHealth::Failed);
        status.last_error = Some(format!("{}: {}", error.code, error.message));
    }
}

pub(crate) fn set_platform_waiting(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.platform = Some(PlatformStatus::waiting_configuration());
    }
}

pub(crate) fn set_platform_reconciling(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.platform = Some(PlatformStatus {
            health: PlatformHealth::Reconciling,
            forgejo_url: None,
            last_error: None,
        });
    }
}

pub(crate) fn set_platform_ready(status: &Arc<RwLock<StatusResponse>>, forgejo_url: PlatformUrl) {
    if let Ok(mut status) = status.write() {
        status.platform = Some(PlatformStatus {
            health: PlatformHealth::Ready,
            forgejo_url: Some(forgejo_url),
            last_error: None,
        });
    }
}

pub(crate) fn set_platform_failed(status: &Arc<RwLock<StatusResponse>>, error: &GateError) {
    if let Ok(mut status) = status.write() {
        status.platform = Some(PlatformStatus {
            health: PlatformHealth::Failed,
            forgejo_url: None,
            last_error: Some(format!("{}: {}", error.code, error.message)),
        });
    }
}
