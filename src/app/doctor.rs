use crate::{
    Error, Result,
    port::{Host, Mesh},
};

use super::App;

impl<H: Host, M: Mesh> App<H, M> {
    pub fn doctor(&self) -> Result<String> {
        let host = self.host.inspect()?;
        let version = self.mesh.installed_version()?.ok_or(Error::ClientMissing)?;
        Ok(format!(
            "doctor elevated={} client={version}",
            host.elevated
        ))
    }
}
