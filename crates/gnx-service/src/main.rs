#[cfg(windows)]
mod pipe;

#[cfg(windows)]
fn main() {
    if let Err(error) = pipe::serve() {
        eprintln!("gnx-service: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("gnx-service requires Windows");
    std::process::exit(1);
}
