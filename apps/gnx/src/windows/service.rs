use crate::protocol::SERVICE;
use std::{
    ffi::OsString,
    io,
    sync::{Arc, OnceLock},
    time::Duration,
};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
};

define_windows_service!(ffi_main, service_main);

pub fn dispatch() -> io::Result<()> {
    service_dispatcher::start(SERVICE, ffi_main).map_err(io::Error::other)
}

fn service_main(_: Vec<OsString>) {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let published = Arc::new(OnceLock::<ServiceStatusHandle>::new());
    let control_status = published.clone();
    let handle = match service_control_handler::register(SERVICE, move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            if let Some(handle) = control_status.get() {
                let _ = handle.set_service_status(status(ServiceState::StopPending, 0));
            }
            let _ = tx.send(true);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    }) {
        Ok(handle) => handle,
        Err(_) => return,
    };
    let _ = published.set(handle);
    if *rx.borrow() {
        let _ = handle.set_service_status(status(ServiceState::Stopped, 0));
        return;
    }
    if handle
        .set_service_status(status(ServiceState::Running, 0))
        .is_err()
    {
        return;
    }
    let result = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .and_then(|runtime| runtime.block_on(super::pipe::serve(rx)));
    // A failed listener is a failed service, never a misleading SCM Running state.
    let _ = handle.set_service_status(status(
        ServiceState::Stopped,
        if result.is_ok() { 0 } else { 1 },
    ));
}

fn status(state: ServiceState, code: u32) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: if state == ServiceState::Running {
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
        } else {
            ServiceControlAccept::empty()
        },
        exit_code: ServiceExitCode::Win32(code),
        checkpoint: if state == ServiceState::StopPending {
            1
        } else {
            0
        },
        wait_hint: if state == ServiceState::StopPending {
            Duration::from_secs(190)
        } else {
            Duration::ZERO
        },
        process_id: None,
    }
}
