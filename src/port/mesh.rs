use std::path::Path;

use crate::{Result, config::Artifact};

pub trait Mesh {
    fn installed_version(&self) -> Result<Option<String>>;
    fn install(&self, artifact: &Artifact) -> Result<()>;
    fn connect(&self, endpoint: &str, setup_key_file: Option<&Path>) -> Result<()>;
    fn ready(&self) -> Result<()>;
}
