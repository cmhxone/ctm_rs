use std::{error::Error, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{broadcast, mpsc},
    time::timeout,
};

use crate::{
    cisco::{
        session::OpenReq, Deserializable, FloatingField, MessageType, Serializable, TagValue, MHDR,
    },
    event::{broker_event::BrokerEvent, cti_event::CTIEvent},
};

///
/// CTI 클라이언트 구조체
///
pub struct CTIClient {
    is_active: bool,
    cti_event_channel_tx: mpsc::Sender<CTIEvent>,
    broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
}

impl CTIClient {
    pub async fn new(
        is_active: bool,
        cti_event_channel_tx: mpsc::Sender<CTIEvent>,
        broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            is_active,
            cti_event_channel_tx,
            broker_event_channel_rx,
        })
    }

    pub async fn connect(mut self) -> () {
        let cti_server_address = dotenv::var(match self.is_active {
            true => "CTI_SERVER_SIDE_A_ADDRESS",
            false => "CTI_SERVER_SIDE_B_ADDRESS",
        })
        .unwrap_or("localhost".to_string());
        let cti_server_port = dotenv::var(match self.is_active {
            true => "CTI_SERVER_SIDE_A_PORT",
            false => "CTI_SERVER_SIDE_B_PORT",
        })
        .unwrap_or("42027".to_string());

        let mut client_stream = match timeout(
            Duration::from_millis(3_000),
            TcpStream::connect(format!("{}:{}", cti_server_address, cti_server_port)),
        )
        .await
        {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                self.cti_event_channel_tx
                    .send(CTIEvent::Error {
                        cti_server_host: cti_server_address,
                        error_cause: e.to_string(),
                    })
                    .await
                    .unwrap();
                return;
            }
            Err(e) => {
                self.cti_event_channel_tx
                    .send(CTIEvent::Error {
                        cti_server_host: cti_server_address,
                        error_cause: e.to_string(),
                    })
                    .await
                    .unwrap();
                return;
            }
        };

        tokio::spawn(async move {
            // OPEN_REQ 메시지 전송
            let open_req = OpenReq {
                mhdr: MHDR {
                    length: 0,
                    message_type: MessageType::OPEN_REQ,
                },
                invoke_id: 0,
                version_number: 24,
                idle_timeout: 100,
                peripheral_id: 5000,
                services_requested: 0x8000_0000 | 0x0000_0004 | 0x0000_0010 | 0x0000_0080,
                call_msg_mask: u32::max_value(),
                agent_state_mask: 0x0000_3FFF,
                config_msg_mask: 0,
                reserved1: 0,
                reserved2: 0,
                reserved3: 0,
                client_id: Some(FloatingField {
                    tag: TagValue::CLIENT_ID_TAG,
                    length: 0,
                    data: "ctmonitor_rs".to_string(),
                }),
                client_password: Some(FloatingField {
                    tag: TagValue::CLIENT_PASSWORD_TAG,
                    length: 0,
                    data: "SomePassword!!".to_string(),
                }),
                client_signature: None,
                agent_extension: None,
                agent_id: None,
                agent_instrument: None,
                application_path_id: None,
                unique_instance_id: None,
            };
            match client_stream.write(&open_req.serialize()).await {
                Ok(_) => {
                    log::info!(
                        "Sent OPEN_REQ message. cti_server_host: {}",
                        cti_server_address
                    );
                }
                Err(e) => {
                    self.cti_event_channel_tx
                        .send(CTIEvent::Error {
                            cti_server_host: cti_server_address,
                            error_cause: e.to_string(),
                        })
                        .await
                        .unwrap();
                    return;
                }
            }

            let (mut rx, _tx) = client_stream.split();

            let mut buffer = vec![0_u8; 4_096];
            loop {
                match timeout(Duration::from_millis(10), rx.read(&mut buffer)).await {
                    Ok(Ok(n)) if n == 0 => {
                        self.cti_event_channel_tx
                            .send(CTIEvent::Error {
                                cti_server_host: cti_server_address.clone(),
                                error_cause: "Disconnected from server".to_string(),
                            })
                            .await
                            .unwrap();
                        log::error!("Disconnected from server");
                        return;
                    }
                    Ok(Ok(n)) => {
                        // CTI 서버로부터 패킷을 전송받은 경우
                        let mut index = 0_usize;

                        // 여러 메시지를 한 패킷에 받을 수 있어 분리해서 처리한다
                        while index < n {
                            // 메시지 헤더 조회
                            let (_, mhdr) =
                                MHDR::deserialize(&mut buffer[index..index + 8].to_vec());

                            self.cti_event_channel_tx
                                .send(CTIEvent::Recevied {
                                    cti_server_host: cti_server_address.clone(),
                                    message_type: mhdr.message_type,
                                    data: buffer[index..index + (mhdr.length + 8) as usize]
                                        .to_vec(),
                                })
                                .await
                                .unwrap();

                            // 현재 인덱스 증가
                            index = index + 8 + mhdr.length as usize;
                        }
                    }
                    Ok(Err(e)) => {
                        // CTI 이벤트 채널로 오류 이벤트를 발생시킨다
                        self.cti_event_channel_tx
                            .send(CTIEvent::Error {
                                cti_server_host: cti_server_address.clone(),
                                error_cause: e.to_string(),
                            })
                            .await
                            .unwrap();
                        log::error!("Read error. {:#?}", e);
                        return;
                    }
                    Err(_) => {}
                }

                match timeout(
                    Duration::from_millis(10),
                    self.broker_event_channel_rx.recv(),
                )
                .await
                {
                    Ok(Ok(event)) => match event {
                        BrokerEvent::ToClientEvent {} => {}
                        BrokerEvent::ToServerEvent {} => {
                            log::debug!("Received broking server event");
                        }
                    },
                    Ok(Err(e)) => {
                        log::error!("Unabled to receive broking event. {:?}", e);
                    }
                    Err(_) => {}
                }
            }
        });
    }
}
