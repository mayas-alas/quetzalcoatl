use super::{
    identity::{Identity, Ownership},
    security,
};
use crate::{
    deployment::{Bundle, File},
    protocol::{DISTRO, invalid},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    time::Duration,
};

const DEFAULT_INPUT: &str = "default-private-services.json";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DefaultInput {
    version: u16,
    owner: Ownership,
    bundle: Bundle,
}

// These are fixed stage messages, never process output, configuration or secrets.
fn log(progress: &tokio::sync::mpsc::Sender<String>, message: impl Into<String>) {
    let _ = progress.try_send(message.into());
}

fn random_key() -> io::Result<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
    security::random(bytes.as_mut())?;
    let mut encoded = String::with_capacity(44);
    for chunk in bytes.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            encoded.push(if index <= chunk.len() {
                ALPHABET[((bits >> shift) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    Ok(encoded)
}

fn saved_default() -> io::Result<DefaultInput> {
    let bytes = zeroize::Zeroizing::new(security::read_private(DEFAULT_INPUT, 128 * 1024)?);
    let saved: DefaultInput = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("entrada private-services guardada inválida; no se regenera"))?;
    if saved.version != 1 || !saved.bundle.private_stack() {
        return Err(invalid("entrada private-services incompatible"));
    }
    saved.bundle.validate()?;
    Ok(saved)
}

pub(super) fn validate_default_owner(sid: &str) -> io::Result<()> {
    if saved_default()?.owner.sid != sid {
        return Err(invalid("entrada private-services ajena"));
    }
    Ok(())
}

fn default_bundle() -> io::Result<Bundle> {
    let owner = Ownership::read()?;
    match saved_default() {
        Ok(saved) => {
            if saved.owner.sid != owner.sid || saved.owner.distribution_id != owner.distribution_id
            {
                return Err(invalid(
                    "private-services pertenece a otro nodo; no se reutiliza",
                ));
            }
            return Ok(saved.bundle);
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut files = Vec::new();
    for (path, contents) in [
        (
            "private.network",
            include_str!("../../../../stacks/private-services/quadlets/private.network"),
        ),
        (
            "pocket-id.container",
            include_str!("../../../../stacks/private-services/quadlets/pocket-id.container"),
        ),
        (
            "rustfs.container",
            include_str!("../../../../stacks/private-services/quadlets/rustfs.container"),
        ),
        (
            "netbird-server.container",
            include_str!("../../../../stacks/private-services/quadlets/netbird-server.container"),
        ),
        (
            "netbird-dashboard.container",
            include_str!(
                "../../../../stacks/private-services/quadlets/netbird-dashboard.container"
            ),
        ),
        (
            "config/private-stack.json",
            include_str!("../../../../stacks/private-services/quadlets/config/private-stack.json"),
        ),
    ] {
        files.push(File {
            path: path.into(),
            contents: contents.into(),
        });
    }
    let mut access_bytes = zeroize::Zeroizing::new([0u8; 16]);
    security::random(access_bytes.as_mut())?;
    let access_key = zeroize::Zeroizing::new(
        access_bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    );
    for (path, contents) in [
        (
            "config/root.crt",
            String::from_utf8(security::read_private("root.crt", 16384)?)
                .map_err(|_| invalid("CA pública inválida"))?,
        ),
        (
            "config/pocket.env",
            format!(
                "ENCRYPTION_KEY={}\nSTATIC_API_KEY={}\n",
                random_key()?,
                random_key()?
            ),
        ),
        (
            "config/rustfs.env",
            format!(
                "RUSTFS_ACCESS_KEY={}\nRUSTFS_SECRET_KEY={}\n",
                access_key.as_str(),
                &random_key()?[..32]
            ),
        ),
        (
            "config/netbird.yaml",
            include_str!("../../../../stacks/private-services/netbird.yaml")
                .replace(
                    "authSecret: \"\"",
                    &format!("authSecret: \"{}\"", random_key()?),
                )
                .replace(
                    "encryptionKey: \"\"",
                    &format!("encryptionKey: \"{}\"", random_key()?),
                ),
        ),
    ] {
        files.push(File {
            path: path.into(),
            contents,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let saved = DefaultInput {
        version: 1,
        owner,
        bundle: Bundle { files },
    };
    saved.bundle.validate()?;
    let _directory = security::private_directory()?;
    let bytes = zeroize::Zeroizing::new(serde_json::to_vec(&saved).map_err(io::Error::other)?);
    // Persist the exact input before reserving a VMID. Never rotate keys on retry
    // or silently use new embedded definitions for an existing deployment.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(security::state_dir()?.join(DEFAULT_INPUT))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(saved.bundle)
}

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

pub fn execute(
    bundle: Option<Bundle>,
    progress: &tokio::sync::mpsc::Sender<String>,
) -> io::Result<String> {
    security::require_system()?;
    log(progress, "Validando entrada, propiedad y CA del nodo.");
    let owner = Ownership::read()?;
    let identity = Identity::open(&owner)?;
    let _directory = security::private_directory()?;
    if security::read_private("intent", 16)? != b"running" {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "nodo detenido; usa gnx start",
        ));
    }
    let bundle = match bundle {
        Some(bundle) => bundle,
        None => {
            log(
                progress,
                "Preparando private-services y su configuración persistente protegida.",
            );
            default_bundle()?
        }
    };
    bundle.validate()?;
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
        log(
            progress,
            format!(
                "Recibo existente: LXC {}. Verificando propiedad sin asignar otro VMID.",
                receipt.vmid
            ),
        );
        if receipt.version != 1
            || receipt.sid != owner.sid
            || receipt.digest != digest
            || !(100..=999999999).contains(&receipt.vmid)
        {
            return Err(invalid("recibo LXC no coincide; no se adopta"));
        }
        receipt
    } else {
        log(
            progress,
            "Comprobando inventario PVE, paquete y almacenamiento.",
        );
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
            log(progress, "Copiando y verificando la base LXC.");
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
        log(progress, format!("Reserva persistida: creando LXC {vmid}."));
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
                "--tags",
                "gnx",
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
        log(
            progress,
            format!("LXC {id} ya completado: consultando su estado, sin reinstalar."),
        );
        let status = command(&identity, &["pct", "status", &id], None, 15)?;
        return Ok(format!(
            "LXC {id} ya registrado; {}. No se recrearon datos.",
            status.trim()
        ));
    }
    let status = command(&identity, &["pct", "status", &id], None, 15)?;
    log(
        progress,
        format!(
            "LXC {id}: preparando guest y transfiriendo Quadlets/configuración (contenido omitido)."
        ),
    );
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
    log(
        progress,
        "Iniciando unidades: Podman descarga las imágenes fijadas si faltan; consulta journal del guest para detalle.",
    );
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
            return Err(io::Error::other(format!(
                "{unit} no está activa; consultar journal del guest"
            )));
        }
        log(progress, format!("LXC {id}: {unit} activa."));
    }
    if private_stack {
        log(progress, "Registrando publicación HTTPS/DNS del stack.");
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
        log(
            progress,
            "Configurando clientes OIDC Pocket ID / RustFS / NetBird; secretos omitidos.",
        );
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
        log(
            progress,
            "OIDC configurado y unidades activas; alta humana pendiente en https://pocket.gnx/setup.",
        );
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
