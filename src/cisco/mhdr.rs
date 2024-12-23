use super::{Deserializable, MessageType, Serializable};

#[derive(Debug)]
///
/// Cisco CTI 프로토콜 메시지 헤더
///
pub struct MHDR {
    pub length: u32,
    pub message_type: MessageType,
}

impl Serializable for MHDR {
    fn serialize(self) -> Vec<u8> {
        let mut result = self.length.serialize();
        result.append(&mut self.message_type.serialize());

        result
    }
}

impl Deserializable for MHDR {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let (mut buffer, length) = u32::deserialize(buffer);
        let (buffer, message_type) = u32::deserialize(&mut buffer);

        (
            buffer,
            Self {
                length,
                message_type: message_type.into(),
            },
        )
    }
}
