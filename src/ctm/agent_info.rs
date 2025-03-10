use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    icm_agent_id: i32,
    agent_id: String,
    agent_state: u16,
    state_duration: u64,
    reason_code: u16,
    skill_group_id: u16,
    direction: u32,
    agent_extension: String,
}

impl AgentInfo {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            icm_agent_id: 0,
            agent_id: agent_id.into(),
            agent_state: 0,
            state_duration: 0,
            reason_code: 0,
            skill_group_id: 0,
            direction: 0,
            agent_extension: "".to_string(),
        }
    }

    pub fn set_icm_agent_id(&mut self, icm_agent_id: i32) {
        self.icm_agent_id = icm_agent_id;
    }

    pub fn set_agent_state(&mut self, agent_state: u16) {
        self.agent_state = agent_state;
    }

    pub fn set_state_duration(&mut self, state_duration: u32) {
        self.state_duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - state_duration as u64;
    }

    pub fn set_reason_code(&mut self, reason_code: u16) {
        match self.agent_state {
            1 | 2 => {
                // LOGOUT, NOT_READY 일때만 할당
                self.reason_code = reason_code;
            }
            _ => {
                self.reason_code = 0;
            }
        }
    }

    pub fn set_skill_group_id(&mut self, skill_group_id: u16) {
        // 통화, 보류 상태일때만 할당
        match self.agent_state {
                4 | 10 => {
                    self.skill_group_id = skill_group_id;
                }
            _ => {
                self.skill_group_id = 0;
            }
        }
    }

    pub fn set_direction(&mut self, direction: u32) {
        match self.agent_state {
            4 | 7 | 8 | 10 => {
                // 통화, 예약, 보류 상태일때만 할당
                self.direction = direction;
            }
            _ => {
                self.direction = 0;
            }
        }
    }

    pub fn set_agent_extension(&mut self, agent_extension: impl Into<String>) {
        match self.agent_state {
            1 | 9 => {
                // 로그아웃, 알수없음 상태일때는 할당받지 않는다
                self.agent_extension = "".to_string();
            }
            _ => {
                self.agent_extension = agent_extension.into();
            }
        }
    }
}
