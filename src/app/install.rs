use crate::{
    Error, Result,
    config::Artifact,
    port::{Host, Mesh},
};

use super::App;

impl<H: Host, M: Mesh> App<H, M> {
    pub fn install(&self, artifact: &Artifact) -> Result<String> {
        if self.mesh.installed_version()?.as_deref() == Some(&artifact.version) {
            return Ok(format!("install client={}", artifact.version));
        }

        self.require_elevation()?;
        self.mesh.install(artifact)?;
        let installed = self.mesh.installed_version()?.ok_or(Error::ClientMissing)?;
        if installed != artifact.version {
            return Err(Error::ClientVersion);
        }
        Ok(format!("install client={installed}"))
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, path::Path};

    use crate::{
        Result,
        config::Artifact,
        port::{Host, HostState, Mesh},
    };

    use super::App;

    struct FakeHost(bool);

    impl Host for FakeHost {
        fn inspect(&self) -> Result<HostState> {
            Ok(HostState { elevated: self.0 })
        }
    }

    struct FakeMesh {
        installed: Cell<bool>,
    }

    impl Mesh for FakeMesh {
        fn installed_version(&self) -> Result<Option<String>> {
            Ok(self.installed.get().then(|| "1.2.3".into()))
        }

        fn install(&self, _: &Artifact) -> Result<()> {
            self.installed.set(true);
            Ok(())
        }

        fn connect(&self, _: &str, _: Option<&Path>) -> Result<()> {
            Ok(())
        }

        fn ready(&self) -> Result<()> {
            Ok(())
        }
    }

    fn artifact() -> Artifact {
        Artifact {
            package: "client.msi".into(),
            version: "1.2.3".into(),
            sha256: "0".repeat(64),
            license: "client.LICENSE".into(),
            sbom: "client.cdx.json".into(),
        }
    }

    #[test]
    fn install_is_idempotent_without_elevation() {
        let app = App {
            host: FakeHost(false),
            mesh: FakeMesh {
                installed: Cell::new(true),
            },
        };
        assert!(app.install(&artifact()).is_ok());
    }

    #[test]
    fn install_requires_elevation_when_missing() {
        let app = App {
            host: FakeHost(false),
            mesh: FakeMesh {
                installed: Cell::new(false),
            },
        };
        assert!(matches!(
            app.install(&artifact()),
            Err(crate::Error::ElevationRequired)
        ));
    }

    #[test]
    fn install_converges_the_expected_version() {
        let app = App {
            host: FakeHost(true),
            mesh: FakeMesh {
                installed: Cell::new(false),
            },
        };
        assert!(app.install(&artifact()).is_ok());
    }
}
