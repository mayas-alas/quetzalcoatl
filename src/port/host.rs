pub trait Host {
    fn prerequisites(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupObservation {
    pub legacy_present: bool,
    pub target_present: bool,
}

pub trait SetupHost {
    fn preflight_setup(&self) -> SetupObservation;
    fn validate_bundle(&self, input: &crate::domain::setup::BundleInput) -> Result<(), String>;
}
