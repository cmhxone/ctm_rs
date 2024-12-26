use uuid::Uuid;

use crate::ctm::agent_info::AgentInfo;

///
/// 서버-클라이언트 브로커 이벤트
///
#[derive(Debug, Clone)]
pub enum BrokerEvent {
    BroadCastAgentState {
        client_id: Option<Uuid>,
        agent_info: AgentInfo,
    },
    RequestAgentStateEvent {
        peripheral_id: u32,
        agent_id: String,
    },
    RequestHeartBeatReq,
}
