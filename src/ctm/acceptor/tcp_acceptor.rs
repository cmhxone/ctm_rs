use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    time::timeout,
};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use uuid::Uuid;

use crate::event::{broker_event::BrokerEvent, client_event::ClientEvent};

use super::Acceptor;

///
/// TCP Acceptor
///
pub struct TCPAcceptor {
    tcp_listener: TcpListener,
    tls_acceptor: Option<TlsAcceptor>,
    broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
    client_event_channel_tx: mpsc::Sender<ClientEvent>,
}

impl TCPAcceptor {
    ///
    /// TCPAcceptor 생성
    ///
    pub async fn new(
        broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
        client_event_channel_tx: mpsc::Sender<ClientEvent>,
    ) -> Result<Self, Box<dyn Error>> {
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
            broker_event_channel_rx,
            client_event_channel_tx,
        })
    }
}

#[async_trait]
impl Acceptor for TCPAcceptor {
    ///
    /// 클라이언트 수신
    ///
    async fn accept(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::info!("TCP server starts accepting");

        loop {
            match self.tcp_listener.accept().await {
                Ok((native_stream, client_addr)) => {
                    let uuid = Uuid::now_v7();
                    log::info!(
                        "TCP client connected. client_addr: {:?}, id: {}",
                        client_addr,
                        uuid
                    );

                    // TLS 적용 여부에 따라 클라이언트 소켓 스트림을 구분
                    let mut client_stream = match self.tls_acceptor {
                        Some(ref tls) => ClientStream::Secure {
                            stream: match tls.accept(native_stream).await {
                                Ok(stream) => stream,
                                Err(_) => continue,
                            },
                            id: uuid,
                            addr: client_addr.clone(),
                        },
                        None => ClientStream::Plain {
                            stream: native_stream,
                            id: uuid,
                            addr: client_addr.clone(),
                        },
                    };

                    // 접속된 클라이언트 핸들링
                    let broker_event_channel_rx = self.broker_event_channel_rx.resubscribe();
                    let client_event_channel_tx = self.client_event_channel_tx.clone();
                    tokio::spawn(async move {
                        client_stream
                            .handle(broker_event_channel_rx, client_event_channel_tx)
                            .await
                            .unwrap();
                        log::info!("TCP client disconnected. client_addr: {:?}", client_addr);
                    });
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

///
/// 클라이언트 TCP 스트림
///
enum ClientStream {
    Plain {
        stream: TcpStream,
        id: Uuid,
        addr: SocketAddr,
    },
    Secure {
        stream: TlsStream<TcpStream>,
        id: Uuid,
        addr: SocketAddr,
    },
}

impl ClientStream {
    ///
    /// ID 반환
    ///
    fn get_id(&self) -> &Uuid {
        match self {
            ClientStream::Plain {
                stream: _,
                id,
                addr: _,
            } => id,
            ClientStream::Secure {
                stream: _,
                id,
                addr: _,
            } => id,
        }
    }

    ///
    /// 주소 반환
    ///
    fn get_addr(&self) -> &SocketAddr {
        match self {
            ClientStream::Plain {
                stream: _,
                id: _,
                addr,
            } => addr,
            ClientStream::Secure {
                stream: _,
                id: _,
                addr,
            } => addr,
        }
    }

    ///
    /// 데이터 전송
    ///
    async fn write(&mut self, buffer: &[u8]) -> Result<usize, Box<dyn Error + Send + Sync>> {
        match self {
            ClientStream::Plain {
                ref mut stream,
                id: _,
                addr: _,
            } => Ok(stream.write(buffer).await?),
            ClientStream::Secure {
                ref mut stream,
                id: _,
                addr: _,
            } => Ok(stream.write(buffer).await?),
        }
    }

    ///
    /// 데이터 수신
    ///
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error + Send + Sync>> {
        match self {
            ClientStream::Plain {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.read(buffer).await?),
            ClientStream::Secure {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.read(buffer).await?),
        }
    }

    ///
    /// 클라이언트 핸들링
    ///
    pub async fn handle(
        &mut self,
        mut broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
        client_event_channel_tx: mpsc::Sender<ClientEvent>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut buffer = vec![0_u8; 4_096];

        // 클라이언트 소켓 접속 이벤트 전송
        client_event_channel_tx
            .send(ClientEvent::Connect {
                id: self.get_id().clone(),
            })
            .await
            .unwrap();

        loop {
            // 소켓 데이터 수신
            match timeout(Duration::from_millis(10), self.read(&mut buffer)).await {
                Ok(Ok(n)) if n == 0 => {
                    break;
                }
                Ok(Ok(n)) => {
                    log::debug!(
                        "Client send. client_id: {}, client_addr: {}, buffer: {:?}",
                        self.get_id(),
                        self.get_addr(),
                        &buffer[0..n]
                    );
                }
                Ok(Err(e)) => {
                    log::error!(
                        "TCP Client error. {:?}, client_addr: {}",
                        e,
                        self.get_addr()
                    );
                    break;
                }
                Err(_) => {}
            }

            // 브로킹 이벤트 수신
            match timeout(Duration::from_millis(10), broker_event_channel_rx.recv()).await {
                Ok(Ok(event)) => match event {
                    BrokerEvent::BroadCastAgentState {
                        agent_info,
                        client_id,
                    } => {
                        match client_id {
                            // id 값이 있을땐 매칭되지 않을 경우 처리하지 않음
                            Some(id) => {
                                if &id != self.get_id() {
                                    continue;
                                }
                            }
                            None => {}
                        }

                        let mut buffer = Vec::new();
                        agent_info
                            .serialize(&mut rmp_serde::Serializer::new(&mut buffer))
                            .unwrap();

                        self.write(&buffer).await.unwrap();
                    }
                    _ => {}
                },
                Ok(Err(e)) => {
                    log::error!("Unable to read broker message. {:?}", e);
                    break;
                }
                Err(_) => {}
            }
        }

        #[allow(unreachable_code)]
        Ok(())
    }
}
