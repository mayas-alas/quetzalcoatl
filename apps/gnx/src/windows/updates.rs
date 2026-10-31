use super::{
    identity::{Identity, Ownership},
    security,
};
use crate::{
    protocol::{DISTRO, NodeState, invalid},
    updates::{Catalog, Component, MAX_CATALOG, NAMES},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::PathBuf,
    time::Duration,
};

const OPERATIONS: &[u8] = include_bytes!("../../../../scripts/update-component.run");
const CONSOLE_INSTALL: &[u8] = include_bytes!("../../../../scripts/install-console.run");
const CONSOLE_HASH: &str = "1877193e802632d2f0e2b2995a06b26e01d7f077d624e5917826ed05b5f7d635";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stack {
    vmid: u32,
    digest: String,
    mac: String,
}

#[derive(Deserialize)]
struct Receipt {
    version: u16,
    sid: String,
    digest: String,
    vmid: u32,
    complete: bool,
}

#[derive(Deserialize)]
struct Running {
    image: String,
    status: String,
}

#[derive(Deserialize)]
struct Snapshot {
    configured: String,
    active: String,
    owned: bool,
    pending: bool,
    running: Option<Running>,
    local_id: String,
    source_hash: String,
    running_guests: usize,
}

struct Context<'a> {
    identity: &'a Identity,
    stack: Option<Stack>,
    stack_running: bool,
}

fn selected(requested: Option<Catalog>) -> io::Result<Catalog> {
    if let Some(catalog) = requested {
        catalog.validate()?;
        return Ok(catalog);
    }
    match security::read_private("updates-catalog.json", MAX_CATALOG as u64) {
        Ok(bytes) => Catalog::decode(&bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Catalog::builtin(),
        Err(e) => Err(e),
    }
}

fn core(identity: &Identity, script: &[u8], args: &[&str], seconds: u64) -> io::Result<String> {
    let mut native = vec![
        "--distribution",
        DISTRO,
        "--exec",
        "/usr/bin/bash",
        "-s",
        "--",
    ];
    native.extend_from_slice(args);
    let output = identity.run_payload(&native, script, Duration::from_secs(seconds))?;
    if output.code != 0 {
        return Err(io::Error::other(format!(
            "operación de actualización exit {}; consultar journal, no repetir sin comprobar estado",
            output.code
        )));
    }
    Ok(output.text())
}

impl<'a> Context<'a> {
    fn new(identity: &'a Identity, owner: &Ownership) -> io::Result<Self> {
        let running = identity.run(&["--list", "--running", "--quiet"], Duration::from_secs(10))?;
        if running.code != 0 {
            return Err(io::Error::other("estado WSL no confirmado"));
        }
        if security::read_private("intent", 16)? != b"running"
            || !running.text().lines().any(|s| s.trim() == DISTRO)
        {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "nodo detenido; no se inicia para actualizar",
            ));
        }
        let stack = core(identity, br#"set -euo pipefail
test "$(id -u)" = 0
systemctl is-active --quiet gnx-core.target
p=/var/lib/gnx-core/private-stack.json
if [[ ! -e "$p" ]]; then printf null; exit 0; fi
[[ -f "$p" && ! -L "$p" && $(stat -c %u "$p") == 0 && $(stat -c %a "$p") == 600 && $(stat -c %s "$p") -le 4096 ]]
cat "$p"
"#, &[], 15)?;
        let stack: Option<Stack> =
            serde_json::from_str(&stack).map_err(|_| invalid("recibo de stack inválido"))?;
        if let Some(stack) = &stack {
            if !(100..=999999999).contains(&stack.vmid)
                || stack.digest.len() != 64
                || !stack.digest.bytes().all(|b| b.is_ascii_hexdigit())
                || stack.mac.len() != 17
            {
                return Err(invalid("propiedad de stack no confirmada"));
            }
            for (suffix, complete) in [(".json", false), (".complete.json", true)] {
                let name = format!("deploy-{}{suffix}", stack.digest);
                let receipt: Receipt =
                    serde_json::from_slice(&security::read_private(&name, 4096)?)
                        .map_err(|_| invalid("recibo Windows inválido"))?;
                if receipt.version != 1
                    || receipt.sid != owner.sid
                    || receipt.digest != stack.digest
                    || receipt.vmid != stack.vmid
                    || receipt.complete != complete
                {
                    return Err(invalid("stack ajeno o incompleto; no se adopta"));
                }
            }
            let endpoint = format!("/nodes/gnx-core-compute/lxc/{}/config", stack.vmid);
            let conf: serde_json::Value = serde_json::from_str(&super::deploy::command(
                identity,
                &["pvesh", "get", &endpoint, "--output-format", "json"],
                None,
                15,
            )?)
            .map_err(|_| invalid("configuración PVE inválida"))?;
            let expected = format!("GnX.Core:{}", stack.digest);
            if conf["description"].as_str().map(str::trim_end) != Some(expected.as_str())
                || conf["unprivileged"].as_u64() != Some(1)
                || !conf["net0"].as_str().is_some_and(|s| {
                    s.to_ascii_lowercase()
                        .contains(&stack.mac.to_ascii_lowercase())
                })
            {
                return Err(invalid("propiedad del LXC no confirmada"));
            }
        }
        let stack_running = if let Some(stack) = &stack {
            let id = stack.vmid.to_string();
            let status = super::deploy::command(identity, &["pct", "status", &id], None, 15)?;
            match status.trim() {
                "status: running" => true,
                "status: stopped" => false,
                _ => return Err(invalid("estado LXC no confirmado")),
            }
        } else {
            false
        };
        Ok(Self {
            identity,
            stack,
            stack_running,
        })
    }

    fn operation(
        &self,
        component: &Component,
        verb: &str,
        hash: Option<&str>,
    ) -> io::Result<String> {
        let baseline = Catalog::builtin()?;
        let core_role = matches!(
            component.name.as_str(),
            "mesh" | "dns" | "gateway" | "compute"
        );
        let seed = if core_role {
            let images: serde_json::Value =
                serde_json::from_slice(include_bytes!("../../../../config/images.json"))
                    .map_err(io::Error::other)?;
            images[&component.name]["id"]
                .as_str()
                .ok_or_else(|| invalid("semilla de imagen inválida"))?
                .to_owned()
        } else {
            baseline.get(&component.name).image.clone()
        };
        let mut args = vec![
            component.name.as_str(),
            verb,
            component.image.as_str(),
            seed.as_str(),
        ];
        if let Some(hash) = hash {
            args.push(hash);
        }
        let seconds = match verb {
            "stage" => 180,
            "apply" => 180,
            _ => 15,
        };
        if core_role {
            return core(self.identity, OPERATIONS, &args, seconds);
        }
        let stack = self
            .stack
            .as_ref()
            .ok_or_else(|| invalid("stack no registrado"))?;
        let id = stack.vmid.to_string();
        // Revalidate at each native step: staging other images may have taken
        // minutes, and a VMID must not silently refer to a replacement guest.
        let endpoint = format!("/nodes/gnx-core-compute/lxc/{id}/config");
        let conf: serde_json::Value = serde_json::from_str(&super::deploy::command(
            self.identity,
            &["pvesh", "get", &endpoint, "--output-format", "json"],
            None,
            15,
        )?)
        .map_err(|_| invalid("configuración PVE inválida"))?;
        let description = format!("GnX.Core:{}", stack.digest);
        if conf["description"].as_str().map(str::trim_end) != Some(description.as_str())
            || conf["unprivileged"].as_u64() != Some(1)
            || !conf["net0"].as_str().is_some_and(|s| {
                s.to_ascii_lowercase()
                    .contains(&stack.mac.to_ascii_lowercase())
            })
        {
            return Err(invalid("propiedad del LXC cambió; no se modifica"));
        }
        let status = super::deploy::command(self.identity, &["pct", "status", &id], None, 15)?;
        if status.trim() != "status: running" {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "LXC detenido; no se inicia",
            ));
        }
        let mut guest = vec!["pct", "exec", &id, "--", "/usr/bin/bash", "-s", "--"];
        guest.extend(args);
        super::deploy::command(self.identity, &guest, Some(OPERATIONS), seconds)
    }
}

fn save_catalog(catalog: &Catalog) -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let _directory = security::private_directory()?;
    let root = security::state_dir()?;
    // A leftover staging file is evidence of an interrupted operation, not trash.
    let temporary = root.join("updates-catalog.new");
    let target = root.join("updates-catalog.json");
    let encoded = serde_json::to_vec(catalog).map_err(io::Error::other)?;
    match security::read_private("updates-catalog.json", MAX_CATALOG as u64) {
        Ok(bytes) => {
            Catalog::decode(&bytes)?;
            if bytes == encoded {
                return Ok(());
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .share_mode(0)
        .open(&temporary)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    drop(file);
    let from: Vec<_> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn console_ready(identity: &Identity) -> io::Result<bool> {
    let text = core(identity, br#"set -euo pipefail
p=/var/lib/gnx-core/web/rustfs-console-v0.1.38/.gnx-console.json
if [[ ! -f "$p" || -L "$p" || $(stat -c %u "$p") != 0 ]]; then printf false; exit 0; fi
jq -e '.version=="v0.1.38" and .sha256=="1877193e802632d2f0e2b2995a06b26e01d7f077d624e5917826ed05b5f7d635"' "$p" >/dev/null && printf true || printf false
"#, &[], 10)?;
    Ok(text.trim() == "true")
}

fn console_archive() -> io::Result<PathBuf> {
    let path: PathBuf = std::env::current_exe()?
        .parent()
        .ok_or_else(|| invalid("ruta de instalación inválida"))?
        .join("payload/console/rustfs-console-v0.1.38.zip");
    let file = fs::File::open(&path)?;
    if file.metadata()?.len() > 16 * 1024 * 1024 {
        return Err(invalid("archivo consola fuera de límite"));
    }
    let mut hash = Sha256::new();
    let mut reader = io::BufReader::new(file);
    let mut buffer = [0; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != CONSOLE_HASH {
        return Err(invalid("hash de consola inválido"));
    }
    Ok(path)
}

fn apply_console(identity: &Identity, path: &std::path::Path) -> io::Result<()> {
    let path = super::bootstrap::linux_path(path)?;
    let ingress = format!(
        "{:x}",
        Sha256::digest(include_bytes!(
            "../../../../config/gateway/private-stack.caddy"
        ))
    );
    core(identity, CONSOLE_INSTALL, &[&path, "apply", &ingress], 150)?;
    Ok(())
}

fn run(
    identity: &Identity,
    owner: &Ownership,
    apply: bool,
    requested: Option<Catalog>,
) -> io::Result<(NodeState, String)> {
    let catalog = selected(requested)?;
    let context = Context::new(identity, owner)?;
    let baseline = Catalog::builtin()?;
    let mut lines = vec!["Catálogo aprobado por digest; no consulta latest. Estado de unidades no equivale a salud funcional.".to_owned()];
    let mut changes = Vec::new();
    let mut blocked = false;
    for name in NAMES {
        let component = catalog.get(name);
        if context.stack.is_none() && !matches!(name, "mesh" | "dns" | "gateway" | "compute") {
            lines.push(format!("{name}: no registrado; sin cambios"));
            continue;
        }
        if !context.stack_running && !matches!(name, "mesh" | "dns" | "gateway" | "compute") {
            lines.push(format!("{name}: LXC detenido; se respeta parada"));
            continue;
        }
        let snapshot: Snapshot = match context.operation(component, "inspect", None).and_then(|s| {
            serde_json::from_str(&s).map_err(|_| invalid("inventario de componente inválido"))
        }) {
            Ok(value) => value,
            Err(_) => {
                lines.push(format!("{name}: no observado; bloqueado"));
                blocked = true;
                continue;
            }
        };
        if snapshot.pending || !snapshot.owned {
            lines.push(format!(
                "{name}: pendiente desconocido o propiedad divergente; bloqueado"
            ));
            blocked = true;
            continue;
        }
        if snapshot.active == "inactive" {
            lines.push(format!("{name}: detenido; se respeta parada"));
            continue;
        }
        if snapshot.active != "active"
            || !snapshot
                .running
                .as_ref()
                .is_some_and(|r| r.status == "running")
        {
            lines.push(format!("{name}: unidad o contenedor no estable; bloqueado"));
            blocked = true;
            continue;
        }
        let running = snapshot
            .running
            .as_ref()
            .expect("checked")
            .image
            .trim_start_matches("sha256:");
        let matches = snapshot.configured == component.image
            || (!snapshot.local_id.is_empty()
                && running == snapshot.local_id.trim_start_matches("sha256:"))
            || (matches!(name, "mesh" | "dns" | "gateway" | "compute")
                && component.image == baseline.get(name).image
                && {
                    let images: serde_json::Value =
                        serde_json::from_slice(include_bytes!("../../../../config/images.json"))
                            .map_err(io::Error::other)?;
                    images[name]["id"]
                        .as_str()
                        .is_some_and(|id| running == id.trim_start_matches("sha256:"))
                });
        let state = if matches {
            "coincide"
        } else {
            "disponible en catálogo"
        };
        lines.push(format!(
            "{name}: ejecutado={} aprobado={} {state}",
            &running[..running.len().min(12)],
            &component.image[component.image.len() - 12..]
        ));
        if !matches {
            if name == "compute" && snapshot.running_guests != 0 {
                lines.push("compute: requiere ventana con guests detenidos explícitamente; no se detienen aquí".into());
                blocked = true;
            } else {
                changes.push((component.clone(), snapshot.source_hash));
            }
        }
    }
    let console_pending = context.stack_running && !console_ready(identity)?;
    if context.stack.is_some() {
        lines.push(format!(
            "console: v0.1.38 {}",
            if !context.stack_running {
                "no inspeccionada; LXC detenido"
            } else if console_pending {
                "pendiente"
            } else {
                "archivos preparados; verificar HTTPS/login por separado"
            }
        ));
    }
    if !apply || blocked {
        if apply && blocked {
            lines.push("No se aplicó nada: resolver bloqueos y volver a comprobar.".into());
        }
        return Ok((
            if blocked {
                NodeState::Degraded
            } else {
                NodeState::Active
            },
            lines.join("\n"),
        ));
    }
    // Download/verify every candidate before changing any unit configuration.
    for (component, _) in &changes {
        if context.operation(component, "stage", None).is_err() {
            lines.push(format!(
                "{}: descarga no confirmada; ninguna unidad modificada",
                component.name
            ));
            return Ok((NodeState::Degraded, lines.join("\n")));
        }
    }
    // Verify the complete offline frontend before restarting any image.
    let console_archive = if console_pending {
        Some(console_archive()?)
    } else {
        None
    };
    save_catalog(&catalog)?;
    for (component, source_hash) in &changes {
        if context
            .operation(component, "apply", Some(source_hash))
            .is_err()
        {
            lines.push(format!("{}: aplicación no confirmada; ejecución detenida, consultar estado nativo antes de reintentar", component.name));
            return Ok((NodeState::Degraded, lines.join("\n")));
        }
        lines.push(format!(
            "{}: imagen y unidad comprobadas; datos conservados",
            component.name
        ));
    }
    if let Some(archive) = console_archive {
        if apply_console(identity, &archive).is_err() {
            lines.push("console: aplicación no confirmada; componentes anteriores pueden estar aplicados, consultar estado nativo".into());
            return Ok((NodeState::Degraded, lines.join("\n")));
        }
        lines.push("console: archivos oficiales v0.1.38 aplicados; comprobar sesión humana".into());
    }
    lines.push("Aplicación explícita terminada. Sin aceptación funcional ni rollback de migraciones implícitos.".into());
    Ok((NodeState::Active, lines.join("\n")))
}

pub fn execute(apply: bool, requested: Option<Catalog>) -> io::Result<(NodeState, String)> {
    let owner = Ownership::read()?;
    let identity = Identity::open(&owner)?;
    let result = run(&identity, &owner, apply, requested);
    if let Ok((state, message)) = &result {
        report(
            message,
            *state == NodeState::Degraded,
            if apply { "aplicar" } else { "comprobar" },
        );
    }
    result
}

fn report(message: &str, failed: bool, phase: &str) {
    for line in message.lines() {
        let text = format!("Actualizaciones ({phase}): {line}");
        if failed {
            super::service::report_error(&io::Error::other(text));
        } else {
            super::service::report_information(&text);
        }
    }
}

pub fn check_on_start(identity: &Identity, owner: &Ownership) {
    match run(identity, owner, false, None) {
        Ok((state, message)) => report(
            &message,
            state == NodeState::Degraded,
            "arranque, solo local",
        ),
        Err(error) => super::service::report_error(&io::Error::other(format!(
            "Comprobación local de actualizaciones no disponible: {error}"
        ))),
    }
}
