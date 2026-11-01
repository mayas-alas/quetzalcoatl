mod access;
mod bootstrap;
mod console;
mod deploy;
mod identity;
mod installer_package;
mod maintenance;
mod node;
mod pipe;
mod product_update;
mod recreation;
mod release_source;
mod security;
pub mod service;
mod updates;

pub fn check_product_release() -> std::io::Result<String> {
    security::effective_admin()?;
    if let Some(receipt) = product_update::receipt()? {
        println!("{receipt}");
    }
    release_source::check()
}

pub fn apply_product(force: bool, discover: bool) -> std::io::Result<bool> {
    product_update::apply(force, discover)
}
pub fn product_worker() -> std::io::Result<()> {
    product_update::dispatch()
}
pub fn product_recreate() -> std::io::Result<()> {
    product_update::recreate_installed().inspect_err(service::report_error)
}

pub fn installer(remove: bool) -> std::io::Result<()> {
    // MSI uses the existing authenticated service boundary, not another token/
    // profile lifecycle in a deferred child process.
    let operation = if remove {
        crate::protocol::Operation::Retire
    } else {
        let intent = match security::read_private("intent", 16) {
            Ok(value) => value,
            // An unprepared installation has no prior running/stopped intent.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        match intent.as_slice() {
            b"running" => crate::protocol::Operation::Start,
            b"stopped" => crate::protocol::Operation::Stop,
            _ => return Err(crate::protocol::invalid("intención inválida")),
        }
    };
    let response = client(operation)?;
    if response.ok {
        Ok(())
    } else {
        Err(std::io::Error::other(response.message))
    }
}

pub fn client(op: crate::protocol::Operation) -> std::io::Result<crate::protocol::Response> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(pipe::client(op))
}

pub fn deploy(bundle: crate::deployment::Bundle) -> std::io::Result<crate::protocol::Response> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(pipe::client_bundle(
            crate::protocol::Operation::Deploy,
            Some(bundle),
        ))
}

pub fn update(
    operation: crate::protocol::Operation,
    catalog: Option<crate::updates::Catalog>,
) -> std::io::Result<crate::protocol::Response> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(pipe::client_request(operation, None, catalog))
}
