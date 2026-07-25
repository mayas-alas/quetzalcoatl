use std::env;

use crate::error::CliResult;

#[cfg(windows)]
mod configure;
#[cfg(windows)]
mod restart;
#[cfg(windows)]
mod status;

pub(crate) enum Action {
    Status { json: bool },
    Configure,
    Restart,
}

pub(crate) fn usage() -> &'static str {
    "Usage:\n  gnx status [--json]\n  gnx configure\n  gnx restart"
}

pub(crate) fn parse_args() -> Result<Action, ()> {
    let mut args = env::args_os().skip(1);
    match (args.next(), args.next(), args.next()) {
        (Some(command), None, None) if command == "status" => Ok(Action::Status { json: false }),
        (Some(command), Some(format), None) if command == "status" && format == "--json" => {
            Ok(Action::Status { json: true })
        }
        (Some(command), None, None) if command == "configure" => Ok(Action::Configure),
        (Some(command), None, None) if command == "restart" => Ok(Action::Restart),
        _ => Err(()),
    }
}

#[cfg(windows)]
pub(crate) fn run(action: Action) -> CliResult<()> {
    match action {
        Action::Status { json } => status::run(json),
        Action::Configure => configure::run(),
        Action::Restart => restart::run(),
    }
}

#[cfg(not(windows))]
pub(crate) fn run(_action: Action) -> CliResult<()> {
    Err("gnx requires Windows".into())
}
