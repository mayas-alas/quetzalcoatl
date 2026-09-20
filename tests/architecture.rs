#[test]
fn dependency_boundaries() {
    for dir in ["src/domain", "src/app"] {
        for f in std::fs::read_dir(dir).unwrap() {
            let s = std::fs::read_to_string(f.unwrap().path()).unwrap();
            for forbidden in ["crate::adapter", "std::process", "std::fs", "windows_sys"] {
                assert!(!s.contains(forbidden), "{dir} imports {forbidden}");
            }
        }
    }
}

#[test]
fn app_portal_is_embedded_and_published_under_control() {
    for path in [
        "runtime/control/app/index.html",
        "runtime/control/app/app.js",
        "runtime/control/app/style.css",
    ] {
        let content = std::fs::read_to_string(path).unwrap();
        assert!(!content.trim().is_empty(), "{path}");
    }
    let caddy = std::fs::read_to_string("src/adapter/caddy.rs").unwrap();
    assert!(caddy.contains("https://app.gnx"));
    assert!(caddy.contains("root * /gnx-app"));
    let linux = std::fs::read_to_string("src/adapter/linux.rs").unwrap();
    assert!(linux.contains("runtime/control/app/index.html"));
    assert!(linux.contains(":/gnx-app:ro"));
}

#[test]
fn managed_runtime_restart_policies_are_bounded() {
    let linux = std::fs::read_to_string("src/adapter/linux.rs").unwrap();
    assert!(linux.contains("StartLimitIntervalSec=300"));
    assert!(linux.contains("StartLimitBurst=5"));
    assert!(linux.contains("Restart=on-failure"));
    assert!(!linux.contains("Restart=always"));
    for path in [
        "runtime/access/gnx-access.container",
        "runtime/access/gnx-dns.container",
    ] {
        let unit = std::fs::read_to_string(path).unwrap();
        assert!(unit.contains("StartLimitIntervalSec=300"), "{path}");
        assert!(unit.contains("StartLimitBurst=5"), "{path}");
        assert!(unit.contains("Restart=on-failure"), "{path}");
        assert!(!unit.contains("Restart=always"), "{path}");
    }
}
