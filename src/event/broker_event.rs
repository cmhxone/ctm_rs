///
/// 서버-클라이언트 브로커 이벤트
///
#[derive(Debug, Clone)]
pub enum BrokerEvent {
    ToClientEvent {},
    ToServerEvent {},
}
