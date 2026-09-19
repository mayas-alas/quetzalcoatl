use gnx::{
    config::Config,
    domain::secret::{Secret, SecretKind},
    wire,
};
#[test]
fn secret_is_a_separate_bounded_frame() {
    let intent = include_str!("../gnx.toml");
    let secret = Secret::new(
        SecretKind::ComputePassword,
        b"canary-secret-123456".to_vec(),
    )
    .unwrap();
    let b = wire::encode("apply", intent, Some(&secret)).unwrap();
    let q = wire::decode(&b).unwrap();
    assert_eq!(q.secret.unwrap().value.as_slice(), secret.value.as_slice());
    assert_eq!(
        Config::parse(&q.intent).unwrap(),
        Config::parse(intent).unwrap()
    );
    assert!(wire::encode("status", intent, Some(&secret)).is_err());
}
#[test]
fn framing_rejects_all_truncations_and_trailing_bytes() {
    let b = wire::encode("plan", include_str!("../gnx.toml"), None).unwrap();
    for n in 0..b.len() {
        assert!(wire::decode(&b[..n]).is_err(), "accepted truncation {n}")
    }
    let mut bad = b.to_vec();
    bad.push(0);
    assert!(wire::decode(&bad).is_err());
    for (pos, value) in [
        (0, 0),
        (4, 255),
        (5, 255),
        (6, 1),
        (8, 255),
        (9, 255),
        (10, 255),
        (11, 255),
    ] {
        let mut bad = b.to_vec();
        bad[pos] = value;
        assert!(wire::decode(&bad).is_err(), "accepted corrupt {pos}");
    }
}
#[test]
fn dangerous_origins_are_refused() {
    let template = include_str!("../gnx.toml");
    for upstream in [
        "http://host/a",
        "http://host/?token=secret",
        "http://host/#secret",
        "http://user:pass@host",
        "http://host\\other",
        "http://host\"",
        "http://host%20",
        "file:///etc/passwd",
        "https://host\nfoo",
    ] {
        assert!(
            Config::parse(&template.replace("http://192.168.1.50:8080", upstream)).is_err(),
            "accepted {upstream}"
        );
    }
}
