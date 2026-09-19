pub trait Host {
    fn prerequisites(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupObservation {
    pub legacy_present: bool,
    pub target_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupVerification {
    Ready,
    RebootRequired,
}

pub trait SetupHost {
    fn preflight_setup(&self) -> SetupObservation;
    fn validate_bundle(&self, input: &crate::domain::setup::BundleInput) -> Result<(), String>;
    /// Bounded provisioning only; never starts a service or migrates legacy data.
    fn provision_setup(&self, _input: &crate::domain::setup::BundleInput) -> Result<(), String> {
        Err("SETUP_PROVISION_UNAVAILABLE".into())
    }

    /// Verify the installed target through the host's service/broker/status
    /// boundary. A default keeps non-Windows and test hosts conservative.
    fn verify_setup(&self) -> Result<SetupVerification, String> {
        Err("SETUP_VERIFY_UNAVAILABLE".into())
    }

    fn recover_setup(&self, _rollback: bool) -> Result<(), String> {
        Err("SETUP_RECOVERY_UNAVAILABLE".into())
    }
}
