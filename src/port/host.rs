pub trait Host {
    fn prerequisites(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpgradeObservation {
    pub legacy_present: bool,
    pub target_present: bool,
}

pub trait UpgradeHost {
    fn preflight_upgrade(&self) -> UpgradeObservation;
}
