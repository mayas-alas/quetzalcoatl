use gnx::{
    adapter::{filesystem::Filesystem, release::PinnedRelease},
    config::Config,
    port::state::StateStore,
};

#[test]
fn release_and_public_names_are_consistent() {
    let release = PinnedRelease::embedded().unwrap();
    assert_eq!(release.version, env!("CARGO_PKG_VERSION"));
    let c = Config::parse(include_str!("../gnx.toml")).unwrap();
    let zone = gnx::adapter::coredns::zone(&c, "100.64.0.1");
    assert!(zone.contains("proxmox IN A 100.64.0.1"));
    let tls = gnx::adapter::caddy::render(&c, "100.64.0.1", "compute", "10.90.0.2");
    assert!(tls.contains("https://proxmox.gnx"));
    assert!(tls.contains("tls_trust_pool file"));
    assert!(!tls.contains("tls_insecure_skip_verify"));
}

#[test]
fn persisted_revision_does_not_change_with_the_running_release() {
    let root = std::env::temp_dir().join(format!("gnx-release-test-{}", std::process::id()));
    let store = Filesystem { root: root.clone() };
    let guard = store.acquire().unwrap();
    let old_release_revision = "0".repeat(64);
    std::fs::write(root.join("last-valid.revision"), &old_release_revision).unwrap();
    assert_eq!(store.current().unwrap(), Some(old_release_revision));
    drop(guard);
    std::fs::remove_dir_all(root).unwrap();
}
