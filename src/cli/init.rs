use std::io::Read;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::GnxError;

pub fn run(config_path: PathBuf, mesh_auth_stdin: bool) -> Result<(), GnxError> {
    let config = Config::load(&config_path)?;
    let controller = config.validate()?;
    println!("Controller: {}", controller.canonical());

    if mesh_auth_stdin {
        let mut secret = Vec::new();
        std::io::stdin()
            .take(4097)
            .read_to_end(&mut secret)
            .map_err(|error| GnxError::io("mesh_auth_stdin", error.to_string()))?;
        while secret.last().is_some_and(|byte| byte.is_ascii_whitespace()) {
            secret.pop();
        }
        #[cfg(target_os = "windows")]
        crate::host::windows::ipc::submit_mesh_auth(&mut secret)?;
        #[cfg(not(target_os = "windows"))]
        {
            secret.fill(0);
            return Err(GnxError::unsupported_host(
                "El enrolamiento por stdin se cerrará en Linux después del flujo Windows.",
            ));
        }
        println!(
            "Credencial efímera entregada al servicio dedicado; no se persistió en config ni argumentos."
        );
    }

    #[cfg(target_os = "windows")]
    crate::host::windows::service::start()?;

    #[cfg(target_os = "linux")]
    crate::host::linux::start_service()?;

    println!("Convergencia solicitada; use gnx status para observarla.");
    Ok(())
}
