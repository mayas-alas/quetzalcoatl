use runtime_control::protocol::parse_operation;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--help"] || args.is_empty() { println!("runtime <check|status>\nRead-only authenticated local queries. No elevation required."); return; }
    #[cfg(windows)]
    if args == ["--service"] { if let Err(e) = runtime_control::windows::service_dispatch() { eprintln!("{e}"); std::process::exit(1); } return; }
    let result = parse_operation(&args).and_then(|op| {
        #[cfg(windows)] { runtime_control::windows::query(op) }
        #[cfg(not(windows))] { let _ = op; Err("Windows host required.".into()) }
    });
    match result { Ok(report) => { println!("{}", serde_json::to_string_pretty(&report).unwrap()); if !report.verified { std::process::exit(3); } }, Err(e) => { eprintln!("{e}"); std::process::exit(2); } }
}
