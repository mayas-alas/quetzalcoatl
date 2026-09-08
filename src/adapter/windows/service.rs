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
        if super::runtime::bootstrap().is_err() || super::broker::serve_one().is_err() {
            std::thread::sleep(Duration::from_secs(1))
        }
    });
    let _ = rx.recv();
    h.set_service_status(status(ServiceState::Stopped))?;
    Ok(())
}
