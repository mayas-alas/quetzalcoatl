mod identity;
mod node;
mod pipe;
mod security;
pub mod service;

pub fn client(op: crate::protocol::Operation) -> std::io::Result<crate::protocol::Response> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(pipe::client(op))
}
