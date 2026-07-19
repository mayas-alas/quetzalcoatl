#[cfg(windows)]
mod checks;
mod exit_codes;
mod model;
#[cfg(windows)]
mod windows;

use std::env;
use std::ffi::OsString;

use exit_codes::*;
use model::{Check, Report, Status};

enum Format {
    Human,
    Json,
}

enum Mode {
    Validate,
    PrepareWsl,
}

struct Options {
    mode: Mode,
    format: Format,
}

fn usage() -> &'static str {
    "Usage: gnx-host-preflight [prepare-wsl] [--format human|json]"
}

fn parse_args_from(mut args: impl Iterator<Item = OsString>) -> Result<Options, ()> {
    let mode = match args.next() {
        Some(arg) if arg == "prepare-wsl" => Mode::PrepareWsl,
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
fn run(format: &Format) -> i32 {
    let mut report = Report::new();
    let checks: [(&str, i32, fn() -> checks::Verdict); 7] = [
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
                Status::Fail
            };
            report.exit_code = code;
            emit(format, &report);
            return code;
        }
    }
    emit(format, &report);
    OK
}

#[cfg(windows)]
fn prepare_wsl(format: &Format) -> i32 {
    let mut report = Report::new();
    let checks: [(&str, i32, fn() -> checks::Verdict); 4] = [
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
    };
    std::process::exit(code);
}
