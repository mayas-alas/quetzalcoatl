mod deployment;
mod images;
mod package;
mod protocol;
mod updates;
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
            println!(
                "gnx start | stop | status | enroll | password | deploy <carpeta> | update --check|--apply [catalogo.json] | update --force"
            );
            Ok(true)
        }
        [arg] if arg == "--help" || arg == "-h" => {
            println!(
                "gnx start | stop | status | enroll | password | deploy <carpeta>\ngnx update --check|--apply [catalogo.json]\ngnx update --force\ngnx update --apply --force\nCheck sin catálogo consulta releases públicas y muestra la transacción. Apply sin catálogo actualiza el producto por MSI/Burn y luego sus componentes; con catálogo solo aplica componentes. Force recrea el nodo instalado: borra datos, identidades y guests administrados tras inventario y confirmación interactiva. Apply --force selecciona primero la release pública. No reinicia Windows ni purga otras WSL. La entrega al worker no confirma finalización; consulta --check y eventos. Enroll/password solicitan entrada sin eco."
            );
            Ok(true)
        }
        [arg] if arg == "--version" => {
            println!("gnx {}", env!("CARGO_PKG_VERSION"));
            Ok(true)
        }
        #[cfg(windows)]
        [arg] if arg == "--msi-uninstall" || arg == "--msi-recover" => {
            windows::installer(arg == "--msi-uninstall")?;
            Ok(true)
        }
        #[cfg(windows)]
        [arg] if arg == "--product-update-worker" => {
            windows::product_worker()?;
            Ok(true)
        }
        #[cfg(windows)]
        [arg] if arg == "--product-recreate" => {
            windows::product_recreate()?;
            Ok(true)
        }
        #[cfg(windows)]
        [arg] if arg == "--service" => {
            windows::service::dispatch()?;
            Ok(true)
        }
        [arg, path] if arg == "--verify-package" => {
            package::verify(std::path::Path::new(path))?;
            println!("paquete íntegro; instalación y salud aún no comprobadas");
            Ok(true)
        }
        [arg, path] if arg == "--verify-update-catalog" => {
            updates::Catalog::builtin()?;
            updates::Catalog::read(std::path::Path::new(path))?;
            println!("catálogo válido; compatibilidad, instalación y salud no comprobadas");
            Ok(true)
        }
        #[cfg(windows)]
        [arg, mode] if arg == "update" && (mode == "--apply" || mode == "--force") => {
            windows::apply_product(mode == "--force", mode == "--apply")
        }
        #[cfg(windows)]
        [arg, mode, force] if arg == "update" && mode == "--apply" && force == "--force" => {
            windows::apply_product(true, true)
        }
        #[cfg(windows)]
        [arg, mode, rest @ ..] if arg == "update" && rest.len() <= 1 => {
            let operation = match mode.as_str() {
                "--check" => protocol::Operation::UpdateCheck,
                "--apply" => protocol::Operation::UpdateApply,
                _ => {
                    return Err(protocol::invalid(
                        "usa update --check|--apply [catalogo.json]",
                    ));
                }
            };
            if mode == "--check" && rest.is_empty() {
                println!("{}", windows::check_product_release()?);
            }
            let catalog = rest
                .first()
                .map(|path| updates::Catalog::read(std::path::Path::new(path)))
                .transpose()?;
            let response = windows::update(operation, catalog)?;
            println!("{}", response.message);
            Ok(response.ok)
        }
        #[cfg(windows)]
        [arg, path] if arg == "deploy" => {
            let bundle = deployment::Bundle::read(std::path::Path::new(path))?;
            let response = windows::deploy(bundle)?;
            println!("{}", response.message);
            Ok(response.ok)
        }
        [arg] => {
            let op = protocol::Operation::parse(arg).ok_or_else(|| {
                protocol::invalid("usa start, stop, status, enroll, password o deploy <carpeta>")
            })?;
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
            "argumentos no admitidos; consulta gnx --help y no pases secretos por argumentos",
        )),
    }
}
