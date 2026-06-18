use bbb_protocol::codec::{Decoder, ProtocolError};

use crate::{Result, WorldDecodeError};

use super::state::SignBlockEntityTextState;

const MAX_NBT_DEPTH: usize = 64;
const MAX_NBT_LIST_ITEMS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
enum NbtValue {
    String(String),
    List(Vec<NbtValue>),
    Compound(Vec<(String, NbtValue)>),
    Other,
}

pub(crate) fn decode_sign_block_entity_text(
    raw_nbt: &[u8],
) -> Result<Option<SignBlockEntityTextState>> {
    let mut decoder = Decoder::new(raw_nbt);
    let Some(root) = read_nbt_any(&mut decoder)? else {
        return Ok(None);
    };
    if !decoder.is_empty() {
        return Err(WorldDecodeError::TrailingBlockEntityData {
            remaining: decoder.remaining_len(),
        });
    }

    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let front = decode_sign_text_side(&entries, "front_text");
    let back = decode_sign_text_side(&entries, "back_text");
    if front.is_none() && back.is_none() {
        return Ok(None);
    }
    Ok(Some(SignBlockEntityTextState {
        front: front.unwrap_or_else(empty_lines),
        back: back.unwrap_or_else(empty_lines),
    }))
}

fn decode_sign_text_side(entries: &[(String, NbtValue)], key: &str) -> Option<[String; 4]> {
    let NbtValue::Compound(text_entries) = find_entry(entries, key)? else {
        return None;
    };
    let NbtValue::List(messages) = find_entry(text_entries, "messages")? else {
        return None;
    };
    if messages.len() != 4 {
        return None;
    }
    Some(std::array::from_fn(|index| {
        component_plain_text(&messages[index])
    }))
}

fn read_nbt_any(decoder: &mut Decoder<'_>) -> Result<Option<NbtValue>> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(None);
    }
    read_nbt_payload(decoder, tag_id, 0).map(Some)
}

fn read_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Result<NbtValue> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData("sign nbt exceeded max depth".to_string()).into());
    }

    match tag_id {
        1 => {
            decoder.read_exact(1, "nbt byte")?;
            Ok(NbtValue::Other)
        }
        2 => {
            decoder.read_exact(2, "nbt short")?;
            Ok(NbtValue::Other)
        }
        3 | 5 => {
            decoder.read_exact(4, "nbt int/float")?;
            Ok(NbtValue::Other)
        }
        4 | 6 => {
            decoder.read_exact(8, "nbt long/double")?;
            Ok(NbtValue::Other)
        }
        7 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len, "nbt byte array")?;
            Ok(NbtValue::Other)
        }
        8 => Ok(NbtValue::String(read_modified_utf8(decoder)?)),
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_LIST_ITEMS).into());
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty sign nbt list has end tag element type".to_string(),
                )
                .into());
            }
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(read_nbt_payload(decoder, element_type, depth + 1)?);
            }
            Ok(NbtValue::List(items))
        }
        10 => {
            let mut entries = Vec::new();
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    break;
                }
                let name = read_modified_utf8(decoder)?;
                let value = read_nbt_payload(decoder, nested_type, depth + 1)?;
                entries.push((name, value));
            }
            Ok(NbtValue::Compound(entries))
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt int array")?;
            Ok(NbtValue::Other)
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt long array")?;
            Ok(NbtValue::Other)
        }
        other => Err(WorldDecodeError::InvalidNbtTag(other)),
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(WorldDecodeError::NegativeNbtArrayLength(len));
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
                )
                .into());
            }
            let b1 = bytes[cursor + 1];
            if b1 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                )
                .into());
            }
            units.push((u16::from(b0 & 0x1f) << 6) | u16::from(b1 & 0x3f));
            cursor += 2;
        } else if b0 & 0xf0 == 0xe0 {
            if cursor + 2 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                )
                .into());
            }
            let b1 = bytes[cursor + 1];
            let b2 = bytes[cursor + 2];
            if b1 & 0xc0 != 0x80 || b2 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                )
                .into());
            }
            units.push(
                (u16::from(b0 & 0x0f) << 12) | (u16::from(b1 & 0x3f) << 6) | u16::from(b2 & 0x3f),
            );
            cursor += 3;
        } else {
            return Err(ProtocolError::InvalidData(
                "invalid modified utf-8 leading byte".to_string(),
            )
            .into());
        }
    }

    String::from_utf16(&units)
        .map_err(|_| ProtocolError::InvalidData("invalid modified utf-8 string".to_string()).into())
}

fn component_plain_text(value: &NbtValue) -> String {
    let mut out = String::new();
    append_component_text(value, &mut out);
    out
}

fn append_component_text(value: &NbtValue, out: &mut String) {
    match value {
        NbtValue::String(text) => out.push_str(text),
        NbtValue::List(items) => {
            for item in items {
                append_component_text(item, out);
            }
        }
        NbtValue::Compound(entries) => {
            append_primary_component_text(entries, out);
            if let Some(extra) = find_entry(entries, "extra") {
                append_component_text(extra, out);
            }
        }
        NbtValue::Other => {}
    }
}

fn append_primary_component_text(entries: &[(String, NbtValue)], out: &mut String) {
    if let Some(text) = find_entry(entries, "text") {
        append_component_text(text, out);
        return;
    }

    if let Some(fallback) = find_entry(entries, "fallback") {
        append_component_text(fallback, out);
    } else if let Some(translate) = find_entry(entries, "translate") {
        append_component_text(translate, out);
    } else if let Some(keybind) = find_entry(entries, "keybind") {
        append_component_text(keybind, out);
    } else if let Some(selector) = find_entry(entries, "selector") {
        append_component_text(selector, out);
    } else if let Some(nbt) = find_entry(entries, "nbt") {
        append_component_text(nbt, out);
    }

    if let Some(with) = find_entry(entries, "with") {
        if !out.is_empty() {
            out.push(' ');
        }
        append_component_text(with, out);
    }
}

fn find_entry<'a>(entries: &'a [(String, NbtValue)], key: &str) -> Option<&'a NbtValue> {
    entries.iter().find(|(name, _)| name == key).map(|(_, v)| v)
}

fn empty_lines() -> [String; 4] {
    std::array::from_fn(|_| String::new())
}
