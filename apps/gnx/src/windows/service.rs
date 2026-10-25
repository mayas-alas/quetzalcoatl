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

pub(crate) fn report_error(error: &io::Error) {
    use windows_sys::Win32::System::EventLog::{
        DeregisterEventSource, EVENTLOG_ERROR_TYPE, RegisterEventSourceW, ReportEventW,
    };
    let source: Vec<u16> = SERVICE.encode_utf16().chain(Some(0)).collect();
    let message: Vec<u16> = error
        .to_string()
        .chars()
        .take(512)
        .flat_map(|c| c.to_string().encode_utf16().collect::<Vec<_>>())
        .chain(Some(0))
        .collect();
    unsafe {
        let handle = RegisterEventSourceW(std::ptr::null(), source.as_ptr());
        if !handle.is_null() {
            let strings = [message.as_ptr()];
            ReportEventW(
                handle,
                EVENTLOG_ERROR_TYPE,
                0,
                1,
                std::ptr::null_mut(),
                1,
                0,
                strings.as_ptr(),
                std::ptr::null(),
            );
            DeregisterEventSource(handle);
        }
    }
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
    if let Err(error) = &result {
        report_error(error);
    }
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
