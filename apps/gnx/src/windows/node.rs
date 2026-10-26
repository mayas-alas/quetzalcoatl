use super::{
    identity::{Identity, Ownership},
    security,
};
use crate::protocol::{DISTRO, NodeState, Operation, Request, Response, TARGET};
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Semaphore;

pub struct Controller {
    mutation: Arc<Semaphore>,
    session: Mutex<Option<WslSession>>,
}

struct WslSession {
    cancel: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Drop for WslSession {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Controller {
    pub fn new() -> Self {
        Self {
            mutation: Arc::new(Semaphore::new(1)),
            session: Mutex::new(None),
        }
    }

    fn hold_session(&self, running: bool) -> io::Result<()> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| io::Error::other("sesión no disponible"))?;
        if !running {
            session.take();
            return Ok(());
        }
        if session
            .as_ref()
            .and_then(|session| session.worker.as_ref())
            .is_some_and(|worker| !worker.is_finished())
        {
            return Ok(());
        }
        session.take();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker = std::thread::spawn(move || {
            let result = Ownership::read()
                .and_then(|owner| Identity::open(&owner))
                .and_then(|identity| identity.keep_alive(&worker_cancel));
            if let Err(error) = result {
                super::service::report_error(&error);
            }
        });
        *session = Some(WslSession {
            cancel,
            worker: Some(worker),
        });
        Ok(())
    }

    pub async fn execute(&self, request: &Request) -> Response {
        // Profile loading/unloading is shared per Windows identity. Serialize
        // its use too; a concurrent status returns immediately, never races it.
        let _permit = match self.mutation.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                return Response::new(
                    request,
                    false,
                    NodeState::Busy,
                    "operación en curso; estado nativo aún no observado",
                );
            }
        };
        let op = request.operation;
        let secret = request
            .secret
            .as_ref()
            .map(|s| zeroize::Zeroizing::new(s.clone()));
        match tokio::task::spawn_blocking(move || {
            execute_secret(op, secret.as_deref().map(String::as_str))
        })
        .await
        {
            Ok(Ok((state, message))) => {
                if matches!(op, Operation::Start | Operation::Stop) {
                    if let Err(error) = self.hold_session(op == Operation::Start) {
                        super::service::report_error(&error);
                        return Response::new(
                            request,
                            false,
                            NodeState::Unknown,
                            "sesión WSL no confirmada",
                        );
                    }
                }
                Response::new(request, true, state, &message)
            }
            Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => Response::new(
                request,
                false,
                NodeState::NotPrepared,
                "núcleo no preparado; falta instalación nueva verificada, no se adopta el legacy",
            ),
            Ok(Err(error)) if error.kind() == io::ErrorKind::TimedOut => Response::new(
                request,
                false,
                NodeState::Unknown,
                "resultado desconocido por timeout; consulta status sin repetir la mutación",
            ),
            Ok(Err(error)) if error.kind() == io::ErrorKind::NotConnected => Response::new(
                request,
                false,
                NodeState::Stopped,
                "nodo detenido; usa start antes de configurar",
            ),
            _ => Response::new(
                request,
                false,
                NodeState::Unknown,
                "operación no confirmada; comprueba propiedad, identidad y runtime dedicados",
            ),
        }
    }

    pub async fn recover(&self) {
        let Ok(_permit) = self.mutation.clone().acquire_owned().await else {
            return;
        };
        if let Ok(Err(error)) = tokio::task::spawn_blocking(recover).await {
            super::service::report_error(&error);
        } else if security::read_private("intent", 16).is_ok_and(|value| value == b"running") {
            if let Err(error) = self.hold_session(true) {
                super::service::report_error(&error);
            }
        }
    }
}

fn execute(op: Operation) -> io::Result<(NodeState, String)> {
    execute_secret(op, None)
}

fn execute_secret(op: Operation, secret: Option<&str>) -> io::Result<(NodeState, String)> {
    let ownership = Ownership::read()?;
    let identity = Identity::open(&ownership)?;
    let running = identity.run(&["--list", "--running", "--quiet"], Duration::from_secs(10))?;
    if running.code != 0 {
        return Err(io::Error::other("no se pudo observar WSL"));
    }
    let running = running.text().lines().any(|line| line.trim() == DISTRO);
    if op == Operation::Status && !running {
        return Ok((NodeState::Stopped, "nodo detenido; WSL no se inició".into()));
    }
    if matches!(op, Operation::Enroll | Operation::Password) {
        if !running {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "nodo detenido"));
        }
        let secret = secret.ok_or_else(|| crate::protocol::invalid("falta entrada protegida"))?;
        let mut input = zeroize::Zeroizing::new(Vec::with_capacity(secret.len() + 1));
        input.extend_from_slice(secret.as_bytes());
        input.push(b'\n');
        let kind = if op == Operation::Enroll {
            "mesh"
        } else {
            "compute"
        };
        let result = identity.run_secret(
            &[
                "--distribution",
                DISTRO,
                "--exec",
                "/usr/bin/bash",
                "/etc/gnx-core/configure.run",
                kind,
            ],
            &input,
            Duration::from_secs(150),
        )?;
        if result.code != 0 {
            return Err(io::Error::other("configuración nativa no confirmada"));
        }
        return Ok((NodeState::Active, if op == Operation::Enroll {
            "Tailscale configurado; resolver de malla preparado, comprueba la etapa con status"
        } else {
            "contraseña configurada; Proxmox iniciado mediante su Quadlet, HTTPS aún no comprobado"
        }.into()));
    }
    if op == Operation::Stop {
        write_intent(false)?;
        if !running {
            return Ok((
                NodeState::Stopped,
                "nodo detenido; datos conservados".into(),
            ));
        }
    }
    if op == Operation::Start {
        write_intent(true)?;
    }
    if op != Operation::Status {
        let verb = if op == Operation::Start {
            "start"
        } else {
            "stop"
        };
        let result = identity.run(
            &[
                "--distribution",
                DISTRO,
                "--exec",
                "/usr/bin/systemctl",
                verb,
                TARGET,
            ],
            Duration::from_secs(150),
        )?;
        if result.code != 0 {
            return Err(io::Error::other("systemd no confirmó la operación"));
        }
        if op == Operation::Start {
            super::access::publish(&identity)?;
        }
    }
    let result = identity.run(
        &[
            "--distribution",
            DISTRO,
            "--exec",
            "/usr/bin/systemctl",
            "show",
            "--property=ActiveState",
            "--value",
            TARGET,
        ],
        Duration::from_secs(10),
    )?;
    if result.code != 0 {
        return Err(io::Error::other("systemd no confirmó el estado"));
    }
    match result.text().trim() {
        "inactive" if op != Operation::Start => Ok((
            NodeState::Stopped,
            "target detenido; datos conservados, otras WSL intactas".into(),
        )),
        "active" if op != Operation::Stop => {
            let message = "target activo; HTTPS, mesh y compute aún no comprobados";
            if op != Operation::Status {
                return Ok((NodeState::Active, message.into()));
            }
            let phase = identity.run(
                &[
                    "--distribution",
                    DISTRO,
                    "--exec",
                    "/usr/bin/cat",
                    "/var/lib/gnx-core/web/status.txt",
                ],
                Duration::from_secs(10),
            )?;
            let message = if phase.code == 0 {
                format!("{message}\nÚltima fase publicada (no es salud en vivo): {}", phase.text().trim())
            } else {
                format!("{message}; fase no disponible")
            };
            Ok((NodeState::Active, message))
        }
        "activating" => Ok((
            NodeState::Starting,
            "systemd iniciando el target; servicios aún no comprobados".into(),
        )),
        _ => Err(io::Error::other("el estado no confirma la petición")),
    }
}

fn write_intent(running: bool) -> io::Result<()> {
    use std::{
        io::Write,
        os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
    };
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    // Validate the protected directory before creating the inherited-ACL file.
    let _directory = security::private_directory()?;
    let root = security::state_dir()?;
    let temporary = root.join("intent.new");
    let destination = root.join("intent");
    // Recover only this fixed, ACL-verified staging file. Never remove unknown
    // files or reset identity/configuration after a failed operation.
    match security::read_private("intent.new", 16) {
        Ok(bytes) if bytes.is_empty() || bytes == b"running" || bytes == b"stopped" => {
            std::fs::remove_file(&temporary)?
        }
        Ok(_) => {
            return Err(crate::protocol::invalid(
                "staging de intención no reconocido",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&temporary)?;
    file.write_all(if running { b"running" } else { b"stopped" })?;
    file.sync_all()?;
    drop(file);
    let from: Vec<_> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
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

pub fn recover() -> io::Result<()> {
    super::bootstrap::prepare()?;
    let intent = match security::read_private("intent", 16) {
        Ok(intent) => intent,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    match intent.as_slice() {
        b"running" => execute(Operation::Start).map(|_| ()),
        b"stopped" => execute(Operation::Stop).map(|_| ()),
        _ => Err(crate::protocol::invalid(
            "intención inválida; no se inicia el nodo",
        )),
    }
}
