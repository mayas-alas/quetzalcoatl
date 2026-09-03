use std::path::PathBuf;

use clap::{Parser, Subcommand, error::ErrorKind};

use crate::{Error, Result, config::Config};

#[derive(Debug, Parser)]
#[command(name = "gnx", version, about = "GNX: access, compute and controller")]
struct Cli {
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Capability,
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum AccessAction {
    Configure,
    Apply,
    Dns,
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum ComputeAction {
    Apply,
    Status,
    Credentials,
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum ControllerAction {
    Apply,
    Status,
}

#[derive(Debug, Subcommand)]
enum Capability {
    Access {
        #[command(subcommand)]
        action: AccessAction,
    },
    Compute {
        #[command(subcommand)]
        action: ComputeAction,
    },
    Controller {
        #[command(subcommand)]
        action: ControllerAction,
    },
}

pub fn run() -> Result<String> {
    let cli = Cli::try_parse().map_err(|error| {
        if matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        ) {
            error.exit();
        }
        Error::Arguments
    })?;
    let path = cli
        .config
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(default_config);
    let config = Config::load(&path)?;

    #[cfg(windows)]
    return crate::platform::forward(&config.host.distribution, &path, &cli.command_args());

    #[cfg(target_os = "linux")]
    return match cli.command {
        Capability::Access { action } => match action {
            AccessAction::Configure => crate::access::configure(&config),
            AccessAction::Apply => crate::access::apply(&config),
            AccessAction::Dns => {
                let report = crate::access::dns(&config)?;
                match report.checks {
                    Ok(()) => Ok(format!("access-dns\n{}", report.fields)),
                    Err(error) => Err(Error::AccessReport {
                        operation: error.label(),
                        fields: report.fields,
                    }),
                }
            }
        },
        Capability::Compute { action } => match action {
            ComputeAction::Apply => crate::compute::apply(&config),
            ComputeAction::Status => crate::compute::status(&config),
            ComputeAction::Credentials => crate::compute::credentials(&config),
        },
        Capability::Controller { action } => match action {
            ControllerAction::Apply => crate::controller::apply(&config),
            ControllerAction::Status => crate::controller::status(&config),
        },
    };

    #[cfg(not(any(windows, target_os = "linux")))]
    Err(Error::HostUnsupported)
}

fn default_config() -> PathBuf {
    #[cfg(windows)]
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        return directory.join("gnx.toml");
    }
    #[cfg(target_os = "linux")]
    return PathBuf::from("/etc/gnx/gnx.toml");
    #[allow(unreachable_code)]
    PathBuf::from("gnx.toml")
}

#[cfg(windows)]
impl Cli {
    fn command_args(&self) -> [&'static str; 2] {
        match self.command {
            Capability::Access { action } => [
                "access",
                match action {
                    AccessAction::Configure => "configure",
                    AccessAction::Apply => "apply",
                    AccessAction::Dns => "dns",
                },
            ],
            Capability::Compute { action } => [
                "compute",
                match action {
                    ComputeAction::Apply => "apply",
                    ComputeAction::Status => "status",
                    ComputeAction::Credentials => "credentials",
                },
            ],
            Capability::Controller { action } => [
                "controller",
                match action {
                    ControllerAction::Apply => "apply",
                    ControllerAction::Status => "status",
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_is_exactly_three_capabilities() {
        for args in [
            ["gnx", "access", "dns"],
            ["gnx", "compute", "status"],
            ["gnx", "controller", "status"],
        ] {
            assert!(Cli::try_parse_from(args).is_ok());
        }
    }

    #[test]
    fn secrets_have_no_cli_position() {
        let marker = "GNX-NONSECRET-INPUT-MARKER";
        assert!(Cli::try_parse_from(["gnx", "access", "configure", marker]).is_err());
        assert!(Cli::try_parse_from(["gnx", "compute", "apply", "--password", marker]).is_err());
    }
}
