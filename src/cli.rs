use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    Result,
    adapter::{NativeMesh, WindowsHost},
    app::App,
    config::{Artifact, Config},
};

#[derive(Debug, Parser)]
#[command(name = "gnx", version, about = "GNX host mesh bootstrap")]
struct Cli {
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Action,
}

#[derive(Debug, Subcommand)]
enum Action {
    /// Validate the host and installed mesh client without mutation.
    Doctor,
    /// Install the pinned local mesh client package.
    Install {
        #[arg(long, value_name = "FILE")]
        release: PathBuf,
    },
    /// Connect the local node to the configured control server.
    Connect {
        #[arg(long, value_name = "FILE")]
        setup_key_file: Option<PathBuf>,
    },
}

pub fn run() -> Result<String> {
    run_with(Cli::parse())
}

fn run_with(cli: Cli) -> Result<String> {
    let config_path = cli.config.ok_or(crate::Error::ConfigRequired)?;
    let config = Config::load(&config_path)?;
    let app = App {
        host: WindowsHost,
        mesh: NativeMesh,
    };
    match cli.command {
        Action::Doctor => app.doctor(),
        Action::Install { release } => app.install(&Artifact::load(&release)?),
        Action::Connect { setup_key_file } => {
            app.connect(&config.control_server, setup_key_file.as_deref())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_required_and_global() {
        let missing = Cli::try_parse_from(["gnx", "doctor"]).unwrap();
        assert!(missing.config.is_none());
        let present = Cli::try_parse_from(["gnx", "doctor", "--config", "gnx.toml"]).unwrap();
        assert_eq!(present.config, Some("gnx.toml".into()));
    }

    #[test]
    fn setup_key_is_a_file_not_a_value() {
        assert!(
            Cli::try_parse_from([
                "gnx",
                "connect",
                "--config",
                "gnx.toml",
                "--setup-key-file",
                "key.txt",
            ])
            .is_ok()
        );
    }
}
