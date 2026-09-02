use std::path::Path;

use crate::{
    Error, Result,
    port::{Host, Mesh},
};

use super::App;

impl<H: Host, M: Mesh> App<H, M> {
    pub fn connect(&self, endpoint: &str, setup_key_file: Option<&Path>) -> Result<String> {
        self.require_elevation()?;
        if self.mesh.installed_version()?.is_none() {
            return Err(Error::ClientMissing);
        }
        if setup_key_file.is_some_and(|path| !path.is_file()) {
            return Err(Error::SetupKeyFile);
        }

        self.mesh.connect(endpoint, setup_key_file)?;
        self.mesh.ready()?;
        Ok("connect state=connected".into())
    }
}
