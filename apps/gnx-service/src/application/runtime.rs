use std::sync::{Arc, RwLock};

use gnx_contracts::StatusResponse;

use super::reconciler;
use super::status::{fail, set_stage};

pub(crate) fn run(status: Arc<RwLock<StatusResponse>>) {
    set_stage(&status, "RUNTIME_IDENTITY");
    if let Err(error) = reconciler::run(&status) {
        fail(&status, error);
        return;
    }
    super::platform::run(&status);
}
