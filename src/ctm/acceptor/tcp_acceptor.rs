use std::{error::Error, fs::File, io::BufReader, sync::Arc};

use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{server::TlsStream, TlsAcceptor};

enum ClientStream {
    Plain { stream: TcpStream },
    Secure { stream: TlsStream<TcpStream> },
}

pub struct TCPAcceptor {
    tcp_listener: TcpListener,
    tls_acceptor: Option<TlsAcceptor>,
}

impl TCPAcceptor {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let ssl_enabled = dotenv::var("TCP_ACCEPTOR_ENABLED")
            .unwrap_or("false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let tcp_listener = TcpListener::bind(format!(
            "0.0.0.0:{}",
            dotenv::var("TCP_ACCEPTOR_PORT").unwrap_or("5110".to_string())
        ))
        .await?;

        let mut tls_acceptor = None;

        if ssl_enabled {
            let cert = dotenv::var("TCP_ACCEPTOR_SECURE_CERT_FILE")
                .unwrap_or("./res/ssl/server.crt".to_string());
            let key = dotenv::var("TCP_ACCEPTOR_SECURE_KEY_FILE")
                .unwrap_or("./res/ssl/server.key".to_string());

            let cert = CertificateDer::pem_file_iter(cert)?.collect::<Result<Vec<_>, _>>()?;
            let key = PrivateKeyDer::from_pem_file(key)?;

            let tls_config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(cert, key)?;

            tls_acceptor = Some(TlsAcceptor::from(Arc::new(tls_config)));
        }

        Ok(Self {
            tcp_listener,
            tls_acceptor,
        })
    }
}
