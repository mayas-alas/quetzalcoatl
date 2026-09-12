use std::{ffi::OsString, sync::mpsc, time::Duration};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};
define_windows_service!(entry, service_main);
pub fn run() -> windows_service::Result<()> {
    service_dispatcher::start("GNXRuntime", entry)
}
fn service_main(_: Vec<OsString>) {
    let _ = serve();
}
fn serve() -> windows_service::Result<()> {
    let (tx, rx) = mpsc::channel();
    let h = service_control_handler::register("GNXRuntime", move |c| match c {
        ServiceControl::Stop => {
            let _ = tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    })?;
    let status = |state| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };
    h.set_service_status(status(ServiceState::Running))?;
    std::thread::spawn(|| loop {
        let bootstrap = super::runtime::bootstrap();
        let broker = if bootstrap.is_ok() {
            super::broker::serve_one()
        } else {
            Err(std::io::Error::other("bootstrap failed"))
        };
        let broker_ready = match broker.as_ref() {
            Ok(()) => true,
            Err(error) => error.kind() == std::io::ErrorKind::TimedOut,
        };
        let phase = serde_json::json!({
            "schema": 1,
            "bootstrap_code": bootstrap.as_ref().err(),
            "broker_code": broker.as_ref().err().filter(|error| error.kind() != std::io::ErrorKind::TimedOut).map(|error| error.to_string()),
            "ready": bootstrap.is_ok() && broker_ready
        });
        let path = std::path::Path::new(super::account::PRIVATE_ROOT).join("bootstrap-status.json");
        let bytes = phase.to_string();
        if std::fs::read(&path).ok().as_deref() != Some(bytes.as_bytes()) {
            let _ = crate::adapter::filesystem::atomic_write(&path, bytes.as_bytes(), 0o600);
        }
        if bootstrap.is_err() || !broker_ready {
            std::thread::sleep(Duration::from_secs(1))
        }
    });
    let mut session: Option<std::process::Child> = None;
    loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let root = std::path::Path::new(super::account::PRIVATE_ROOT);
        if root.join("bundle.tar").exists() || !root.join("wsl/ext4.vhdx").exists() {
            continue;
        }
        if session
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))))
        {
            session = None;
        }
        if session.is_none() {
            session = super::runtime::keep_alive().ok();
        }
    }
    if let Some(mut child) = session {
        let _ = child.kill();
        let _ = child.wait();
    }
    let root = std::path::Path::new(super::account::PRIVATE_ROOT);
    // The service account can read but cannot write the administrator-owned
    // install root. A compromised runtime therefore cannot arm its own removal.
    let request = std::path::Path::new("C:\\Program Files\\GNX\\uninstall.request");
    let uninstall_requested = std::fs::read_to_string(request)
        .ok()
        .is_some_and(|value| value.trim_end_matches(&['\r', '\n'][..]) == "GNX-UNINSTALL-1");
    if uninstall_requested {
        let (state, code): (&str, String) = match super::runtime::unregister_for_uninstall() {
            Ok(()) => ("READY", "WSL_UNREGISTERED".into()),
            Err(code) => ("FAILED", code),
        };
        let result =
            serde_json::json!({"schema":1,"operation":"uninstall","state":state,"code":code});
        let _ = crate::adapter::filesystem::atomic_write(
            &root.join("uninstall-result.json"),
            result.to_string().as_bytes(),
            0o600,
        );
        let _ = std::fs::remove_file(request);
    }
    h.set_service_status(status(ServiceState::Stopped))?;
    Ok(())
}
