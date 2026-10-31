use super::{
    identity::{Identity, Ownership},
    security,
};
use crate::{
    deployment::Bundle,
    protocol::{DISTRO, invalid},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resources {
    cores: u16,
    memory_mb: u32,
    swap_mb: u32,
    disk_gb: u16,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: u16,
    sid: String,
    digest: String,
    vmid: u32,
    complete: bool,
}

pub(super) fn command(
    identity: &Identity,
    args: &[&str],
    input: Option<&[u8]>,
    seconds: u64,
) -> io::Result<String> {
    let mut native = vec![
        "--distribution",
        DISTRO,
        "--exec",
        "/usr/bin/podman",
        "exec",
    ];
    if input.is_some() {
        native.push("-i");
    }
    native.push("gnx-core-compute");
    native.extend_from_slice(args);
    let output = if let Some(input) = input {
        identity.run_payload(&native, input, Duration::from_secs(seconds))?
    } else {
        identity.run(&native, Duration::from_secs(seconds))?
    };
    if output.code != 0 {
        return Err(io::Error::other(format!(
            "operación PVE no confirmada (exit {}); consultar journal y tareas PVE, no repetir a ciegas",
            output.code
        )));
    }
    Ok(output.text())
}

fn save(path: &std::path::Path, receipt: &Receipt) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec(receipt).map_err(io::Error::other)?)?;
    file.sync_all()
}

pub fn apply(bundle: &Bundle) -> io::Result<String> {
    bundle.validate()?;
    let owner = Ownership::read()?;
    let identity = Identity::open(&owner)?;
    let _directory = security::private_directory()?;
    if security::read_private("intent", 16)? != b"running" {
        return Err(io::Error::new(io::ErrorKind::NotConnected, "nodo detenido"));
    }
    let digest = bundle.digest()?;
    let private_stack = bundle.private_stack();
    if private_stack {
        let public = bundle
            .files
            .iter()
            .find(|file| file.path == "config/root.crt")
            .ok_or_else(|| invalid("falta CA pública del nodo"))?;
        let trusted = security::read_private("root.crt", 16384)?;
        if public.contents.trim()
            != std::str::from_utf8(&trusted)
                .map_err(|_| invalid("CA pública inválida"))?
                .trim()
        {
            return Err(invalid("la CA del stack no coincide con la del nodo"));
        }
    }
    let receipt_name = format!("deploy-{digest}.json");
    let path = security::state_dir()?.join(&receipt_name);
    let payload = std::env::current_exe()?
        .parent()
        .ok_or_else(|| invalid("ruta inválida"))?
        .join("payload");
    let resources: Resources = serde_json::from_slice(&fs::read(payload.join("config/lxc.json"))?)
        .map_err(io::Error::other)?;
    if !(1..=8).contains(&resources.cores)
        || !(1024..=16384).contains(&resources.memory_mb)
        || resources.swap_mb > 4096
        || !(4..=64).contains(&resources.disk_gb)
    {
        return Err(invalid("recursos LXC fuera de límite"));
    }
    let completed_name = format!("deploy-{digest}.complete.json");
    let completed = match security::read_private(&completed_name, 4096) {
        Ok(bytes) => Some(serde_json::from_slice::<Receipt>(&bytes).map_err(io::Error::other)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let reserved = match security::read_private(&receipt_name, 4096) {
        Ok(bytes) => Some(serde_json::from_slice::<Receipt>(&bytes).map_err(io::Error::other)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if let (Some(completed), Some(reserved)) = (&completed, &reserved) {
        if !completed.complete
            || completed.vmid != reserved.vmid
            || completed.digest != reserved.digest
            || completed.sid != reserved.sid
        {
            return Err(invalid("recibos LXC inconsistentes; no se modifica"));
        }
    } else if completed.is_some() {
        return Err(invalid("reserva LXC ausente; no se adopta"));
    }
    let saved = completed.or(reserved);
    let description = format!("GnX.Core:{digest}");
    let receipt = if let Some(receipt) = saved {
        if receipt.version != 1
            || receipt.sid != owner.sid
            || receipt.digest != digest
            || !(100..=999999999).contains(&receipt.vmid)
        {
            return Err(invalid("recibo LXC no coincide; no se adopta"));
        }
        receipt
    } else {
        if private_stack {
            let present = identity.run(
                &[
                    "--distribution",
                    DISTRO,
                    "--exec",
                    "/usr/bin/test",
                    "-e",
                    "/var/lib/gnx-core/private-stack.json",
                ],
                Duration::from_secs(15),
            )?;
            if present.code != 1 {
                return Err(invalid(
                    "ya existe un stack privado registrado o su estado no es verificable; no se reemplaza",
                ));
            }
            let guests: serde_json::Value = serde_json::from_str(&command(
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
            )?)
            .map_err(io::Error::other)?;
            let guests = guests
                .as_array()
                .ok_or_else(|| invalid("inventario PVE inválido"))?;
            if guests.len() > 256 {
                return Err(invalid("inventario PVE fuera de límite"));
            }
            for guest in guests {
                let vmid = guest["vmid"]
                    .as_u64()
                    .filter(|id| (100..=999999999).contains(id))
                    .ok_or_else(|| invalid("VMID inválido"))?;
                if guest["type"] == "lxc" {
                    let endpoint = format!("/nodes/gnx-core-compute/lxc/{vmid}/config");
                    let conf = command(
                        &identity,
                        &["pvesh", "get", &endpoint, "--output-format", "json"],
                        None,
                        15,
                    )?;
                    if conf.contains("172.30.0.100") {
                        return Err(invalid(
                            "dirección del stack ya asignada; no se adopta ni reemplaza el guest",
                        ));
                    }
                }
            }
        }
        crate::package::verify(&payload)?;
        let storage: serde_json::Value = serde_json::from_str(&command(
            &identity,
            &["pvesh", "get", "/storage/local", "--output-format", "json"],
            None,
            15,
        )?)
        .map_err(io::Error::other)?;
        if storage["type"] != "dir" || storage["path"] != "/var/lib/vz" {
            return Err(invalid("storage local no coincide con el runtime propio"));
        }
        let rootfs = payload.join("rootfs.tar");
        let mut source = fs::File::open(&rootfs)?;
        let mut hasher = Sha256::new();
        let mut bytes = [0u8; 65536];
        loop {
            let n = source.read(&mut bytes)?;
            if n == 0 {
                break;
            }
            hasher.update(&bytes[..n]);
        }
        let root_hash = format!("{:x}", hasher.finalize());
        let template_name = format!("gnx-{}.tar", &root_hash[..16]);
        let template_path = format!("/var/lib/vz/template/cache/{template_name}");
        let exists = identity.run(
            &[
                "--distribution",
                DISTRO,
                "--exec",
                "/usr/bin/podman",
                "exec",
                "gnx-core-compute",
                "test",
                "-f",
                &template_path,
            ],
            Duration::from_secs(15),
        )?;
        if exists.code == 1 {
            let linux = linux_path(&rootfs)?;
            let destination = format!("gnx-core-compute:{template_path}");
            let copied = identity.run(
                &[
                    "--distribution",
                    DISTRO,
                    "--exec",
                    "/usr/bin/podman",
                    "cp",
                    &linux,
                    &destination,
                ],
                Duration::from_secs(180),
            )?;
            if copied.code != 0 {
                return Err(io::Error::other("copia de base LXC no confirmada"));
            }
        } else if exists.code != 0 {
            return Err(io::Error::other("estado de base LXC no confirmado"));
        }
        let checksum = command(&identity, &["sha256sum", &template_path], None, 90)?;
        if checksum.split_whitespace().next() != Some(root_hash.as_str()) {
            return Err(invalid("base LXC distinta; no se sobrescribe"));
        }
        let next = command(&identity, &["pvesh", "get", "/cluster/nextid"], None, 15)?;
        let vmid: u32 = next
            .trim()
            .parse()
            .map_err(|_| invalid("VMID no confirmado"))?;
        if !(100..=999999999).contains(&vmid) {
            return Err(invalid("VMID inválido"));
        }
        let receipt = Receipt {
            version: 1,
            sid: owner.sid,
            digest,
            vmid,
            complete: false,
        };
        save(&path, &receipt)?; // Before mutation: a failed/unknown create cannot silently allocate another guest.
        let id = vmid.to_string();
        let hostname = format!("gnx-{}", &receipt.digest[..12]);
        command(
            &identity,
            &[
                "pct",
                "create",
                &id,
                &format!("local:vztmpl/{template_name}"),
                "--hostname",
                &hostname,
                "--unprivileged",
                "1",
                "--features",
                "nesting=1,keyctl=1,fuse=1",
                "--rootfs",
                &format!("local:{}", resources.disk_gb),
                "--cores",
                &resources.cores.to_string(),
                "--memory",
                &resources.memory_mb.to_string(),
                "--swap",
                &resources.swap_mb.to_string(),
                "--net0",
                if private_stack {
                    "name=eth0,bridge=vmbr0,ip=172.30.0.100/24,gw=172.30.0.1,type=veth"
                } else {
                    "name=eth0,bridge=vmbr0,ip=dhcp,type=veth"
                },
                "--nameserver",
                "172.30.0.1",
                "--ostype",
                "fedora",
                "--onboot",
                "1",
                "--description",
                &description,
            ],
            None,
            180,
        )?;
        receipt
    };
    let id = receipt.vmid.to_string();
    let endpoint = format!("/nodes/gnx-core-compute/lxc/{id}/config");
    let conf: serde_json::Value = serde_json::from_str(&command(
        &identity,
        &["pvesh", "get", &endpoint, "--output-format", "json"],
        None,
        15,
    )?)
    .map_err(io::Error::other)?;
    if conf["description"]
        .as_str()
        .map(|value| value.trim_end_matches('\n'))
        != Some(description.as_str())
        || conf["unprivileged"].as_u64() != Some(1)
    {
        return Err(invalid("propiedad del LXC no confirmada; no se modifica"));
    }
    if receipt.complete {
        let status = command(&identity, &["pct", "status", &id], None, 15)?;
        return Ok(format!(
            "LXC {id} ya registrado; {}. No se recrearon datos.",
            status.trim()
        ));
    }
    let status = command(&identity, &["pct", "status", &id], None, 15)?;
    if status.trim() == "status: stopped" {
        command(&identity, &["pct", "start", &id], None, 60)?;
    }
    let preparation = fs::read(payload.join("scripts/prepare-lxc.run"))?;
    command(
        &identity,
        &[
            "pct",
            "exec",
            &id,
            "--",
            "/usr/bin/bash",
            "-s",
            "--",
            if private_stack {
                "private-stack"
            } else {
                "dhcp"
            },
        ],
        Some(&preparation),
        60,
    )?;
    let mut units = Vec::new();
    for file in &bundle.files {
        let destination = if !file.path.contains('/') {
            if let Some(name) = file.path.strip_suffix(".container") {
                units.push(format!("{name}.service"));
            }
            format!("/etc/containers/systemd/{}", file.path)
        } else {
            format!("/etc/gnx-app/{}", file.path)
        };
        let parent = destination
            .rsplit_once('/')
            .ok_or_else(|| invalid("ruta inválida"))?
            .0;
        command(
            &identity,
            &[
                "pct",
                "exec",
                &id,
                "--",
                "/usr/bin/install",
                "-d",
                "-m",
                "0700",
                parent,
            ],
            None,
            15,
        )?;
        command(
            &identity,
            &[
                "pct",
                "exec",
                &id,
                "--",
                "/usr/bin/dd",
                &format!("of={destination}"),
                "status=none",
            ],
            Some(file.contents.as_bytes()),
            15,
        )?;
        command(
            &identity,
            &[
                "pct",
                "exec",
                &id,
                "--",
                "/usr/bin/chmod",
                if private_stack && file.path == "config/root.crt" {
                    "0644"
                } else {
                    "0600"
                },
                &destination,
            ],
            None,
            15,
        )?;
    }
    command(
        &identity,
        &[
            "pct",
            "exec",
            &id,
            "--",
            "/usr/bin/systemctl",
            "daemon-reload",
        ],
        None,
        30,
    )?;
    for file in &bundle.files {
        if file.path.ends_with(".container") && !file.path.contains('/') {
            for line in file.contents.lines() {
                if let Some(volume) = line.trim().strip_prefix("Volume=") {
                    let source = volume
                        .trim()
                        .split(':')
                        .next()
                        .ok_or_else(|| invalid("volume inválido"))?;
                    if source.starts_with("/var/lib/gnx-app/") {
                        command(
                            &identity,
                            &[
                                "pct",
                                "exec",
                                &id,
                                "--",
                                "/usr/bin/install",
                                "-d",
                                "-m",
                                "0700",
                                source,
                            ],
                            None,
                            15,
                        )?;
                    } else {
                        command(
                            &identity,
                            &["pct", "exec", &id, "--", "/usr/bin/test", "-f", source],
                            None,
                            15,
                        )?;
                    }
                }
            }
        }
    }
    let mut args = vec!["pct", "exec", &id, "--", "/usr/bin/systemctl", "start"];
    args.extend(units.iter().map(String::as_str));
    command(&identity, &args, None, 600)?;
    for unit in &units {
        let state = command(
            &identity,
            &[
                "pct",
                "exec",
                &id,
                "--",
                "/usr/bin/systemctl",
                "show",
                "--property=ActiveState",
                "--value",
                unit,
            ],
            None,
            15,
        )?;
        if state.trim() != "active" {
            return Err(io::Error::other(
                "no están activas todas las unidades solicitadas; consultar journal del guest",
            ));
        }
    }
    if private_stack {
        let input = serde_json::to_vec(
            &serde_json::json!({"vmid": receipt.vmid, "digest": receipt.digest}),
        )
        .map_err(io::Error::other)?;
        let output = identity.run_payload(
            &[
                "--distribution",
                DISTRO,
                "--exec",
                "/usr/bin/bash",
                "/etc/gnx-core/stack-access.run",
                "register",
            ],
            &input,
            Duration::from_secs(60),
        )?;
        if output.code != 0 {
            return Err(io::Error::other(
                "publicación del stack no confirmada; conservar recibo y consultar journal",
            ));
        }
        let enrollment = fs::read(payload.join("scripts/enroll-stack.run"))?;
        command(
            &identity,
            &["pct", "exec", &id, "--", "/usr/bin/python3", "-"],
            Some(&enrollment),
            360,
        )?;
        for unit in &units {
            let state = command(
                &identity,
                &[
                    "pct",
                    "exec",
                    &id,
                    "--",
                    "/usr/bin/systemctl",
                    "show",
                    "--property=ActiveState",
                    "--value",
                    unit,
                ],
                None,
                15,
            )?;
            if state.trim() != "active" {
                return Err(io::Error::other(
                    "servicio del stack no activo tras configurar OIDC",
                ));
            }
        }
    }
    let completed = Receipt {
        complete: true,
        ..receipt
    };
    // Keep the original reservation; completion is a separate create-only receipt.
    let completion =
        security::state_dir()?.join(format!("deploy-{}.complete.json", completed.digest));
    if !completion.exists() {
        save(&completion, &completed)?;
    }
    Ok(format!(
        "LXC {id}: {} servicios activos mediante Quadlets. Salud y login de aplicaciones se comprueban por separado.",
        units.len()
    ))
}

fn linux_path(path: &std::path::Path) -> io::Result<String> {
    let path = path.to_string_lossy().replace('\\', "/");
    if path.len() < 3 || path.as_bytes()[1] != b':' {
        return Err(invalid("payload fuera de unidad local"));
    }
    Ok(format!(
        "/mnt/{}/{}",
        path[..1].to_ascii_lowercase(),
        &path[3..]
    ))
}
