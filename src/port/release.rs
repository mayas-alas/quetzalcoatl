pub trait Release {
    fn verify(&self) -> Result<(), String>;
}
