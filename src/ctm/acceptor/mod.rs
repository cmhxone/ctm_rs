use std::error::Error;

pub mod tcp_acceptor;
pub mod websocket_acceptor;

pub trait Acceptor {
    async fn accept(self) -> Result<(), Box<dyn Error>>;
}
