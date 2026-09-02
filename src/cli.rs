use std::path::PathBuf;

use clap::{Parser, Subcommand, error::ErrorKind};

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
    /// Reveal a saved account to the human in a temporary console screen.
    Credentials {
        #[arg(value_enum)]
        account: crate::credentials::Account,
    },
    /// Configure private access or show the nameserver form values.
    Access {
        #[command(subcommand)]
        command: AccessAction,
    },
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

#[derive(Debug, Subcommand)]
enum AccessAction {
    /// Prompt for enrollment only when needed; input is hidden and temporary.
    Configure,
    /// Reconcile access without prompting or accepting credentials.
    Apply,
    /// Show exact DNS form values and verify local readiness; no changes.
    Dns,
}

pub fn run() -> Result<String> {
    let cli = Cli::try_parse().map_err(|error| {
        if matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        ) {
            error.exit();
        }
        // A misplaced secret must not be echoed by the argument parser.
        crate::Error::Arguments
    })?;
    run_with(cli)
}

fn run_with(cli: Cli) -> Result<String> {
    if let Action::Credentials { account } = cli.command {
        return crate::credentials::show(account);
    }
    if let Action::Access { command } = cli.command {
        let path = match cli.config {
            Some(path) => path,
            None => std::env::current_exe()
                .map_err(crate::Error::Spawn)?
                .with_file_name("access.toml"),
        };
        let result = match command {
            AccessAction::Configure => gnx_access::configure(&path),
            AccessAction::Apply => gnx_access::apply(&path),
            AccessAction::Dns => {
                let report = gnx_access::dns(&path)
                    .map_err(|operation| crate::Error::External { operation, code: 1 })?;
                return match report.checks {
                    Ok(()) => Ok(format!("access-dns\n{}", report.fields)),
                    Err(operation) => Err(crate::Error::AccessReport {
                        operation,
                        fields: report.fields,
                    }),
                };
            }
        };
        return result.map_err(|operation| crate::Error::External { operation, code: 1 });
    }
    let config_path = cli.config.ok_or(crate::Error::ConfigRequired)?;
    let config = Config::load(&config_path)?;
    let app = App {
        host: WindowsHost,
        mesh: NativeMesh,
    };
    match cli.command {
        Action::Access { .. } => unreachable!("access has its own configuration"),
        Action::Credentials { .. } => unreachable!("credentials do not use mesh configuration"),
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

    #[test]
    fn access_has_a_human_prompt_and_read_only_dns_without_key_arguments() {
        for action in ["configure", "apply", "dns"] {
            assert!(Cli::try_parse_from(["gnx", "access", action]).is_ok());
            for flag in ["--key", "--key-file", "--setup-key-file"] {
                assert!(Cli::try_parse_from(["gnx", "access", action, flag, "example"]).is_err());
            }
        }
    }

    #[test]
    fn credentials_are_limited_to_the_two_saved_accounts() {
        for account in ["control", "compute"] {
            assert!(Cli::try_parse_from(["gnx", "credentials", account]).is_ok());
            for flag in ["--password", "--output", "--copy", "--file"] {
                assert!(
                    Cli::try_parse_from(["gnx", "credentials", account, flag, "example"]).is_err()
                );
            }
        }
        assert!(Cli::try_parse_from(["gnx", "credentials", "recovery"]).is_err());
    }
}
