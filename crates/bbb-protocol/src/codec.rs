use std::fmt;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("buffer ended while reading {what}")]
    UnexpectedEof { what: &'static str },
    #[error("varint is too long")]
    VarIntTooLong,
    #[error("varlong is too long")]
    VarLongTooLong,
    #[error("negative length {0}")]
    NegativeLength(i32),
    #[error("string has {actual} bytes, max is {max}")]
    StringTooLong { actual: usize, max: usize },
    #[error("invalid utf-8 string")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("packet length {0} exceeds configured maximum {1}")]
    PacketTooLarge(usize, usize),
    #[error("compressed packet declares {declared} bytes but decoded to {actual}")]
    BadCompressionLength { declared: usize, actual: usize },
    #[error("zlib error: {0}")]
    Zlib(std::io::Error),
    #[error("unknown packet id {id} in {state}")]
    UnknownPacket { state: &'static str, id: i32 },
    #[error("{0}")]
    InvalidData(String),
}

pub type Result<T> = std::result::Result<T, ProtocolError>;

#[derive(Clone, Default)]
pub struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.bytes.push(if value { 1 } else { 0 });
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.bytes.push(value as u8);
    }

    pub fn write_i16(&mut self, value: i16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_f64(&mut self, value: f64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_var_i32(&mut self, value: i32) {
        write_var_i32_to(&mut self.bytes, value);
    }

    pub fn write_var_i64(&mut self, value: i64) {
        write_var_i64_to(&mut self.bytes, value);
    }

    pub fn write_string(&mut self, value: &str) {
        self.write_var_i32(value.len() as i32);
        self.write_bytes(value.as_bytes());
    }

    pub fn write_uuid(&mut self, value: Uuid) {
        self.write_bytes(value.as_bytes());
    }
}

impl fmt::Debug for Encoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Encoder")
            .field("len", &self.bytes.len())
            .finish()
    }
}

pub struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    pub fn position(&self) -> usize {
        self.cursor
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.cursor..]
    }

    pub fn bytes_from(&self, start: usize) -> &'a [u8] {
        &self.bytes[start.min(self.cursor)..self.cursor]
    }

    pub fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining_len() == 0
    }

    pub fn read_exact(&mut self, len: usize, what: &'static str) -> Result<&'a [u8]> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or_else(|| ProtocolError::InvalidData("read overflow".to_string()))?;
        if end > self.bytes.len() {
            return Err(ProtocolError::UnexpectedEof { what });
        }
        let out = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(out)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_exact(1, "u8")?[0])
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        let bytes = self.read_exact(2, "i16")?;
        Ok(i16::from_be_bytes(bytes.try_into().expect("fixed length")))
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_exact(2, "u16")?;
        Ok(u16::from_be_bytes(bytes.try_into().expect("fixed length")))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        let bytes = self.read_exact(4, "i32")?;
        Ok(i32::from_be_bytes(bytes.try_into().expect("fixed length")))
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        let bytes = self.read_exact(8, "i64")?;
        Ok(i64::from_be_bytes(bytes.try_into().expect("fixed length")))
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        let bytes = self.read_exact(4, "f32")?;
        Ok(f32::from_be_bytes(bytes.try_into().expect("fixed length")))
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        let bytes = self.read_exact(8, "f64")?;
        Ok(f64::from_be_bytes(bytes.try_into().expect("fixed length")))
    }

    pub fn read_var_i32(&mut self) -> Result<i32> {
        let mut value = 0u32;
        for position in 0..5 {
            let byte = self.read_u8()?;
            value |= ((byte & 0x7f) as u32) << (7 * position);
            if byte & 0x80 == 0 {
                return Ok(value as i32);
            }
        }
        Err(ProtocolError::VarIntTooLong)
    }

    pub fn read_var_i64(&mut self) -> Result<i64> {
        let mut value = 0u64;
        for position in 0..10 {
            let byte = self.read_u8()?;
            value |= ((byte & 0x7f) as u64) << (7 * position);
            if byte & 0x80 == 0 {
                return Ok(value as i64);
            }
        }
        Err(ProtocolError::VarLongTooLong)
    }

    pub fn read_string(&mut self, max_chars: usize) -> Result<String> {
        let len = self.read_len()?;
        let max_bytes = max_chars.saturating_mul(4);
        if len > max_bytes {
            return Err(ProtocolError::StringTooLong {
                actual: len,
                max: max_bytes,
            });
        }
        let bytes = self.read_exact(len, "string")?;
        let s = std::str::from_utf8(bytes)?;
        if s.chars().count() > max_chars {
            return Err(ProtocolError::StringTooLong {
                actual: s.chars().count(),
                max: max_chars,
            });
        }
        Ok(s.to_string())
    }

    pub fn read_uuid(&mut self) -> Result<Uuid> {
        let bytes = self.read_exact(16, "uuid")?;
        Ok(Uuid::from_bytes(bytes.try_into().expect("fixed length")))
    }

    pub fn read_len(&mut self) -> Result<usize> {
        let len = self.read_var_i32()?;
        if len < 0 {
            return Err(ProtocolError::NegativeLength(len));
        }
        Ok(len as usize)
    }
}

pub fn write_var_i32_to(out: &mut Vec<u8>, value: i32) {
    let mut value = value as u32;
    loop {
        if value & !0x7f == 0 {
            out.push(value as u8);
            return;
        }
        out.push(((value & 0x7f) | 0x80) as u8);
        value >>= 7;
    }
}

pub fn write_var_i64_to(out: &mut Vec<u8>, value: i64) {
    let mut value = value as u64;
    loop {
        if value & !0x7f == 0 {
            out.push(value as u8);
            return;
        }
        out.push(((value & 0x7f) | 0x80) as u8);
        value >>= 7;
    }
}

pub fn offline_player_uuid(username: &str) -> Uuid {
    let digest = md5::compute(format!("OfflinePlayer:{username}"));
    let mut bytes = digest.0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip() {
        for value in [0, 1, 127, 128, 255, 2_097_151, i32::MAX, -1, i32::MIN] {
            let mut e = Encoder::new();
            e.write_var_i32(value);
            let bytes = e.into_inner();
            let mut d = Decoder::new(&bytes);
            assert_eq!(d.read_var_i32().unwrap(), value);
            assert!(d.is_empty());
        }
    }

    #[test]
    fn varlong_roundtrip() {
        for value in [0, 1, 127, 128, 255, i64::MAX, -1, i64::MIN] {
            let mut e = Encoder::new();
            e.write_var_i64(value);
            let bytes = e.into_inner();
            let mut d = Decoder::new(&bytes);
            assert_eq!(d.read_var_i64().unwrap(), value);
            assert!(d.is_empty());
        }
    }

    #[test]
    fn offline_uuid_matches_java_name_uuid_shape() {
        let uuid = offline_player_uuid("bbb-client");
        assert_eq!(uuid.get_version_num(), 3);
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
    }
}
