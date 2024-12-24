///
/// 서버-클라이언트 브로커 이벤트
///
#[derive(Debug, Clone)]
pub enum BrokerEvent {
    BroadCastAgentState {},
    RequestAgentStateEvent { peripheral_id: u32, agent_id: String },
}
