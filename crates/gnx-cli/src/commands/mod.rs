use std::env;
use std::ffi::OsString;

use crate::error::CliResult;

#[cfg(windows)]
mod configure;
#[cfg(windows)]
mod restart;
#[cfg(windows)]
mod status;
mod version;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Status { json: bool },
    Configure,
    Restart,
    Version,
}

pub(crate) fn usage() -> &'static str {
    "Usage:\n  gnx status [--json]\n  gnx configure\n  gnx restart\n  gnx -v\n  gnx --version"
}

fn parse_args_from(mut args: impl Iterator<Item = OsString>) -> Result<Action, ()> {
    match (args.next(), args.next(), args.next()) {
        (Some(command), None, None) if command == "status" => Ok(Action::Status { json: false }),
        (Some(command), Some(format), None) if command == "status" && format == "--json" => {
            Ok(Action::Status { json: true })
        }
        (Some(command), None, None) if command == "configure" => Ok(Action::Configure),
        (Some(command), None, None) if command == "restart" => Ok(Action::Restart),
        (Some(command), None, None) if command == "-v" || command == "--version" => {
            Ok(Action::Version)
        }
        _ => Err(()),
    }
}

pub(crate) fn parse_args() -> Result<Action, ()> {
    parse_args_from(env::args_os().skip(1))
}

#[cfg(windows)]
pub(crate) fn run(action: Action) -> CliResult<()> {
    match action {
        Action::Status { json } => status::run(json),
        Action::Configure => configure::run(),
        Action::Restart => restart::run(),
        Action::Version => version::run(),
    }
}

#[cfg(not(windows))]
pub(crate) fn run(action: Action) -> CliResult<()> {
    match action {
        Action::Version => version::run(),
        _ => Err("gnx requires Windows".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(values: &[&str]) -> Result<Action, ()> {
        parse_args_from(values.iter().map(|value| OsString::from(*value)))
    }

    #[test]
    fn version_flags_are_local_cli_actions() {
        assert_eq!(parse(&["-v"]), Ok(Action::Version));
        assert_eq!(parse(&["--version"]), Ok(Action::Version));
    }

    #[test]
    fn version_flags_reject_trailing_arguments() {
        assert!(parse(&["-v", "extra"]).is_err());
        assert!(parse(&["--version", "extra"]).is_err());
    }
}
