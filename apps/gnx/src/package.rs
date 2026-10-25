use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::{Component, Path},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u16,
    architecture: String,
    files: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    sha256: String,
}

pub fn verify(root: &Path) -> io::Result<()> {
    let file = fs::File::open(root.join("manifest.json"))?;
    if file.metadata()?.len() > 65536 {
        return Err(io::Error::other("manifiesto fuera de límite"));
    }
    let manifest: Manifest = serde_json::from_reader(file.take(65537))
        .map_err(|_| io::Error::other("manifiesto inválido"))?;
    if manifest.version != 1 || manifest.architecture != "x86_64" || manifest.files.len() > 256 {
        return Err(io::Error::other("paquete incompatible"));
    }
    let required = [
        "rootfs.tar",
        "images/mesh.oci.tar",
        "images/dns.oci.tar",
        "images/gateway.oci.tar",
        "images/compute.oci.tar",
        "scripts/install.run",
        "config/wsl.conf",
        "config/images.json",
    ];
    for name in required {
        if !manifest.files.iter().any(|entry| entry.path == name) {
            return Err(io::Error::other(format!("paquete incompleto: {name}")));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for entry in manifest.files {
        let path = Path::new(&entry.path);
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || entry.path.contains([':', '\\'])
            || !seen.insert(entry.path.to_ascii_lowercase())
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(io::Error::other("entrada de paquete inválida"));
        }
        let mut parent = root.to_path_buf();
        for part in path.components() {
            parent.push(part.as_os_str());
            let metadata = fs::symlink_metadata(&parent)?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err(io::Error::other("enlace en el paquete"));
                }
            }
            if metadata.file_type().is_symlink() {
                return Err(io::Error::other("enlace en el paquete"));
            }
        }
        let mut file = fs::File::open(root.join(path))?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 65536];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        if format!("{:x}", hash.finalize()) != entry.sha256.to_ascii_lowercase() {
            return Err(io::Error::other(format!("hash inválido: {}", entry.path)));
        }
    }
    Ok(())
}
