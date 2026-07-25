use std::sync::{Arc, RwLock};

use gnx_protocol::StatusResponse;

pub(crate) struct RuntimeControl;

impl RuntimeControl {
    pub(crate) fn start(status: Arc<RwLock<StatusResponse>>) {
        std::thread::spawn(move || super::run(status));
    }
}
