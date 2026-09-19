use crate::{
    port::host::{SetupHost, SetupObservation, SetupVerification},
    report::{Capability, Report, State},
};

pub fn provision(
    host: Option<&dyn SetupHost>,
    input: &crate::domain::setup::BundleInput,
) -> Report {
    let Some(host) = host else {
        return Report::new(
            "setup",
            State::ActionRequired,
            "SETUP_WINDOWS_ONLY",
            Some("Run setup on the Windows host."),
        );
    };
    match host.provision_setup(input) {
        Ok(()) => {
            let mut result = Report::new(
                "setup",
                State::ActionRequired,
                "SETUP_PROVISIONED",
                Some("Continue with runtime bootstrap and verification before cutover."),
            );
            result.changes = vec![
                "Trusted release staged".into(),
                "Dedicated account and stopped GNXRuntime registered".into(),
                "Legacy installation preserved".into(),
            ];
            result
        }
        Err(code) => {
            // Adapter errors are machine codes only; never return OS messages or input paths.
            let safe = code.len() <= 80
                && !code.is_empty()
                && code
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b == b'_' || b.is_ascii_digit());
            Report::new("setup", State::Failed, if safe { &code } else { "SETUP_PROVISION_FAILED" }, Some("Inspect the protected setup journal and snapshot; recover partial provisioning before retrying. Legacy has not been cut over."))
        }
    }
}

/// Apply is the complete finite setup transaction. Provisioning alone never
/// implies runtime readiness; readiness is granted only by host verification.
pub fn apply(host: Option<&dyn SetupHost>, input: &crate::domain::setup::BundleInput) -> Report {
    let Some(host) = host else {
        return Report::new(
            "setup",
            State::ActionRequired,
            "SETUP_WINDOWS_ONLY",
            Some("Run setup on the Windows host."),
        );
    };
    if let Err(code) = host.provision_setup(input) {
        return failed(code);
    }
    match host.verify_setup() {
        Ok(SetupVerification::Ready) => Report::new("setup", State::Ready, "SETUP_READY", None),
        Ok(SetupVerification::RebootRequired) => Report::new(
            "setup",
            State::ActionRequired,
            "SETUP_REBOOT_REQUIRED",
            Some("Restart Windows, then run setup status to complete verification."),
        ),
        Err(code) => failed(code),
    }
}

pub fn recover(host: Option<&dyn SetupHost>, rollback: bool) -> Report {
    let Some(host) = host else {
        return Report::new(
            "setup",
            State::ActionRequired,
            "SETUP_WINDOWS_ONLY",
            Some("Run setup recovery on the Windows host."),
        );
    };
    match host.recover_setup(rollback) {
        Ok(()) => Report::new(
            "setup",
            State::ActionRequired,
            if rollback {
                "SETUP_ROLLED_BACK"
            } else {
                "SETUP_RECOVERED"
            },
            Some("Run setup status before attempting another apply."),
        ),
        Err(code) => failed(code),
    }
}

fn failed(code: String) -> Report {
    let safe = code.len() <= 80
        && !code.is_empty()
        && code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b == b'_' || b.is_ascii_digit());
    Report::new(
        "setup",
        State::Failed,
        if safe { &code } else { "SETUP_FAILED" },
        Some("Inspect protected setup state and recover before retrying."),
    )
}

/// Runs the non-mutating host preflight for a 0.3.1 upgrade.
///
/// Upgrade is deliberately outside the four-operation runtime broker protocol.
/// This stage only observes the host through the upgrade port and never stops
/// a service, changes ACLs, writes state, or touches WSL.
pub fn preflight(
    host: Option<&dyn SetupHost>,
    bundle: Option<&crate::domain::setup::BundleInput>,
) -> Report {
    let Some(host) = host else {
        return Report::new(
            "setup",
            State::ActionRequired,
            "SETUP_WINDOWS_ONLY",
            Some("Run the setup preflight on the Windows host."),
        );
    };
    let mut report = report(host.preflight_setup());
    if let Some(bundle) = bundle {
        match host.validate_bundle(bundle) {
            Ok(()) => {
                if report.code == "SETUP_SOURCE_NOT_FOUND" {
                    report.code = "SETUP_SOURCE_FOUND".into();
                    report.next_action = Some(
                        "Run --provision with the authenticated bundle and rootfs inputs.".into(),
                    );
                }
                report
                    .capabilities
                    .push(capability("release-bundle", true, "BUNDLE_AUTHENTICATED"))
            }
            Err(code) => {
                report.state = State::Failed;
                report.code = code;
                report.next_action = Some("Use a trusted GNX bundle and verify its hashes.".into());
                report
                    .capabilities
                    .push(capability("release-bundle", false, "BUNDLE_INVALID"));
            }
        }
    }
    report
}

fn report(observed: SetupObservation) -> Report {
    if observed.legacy_present {
        let mut report = Report::new(
            "setup",
            State::Failed,
            "SETUP_LEGACY_CONFLICT",
            Some("Legacy GNX roots are present; setup refuses adoption or overwrite."),
        );
        report.capabilities = vec![
            capability("legacy-installation", true, "LEGACY_CONFLICT"),
            capability(
                "target-installation",
                observed.target_present,
                "TARGET_PRESENT",
            ),
            capability("non-mutating-preflight", true, "NO_MUTATION_PERFORMED"),
        ];
        report.changes = vec![
            "No services stopped".into(),
            "No files migrated".into(),
            "No credentials read or changed".into(),
        ];
        return report;
    }
    if observed.target_present {
        let mut report = Report::new(
            "setup",
            State::Failed,
            "SETUP_TARGET_CONFLICT",
            Some(
                "The GNX 0.3.1 target root already exists; recover it explicitly before retrying.",
            ),
        );
        report.capabilities = vec![
            capability("legacy-installation", false, "LEGACY_ABSENT"),
            capability("target-installation", true, "TARGET_CONFLICT"),
            capability("non-mutating-preflight", true, "NO_MUTATION_PERFORMED"),
        ];
        report.changes = vec![
            "No services stopped".into(),
            "No files migrated".into(),
            "No credentials read or changed".into(),
        ];
        return report;
    }
    let mut report = Report::new(
        "setup",
        State::ActionRequired,
        "SETUP_SOURCE_NOT_FOUND",
        Some("Provide the trusted release bundle and rootfs inputs."),
    );
    report.capabilities = vec![
        capability(
            "legacy-installation",
            observed.legacy_present,
            "LEGACY_PRESENT",
        ),
        capability(
            "target-installation",
            observed.target_present,
            "TARGET_PRESENT",
        ),
        capability("non-mutating-preflight", true, "NO_MUTATION_PERFORMED"),
    ];
    report.changes = vec![
        "No services stopped".into(),
        "No files migrated".into(),
        "No credentials read or changed".into(),
    ];
    report
}

fn capability(name: &str, healthy: bool, code: &str) -> Capability {
    Capability {
        name: name.into(),
        healthy,
        code: code.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(SetupObservation);

    impl SetupHost for Fixture {
        fn preflight_setup(&self) -> SetupObservation {
            self.0
        }

        fn validate_bundle(
            &self,
            _input: &crate::domain::setup::BundleInput,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn preflight_is_explicitly_non_mutating() {
        let host = Fixture(SetupObservation {
            legacy_present: true,
            target_present: false,
        });
        let report = preflight(Some(&host), None);
        assert_eq!(report.operation, "setup");
        assert_eq!(report.code, "SETUP_LEGACY_CONFLICT");
        assert_eq!(report.state, State::Failed);
        assert!(report
            .changes
            .iter()
            .any(|change| change == "No credentials read or changed"));
    }

    #[test]
    fn preflight_reports_target_conflict_as_failure() {
        let host = Fixture(SetupObservation {
            legacy_present: false,
            target_present: true,
        });
        let report = preflight(Some(&host), None);
        assert_eq!(report.state, State::Failed);
        assert_eq!(report.code, "SETUP_TARGET_CONFLICT");
        assert!(report
            .changes
            .iter()
            .all(|change| change.starts_with("No ")));
    }

    struct ProvisionFixture(Result<(), String>);
    impl SetupHost for ProvisionFixture {
        fn preflight_setup(&self) -> SetupObservation {
            panic!("provision must own its preflight under lock")
        }
        fn validate_bundle(&self, _: &crate::domain::setup::BundleInput) -> Result<(), String> {
            panic!("provision must authenticate its staged copies")
        }
        fn provision_setup(&self, _: &crate::domain::setup::BundleInput) -> Result<(), String> {
            self.0.clone()
        }
    }
    fn input() -> crate::domain::setup::BundleInput {
        crate::domain::setup::BundleInput {
            bundle: "unused".into(),
            rootfs: "unused".into(),
            manifest_sha256: "a".repeat(64),
            rootfs_sha256: "b".repeat(64),
        }
    }
    #[test]
    fn provision_never_claims_runtime_readiness() {
        let result = provision(Some(&ProvisionFixture(Ok(()))), &input());
        assert_eq!(result.state, State::ActionRequired);
        assert_eq!(result.code, "SETUP_PROVISIONED");
    }
    #[test]
    fn provision_sanitizes_failures() {
        let result = provision(
            Some(&ProvisionFixture(Err("password=canary".into()))),
            &input(),
        );
        assert_eq!(result.state, State::Failed);
        assert_eq!(result.code, "SETUP_PROVISION_FAILED");
        assert!(!serde_json::to_string(&result).unwrap().contains("canary"));
        let result = provision(
            Some(&ProvisionFixture(Err("SETUP_RECOVERY_REQUIRED".into()))),
            &input(),
        );
        assert_eq!(result.code, "SETUP_RECOVERY_REQUIRED");
    }
}
