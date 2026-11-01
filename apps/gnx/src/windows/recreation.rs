use super::{
    identity::{Identity, Ownership},
    security,
};
use crate::protocol::{DISTRO, invalid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct Inventory {
    pub sid: String,
    pub distribution: String,
    pub guests: Vec<Guest>,
    pub configuration_digest: String,
    pub retired_files: Vec<String>,
    #[serde(default)]
    pub guests_observed: bool,
    #[serde(default)]
    pub removed: bool,
    #[serde(default)]
    pub partial: bool,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct Guest {
    pub vmid: u64,
    pub kind: String,
    pub status: String,
}

impl Inventory {
    pub(super) fn difference(&self, expected: &Self) -> Option<&'static str> {
        if self.sid != expected.sid {
            Some("SID Windows")
        } else if self.distribution != expected.distribution {
            Some("GUID WSL")
        } else if self.guests != expected.guests {
            Some("guests o estados PVE")
        } else if self.configuration_digest != expected.configuration_digest {
            Some("configuración PVE")
        } else if self.retired_files != expected.retired_files {
            Some("recibos Windows")
        } else {
            None
        }
    }
}

// PVE renders storage.content from an unordered Perl set. JSON key ordering
// alone cannot canonicalize a set encoded as a comma-separated string.
fn canonical_content(value: &str) -> io::Result<String> {
    let mut items: Vec<_> = value.split(',').collect();
    if items
        .iter()
        .any(|item| item.is_empty() || !item.bytes().all(|c| c.is_ascii_lowercase()))
    {
        return Err(invalid("tipos de contenido PVE no reconocidos"));
    }
    items.sort_unstable();
    items.dedup();
    Ok(items.join(","))
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
        let mut storages: Vec<serde_json::Value> = serde_json::from_str(&storage)
            .map_err(|_| invalid("almacenamiento PVE no observado"))?;
        for value in &mut storages {
            if value["storage"].as_str() != Some("local")
                || value["type"].as_str() != Some("dir")
                || value["path"].as_str() != Some("/var/lib/vz")
            {
                return Err(invalid(
                    "almacenamiento externo o no reconocido; no se recrea ni se adopta",
                ));
            }
            let content = value["content"]
                .as_str()
                .ok_or_else(|| invalid("contenido PVE no observado"))?;
            value["content"] = canonical_content(content)?.into();
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
        guests_observed: true,
        removed: false,
        partial: false,
    })
}

// Force authorizes the entire registered WSL disk, not a running PVE stack.
// A cold/unreachable PVE cannot provide a current guest list: label the last
// confirmed list explicitly rather than starting WSL or fabricating evidence.
pub(super) fn force_inventory(previous: Option<&Inventory>, resume: bool) -> io::Result<Inventory> {
    security::require_system()?;
    let owner = match Ownership::read() {
        Ok(owner) => owner,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if !resume {
                return Err(invalid(
                    "sin retirada pendiente no se continúa un bootstrap",
                ));
            }
            let expected = previous
                .ok_or_else(|| invalid("sin identidad anterior no se continúa una retirada"))?;
            if let Some(owner) = pending_target(expected)? {
                let retired_files = files(&owner.sid)?;
                return Ok(Inventory {
                    sid: owner.sid,
                    distribution: owner.distribution_id,
                    guests: Vec::new(),
                    configuration_digest: expected.configuration_digest.clone(),
                    retired_files,
                    guests_observed: false,
                    removed: false,
                    partial: true,
                });
            }
            verify_removed(expected)?;
            let mut inventory = expected.clone();
            inventory.removed = true;
            inventory.guests_observed = false;
            inventory.retired_files.clear();
            return Ok(inventory);
        }
        Err(error) => return Err(error),
    };
    if super::bootstrap::owned_account_sid()? != owner.sid {
        return Err(invalid("SID de servicio divergente"));
    }
    let identity = Identity::for_retirement(&owner)?;
    let retired_files = files(&owner.sid)?;
    let mut guests = previous
        .filter(|p| p.sid == owner.sid && p.distribution == owner.distribution_id)
        .map(|p| p.guests.clone())
        .unwrap_or_default();
    let mut guests_observed = false;
    let running = identity.run(&["--list", "--running", "--quiet"], Duration::from_secs(15));
    if running.as_ref().is_ok_and(|running| {
        running.code == 0 && running.text().lines().any(|s| s.trim() == DISTRO)
    }) {
        if let Ok(output) = super::deploy::command(
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
            15,
        ) {
            if let Ok(resources) = serde_json::from_str::<Vec<serde_json::Value>>(&output) {
                let parsed: Option<Vec<_>> = resources
                    .iter()
                    .map(|value| {
                        Some(Guest {
                            vmid: value["vmid"]
                                .as_u64()
                                .filter(|id| (100..=999999999).contains(id))?,
                            kind: value["type"]
                                .as_str()
                                .filter(|kind| ["lxc", "qemu"].contains(kind))?
                                .to_owned(),
                            status: value["status"]
                                .as_str()
                                .filter(|status| ["running", "stopped"].contains(status))?
                                .to_owned(),
                        })
                    })
                    .collect();
                if resources.len() <= 1024
                    && resources
                        .iter()
                        .all(|v| v["node"].as_str() == Some("gnx-core-compute"))
                {
                    if let Some(parsed) = parsed {
                        guests = parsed;
                        guests_observed = true;
                    }
                }
            }
        }
    }
    guests.sort_by_key(|guest| guest.vmid);
    Ok(Inventory {
        sid: owner.sid,
        distribution: owner.distribution_id,
        guests,
        configuration_digest: format!("{:x}", Sha256::digest(b"entire-owned-wsl-disk")),
        retired_files,
        guests_observed,
        removed: false,
        partial: false,
    })
}

// One ownership boundary for interrupted force and native installer retirement.
pub(super) fn pending_target(expected: &Inventory) -> io::Result<Option<Ownership>> {
    if super::bootstrap::owned_account_sid()? != expected.sid {
        return Err(invalid("SID parcial divergente"));
    }
    match Ownership::record() {
        Ok(owner) => {
            if owner.sid != expected.sid
                || (expected.partial && owner.distribution_id != expected.distribution)
            {
                return Err(invalid("identidad parcial cambió"));
            }
            let _verified = Identity::for_retirement(&owner)?;
            return Ok(Some(owner));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    if !files(&expected.sid)?.is_empty() {
        return Err(invalid(
            "estado parcial sin recibo de identidad; no se adopta",
        ));
    }
    let previous = Ownership {
        version: 1,
        sid: expected.sid.clone(),
        distribution_id: expected.distribution.clone(),
    };
    if expected.partial {
        let _verified = Identity::for_retirement(&previous)?;
        return Ok(Some(previous));
    }
    let (_, current) = Identity::inspect_recreation(&previous)?;
    if current.is_some() && !expected.removed {
        return Err(invalid("nodo nuevo sin retirada confirmada; no se adopta"));
    }
    Ok(current)
}

fn target_owner(expected: &Inventory) -> io::Result<Ownership> {
    let owner = if expected.partial {
        pending_target(expected)?
            .ok_or_else(|| invalid("WSL parcial ausente; no se sustituye por otra"))?
    } else {
        Ownership::read()?
    };
    if owner.sid != expected.sid
        || owner.distribution_id != expected.distribution
        || super::bootstrap::owned_account_sid()? != owner.sid
    {
        return Err(invalid("identidad de la WSL confirmada cambió"));
    }
    Ok(owner)
}

fn verify_removed(expected: &Inventory) -> io::Result<()> {
    security::require_system()?;
    if super::bootstrap::owned_account_sid()? != expected.sid || !files(&expected.sid)?.is_empty() {
        return Err(invalid(
            "cuenta divergente o estado de nodo presente; no se continúa el bootstrap",
        ));
    }
    let owner = Ownership {
        version: 1,
        sid: expected.sid.clone(),
        distribution_id: expected.distribution.clone(),
    };
    let identity = Identity::after_removal(&owner)?;
    let path = identity.profile_dir().join(r"AppData\Local\GnX Core\wsl");
    match fs::read_dir(path) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                Err(invalid("directorio WSL no vacío; no se adopta ni se borra"))
            } else {
                Ok(())
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn verify_target(expected: &Inventory) -> io::Result<()> {
    if expected.removed {
        return verify_removed(expected);
    }
    let owner = target_owner(expected)?;
    let _verified = Identity::for_retirement(&owner)?;
    if files(&owner.sid)? != expected.retired_files {
        return Err(invalid(
            "recibos Windows cambiaron; no se retiran recursos no confirmados",
        ));
    }
    Ok(())
}

pub(super) fn execute(expected: &Inventory) -> io::Result<()> {
    // Called only by the installed executable as SYSTEM, with GnXCore stopped
    // and the worker holding the cross-process maintenance lease.
    security::require_system()?;
    verify_target(expected)?;
    if expected.removed {
        super::service::report_information(
            "Continuación: WSL anterior ausente y scaffold verificado; solo bootstrap, sin unregister.",
        );
        return super::bootstrap::prepare();
    }
    let owner = target_owner(expected)?;
    let identity = Identity::for_retirement(&owner)?;
    // This is not graceful uninstall. Do not gate destructive unregister on
    // systemctl/PVE availability or preserve the old disk as a fallback.
    super::access::remove()?;
    // Retirement identity validated SID, GUID and the dedicated profile/base path.
    // Never unregister by an unverified display name or recursively delete a path.
    let checked = target_owner(expected)?;
    if checked.sid != expected.sid || checked.distribution_id != expected.distribution {
        return Err(invalid("propiedad cambió"));
    }
    let _verified = Identity::for_retirement(&checked)?;
    let result = identity.run(&["--unregister", DISTRO], Duration::from_secs(180))?;
    if result.code != 0 {
        return Err(io::Error::other(format!(
            "unregister de WSL propia devolvió {}; no repetir sin comprobar estado",
            result.code
        )));
    }
    identity.verify_unregistered(&checked)?;
    super::service::report_information(
        "Recreación: GUID WSL anterior retirado y VHD anterior ausente; preparando el nodo desde cero.",
    );
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

pub(super) fn confirm(inventory: &Inventory) -> io::Result<bool> {
    let word = if inventory.removed {
        println!(
            "CONTINUACIÓN DE PREPARACIÓN: WSL anterior {} (SID {}) ya ausente.",
            inventory.distribution, inventory.sid
        );
        println!(
            "Se verificó la ausencia del GUID, del VHD y de otra WSL GnX en la cuenta propia. No se repite --unregister ni se borra otro nodo; solo se prepara la base desde el payload instalado."
        );
        "PREPARAR"
    } else {
        println!(
            "RECREACIÓN SIN RECUPERACIÓN AUTOMÁTICA: WSL {} (SID {}).",
            inventory.distribution, inventory.sid
        );
        if inventory.partial {
            println!(
                "Nodo parcial: esta autorización retira ESTE GUID y su VHD actual; no es una continuación conservadora."
            );
        }
        println!(
            "Se pierden datos, CA, identidad mesh, credencial PVE, configuración y TODOS los guests del almacenamiento administrado:"
        );
        println!(
            "Se retira el registro WSL propio mediante --unregister y su VHD si aún existe; no se conserva copia ni se vuelve al nodo anterior."
        );
        if !inventory.guests_observed {
            println!(
                "PVE no proporcionó una lista actual. La lista siguiente es la última confirmada o está vacía; la autorización incluye TODO el contenido del VHD, también guests no observables ahora."
            );
        }
        for guest in &inventory.guests {
            println!("  {} {}: {}", guest.kind, guest.vmid, guest.status);
        }
        println!(
            "Recibos Windows retirados: {}. Se conserva solo la cuenta Windows privada de servicio y los archivos de esta transacción. No Windows, otras WSL ni recursos externos.",
            inventory.retired_files.join(", ")
        );
        "RECREAR"
    };
    print!("Escribe {word} para autorizar la operación indicada: ");
    io::stdout().flush()?;
    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation)?;
    Ok(confirmation.trim_end_matches(['\r', '\n']) == word)
}

#[cfg(test)]
mod checks {
    use super::canonical_content;

    #[test]
    fn content_order_is_semantically_stable() {
        assert_eq!(
            canonical_content("images,rootdir,iso").unwrap(),
            canonical_content("iso,images,rootdir").unwrap()
        );
    }

    #[test]
    fn content_changes_are_not_ignored() {
        assert_ne!(
            canonical_content("images,rootdir,iso").unwrap(),
            canonical_content("images,rootdir").unwrap()
        );
        assert!(canonical_content("images,,rootdir").is_err());
    }
}
