use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use semver::Version;
use serde::Deserialize;
use url::Url;

use crate::{Error, Result};

const FORMAT_VERSION: u8 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    version: u8,
    mesh: MeshFile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshFile {
    control_server: String,
}

#[derive(Debug)]
pub struct Config {
    pub control_server: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(Error::ConfigRead)?;
        Self::parse(&text)
    }

    fn parse(text: &str) -> Result<Self> {
        let raw: ConfigFile = toml::from_str(text).map_err(Error::ConfigParse)?;
        if raw.version != FORMAT_VERSION {
            return Err(Error::ConfigVersion);
        }

        let endpoint = Url::parse(&raw.mesh.control_server).map_err(|_| Error::Endpoint)?;
        let valid = endpoint.scheme() == "https"
            && endpoint.host_str().is_some()
            && endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.query().is_none()
            && endpoint.fragment().is_none()
            && endpoint.path() == "/";
        if !valid {
            return Err(Error::Endpoint);
        }

        Ok(Self {
            control_server: raw.mesh.control_server,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseFile {
    version: u8,
    windows: WindowsRelease,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowsRelease {
    mesh_client: ArtifactFile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactFile {
    package: PathBuf,
    version: String,
    sha256: String,
    license: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub package: PathBuf,
    pub version: String,
    pub sha256: String,
    pub license: PathBuf,
}

impl Artifact {
    pub fn load(release_path: &Path) -> Result<Self> {
        let text = fs::read_to_string(release_path).map_err(Error::ReleaseRead)?;
        let raw: ReleaseFile = toml::from_str(&text).map_err(Error::ReleaseParse)?;
        let item = raw.windows.mesh_client;
        let digest_ok =
            item.sha256.len() == 64 && item.sha256.bytes().all(|b| b.is_ascii_hexdigit());
        let package_ok = item
            .package
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("msi"));
        let version = item.version.trim().trim_start_matches('v');
        if raw.version != FORMAT_VERSION
            || Version::parse(version).is_err()
            || !digest_ok
            || !package_ok
            || !bundle_path(&item.package)
            || !bundle_path(&item.license)
        {
            return Err(Error::ReleaseInvalid);
        }

        let parent = release_path.parent().unwrap_or_else(|| Path::new("."));
        Ok(Self {
            package: parent.join(item.package),
            version: version.to_owned(),
            sha256: item.sha256.to_ascii_lowercase(),
            license: parent.join(item.license),
        })
    }
}

fn bundle_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minimal_https_config() {
        let config = Config::parse("version=1\n[mesh]\ncontrol_server='https://mesh.gnx'").unwrap();
        assert_eq!(config.control_server, "https://mesh.gnx");
    }

    #[test]
    fn rejects_http_and_unknown_fields() {
        assert!(matches!(
            Config::parse("version=1\n[mesh]\ncontrol_server='http://mesh.gnx'"),
            Err(Error::Endpoint)
        ));
        assert!(matches!(
            Config::parse("version=1\nextra=true\n[mesh]\ncontrol_server='https://mesh.gnx'"),
            Err(Error::ConfigParse(_))
        ));
    }

    #[test]
    fn rejects_endpoint_credentials_or_path() {
        for endpoint in [
            "https://u:p@mesh.gnx",
            "https://mesh.gnx/api",
            "https://mesh.gnx/?x=1",
        ] {
            let text = format!("version=1\n[mesh]\ncontrol_server='{endpoint}'");
            assert!(matches!(Config::parse(&text), Err(Error::Endpoint)));
        }
    }

    #[test]
    fn release_files_must_stay_inside_the_bundle() {
        assert!(bundle_path(Path::new("artifacts/client.msi")));
        assert!(!bundle_path(Path::new("../client.msi")));
        assert!(!bundle_path(Path::new(r"C:\client.msi")));
    }
}
