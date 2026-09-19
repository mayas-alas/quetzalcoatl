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

fn progress(result: Option<&Report>) -> Progress<'_> {
    Progress {
        schema: 1,
        operation: "setup-check",
        phase: if result.is_some() {
            "completed"
        } else {
            "started"
        },
        code: match result.map(|r| &r.state) {
            None => "SETUP_CHECK_STARTED",
            Some(State::Ready) => "SETUP_CHECK_COMPLETED",
            Some(State::Failed) => "SETUP_CHECK_FAILED",
            Some(State::ActionRequired) => "SETUP_CHECK_ACTION_REQUIRED",
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
        || args[0] != "--check"
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
                "Use gnx-setup --check [bundle options].",
            )
        }
    };
    #[cfg(windows)]
    {
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
    // The transport switch must precede the existing strict check arguments.
    let streaming = args.first().is_some_and(|arg| arg == "--json-progress");
    if streaming {
        args.remove(0);
    }
    let mut out = io::stdout().lock();
    if streaming && write_json(&mut out, &progress(None)).is_err() {
        std::process::exit(1);
    }
    let result = run(&args);
    let written = if streaming {
        write_json(&mut out, &progress(Some(&result)))
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
            let event = serde_json::to_value(progress(Some(&result))).unwrap();
            assert!(!event.to_string().contains("SECRET_CANARY"));
            assert_eq!(event["exit_code"], result.exit());
            assert_eq!(event["apply_available"], false);
        }
    }
}
