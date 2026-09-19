use crate::{
    port::host::{SetupHost, SetupObservation},
    report::{Capability, Report, State},
};

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
            Ok(()) => report.capabilities.push(capability(
                "release-bundle",
                true,
                "BUNDLE_AUTHENTICATED",
            )),
            Err(code) => {
                report.state = State::Failed;
                report.code = code;
                report.next_action = Some("Use a trusted GNX bundle and verify its hashes.".into());
                report.capabilities.push(capability(
                    "release-bundle",
                    false,
                    "BUNDLE_INVALID",
                ));
            }
        }
    }
    report
}

fn report(observed: SetupObservation) -> Report {
    let mut report = Report::new(
        "setup",
        if observed.legacy_present {
            State::Ready
        } else {
            State::ActionRequired
        },
        if observed.legacy_present {
            "SETUP_PREFLIGHT_READY"
        } else {
            "SETUP_SOURCE_NOT_FOUND"
        },
        if observed.legacy_present {
            Some("Review this preflight; the mutating setup phase is not enabled yet.")
        } else {
            Some("Install the supported previous GNX release before setup.")
        },
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
        capability(
            "non-mutating-preflight",
            true,
            "NO_MUTATION_PERFORMED",
        ),
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
        assert_eq!(report.code, "SETUP_PREFLIGHT_READY");
        assert!(report
            .changes
            .iter()
            .any(|change| change == "No credentials read or changed"));
    }
}
