use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

mod snbt;

pub use snbt::parse_snbt_compound;

const MAX_NBT_DEPTH: usize = 64;
const MAX_NBT_ARRAY_ITEMS: usize = 4096;
pub(crate) const MAX_NBT_COMPOUND_ENTRIES: usize = 4096;
const MAX_NBT_LIST_ITEMS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum NbtSummaryValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtSummaryValue>),
    Compound(Vec<NbtSummaryEntry>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NbtSummaryEntry {
    pub name: String,
    pub value: NbtSummaryValue,
}

pub(crate) fn decode_nbt_summary_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Option<NbtSummaryValue>> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(None);
    }
    read_nbt_payload(decoder, tag_id, 0).map(Some)
}

pub fn decode_nbt_summary(bytes: &[u8]) -> Result<Option<NbtSummaryValue>> {
    let mut decoder = Decoder::new(bytes);
    let value = decode_nbt_summary_from_decoder(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after network nbt root".to_string(),
        ));
    }
    Ok(value)
}

fn read_nbt_payload(
    decoder: &mut Decoder<'_>,
    tag_id: u8,
    depth: usize,
) -> Result<NbtSummaryValue> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "network nbt exceeded max depth".to_string(),
        ));
    }

    match tag_id {
        1 => Ok(NbtSummaryValue::Byte(decoder.read_i8()?)),
        2 => Ok(NbtSummaryValue::Short(decoder.read_i16()?)),
        3 => Ok(NbtSummaryValue::Int(decoder.read_i32()?)),
        4 => Ok(NbtSummaryValue::Long(decoder.read_i64()?)),
        5 => Ok(NbtSummaryValue::Float(decoder.read_f32()?.to_bits())),
        6 => Ok(NbtSummaryValue::Double(decoder.read_f64()?.to_bits())),
        7 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_ARRAY_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_ARRAY_ITEMS));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i8()?);
            }
            Ok(NbtSummaryValue::ByteArray(values))
        }
        8 => Ok(NbtSummaryValue::String(read_modified_utf8(decoder)?)),
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_LIST_ITEMS));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty nbt list has end tag element type".to_string(),
                ));
            }
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(read_nbt_payload(decoder, element_type, depth + 1)?);
            }
            Ok(NbtSummaryValue::List(items))
        }
        10 => {
            let mut entries = Vec::new();
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    break;
                }
                if entries.len() >= MAX_NBT_COMPOUND_ENTRIES {
                    return Err(ProtocolError::PacketTooLarge(
                        entries.len() + 1,
                        MAX_NBT_COMPOUND_ENTRIES,
                    ));
                }
                let name = read_modified_utf8(decoder)?;
                let value = read_nbt_payload(decoder, nested_type, depth + 1)?;
                entries.push(NbtSummaryEntry { name, value });
            }
            Ok(NbtSummaryValue::Compound(entries))
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_ARRAY_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_ARRAY_ITEMS));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i32()?);
            }
            Ok(NbtSummaryValue::IntArray(values))
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_ARRAY_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_ARRAY_ITEMS));
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i64()?);
            }
            Ok(NbtSummaryValue::LongArray(values))
        }
        other => Err(ProtocolError::InvalidData(format!(
            "invalid network nbt tag id {other}"
        ))),
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(ProtocolError::NegativeLength(len));
    }
    Ok(len as usize)
}

fn read_modified_utf8(decoder: &mut Decoder<'_>) -> Result<String> {
    let len = decoder.read_u16()? as usize;
    let bytes = decoder.read_exact(len, "nbt string")?;
    let mut units = Vec::with_capacity(len);
    let mut cursor = 0;

    while cursor < bytes.len() {
        let b0 = bytes[cursor];
        if b0 & 0x80 == 0 {
            units.push(u16::from(b0));
            cursor += 1;
        } else if b0 & 0xe0 == 0xc0 {
            if cursor + 1 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            let b1 = bytes[cursor + 1];
            if b1 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            units.push((u16::from(b0 & 0x1f) << 6) | u16::from(b1 & 0x3f));
            cursor += 2;
        } else if b0 & 0xf0 == 0xe0 {
            if cursor + 2 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            let b1 = bytes[cursor + 1];
            let b2 = bytes[cursor + 2];
            if b1 & 0xc0 != 0x80 || b2 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            units.push(
                (u16::from(b0 & 0x0f) << 12) | (u16::from(b1 & 0x3f) << 6) | u16::from(b2 & 0x3f),
            );
            cursor += 3;
        } else {
            return Err(ProtocolError::InvalidData(
                "invalid modified utf-8 leading byte".to_string(),
            ));
        }
    }

    String::from_utf16(&units)
        .map_err(|_| ProtocolError::InvalidData("invalid modified utf-8 string".to_string()))
}

pub(crate) fn encode_nbt_summary_bounded(
    value: Option<&NbtSummaryValue>,
    max_len: usize,
    max_depth: usize,
) -> Result<Vec<u8>> {
    let mut encoder = BoundedNbtEncoder::new(max_len);
    match value {
        Some(value) => {
            encoder.write_u8(nbt_tag_id(value))?;
            encode_nbt_payload(&mut encoder, value, 0, max_depth)?;
        }
        None => encoder.write_u8(0)?,
    }
    Ok(encoder.into_inner())
}

fn encode_nbt_payload(
    encoder: &mut BoundedNbtEncoder,
    value: &NbtSummaryValue,
    depth: usize,
    max_depth: usize,
) -> Result<()> {
    match value {
        NbtSummaryValue::Byte(value) => encoder.write_u8(*value as u8),
        NbtSummaryValue::Short(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::Int(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::Long(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::Float(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::Double(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::ByteArray(values) => {
            encode_nbt_collection_len(encoder, values.len())?;
            for value in values {
                encoder.write_u8(*value as u8)?;
            }
            Ok(())
        }
        NbtSummaryValue::String(value) => encode_modified_utf8(encoder, value),
        NbtSummaryValue::List(values) => {
            let nested_depth = enter_nbt_container(depth, max_depth)?;
            let element_type = values.first().map_or(0, nbt_tag_id);
            if let Some((index, value)) = values
                .iter()
                .enumerate()
                .find(|(_, value)| nbt_tag_id(value) != element_type)
            {
                return Err(ProtocolError::InvalidData(format!(
                    "network nbt list element {index} has tag id {}, expected {element_type}",
                    nbt_tag_id(value)
                )));
            }
            encoder.write_u8(element_type)?;
            encode_nbt_collection_len(encoder, values.len())?;
            for value in values {
                encode_nbt_payload(encoder, value, nested_depth, max_depth)?;
            }
            Ok(())
        }
        NbtSummaryValue::Compound(entries) => {
            let nested_depth = enter_nbt_container(depth, max_depth)?;
            if entries.len() > MAX_NBT_COMPOUND_ENTRIES {
                return Err(ProtocolError::PacketTooLarge(
                    entries.len(),
                    MAX_NBT_COMPOUND_ENTRIES,
                ));
            }
            for entry in entries {
                encoder.write_u8(nbt_tag_id(&entry.value))?;
                encode_modified_utf8(encoder, &entry.name)?;
                encode_nbt_payload(encoder, &entry.value, nested_depth, max_depth)?;
            }
            encoder.write_u8(0)
        }
        NbtSummaryValue::IntArray(values) => {
            encode_nbt_collection_len(encoder, values.len())?;
            for value in values {
                encoder.write_bytes(&value.to_be_bytes())?;
            }
            Ok(())
        }
        NbtSummaryValue::LongArray(values) => {
            encode_nbt_collection_len(encoder, values.len())?;
            for value in values {
                encoder.write_bytes(&value.to_be_bytes())?;
            }
            Ok(())
        }
    }
}

fn nbt_tag_id(value: &NbtSummaryValue) -> u8 {
    match value {
        NbtSummaryValue::Byte(_) => 1,
        NbtSummaryValue::Short(_) => 2,
        NbtSummaryValue::Int(_) => 3,
        NbtSummaryValue::Long(_) => 4,
        NbtSummaryValue::Float(_) => 5,
        NbtSummaryValue::Double(_) => 6,
        NbtSummaryValue::ByteArray(_) => 7,
        NbtSummaryValue::String(_) => 8,
        NbtSummaryValue::List(_) => 9,
        NbtSummaryValue::Compound(_) => 10,
        NbtSummaryValue::IntArray(_) => 11,
        NbtSummaryValue::LongArray(_) => 12,
    }
}

fn enter_nbt_container(depth: usize, max_depth: usize) -> Result<usize> {
    if depth >= max_depth {
        return Err(ProtocolError::InvalidData(format!(
            "network nbt exceeded max depth {max_depth}"
        )));
    }
    Ok(depth + 1)
}

fn encode_nbt_collection_len(encoder: &mut BoundedNbtEncoder, len: usize) -> Result<()> {
    let len = i32::try_from(len).map_err(|_| {
        ProtocolError::InvalidData("network nbt collection length exceeds i32".to_string())
    })?;
    encoder.write_bytes(&len.to_be_bytes())
}

fn encode_modified_utf8(encoder: &mut BoundedNbtEncoder, value: &str) -> Result<()> {
    let encoded_len = value.encode_utf16().try_fold(0usize, |len, unit| {
        len.checked_add(if unit == 0 {
            2
        } else if unit <= 0x7f {
            1
        } else if unit <= 0x7ff {
            2
        } else {
            3
        })
    });
    let encoded_len = encoded_len
        .ok_or_else(|| ProtocolError::InvalidData("modified utf-8 length overflow".to_string()))?;
    let encoded_len_u16 = u16::try_from(encoded_len).map_err(|_| ProtocolError::StringTooLong {
        actual: encoded_len,
        max: usize::from(u16::MAX),
    })?;
    encoder.write_bytes(&encoded_len_u16.to_be_bytes())?;
    for unit in value.encode_utf16() {
        if unit == 0 {
            encoder.write_bytes(&[0xc0, 0x80])?;
        } else if unit <= 0x7f {
            encoder.write_u8(unit as u8)?;
        } else if unit <= 0x7ff {
            encoder.write_bytes(&[
                (0xc0 | ((unit >> 6) & 0x1f)) as u8,
                (0x80 | (unit & 0x3f)) as u8,
            ])?;
        } else {
            encoder.write_bytes(&[
                (0xe0 | ((unit >> 12) & 0x0f)) as u8,
                (0x80 | ((unit >> 6) & 0x3f)) as u8,
                (0x80 | (unit & 0x3f)) as u8,
            ])?;
        }
    }
    Ok(())
}

struct BoundedNbtEncoder {
    bytes: Vec<u8>,
    max_len: usize,
}

impl BoundedNbtEncoder {
    fn new(max_len: usize) -> Self {
        Self {
            bytes: Vec::new(),
            max_len,
        }
    }

    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.write_bytes(&[value])
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let next_len = self.bytes.len().checked_add(bytes.len()).ok_or_else(|| {
            ProtocolError::InvalidData("network nbt encoded length overflow".to_string())
        })?;
        if next_len > self.max_len {
            return Err(ProtocolError::PacketTooLarge(next_len, self.max_len));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_nbt_summary_decoder_requires_one_complete_root() {
        assert_eq!(
            decode_nbt_summary(&[10, 0]).unwrap(),
            Some(NbtSummaryValue::Compound(Vec::new()))
        );
        assert!(decode_nbt_summary(&[10, 0, 0])
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after network nbt root"));
    }

    #[test]
    fn public_nbt_summary_decoder_rejects_oversized_arrays_before_body_read() {
        let mut bytes = vec![7];
        bytes.extend_from_slice(&((MAX_NBT_ARRAY_ITEMS + 1) as i32).to_be_bytes());
        assert!(matches!(
            decode_nbt_summary(&bytes),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_NBT_ARRAY_ITEMS + 1 && max == MAX_NBT_ARRAY_ITEMS
        ));
    }
}
