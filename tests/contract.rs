use std::process::Command;

#[test]
fn missing_config_has_a_stable_failure_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_gnx"))
        .arg("doctor")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "FAILED CONFIG_REQUIRED\n"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn misplaced_key_material_is_not_echoed() {
    let example = "GNX-NONSECRET-INPUT-MARKER";
    let output = Command::new(env!("CARGO_BIN_EXE_gnx"))
        .args(["access", "configure", example])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "FAILED ARGUMENTS\n"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn configure_rejects_redirected_input_before_mutation() {
    let config =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/access/access.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_gnx"))
        .args(["access", "configure", "--config"])
        .arg(config)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(6));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "FAILED ACCESS_TERMINAL_REQUIRED\n"
    );
    assert!(output.stdout.is_empty());
}
#[test]
fn credentials_cannot_be_revealed_into_captured_streams() {
    for account in ["control", "compute"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_gnx"))
            .args(["credentials", account])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"FAILED CREDENTIAL_TERMINAL_REQUIRED\n");
    }
}
