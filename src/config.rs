use std::{fs, path::Path};

use serde::Deserialize;
use url::Url;

use crate::{Error, Result};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    version: u8,
    pub host: Host,
    pub access: Access,
    pub compute: Compute,
    pub controller: Controller,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub distribution: String,
    pub runtime_dir: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    pub state_dir: String,
    pub zone: String,
    pub hostname: String,
    pub tag: String,
    pub uplink: String,
    pub uplink_mtu: u16,
    pub services: Vec<Service>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub name: String,
    pub alias: String,
    pub fqdn: String,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compute {
    pub state_dir: String,
    pub node: String,
    pub username: String,
    pub verify_endpoint: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controller {
    pub state_dir: String,
    pub autonomous_ca: bool,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path).map_err(Error::ConfigRead)?;
        let config: Self = toml::from_str(&source).map_err(Error::ConfigParse)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        let token = |value: &str| {
            !value.is_empty()
                && value.len() <= 63
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        };
        let dns = |value: &str| {
            value.len() <= 253
                && value.contains('.')
                && value.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && !label.starts_with('-')
                        && !label.ends_with('-')
                        && label
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                })
        };
        let local_http = |value: &str| {
            Url::parse(value).ok().is_some_and(|url| {
                url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none()
            })
        };
        let services = !self.access.services.is_empty()
            && self.access.services.iter().all(|service| {
                token(&service.name)
                    && dns(&service.alias)
                    && service.alias.ends_with(&format!(".{}", self.access.zone))
                    && dns(&service.fqdn)
                    && service.fqdn.ends_with(".ts.net")
                    && local_http(&service.target)
            });
        let account = self
            .compute
            .username
            .split_once('@')
            .is_some_and(|(name, realm)| token(name) && token(realm));

        if self.version != 1
            || !token(&self.host.distribution)
            || self.host.runtime_dir != "/usr/local/share/gnx/runtime"
            || self.access.state_dir != "/var/lib/gnx/access"
            || self.access.zone != "gnx"
            || !token(&self.access.hostname)
            || !self.access.tag.starts_with("tag:")
            || !token(self.access.tag.trim_start_matches("tag:"))
            || !token(&self.access.uplink)
            || self.access.uplink == "gnx-access"
            || !(1280..=9000).contains(&self.access.uplink_mtu)
            || !services
            || self.compute.state_dir != "/var/lib/gnx/compute"
            || !token(&self.compute.node)
            || !account
            || !local_http(&self.compute.verify_endpoint)
            || self.controller.state_dir != "/var/lib/gnx/controller"
        {
            return Err(Error::ConfigInvalid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = include_str!("../config/gnx.toml");

    #[test]
    fn accepts_only_the_three_declared_capabilities() {
        let config: Config = toml::from_str(CONFIG).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.access.services[0].alias, "compute.gnx");
        assert!(config.controller.autonomous_ca);
    }

    #[test]
    fn rejects_secrets_unknown_fields_and_remote_backends() {
        for source in [
            format!("{CONFIG}\nsecret='forbidden'"),
            CONFIG.replace("127.0.0.1", "remote.invalid"),
            CONFIG.replace("/var/lib/gnx/access", "/"),
            CONFIG.replace("zone = \"gnx\"", "zone = \"invalid\""),
            CONFIG.replace("compute.gnx", "compute_.gnx"),
        ] {
            assert!(
                toml::from_str::<Config>(&source).map_or(true, |value| value.validate().is_err())
            );
        }
    }
}
