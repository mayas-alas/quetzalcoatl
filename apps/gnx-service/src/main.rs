#[cfg(windows)]
mod application;
#[cfg(windows)]
mod domain;
#[cfg(windows)]
mod infrastructure;

#[cfg(windows)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Service,
    ValidateInstallation,
}

#[cfg(windows)]
fn parse_mode(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<Mode, &'static str> {
    match (args.next(), args.next()) {
        (None, None) => Ok(Mode::Service),
        (Some(argument), None) if argument == "--validate-installation" => {
            Ok(Mode::ValidateInstallation)
        }
        _ => Err("usage: gnx-service [--validate-installation]"),
    }
}

#[cfg(windows)]
fn run() -> Result<(), String> {
    match parse_mode(std::env::args_os().skip(1)).map_err(str::to_owned)? {
        Mode::Service => application::pipe_service::run().map_err(|error| error.to_string()),
        Mode::ValidateInstallation => {
            application::installation::validate()
                .map_err(|error| format!("{}: {}", error.code, error.message))?;
            println!("installation-validation: ok");
            Ok(())
        }
    }
}

#[cfg(windows)]
fn main() {
    if let Err(error) = run() {
        eprintln!("gnx-service: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("gnx-service requires Windows");
    std::process::exit(1);
}

#[cfg(all(test, windows))]
mod tests {
    use super::{Mode, parse_mode};
    use std::ffi::OsString;

    #[test]
    fn installation_validation_is_a_closed_local_mode() {
        assert_eq!(
            parse_mode([OsString::from("--validate-installation")].into_iter()),
            Ok(Mode::ValidateInstallation)
        );
        assert_eq!(parse_mode(std::iter::empty()), Ok(Mode::Service));
        assert!(parse_mode([OsString::from("--other")].into_iter()).is_err());
        assert!(
            parse_mode(
                [
                    OsString::from("--validate-installation"),
                    OsString::from("extra")
                ]
                .into_iter()
            )
            .is_err()
        );
    }
}
