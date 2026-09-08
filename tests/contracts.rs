use gnx::{
    config::{Config, Route},
    plan,
};
use std::process::Command;

fn config() -> Config {
    Config::parse(include_str!("../config/gnx.example.toml")).unwrap()
}

#[test]
fn public_intent_is_small_and_strict() {
    let c = config();
    assert_eq!(c.schema, 1);
    assert_eq!(c.instance, "gnx");
    assert_eq!(c.node, "compute");

    let error = Config::parse(
        &(include_str!("../config/gnx.example.toml").to_owned()
            + "\npassword = 'CANARY-SECRET'\n"),
    )
    .unwrap_err();
    assert!(!error.to_string().contains("CANARY"));
}

#[test]
fn rejects_unsafe_networks_and_paths() {
    let original = config();
    for change in 0..6 {
        let mut c = original.clone();
        match change {
            0 => c.network.compute_ip = c.network.access_ip,
            1 => c.network.compute_ip = "10.90.0.1".parse().unwrap(),
            2 => c.network.subnet = "100.64.0.0/24".into(),
            3 => c.state_dir = "/var/lib/../etc".into(),
            4 => c.units_dir = format!("{}/units", c.state_dir),
            _ => c.network.identity_ip = Some("192.168.1.2".parse().unwrap()),
        }
        assert!(c.validate().is_err(), "invalid case {change}");
    }
}

#[test]
fn routes_cannot_inject_or_loop_into_control() {
    for upstream in [
        "http://localhost",
        "http://[::1]",
        "http://[::ffff:127.0.0.1]",
        "http://compute.gnx",
        "http://10.90.0.3:8006",
        "https://user:secret@example.com",
        "http://example.com/{evil}",
    ] {
        let mut c = config();
        c.routes.push(Route {
            hostname: "app.gnx".into(),
            upstream: upstream.into(),
        });
        assert!(c.validate().is_err(), "accepted {upstream}");
    }

    let mut c = config();
    c.routes.push(Route {
        hostname: "compute.gnx".into(),
        upstream: "http://192.168.1.50:8080".into(),
    });
    assert!(c.validate().is_err());
}

#[test]
fn generated_runtime_preserves_compute_and_control_boundaries() {
    let mut c = config();
    c.network.identity_ip = Some("100.64.0.10".parse().unwrap());
    let artifacts = plan::render(&c, "all").unwrap();

    let compute = &artifacts
        .iter()
        .find(|a| a.path.ends_with("gnx-compute.container"))
        .unwrap()
        .content;
    assert!(compute.contains(":/var/lib/pve-cluster"));
    assert!(compute.contains(":/var/lib/vz"));
    assert!(!compute.contains("PublishPort"));
    assert!(!compute.contains("Environment=PASSWORD"));

    let control = &artifacts
        .iter()
        .find(|a| a.path.ends_with("Caddyfile"))
        .unwrap()
        .content;
    assert!(control.contains("https://compute.gnx"));
    assert!(control.contains("tls_trust_pool"));
    assert!(!control.contains("insecure"));

    for cap in ["dns", "control"] {
        let unit = &artifacts
            .iter()
            .find(|a| a.path.ends_with(format!("gnx-{cap}.container")))
            .unwrap()
            .content;
        assert!(unit.contains("Network=gnx-access.container"));
        assert!(unit.contains("BindsTo=gnx-access.service"));
    }
}

#[test]
fn plan_is_idempotent_and_preserves_unmanaged_files() {
    let dir = tempfile::tempdir().unwrap();
    let c = config();
    let mut artifacts = plan::render(&c, "compute").unwrap();
    for (i, a) in artifacts.iter_mut().enumerate() {
        a.path = dir.path().join(i.to_string());
    }
    assert_eq!(plan::changes(&c, &artifacts).unwrap().len(), artifacts.len());
    for a in &artifacts {
        std::fs::write(&a.path, &a.content).unwrap();
    }
    assert!(plan::changes(&c, &artifacts).unwrap().is_empty());

    std::fs::write(&artifacts[0].path, "existing user's data").unwrap();
    assert_eq!(
        plan::changes(&c, &artifacts).unwrap_err().code,
        "FILE_OWNERSHIP"
    );
}

#[test]
fn cli_init_does_not_overwrite_and_plan_has_no_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intent.toml");
    let run = |action: &str| {
        Command::new(env!("CARGO_BIN_EXE_gnx"))
            .arg("--config")
            .arg(&path)
            .arg(action)
            .output()
            .unwrap()
    };
    assert!(run("init").status.success());
    let before = std::fs::read(&path).unwrap();
    assert!(!run("init").status.success());
    let output = run("plan");
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["details"]["applied"], false);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
