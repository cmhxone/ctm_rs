use std::{error::Error, sync::Arc};

use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tokio_rustls::{server::TlsStream, TlsAcceptor};

use super::Acceptor;

///
/// 클라이언트 TCP 스트림
///
enum ClientStream {
    Plain { stream: TcpStream },
    Secure { stream: TlsStream<TcpStream> },
}

impl ClientStream {
    ///
    /// 데이터 전송
    ///
    pub async fn write(self, buffer: &[u8]) -> Result<(), Box<dyn Error>> {
        match self {
            ClientStream::Plain { mut stream } => {
                stream.write(buffer).await?;
            }
            ClientStream::Secure { mut stream } => {
                stream.write(buffer).await?;
            }
        }

        Ok(())
    }
}

///
/// TCP Acceptor
///
pub struct TCPAcceptor {
    tcp_listener: TcpListener,
    tls_acceptor: Option<TlsAcceptor>,
}

impl TCPAcceptor {
    ///
    /// TCPAcceptor 생성
    ///
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let ssl_enabled = dotenv::var("TCP_ACCEPTOR_SECURE")
            .unwrap_or("false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // TCP 소켓 서버 초기화
        let tcp_listener = TcpListener::bind(format!(
            "0.0.0.0:{}",
            dotenv::var("TCP_ACCEPTOR_PORT").unwrap_or("5110".to_string())
        ))
        .await?;

        // TLS acceptor 생성
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

impl Acceptor for TCPAcceptor {
    ///
    /// 클라이언트 수신
    ///
    async fn accept(self) -> Result<(), Box<dyn Error>> {
        log::info!("TCP server starts accepting");

        loop {
            match self.tcp_listener.accept().await {
                Ok((native_stream, client_addr)) => {
                    log::info!("TCP client connected. client_addr: {:?}", client_addr);

                    // TLS 적용 여부에 따라 클라이언트 소켓 스트림을 구분
                    let client_stream = match self.tls_acceptor {
                        Some(ref tls) => ClientStream::Secure {
                            stream: match tls.accept(native_stream).await {
                                Ok(stream) => stream,
                                Err(_) => continue,
                            },
                        },
                        None => ClientStream::Plain {
                            stream: native_stream,
                        },
                    };

                    client_stream
                        .write("hello from ctm".as_bytes())
                        .await
                        .unwrap();
                }
                Err(e) => {
                    log::error!("Unable to accept TCP client connection. {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
