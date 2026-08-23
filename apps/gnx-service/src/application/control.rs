use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use gnx_contracts::StatusResponse;

pub(crate) struct RuntimeControl;

impl RuntimeControl {
    pub(crate) fn start(status: Arc<RwLock<StatusResponse>>) -> JoinHandle<()> {
        std::thread::spawn(move || super::runtime::run(status))
    }
}
