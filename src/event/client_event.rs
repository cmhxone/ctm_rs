use uuid::Uuid;

#[allow(unused)]
#[derive(Debug, Clone)]
///
/// 클라이언트 이벤트
///
pub enum ClientEvent {
    Connect { id: Uuid },
    Receive { id: Uuid, data: Vec<u8> },
    Disconnect { id: Uuid },
}
