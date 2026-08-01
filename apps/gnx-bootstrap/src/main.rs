#[cfg(windows)]
mod dependencies;
mod exit_codes;
#[cfg(windows)]
mod host;

#[cfg(windows)]
mod recovery;
mod report;
#[cfg(windows)]
mod windows;

use std::env;
use std::ffi::OsString;

use exit_codes::{
    FEATURES_DISABLED, HOST_RESOURCES_INSUFFICIENT, NOT_ELEVATED, OK, OPERATIONAL_ERROR,
    PODMAN_INCOMPATIBLE, REBOOT_PENDING, REBOOT_REQUIRED, USAGE, VIRTUALIZATION_DISABLED,
    WINDOWS_INCOMPATIBLE, WSL_UNAVAILABLE,
};
use report::{Check, Report, Status};

#[cfg(windows)]
type CheckSpec = (&'static str, i32, fn() -> host::checks::Verdict);

enum Format {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestedOperation {
    Install,
    Repair,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Validate,
    PrepareWsl,
    InstallWsl,
    InstallPodman,
}

struct Options {
    mode: Mode,
    format: Format,
    operation: RequestedOperation,
}

fn usage() -> &'static str {
    "Usage: gnx-bootstrap [prepare-wsl|install-wsl|install-podman] [--operation install|repair] [--format human|json]"
}

fn parse_args_from(mut args: impl Iterator<Item = OsString>) -> Result<Options, ()> {
    let first = args.next();
    let mode = match first.as_ref() {
        Some(arg) if arg == "prepare-wsl" => Mode::PrepareWsl,
        Some(arg) if arg == "install-wsl" => Mode::InstallWsl,
        Some(arg) if arg == "install-podman" => Mode::InstallPodman,
        Some(arg) if arg == "--help" || arg == "-h" => {
            if args.next().is_none() {
                println!("{}", usage());
                std::process::exit(OK);
            }
            return Err(());
        }
        Some(arg) if arg == "--format" || arg == "--operation" => {
            let mut remaining = vec![arg.clone()];
            remaining.extend(args);
            let (format, operation) = parse_options(remaining.into_iter())?;
            return Ok(Options {
                mode: Mode::Validate,
                format,
                operation,
            });
        }
        Some(_) => return Err(()),
        None => {
            return Ok(Options {
                mode: Mode::Validate,
                format: Format::Human,
                operation: RequestedOperation::Install,
            });
        }
    };

    let (format, operation) = parse_options(args)?;
    Ok(Options {
        mode,
        format,
        operation,
    })
}

fn parse_options(
    mut args: impl Iterator<Item = OsString>,
) -> Result<(Format, RequestedOperation), ()> {
    let mut format = Format::Human;
    let mut operation = RequestedOperation::Install;
    let mut saw_format = false;
    let mut saw_operation = false;
    while let Some(option) = args.next() {
        if option == "--format" && !saw_format {
            format = match args.next() {
                Some(value) if value == "human" => Format::Human,
                Some(value) if value == "json" => Format::Json,
                _ => return Err(()),
            };
            saw_format = true;
        } else if option == "--operation" && !saw_operation {
            operation = match args.next() {
                Some(value) if value == "install" => RequestedOperation::Install,
                Some(value) if value == "repair" => RequestedOperation::Repair,
                _ => return Err(()),
            };
            saw_operation = true;
        } else {
            return Err(());
        }
    }
    Ok((format, operation))
}

fn parse_args() -> Result<Options, ()> {
    parse_args_from(env::args_os().skip(1))
}

fn emit(format: &Format, report: &Report) {
    match format {
        Format::Human => {
            if let Some(profile) = &report.host_profile {
                println!("host_profile: {}", host::profile::summary(profile));
                for warning in &profile.warnings {
                    println!("host_profile_warning: {warning}");
                }
            }
            for check in &report.checks {
                let status = match check.status {
                    Status::Pass => "pass",
                    Status::Fail => "fail",
                    Status::Error => "error",
                    Status::RebootRequired => "reboot_required",
                };
                println!("{}: {} - {}", check.id, status, check.message);
            }
        }
        Format::Json => println!(
            "{}",
            serde_json::to_string(report).expect("serializable report")
        ),
    }
}

#[cfg(windows)]
fn append_host_profile(
    format: &Format,
    report: &mut Report,
    operation: RequestedOperation,
) -> Result<(), i32> {
    match host::profile::detect_and_store(operation == RequestedOperation::Repair) {
        Ok(profile) => {
            let supported = profile.supported;
            let message = host::profile::summary(&profile);
            report.host_profile = Some(profile);
            report.checks.push(Check {
                id: "host_resources",
                status: if supported {
                    Status::Pass
                } else {
                    Status::Fail
                },
                message,
            });
            if supported {
                Ok(())
            } else {
                report.status = Status::Fail;
                report.exit_code = HOST_RESOURCES_INSUFFICIENT;
                emit(format, report);
                Err(HOST_RESOURCES_INSUFFICIENT)
            }
        }
        Err(error) => {
            report.status = Status::Error;
            report.exit_code = OPERATIONAL_ERROR;
            report.checks.push(Check {
                id: "host_resources",
                status: Status::Error,
                message: error.message().into(),
            });
            emit(format, report);
            Err(OPERATIONAL_ERROR)
        }
    }
}

#[cfg(windows)]
fn run(format: &Format) -> i32 {
    let mut report = Report::new();
    let checks: [CheckSpec; 7] = [
        (
            "windows_host",
            WINDOWS_INCOMPATIBLE,
            host::checks::windows_host,
        ),
        ("elevation", NOT_ELEVATED, host::checks::elevation),
        (
            "virtualization",
            VIRTUALIZATION_DISABLED,
            host::checks::virtualization,
        ),
        (
            "windows_features",
            FEATURES_DISABLED,
            host::checks::windows_features,
        ),
        (
            "pending_reboot",
            REBOOT_PENDING,
            host::checks::pending_reboot,
        ),
        ("wsl", WSL_UNAVAILABLE, host::checks::wsl),
        ("podman_msi", PODMAN_INCOMPATIBLE, host::checks::podman_msi),
    ];
    for (id, fail_code, check) in checks {
        let verdict = check();
        let (status, message, code) = match verdict {
            host::checks::Verdict::Pass(message) => (Status::Pass, message, OK),
            host::checks::Verdict::Fail(message) => (Status::Fail, message, fail_code),
            host::checks::Verdict::Error(message) => (Status::Error, message, OPERATIONAL_ERROR),
            host::checks::Verdict::Reboot(message) => {
                (Status::RebootRequired, message, REBOOT_REQUIRED)
            }
        };
        report.checks.push(Check {
            id,
            status,
            message,
        });
        if code != OK {
            report.status = if code == OPERATIONAL_ERROR {
                Status::Error
            } else {
                status
            };
            report.exit_code = code;
            emit(format, &report);
            return code;
        }
    }
    if let Err(code) = append_host_profile(format, &mut report, RequestedOperation::Install) {
        return code;
    }
    emit(format, &report);
    OK
}

#[cfg(windows)]
fn prepare_wsl(format: &Format, operation: RequestedOperation) -> i32 {
    let mut report = Report::new();
    let checks: [CheckSpec; 4] = [
        (
            "windows_host",
            WINDOWS_INCOMPATIBLE,
            host::checks::windows_host,
        ),
        ("elevation", NOT_ELEVATED, host::checks::elevation),
        (
            "virtualization",
            VIRTUALIZATION_DISABLED,
            host::checks::virtualization,
        ),
        (
            "pending_reboot",
            REBOOT_PENDING,
            host::checks::pending_reboot,
        ),
    ];
    for (id, fail_code, check) in checks {
        let verdict = check();
        let (status, message, code) = match verdict {
            host::checks::Verdict::Pass(message) => (Status::Pass, message, OK),
            host::checks::Verdict::Fail(message) => (Status::Fail, message, fail_code),
            host::checks::Verdict::Error(message) => (Status::Error, message, OPERATIONAL_ERROR),
            host::checks::Verdict::Reboot(message) => {
                (Status::RebootRequired, message, REBOOT_REQUIRED)
            }
        };
        report.checks.push(Check {
            id,
            status,
            message,
        });
        if code != OK {
            report.status = status;
            report.exit_code = code;
            emit(format, &report);
            return code;
        }
    }

    if let Err(code) = append_host_profile(format, &mut report, operation) {
        return code;
    }

    let verdict = host::checks::prepare_windows_features();
    let (status, message, code) = match verdict {
        host::checks::Verdict::Pass(message) => (Status::Pass, message, OK),
        host::checks::Verdict::Fail(message) => (Status::Fail, message, FEATURES_DISABLED),
        host::checks::Verdict::Error(message) => (Status::Error, message, OPERATIONAL_ERROR),
        host::checks::Verdict::Reboot(message) => {
            (Status::RebootRequired, message, REBOOT_REQUIRED)
        }
    };
    report.checks.push(Check {
        id: "windows_features",
        status,
        message,
    });
    report.status = status;
    report.exit_code = code;
    emit(format, &report);
    code
}

#[cfg(windows)]
fn install_dependency(
    format: &Format,
    selected: dependencies::Dependency,
    operation: RequestedOperation,
) -> i32 {
    let mut report = Report::new();
    let check_id = selected.check_id();
    match dependencies::install(selected, operation) {
        Ok(dependencies::InstallOutcome::Success(message)) => {
            report.checks.push(Check {
                id: check_id,
                status: Status::Pass,
                message,
            });
            emit(format, &report);
            OK
        }
        Ok(dependencies::InstallOutcome::Reboot(message)) => {
            report.status = Status::RebootRequired;
            report.exit_code = REBOOT_REQUIRED;
            report.checks.push(Check {
                id: check_id,
                status: Status::RebootRequired,
                message,
            });
            emit(format, &report);
            REBOOT_REQUIRED
        }
        Err(error) => {
            report.status = if error.exit_code >= OPERATIONAL_ERROR {
                Status::Error
            } else {
                Status::Fail
            };
            report.exit_code = error.exit_code;
            report.checks.push(Check {
                id: check_id,
                status: report.status,
                message: format!("{}: {}", error.code, error.message),
            });
            emit(format, &report);
            error.exit_code
        }
    }
}

#[cfg(not(windows))]
fn prepare_wsl(format: &Format, _operation: RequestedOperation) -> i32 {
    run(format)
}

#[cfg(not(windows))]
fn run(format: &Format) -> i32 {
    let mut report = Report::new();
    report.status = Status::Fail;
    report.exit_code = WINDOWS_INCOMPATIBLE;
    report.checks.push(Check {
        id: "windows_host",
        status: Status::Fail,
        message: "requires Windows 11 x64 build 22000 or newer".into(),
    });
    emit(format, &report);
    WINDOWS_INCOMPATIBLE
}

#[cfg(not(windows))]
fn install_dependency(format: &Format, _selected: (), _operation: RequestedOperation) -> i32 {
    run(format)
}

fn main() {
    let options = match parse_args() {
        Ok(options) => options,
        Err(()) => {
            eprintln!("{}", usage());
            std::process::exit(USAGE);
        }
    };
    let code = match options.mode {
        Mode::Validate => run(&options.format),
        Mode::PrepareWsl => prepare_wsl(&options.format, options.operation),
        #[cfg(windows)]
        Mode::InstallWsl => install_dependency(
            &options.format,
            dependencies::Dependency::Wsl,
            options.operation,
        ),
        #[cfg(windows)]
        Mode::InstallPodman => install_dependency(
            &options.format,
            dependencies::Dependency::Podman,
            options.operation,
        ),
        #[cfg(not(windows))]
        Mode::InstallWsl | Mode::InstallPodman => {
            install_dependency(&options.format, (), options.operation)
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(values: &[&str]) -> Result<Options, ()> {
        parse_args_from(values.iter().map(|value| OsString::from(*value)))
    }

    #[test]
    fn dependency_modes_accept_only_the_optional_format() {
        assert!(parse(&["install-wsl"]).is_ok());
        assert!(parse(&["install-podman", "--format", "json"]).is_ok());
        assert!(
            parse(&[
                "install-podman",
                "--operation",
                "repair",
                "--format",
                "json"
            ])
            .is_ok()
        );
        assert!(parse(&["install-wsl", "--operation", "upgrade"]).is_err());
        assert!(parse(&["install-wsl", "--operation", "uninstall"]).is_err());
        assert!(
            parse(&[
                "install-wsl",
                "--operation",
                "repair",
                "--operation",
                "repair"
            ])
            .is_err()
        );
        assert!(parse(&["install-podman", "extra"]).is_err());
    }
}
