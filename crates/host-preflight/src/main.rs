#[cfg(windows)]
mod checks;
mod exit_codes;
mod model;
#[cfg(windows)]
mod windows;

use std::env;
use std::process::ExitCode;

use exit_codes::*;
use model::{Check, Report, Status};

enum Format {
    Human,
    Json,
}

fn usage() -> &'static str {
    "Usage: gnx-host-preflight [--format human|json]"
}

fn parse_args() -> Result<Format, ()> {
    let mut args = env::args_os().skip(1);
    match args.next() {
        None => Ok(Format::Human),
        Some(arg) if arg == "--help" || arg == "-h" => {
            if args.next().is_none() {
                println!("{}", usage());
                std::process::exit(OK);
            }
            Err(())
        }
        Some(arg) if arg == "--format" => match (args.next(), args.next()) {
            (Some(value), None) if value == "human" => Ok(Format::Human),
            (Some(value), None) if value == "json" => Ok(Format::Json),
            _ => Err(()),
        },
        _ => Err(()),
    }
}

fn emit(format: &Format, report: &Report) {
    match format {
        Format::Human => {
            for check in &report.checks {
                let status = match check.status {
                    Status::Pass => "pass",
                    Status::Fail => "fail",
                    Status::Error => "error",
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

fn main() -> ExitCode {
    let format = match parse_args() {
        Ok(format) => format,
        Err(()) => {
            eprintln!("{}", usage());
            return ExitCode::from(USAGE as u8);
        }
    };
    ExitCode::from(run(&format) as u8)
}
