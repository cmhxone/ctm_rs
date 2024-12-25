use crate::cisco::{Serializable, MHDR};

#[allow(unused)]
#[derive(Debug)]
pub struct HeartBeatReq {
    pub mhdr: MHDR,
    pub invoke_id: u32,
}

impl Serializable for HeartBeatReq {
    fn serialize(self) -> Vec<u8> {
        let mut result = MHDR {
            length: 4,
            message_type: crate::cisco::MessageType::HEARTBEAT_REQ,
        }
        .serialize();
        result.append(&mut self.invoke_id.serialize());

        result
    }
}
