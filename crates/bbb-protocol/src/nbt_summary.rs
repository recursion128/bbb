use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

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
