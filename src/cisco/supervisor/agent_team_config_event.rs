use crate::cisco::{Deserializable, FloatingField, TagValue, MHDR};

#[allow(unused)]
#[derive(Debug)]
///
/// Cisco CTI 프로토콜 AGENT_TEAM_CONFIG_EVENT
/// 
pub struct AgentTeamConfigEvent {
    pub mhdr: MHDR,
    pub peripheral_id: u32,
    pub team_id: u32,
    pub number_of_agents: u16,
    pub config_operation: u16,
    pub department_id: i32,
    pub agent_team_name: Option<FloatingField<String>>,
    pub agents: Vec<AgentTeamConfigEventAgent>,
}

#[allow(unused)]
#[derive(Debug)]
pub struct AgentTeamConfigEventAgent {
    pub agent_id: Option<FloatingField<String>>,
    pub agent_flags: Option<FloatingField<u16>>,
    pub agent_state: Option<FloatingField<u16>>,
    pub state_duration: Option<FloatingField<u32>>,
}

impl Deserializable for AgentTeamConfigEvent {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let (mut buffer, mhdr) = MHDR::deserialize(buffer);
        let (mut buffer, peripheral_id) = u32::deserialize(&mut buffer);
        let (mut buffer, team_id) = u32::deserialize(&mut buffer);
        let (mut buffer, number_of_agents) = u16::deserialize(&mut buffer);
        let (mut buffer, config_operation) = u16::deserialize(&mut buffer);
        let (mut buffer, department_id) = i32::deserialize(&mut buffer);

        let mut agent_team_name = None;
        let mut agents = vec![];
        let mut agent_index = 0;

        loop {
            let (_, floating_field) = Option::<FloatingField<Vec<u8>>>::deserialize(&mut buffer);
            match floating_field {
                Some(field) if field.length == 0 => {
                    continue;
                }
                Some(mut field) => match field.tag {
                    TagValue::AGENT_TEAM_NAME_TAG => {
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agent_team_name = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::ATC_AGENT_ID_TAG => {
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agents.push(AgentTeamConfigEventAgent {
                            agent_id: Some(FloatingField {
                                tag: field.tag,
                                length: field.length,
                                data: sub_result,
                            }),
                            agent_flags: None,
                            agent_state: None,
                            state_duration: None,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::AGENT_FLAGS_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        agents[agent_index].agent_flags = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::ATC_AGENT_STATE_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        agents[agent_index].agent_state = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::ATC_AGENT_STATE_DURATION_TAG => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        agents[agent_index].state_duration = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        agent_index = agent_index + 1;
                        buffer = sub_buffer;
                    }
                    _ => {}
                },
                None => {
                    break;
                }
            }
        }

        (
            buffer,
            Self {
                mhdr,
                peripheral_id,
                team_id,
                number_of_agents,
                config_operation,
                department_id,
                agent_team_name,
                agents,
            },
        )
    }
}
