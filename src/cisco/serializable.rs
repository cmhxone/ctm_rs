pub trait Serializable {
    fn serialize(self) -> Vec<u8>;
}

impl Serializable for u8 {
    fn serialize(self) -> Vec<u8> {
        vec![self]
    }
}

impl Serializable for i16 {
    fn serialize(self) -> Vec<u8> {
        let mut result = vec![0_u8; 0];

        for i in 0..2 {
            result.append(&mut ((self >> i * 8) as u8 & 0xFF).serialize());
        }

        result.reverse();
        result
    }
}

impl Serializable for u16 {
    fn serialize(self) -> Vec<u8> {
        let mut result = vec![0_u8; 0];

        for i in 0..2 {
            result.append(&mut ((self >> i * 8) as u8 & 0xFF).serialize());
        }

        result.reverse();
        result
    }
}

impl Serializable for i32 {
    fn serialize(self) -> Vec<u8> {
        let mut result = vec![0_u8; 0];

        for i in 0..4 {
            result.append(&mut ((self >> i * 8) as u8 & 0xFF).serialize());
        }

        result.reverse();
        result
    }
}

impl Serializable for u32 {
    fn serialize(self) -> Vec<u8> {
        let mut result = vec![0_u8; 0];

        for i in 0..4 {
            result.append(&mut ((self >> i * 8) as u8 & 0xFF).serialize());
        }

        result.reverse();
        result
    }
}

impl Serializable for String {
    fn serialize(self) -> Vec<u8> {
        let mut result = vec![0_u8; 0];

        for b in self.as_bytes() {
            result.push(*b);
        }
        result.push(0);

        result
    }
}

impl<T> Serializable for Option<T>
where
    T: Serializable,
{
    fn serialize(self) -> Vec<u8> {
        match self {
            Some(value) => value.serialize(),
            None => vec![0_u8; 0],
        }
    }
}
