use crate::cisco::{Deserializable, FloatingField, TagValue, MHDR};

#[allow(unused)]
#[derive(Debug)]
pub struct OpenConf {
    mhdr: MHDR,
    invoke_id: u32,
    service_granted: u32,
    monitor_id: u32,
    pg_status: u32,
    icm_central_controller_time: u32,
    peripheral_online: bool,
    peripheral_type: u16,
    agent_state: u16,
    department_id: i32,
    session_type: u16,
    agent_extension: Option<FloatingField<String>>,
    agent_id: Option<FloatingField<String>>,
    agent_instrument: Option<FloatingField<String>>,
    num_peripherals: Option<FloatingField<u16>>,
    flt_peripheral_id: Option<FloatingField<u32>>,
    multiline_agent_control: Option<FloatingField<u16>>,
}

impl Deserializable for OpenConf {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let (mut buffer, mhdr) = MHDR::deserialize(buffer);
        let (mut buffer, invoke_id) = u32::deserialize(&mut buffer);
        let (mut buffer, service_granted) = u32::deserialize(&mut buffer);
        let (mut buffer, monitor_id) = u32::deserialize(&mut buffer);
        let (mut buffer, pg_status) = u32::deserialize(&mut buffer);
        let (mut buffer, icm_central_controller_time) = u32::deserialize(&mut buffer);
        let (mut buffer, peripheral_online) = bool::deserialize(&mut buffer);
        let (mut buffer, peripheral_type) = u16::deserialize(&mut buffer);
        let (mut buffer, agent_state) = u16::deserialize(&mut buffer);
        let (mut buffer, department_id) = i32::deserialize(&mut buffer);
        let (mut buffer, session_type) = u16::deserialize(&mut buffer);

        let mut agent_extension: Option<FloatingField<String>> = None;
        let mut agent_id: Option<FloatingField<String>> = None;
        let mut agent_instrument: Option<FloatingField<String>> = None;
        let mut num_peripherals: Option<FloatingField<u16>> = None;
        let mut flt_peripheral_id: Option<FloatingField<u32>> = None;
        let mut multiline_agent_control: Option<FloatingField<u16>> = None;

        loop {
            let (_, floating_field) = Option::<FloatingField<Vec<u8>>>::deserialize(&mut buffer);
            match floating_field {
                Some(mut field) => match field.tag {
                    TagValue::AGENT_EXTENSION_TAG => {
                        if field.length == 0 {
                            continue;
                        }
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agent_extension = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::AGENT_ID_TAG => {
                        if field.length == 0 {
                            continue;
                        }
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agent_id = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::AGENT_INSTRUMENT_TAG => {
                        if field.length == 0 {
                            continue;
                        }
                        let (sub_buffer, sub_result) = String::deserialize(&mut field.data);
                        agent_instrument = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::NUM_PERIPHERALS_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        num_peripherals = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::PERIPHERAL_ID_TAG_V11 => {
                        let (sub_buffer, sub_result) = u32::deserialize(&mut field.data);
                        flt_peripheral_id = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    TagValue::MULTI_LINE_AGENT_CONTROL_TAG => {
                        let (sub_buffer, sub_result) = u16::deserialize(&mut field.data);
                        multiline_agent_control = Some(FloatingField {
                            tag: field.tag,
                            length: field.length,
                            data: sub_result,
                        });
                        buffer = sub_buffer;
                    }
                    _ => {}
                },
                None => break,
            }
        }

        (
            buffer,
            Self {
                mhdr,
                invoke_id,
                service_granted,
                monitor_id,
                pg_status,
                icm_central_controller_time,
                peripheral_online,
                peripheral_type,
                agent_state,
                department_id,
                session_type,
                agent_extension,
                agent_id,
                agent_instrument,
                num_peripherals,
                flt_peripheral_id,
                multiline_agent_control,
            },
        )
    }
}
