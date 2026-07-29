#[cfg(windows)]
mod checks;
#[cfg(windows)]
mod dependency;
mod exit_codes;
mod host_profile;
#[cfg(windows)]
mod journal;
mod model;
#[cfg(windows)]
mod staging;
#[cfg(windows)]
mod windows;

use std::env;
use std::ffi::OsString;

use exit_codes::*;
use model::{Check, Report, Status};

#[cfg(windows)]
type CheckSpec = (&'static str, i32, fn() -> checks::Verdict);

enum Format {
    Human,
    Json,
}

enum Mode {
    Validate,
    PrepareWsl,
    InstallWsl,
    InstallPodman,
}

struct Options {
    mode: Mode,
    format: Format,
}

fn usage() -> &'static str {
    "Usage: gnx-host-preflight [prepare-wsl|install-wsl|install-podman] [--format human|json]"
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
        Some(arg) if arg == "--format" => {
            return parse_format(args).map(|format| Options {
                mode: Mode::Validate,
                format,
            });
        }
        Some(_) => return Err(()),
        None => {
            return Ok(Options {
                mode: Mode::Validate,
                format: Format::Human,
            });
        }
    };

    let format = match args.next() {
        None => Format::Human,
        Some(arg) if arg == "--format" => parse_format(args)?,
        Some(_) => return Err(()),
    };
    Ok(Options { mode, format })
}

fn parse_format(mut args: impl Iterator<Item = OsString>) -> Result<Format, ()> {
    match args.next() {
        Some(value) if value == "human" && args.next().is_none() => Ok(Format::Human),
        Some(value) if value == "json" && args.next().is_none() => Ok(Format::Json),
        _ => Err(()),
    }
}

fn parse_args() -> Result<Options, ()> {
    parse_args_from(env::args_os().skip(1))
}

fn emit(format: &Format, report: &Report) {
    match format {
        Format::Human => {
            if let Some(profile) = &report.host_profile {
                println!("host_profile: {}", host_profile::summary(profile));
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
fn append_host_profile(format: &Format, report: &mut Report) -> Result<(), i32> {
    match host_profile::detect_and_store() {
        Ok(profile) => {
            let supported = profile.supported;
            let message = host_profile::summary(&profile);
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
        ("windows_host", WINDOWS_INCOMPATIBLE, checks::windows_host),
        ("elevation", NOT_ELEVATED, checks::elevation),
        (
            "virtualization",
            VIRTUALIZATION_DISABLED,
            checks::virtualization,
        ),
        (
            "windows_features",
            FEATURES_DISABLED,
            checks::windows_features,
        ),
        ("pending_reboot", REBOOT_PENDING, checks::pending_reboot),
        ("wsl", WSL_UNAVAILABLE, checks::wsl),
        ("podman_msi", PODMAN_INCOMPATIBLE, checks::podman_msi),
    ];
    for (id, fail_code, check) in checks {
        let verdict = check();
        let (status, message, code) = match verdict {
            checks::Verdict::Pass(message) => (Status::Pass, message, OK),
            checks::Verdict::Fail(message) => (Status::Fail, message, fail_code),
            checks::Verdict::Error(message) => (Status::Error, message, OPERATIONAL_ERROR),
            checks::Verdict::Reboot(message) => (Status::RebootRequired, message, REBOOT_REQUIRED),
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
    if let Err(code) = append_host_profile(format, &mut report) {
        return code;
    }
    emit(format, &report);
    OK
}

#[cfg(windows)]
fn prepare_wsl(format: &Format) -> i32 {
    let mut report = Report::new();
    let checks: [CheckSpec; 4] = [
        ("windows_host", WINDOWS_INCOMPATIBLE, checks::windows_host),
        ("elevation", NOT_ELEVATED, checks::elevation),
        (
            "virtualization",
            VIRTUALIZATION_DISABLED,
            checks::virtualization,
        ),
        ("pending_reboot", REBOOT_PENDING, checks::pending_reboot),
    ];
    for (id, fail_code, check) in checks {
        let verdict = check();
        let (status, message, code) = match verdict {
            checks::Verdict::Pass(message) => (Status::Pass, message, OK),
            checks::Verdict::Fail(message) => (Status::Fail, message, fail_code),
            checks::Verdict::Error(message) => (Status::Error, message, OPERATIONAL_ERROR),
            checks::Verdict::Reboot(message) => (Status::RebootRequired, message, REBOOT_REQUIRED),
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

    if let Err(code) = append_host_profile(format, &mut report) {
        return code;
    }

    let verdict = checks::prepare_windows_features();
    let (status, message, code) = match verdict {
        checks::Verdict::Pass(message) => (Status::Pass, message, OK),
        checks::Verdict::Fail(message) => (Status::Fail, message, FEATURES_DISABLED),
        checks::Verdict::Error(message) => (Status::Error, message, OPERATIONAL_ERROR),
        checks::Verdict::Reboot(message) => (Status::RebootRequired, message, REBOOT_REQUIRED),
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
fn install_dependency(format: &Format, selected: dependency::Dependency) -> i32 {
    let mut report = Report::new();
    let check_id = selected.check_id();
    match dependency::install(selected) {
        Ok(dependency::InstallOutcome::Success(message)) => {
            report.checks.push(Check {
                id: check_id,
                status: Status::Pass,
                message,
            });
            emit(format, &report);
            OK
        }
        Ok(dependency::InstallOutcome::Reboot(message)) => {
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
fn prepare_wsl(format: &Format) -> i32 {
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
fn install_dependency(format: &Format, _selected: ()) -> i32 {
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
        Mode::PrepareWsl => prepare_wsl(&options.format),
        #[cfg(windows)]
        Mode::InstallWsl => install_dependency(&options.format, dependency::Dependency::Wsl),
        #[cfg(windows)]
        Mode::InstallPodman => install_dependency(&options.format, dependency::Dependency::Podman),
        #[cfg(not(windows))]
        Mode::InstallWsl | Mode::InstallPodman => install_dependency(&options.format, ()),
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
        assert!(parse(&["install-podman", "extra"]).is_err());
    }
}
