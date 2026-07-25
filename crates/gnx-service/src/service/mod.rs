use std::sync::{Arc, RwLock};

use gnx_protocol::StatusResponse;

use crate::runtime::control::RuntimeControl;

pub(crate) fn run() -> Result<(), String> {
    let status = Arc::new(RwLock::new(StatusResponse::service_ready()));
    RuntimeControl::start(Arc::clone(&status));
    crate::ipc::serve(status)
}
