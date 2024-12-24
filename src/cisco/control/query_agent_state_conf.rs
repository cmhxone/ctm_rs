use crate::cisco::{Deserializable, FloatingField, TagValue, MHDR};

#[allow(unused)]
#[derive(Debug)]
///
/// Cisco CTI 프로토콜 QUERY_AGENT_STATE_CONF 메시지
///
pub struct QueryAgentStateConf {
    pub mhdr: MHDR,
    pub invoke_id: u32,
    pub agent_state: u16,
    pub num_skill_groups: u16,
    pub mrd_id: i32,
    pub num_task: u32,
    pub agent_mode: u16,
    pub max_task_limit: u32,
    pub icm_agent_id: i32,
    pub agent_availability_status: u32,
    pub department_id: i32,
    pub agent_id: Option<FloatingField<String>>,
    pub agent_extension: Option<FloatingField<String>>,
    pub agent_instrument: Option<FloatingField<String>>,
    pub skill_group_number: Option<FloatingField<u32>>,
    pub skill_group_id: Option<FloatingField<u32>>,
    pub skill_group_priority: Option<FloatingField<u16>>,
    pub skill_group_state: Option<FloatingField<u16>>,
    pub internal_agent_state: Option<FloatingField<u16>>,
    pub max_beyond_task_limit: Option<FloatingField<u32>>,
}

impl Deserializable for QueryAgentStateConf {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let (mut buffer, mhdr) = MHDR::deserialize(buffer);
        let (mut buffer, invoke_id) = u32::deserialize(&mut buffer);
        let (mut buffer, agent_state) = u16::deserialize(&mut buffer);
        let (mut buffer, num_skill_groups) = u16::deserialize(&mut buffer);
        let (mut buffer, mrd_id) = i32::deserialize(&mut buffer);
        let (mut buffer, num_task) = u32::deserialize(&mut buffer);
        let (mut buffer, agent_mode) = u16::deserialize(&mut buffer);
        let (mut buffer, max_task_limit) = u32::deserialize(&mut buffer);
        let (mut buffer, icm_agent_id) = i32::deserialize(&mut buffer);
        let (mut buffer, agent_availability_status) = u32::deserialize(&mut buffer);
        let (mut buffer, department_id) = i32::deserialize(&mut buffer);
        let mut agent_id = None;
        let mut agent_extension = None;
        let mut agent_instrument = None;
        let mut skill_group_number = None;
        let mut skill_group_id = None;
        let mut skill_group_priority = None;
        let mut skill_group_state = None;
        let mut internal_agent_state = None;
        let mut max_beyond_task_limit = None;

        loop {
            let (_, floating_field) = Option::<FloatingField<Vec<u8>>>::deserialize(&mut buffer);

            match floating_field {
                Some(field) if field.length == 0 => {
                    buffer = field.data;
                    continue;
                }
                Some(mut field) => match field.tag {
                    TagValue::AGENT_ID_TAG => {
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agent_id = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::AGENT_EXTENSION_TAG => {
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agent_extension = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::AGENT_INSTRUMENT_TAG => {
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agent_instrument = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_NUMBER_TAG => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        skill_group_number = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_ID_TAG => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        skill_group_id = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_PRIORITY_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        skill_group_priority = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_STATE_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        skill_group_state = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::INTERNAL_AGENT_STATE_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        internal_agent_state = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::MAX_BEYOND_TASK_LIMIT_TAG => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        max_beyond_task_limit = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    _ => {}
                },
                None => {
                    break;
                }
            };
        }

        (
            buffer,
            Self {
                mhdr,
                invoke_id,
                agent_state,
                num_skill_groups,
                mrd_id,
                num_task,
                agent_mode,
                max_task_limit,
                icm_agent_id,
                agent_availability_status,
                department_id,
                agent_id,
                agent_extension,
                agent_instrument,
                skill_group_number,
                skill_group_id,
                skill_group_priority,
                skill_group_state,
                internal_agent_state,
                max_beyond_task_limit,
            },
        )
    }
}
