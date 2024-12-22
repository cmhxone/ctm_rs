use crate::cisco::{FloatingField, MessageType, Serializable, MHDR};

#[allow(unused)]
pub struct OpenReq {
    pub mhdr: MHDR,
    pub invoke_id: u32,
    pub version_number: u32,
    pub idle_timeout: u32,
    pub peripheral_id: u32,
    pub services_requested: u32,
    pub call_msg_mask: u32,
    pub agent_state_mask: u32,
    pub config_msg_mask: u32,
    pub reserved1: u32,
    pub reserved2: u32,
    pub reserved3: u32,
    pub client_id: Option<FloatingField<String>>,
    pub client_password: Option<FloatingField<String>>,
    pub client_signature: Option<FloatingField<String>>,
    pub agent_extension: Option<FloatingField<String>>,
    pub agent_id: Option<FloatingField<String>>,
    pub agent_instrument: Option<FloatingField<String>>,
    pub application_path_id: Option<FloatingField<i32>>,
    pub unique_instance_id: Option<FloatingField<i32>>,
}

impl Serializable for OpenReq {
    fn serialize(self) -> Vec<u8> {
        let mut buffer = self.invoke_id.serialize();
        buffer.append(&mut self.version_number.serialize());
        buffer.append(&mut self.idle_timeout.serialize());
        buffer.append(&mut self.peripheral_id.serialize());
        buffer.append(&mut self.services_requested.serialize());
        buffer.append(&mut self.call_msg_mask.serialize());
        buffer.append(&mut self.agent_state_mask.serialize());
        buffer.append(&mut self.config_msg_mask.serialize());
        buffer.append(&mut self.reserved1.serialize());
        buffer.append(&mut self.reserved2.serialize());
        buffer.append(&mut self.reserved3.serialize());
        buffer.append(&mut self.client_id.serialize());
        buffer.append(&mut self.client_password.serialize());
        buffer.append(&mut self.client_signature.serialize());
        buffer.append(&mut self.agent_extension.serialize());
        buffer.append(&mut self.agent_id.serialize());
        buffer.append(&mut self.agent_instrument.serialize());
        buffer.append(&mut self.application_path_id.serialize());
        buffer.append(&mut self.unique_instance_id.serialize());

        let mhdr = MHDR {
            length: buffer.len() as u32,
            message_type: MessageType::OPEN_REQ,
        };

        let mut result = mhdr.serialize();
        result.append(&mut buffer);

        result
    }
}
