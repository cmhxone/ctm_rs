use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;
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

const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"; // RFC 6455
const WEBSOCKET_FIN_TRUE: u8 = 0x80;
#[allow(unused)]
const WEBSOCKET_FIN_FALSE: u8 = 0x00;
#[allow(unused)]
const WEBSOCKET_OP_CODE_CONTINUATION_FRAME: u8 = 0x00;
const WEBSOCKET_OP_CODE_TEXT_FRAME: u8 = 0x01;
const WEBSOCKET_OP_CODE_BINARY_FRAME: u8 = 0x02;
#[allow(unused)]
const WEBSOCKET_OP_CODE_CLOSE_FRAME: u8 = 0x08;
#[allow(unused)]
const WEBSOCKET_OP_CODE_PING_FRAME: u8 = 0x09;
#[allow(unused)]
const WEBSOCKET_OP_CODE_PONG_FRAME: u8 = 0x0A;

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

#[async_trait]
impl Acceptor for WebsocketAcceptor {
    async fn accept(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
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
                        hasher.update(format!("{}{}", websocket_key, WEBSOCKET_GUID));
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
    /// 패킷 데이터 전송
    ///
    async fn write(&mut self, buffer: &[u8]) -> Result<usize, Box<dyn Error + Send + Sync>> {
        match self {
            ClientStream::Plain {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&buffer).await?),
            ClientStream::Secure {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&buffer).await?),
        }
    }

    ///
    /// 이진 데이터 전송
    ///
    async fn write_binary(&mut self, buffer: &[u8]) -> Result<usize, Box<dyn Error + Send + Sync>> {
        let length = buffer.len();
        let mut send_buffer = Vec::new();

        // 웹 소켓 프레임 헤더 추가
        send_buffer.push(WEBSOCKET_FIN_TRUE | WEBSOCKET_OP_CODE_BINARY_FRAME);

        // 웹 소켓 길이 패킷 추가
        match length {
            0_usize..126_usize => {
                send_buffer.push(length as u8);
            }
            126_usize..65535_usize => {
                send_buffer.push(126_u8);
                send_buffer.push(((length & 0xFF00) >> 8) as u8);
                send_buffer.push((length & 0x00FF) as u8);
            }
            65535_usize.. => {
                send_buffer.push(127_u8);
                send_buffer.push(((length & 0xFF00_0000_0000_0000) >> 56) as u8);
                send_buffer.push(((length & 0x00FF_0000_0000_0000) >> 48) as u8);
                send_buffer.push(((length & 0x0000_FF00_0000_0000) >> 40) as u8);
                send_buffer.push(((length & 0x0000_00FF_0000_0000) >> 32) as u8);
                send_buffer.push(((length & 0x0000_0000_FF00_0000) >> 24) as u8);
                send_buffer.push(((length & 0x0000_0000_00FF_0000) >> 16) as u8);
                send_buffer.push(((length & 0x0000_0000_0000_FF00) >> 8) as u8);
                send_buffer.push((length & 0x0000_0000_0000_00FF) as u8);
            }
        };

        // 웹 소켓 데이터 추가
        send_buffer.append(&mut buffer.to_vec());

        match self {
            ClientStream::Plain {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&send_buffer).await?),
            ClientStream::Secure {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&send_buffer).await?),
        }
    }

    ///
    /// 종료 프레임 전송
    ///
    async fn write_close(&mut self, reason: u16) -> Result<usize, Box<dyn Error + Send + Sync>> {
        let length = 2;
        let mut send_buffer = Vec::new();

        // 웹 소켓 프레임 헤더 추가
        send_buffer.push(WEBSOCKET_FIN_TRUE | WEBSOCKET_OP_CODE_CLOSE_FRAME);
        // 웹 소켓 길이 패킷 추가
        send_buffer.push(length as u8);

        // CLOSE 이유 패킷 추가
        send_buffer.push(((reason & 0xFF00) >> 8) as u8);
        send_buffer.push((reason & 0xFF) as u8);

        match self {
            ClientStream::Plain {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&send_buffer).await.unwrap()),
            ClientStream::Secure {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&send_buffer).await.unwrap()),
        }
    }

    ///
    /// 커넥션 종료
    ///
    async fn close(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        match self {
            ClientStream::Plain {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.shutdown().await?),
            ClientStream::Secure {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.shutdown().await?),
        }
    }

    ///
    /// 텍스트 데이터 전송
    ///
    #[allow(unused)]
    async fn write_text(&mut self, message: String) -> Result<usize, Box<dyn Error + Send + Sync>> {
        let length = message.len();
        let mut send_buffer = Vec::new();

        // 웹 소켓 프레임 헤더 추가
        send_buffer.push(WEBSOCKET_FIN_TRUE | WEBSOCKET_OP_CODE_TEXT_FRAME);

        // 웹 소켓 길이 패킷 추가
        match length {
            0_usize..126_usize => {
                send_buffer.push(length as u8);
            }
            126_usize..65535_usize => {
                send_buffer.push(126_u8);
                send_buffer.push(((length & 0xFF00) >> 8) as u8);
                send_buffer.push((length & 0x00FF) as u8);
            }
            65535_usize.. => {
                send_buffer.push(127_u8);
                send_buffer.push(((length & 0xFF00_0000_0000_0000) >> 56) as u8);
                send_buffer.push(((length & 0x00FF_0000_0000_0000) >> 48) as u8);
                send_buffer.push(((length & 0x0000_FF00_0000_0000) >> 40) as u8);
                send_buffer.push(((length & 0x0000_00FF_0000_0000) >> 32) as u8);
                send_buffer.push(((length & 0x0000_0000_FF00_0000) >> 24) as u8);
                send_buffer.push(((length & 0x0000_0000_00FF_0000) >> 16) as u8);
                send_buffer.push(((length & 0x0000_0000_0000_FF00) >> 8) as u8);
                send_buffer.push((length & 0x0000_0000_0000_00FF) as u8);
            }
        };

        // 웹 소켓 데이터 추가
        send_buffer.append(&mut message.as_bytes().to_vec());

        match self {
            ClientStream::Plain {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&send_buffer).await?),
            ClientStream::Secure {
                stream,
                id: _,
                addr: _,
            } => Ok(stream.write(&send_buffer).await?),
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
            // 웹 소켓 데이터 수신
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

                    // CLOSE 프레임 수신하면 커넥션 닫아버림
                    if &buffer[0] & WEBSOCKET_OP_CODE_CLOSE_FRAME != 0_u8 {
                        self.write_close(1_000_u16).await?;
                        self.close().await?;
                    }
                }
                Ok(Err(e)) => {
                    log::error!(
                        "Websocket client error. {:?}, client_addr: {}",
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

        Ok(())
    }
}
