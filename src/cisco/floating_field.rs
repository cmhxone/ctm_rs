use super::{Deserializable, Serializable, TagValue};

#[allow(unused)]
#[derive(Debug, Clone)]
///
/// Cisco CTI 프로토콜 가변 필드
///
pub struct FloatingField<T> {
    pub tag: TagValue,
    pub length: u16,
    pub data: T,
}

impl<T> Serializable for FloatingField<T>
where
    T: Serializable,
{
    fn serialize(self) -> Vec<u8> {
        let mut buffer = self.data.serialize();

        let mut result = vec![0_u8; 0];
        result.append(&mut self.tag.serialize());
        result.append(&mut (buffer.len() as u16).serialize());
        result.append(&mut buffer);

        result
    }
}

impl<T> Deserializable for FloatingField<T>
where
    T: Deserializable,
{
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let (mut buffer, tag) = TagValue::deserialize(buffer);
        let (mut buffer, length) = u16::deserialize(&mut buffer);
        let (buffer, data) = T::deserialize(&mut buffer);

        (buffer, Self { tag, length, data })
    }
}
