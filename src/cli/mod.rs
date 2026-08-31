mod doctor;
mod init;
mod install;
mod status;
mod uninstall;
mod update;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

use crate::config::default_config_path;
use crate::error::GnxError;

#[derive(Debug, Parser)]
#[command(
    name = "gnx",
    version,
    about = "Quetzalcoatl Next — orquestador soberano",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Ruta explícita a config.toml.
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Converge el runtime declarado.
    Init,
    /// Muestra el estado observado sin mutar.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Ejecuta diagnósticos de sólo lectura.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Muestra eventos persistentes de instalación y runtime.
    Logs {
        /// Número máximo de eventos recientes.
        #[arg(long, default_value_t = 100, value_name = "N")]
        tail: usize,
        /// Conserva cada evento como JSONL.
        #[arg(long)]
        json: bool,
    },
    /// Reconverge instalación y runtime.
    Repair,
    /// Actualiza GNX mediante un release verificado.
    Update {
        /// EXE o AppImage nuevo obtenido del release GNX.
        #[arg(long, value_name = "PATH")]
        from: PathBuf,
        /// SHA-256 publicado para ese artefacto.
        #[arg(long, value_name = "HEX")]
        sha256: String,
    },
    /// Retira GNX sin destruir datos de workloads.
    Uninstall {
        #[arg(long, hide = true)]
        elevated: bool,
    },
    /// Muestra la versión del producto.
    Version,
    #[command(hide = true, name = "__service")]
    Service,
    #[command(hide = true, name = "__tray")]
    Tray,
    #[command(hide = true, name = "__resume")]
    Resume,
    #[command(hide = true, name = "__install")]
    InternalInstall {
        #[arg(long, hide = true)]
        elevated: bool,
        #[arg(long, hide = true)]
        resume: bool,
    },
}

pub fn execute<I, T>(args: I) -> Result<(), GnxError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut arguments: Vec<OsString> = args.into_iter().map(Into::into).collect();
    if arguments.len() == 1 {
        if crate::host::running_from_installed_path() {
            Cli::command()
                .print_help()
                .map_err(|error| GnxError::io("help", error.to_string()))?;
            println!();
            return Ok(());
        }
        arguments.push(OsString::from("__install"));
    }
    let cli = Cli::parse_from(arguments);
    let config_path = cli.config.unwrap_or_else(default_config_path);

    match cli.command {
        Some(Command::Init) => init::run(config_path),
        Some(Command::Status { json }) => status::run(config_path, json),
        Some(Command::Doctor { json }) => doctor::run(config_path, json),
        Some(Command::Logs { tail, json }) => crate::logs::print_tail(tail, json),
        Some(Command::Repair) => repair(config_path),
        Some(Command::Version) => {
            println!("gnx {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(Command::Update { from, sha256 }) => update::run(from, sha256),
        Some(Command::Uninstall { elevated }) => uninstall::run(elevated),
        Some(Command::Service) => run_service(),
        Some(Command::Tray) => run_tray(config_path),
        Some(Command::Resume) => install::run(false, true).map(|_| ()),
        Some(Command::InternalInstall { elevated, resume }) => {
            install::run(elevated, resume).map(|_| ())
        }
        None => Ok(()),
    }
}

fn repair(config_path: PathBuf) -> Result<(), GnxError> {
    match install::run(false, true)? {
        crate::host::InstallOutcome::Installed => init::run(config_path),
        crate::host::InstallOutcome::RebootRequired
        | crate::host::InstallOutcome::RelaunchedElevated => Ok(()),
    }
}

#[cfg(target_os = "windows")]
fn run_service() -> Result<(), GnxError> {
    crate::host::windows::service::run()
}

#[cfg(not(target_os = "windows"))]
fn run_service() -> Result<(), GnxError> {
    crate::host::linux::run_service()
}

#[cfg(target_os = "windows")]
fn run_tray(config_path: PathBuf) -> Result<(), GnxError> {
    crate::host::windows::tray::run(config_path)
}

#[cfg(not(target_os = "windows"))]
fn run_tray(config_path: PathBuf) -> Result<(), GnxError> {
    status::run(config_path, false)
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_contract_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn internal_modes_are_hidden_from_help() {
        let help = Cli::command().render_long_help().to_string();
        assert!(!help.contains("__service"));
        assert!(!help.contains("__tray"));
        assert!(!help.contains("__resume"));
        assert!(!help.contains("__install"));
    }

    #[test]
    fn internal_install_is_valid_default_target() {
        let cli = Cli::try_parse_from(["gnx", "__install"]).unwrap();
        assert!(matches!(cli.command, Some(Command::InternalInstall { .. })));
    }

    #[test]
    fn install_is_not_a_public_command() {
        assert!(Cli::try_parse_from(["gnx", "install"]).is_err());
    }
}
