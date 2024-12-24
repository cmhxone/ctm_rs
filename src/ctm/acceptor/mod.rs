use std::error::Error;

pub mod tcp_acceptor;
pub mod websocket_acceptor;

pub trait Acceptor {
    async fn accept() -> Result<(), Box<dyn Error>>;
}
