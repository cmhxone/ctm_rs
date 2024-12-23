use std::{error::Error, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

use crate::cisco::{
    session::{OpenConf, OpenReq},
    Deserializable, FloatingField, MessageType, Serializable, TagValue, MHDR,
};

///
/// CTI 클라이언트 구조체
///
pub struct CTIClient {
    client_stream: TcpStream,
}

impl CTIClient {
    pub async fn new(is_active: bool) -> Result<Self, Box<dyn Error>> {
        let cti_server_address = dotenv::var(match is_active {
            true => "CTI_SERVER_SIDE_A_ADDRESS",
            false => "CTI_SERVER_SIDE_B_ADDRESS",
        })?;
        let cti_server_port = dotenv::var(match is_active {
            true => "CTI_SERVER_SIDE_A_PORT",
            false => "CTI_SERVER_SIDE_B_PORT",
        })?;

        let mut client_stream =
            TcpStream::connect(format!("{}:{}", cti_server_address, cti_server_port)).await?;

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
        client_stream.write(&open_req.serialize()).await?;

        Ok(Self { client_stream })
    }

    pub async fn connect(mut self) -> Result<(), Box<dyn Error>> {
        tokio::spawn(async move {
            let (mut rx, _tx) = self.client_stream.split();

            let mut buffer = vec![0_u8; 4_096];
            loop {
                match timeout(Duration::from_millis(10), rx.read(&mut buffer)).await {
                    Ok(Ok(n)) if n == 0 => {}
                    Ok(Ok(n)) => {
                        // CTI 서버로부터 패킷을 전송받은 경우
                        let mut index = 0_usize;

                        // 여러 메시지를 한 패킷에 받을 수 있어 분리해서 처리한다
                        while index < n {
                            // 메시지 헤더 조회
                            let (_, mhdr) =
                                MHDR::deserialize(&mut buffer[index..index + 8].to_vec());

                            // 메시지 역직렬화
                            match mhdr.message_type {
                                // OPEN_CONF 메시지 수신
                                MessageType::OPEN_CONF => {
                                    let (_, open_conf) = OpenConf::deserialize(
                                        &mut buffer[index..(mhdr.length + 8) as usize].to_vec(),
                                    );
                                    println!("{:?}", open_conf);
                                }
                                // 처리되지 않은 메시지 수신
                                message_type => {
                                    println!(
                                        "Received CTI message. message_type: {:?}",
                                        message_type
                                    );
                                }
                            }

                            // 현재 인덱스 증가
                            index = index + 8 + mhdr.length as usize;
                        }
                    }
                    Ok(Err(e)) => {
                        log::error!("Read error. {:#?}", e);
                    }
                    Err(_) => {}
                }
            }
        });

        Ok(())
    }
}
