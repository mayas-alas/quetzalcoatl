mod access;
mod bootstrap;
mod deploy;
mod identity;
mod node;
mod pipe;
mod security;
pub mod service;
mod updates;

pub fn installer(remove: bool) -> std::io::Result<()> {
    // MSI uses the existing authenticated service boundary, not another token/
    // profile lifecycle in a deferred child process.
    let operation = if remove {
        crate::protocol::Operation::Retire
    } else {
        match security::read_private("intent", 16)?.as_slice() {
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
