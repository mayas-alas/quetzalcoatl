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
    let status = |state, exit_code| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(exit_code),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };
    h.set_service_status(status(ServiceState::StartPending, 0))?;
    // Provisioning is setup-owned and must not retry forever inside the
    // service. A failed bootstrap stops the service; setup records the gate
    // and decides whether a bounded resume is allowed.
    if super::runtime::bootstrap().is_err() {
        h.set_service_status(status(ServiceState::Stopped, 1))?;
        return Ok(());
    }
    if super::broker::validate_operator_sid().is_err() {
        h.set_service_status(status(ServiceState::Stopped, 1))?;
        return Ok(());
    }
    // The operator SID, SCM state, and pipe ACL are checked by serve_one before
    // advertising readiness. A failed request is terminal after a small,
    // bounded budget so a broken install cannot become an infinite retry loop.
    h.set_service_status(status(ServiceState::Running, 0))?;
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    let mut failures = 0;
    loop {
        if rx.try_recv().is_ok() {
            break;
        }
        match super::broker::serve_one() {
            Ok(()) => failures = 0,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                // The bounded connect wait is an idle poll, not a failed
                // request; keep the service available without blocking stop.
                failures = 0;
            }
            Err(_) => {
                failures += 1;
                if failures >= MAX_CONSECUTIVE_FAILURES {
                    h.set_service_status(status(ServiceState::Stopped, 1))?;
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    h.set_service_status(status(ServiceState::Stopped, 0))?;
    Ok(())
}
