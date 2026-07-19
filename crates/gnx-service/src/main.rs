#[cfg(windows)]
mod pipe;
#[cfg(windows)]
mod runtime_gate;
#[cfg(windows)]
mod secrets;

#[cfg(windows)]
fn main() {
    use std::sync::{Arc, RwLock};

    let status = Arc::new(RwLock::new(gnx_protocol::StatusResponse::service_ready()));
    let runtime_status = Arc::clone(&status);
    std::thread::spawn(move || runtime_gate::run(runtime_status));

    if let Err(error) = pipe::serve(status) {
        eprintln!("gnx-service: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("gnx-service requires Windows");
    std::process::exit(1);
}
