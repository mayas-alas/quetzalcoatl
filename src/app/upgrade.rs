use crate::report::{Capability, Report, State};

/// Runs the non-mutating host preflight for a 0.3.1 upgrade.
///
/// Upgrade is deliberately outside the four-operation runtime broker protocol.
/// This first phase only observes well-known installation locations and never
/// stops a service, changes ACLs, writes state, or touches WSL.
pub fn check() -> Report {
    #[cfg(windows)]
    {
        let legacy_program = std::path::Path::new(r"C:\Program Files\QuetzalcoatlNext");
        let current_program = std::path::Path::new(r"C:\Program Files\GNX");
        let legacy_state = std::path::Path::new(r"C:\ProgramData\QuetzalcoatlNext");
        let service_binary = current_program.join("gnx-service.exe");
        let old_tray_binary = current_program.join("gnx-tray.exe");

        // The existing main/rama-mvp installer already uses C:\Program Files\GNX.
        // Its distinguishing marker is the tray binary and the absence of the
        // 0.3.1 service binary, so the preflight must not mistake it for the
        // target installation.
        let legacy_present = legacy_program.exists()
            || legacy_state.exists()
            || (old_tray_binary.exists() && !service_binary.exists());
        let current_present = service_binary.exists();
        let mut report = Report::new(
            "upgrade",
            if legacy_present {
                State::Ready
            } else {
                State::ActionRequired
            },
            if legacy_present {
                "UPGRADE_PREFLIGHT_READY"
            } else {
                "UPGRADE_SOURCE_NOT_FOUND"
            },
            if legacy_present {
                Some("Review this preflight; the mutating upgrade phase is not enabled yet.")
            } else {
                Some("Install the supported previous GNX release before upgrading.")
            },
        );
        report.capabilities = vec![
            capability("legacy-installation", legacy_present, "LEGACY_PRESENT"),
            capability("target-installation", current_present, "TARGET_PRESENT"),
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

    #[cfg(not(windows))]
    {
        Report::new(
            "upgrade",
            State::ActionRequired,
            "UPGRADE_WINDOWS_ONLY",
            Some("Run the upgrade preflight on the Windows host."),
        )
    }
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

    #[test]
    fn preflight_is_explicitly_non_mutating() {
        let report = check();
        assert_eq!(report.operation, "upgrade");
        assert!(report
            .changes
            .iter()
            .any(|change| change == "No credentials read or changed"));
    }
}
