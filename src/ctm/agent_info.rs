use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub icm_agent_id: i32,
    pub agent_id: String,
    pub agent_state: u16,
    pub state_duration: u64,
    pub reason_code: u16,
    pub skill_group_id: u16,
    pub direction: u32,
    pub agent_extension: String,
}
