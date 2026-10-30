use crate::protocol::invalid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{self, Read},
    path::Path,
};
use zeroize::Zeroize;

const MAX_TOTAL: usize = 48 * 1024;
const MAX_FILE: usize = 16 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub path: String,
    pub contents: String,
}

impl Drop for File {
    fn drop(&mut self) {
        self.contents.zeroize();
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub files: Vec<File>,
}

impl Bundle {
    pub fn read(root: &Path) -> io::Result<Self> {
        fn collect(root: &Path, directory: &Path, files: &mut Vec<File>) -> io::Result<()> {
            for entry in fs::read_dir(directory)? {
                let path = entry?.path();
                let metadata = fs::symlink_metadata(&path)?;
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 {
                        return Err(invalid("enlace/reparse point en la entrada"));
                    }
                }
                if metadata.file_type().is_symlink() {
                    return Err(invalid("enlace en la entrada"));
                }
                let relative = path.strip_prefix(root).map_err(io::Error::other)?;
                let name = relative.to_string_lossy().replace('\\', "/");
                if metadata.is_dir() {
                    if !name.starts_with("config/") && name != "config" {
                        return Err(invalid("solo se admiten subdirectorios de config"));
                    }
                    if name.split('/').count() > 4 {
                        return Err(invalid("directorio fuera de límite"));
                    }
                    collect(root, &path, files)?;
                } else {
                    if !metadata.is_file() || metadata.len() > MAX_FILE as u64 || files.len() >= 32
                    {
                        return Err(invalid("archivo o cantidad fuera de límite"));
                    }
                    let mut contents = String::new();
                    fs::File::open(&path)?
                        .take((MAX_FILE + 1) as u64)
                        .read_to_string(&mut contents)?;
                    if contents.len() > MAX_FILE {
                        return Err(invalid("archivo fuera de límite"));
                    }
                    files.push(File {
                        path: name,
                        contents,
                    });
                    if files.iter().map(|f| f.contents.len()).sum::<usize>() > MAX_TOTAL {
                        return Err(invalid("carpeta fuera de límite"));
                    }
                }
            }
            Ok(())
        }
        if fs::symlink_metadata(root)?.file_type().is_symlink() || !root.is_dir() {
            return Err(invalid("se requiere una carpeta local sin enlaces"));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if fs::symlink_metadata(root)?.file_attributes() & 0x400 != 0 {
                return Err(invalid("carpeta de entrada enlazada"));
            }
        }
        let root = fs::canonicalize(root)?;
        let mut bundle = Self { files: Vec::new() };
        collect(&root, &root, &mut bundle.files)?;
        bundle.files.sort_by(|a, b| a.path.cmp(&b.path));
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.files.is_empty() || self.files.len() > 32 {
            return Err(invalid("cantidad de archivos fuera de límite"));
        }
        let mut seen = HashSet::new();
        let mut total = 0;
        let mut containers = 0;
        for file in &self.files {
            total += file.contents.len();
            if file.path.len() > 160
                || file.path.split('/').count() > 5
                || !file
                    .path
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-._/".contains(&b))
                || file
                    .path
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == ".." || p.starts_with('.'))
                || !seen.insert(&file.path)
                || file.contents.len() > MAX_FILE
                || file.contents.contains('\0')
            {
                return Err(invalid("archivo de despliegue inválido"));
            }
            if !file.path.contains('/') && file.path.ends_with(".container") {
                validate_quadlet(&file.contents)?;
                containers += 1;
            } else if !file.path.contains('/') && file.path.ends_with(".network") {
                validate_network(&file.contents)?;
            } else if !file.path.starts_with("config/") {
                return Err(invalid(
                    "solo se admiten .container, .network y archivos bajo config/",
                ));
            }
        }
        if total > MAX_TOTAL || containers == 0 || containers > 8 {
            return Err(invalid("tamaño o cantidad de Quadlets fuera de límite"));
        }
        for file in &self.files {
            if !file.path.contains('/') && file.path.ends_with(".container") {
                for line in file.contents.lines() {
                    if let Some(network) = line.trim().strip_prefix("Network=") {
                        let network = network.trim();
                        if network.ends_with(".network")
                            && !self.files.iter().any(|entry| entry.path == network)
                        {
                            return Err(invalid("falta el archivo .network referenciado"));
                        }
                    }
                }
            }
        }
        if self.private_stack() {
            let marker = self
                .files
                .iter()
                .find(|file| file.path == "config/private-stack.json")
                .ok_or_else(|| invalid("falta marcador de stack"))?;
            let value: serde_json::Value = serde_json::from_str(&marker.contents)
                .map_err(|_| invalid("marcador de stack inválido"))?;
            if value != serde_json::json!({"version":1}) {
                return Err(invalid("marcador de stack incompatible"));
            }
            for path in [
                "private.network",
                "pocket-id.container",
                "rustfs.container",
                "netbird-server.container",
                "netbird-dashboard.container",
                "config/root.crt",
                "config/pocket.env",
                "config/rustfs.env",
                "config/netbird.yaml",
            ] {
                if !self.files.iter().any(|file| file.path == path) {
                    return Err(invalid("stack privado incompleto"));
                }
            }
        }
        Ok(())
    }

    pub fn private_stack(&self) -> bool {
        self.files
            .iter()
            .any(|file| file.path == "config/private-stack.json")
    }

    pub fn digest(&self) -> io::Result<String> {
        let bytes = zeroize::Zeroizing::new(serde_json::to_vec(self).map_err(io::Error::other)?);
        Ok(format!("{:x}", Sha256::digest(&*bytes)))
    }
}

fn validate_quadlet(text: &str) -> io::Result<()> {
    let mut section = "";
    let mut image = None;
    let mut pull = None;
    let mut install = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if line.ends_with('\\') {
            return Err(invalid("no se admiten continuaciones de línea"));
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1];
            if !matches!(section, "Unit" | "Container" | "Service" | "Install") {
                return Err(invalid("sección Quadlet no admitida"));
            }
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| invalid("directiva inválida"))?;
        let value = value.trim();
        let allowed = match section {
            "Unit" => matches!(key, "Description" | "After" | "Wants" | "Requires"),
            "Container" => matches!(
                key,
                "Image"
                    | "ContainerName"
                    | "Network"
                    | "NetworkAlias"
                    | "PublishPort"
                    | "Volume"
                    | "EnvironmentFile"
                    | "Environment"
                    | "User"
                    | "Exec"
                    | "WorkingDir"
                    | "Pull"
                    | "HealthCmd"
                    | "HealthInterval"
                    | "HealthTimeout"
                    | "HealthRetries"
                    | "HealthStartPeriod"
                    | "HealthOnFailure"
                    | "Notify"
                    | "DropCapability"
                    | "NoNewPrivileges"
            ),
            "Service" => matches!(
                key,
                "Restart" | "RestartSec" | "TimeoutStartSec" | "TimeoutStopSec"
            ),
            "Install" => key == "WantedBy" && value == "multi-user.target",
            _ => false,
        };
        if !allowed || value.is_empty() || value.contains('%') {
            return Err(invalid("directiva o privilegio no admitido"));
        }
        match key {
            "Image" => {
                if image.replace(value).is_some() {
                    return Err(invalid("Image repetido"));
                }
            }
            "Pull" => {
                if pull.replace(value).is_some() {
                    return Err(invalid("Pull repetido"));
                }
            }
            "Volume" | "EnvironmentFile" => {
                if !value.starts_with("/etc/gnx-app/config/")
                    && !value.starts_with("/var/lib/gnx-app/")
                {
                    return Err(invalid("mount/archivo fuera del directorio del guest"));
                }
                if value.contains("..") {
                    return Err(invalid("escape en mount/archivo"));
                }
            }
            "Network"
                if !matches!(value, "host" | "none" | "bridge")
                    && !(value.ends_with(".network")
                        && !value.contains('/')
                        && value.bytes().all(|b| {
                            b.is_ascii_lowercase() || b.is_ascii_digit() || b"-._".contains(&b)
                        })) =>
            {
                return Err(invalid("red no admitida"));
            }
            "NetworkAlias"
                if value.len() > 63
                    || !value.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"-.".contains(&b)
                    }) =>
            {
                return Err(invalid("alias de red inválido"));
            }
            "NoNewPrivileges" if value != "true" => {
                return Err(invalid("privilegios adicionales no admitidos"));
            }
            "DropCapability" if value != "all" => return Err(invalid("capacidades no admitidas")),
            "WantedBy" => install = true,
            _ => {}
        }
    }
    let image = image.ok_or_else(|| invalid("Quadlet sin imagen"))?;
    crate::images::validate(image, pull)?;
    if !install {
        return Err(invalid("Quadlet sin imagen o instalación nativa"));
    }
    Ok(())
}

fn validate_network(text: &str) -> io::Result<()> {
    let mut section = "";
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if line == "[Network]" {
            section = "Network";
            continue;
        }
        if section != "Network" || line.ends_with('\\') || line.contains('%') {
            return Err(invalid("sección o escape de red no admitido"));
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| invalid("directiva de red inválida"))?;
        let value = value.trim();
        match key {
            "Driver" if value == "bridge" => {}
            "NetworkName"
                if value.starts_with("gnx-app-")
                    && value.len() <= 63
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') => {}
            "Internal" | "IPv6" if matches!(value, "true" | "false") => {}
            _ => return Err(invalid("directiva de red fuera del subconjunto admitido")),
        }
    }
    if section != "Network" {
        return Err(invalid("falta sección Network"));
    }
    Ok(())
}
