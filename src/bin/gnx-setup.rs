use gnx::{
    domain::setup::BundleInput,
    report::{Report, State},
};
use std::{
    io::{self, Write},
    path::PathBuf,
};

// This wire projection deliberately excludes paths, arguments, child output,
// credentials, and arbitrary adapter error strings. UI clients only render it.
#[derive(serde::Serialize)]
struct Progress<'a> {
    schema: u32,
    operation: &'static str,
    phase: &'static str,
    code: &'static str,
    state: Option<&'a State>,
    exit_code: Option<i32>,
    apply_available: bool,
}

fn progress<'a>(result: Option<&'a Report>, operation: &'static str) -> Progress<'a> {
    let prefix = if operation == "setup-provision" {
        "SETUP_PROVISION"
    } else {
        "SETUP_CHECK"
    };
    Progress {
        schema: 1,
        operation,
        phase: if result.is_some() {
            "completed"
        } else {
            "started"
        },
        code: match result.map(|r| &r.state) {
            None => {
                if operation == "setup-provision" {
                    "SETUP_PROVISION_STARTED"
                } else {
                    "SETUP_CHECK_STARTED"
                }
            }
            Some(State::Ready) => {
                if prefix == "SETUP_PROVISION" {
                    "SETUP_PROVISION_COMPLETED"
                } else {
                    "SETUP_CHECK_COMPLETED"
                }
            }
            Some(State::Failed) => {
                if prefix == "SETUP_PROVISION" {
                    "SETUP_PROVISION_FAILED"
                } else {
                    "SETUP_CHECK_FAILED"
                }
            }
            Some(State::ActionRequired) => {
                if prefix == "SETUP_PROVISION" {
                    "SETUP_PROVISION_ACTION_REQUIRED"
                } else {
                    "SETUP_CHECK_ACTION_REQUIRED"
                }
            }
        },
        state: result.map(|r| &r.state),
        exit_code: result.map(Report::exit),
        apply_available: false,
    }
}

fn write_json(out: &mut impl Write, value: &impl serde::Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)?;
    out.flush()
}

fn report(code: &str, state: State, action: &str) -> Report {
    Report::new("setup", state, code, Some(action))
}

fn parse_bundle(args: &[String]) -> Result<Option<BundleInput>, &'static str> {
    if args == ["--check"] {
        return Ok(None);
    }
    if args.len() != 9
        || !matches!(args[0].as_str(), "--check" | "--provision")
        || args[1] != "--bundle"
        || args[3] != "--manifest-sha256"
        || args[5] != "--rootfs"
        || args[7] != "--rootfs-sha256"
    {
        return Err("INVALID_ARGUMENT");
    }
    Ok(Some(BundleInput {
        bundle: PathBuf::from(&args[2]),
        manifest_sha256: args[4].clone(),
        rootfs: PathBuf::from(&args[6]),
        rootfs_sha256: args[8].clone(),
    }))
}

fn run(args: &[String]) -> Report {
    let bundle = match parse_bundle(args) {
        Ok(value) => value,
        Err(code) => {
            return report(
                code,
                State::Failed,
                "Use gnx-setup --check [bundle options] or --provision with all bundle options.",
            )
        }
    };
    #[cfg(windows)]
    {
        if args[0] == "--provision" {
            return gnx::app::setup::provision(
                Some(&gnx::adapter::windows::setup::WindowsSetupHost),
                bundle.as_ref().expect("provision requires bundle options"),
            );
        }
        gnx::app::setup::preflight(
            Some(&gnx::adapter::windows::setup::WindowsSetupHost),
            bundle.as_ref(),
        )
    }
    #[cfg(not(windows))]
    {
        let _ = bundle;
        gnx::app::setup::preflight(None, None)
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let streaming = args.first().is_some_and(|arg| arg == "--json-progress");
    if streaming {
        args.remove(0);
    }
    let operation = if args.first().is_some_and(|arg| arg == "--provision") {
        "setup-provision"
    } else {
        "setup-check"
    };
    let mut out = io::stdout().lock();
    if streaming && write_json(&mut out, &progress(None, operation)).is_err() {
        std::process::exit(1);
    }
    let result = run(&args);
    let written = if streaming {
        write_json(&mut out, &progress(Some(&result), operation))
    } else {
        write_json(&mut out, &result)
    };
    if written.is_err() {
        std::process::exit(1);
    }
    std::process::exit(result.exit());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_excludes_untrusted_report_details() {
        for state in [State::Ready, State::Failed, State::ActionRequired] {
            let result = report("SECRET_CANARY", state, "C:\\private\\SECRET_CANARY");
            let event = serde_json::to_value(progress(Some(&result), "setup-check")).unwrap();
            assert!(!event.to_string().contains("SECRET_CANARY"));
            assert_eq!(event["exit_code"], result.exit());
            assert_eq!(event["apply_available"], false);
        }
    }

    #[test]
    fn provision_requires_all_trust_inputs() {
        for args in [
            vec![],
            vec!["--provision"],
            vec!["--provision", "--password", "canary"],
        ] {
            let result = run(&args.into_iter().map(String::from).collect::<Vec<_>>());
            assert_eq!(result.code, "INVALID_ARGUMENT");
            assert!(!serde_json::to_string(&result).unwrap().contains("canary"));
        }
        let args = [
            "--provision",
            "--bundle",
            "bundle",
            "--manifest-sha256",
            "hash",
            "--rootfs",
            "rootfs",
            "--rootfs-sha256",
            "hash",
        ]
        .map(String::from);
        assert!(parse_bundle(&args).unwrap().is_some());
    }
}
