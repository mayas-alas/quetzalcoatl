use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use windows_service::define_windows_service;
use windows_service::service::{
    ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
    ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use crate::{Error, Result};
use crate::windows::account::RuntimeCredential;

pub const SERVICE_NAME: &str = "GNXRuntime";
const SERVICE_DISPLAY_NAME: &str = "GNX Runtime";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

pub fn install(executable: &Path, operator_sid: &str) -> Result<()> {
    stop()?;
    let credential = crate::windows::account::ensure_runtime_account()?;
    crate::windows::account::grant_data_access(
        Path::new(crate::windows::runtime::DATA_ROOT),
        operator_sid,
    )?;
    register(executable, credential)
}

pub fn register(executable: &Path, credential: RuntimeCredential) -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .map_err(|_| Error::Operation("SERVICE_MANAGER"))?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: executable.to_path_buf(),
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: Some(OsString::from(credential.account_name)),
        account_password: Some(OsString::from(credential.password)),
    };
    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(service) => {
            service
                .change_config(&service_info)
                .map_err(|_| Error::Operation("SERVICE_UPDATE"))?;
            service
        }
        Err(_) => manager
            .create_service(
                &service_info,
                ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
            )
            .map_err(|_| Error::Operation("SERVICE_REGISTER"))?,
    };
    service
        .set_description("Owns the isolated GNX WSL runtime and brokers allowlisted GNX actions")
        .map_err(|_| Error::Operation("SERVICE_DESCRIPTION"))?;
    service
        .update_failure_actions(ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86_400)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![
                ServiceAction {
                    action_type: ServiceActionType::Restart,
                    delay: Duration::from_secs(10),
                },
                ServiceAction {
                    action_type: ServiceActionType::Restart,
                    delay: Duration::from_secs(30),
                },
                ServiceAction {
                    action_type: ServiceActionType::Restart,
                    delay: Duration::from_secs(60),
                },
            ]),
        })
        .map_err(|_| Error::Operation("SERVICE_FAILURE_ACTIONS"))?;
    service
        .set_failure_actions_on_non_crash_failures(true)
        .map_err(|_| Error::Operation("SERVICE_FAILURE_POLICY"))?;
    Ok(())
}

pub fn start() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|_| Error::Operation("SERVICE_MANAGER"))?;
    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::START | ServiceAccess::QUERY_STATUS,
        )
        .map_err(|_| Error::Operation("SERVICE_OPEN"))?;
    let status = service
        .query_status()
        .map_err(|_| Error::Operation("SERVICE_STATUS"))?;
    if status.current_state == ServiceState::Stopped {
        service
            .start::<&OsStr>(&[])
            .map_err(|_| Error::Operation("SERVICE_START"))?;
    }
    Ok(())
}

pub fn stop() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|_| Error::Operation("SERVICE_MANAGER"))?;
    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(service) => service,
        Err(_) => return Ok(()),
    };
    if service
        .query_status()
        .map_err(|_| Error::Operation("SERVICE_STATUS"))?
        .current_state
        == ServiceState::Stopped
    {
        return Ok(());
    }
    service
        .stop()
        .map_err(|_| Error::Operation("SERVICE_STOP"))?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        if service
            .query_status()
            .map_err(|_| Error::Operation("SERVICE_STATUS"))?
            .current_state
            == ServiceState::Stopped
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(Error::Operation("SERVICE_STOP_TIMEOUT"))
}

pub fn run() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .map_err(|_| Error::Operation("SERVICE_DISPATCH"))
}

fn service_main(_arguments: Vec<OsString>) {
    let _ = run_worker();
}

fn run_worker() -> windows_service::Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let event_handler = move |event| match event {
        ServiceControl::Stop => {
            let _ = shutdown_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    if let Err(error) = crate::windows::runtime::bootstrap() {
        crate::windows::runtime::write_failure(error.label());
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::ServiceSpecific(6),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;
        return Ok(());
    }
    crate::windows::runtime::clear_failure();

    thread::spawn(|| {
        if let Err(error) = crate::windows::broker::serve() {
            crate::windows::runtime::write_failure(error.label());
            std::process::exit(6);
        }
    });

    let _ = shutdown_rx.recv();
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    Ok(())
}
