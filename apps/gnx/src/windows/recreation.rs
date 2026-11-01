use super::{
    identity::{Identity, Ownership},
    security,
};
use crate::protocol::{DISTRO, invalid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io, time::Duration};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct Inventory {
    pub sid: String,
    pub distribution: String,
    pub guests: Vec<Guest>,
    pub configuration_digest: String,
    pub retired_files: Vec<String>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct Guest {
    pub vmid: u64,
    pub kind: String,
    pub status: String,
}
fn files(sid: &str) -> io::Result<Vec<String>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(super::bootstrap::private_root()?)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("nombre de estado no válido"))?;
        if [
            "owner",
            "account.json",
            "account.dpapi",
            "maintenance.lock",
            "product-update.json",
            "product-update.exe",
            "product-update.msi",
            "product-update-bundle.exe",
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        let _checked = security::private_file(&name)?;
        if entry.file_type()?.is_dir() {
            return Err(invalid("directorio de estado no reconocido; no se recrea"));
        }
        if name.starts_with("deploy-") && name.ends_with(".json") {
            let receipt: serde_json::Value =
                serde_json::from_slice(&security::read_private(&name, 4096)?)
                    .map_err(|_| invalid("recibo de despliegue inválido"))?;
            let digest = receipt["digest"]
                .as_str()
                .ok_or_else(|| invalid("recibo sin digest"))?;
            if receipt["sid"].as_str() != Some(sid)
                || digest.len() != 64
                || !digest.bytes().all(|c| c.is_ascii_hexdigit())
                || (name != format!("deploy-{digest}.json")
                    && name != format!("deploy-{digest}.complete.json"))
            {
                return Err(invalid("recibo no pertenece a esta identidad"));
            }
        } else if ![
            "prepared",
            "identity.json",
            "intent",
            "applied-version",
            "updates-catalog.json",
            "root.crt",
            "portproxy",
            "portproxy-80",
            "revocation",
            "trace.log",
        ]
        .contains(&name.as_str())
        {
            return Err(invalid(
                "estado no reconocido; se conserva y se bloquea la recreación",
            ));
        }
        result.push(name);
    }
    result.sort();
    Ok(result)
}

pub(super) fn inventory() -> io::Result<Inventory> {
    security::require_system()?;
    let owner = Ownership::read()?;
    if super::bootstrap::owned_account_sid()? != owner.sid {
        return Err(invalid("SID de servicio divergente"));
    }
    let identity = Identity::open(&owner)?;
    let running = identity.run(&["--list", "--running", "--quiet"], Duration::from_secs(15))?;
    if running.code != 0 || !running.text().lines().any(|s| s.trim() == DISTRO) {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "WSL detenida; no se inicia para inventariar",
        ));
    }
    let managed = identity.run(&["--distribution", DISTRO, "--exec", "/usr/bin/bash", "-c", "set -eu; for p in /var/lib/gnx-core /var/lib/gnx-core/compute /var/lib/gnx-core/compute-config; do test -d \"$p\"; test ! -L \"$p\"; test \"$(stat -c %d \"$p\")\" = \"$(stat -c %d /)\"; done"], Duration::from_secs(15))?;
    if managed.code != 0 {
        return Err(invalid(
            "datos compute fuera del filesystem WSL propio; no se recrea",
        ));
    }
    let compute = identity.run(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/podman",
            "container",
            "exists",
            "gnx-core-compute",
        ],
        Duration::from_secs(15),
    )?;
    let (mut guests, configuration_digest) = if compute.code == 1 {
        let empty = identity.run(&["--distribution", DISTRO, "--exec", "/usr/bin/bash", "-c", "test ! -e /var/lib/gnx-core/secrets/compute.env && test -z \"$(find /var/lib/gnx-core/compute -mindepth 1 -maxdepth 1 -print -quit)\""], Duration::from_secs(15))?;
        if empty.code != 0 {
            return Err(invalid(
                "compute ausente con estado persistente; inventario no confirmado",
            ));
        }
        (Vec::new(), format!("{:x}", Sha256::digest(b"unconfigured")))
    } else {
        if compute.code != 0 {
            return Err(invalid("compute no observado"));
        }
        let mount = identity.run(
            &[
                "--distribution",
                DISTRO,
                "--exec",
                "/usr/bin/podman",
                "inspect",
                "--format",
                "{{range .Mounts}}{{if eq .Destination \"/var/lib/vz\"}}{{.Source}}{{end}}{{end}}",
                "gnx-core-compute",
            ],
            Duration::from_secs(15),
        )?;
        if mount.code != 0 || mount.text().trim() != "/var/lib/gnx-core/compute" {
            return Err(invalid(
                "montaje compute no pertenece al almacenamiento declarado",
            ));
        }
        let storage = super::deploy::command(
            &identity,
            &["pvesh", "get", "/storage", "--output-format", "json"],
            None,
            30,
        )?;
        let storages: Vec<serde_json::Value> = serde_json::from_str(&storage)
            .map_err(|_| invalid("almacenamiento PVE no observado"))?;
        for value in &storages {
            if value["storage"].as_str() != Some("local")
                || value["type"].as_str() != Some("dir")
                || value["path"].as_str() != Some("/var/lib/vz")
            {
                return Err(invalid(
                    "almacenamiento externo o no reconocido; no se recrea ni se adopta",
                ));
            }
        }
        if storages.len() != 1 {
            return Err(invalid("almacenamiento administrado no confirmado"));
        }
        let resources = super::deploy::command(
            &identity,
            &[
                "pvesh",
                "get",
                "/cluster/resources",
                "--type",
                "vm",
                "--output-format",
                "json",
            ],
            None,
            30,
        )?;
        let mut resources: Vec<serde_json::Value> =
            serde_json::from_str(&resources).map_err(|_| invalid("inventario PVE inválido"))?;
        resources.sort_by_key(|v| v["vmid"].as_u64());
        if resources.len() > 1024 {
            return Err(invalid("demasiados guests para inventariar"));
        }
        let mut guests = Vec::new();
        let mut hash = Sha256::new();
        hash.update(serde_json::to_vec(&storages).map_err(io::Error::other)?);
        for resource in resources {
            let vmid = resource["vmid"]
                .as_u64()
                .filter(|id| (100..=999999999).contains(id))
                .ok_or_else(|| invalid("VMID inválido"))?;
            let kind = resource["type"]
                .as_str()
                .filter(|v| ["lxc", "qemu"].contains(v))
                .ok_or_else(|| invalid("tipo de guest inválido"))?
                .to_owned();
            if resource["node"].as_str() != Some("gnx-core-compute") {
                return Err(invalid("guest de nodo externo; no se adopta"));
            }
            let status = resource["status"]
                .as_str()
                .filter(|v| ["running", "stopped"].contains(v))
                .ok_or_else(|| invalid("estado de guest no confirmado"))?
                .to_owned();
            let config = super::deploy::command(
                &identity,
                &[
                    "pvesh",
                    "get",
                    &format!("/nodes/gnx-core-compute/{kind}/{vmid}/config"),
                    "--output-format",
                    "json",
                ],
                None,
                30,
            )?;
            let config_value: serde_json::Value = serde_json::from_str(&config)
                .map_err(|_| invalid("configuración de guest inválida"))?;
            if let Some(object) = config_value.as_object() {
                for (key, value) in object {
                    if key.starts_with("mp")
                        || key == "rootfs"
                        || key.starts_with("scsi")
                        || key.starts_with("virtio")
                        || key.starts_with("sata")
                        || key.starts_with("ide")
                    {
                        let value = value
                            .as_str()
                            .ok_or_else(|| invalid("disco no reconocido"))?;
                        if !value.starts_with("local:") && value != "none,media=cdrom" {
                            return Err(invalid(
                                "guest con bind/disco externo; se bloquea la recreación",
                            ));
                        }
                    }
                }
            }
            hash.update(vmid.to_le_bytes());
            hash.update(serde_json::to_vec(&config_value).map_err(io::Error::other)?);
            guests.push(Guest { vmid, kind, status });
        }
        (guests, format!("{:x}", hash.finalize()))
    };
    guests.sort_by_key(|g| g.vmid);
    Ok(Inventory {
        sid: owner.sid.clone(),
        distribution: owner.distribution_id,
        guests,
        configuration_digest,
        retired_files: files(&owner.sid)?,
    })
}

pub(super) fn execute(expected: &Inventory) -> io::Result<()> {
    // Called only by the installed executable as SYSTEM, with GnXCore stopped
    // and the worker holding the cross-process maintenance lease.
    security::require_system()?;
    if &inventory()? != expected {
        return Err(invalid("inventario cambió; no se retira el nodo"));
    }
    let owner = Ownership::read()?;
    let identity = Identity::open(&owner)?;
    super::node::retire()?;
    // Identity::open validated SID, GUID and the dedicated profile/base path.
    // Never unregister by an unverified display name or recursively delete a path.
    let checked = Ownership::read()?;
    if checked.sid != expected.sid || checked.distribution_id != expected.distribution {
        return Err(invalid("propiedad cambió"));
    }
    let _verified = Identity::open(&checked)?;
    let result = identity.run(&["--unregister", DISTRO], Duration::from_secs(180))?;
    if result.code != 0 {
        return Err(io::Error::other("retirada WSL no confirmada; no repetir"));
    }
    drop(_verified);
    drop(identity);
    let root = super::bootstrap::private_root()?;
    for name in &expected.retired_files {
        match security::private_file(name) {
            Ok(file) => {
                drop(file);
                fs::remove_file(root.join(name))?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound && name == "applied-version" => {}
            Err(error) => return Err(error),
        }
    }
    // Windows service-account scaffold stays private. WSL, node CA, mesh identity,
    // PVE credential/configuration, managed storage and guests are new/empty.
    super::bootstrap::prepare()
}
