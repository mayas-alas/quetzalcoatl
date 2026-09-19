use crate::{config::Config, domain::secret::Secret};
pub trait Runtime {
    fn observe(&self) -> Vec<crate::report::Capability>;
    fn reconcile(&self, config: &Config) -> Result<(), String>;
    fn reconcile_secret(&self, config: &Config, _secret: Option<&Secret>) -> Result<(), String> {
        self.reconcile(config)
    }
    fn restore(&self, _previous: Option<&Config>) -> Result<(), String> {
        Ok(())
    }
    fn optional_routes(&self) -> Vec<crate::report::Capability> {
        vec![]
    }
    fn public_root(&self) -> Option<String> {
        None
    }
}
