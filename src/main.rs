use std::error::Error;

use cisco::{
    session::{OpenConf, OpenReq},
    Deserializable, FloatingField, MessageType, Serializable, TagValue, MHDR,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

mod cisco;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    let mut cti_client = TcpStream::connect("172.30.1.11:42027").await?;
    let (mut rx, mut tx) = cti_client.split();

    tx.write(&mut open_req.serialize()).await?;

    let mut buffer = vec![0_u8; 4_096];
    loop {
        match rx.read(&mut buffer).await {
            Ok(n) if n == 0 => {
                break;
            }
            Ok(n) => {
                let mut index = 0_usize;
                while index < n {
                    let (_, mhdr) = MHDR::deserialize(&mut buffer[index..index + 8].to_vec());
                    match mhdr.message_type {
                        MessageType::OPEN_CONF => {
                            let (_, open_conf) = OpenConf::deserialize(
                                &mut buffer[index..(mhdr.length + 8) as usize].to_vec(),
                            );
                            println!("{:?}", open_conf);
                        }
                        message_type => {
                            println!("Received CTI message. message_type: {:?}", message_type);
                        }
                    }

                    index = index + 8 + mhdr.length as usize;
                }
            }
            Err(e) => {
                eprintln!("{:#?}", e);
                break;
            }
        }
    }

    Ok(())
}
