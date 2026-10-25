mod protocol;
#[cfg(windows)]
mod windows;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(error) => {
            eprintln!("gnx: {error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run() -> std::io::Result<bool> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!("gnx start | stop | status");
            Ok(true)
        }
        [arg] if arg == "--help" || arg == "-h" => {
            println!(
                "gnx start | stop | status\nControl del nodo propio mediante systemd. No instala ni actualiza."
            );
            Ok(true)
        }
        [arg] if arg == "--version" => {
            println!("gnx {}", env!("CARGO_PKG_VERSION"));
            Ok(true)
        }
        #[cfg(windows)]
        [arg] if arg == "--service" => {
            windows::service::dispatch()?;
            Ok(true)
        }
        [arg] => {
            let op = protocol::Operation::parse(arg)
                .ok_or_else(|| protocol::invalid("usa start, stop o status"))?;
            #[cfg(windows)]
            {
                let response = windows::client(op)?;
                println!("{}", response.message);
                Ok(response.ok)
            }
            #[cfg(not(windows))]
            {
                let _ = op;
                Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "la CLI requiere Windows",
                ))
            }
        }
        _ => Err(protocol::invalid(
            "no se admiten comandos, rutas ni secretos como argumentos",
        )),
    }
}
