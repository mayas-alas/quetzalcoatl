use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::{Host, Url};

use crate::error::GnxError;

pub const CONFIG_SCHEMA: u32 = 1;
pub const MACHINE_NAME: &str = "quetzalcoatl";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: u32,
    pub mesh: MeshConfig,
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub services: ServicesConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeshConfig {
    pub controller_url: String,
    #[serde(default)]
    pub expected_domain: Option<String>,
    /// Direcciones de bootstrap para resolver el controller antes del enrolamiento.
    #[serde(default)]
    pub bootstrap_addresses: Vec<IpAddr>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub machine_name: String,
    pub profile: RuntimeProfile,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeProfile {
    Lab,
    Standard,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServicesConfig {
    #[serde(default)]
    pub garage: bool,
    #[serde(default)]
    pub forgejo: bool,
    #[serde(default)]
    pub runner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerUrl {
    canonical: String,
    host: String,
}

impl ControllerUrl {
    pub fn parse(value: &str) -> Result<Self, GnxError> {
        let mut url = Url::parse(value)
            .map_err(|error| GnxError::controller_invalid(format!("URL inválida: {error}.")))?;

        if url.scheme() != "https" {
            return Err(GnxError::controller_invalid(
                "El controller debe utilizar HTTPS.",
            ));
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(GnxError::controller_invalid(
                "El controller no puede contener credenciales.",
            ));
        }

        let host = match url.host() {
            Some(Host::Domain(host)) => host.to_ascii_lowercase(),
            Some(Host::Ipv4(_)) | Some(Host::Ipv6(_)) => {
                return Err(GnxError::controller_invalid(
                    "El controller requiere un nombre DNS; las IP literales no están permitidas.",
                ));
            }
            None => {
                return Err(GnxError::controller_invalid(
                    "El controller requiere un hostname DNS.",
                ));
            }
        };

        if url.port().is_some_and(|port| port != 443) {
            return Err(GnxError::controller_invalid(
                "El controller sólo puede usar el puerto HTTPS 443.",
            ));
        }

        if url.query().is_some() || url.fragment().is_some() {
            return Err(GnxError::controller_invalid(
                "El controller no puede contener query ni fragment.",
            ));
        }

        if url.path() != "/" && !url.path().is_empty() {
            return Err(GnxError::controller_invalid(
                "El controller no puede contener una ruta adicional.",
            ));
        }

        url.set_path("");
        let canonical = url.as_str().trim_end_matches('/').to_string();

        Ok(Self { canonical, host })
    }

    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    pub fn host(&self) -> &str {
        &self.host
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, GnxError> {
        let content = fs::read_to_string(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                GnxError::config_not_found(path)
            } else {
                GnxError::config_invalid(format!("No se pudo leer {}: {error}.", path.display()))
            }
        })?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self, GnxError> {
        let config: Self = toml::from_str(content)
            .map_err(|error| GnxError::config_invalid(format!("TOML inválido: {error}.")))?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), GnxError> {
        self.validate()?;
        let mut content = toml::to_string_pretty(self).map_err(|error| {
            GnxError::config_invalid(format!("No se pudo serializar la configuración: {error}."))
        })?;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        crate::state::atomic_write(path, content.as_bytes())
    }

    pub fn validate(&self) -> Result<ControllerUrl, GnxError> {
        if self.schema != CONFIG_SCHEMA {
            return Err(GnxError::config_invalid(format!(
                "Schema {} no soportado; esta versión requiere schema {CONFIG_SCHEMA}.",
                self.schema
            )));
        }

        if self.runtime.machine_name != MACHINE_NAME {
            return Err(GnxError::config_invalid(format!(
                "runtime.machine_name debe ser '{MACHINE_NAME}'."
            )));
        }

        if self
            .mesh
            .expected_domain
            .as_deref()
            .is_some_and(|domain| domain.trim().is_empty())
        {
            return Err(GnxError::config_invalid(
                "mesh.expected_domain no puede estar vacío.",
            ));
        }

        if self.mesh.bootstrap_addresses.len() > 4 {
            return Err(GnxError::config_invalid(
                "mesh.bootstrap_addresses admite como máximo cuatro direcciones.",
            ));
        }
        for (index, address) in self.mesh.bootstrap_addresses.iter().enumerate() {
            if address.is_unspecified() || address.is_multicast() || address.is_loopback() {
                return Err(GnxError::config_invalid(format!(
                    "mesh.bootstrap_addresses[{index}] no puede ser unspecified, multicast ni loopback."
                )));
            }
            if self.mesh.bootstrap_addresses[..index].contains(address) {
                return Err(GnxError::config_invalid(format!(
                    "mesh.bootstrap_addresses contiene la dirección duplicada {address}."
                )));
            }
        }

        ControllerUrl::parse(&self.mesh.controller_url)
    }
}

pub fn default_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        root.join("QuetzalcoatlNext").join("config.toml")
    }

    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/etc/quetzalcoatl-next/config.toml")
    }
}

pub fn data_root() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        root.join("QuetzalcoatlNext")
    }

    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/var/lib/quetzalcoatl-next")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config(controller_url: &str) -> String {
        format!(
            r#"
schema = 1

[mesh]
controller_url = "{controller_url}"
expected_domain = "node.gnx"
bootstrap_addresses = []

[runtime]
machine_name = "quetzalcoatl"
profile = "lab"
"#
        )
    }

    #[test]
    fn accepts_required_gnx_controller_names() {
        for controller in [
            "https://headscale.node.gnx",
            "https://controlplane.node.gnx",
        ] {
            let config = Config::parse(&valid_config(controller)).unwrap();
            assert_eq!(config.validate().unwrap().canonical(), controller);
        }
    }

    #[test]
    fn canonicalizes_host_case_and_default_port() {
        let controller = ControllerUrl::parse("https://ControlPlane.Node.GNX:443/").unwrap();
        assert_eq!(controller.canonical(), "https://controlplane.node.gnx");
        assert_eq!(controller.host(), "controlplane.node.gnx");
    }

    #[test]
    fn accepts_other_https_dns_controller_without_brand_policy() {
        let controller = Config::parse(&valid_config("https://controller.example.test"))
            .unwrap()
            .validate()
            .unwrap();
        assert_eq!(controller.canonical(), "https://controller.example.test");
    }

    #[test]
    fn rejects_non_https_controller() {
        let error = Config::parse(&valid_config("http://controlplane.node.gnx")).unwrap_err();
        assert_eq!(error.code, "MESH_CONTROLLER_URL_INVALID");
    }

    #[test]
    fn rejects_ip_literal_controller() {
        let error = Config::parse(&valid_config("https://192.0.2.10")).unwrap_err();
        assert_eq!(error.code, "MESH_CONTROLLER_URL_INVALID");
    }

    #[test]
    fn rejects_controller_path_query_and_nonstandard_port() {
        for controller in [
            "https://controlplane.node.gnx/api",
            "https://controlplane.node.gnx?token=no",
            "https://controlplane.node.gnx:8443",
        ] {
            let error = Config::parse(&valid_config(controller)).unwrap_err();
            assert_eq!(error.code, "MESH_CONTROLLER_URL_INVALID");
        }
    }

    #[test]
    fn rejects_unknown_configuration_fields() {
        let source = format!(
            "{}\nlegacy_mode = true\n",
            valid_config("https://headscale.node.gnx")
        );
        let error = Config::parse(&source).unwrap_err();
        assert_eq!(error.code, "CONFIG_INVALID");
    }

    #[test]
    fn rejects_alternate_machine_name() {
        let source = valid_config("https://headscale.node.gnx").replace(
            "machine_name = \"quetzalcoatl\"",
            "machine_name = \"legacy\"",
        );
        let error = Config::parse(&source).unwrap_err();
        assert_eq!(error.code, "CONFIG_INVALID");
    }

    #[test]
    fn accepts_private_and_tailnet_bootstrap_addresses() {
        let source = valid_config("https://controlplane.node.gnx").replace(
            "bootstrap_addresses = []",
            "bootstrap_addresses = [\"192.168.10.5\", \"100.64.10.5\"]",
        );
        let config = Config::parse(&source).unwrap();
        assert_eq!(config.mesh.bootstrap_addresses.len(), 2);
    }

    #[test]
    fn rejects_unsafe_or_duplicate_bootstrap_addresses() {
        for addresses in [
            "[\"127.0.0.1\"]",
            "[\"224.0.0.1\"]",
            "[\"192.168.10.5\", \"192.168.10.5\"]",
        ] {
            let source = valid_config("https://controlplane.node.gnx").replace(
                "bootstrap_addresses = []",
                &format!("bootstrap_addresses = {addresses}"),
            );
            assert!(Config::parse(&source).is_err());
        }
    }
}
