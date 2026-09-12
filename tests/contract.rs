use gnx::{
    config::Config,
    report::{Report, State},
};
use std::process::Command;
#[test]
fn exit_contract() {
    for (s, e) in [
        (State::Ready, 0),
        (State::Failed, 1),
        (State::ActionRequired, 2),
    ] {
        assert_eq!(Report::new("status", s, "OK", None).exit(), e)
    }
}
#[test]
fn strict_intent() {
    let s = format!(
        "{}\n[[routes]]\nhostname = 'service.gnx'\nupstream = 'http://192.168.1.50:8080'\n",
        include_str!("../gnx.toml")
    );
    assert!(Config::parse(&s).is_ok());
    assert!(Config::parse(&s.replace("schema = 1", "schema = 99")).is_err());
    assert!(Config::parse(&s.replace("schema = 1", "secret = 'canary'\nschema = 1")).is_err());
    assert!(
        Config::parse(&s.replace("http://192.168.1.50:8080", "http://user:secret@host")).is_err()
    );
}
#[test]
fn invalid_cli_is_single_json() {
    let o = Command::new(env!("CARGO_BIN_EXE_gnx"))
        .arg("exec")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let r: Report = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(r.code, "INVALID_OPERATION");
}
#[test]
fn unknown_arguments_refused() {
    let o = Command::new(env!("CARGO_BIN_EXE_gnx"))
        .args(["apply", "--shell", "canary"])
        .output()
        .unwrap();
    let r: Report = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(r.code, "INVALID_ARGUMENT");
    assert!(!String::from_utf8(o.stdout).unwrap().contains("canary"));
}

#[test]
fn normal_cli_never_reads_secrets_from_environment() {
    let main = include_str!("../src/main.rs");
    assert!(!main.contains("std::env::var"));
    assert!(!main.contains("TAILSCALE_AUTHKEY"));
    assert!(!main.contains("GNX_COMPUTE_PASSWORD"));
}
