use crate::codec::{Decoder, ProtocolError, Result};

const MAX_NBT_DEPTH: usize = 64;
const MAX_NBT_LIST_ITEMS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
enum NbtValue {
    String(String),
    List(Vec<NbtValue>),
    Compound(Vec<(String, NbtValue)>),
    Other,
}

pub(crate) fn decode_component_summary_from_decoder(decoder: &mut Decoder<'_>) -> Result<String> {
    let root = read_nbt_any(decoder)?;

    let mut out = String::new();
    append_component_text(&root, &mut out);
    if out.is_empty() {
        Ok("component nbt".to_string())
    } else {
        Ok(out)
    }
}

pub(crate) fn skip_nbt_tag_from_decoder(decoder: &mut Decoder<'_>) -> Result<()> {
    read_nbt_any(decoder).map(|_| ())
}

pub(crate) fn decode_component_summary(payload: &[u8]) -> Result<String> {
    let mut decoder = Decoder::new(payload);
    let summary = decode_component_summary_from_decoder(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after component nbt".to_string(),
        ));
    }
    if summary == "component nbt" {
        Ok(format!("component nbt ({} bytes)", payload.len()))
    } else {
        Ok(summary)
    }
}

fn read_nbt_any(decoder: &mut Decoder<'_>) -> Result<NbtValue> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(NbtValue::Other);
    }
    read_nbt_payload(decoder, tag_id, 0)
}

fn read_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Result<NbtValue> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "component nbt exceeded max depth".to_string(),
        ));
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
        other => Err(ProtocolError::InvalidData(format!(
            "invalid component nbt tag id {other}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_string_component_nbt() {
        let payload = nbt_string_root("Disconnected");
        assert_eq!(
            decode_component_summary(&payload).unwrap(),
            "Disconnected".to_string()
        );
    }

    #[test]
    fn decodes_component_text_without_trimming_magic_names() {
        let payload = nbt_string_root(" jeb_ ");
        assert_eq!(decode_component_summary(&payload).unwrap(), " jeb_ ");
    }

    #[test]
    fn decodes_compound_text_and_extra_component_nbt() {
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "Kicked");
        payload.push(9);
        write_mutf8(&mut payload, "extra");
        payload.push(10);
        payload.extend_from_slice(&1i32.to_be_bytes());
        write_named_string(&mut payload, "text", " by server");
        payload.push(0);
        payload.push(0);

        assert_eq!(
            decode_component_summary(&payload).unwrap(),
            "Kicked by server".to_string()
        );
    }

    fn nbt_string_root(value: &str) -> Vec<u8> {
        let mut payload = vec![8];
        write_mutf8(&mut payload, value);
        payload
    }

    fn write_named_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_mutf8(out, name);
        write_mutf8(out, value);
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
