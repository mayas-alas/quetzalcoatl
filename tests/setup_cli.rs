use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gnx-setup"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn default_transport_remains_one_report() {
    let output = run(&["--apply", "SECRET_CANARY"]);
    assert_eq!(output.status.code(), Some(1));
    let report: gnx::report::Report = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report.code, "INVALID_ARGUMENT");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("SECRET_CANARY"));
    assert!(output.stderr.is_empty());
}

#[test]
fn stream_preserves_failure_without_echoing_arguments() {
    let output = run(&["--json-progress", "--apply", "SECRET_CANARY"]);
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stdout).unwrap();
    let events: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["phase"], "started");
    assert_eq!(events[1]["phase"], "completed");
    assert_eq!(events[1]["state"], "FAILED");
    assert_eq!(events[1]["exit_code"], 1);
    assert_eq!(events[1]["apply_available"], false);
    assert!(!text.contains("SECRET_CANARY"));
    assert!(output.stderr.is_empty());
}

#[test]
fn stream_and_report_have_identical_preflight_outcomes() {
    let ordinary = run(&["--check"]);
    let streaming = run(&["--json-progress", "--check"]);
    let report: gnx::report::Report = serde_json::from_slice(&ordinary.stdout).unwrap();
    let text = String::from_utf8(streaming.stdout).unwrap();
    let completed: serde_json::Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
    assert_eq!(ordinary.status.code(), streaming.status.code());
    assert_eq!(
        completed["state"],
        serde_json::to_value(report.state).unwrap()
    );
    assert_eq!(completed["exit_code"], streaming.status.code().unwrap());
    assert_eq!(completed["apply_available"], false);
}
