use super::*;

pub(super) fn set_stage(status: &Arc<RwLock<StatusResponse>>, stage: &str) {
    if let Ok(mut status) = status.write() {
        status.stage = stage.to_owned();
        status.overall = "pending".into();
        status.last_error = None;
    }
}

pub(super) fn set_component(
    status: &Arc<RwLock<StatusResponse>>,
    component: Component,
    value: &str,
) {
    if let Ok(mut status) = status.write() {
        component.set(&mut status, value);
    }
}

pub(super) fn set_controller(status: &Arc<RwLock<StatusResponse>>, hostname: &str) {
    if let Ok(mut status) = status.write() {
        status.role = Some("controller".into());
        status.controller = Some(hostname.into());
    }
}

pub(super) fn set_member_joining_status(status: &Arc<RwLock<StatusResponse>>, controller: &str) {
    if let Ok(mut status) = status.write() {
        status.role = Some("member".into());
        status.controller = Some(controller.into());
        status.stage = "MEMBER_JOINING".into();
        status.overall = "pending".into();
    }
}

pub(super) fn set_member_ready_status(status: &Arc<RwLock<StatusResponse>>, controller: &str) {
    if let Ok(mut value) = status.write() {
        *value = StatusResponse::member_ready(controller.to_owned());
    }
}

pub(super) fn set_cluster_ready(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.cluster.joined = true;
        status.cluster.quorate = true;
    }
}

pub(super) fn complete(status: &Arc<RwLock<StatusResponse>>) {
    if let Ok(mut status) = status.write() {
        status.overall = "ready".into();
        status.stage = "READY".into();
        status.last_error = None;
    }
}

pub(super) fn fail(status: &Arc<RwLock<StatusResponse>>, error: GateError) {
    if let Ok(mut status) = status.write() {
        status.overall = "failed".into();
        status.stage = "FAILED".into();
        error.component.set(&mut status, "failed");
        status.last_error = Some(format!("{}: {}", error.code, error.message));
    }
}
