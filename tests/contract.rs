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
