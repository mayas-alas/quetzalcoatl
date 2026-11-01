use super::{
    installer_package, maintenance,
    recreation::{self, Inventory},
    release_source::{self, wide},
    security,
};
use crate::protocol::{Operation, SERVICE, invalid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, IsTerminal, Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use windows_sys::Win32::{
    Foundation::{FILETIME, LocalFree, WAIT_OBJECT_0},
    System::Threading::{
        GetCurrentProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SYNCHRONIZE, WaitForSingleObject,
    },
    UI::Shell::CommandLineToArgvW,
};

const WORKER: &str = "GnXCoreUpdate";
const RECEIPT: &str = "product-update.json";
const HELPER: &str = "product-update.exe";
const MSI: &str = "product-update.msi";
const BUNDLE: &str = "product-update-bundle.exe";

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Asset {
    name: String,
    hash: String,
    size: u64,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    schema: u16,
    phase: String,
    target: String,
    force: bool,
    install: bool,
    bundle: bool,
    binary: PathBuf,
    parent: u32,
    parent_created: u64,
    assets: Vec<Asset>,
    inventory: Option<Inventory>,
    #[serde(default)]
    unresolved_recreation: bool,
    message: String,
}
fn save(transaction: &Transaction) -> io::Result<()> {
    maintenance::write(
        RECEIPT,
        &serde_json::to_vec(transaction).map_err(io::Error::other)?,
    )
}
fn read() -> io::Result<Transaction> {
    let value: Transaction = serde_json::from_slice(&security::read_private(RECEIPT, 256 * 1024)?)
        .map_err(|_| invalid("transacción de producto inválida"))?;
    if value.schema != 1
        || value.parent == 0
        || value.parent_created == 0
        || value.assets.iter().any(|a| {
            ![HELPER, MSI, BUNDLE].contains(&a.name.as_str())
                || a.size == 0
                || a.hash.len() != 64
                || !a
                    .hash
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        })
        || value.assets.iter().filter(|a| a.name == HELPER).count() != 1
        || value.assets.iter().filter(|a| a.name == MSI).count() != usize::from(value.install)
        || value.assets.iter().filter(|a| a.name == BUNDLE).count() != usize::from(value.bundle)
        || (value.bundle && !value.install)
    {
        return Err(invalid("transacción no reconocida"));
    }
    release_source::version(&value.target)?;
    Ok(value)
}
fn phase(transaction: &mut Transaction, value: &str, message: &str) -> io::Result<()> {
    transaction.phase = value.into();
    transaction.message = message.into();
    save(transaction)?;
    super::service::report_information(&format!(
        "Actualización {}: {value}; {message}",
        transaction.target
    ));
    Ok(())
}
pub(super) fn receipt() -> io::Result<Option<String>> {
    match read() {
        Ok(t) => Ok(Some(format!(
            "Transacción de producto {}: {}; {}{}",
            t.target,
            t.phase,
            t.message,
            if t.unresolved_recreation && t.phase != "complete" {
                "; recreación anterior pendiente: cancelar esta solicitud no libera otras mutaciones"
            } else {
                ""
            }
        ))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
pub(super) fn permit(operation: Option<Operation>) -> io::Result<()> {
    if matches!(operation, Some(Operation::Status | Operation::UpdateCheck)) {
        return Ok(());
    }
    match read() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
        Ok(t)
            if ["complete", "starting_service", "applying_components"]
                .contains(&t.phase.as_str())
                || (!t.unresolved_recreation
                    && ["cancelled", "failed_before_native"].contains(&t.phase.as_str())) =>
        {
            Ok(())
        }
        _ => Err(invalid(
            "mantenimiento de producto pendiente; consultar update --check y SCM, no repetir mutaciones",
        )),
    }
}
struct RuntimeHold {
    cancel: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl RuntimeHold {
    fn new() -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let thread = std::thread::spawn(move || {
            let _ = super::identity::Ownership::read()
                .and_then(|owner| super::identity::Identity::open(&owner))
                .and_then(|identity| identity.keep_alive(&stop));
        });
        Self {
            cancel,
            thread: Some(thread),
        }
    }
}
impl Drop for RuntimeHold {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn hash(file: &mut File) -> io::Result<(u64, String)> {
    let mut digest = Sha256::new();
    let mut length = 0;
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        length += count as u64;
    }
    Ok((length, format!("{:x}", digest.finalize())))
}
fn verify(asset: &Asset) -> io::Result<()> {
    let (size, digest) = hash(&mut security::private_file(&asset.name)?)?;
    if size != asset.size || digest != asset.hash {
        return Err(invalid("archivo de transacción cambió; no se ejecuta"));
    }
    Ok(())
}
fn manager(access: ServiceManagerAccess) -> io::Result<ServiceManager> {
    ServiceManager::local_computer(None::<&str>, access).map_err(io::Error::other)
}
fn service_command(name: &str, argument: &str) -> io::Result<PathBuf> {
    let service = manager(ServiceManagerAccess::CONNECT)?
        .open_service(name, ServiceAccess::QUERY_CONFIG)
        .map_err(io::Error::other)?;
    let config = service.query_config().map_err(io::Error::other)?;
    if config.service_type != ServiceType::OWN_PROCESS
        || config.account_name.as_ref().is_some_and(|s| {
            !["localsystem", "nt authority\\system"]
                .contains(&s.to_string_lossy().to_ascii_lowercase().as_str())
        })
    {
        return Err(invalid(
            "identidad SCM no coincide con el servicio del producto",
        ));
    }
    let mut count = 0;
    let args = unsafe {
        CommandLineToArgvW(
            wide(&config.executable_path.to_string_lossy()).as_ptr(),
            &mut count,
        )
    };
    if args.is_null() {
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        if count != 2 {
            return Err(invalid("línea de servicio no reconocida"));
        }
        let strings: Vec<_> = unsafe { std::slice::from_raw_parts(args, count as usize) }
            .iter()
            .map(|&arg| {
                let mut length = 0;
                while unsafe { *arg.add(length) } != 0 {
                    length += 1;
                }
                String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(arg, length) })
            })
            .collect();
        if strings[1] != argument {
            return Err(invalid("entrada SCM no pertenece al servicio GnX"));
        }
        fs::canonicalize(&strings[0])
    })();
    unsafe {
        LocalFree(args.cast());
    }
    result
}
fn service_binary() -> io::Result<PathBuf> {
    service_command(SERVICE, "--service")
}
fn created(handle: windows_sys::Win32::Foundation::HANDLE) -> io::Result<u64> {
    let mut creation: FILETIME = unsafe { std::mem::zeroed() };
    let mut exit = creation;
    let mut kernel = creation;
    let mut user = creation;
    if unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}
fn worker_absent() -> io::Result<()> {
    match manager(ServiceManagerAccess::CONNECT)?.open_service(WORKER, ServiceAccess::QUERY_STATUS)
    {
        Err(windows_service::Error::Winapi(e)) if e.raw_os_error() == Some(1060) => Ok(()),
        _ => Err(invalid(
            "worker previo existente o no observable; comprobar SCM antes de repetir",
        )),
    }
}
fn clean_previous(force: bool, binary: &Path) -> io::Result<(Option<Inventory>, bool)> {
    worker_absent()?;
    match read() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok((None, false)),
        Err(e) => Err(e),
        Ok(previous) => {
            let unresolved = previous.phase == "interrupted"
                || (previous.unresolved_recreation && previous.phase != "complete");
            let new_destructive_request = force
                && previous.force
                && !previous.install
                && unresolved
                && ["interrupted", "cancelled", "failed_before_native"]
                    .contains(&previous.phase.as_str());
            if (unresolved
                || !["complete", "cancelled", "failed_before_native"]
                    .contains(&previous.phase.as_str()))
                && !new_destructive_request
            {
                return Err(invalid(
                    "transacción previa pendiente, desconocida o con reinicio requerido; no se repite automáticamente",
                ));
            }
            if new_destructive_request {
                let expected = previous
                    .inventory
                    .as_ref()
                    .ok_or_else(|| invalid("falta identidad de la retirada anterior"))?;
                let owner = super::identity::Ownership::read()?;
                if owner.sid != expected.sid || owner.distribution_id != expected.distribution {
                    return Err(invalid(
                        "la instancia anterior cambió; no se adopta otra para borrar",
                    ));
                }
                let service = manager(ServiceManagerAccess::CONNECT)?
                    .open_service(SERVICE, ServiceAccess::QUERY_STATUS)
                    .map_err(io::Error::other)?;
                security::require_idle_maintenance(
                    binary,
                    &super::bootstrap::private_root()?.join(HELPER),
                    service.query_status().map_err(io::Error::other)?.process_id,
                )?;
                println!(
                    "La retirada anterior falló. Esta solicitud requiere un nuevo inventario y confirmar el borrado completo; no se reanuda ni se revierte automáticamente."
                );
            }
            for asset in &previous.assets {
                verify(asset)?;
            }
            let root = super::bootstrap::private_root()?;
            for asset in &previous.assets {
                fs::remove_file(root.join(&asset.name))?;
            }
            // Keep the prior barrier on disk throughout staging. The atomic save
            // of the replacement carries it through cancellation/early failure.
            Ok((previous.inventory, unresolved))
        }
    }
}
fn stage_download(name: &str, asset: &release_source::Asset, expected: &str) -> io::Result<Asset> {
    let path = super::bootstrap::private_root()?.join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&path)?;
    let result = (|| {
        let (size, digest) =
            release_source::transfer(&asset.browser_download_url, &mut file, asset.size)?;
        file.sync_all()?;
        if size != asset.size || digest != expected {
            return Err(invalid("hash/tamaño de instalador no coincide"));
        }
        Ok(Asset {
            name: name.into(),
            size,
            hash: digest,
        })
    })();
    drop(file);
    if result.is_err() {
        fs::remove_file(path)?;
    }
    result
}
pub fn apply(force: bool, discover: bool) -> io::Result<bool> {
    security::effective_admin()?;
    let binary = service_binary()?;
    if fs::canonicalize(std::env::current_exe()?)? != binary
        || installer_package::installed()? != env!("CARGO_PKG_VERSION")
    {
        return Err(invalid(
            "ejecuta la CLI instalada que coincide con su MSI; un CLI técnico no es portable",
        ));
    }
    let mut transaction = Transaction {
        schema: 1,
        phase: "staging".into(),
        target: env!("CARGO_PKG_VERSION").into(),
        force,
        install: false,
        bundle: false,
        binary,
        parent: std::process::id(),
        parent_created: created(unsafe { GetCurrentProcess() })?,
        assets: Vec::new(),
        inventory: None,
        unresolved_recreation: false,
        message: String::new(),
    };
    // Discovery and integrity failures occur before touching SCM or the node.
    let source = if discover {
        let (repository, msi) = release_source::source()?;
        let release = release_source::latest(&repository)?;
        let hashes = release.hashes(&repository, &msi)?;
        if release.number()? < release_source::version(env!("CARGO_PKG_VERSION"))? {
            return Err(invalid("release anterior a la CLI; downgrade rechazado"));
        }
        transaction.install =
            release.number()? > release_source::version(env!("CARGO_PKG_VERSION"))?;
        transaction.target = release.tag_name.trim_start_matches('v').into();
        Some((repository, msi, release, hashes))
    } else {
        None
    };
    if !transaction.install && !force {
        let result = super::update(
            Operation::UpdateApply,
            Some(crate::updates::Catalog::builtin()?),
        )?;
        println!("{}", result.message);
        return Ok(result.ok);
    }
    if force && !io::stdin().is_terminal() {
        return Err(invalid(
            "la recreación requiere consola interactiva e inventario confirmado; sin confirmación implícita",
        ));
    }
    let lease = maintenance::lease()?;
    (transaction.inventory, transaction.unresolved_recreation) =
        clean_previous(force, &transaction.binary)?;
    for name in [HELPER, MSI, BUNDLE] {
        match fs::symlink_metadata(super::bootstrap::private_root()?.join(name)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            _ => {
                return Err(invalid(
                    "archivo reservado existente sin retirada verificada; no se adopta ni sobrescribe",
                ));
            }
        }
    }
    if let Some((repository, msi, release, hashes)) = source.filter(|_| transaction.install) {
        println!("Descargando y verificando MSI de {}...", release.tag_name);
        transaction.assets.push(stage_download(
            MSI,
            release.asset(&msi, &repository)?,
            &hashes[&msi],
        )?);
        installer_package::inspect(
            &super::bootstrap::private_root()?.join(MSI),
            &transaction.target,
        )?;
        transaction.bundle = installer_package::uses_bundle()?;
        if transaction.bundle {
            transaction.assets.push(stage_download(
                BUNDLE,
                release.asset("GnxCoreSetup.exe", &repository)?,
                &hashes["GnxCoreSetup.exe"],
            )?);
        }
    }
    let root = super::bootstrap::private_root()?;
    let mut input = File::open(&transaction.binary)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(root.join(HELPER))?;
    io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    drop(output);
    let (size, digest) = hash(&mut security::private_file(HELPER)?)?;
    transaction.assets.push(Asset {
        name: HELPER.into(),
        size,
        hash: digest,
    });
    save(&transaction)?;
    let service = manager(ServiceManagerAccess::CREATE_SERVICE)?
        .create_service(
            &ServiceInfo {
                name: WORKER.into(),
                display_name: "GnX explicit product update".into(),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::OnDemand,
                error_control: ServiceErrorControl::Normal,
                executable_path: root.join(HELPER),
                launch_arguments: vec!["--product-update-worker".into()],
                dependencies: vec![],
                account_name: None,
                account_password: None,
            },
            ServiceAccess::START | ServiceAccess::QUERY_STATUS,
        )
        .map_err(io::Error::other)?;
    drop(lease);
    service.start::<&str>(&[]).map_err(io::Error::other)?;
    let started = Instant::now();
    loop {
        let observed = read()?;
        match observed.phase.as_str() {
            "ready" => {
                transaction = observed;
                break;
            }
            "failed_before_native" | "interrupted" => {
                return Err(io::Error::other(observed.message));
            }
            _ => {}
        }
        if started.elapsed() > Duration::from_secs(180) {
            return Err(io::Error::other(
                "inventario no confirmado; worker no autorizado a instalar; comprobar transacción",
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    if force {
        let inventory = transaction
            .inventory
            .as_ref()
            .ok_or_else(|| invalid("falta inventario"))?;
        println!(
            "RECREACIÓN SIN RECUPERACIÓN AUTOMÁTICA: WSL {} (SID {}).",
            inventory.distribution, inventory.sid
        );
        println!(
            "Se pierden datos, CA, identidad mesh, credencial PVE, configuración y TODOS los guests del almacenamiento administrado:"
        );
        println!(
            "Se elimina el VHD de la WSL propia mediante --unregister; no se conserva copia ni se vuelve al nodo anterior."
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
        print!("Escribe RECREAR para autorizar la pérdida indicada: ");
        io::stdout().flush()?;
        let mut confirmation = String::new();
        io::stdin().read_line(&mut confirmation)?;
        if confirmation.trim_end_matches(['\r', '\n']) != "RECREAR" {
            let message = if transaction.unresolved_recreation {
                "nueva solicitud cancelada; la recreación anterior sigue sin resolver y bloquea otras mutaciones"
            } else {
                "no se autorizó ninguna instalación ni recreación"
            };
            phase(&mut transaction, "cancelled", message)?;
            return Ok(false);
        }
    }
    phase(
        &mut transaction,
        "authorized",
        "esperando salida de la CLI iniciadora; sin reinicio de Windows",
    )?;
    println!(
        "Transacción {} entregada al worker SCM. Esta CLI debe salir antes de sustituir archivos. Consulta gnx update --check y el Visor de eventos; este mensaje no confirma instalación ni recreación.",
        transaction.target
    );
    Ok(true)
}

define_windows_service!(ffi_worker, worker_main);
pub fn dispatch() -> io::Result<()> {
    service_dispatcher::start(WORKER, ffi_worker).map_err(io::Error::other)
}
fn worker_main(_: Vec<OsString>) {
    let Ok(handle) = service_control_handler::register(WORKER, |control| match control {
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    }) else {
        return;
    };
    let status = |state, code| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(code),
        checkpoint: 0,
        wait_hint: Duration::ZERO,
        process_id: None,
    };
    if handle
        .set_service_status(status(ServiceState::Running, 0))
        .is_err()
    {
        return;
    }
    let result = worker();
    if let Err(error) = &result {
        super::service::report_error(error);
    }
    // Only the service started from this protected transaction deletes itself.
    if service_command(WORKER, "--product-update-worker").is_ok_and(|path| {
        std::env::current_exe()
            .and_then(fs::canonicalize)
            .is_ok_and(|current| current == path)
    }) {
        if let Ok(manager) = manager(ServiceManagerAccess::CONNECT) {
            if let Ok(service) = manager.open_service(WORKER, ServiceAccess::DELETE) {
                let _ = service.delete();
            }
        }
    }
    let _ = handle.set_service_status(status(
        ServiceState::Stopped,
        if result.is_ok() { 0 } else { 1 },
    ));
}
fn stop_main() -> io::Result<()> {
    let service = manager(ServiceManagerAccess::CONNECT)?
        .open_service(SERVICE, ServiceAccess::STOP | ServiceAccess::QUERY_STATUS)
        .map_err(io::Error::other)?;
    match service
        .query_status()
        .map_err(io::Error::other)?
        .current_state
    {
        ServiceState::Stopped | ServiceState::StopPending => {}
        ServiceState::Running => {
            service.stop().map_err(io::Error::other)?;
        }
        _ => return Err(invalid("estado SCM no admite parada confirmada")),
    }
    let start = Instant::now();
    while service
        .query_status()
        .map_err(io::Error::other)?
        .current_state
        != ServiceState::Stopped
    {
        if start.elapsed() > Duration::from_secs(240) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "parada SCM desconocida; no se sustituye el producto",
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Ok(())
}
fn native(program: &Path, args: &[&str], seconds: u64) -> io::Result<u32> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return status
                .code()
                .map(|v| v as u32)
                .ok_or_else(|| io::Error::other("resultado nativo sin código"));
        }
        if start.elapsed() > Duration::from_secs(seconds) {
            // In particular, never kill or retry an MSI whose outcome is unknown.
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "operación nativa sigue sin resultado confirmado; no se repite ni se reinicia Windows",
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
fn worker() -> io::Result<()> {
    security::require_system()?;
    security::enable_identity_privileges()?;
    let mut transaction = read()?;
    let result = (|| {
        if transaction.phase != "staging" {
            return Err(invalid("worker no admite reintentar una fase existente"));
        }
        if fs::canonicalize(std::env::current_exe()?)?
            != fs::canonicalize(super::bootstrap::private_root()?.join(HELPER))?
        {
            return Err(invalid("ruta del worker no coincide"));
        }
        for asset in &transaction.assets {
            verify(asset)?;
        }
        let _guards: Vec<_> = transaction
            .assets
            .iter()
            .map(|a| security::private_file(&a.name))
            .collect::<io::Result<_>>()?;
        if service_binary()? != transaction.binary {
            return Err(invalid("servicio cambió antes de iniciar"));
        }
        let parent = security::Handle(unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                transaction.parent,
            )
        });
        if parent.0.is_null() || created(parent.0)? != transaction.parent_created {
            return Err(invalid("proceso iniciador no confirmado"));
        }
        let lease = maintenance::lease()?;
        let inventory = if transaction.force {
            recreation::force_inventory(transaction.inventory.as_ref())?
        } else {
            recreation::inventory()?
        };
        if !transaction.force && inventory.guests.iter().any(|g| g.status != "stopped") {
            return Err(invalid(
                "hay guests activos; detenerlos explícitamente en PVE antes de actualizar sin fuerza",
            ));
        }
        transaction.inventory = Some(inventory);
        phase(
            &mut transaction,
            "ready",
            "inventario observado; esperando autorización del iniciador",
        )?;
        let start = Instant::now();
        loop {
            let next = read()?;
            if next.parent != transaction.parent
                || next.parent_created != transaction.parent_created
                || next.target != transaction.target
                || next.inventory != transaction.inventory
                || next.force != transaction.force
                || next.install != transaction.install
                || next.bundle != transaction.bundle
                || next.binary != transaction.binary
                || next.assets != transaction.assets
                || next.unresolved_recreation != transaction.unresolved_recreation
            {
                return Err(invalid("handoff de transacción divergente"));
            }
            match next.phase.as_str() {
                "authorized" => {
                    transaction = next;
                    break;
                }
                "cancelled" => return Ok(()),
                "ready" => {}
                _ => return Err(invalid("fase de handoff no reconocida")),
            }
            if start.elapsed() > Duration::from_secs(300) {
                return Err(invalid(
                    "autorización agotó su plazo; no se instala ni recrea",
                ));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        if unsafe { WaitForSingleObject(parent.0, 30000) } != WAIT_OBJECT_0 {
            return Err(invalid("la CLI iniciadora no salió; no se reemplaza"));
        }
        if transaction.force {
            recreation::verify_target(
                transaction
                    .inventory
                    .as_ref()
                    .ok_or_else(|| invalid("falta destino confirmado"))?,
            )?;
        } else if let Some(field) = recreation::inventory()?.difference(
            transaction
                .inventory
                .as_ref()
                .ok_or_else(|| invalid("falta inventario confirmado"))?,
        ) {
            return Err(invalid(&format!(
                "inventario cambió después de confirmar: {field}"
            )));
        }
        if service_binary()? != transaction.binary {
            return Err(invalid("servicio divergente"));
        }
        if installer_package::installed()? != env!("CARGO_PKG_VERSION") {
            return Err(invalid(
                "la instalación cambió durante el handoff; no se mantiene una versión ajena",
            ));
        }
        for asset in &transaction.assets {
            verify(asset)?;
        }
        // Only conservative updates need a live runtime across SCM maintenance.
        // Force must not start a cold WSL merely to keep the old node alive.
        let mut runtime_hold = (!transaction.force).then(RuntimeHold::new);
        phase(
            &mut transaction,
            "stopping_service",
            "handoff confirmado; iniciando mantenimiento SCM",
        )?;
        stop_main()?;
        let root = super::bootstrap::private_root()?;
        if transaction.install {
            installer_package::inspect(&root.join(MSI), &transaction.target)?;
            phase(
                &mut transaction,
                "installing",
                "instalador nativo iniciado; resultado aún desconocido",
            )?;
            let code = if transaction.bundle {
                native(&root.join(BUNDLE), &["/quiet", "/norestart"], 3600)?
            } else {
                let system = PathBuf::from(
                    std::env::var_os("SystemRoot").ok_or_else(|| invalid("SystemRoot ausente"))?,
                )
                .join("System32/msiexec.exe");
                native(
                    &system,
                    &[
                        "/i",
                        &root.join(MSI).to_string_lossy(),
                        "/qn",
                        "/norestart",
                        "REBOOT=ReallySuppress",
                    ],
                    3600,
                )?
            };
            if code == 3010 || code == 1641 {
                phase(
                    &mut transaction,
                    "reboot_required",
                    "instalador requiere reinicio; no se reinicia ni se continúa con recreación/componentes",
                )?;
                return Ok(());
            }
            if code != 0 {
                return Err(io::Error::other(format!(
                    "instalador devolvió {code}; inspeccionar estado nativo, no repetir"
                )));
            }
            if installer_package::installed()? != transaction.target {
                return Err(invalid("versión MSI instalada no coincide"));
            }
            stop_main()?; // MSI may have started its service while the lease blocked recovery.
        }
        let binary = service_binary()?;
        if binary != transaction.binary {
            return Err(invalid(
                "MSI cambió la ubicación del servicio inesperadamente",
            ));
        }
        if native(
            &binary,
            &[
                "--verify-package",
                &binary
                    .parent()
                    .ok_or_else(|| invalid("ruta instalada inválida"))?
                    .join("payload")
                    .to_string_lossy(),
            ],
            300,
        )? != 0
        {
            return Err(invalid("payload instalado no verificado"));
        }
        let manifest_path = binary
            .parent()
            .ok_or_else(|| invalid("ruta instalada inválida"))?
            .join("payload/manifest.json");
        let manifest_file = File::open(manifest_path)?;
        if manifest_file.metadata()?.len() > 1024 * 1024 {
            return Err(invalid("manifiesto instalado fuera de límite"));
        }
        let manifest: serde_json::Value = serde_json::from_reader(manifest_file)
            .map_err(|_| invalid("manifiesto instalado inválido"))?;
        if manifest["product_version"].as_str() != Some(transaction.target.as_str()) {
            return Err(invalid("payload instalado no corresponde a la release"));
        }
        if transaction.force {
            // The package has not bootstrapped while our lease was held. Recheck
            // the owned target before invoking recreation in the installed binary.
            recreation::verify_target(
                transaction
                    .inventory
                    .as_ref()
                    .ok_or_else(|| invalid("falta destino confirmado"))?,
            )?;
            phase(
                &mut transaction,
                "recreating",
                "retirada exclusiva de la WSL verificada y su estado; sin rollback de datos",
            )?;
            let code = native(&binary, &["--product-recreate"], 1800)?;
            if code != 0 {
                return Err(invalid(&format!(
                    "recreación devolvió {code}; consultar el error específico en eventos, no repetir a ciegas"
                )));
            }
        }
        phase(
            &mut transaction,
            "starting_service",
            "instalador terminado; preparando recuperación del servicio",
        )?;
        drop(lease);
        let service = manager(ServiceManagerAccess::CONNECT)?
            .open_service(SERVICE, ServiceAccess::START | ServiceAccess::QUERY_STATUS)
            .map_err(io::Error::other)?;
        service.start::<&str>(&[]).map_err(io::Error::other)?;
        let start = Instant::now();
        loop {
            if service
                .query_status()
                .map_err(io::Error::other)?
                .current_state
                == ServiceState::Running
                && security::read_private("applied-version", 64)
                    .is_ok_and(|v| v == transaction.target.as_bytes())
            {
                break;
            }
            if start.elapsed() > Duration::from_secs(900) {
                return Err(invalid(
                    "preparación del servicio no confirmada; consultar eventos",
                ));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        drop(runtime_hold.take());
        if !transaction.force {
            phase(
                &mut transaction,
                "applying_components",
                "aplicando catálogo incorporado en el payload instalado",
            )?;
            let catalog = binary
                .parent()
                .ok_or_else(|| invalid("ruta de payload ausente"))?
                .join("payload/config/update-catalog.json");
            if native(
                &binary,
                &["update", "--apply", &catalog.to_string_lossy()],
                3600,
            )? != 0
            {
                return Err(invalid(
                    "producto instalado; componentes no confirmados, consultar recibos y eventos",
                ));
            }
        }
        let message = if transaction.force {
            "nodo recreado vacío; configura de nuevo password y enroll; login y salud no aceptados"
        } else {
            "instalación y aplicación nativas terminadas; salud/login y rollback de migraciones no acreditados"
        };
        phase(&mut transaction, "complete", message)
    })();
    if let Err(error) = &result {
        let before =
            ["staging", "ready", "authorized", "cancelled"].contains(&transaction.phase.as_str());
        let _ = phase(
            &mut transaction,
            if before {
                "failed_before_native"
            } else {
                "interrupted"
            },
            &error.to_string(),
        );
    }
    result
}
pub fn recreate_installed() -> io::Result<()> {
    security::require_system()?;
    security::enable_identity_privileges()?;
    let transaction = read()?;
    if !transaction.force
        || transaction.phase != "recreating"
        || service_binary()? != fs::canonicalize(std::env::current_exe()?)?
    {
        return Err(invalid("recreación interna no autorizada por transacción"));
    }
    let service = manager(ServiceManagerAccess::CONNECT)?
        .open_service(SERVICE, ServiceAccess::QUERY_STATUS)
        .map_err(io::Error::other)?;
    if service
        .query_status()
        .map_err(io::Error::other)?
        .current_state
        != ServiceState::Stopped
    {
        return Err(invalid("servicio principal no está detenido"));
    }
    recreation::execute(
        transaction
            .inventory
            .as_ref()
            .ok_or_else(|| invalid("falta inventario confirmado"))?,
    )
}
