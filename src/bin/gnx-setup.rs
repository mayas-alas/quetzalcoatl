use gnx::report::{Report, State};

fn invalid() -> Report {
    Report::new(
        "setup",
        State::Failed,
        "INVALID_ARGUMENT",
        Some("Use gnx-setup --check."),
    )
}

fn run(args: &[String]) -> Report {
    if args != ["--check"] {
        return invalid();
    }
    #[cfg(windows)]
    {
        gnx::app::setup::preflight(Some(
            &gnx::adapter::windows::setup::WindowsSetupHost,
        ))
    }
    #[cfg(not(windows))]
    {
        gnx::app::setup::preflight(None)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let report = run(&args);
    println!("{}", serde_json::to_string(&report).unwrap());
    std::process::exit(report.exit());
}
