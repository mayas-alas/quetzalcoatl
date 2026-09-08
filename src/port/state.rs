use crate::config::Config;
pub trait TransactionGuard {}
impl<T> TransactionGuard for T {}
pub trait StateStore {
    fn current(&self) -> Result<Option<String>, String>;
    fn previous(&self) -> Result<Option<Config>, String> {
        Ok(None)
    }
    fn acquire(&self) -> Result<Box<dyn TransactionGuard>, String> {
        Ok(Box::new(()))
    }
    fn interrupted(&self) -> Result<bool, String> {
        Ok(false)
    }
    fn stage(&self, config: &Config) -> Result<(), String>;
    fn phase(&self, _phase: &str) -> Result<(), String> {
        Ok(())
    }
    fn promote(&self) -> Result<(), String>;
    fn abort(&self) -> Result<(), String> {
        Ok(())
    }
}
