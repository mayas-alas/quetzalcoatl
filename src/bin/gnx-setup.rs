use gnx::{domain::setup::BundleInput, report::{Report, State}};
use std::path::PathBuf;

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
        Err(code) => return report(code, State::Failed, "Use gnx-setup --check [bundle options]."),
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = run(&args);
    println!("{}", serde_json::to_string(&result).unwrap());
    std::process::exit(result.exit());
}
