use crate::cisco::{FloatingField, Serializable, MHDR};

#[allow(unused)]
#[derive(Debug)]
///
/// Cisco CTI 프로토콜 QUERY_AGENT_STATE_REQ 메시지
///
pub struct QueryAgentStateReq {
    pub mhdr: MHDR,
    pub invoke_id: u32,
    pub peripheral_id: u32,
    pub mrd_id: i32,
    pub icm_agent_id: i32,
    pub agent_extension: Option<FloatingField<String>>,
    pub agent_id: Option<FloatingField<String>>,
    pub agent_instrument: Option<FloatingField<String>>,
}

impl Serializable for QueryAgentStateReq {
    fn serialize(self) -> Vec<u8> {
        let mut buffer = vec![0_u8; 0];
        buffer.append(&mut self.invoke_id.serialize());
        buffer.append(&mut self.peripheral_id.serialize());
        buffer.append(&mut self.mrd_id.serialize());
        buffer.append(&mut self.icm_agent_id.serialize());
        buffer.append(&mut self.agent_extension.serialize());
        buffer.append(&mut self.agent_id.serialize());
        buffer.append(&mut self.agent_instrument.serialize());

        let mhdr = MHDR {
            length: buffer.len() as u32,
            message_type: crate::cisco::MessageType::QUERY_AGENT_STATE_REQ,
        };

        let mut result = mhdr.serialize();
        result.append(&mut buffer);

        result
    }
}
