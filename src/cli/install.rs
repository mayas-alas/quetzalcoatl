use crate::error::GnxError;
use crate::host::{InstallOptions, InstallOutcome};

pub fn run(elevated: bool, resume: bool) -> Result<InstallOutcome, GnxError> {
    println!("Quetzalcoatl Next {}", env!("CARGO_PKG_VERSION"));
    println!("Preparando {} x86_64…", std::env::consts::OS);
    let outcome = crate::host::install(InstallOptions { elevated, resume })?;
    match outcome {
        InstallOutcome::Installed => println!("Instalación base terminada."),
        InstallOutcome::RebootRequired => {
            println!("Reinicio requerido; GNX continuará automáticamente después del logon.")
        }
        InstallOutcome::RelaunchedElevated => {
            println!("El instalador continúa en una ventana elevada.")
        }
    }
    Ok(outcome)
}
