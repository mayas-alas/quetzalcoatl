use crate::protocol::invalid;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    io::{self, Read},
    path::Path,
};

pub const MAX_CATALOG: usize = 16384;
pub const NAMES: [&str; 8] = [
    "pocket-id",
    "netbird-server",
    "netbird-dashboard",
    "rustfs",
    "dns",
    "gateway",
    "mesh",
    "compute",
];

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema: u16,
    pub architecture: String,
    pub components: Vec<Component>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub name: String,
    pub image: String,
}

impl Catalog {
    pub fn builtin() -> io::Result<Self> {
        let catalog = Self::decode(include_bytes!("../../../config/update-catalog.json"))?;
        let images: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../config/images.json"))
                .map_err(|_| invalid("semilla del núcleo inválida"))?;
        for role in ["mesh", "dns", "gateway", "compute"] {
            if images[role]["source"].as_str() != Some(catalog.get(role).image.as_str()) {
                return Err(invalid(
                    "catálogo incorporado difiere de las imágenes offline",
                ));
            }
        }
        for (role, quadlet) in [
            (
                "pocket-id",
                include_str!("../../../stacks/private-services/quadlets/pocket-id.container"),
            ),
            (
                "rustfs",
                include_str!("../../../stacks/private-services/quadlets/rustfs.container"),
            ),
            (
                "netbird-server",
                include_str!("../../../stacks/private-services/quadlets/netbird-server.container"),
            ),
            (
                "netbird-dashboard",
                include_str!(
                    "../../../stacks/private-services/quadlets/netbird-dashboard.container"
                ),
            ),
        ] {
            if quadlet.lines().find_map(|s| s.strip_prefix("Image="))
                != Some(catalog.get(role).image.as_str())
            {
                return Err(invalid("catálogo incorporado difiere del stack privado"));
            }
        }
        Ok(catalog)
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > MAX_CATALOG {
            return Err(invalid("catálogo fuera de límite"));
        }
        let value: Self =
            serde_json::from_slice(bytes).map_err(|_| invalid("catálogo inválido"))?;
        value.validate()?;
        Ok(value)
    }

    pub fn read(path: &Path) -> io::Result<Self> {
        let mut parent = Some(path);
        while let Some(p) = parent {
            let metadata = fs::symlink_metadata(p)?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err(invalid("enlace en catálogo"));
                }
            }
            if metadata.file_type().is_symlink() {
                return Err(invalid("enlace en catálogo"));
            }
            parent = p.parent().filter(|p| !p.as_os_str().is_empty());
        }
        let file = fs::File::open(path)?;
        if !file.metadata()?.is_file() || file.metadata()?.len() > MAX_CATALOG as u64 {
            return Err(invalid("catálogo fuera de límite"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_CATALOG as u64 + 1).read_to_end(&mut bytes)?;
        Self::decode(&bytes)
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.schema != 1 || self.architecture != "linux/amd64" || self.components.len() != 8 {
            return Err(invalid(
                "catálogo incompatible; requiere los ocho componentes linux/amd64",
            ));
        }
        let mut seen = HashSet::new();
        for component in &self.components {
            let repository = match component.name.as_str() {
                "mesh" => "docker.io/tailscale/tailscale",
                "dns" => "docker.io/pihole/pihole",
                "gateway" => "docker.io/library/caddy",
                "compute" => "ghcr.io/mayas-alas/tailnet-proxmox",
                "pocket-id" => "ghcr.io/pocket-id/pocket-id",
                "rustfs" => "docker.io/rustfs/rustfs",
                "netbird-server" => "docker.io/netbirdio/netbird-server",
                "netbird-dashboard" => "docker.io/netbirdio/dashboard",
                _ => return Err(invalid("componente no permitido")),
            };
            let prefix = format!("{repository}@sha256:");
            let digest = component
                .image
                .strip_prefix(&prefix)
                .ok_or_else(|| invalid("repositorio no permitido; usa digest explícito"))?;
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || !seen.insert(&component.name)
            {
                return Err(invalid("digest inválido o componente duplicado"));
            }
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> &Component {
        self.components
            .iter()
            .find(|c| c.name == name)
            .expect("validated fixed component set")
    }
}
