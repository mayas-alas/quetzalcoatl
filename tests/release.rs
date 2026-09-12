use gnx::{
    adapter::{filesystem::Filesystem, release::PinnedRelease},
    config::Config,
    port::{runtime::Runtime, state::StateStore},
};
use std::process::Command;

#[test]
fn release_and_public_names_are_consistent() {
    Config::parse(include_str!("../config/gnx.example.toml")).unwrap();
    let release = PinnedRelease::embedded().unwrap();
    assert_eq!(release.version, env!("CARGO_PKG_VERSION"));
    let c = Config::parse(include_str!("../gnx.toml")).unwrap();
    let zone = gnx::adapter::coredns::zone(&c, "100.64.0.1");
    assert!(zone.contains("compute IN A 100.64.0.1"));
    assert!(zone.contains("app IN A 100.64.0.1"));
    let tls = gnx::adapter::caddy::render(&c, "100.64.0.1", "compute", "10.90.0.2");
    assert!(tls.contains("https://app.gnx"));
    assert!(tls.contains("root * /gnx-app"));
    assert!(tls.contains("intermediate_cn \"GNX Local Auth\""));
    assert!(tls.contains("root_cn \"GNX Root\""));
    assert!(tls.contains("tls_trust_pool file"));
    assert!(!tls.contains("tls_insecure_skip_verify"));
}

#[test]
fn windows_installer_publishes_the_cli_outside_the_service_runtime() {
    let setup = include_str!("../packaging/windows/setup.ps1");
    assert!(setup.contains("$cli=Join-Path $installRoot 'gnx.exe'"));
    assert!(setup.contains("Copy-Item \"$bundle/gnx.exe\" $cli -Force"));
    assert!(setup.contains("$bin=Join-Path $installRoot 'runtime'"));
}

#[test]
fn windows_release_build_requires_a_pinned_signing_authority() {
    let build = include_str!("../packaging/windows/build.ps1");
    let setup = include_str!("../packaging/windows/setup.ps1");
    let installer = include_str!("../src/bin/gnx-install.rs");
    assert!(build.contains("[Parameter(Mandatory)][string]$SigningKey"));
    assert!(build.contains("manifest.json.sig"));
    assert!(build.contains("release_serial=$releaseSerial"));
    assert!(build.contains("keyInfo.public_key -cne $trusted"));
    assert!(setup.contains("$manifest.signing_key_id"));
    assert!(installer.contains("release_auth::verify"));
    assert!(installer.contains("trusted-release.pub"));
    assert!(installer.contains("RELEASE_AUTHENTIC"));
    assert!(installer.contains("validate_release_artifacts"));
    assert!(installer.contains("ARTIFACT_MISMATCH"));
    assert!(installer.contains("MANIFEST_SCHEMA_INVALID"));
    assert!(installer.contains("next_action"));
    assert!(build.contains("Pinned installer rejected the signed release"));
    assert!(build.contains("$rootfsTarget"));
    assert!(build.contains("LastWriteTimeUtc=$epoch"));
}

#[test]
fn release_negative_matrix_is_executable_and_sanitized() {
    let verifier = include_str!("verify_release.ps1");
    assert!(verifier.contains("G0_RELEASE_MATRIX_PASSED"));
    assert!(verifier.contains("corrupt_artifact"));
    assert!(verifier.contains("unsupported_schema"));
    assert!(verifier.contains("MISSING_NEXT_ACTION"));
    assert!(!verifier.contains("Get-Content Env:"));
}

#[test]
fn release_signer_cli_produces_a_strictly_verifiable_detached_signature() {
    const TEST_SECRET: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const TEST_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let root = std::env::temp_dir().join(format!("gnx-sign-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let key = root.join("test.seed");
    let manifest = root.join("manifest.json");
    let signature = root.join("manifest.json.sig");
    std::fs::write(&key, TEST_SECRET).unwrap();
    std::fs::write(&manifest, br#"{"schema":1,"version":"test"}"#).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_gnx-sign"))
        .args([
            "sign",
            "--private-key",
            key.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--output",
            signature.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
    let signed = std::fs::read(&signature).unwrap();
    assert!(
        gnx::release_auth::verify(&std::fs::read(&manifest).unwrap(), &signed, TEST_PUBLIC).is_ok()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn windows_uninstall_requires_receipt_confirmation_and_owned_targets() {
    let uninstall = include_str!("../packaging/windows/uninstall.ps1");
    assert!(uninstall.contains("REMOVE-GNX-AND-DATA"));
    assert!(uninstall.contains("install-receipt.json"));
    assert!(uninstall.contains("$installRoot/uninstall.request"));
    assert!(uninstall.contains("SERVICE_OWNERSHIP_MISMATCH"));
    assert!(uninstall.contains("WSL_UNREGISTERED"));
    assert!(uninstall.contains("Remove-LocalUser gnx-runtime"));
    assert!(!uninstall.contains("Ubuntu-24.04"));
}

#[test]
fn windows_release_update_is_two_phase_and_rolls_both_boundaries_back() {
    let update = include_str!("../packaging/windows/update.ps1");
    let runtime = include_str!("../src/adapter/windows/runtime.rs");
    assert!(update.contains("RELEASE_NOT_NEWER"));
    assert!(update.contains("RELEASE_NOT_OLDER"));
    assert!(update.contains("release-previous"));
    assert!(update.contains("GNX-RELEASE-COMMIT-1"));
    assert!(update.contains("GNX-RELEASE-ROLLBACK-1"));
    assert!(update.contains("RELEASE_ROLLBACK_FAILED"));
    assert!(runtime.contains("AWAITING_COMMIT"));
    assert!(runtime.contains("RELEASE_CANDIDATE_READY"));
    assert!(runtime.contains("/usr/local/bin/gnx apply --config"));
    assert!(runtime.contains("/usr/local/bin/gnx status --config"));
    assert!(!update.contains("Ubuntu-24.04"));
}

#[test]
fn windows_service_unregisters_only_the_fixed_product_distro() {
    let service = include_str!("../src/adapter/windows/service.rs");
    let runtime = include_str!("../src/adapter/windows/runtime.rs");
    assert!(service.contains("GNX-UNINSTALL-1"));
    assert!(runtime.contains("[\"--unregister\", \"GNX\"]"));
    assert!(!runtime.contains("--unregister\", \"Ubuntu-24.04"));
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

#[test]
fn linux_runtime_revision_binds_intent_to_the_embedded_release() {
    let config = Config::parse(include_str!("../gnx.toml")).unwrap();
    let linux = gnx::adapter::linux::Linux::new(config.clone());
    let revision = linux.revision(&config);
    assert_eq!(revision.len(), 64);
    assert_ne!(revision, config.revision());
}

#[test]
fn linux_negotiates_secrets_before_runtime_mutation() {
    let linux = include_str!("../src/adapter/linux.rs");
    let prepare = linux.find("self.prepare_secrets(secret)?").unwrap();
    assert!(prepare < linux.find("self.ensure_images(&release)?").unwrap());
    assert!(prepare < linux.find("self.ensure_network()?").unwrap());
}
