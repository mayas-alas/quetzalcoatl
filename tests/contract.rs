use std::process::Command;

#[test]
fn missing_config_has_a_stable_failure_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_gnx"))
        .args(["access", "dns"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "FAILED CONFIG_READ\n"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn secrets_have_no_command_line_position() {
    let marker = "GNX-NONSECRET-INPUT-MARKER";
    let output = Command::new(env!("CARGO_BIN_EXE_gnx"))
        .args(["access", "configure", marker])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "FAILED ARGUMENTS\n"
    );
    assert!(output.stdout.is_empty());
}
