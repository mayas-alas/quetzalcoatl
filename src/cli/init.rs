use std::io::Read;
use std::net::IpAddr;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::GnxError;

pub fn run(
    config_path: PathBuf,
    mesh_auth_stdin: bool,
    controller_addresses: Vec<IpAddr>,
) -> Result<(), GnxError> {
    if !controller_addresses.is_empty() && config_path != crate::config::default_config_path() {
        return Err(GnxError::config_invalid(
            "--controller-address sólo modifica la configuración instalada por GNX; retire --config.",
        ));
    }

    #[cfg(target_os = "windows")]
    if !controller_addresses.is_empty() && !mesh_auth_stdin {
        crate::host::windows::resolution::configure(controller_addresses.clone())?;
    }

    #[cfg(not(target_os = "windows"))]
    if !controller_addresses.is_empty() {
        return Err(GnxError::unsupported_host(
            "La configuración automática del bootstrap se cerrará en Linux después del flujo Windows.",
        ));
    }

    let config = Config::load(&config_path)?;
    if !controller_addresses.is_empty() {
        let mut candidate = config.clone();
        candidate.mesh.bootstrap_addresses = controller_addresses.clone();
        candidate.validate()?;
    }
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
        crate::host::windows::ipc::submit_mesh_auth(&mut secret, &controller_addresses)?;
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
