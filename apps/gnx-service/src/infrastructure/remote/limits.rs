use std::time::Duration;

pub(crate) const MAX_REMOTE_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_REMOTE_OUTPUT_BYTES: usize = 1024 * 1024;
pub(crate) const REMOTE_COMMAND_TIMEOUT: Duration = Duration::from_secs(15 * 60);
pub(crate) const REMOTE_POLL_INTERVAL: Duration = Duration::from_millis(50);
