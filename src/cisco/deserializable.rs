pub trait Deserializable {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self);
}

impl Deserializable for bool {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let result = (buffer.as_mut()[0] | buffer.as_mut()[1]) > 0;

        (buffer.as_mut()[2..].to_vec(), result)
    }
}

impl Deserializable for u8 {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let result = buffer.as_mut()[0];

        (buffer.as_mut()[1..].to_vec(), result)
    }
}

impl Deserializable for i16 {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let mut result = 0_i16;

        for i in (0..2).rev() {
            result |= (buffer.as_mut()[1 - i] as i16) << (8 * i);
        }

        (buffer.as_mut()[2..].to_vec(), result)
    }
}

impl Deserializable for u16 {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let mut result = 0_u16;

        for i in (0..2).rev() {
            result |= (buffer.as_mut()[1 - i] as u16) << (8 * i);
        }

        (buffer.as_mut()[2..].to_vec(), result)
    }
}

impl Deserializable for i32 {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let mut result = 0_i32;

        for i in (0..4).rev() {
            result |= (buffer.as_mut()[3 - i] as i32) << (8 * i);
        }

        (buffer.as_mut()[4..].to_vec(), result)
    }
}

impl Deserializable for u32 {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let mut result = 0_u32;

        for i in (0..4).rev() {
            result |= (buffer.as_mut()[3 - i] as u32) << (8 * i);
        }
        (buffer.as_mut()[4..].to_vec(), result)
    }
}

impl Deserializable for String {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        let index = buffer.as_mut().binary_search(&0).unwrap();
        let result = String::from_utf8(buffer.as_mut()[0..index].to_vec()).unwrap();

        (buffer.as_mut()[index..].to_vec(), result)
    }
}

impl Deserializable for Vec<u8> {
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        (vec![0_u8; 0], buffer.as_mut().to_vec())
    }
}

impl<T> Deserializable for Option<T>
where
    T: Deserializable,
{
    fn deserialize<Buffer: AsMut<[u8]>>(buffer: &mut Buffer) -> (Vec<u8>, Self) {
        if buffer.as_mut().len() > 0 {
            let (buffer, result) = T::deserialize(buffer);
            (buffer, Some(result))
        } else {
            (buffer.as_mut().to_vec(), None)
        }
    }
}
