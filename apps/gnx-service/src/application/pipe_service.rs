use std::sync::{Arc, RwLock};

use gnx_contracts::StatusResponse;

use crate::application::control::RuntimeControl;

pub(crate) fn run() -> Result<(), String> {
    let status = Arc::new(RwLock::new(StatusResponse::service_ready()));
    RuntimeControl::start(Arc::clone(&status));
    crate::infrastructure::windows_pipe::serve(status)
}
