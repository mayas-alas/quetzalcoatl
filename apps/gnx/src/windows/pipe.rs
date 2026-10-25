use crate::protocol::{self, MAX_FRAME, Operation, PIPE, Request, Response, SERVICE, VERSION};
use std::{
    collections::VecDeque,
    io,
    os::windows::io::AsRawHandle,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions},
    sync::{Semaphore, watch},
    task::JoinSet,
    time::timeout,
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        RevertToSelf, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::SECURITY_IDENTIFICATION,
    System::Pipes::{GetNamedPipeServerProcessId, ImpersonateNamedPipeClient},
};
use zeroize::Zeroizing;

const IO_TIMEOUT: Duration = Duration::from_secs(5);
const OP_TIMEOUT: Duration = Duration::from_secs(190);
const PIPE_DACL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)";

fn listener(first: bool) -> io::Result<NamedPipeServer> {
    let text: Vec<_> = PIPE_DACL.encode_utf16().chain(Some(0)).collect();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: nul-terminated SDDL and valid output pointer; descriptor is freed after CreateNamedPipe copies it.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let result = unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .max_instances(16)
            .in_buffer_size(MAX_FRAME as u32)
            .out_buffer_size(MAX_FRAME as u32)
            .create_with_security_attributes_raw(
                PIPE,
                (&mut attrs as *mut SECURITY_ATTRIBUTES).cast(),
            )
    };
    unsafe { LocalFree(descriptor) };
    result
}

fn authorize(server: &NamedPipeServer) -> io::Result<()> {
    // No await is permitted while impersonating: the token is thread-local.
    if unsafe { ImpersonateNamedPipeClient(server.as_raw_handle().cast()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = super::security::effective_admin();
    if unsafe { RevertToSelf() } == 0 {
        // Continuing under an unknown effective token is unsafe.
        std::process::abort();
    }
    result
}

pub async fn read_frame(reader: &mut (impl AsyncRead + Unpin)) -> io::Result<Vec<u8>> {
    let length = reader.read_u32_le().await? as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(protocol::invalid("frame fuera de límite"));
    }
    let mut data = vec![0; length];
    reader.read_exact(&mut data).await?;
    Ok(data)
}

pub async fn write_frame(writer: &mut (impl AsyncWrite + Unpin), data: &[u8]) -> io::Result<()> {
    if data.is_empty() || data.len() > MAX_FRAME {
        return Err(protocol::invalid("frame fuera de límite"));
    }
    writer.write_u32_le(data.len() as u32).await?;
    writer.write_all(data).await
}

#[derive(Default)]
struct ReplayWindow(VecDeque<[u8; 16]>);
impl ReplayWindow {
    fn accept(&mut self, id: [u8; 16]) -> bool {
        if self.0.contains(&id) {
            return false;
        }
        if self.0.len() == 256 {
            self.0.pop_front();
        }
        self.0.push_back(id);
        true
    }
}

pub async fn serve(mut shutdown: watch::Receiver<bool>) -> io::Result<()> {
    super::security::require_system()?;
    super::security::enable_identity_privileges()?;
    let mut incoming = listener(true)?;
    let clients = Arc::new(Semaphore::new(8));
    let seen = Arc::new(Mutex::new(ReplayWindow::default()));
    let controller = Arc::new(super::node::Controller::new());
    let mut tasks = JoinSet::new();
    let recovery = controller.clone();
    tasks.spawn(async move {
        recovery.recover().await;
    });
    loop {
        tokio::select! {
            _ = shutdown.changed() => break,
            connection = incoming.connect() => {
                connection?;
                // Keep one instance alive across handover: no name-squatting gap.
                let connected = std::mem::replace(&mut incoming, listener(false)?);
                let Ok(permit) = clients.clone().try_acquire_owned() else { continue; };
                let seen = seen.clone();
                let controller = controller.clone();
                tasks.spawn(async move {
                    let _permit = permit;
                    let _ = connection_task(connected, seen, controller).await;
                });
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
    }
    // Drain active, bounded mutations. Dropping a CLI never cancels a native job.
    drop(incoming);
    while tasks.join_next().await.is_some() {}
    Ok(())
}

async fn connection_task(
    mut pipe: NamedPipeServer,
    seen: Arc<Mutex<ReplayWindow>>,
    controller: Arc<super::node::Controller>,
) -> io::Result<()> {
    let bytes = Zeroizing::new(timeout(IO_TIMEOUT, read_frame(&mut pipe)).await??);
    authorize(&pipe)?;
    let request = Request::decode(&bytes)?;
    if !seen
        .lock()
        .map_err(|_| io::Error::other("control no disponible"))?
        .accept(request.id)
    {
        return Err(protocol::invalid("petición repetida"));
    }
    let response = controller.execute(&request).await;
    let bytes = serde_json::to_vec(&response).map_err(io::Error::other)?;
    timeout(IO_TIMEOUT, write_frame(&mut pipe, &bytes)).await??;
    // Bounded application ACK replaces unbounded FlushFileBuffers.
    let _ = timeout(IO_TIMEOUT, pipe.read_u8()).await;
    Ok(())
}

pub async fn client(operation: Operation) -> io::Result<Response> {
    super::security::effective_admin()?;
    let manager = windows_service::service_manager::ServiceManager::local_computer(
        None::<&str>,
        windows_service::service_manager::ServiceManagerAccess::CONNECT,
    )
    .map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "GnXCore no instalado o no accesible",
        )
    })?;
    let service = manager
        .open_service(
            SERVICE,
            windows_service::service::ServiceAccess::QUERY_STATUS,
        )
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "GnXCore no instalado; el servicio legacy no se utiliza",
            )
        })?;
    let status = service.query_status().map_err(io::Error::other)?;
    if status.current_state != windows_service::service::ServiceState::Running {
        return Err(io::Error::other(
            "servicio GnXCore detenido; no se inició ni modificó nada",
        ));
    }
    super::security::process_is_system(
        status
            .process_id
            .ok_or_else(|| io::Error::other("PID del servicio no disponible"))?,
    )?;
    // Human input happens before opening the short-lived connection. The pipe
    // server is authenticated again before any bytes or credentials are sent.
    let secret = match operation {
        Operation::Enroll => Some(prompt_secret("Key Tailscale (sin eco): ")?),
        Operation::Password => {
            let value = Zeroizing::new(prompt_secret("Contraseña Proxmox (mínimo 12 bytes): ")?);
            let confirmation = Zeroizing::new(prompt_secret("Confirmar contraseña: ")?);
            if *value != *confirmation {
                return Err(protocol::invalid("las contraseñas no coinciden"));
            }
            Some(value.to_string())
        }
        _ => None,
    };
    let mut id = [0u8; 16];
    super::security::random(&mut id)?;
    let request = Request {
        version: VERSION,
        id,
        operation,
        secret,
    };
    let bytes = Zeroizing::new(serde_json::to_vec(&request).map_err(io::Error::other)?);
    let _ = Request::decode(&bytes)?;
    let mut pipe = ClientOptions::new()
        .security_qos_flags(SECURITY_IDENTIFICATION)
        .open(PIPE)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "pipe no disponible; usa una consola elevada y comprueba GnXCore",
            )
        })?;
    let mut pid = 0;
    if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle().cast(), &mut pid) } == 0
        || Some(pid) != service.query_status().map_err(io::Error::other)?.process_id
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "servidor de pipe no autenticado",
        ));
    }
    super::security::process_is_system(pid)?;
    timeout(IO_TIMEOUT, write_frame(&mut pipe, &bytes)).await??;
    let bytes = timeout(OP_TIMEOUT, read_frame(&mut pipe))
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "resultado desconocido; consulta status, no repitas una mutación a ciegas",
            )
        })??;
    let response = Response::decode(&bytes, id)?;
    timeout(IO_TIMEOUT, pipe.write_u8(1)).await??;
    Ok(response)
}

fn prompt_secret(prompt: &str) -> io::Result<String> {
    use std::io::{BufRead, IsTerminal};
    if std::io::stdin().is_terminal() {
        return rpassword::prompt_password(prompt);
    }
    // Redirected stdin is already a private inherited pipe, not a console
    // whose echo mode needs changing. Never read secrets from arguments.
    let mut value = String::new();
    std::io::stdin().lock().read_line(&mut value)?;
    Ok(value.trim_end_matches(['\r', '\n']).to_owned())
}
