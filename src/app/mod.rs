mod connect;
mod doctor;
mod install;

use crate::port::{Host, Mesh};

pub struct App<H, M> {
    pub host: H,
    pub mesh: M,
}

impl<H: Host, M: Mesh> App<H, M> {
    fn require_elevation(&self) -> crate::Result<()> {
        if self.host.inspect()?.elevated {
            Ok(())
        } else {
            Err(crate::Error::ElevationRequired)
        }
    }
}
