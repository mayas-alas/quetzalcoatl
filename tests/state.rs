use gnx::{adapter::filesystem::Filesystem, config::Config, port::state::StateStore};
fn temp() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "gnx-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
#[test]
fn transaction_lock_and_atomic_promotion() {
    let root = temp();
    let store = Filesystem { root: root.clone() };
    assert_eq!(store.current().unwrap(), None);
    assert!(!root.exists());
    let c = Config::parse(include_str!("../gnx.toml")).unwrap();
    let guard = store.acquire().unwrap();
    assert!(store.acquire().is_err());
    let revision = "1".repeat(64);
    store.stage(&c, &revision).unwrap();
    assert!(store.interrupted().unwrap());
    assert_eq!(store.current().unwrap(), None);
    store.promote().unwrap();
    assert_eq!(store.current().unwrap(), Some(revision.clone()));
    assert_eq!(store.previous().unwrap(), Some(c.clone()));
    // A staged next candidate cannot change either part of the committed record.
    assert!(!store.interrupted().unwrap());
    let mut next = c.clone();
    next.node = "next".into();
    store.stage(&next, &"2".repeat(64)).unwrap();
    store.abort().unwrap();
    assert_eq!(store.current().unwrap(), Some(revision));
    assert_eq!(store.previous().unwrap(), Some(c));
    drop(guard);
    assert!(store.acquire().is_ok());
    std::fs::remove_dir_all(root).unwrap();
}
#[cfg(unix)]
#[test]
fn state_directory_and_files_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp();
    let store = Filesystem { root: root.clone() };
    let guard = store.acquire().unwrap();
    store
        .stage(
            &Config::parse(include_str!("../gnx.toml")).unwrap(),
            &"1".repeat(64),
        )
        .unwrap();
    assert_eq!(
        std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(root.join("candidate.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    drop(guard);
    std::fs::remove_dir_all(root).unwrap();
}
