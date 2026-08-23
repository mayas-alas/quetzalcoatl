use std::sync::{Arc, RwLock};

use gnx_contracts::StatusResponse;

use crate::application::control::RuntimeControl;
use crate::infrastructure::service_shutdown::ShutdownToken;

pub(crate) fn run(shutdown: ShutdownToken) -> Result<(), String> {
    let status = Arc::new(RwLock::new(StatusResponse::service_ready()));
    let runtime = RuntimeControl::start(Arc::clone(&status));
    crate::infrastructure::windows_pipe::serve(status, shutdown)?;
    runtime
        .join()
        .map_err(|_| "runtime reconciliation thread panicked during shutdown".to_string())
}
