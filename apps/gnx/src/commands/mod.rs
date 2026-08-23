use std::env;
use std::ffi::OsString;

use crate::error::CliResult;

#[cfg(windows)]
mod configure;
#[cfg(windows)]
mod forgejo;
#[cfg(windows)]
mod restart;
#[cfg(windows)]
mod status;
mod version;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Status { json: bool },
    Configure,
    ConfigurePlatform,
    ForgejoAdminShow,
    ForgejoAdminReset,
    Restart,
    Version,
}

pub(crate) fn usage() -> &'static str {
    "Usage:\n  gnx status [--json]\n  gnx configure\n  gnx configure platform\n  gnx forgejo admin show\n  gnx forgejo admin reset --confirm\n  gnx restart\n  gnx version\n  gnx --version\n  gnx -V"
}

fn parse_args_from(args: impl Iterator<Item = OsString>) -> Result<Action, ()> {
    let arguments = args.collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "status" => Ok(Action::Status { json: false }),
        [command, format] if command == "status" && format == "--json" => {
            Ok(Action::Status { json: true })
        }
        [command] if command == "configure" => Ok(Action::Configure),
        [command, scope] if command == "configure" && scope == "platform" => {
            Ok(Action::ConfigurePlatform)
        }
        [resource, identity, action]
            if resource == "forgejo" && identity == "admin" && action == "show" =>
        {
            Ok(Action::ForgejoAdminShow)
        }
        [resource, identity, action, confirmation]
            if resource == "forgejo"
                && identity == "admin"
                && action == "reset"
                && confirmation == "--confirm" =>
        {
            Ok(Action::ForgejoAdminReset)
        }
        [command] if command == "restart" => Ok(Action::Restart),
        [command] if command == "version" || command == "--version" || command == "-V" => {
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
        Action::ConfigurePlatform => configure::run_platform(),
        Action::ForgejoAdminShow => forgejo::show(),
        Action::ForgejoAdminReset => forgejo::reset(),
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
        assert_eq!(parse(&["version"]), Ok(Action::Version));
        assert_eq!(parse(&["--version"]), Ok(Action::Version));
        assert_eq!(parse(&["-V"]), Ok(Action::Version));
        assert!(parse(&["-v"]).is_err());
    }

    #[test]
    fn version_flags_reject_trailing_arguments() {
        assert!(parse(&["version", "extra"]).is_err());
        assert!(parse(&["--version", "extra"]).is_err());
        assert!(parse(&["-V", "extra"]).is_err());
    }

    #[test]
    fn platform_configuration_has_one_canonical_command() {
        assert_eq!(
            parse(&["configure", "platform"]),
            Ok(Action::ConfigurePlatform)
        );
        assert!(parse(&["platform", "configure"]).is_err());
        assert!(parse(&["configure", "platform", "extra"]).is_err());
    }

    #[test]
    fn forgejo_admin_commands_are_hierarchical_and_closed() {
        assert_eq!(
            parse(&["forgejo", "admin", "show"]),
            Ok(Action::ForgejoAdminShow)
        );
        assert_eq!(
            parse(&["forgejo", "admin", "reset", "--confirm"]),
            Ok(Action::ForgejoAdminReset)
        );
        assert!(parse(&["credentials", "forgejo"]).is_err());
        assert!(parse(&["reset", "forgejo-admin"]).is_err());
        assert!(parse(&["forgejo", "admin", "reset"]).is_err());
    }
}
