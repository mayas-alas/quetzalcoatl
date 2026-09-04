#[cfg(windows)]
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [] => gnx::windows::service::run(),
        [flag] if flag == "--version" => {
            println!("gnx-service {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [flag] if flag == "--start" => gnx::windows::service::start(),
        [flag] if flag == "--stop" => gnx::windows::service::stop(),
        [flag] if flag == "--ping" => gnx::windows::broker::ping(),
        [flag, sid] if flag == "--install" => std::env::current_exe()
            .map_err(gnx::Error::Spawn)
            .and_then(|path| gnx::windows::service::install(&path, sid)),
        _ => Err(gnx::Error::Arguments),
    };
    if let Err(error) = result {
        eprintln!("FAILED {}", error.label());
        std::process::exit(error.exit_code() as i32);
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("FAILED BROKER_HOST_UNSUPPORTED");
    std::process::ExitCode::FAILURE
}
