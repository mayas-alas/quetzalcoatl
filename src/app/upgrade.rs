use crate::{
    port::host::{UpgradeHost, UpgradeObservation},
    report::{Capability, Report, State},
};

/// Runs the non-mutating host preflight for a 0.3.1 upgrade.
///
/// Upgrade is deliberately outside the four-operation runtime broker protocol.
/// This stage only observes the host through the upgrade port and never stops
/// a service, changes ACLs, writes state, or touches WSL.
pub fn check(host: Option<&dyn UpgradeHost>) -> Report {
    let Some(host) = host else {
        return Report::new(
            "upgrade",
            State::ActionRequired,
            "UPGRADE_WINDOWS_ONLY",
            Some("Run the upgrade preflight on the Windows host."),
        );
    };
    report(host.preflight_upgrade())
}

fn report(observed: UpgradeObservation) -> Report {
    let mut report = Report::new(
        "upgrade",
        if observed.legacy_present {
            State::Ready
        } else {
            State::ActionRequired
        },
        if observed.legacy_present {
            "UPGRADE_PREFLIGHT_READY"
        } else {
            "UPGRADE_SOURCE_NOT_FOUND"
        },
        if observed.legacy_present {
            Some("Review this preflight; the mutating upgrade phase is not enabled yet.")
        } else {
            Some("Install the supported previous GNX release before upgrading.")
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

    struct Fixture(UpgradeObservation);

    impl UpgradeHost for Fixture {
        fn preflight_upgrade(&self) -> UpgradeObservation {
            self.0
        }
    }

    #[test]
    fn preflight_is_explicitly_non_mutating() {
        let host = Fixture(UpgradeObservation {
            legacy_present: true,
            target_present: false,
        });
        let report = check(Some(&host));
        assert_eq!(report.operation, "upgrade");
        assert_eq!(report.code, "UPGRADE_PREFLIGHT_READY");
        assert!(report
            .changes
            .iter()
            .any(|change| change == "No credentials read or changed"));
    }
}
