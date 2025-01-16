use std::error::Error;

use async_trait::async_trait;

pub mod tcp_acceptor;
pub mod websocket_acceptor;

#[async_trait]
pub trait Acceptor: Send {
    async fn accept(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}
