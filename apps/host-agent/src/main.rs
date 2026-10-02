mod ca;
mod cleanup;
mod control_pipe;
mod dedicated_account;
mod github;
mod host_access;
mod linux_node;
mod provisioning;
mod secret_store;
mod state;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let args = std::env::args_os().skip(1).collect::<Vec<_>>();
        if args.as_slice() == [std::ffi::OsStr::new("--cleanup")] {
            return cleanup::run();
        }
        if args.as_slice() == [std::ffi::OsStr::new("--diagnose-compute")] {
            return linux_node::diagnose_compute().map_err(|error| error.into());
        }
        if !args.is_empty() {
            return Err("invalid host-agent command".into());
        }
        // The installed binary is a real SCM service. GNX_CONSOLE=1 is an explicit
        // developer/diagnostic escape hatch for running the same pipe server in a console.
        if std::env::var_os("GNX_CONSOLE").is_some() {
            return control_pipe::run();
        }
        service::run().map_err(|e| e.into())
    }
    #[cfg(not(windows))]
    control_pipe::run()
}

#[cfg(windows)]
mod service {
    use super::control_pipe;
    use gnx_control_protocol::NodeState;
    use std::ffi::OsString;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service, service_dispatcher, Result};

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> Result<()> {
        service_dispatcher::start("GnXHostAgent", ffi_service_main)
    }

    fn service_main(_arguments: Vec<OsString>) {
        let (stop_tx, stop_rx) = mpsc::channel();
        let event_handler = move |event| match event {
            service::ServiceControl::Stop => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            service::ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let status_handle = match service_control_handler::register("GnXHostAgent", event_handler) {
            Ok(handle) => handle,
            Err(_) => return,
        };
        let _ = status_handle.set_service_status(service::ServiceStatus {
            service_type: service::ServiceType::OWN_PROCESS,
            current_state: service::ServiceState::Running,
            controls_accepted: service::ServiceControlAccept::STOP,
            exit_code: service::ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        });
        // A service process is not healthy without its control pipe. Supervise
        // the pipe loop so a transient startup collision during MSI removal /
        // reinstall, or any unexpected server error, cannot leave SCM showing
        // Running while every CLI request reports the service unavailable.
        let stop_service = Arc::new(AtomicBool::new(false));
        let server_flag = Arc::clone(&stop_service);
        let server = std::thread::spawn(move || {
            while !server_flag.load(Ordering::Acquire) {
                let _ = control_pipe::run();
                if !server_flag.load(Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        });
        // WSL intentionally shuts a distro down when its last Windows-side
        // client exits, even while systemd units are active. Keep one harmless
        // client attached so the private DNS and HTTPS endpoints stay online.
        let keepalive_flag = Arc::clone(&stop_service);
        let keepalive = std::thread::spawn(move || {
            let args = vec![
                "--distribution".into(),
                crate::linux_node::DISTRO_NAME.into(),
                "--".into(),
                "/bin/sleep".into(),
                "5".into(),
            ];
            while !keepalive_flag.load(Ordering::Relaxed) {
                // A fresh node has no runtime to keep alive. Starting WSL as
                // soon as the account credential appears races Provision's
                // required .wslconfig copy and first distro import.
                let enrolled = crate::state::load()
                    // A failed readiness check must not repeatedly idle an
                    // enrolled distro during recovery. An explicit runtime
                    // stop still releases it, even though its address persists.
                    .map(|progress| {
                        progress.node_private_ip.is_some()
                            && progress.state != NodeState::RuntimeStopped
                            && !crate::linux_node::runtime_stop_requested()
                    })
                    .unwrap_or(false);
                if !enrolled {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
                if crate::dedicated_account::run_wsl_as_windows_identity(&args).is_err() {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        });
        // Best-effort host browser access reconciliation is intentionally
        // limited to an already-ready persisted node. Provisioning remains the
        // only path that creates readiness, and startup never exports secrets or
        // restarts WSL to force a network address.
        if let Ok(progress) = crate::state::load() {
            if progress.state == NodeState::NodeReady {
                if let Some(certificate) = progress.private_ca_certificate.as_deref() {
                    let _ = crate::host_access::reconcile(std::path::Path::new(certificate));
                }
            }
        }
        // Reuse the canonical verifier instead of inventing browser-side
        // health checks. This is a thread of the existing host service, not a
        // new network service or a browser control channel.
        let status_flag = Arc::clone(&stop_service);
        let status_monitor = std::thread::spawn(move || {
            while !status_flag.load(Ordering::Relaxed) {
                let _ = crate::state::refresh_app_status();
                for _ in 0..15 {
                    if status_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        });
        let _ = stop_rx.recv();
        stop_service.store(true, Ordering::Release);
        let _ = status_handle.set_service_status(service::ServiceStatus {
            service_type: service::ServiceType::OWN_PROCESS,
            current_state: service::ServiceState::StopPending,
            controls_accepted: service::ServiceControlAccept::empty(),
            exit_code: service::ServiceExitCode::Win32(0),
            checkpoint: 1,
            wait_hint: std::time::Duration::from_secs(10),
            process_id: None,
        });
        control_pipe::request_stop();
        let _ = server.join();
        let _ = keepalive.join();
        let _ = status_monitor.join();
        let _ = status_handle.set_service_status(service::ServiceStatus {
            service_type: service::ServiceType::OWN_PROCESS,
            current_state: service::ServiceState::Stopped,
            controls_accepted: service::ServiceControlAccept::empty(),
            exit_code: service::ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        });
    }
}
