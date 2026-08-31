use crate::error::GnxError;
use crate::host::UninstallOutcome;

pub fn run(elevated: bool) -> Result<(), GnxError> {
    match crate::host::uninstall(elevated)? {
        UninstallOutcome::Removed => {
            println!("GNX y Podman CLI fueron retirados; configuración y datos se conservaron.")
        }
        UninstallOutcome::RebootRequired => println!(
            "GNX y Podman CLI fueron retirados; Windows completará la limpieza al reiniciar."
        ),
        UninstallOutcome::RelaunchedElevated => {
            println!("La desinstalación continúa en una ventana elevada.")
        }
    }
    Ok(())
}
