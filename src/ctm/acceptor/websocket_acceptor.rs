use std::{error::Error, sync::Arc, time::Duration};

use base64::{prelude::BASE64_STANDARD, Engine};
use rustls::{
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use serde::Serialize;
use sha1::{Digest, Sha1};
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

pub struct WebsocketAcceptor {
    websocket_listener: TcpListener,
    tls_acceptor: Option<TlsAcceptor>,
    broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
    client_event_channel_tx: mpsc::Sender<ClientEvent>,
}

impl WebsocketAcceptor {
    pub async fn new(
        broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
        client_event_channel_tx: mpsc::Sender<ClientEvent>,
    ) -> Result<Self, Box<dyn Error>> {
        let ssl_enabled = dotenv::var("WEBSOCKET_ACCEPTOR_SECURE")
            .unwrap_or("false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // 웹 소켓 서버 초기화
        let websocket_listener = TcpListener::bind(format!(
            "0.0.0.0:{}",
            dotenv::var("WEBSOCKET_ACCEPTOR_PORT").unwrap_or("8085".to_string())
        ))
        .await?;

        // TLS acceptor 생성
        let mut tls_acceptor = None;
        if ssl_enabled {
            let cert = dotenv::var("WEBSOCKET_ACCEPTOR_SECURE_CERT_FILE")
                .unwrap_or("./res/ssl/server.crt".to_string());
            let key = dotenv::var("WEBSOCKET_ACCEPTOR_SECURE_KEY_FILE")
                .unwrap_or("./res/ssl/server.key".to_string());
            let cert = CertificateDer::pem_file_iter(cert)?.collect::<Result<Vec<_>, _>>()?;
            let key = PrivateKeyDer::from_pem_file(key)?;

            let tls_config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(cert, key)?;

            tls_acceptor = Some(TlsAcceptor::from(Arc::new(tls_config)));
        }

        Ok(Self {
            websocket_listener,
            tls_acceptor,
            broker_event_channel_rx,
            client_event_channel_tx,
        })
    }
}

impl Acceptor for WebsocketAcceptor {
    async fn accept(self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Websocket server starts accepting");

        loop {
            match self.websocket_listener.accept().await {
                Ok((native_stream, client_addr)) => {
                    let uuid = Uuid::now_v7();
                    log::info!(
                        "Websocket client connected. client_addr: {:?}, id: {}",
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
                        },
                        None => ClientStream::Plain {
                            stream: native_stream,
                            id: uuid,
                        },
                    };

                    // 접속된 클라이언트 핸들링
                    let broker_event_channel_rx = self.broker_event_channel_rx.resubscribe();
                    let client_event_channel_tx = self.client_event_channel_tx.clone();
                    tokio::spawn(async move {
                        // HTTP 요청 수신
                        let mut buffer = vec![0_u8; 2_048];
                        let length = client_stream.read(&mut buffer).await.unwrap();

                        let path = dotenv::var("WEBSOCKET_ACCEPTOR_PATH")
                            .unwrap_or("/ctmonitor".to_string());

                        // 요청 헤더 데이터 검증
                        let request_header =
                            String::from_utf8((&buffer[0..length]).to_vec()).unwrap();
                        log::debug!("Websocket client request header: {}", request_header);

                        // 헤더 경로가 잘못된 경우 허용하지 않는다
                        let header_regex =
                            regex::Regex::new(format!(r"^GET {} ", path).as_str()).unwrap();
                        match header_regex.captures(&request_header) {
                            Some(_) => {}
                            None => {
                                log::debug!("Websocket client requested invalid path");
                                client_stream
                                    .write(r"HTTP/1.1 400 Bad Request".as_bytes())
                                    .await
                                    .unwrap();
                                return;
                            }
                        }

                        let header_regex = regex::Regex::new(r"^Upgrade|Sec-WebSocket").unwrap();
                        // 업그레이드, 웹소켓 메시지가 없는 경우 허용하지 않는다
                        match header_regex.captures(&request_header) {
                            Some(_) => {}
                            None => {
                                log::debug!(
                                    "Websocket client requested without websocket default headers"
                                );
                                client_stream
                                    .write(r"HTTP/1.1 400 Bad Request".as_bytes())
                                    .await
                                    .unwrap();
                                return;
                            }
                        };

                        // 웹 소켓 키를 사용해 Accept 키를 만든다
                        let header_regex =
                            regex::Regex::new(r"Sec-WebSocket-Key: ([0-9a-zA-Z+=/]*)").unwrap();
                        let websocket_key = match header_regex.captures(&request_header) {
                            Some(captures) => captures.get(1).unwrap().as_str(),
                            None => {
                                log::debug!(
                                    "Websocket client request without Sec-WebSocket-Key header"
                                );
                                client_stream
                                    .write(r"HTTP/1.1 400 Bad Request".as_bytes())
                                    .await
                                    .unwrap();
                                return;
                            }
                        };
                        log::debug!("Websocket client request to accept. client_addr: {:?}, websocket_key: '{}'", client_addr, websocket_key);

                        // 웹소켓 Upgrade 응답 메시지 전송
                        let mut hasher = Sha1::new();
                        hasher.update(format!(
                            "{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", // RFC 6455
                            websocket_key
                        ));
                        let websocket_accept = hasher.finalize();
                        let websocket_accept = BASE64_STANDARD.encode(websocket_accept);

                        log::debug!("Websocket client accept key: {}", websocket_accept);

                        // 웹소켓 101 Switching Protocols 전송
                        client_stream
                            .write(
                                format!(
                                    "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-Websocket-Accept: {}\r\n\r\n",
                                    websocket_accept
                                )
                                .as_bytes(),
                            )
                            .await
                            .unwrap();

                        client_stream
                            .handle(broker_event_channel_rx, client_event_channel_tx)
                            .await
                            .unwrap();
                        log::info!(
                            "Websocket client disconnected. client_addr: {:?}",
                            client_addr
                        );
                    });
                }
                Err(e) => {
                    log::error!("Unable to accept Websocket client connection. {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

///
/// 클라이언트 웹 소켓 스트림
///
enum ClientStream {
    Plain {
        stream: TcpStream,
        id: Uuid,
    },
    Secure {
        stream: TlsStream<TcpStream>,
        id: Uuid,
    },
}

impl ClientStream {
    ///
    /// ID 반환
    ///
    fn get_id(&self) -> &Uuid {
        match self {
            ClientStream::Plain { stream: _, id } => id,
            ClientStream::Secure { stream: _, id } => id,
        }
    }

    ///
    /// 패킷 데이터 전송
    ///
    async fn write(&mut self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
        match self {
            ClientStream::Plain { stream, id: _ } => Ok(stream.write(&buffer).await?),
            ClientStream::Secure { stream, id: _ } => Ok(stream.write(&buffer).await?),
        }
    }

    ///
    /// 이진 데이터 전송
    ///
    async fn write_binary(&mut self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
        match self {
            ClientStream::Plain { stream, id: _ } => todo!(),
            ClientStream::Secure { stream, id: _ } => todo!(),
        }
    }

    ///
    /// 텍스트 데이터 전송
    ///
    async fn write_text(&mut self, message: String) -> Result<usize, Box<dyn Error>> {
        match self {
            ClientStream::Plain { stream, id: _ } => todo!(),
            ClientStream::Secure { stream, id: _ } => todo!(),
        }
    }

    ///
    /// 데이터 수신
    ///
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error>> {
        match self {
            ClientStream::Plain { stream, id: _ } => Ok(stream.read(buffer).await?),
            ClientStream::Secure { stream, id: _ } => Ok(stream.read(buffer).await?),
        }
    }

    pub async fn handle(
        &mut self,
        mut broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
        client_event_channel_tx: mpsc::Sender<ClientEvent>,
    ) -> Result<(), Box<dyn Error>> {
        let mut buffer = vec![0_u8; 4_096];

        // 클라이언트 소켓 접속 이벤트 전송
        client_event_channel_tx
            .send(ClientEvent::Connect {
                id: self.get_id().clone(),
            })
            .await
            .unwrap();

        loop {
            // 웹 소켓 데이터 수신
            match timeout(Duration::from_millis(10), self.read(&mut buffer)).await {
                Ok(Ok(n)) if n == 0 => {
                    break;
                }
                Ok(Ok(n)) => {
                    log::info!("Client send. {:?}", &buffer[0..n]);
                }
                Ok(Err(e)) => {
                    log::error!("Websocket client error. {:?}", e);
                    break;
                }
                Err(_) => {}
            }

            // 브로킹 이벤트 수신
            match timeout(Duration::from_millis(10), broker_event_channel_rx.recv()).await {
                Ok(Ok(event)) => match event {
                    BrokerEvent::BroadCastAgentState {
                        client_id,
                        agent_info,
                    } => {
                        match client_id {
                            Some(id) => {
                                if &id != self.get_id() {
                                    continue;
                                }
                            }
                            None => {}
                        };

                        let mut buffer = Vec::new();
                        agent_info
                            .serialize(&mut rmp_serde::Serializer::new(&mut buffer))
                            .unwrap();

                        self.write_binary(&buffer).await.unwrap();
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

        todo!()
    }
}
