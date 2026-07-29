#[cfg(windows)]
mod application;
#[cfg(windows)]
mod domain;
#[cfg(windows)]
mod infrastructure;

#[cfg(windows)]
fn main() {
    if let Err(error) = application::pipe_service::run() {
        eprintln!("gnx-service: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("gnx-service requires Windows");
    std::process::exit(1);
}
