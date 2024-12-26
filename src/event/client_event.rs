#[derive(Debug, Clone)]
///
/// 클라이언트 이벤트
///
pub enum ClientEvent {
    Connect {},
    Receive { data: Vec<u8> },
    Disconnect {},
}
