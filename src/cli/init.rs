use std::path::PathBuf;

use crate::config::Config;
use crate::error::GnxError;
use crate::state::{OperationalState, Stage, default_state_path};

pub fn run(config_path: PathBuf) -> Result<(), GnxError> {
    let config = Config::load(&config_path)?;
    let controller = config.validate()?;
    println!("Controller: {}", controller.canonical());
    crate::runtime::headscale::verify_controller(&controller)?;

    #[cfg(target_os = "windows")]
    crate::host::windows::service::start()?;

    #[cfg(target_os = "linux")]
    crate::host::linux::start_service()?;

    let mut state = OperationalState::load(&default_state_path())?.unwrap_or_default();
    state.stage = Stage::Working;
    state.machine = "requested".to_string();
    state.mesh = "controller_configured".to_string();
    state.save(&default_state_path())?;
    println!("Convergencia solicitada; use gnx status para observarla.");
    Ok(())
}
