use crate::cisco::MessageType;

///
/// CTI 서버 이벤트
///
#[derive(Debug, Clone)]
pub enum CTIEvent {
    Error {
        cti_server_host: String,
        error_cause: String,
    },
    Recevied {
        cti_server_host: String,
        message_type: MessageType,
        data: Vec<u8>,
    },
}
