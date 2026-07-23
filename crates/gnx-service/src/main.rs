#[cfg(windows)]
mod pipe;
#[cfg(windows)]
mod runtime_gate;
#[cfg(windows)]
mod secrets;
#[cfg(windows)]
mod service_secrets;
#[cfg(windows)]
mod state;

#[cfg(windows)]
fn main() {
    use std::sync::{Arc, Mutex, RwLock};

    let status = Arc::new(RwLock::new(gnx_protocol::StatusResponse::service_ready()));
    let operation = Arc::new(Mutex::new(()));
    let runtime_status = Arc::clone(&status);
    let runtime_operation = Arc::clone(&operation);
    std::thread::spawn(move || match runtime_operation.lock() {
        Ok(_guard) => runtime_gate::run(runtime_status),
        Err(_) => eprintln!("gnx-service: runtime operation lock is poisoned"),
    });

    if let Err(error) = pipe::serve(status, operation) {
        eprintln!("gnx-service: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("gnx-service requires Windows");
    std::process::exit(1);
}
