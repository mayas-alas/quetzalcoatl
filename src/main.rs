use quetzalcoatl_gnx::{protocol::{parse_operation, Report}, CLI_EXE, PRODUCT};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--help"] || args.is_empty() { println!("{PRODUCT}\n{CLI_EXE} <check|status|wait>\nRead-only authenticated local queries. No elevation required."); return; }
    #[cfg(windows)]
    if args == ["--service"] { if let Err(e) = quetzalcoatl_gnx::windows::service_dispatch() { eprintln!("{e}"); std::process::exit(1); } return; }
    #[cfg(windows)]
    if args == ["wait"] {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3600);
        loop {
            match quetzalcoatl_gnx::windows::query(quetzalcoatl_gnx::protocol::Operation::Status) {
                Ok(report) => {
                    println!("{}: {}", report.state, report.detail);
                    if report.verified { return; }
                    if report.state == "degraded" || report.state == "stopped" { std::process::exit(3); }
                }
                Err(e) => { eprintln!("{e}"); std::process::exit(2); }
            }
            if std::time::Instant::now() >= deadline { eprintln!("Timed out waiting for readiness."); std::process::exit(3); }
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }
    let result: Result<Report, String> = parse_operation(&args).and_then(|op| {
        #[cfg(windows)] { quetzalcoatl_gnx::windows::query(op) }
        #[cfg(not(windows))] { let _ = op; Err("Windows host required.".into()) }
    });
    match result { Ok(report) => { println!("{}", serde_json::to_string_pretty(&report).unwrap()); if !report.verified { std::process::exit(3); } }, Err(e) => { eprintln!("{e}"); std::process::exit(2); } }
}
