mod java_utf8;

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

const MAX_EAGER_COLLECTION_CAPACITY: usize = 4_096;

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

    pub fn write_utf16_string(&mut self, value: &[u16]) {
        if value.is_empty() {
            self.bytes.push(0);
            return;
        }
        self.bytes.reserve(5 + value.len());
        let len_pos = self.bytes.len();
        self.bytes.push(0);
        if value.iter().all(|&unit| unit < 0x80) {
            self.bytes.extend(value.iter().map(|&unit| unit as u8));
        } else {
            java_utf8::encode_units(value, |bytes| self.bytes.extend_from_slice(bytes));
        }
        let byte_len = self.bytes.len() - (len_pos + 1);
        if byte_len < 0x80 {
            self.bytes[len_pos] = byte_len as u8;
        } else {
            let mut header = [0_u8; 5];
            let header_len = encode_var_i32_slice(byte_len as i32, &mut header);
            let shift = header_len - 1;
            self.bytes.resize(self.bytes.len() + shift, 0);
            self.bytes
                .copy_within(len_pos + 1..len_pos + 1 + byte_len, len_pos + header_len);
            self.bytes[len_pos..len_pos + header_len].copy_from_slice(&header[..header_len]);
        }
    }

    pub fn write_java_string(
        &mut self,
        value: &crate::JavaString,
        max_utf16_units: usize,
    ) -> Result<()> {
        let actual = value.units().len();
        if actual > max_utf16_units {
            return Err(ProtocolError::StringTooLong {
                actual,
                max: max_utf16_units,
            });
        }
        self.write_utf16_string(value.units());
        Ok(())
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
    recursion_depth: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            cursor: 0,
            recursion_depth: 0,
        }
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

    /// Bounds eager `Vec::with_capacity` pre-allocation by both the remaining
    /// wire bytes (given each element's minimum encoded width) and a modest
    /// per-collection ceiling so short truncated packets cannot amplify heap
    /// allocations.
    pub(crate) fn bounded_capacity(&self, count: usize, min_wire_bytes: usize) -> usize {
        let wire_bound = self.remaining_len() / min_wire_bytes.max(1);
        count.min(wire_bound).min(MAX_EAGER_COLLECTION_CAPACITY)
    }

    pub(crate) fn with_recursion_limit<T>(
        &mut self,
        max_depth: usize,
        what: &'static str,
        decode: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        if self.recursion_depth >= max_depth {
            return Err(ProtocolError::InvalidData(format!(
                "{what} nesting exceeds {max_depth}"
            )));
        }
        self.recursion_depth += 1;
        let result = decode(self);
        self.recursion_depth -= 1;
        result
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

    /// `Utf8String.read` (Utf8String.java:10-31) bounds the encoded slice with
    /// Netty's `utf8MaxBytes` (3 bytes per unit), decodes it through
    /// `ByteBuf.toString(UTF_8)` -- which replaces a malformed sequence rather
    /// than refusing the packet -- and then bounds `String.length()`, which
    /// counts UTF-16 units and not code points.
    pub fn read_string(&mut self, max_utf16_units: usize) -> Result<String> {
        let len = self.read_len()?;
        let max_bytes = max_utf16_units.saturating_mul(3);
        if len > max_bytes {
            return Err(ProtocolError::StringTooLong {
                actual: len,
                max: max_bytes,
            });
        }
        let raw = self.read_exact(len, "string")?;
        if let Ok(valid) = std::str::from_utf8(raw) {
            let units = utf8_utf16_len(valid);
            if units > max_utf16_units {
                return Err(ProtocolError::StringTooLong {
                    actual: units,
                    max: max_utf16_units,
                });
            }
            return Ok(valid.to_owned());
        }
        let value = java_utf8::decode(raw);
        let units = utf8_utf16_len(&value);
        if units > max_utf16_units {
            return Err(ProtocolError::StringTooLong {
                actual: units,
                max: max_utf16_units,
            });
        }
        Ok(value)
    }

    pub fn read_java_string(&mut self, max_utf16_units: usize) -> Result<crate::JavaString> {
        let len = self.read_len()?;
        let max_bytes = max_utf16_units.saturating_mul(3);
        if len > max_bytes {
            return Err(ProtocolError::StringTooLong {
                actual: len,
                max: max_bytes,
            });
        }
        let raw = self.read_exact(len, "string")?;
        if raw.is_ascii() {
            if raw.len() > max_utf16_units {
                return Err(ProtocolError::StringTooLong {
                    actual: raw.len(),
                    max: max_utf16_units,
                });
            }
            return Ok(crate::JavaString::from_ascii_bytes(raw));
        }
        if let Ok(valid) = std::str::from_utf8(raw) {
            let units = utf8_utf16_len(valid);
            if units > max_utf16_units {
                return Err(ProtocolError::StringTooLong {
                    actual: units,
                    max: max_utf16_units,
                });
            }
            return Ok(crate::JavaString::from_str_with_utf16_len(valid, units));
        }
        let units = java_utf8::decode_units(raw);
        if units.len() > max_utf16_units {
            return Err(ProtocolError::StringTooLong {
                actual: units.len(),
                max: max_utf16_units,
            });
        }
        Ok(crate::JavaString::from_valid_utf16_vec(units))
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

#[inline]
pub(crate) fn utf8_utf16_len(valid: &str) -> usize {
    if valid.is_ascii() {
        return valid.len();
    }
    valid
        .bytes()
        .map(|byte| usize::from((byte & 0xc0) != 0x80) + usize::from(byte >= 0xf0))
        .sum()
}

fn encode_var_i32_slice(value: i32, out: &mut [u8; 5]) -> usize {
    let mut value = value as u32;
    let mut index = 0;
    loop {
        if value & !0x7f == 0 {
            out[index] = value as u8;
            return index + 1;
        }
        out[index] = ((value & 0x7f) | 0x80) as u8;
        index += 1;
        value >>= 7;
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
    let mut hash = md5::Context::new();
    hash.consume(b"OfflinePlayer:");
    hash.consume(username.as_bytes());
    name_uuid_from_digest(hash.finalize().0)
}

/// `UUIDUtil.createOfflinePlayerUUID` (`UUIDUtil.java:86-87`).
pub fn offline_player_uuid_utf16(username: &[u16]) -> Uuid {
    let mut hash = md5::Context::new();
    hash.consume(b"OfflinePlayer:");
    crate::java_string::encode_jdk_utf8(username, |bytes| hash.consume(bytes));
    name_uuid_from_digest(hash.finalize().0)
}

/// `UUID.nameUUIDFromBytes`: an MD5 name-based (version 3) UUID.
pub fn name_uuid_from_bytes(name: &[u8]) -> Uuid {
    name_uuid_from_digest(md5::compute(name).0)
}

fn name_uuid_from_digest(mut bytes: [u8; 16]) -> Uuid {
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_string_matches_netty_4_2_7_heap_and_direct_vectors() {
        let cases: &[(&[u16], &[u8])] = &[
            (&[0xd800], b"?"),
            (&[0xdc00], b"?"),
            (&[0xd800, 0x100], &[0x3f, 0]),
            (&[0xd800, 0x20ac], &[0x3f, 0xac]),
            (&[0xd800, 0xd800], b"??"),
            (&[0xd800, 0xdc00], &[0xf0, 0x90, 0x80, 0x80]),
            (&[0xd800, 65, 66], b"?AB"),
        ];
        for (units, expected) in cases {
            let mut encoder = Encoder::new();
            encoder.write_utf16_string(units);
            let mut framed = vec![expected.len() as u8];
            framed.extend_from_slice(expected);
            assert_eq!(encoder.into_inner(), framed, "{units:x?}");
        }
        for text in [
            String::new(),
            "ASCII".into(),
            "é水😀".into(),
            "a".repeat(127),
            "a".repeat(128),
            "a".repeat(16_383),
            "é".repeat(8_192),
        ] {
            let mut raw = Encoder::new();
            raw.write_u8(0x55);
            raw.write_utf16_string(&text.encode_utf16().collect::<Vec<_>>());
            raw.write_utf16_string(&[0xd800]);
            let mut string = Encoder::new();
            string.write_u8(0x55);
            string.write_string(&text);
            string.write_string("?");
            assert_eq!(raw.into_inner(), string.into_inner());
        }
    }

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
        // OpenJDK 25.0.3 UUID.nameUUIDFromBytes over JDK UTF-8 bytes.
        let cases: &[(&[u16], &str)] = &[
            (&[0xd800], "0386ba32-ea5c-3926-abb9-89e2b71b0e13"),
            (&[0xd801], "0386ba32-ea5c-3926-abb9-89e2b71b0e13"),
            (&[0xfffd], "7f21183c-0ff2-3465-9df0-9ca82956deb0"),
            (
                &[0xd800, 0xd800, 0xdc00],
                "3d0c91aa-7dac-34f6-8e8e-60073dbf6682",
            ),
            (&[0xd800, 65], "72b6a10a-1eec-3d6c-a905-017f15285353"),
            (&[0xd83d, 0xde00], "869a1aa4-b58d-34ab-bcea-41045449597a"),
            (
                &[65, 83, 67, 73, 73],
                "d9a0a6aa-9e83-3f40-9a2e-a6b7ae5cda76",
            ),
            (&[0xdc00], "0386ba32-ea5c-3926-abb9-89e2b71b0e13"),
            (&[0xd800, 0x100], "be9794e8-1f5a-322b-aacb-af5da82d9d37"),
            (&[0xd800, 0x20ac], "25249566-ba5a-3c5d-a8d5-cd7c767cf24b"),
            (&[], "fc5bc365-aedf-30a8-8b89-04e462e29bde"),
        ];
        for (units, expected) in cases {
            let expected = Uuid::parse_str(expected).unwrap();
            assert_eq!(offline_player_uuid_utf16(units), expected);
            if let Ok(username) = String::from_utf16(units) {
                assert_eq!(offline_player_uuid(&username), expected);
            }
        }
    }

    #[test]
    fn recursion_limit_restores_depth_after_errors() {
        let mut decoder = Decoder::new(&[]);
        let error = decoder
            .with_recursion_limit(1, "test value", |decoder| {
                decoder.with_recursion_limit(1, "test value", |_| Ok(()))
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "test value nesting exceeds 1");

        decoder
            .with_recursion_limit(1, "test value", |_| Ok(()))
            .unwrap();
    }

    #[test]
    fn collection_initial_capacity_matches_wire_and_eager_bounds() {
        let decoder = Decoder::new(&[0u8; 8]);
        assert_eq!(decoder.bounded_capacity(0, 1), 0);
        assert_eq!(decoder.bounded_capacity(65_536, 4), 2);
        assert_eq!(decoder.bounded_capacity(65_536, 1), 8);

        let large = vec![0u8; MAX_EAGER_COLLECTION_CAPACITY * 2];
        let decoder = Decoder::new(&large);
        assert_eq!(
            decoder.bounded_capacity(usize::MAX, 1),
            MAX_EAGER_COLLECTION_CAPACITY
        );
    }
}
