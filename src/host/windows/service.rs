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

use crate::error::GnxError;
use crate::host::windows::account::{RuntimeCredential, SERVICE_DISPLAY_NAME, SERVICE_NAME};
use crate::state::{OperationalState, Stage, default_state_path};

const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

pub fn register(executable: &Path, credential: RuntimeCredential) -> Result<(), GnxError> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .map_err(service_error("service_manager"))?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: executable.to_path_buf(),
        launch_arguments: vec![OsString::from("__service")],
        dependencies: vec![],
        account_name: Some(credential.account_name),
        account_password: Some(credential.password),
    };
    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(service) => {
            service
                .change_config(&service_info)
                .map_err(service_error("service_update"))?;
            service
        }
        Err(_) => manager
            .create_service(
                &service_info,
                ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
            )
            .map_err(service_error("service_register"))?,
    };
    service
        .set_description("Owns the Quetzalcoatl Podman Machine and converges GNX runtime")
        .map_err(service_error("service_description"))?;
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
        .map_err(service_error("service_failure_actions"))?;
    service
        .set_failure_actions_on_non_crash_failures(true)
        .map_err(service_error("service_failure_policy"))?;
    Ok(())
}

pub fn remove() -> Result<(), GnxError> {
    stop()?;
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(service_error("service_manager"))?;
    let service = match manager.open_service(SERVICE_NAME, ServiceAccess::DELETE) {
        Ok(service) => service,
        Err(_) => return Ok(()),
    };
    service.delete().map_err(service_error("service_delete"))?;
    Ok(())
}

pub fn start() -> Result<(), GnxError> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(service_error("service_manager"))?;
    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::START | ServiceAccess::QUERY_STATUS,
        )
        .map_err(service_error("service_open"))?;
    let status = service
        .query_status()
        .map_err(service_error("service_status"))?;
    if status.current_state == ServiceState::Stopped {
        service
            .start::<&OsStr>(&[])
            .map_err(service_error("service_start"))?;
    }
    Ok(())
}

pub fn stop() -> Result<(), GnxError> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(service_error("service_manager"))?;
    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(service) => service,
        Err(_) => return Ok(()),
    };
    if service
        .query_status()
        .map_err(service_error("service_status"))?
        .current_state
        == ServiceState::Stopped
    {
        return Ok(());
    }
    service.stop().map_err(service_error("service_stop"))?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(90) {
        if service
            .query_status()
            .map_err(service_error("service_status"))?
            .current_state
            == ServiceState::Stopped
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(GnxError::new(
        "INSTALL_SERVICE_STOP_TIMEOUT",
        "install",
        "service_stop",
        "El servicio GNX no se detuvo en 90 segundos.",
        "Espere a que termine la convergencia y vuelva a intentar.",
        true,
        14,
    ))
}

pub fn run() -> Result<(), GnxError> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .map_err(service_error("service_dispatch"))
}

fn service_main(_arguments: Vec<OsString>) {
    crate::logs::event("info", "service", "start", "Servicio GNX iniciado");
    if let Err(error) = run_worker() {
        crate::logs::event(
            "error",
            "service",
            "worker",
            format!("Windows Service API: {error}"),
        );
    }
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

    loop {
        crate::logs::event(
            "info",
            "service",
            "converge",
            "Iniciando intento de convergencia",
        );
        let mut state = OperationalState {
            stage: Stage::Working,
            ..OperationalState::default()
        };
        let _ = state.save(&default_state_path());
        let convergence = converge(&mut state);
        match convergence {
            Ok(()) => {
                state.stage = Stage::Ready;
                state.machine = "ready".to_string();
                state.mesh = "ready".to_string();
                state.docktail = "deployed".to_string();
                state.proxmox = "ready".to_string();
                state.infra = "applied".to_string();
                state.last_error = None;
                crate::logs::event("info", "service", "converge", "Runtime convergido");
            }
            Err(error) => {
                state.stage = Stage::Failed;
                if state.machine != "ready" {
                    state.machine = "failed".to_string();
                } else {
                    match error.component {
                        "mesh" => state.mesh = "failed".to_string(),
                        "docktail" => state.docktail = "failed".to_string(),
                        "proxmox" => state.proxmox = "failed".to_string(),
                        "infra" | "opentofu" => state.infra = "failed".to_string(),
                        _ => {}
                    }
                }
                state.last_error = Some(error.code.to_string());
                crate::logs::error(&error);
            }
        }
        let _ = state.save(&default_state_path());
        match shutdown_rx.recv_timeout(Duration::from_secs(60)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) if state.stage == Stage::Failed => continue,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = shutdown_rx.recv();
                break;
            }
        }
    }
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;
    crate::logs::event("info", "service", "stop", "Servicio GNX detenido");
    Ok(())
}

fn converge(state: &mut OperationalState) -> Result<(), GnxError> {
    let config = crate::config::Config::load(&crate::config::default_config_path())?;
    let controller = config.validate()?;

    crate::runtime::machine::prepare()?;
    state.machine = "ready".to_string();
    let _ = state.save(&default_state_path());

    crate::runtime::headscale::verify_controller(&controller)?;
    state.mesh = "controller_reachable".to_string();
    let _ = state.save(&default_state_path());

    crate::runtime::machine::install_runtime(&controller, &config.mesh.bootstrap_addresses)?;

    crate::runtime::machine::converge_mesh(&controller)?;
    state.mesh = "ready".to_string();
    let _ = state.save(&default_state_path());

    crate::runtime::machine::converge_docktail()?;
    state.docktail = "ready".to_string();
    let _ = state.save(&default_state_path());

    crate::runtime::machine::converge_proxmox()?;
    state.proxmox = "ready".to_string();
    let _ = state.save(&default_state_path());

    crate::runtime::machine::converge_infra()?;
    state.infra = "applied".to_string();
    let _ = state.save(&default_state_path());

    crate::runtime::machine::finalize_mesh_auth()
}

fn service_error(operation: &'static str) -> impl FnOnce(windows_service::Error) -> GnxError {
    move |error| {
        GnxError::new(
            "INSTALL_SERVICE_FAILED",
            "install",
            operation,
            error.to_string(),
            "Ejecute gnx repair desde una consola elevada.",
            true,
            14,
        )
    }
}
