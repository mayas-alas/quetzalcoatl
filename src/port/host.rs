pub trait Host {
    fn prerequisites(&self) -> Result<(), String>;
}
