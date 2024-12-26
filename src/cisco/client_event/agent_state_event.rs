use crate::cisco::{Deserializable, FloatingField, TagValue, MHDR};

#[allow(unused)]
#[derive(Debug)]
///
/// Cisco CTI 프로토콜 AGENT_STATE_EVENT 메시지
///
pub struct AgentStateEvent {
    pub mhdr: MHDR,
    pub monitor_id: u32,
    pub peripheral_id: u32,
    pub session_id: u32,
    pub peripheral_type: u16,
    pub skill_group_state: u16,
    pub state_duration: u32,
    pub skill_group_number: u32,
    pub skill_group_id: u32,
    pub skill_group_priority: u16,
    pub agent_state: u16,
    pub event_reason_code: u16,
    pub mrd_id: i32,
    pub num_tasks: u32,
    pub agent_mode: u16,
    pub max_task_limit: u32,
    pub icm_agent_id: i32,
    pub agent_availability_status: u32,
    pub num_flt_skill_groups: u16,
    pub department_id: i32,
    pub cti_client_signature: Option<FloatingField<String>>,
    pub agent_id: Option<FloatingField<String>>,
    pub agent_extension: Option<FloatingField<String>>,
    pub active_terminal: Option<FloatingField<String>>,
    pub agent_instrument: Option<FloatingField<String>>,
    pub duration: Option<FloatingField<u32>>,
    pub next_agent_state: Option<FloatingField<u16>>,
    pub direction: Option<FloatingField<u32>>,
    pub flt_skill_group_number: Option<FloatingField<i32>>,
    pub flt_skill_group_id: Option<FloatingField<u32>>,
    pub flt_skill_group_priority: Option<FloatingField<u16>>,
    pub flt_skill_group_state: Option<FloatingField<u16>>,
    pub max_beyond_task_limit: Option<FloatingField<u32>>,
}

impl Deserializable for AgentStateEvent {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let (mut buffer, mhdr) = MHDR::deserialize(buffer);
        let (mut buffer, monitor_id) = u32::deserialize(&mut buffer);
        let (mut buffer, peripheral_id) = u32::deserialize(&mut buffer);
        let (mut buffer, session_id) = u32::deserialize(&mut buffer);
        let (mut buffer, peripheral_type) = u16::deserialize(&mut buffer);
        let (mut buffer, skill_group_state) = u16::deserialize(&mut buffer);
        let (mut buffer, state_duration) = u32::deserialize(&mut buffer);
        let (mut buffer, skill_group_number) = u32::deserialize(&mut buffer);
        let (mut buffer, skill_group_id) = u32::deserialize(&mut buffer);
        let (mut buffer, skill_group_priority) = u16::deserialize(&mut buffer);
        let (mut buffer, agent_state) = u16::deserialize(&mut buffer);
        let (mut buffer, event_reason_code) = u16::deserialize(&mut buffer);
        let (mut buffer, mrd_id) = i32::deserialize(&mut buffer);
        let (mut buffer, num_tasks) = u32::deserialize(&mut buffer);
        let (mut buffer, agent_mode) = u16::deserialize(&mut buffer);
        let (mut buffer, max_task_limit) = u32::deserialize(&mut buffer);
        let (mut buffer, icm_agent_id) = i32::deserialize(&mut buffer);
        let (mut buffer, agent_availability_status) = u32::deserialize(&mut buffer);
        let (mut buffer, num_flt_skill_groups) = u16::deserialize(&mut buffer);
        let (mut buffer, department_id) = i32::deserialize(&mut buffer);
        let mut cti_client_signature = None;
        let mut agent_id = None;
        let mut agent_extension = None;
        let mut active_terminal = None;
        let mut agent_instrument = None;
        let mut duration = None;
        #[allow(unused)]
        let mut next_agent_state = None;
        let mut direction = None;
        let mut flt_skill_group_number = None;
        let mut flt_skill_group_id = None;
        let mut flt_skill_group_priority = None;
        let mut flt_skill_group_state = None;
        let mut max_beyond_task_limit = None;

        loop {
            let (_, floating_field) = Option::<FloatingField<Vec<u8>>>::deserialize(&mut buffer);

            match floating_field {
                Some(field) if field.length == 0 => buffer = field.data,
                Some(mut field) => match field.tag {
                    TagValue::CTI_CLIENT_SIGNATURE_TAG => {
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        cti_client_signature = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
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
                    TagValue::ACTIVE_CONN_DEVID_TAG => {
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        active_terminal = Some(FloatingField {
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
                    TagValue::DURATION_TAG => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        duration = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::DIRECTION_TAG => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        direction = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_NUMBER_TAG => {
                        let (sub_buffer, sub_result) = i32::deserialize(&mut field.data);
                        flt_skill_group_number = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_ID_TAG => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        flt_skill_group_id = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_PRIORITY_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        flt_skill_group_priority = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::SKILL_GROUP_STATE_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        flt_skill_group_state = Some(FloatingField {
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
                    _ => {
                        buffer = field.data[field.length as usize..].to_vec();
                    }
                },
                None => break,
            };
        }

        (
            buffer,
            Self {
                mhdr,
                monitor_id,
                peripheral_id,
                session_id,
                peripheral_type,
                skill_group_state,
                state_duration,
                skill_group_number,
                skill_group_id,
                skill_group_priority,
                agent_state,
                event_reason_code,
                mrd_id,
                num_tasks,
                agent_mode,
                max_task_limit,
                icm_agent_id,
                agent_availability_status,
                num_flt_skill_groups,
                department_id,
                cti_client_signature,
                agent_id,
                agent_extension,
                active_terminal,
                agent_instrument,
                duration,
                next_agent_state,
                direction,
                flt_skill_group_number,
                flt_skill_group_id,
                flt_skill_group_priority,
                flt_skill_group_state,
                max_beyond_task_limit,
            },
        )
    }
}
