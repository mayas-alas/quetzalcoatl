use std::path::PathBuf;

use crate::config::Config;
use crate::error::GnxError;

pub fn run(config_path: PathBuf) -> Result<(), GnxError> {
    let config = Config::load(&config_path)?;
    let controller = config.validate()?;
    println!("Controller: {}", controller.canonical());

    #[cfg(target_os = "windows")]
    crate::host::windows::service::start()?;

    #[cfg(target_os = "linux")]
    crate::host::linux::start_service()?;

    println!("Convergencia solicitada; use gnx status para observarla.");
    Ok(())
}
