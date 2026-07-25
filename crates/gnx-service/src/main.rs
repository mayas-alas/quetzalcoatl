#[cfg(windows)]
mod ipc;
#[cfg(windows)]
mod runtime;
#[cfg(windows)]
mod secrets;
#[cfg(windows)]
mod service;
#[cfg(windows)]
mod state;

#[cfg(windows)]
fn main() {
    if let Err(error) = service::run() {
        eprintln!("gnx-service: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("gnx-service requires Windows");
    std::process::exit(1);
}
