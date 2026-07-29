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
    StopManagedMachine,
}

#[cfg(windows)]
fn parse_mode(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<Mode, &'static str> {
    match (args.next(), args.next()) {
        (None, None) => Ok(Mode::Service),
        (Some(argument), None) if argument == "--validate-installation" => {
            Ok(Mode::ValidateInstallation)
        }
        (Some(argument), None) if argument == "--stop-managed-machine" => {
            Ok(Mode::StopManagedMachine)
        }
        _ => Err("usage: gnx-service [--validate-installation|--stop-managed-machine]"),
    }
}

#[cfg(windows)]
fn run() -> Result<(), String> {
    match parse_mode(std::env::args_os().skip(1)).map_err(str::to_owned)? {
        Mode::Service => {
            infrastructure::service_shutdown::arm()?;
            application::pipe_service::run().map_err(|error| error.to_string())
        }
        Mode::ValidateInstallation => {
            application::installation::validate()
                .map_err(|error| format!("{}: {}", error.code, error.message))?;
            println!("installation-validation: ok");
            Ok(())
        }
        Mode::StopManagedMachine => {
            infrastructure::host::validate_identity().map_err(|error| error.message)?;
            let podman = infrastructure::remote::podman_binary().map_err(|error| error.message)?;
            infrastructure::podman::stop_managed_machine(&podman)
                .map_err(|error| format!("{}: {}", error.code, error.message))?;
            infrastructure::service_shutdown::signal()
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
    fn service_maintenance_modes_are_closed_and_local() {
        assert_eq!(
            parse_mode([OsString::from("--validate-installation")].into_iter()),
            Ok(Mode::ValidateInstallation)
        );
        assert_eq!(parse_mode(std::iter::empty()), Ok(Mode::Service));
        assert_eq!(
            parse_mode([OsString::from("--stop-managed-machine")].into_iter()),
            Ok(Mode::StopManagedMachine)
        );
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
