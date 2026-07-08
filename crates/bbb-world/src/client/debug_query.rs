use bbb_protocol::{
    codec::{Decoder, ProtocolError},
    packets::TagQuery as ProtocolTagQuery,
};
use serde::{Deserialize, Serialize};

use crate::{Result, WorldDecodeError, WorldStore};

const MAX_TAG_QUERY_NBT_DEPTH: usize = 64;
const MAX_TAG_QUERY_NBT_LIST_ITEMS: usize = 1_000_000;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDebugQueryState {
    #[serde(default)]
    pub last_tag_query: Option<TagQueryResponseState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagQueryResponseState {
    pub transaction_id: i32,
    pub tag_present: bool,
    pub raw_nbt: Vec<u8>,
}

impl TagQueryResponseState {
    pub fn raw_nbt_len(&self) -> usize {
        self.raw_nbt.len()
    }

    pub fn compact_snbt(&self) -> Result<Option<String>> {
        self.filtered_snbt(&[], SnbtFormat::Compact)
    }

    pub fn pretty_snbt_without_root_keys(&self, removed_keys: &[&str]) -> Result<Option<String>> {
        self.filtered_snbt(removed_keys, SnbtFormat::Pretty)
    }

    fn filtered_snbt(&self, removed_keys: &[&str], format: SnbtFormat) -> Result<Option<String>> {
        if !self.tag_present {
            return Ok(None);
        }
        let Some(value) = decode_tag_query_nbt_root(&self.raw_nbt)? else {
            return Ok(None);
        };
        let NbtDebugValue::Compound(entries) = value else {
            return Err(ProtocolError::InvalidData(
                "tag query nbt root must be a compound".to_string(),
            )
            .into());
        };
        Ok(Some(write_snbt_compound(&entries, removed_keys, format)))
    }
}

impl WorldStore {
    pub fn apply_tag_query(&mut self, packet: ProtocolTagQuery) {
        self.counters.tag_query_packets += 1;
        self.debug_query.last_tag_query = Some(TagQueryResponseState {
            transaction_id: packet.transaction_id,
            tag_present: packet.tag_present,
            raw_nbt: packet.raw_nbt,
        });
    }

    pub fn debug_query(&self) -> &ClientDebugQueryState {
        &self.debug_query
    }

    pub fn client_debug_query(&self) -> &ClientDebugQueryState {
        self.debug_query()
    }

    pub fn last_tag_query(&self) -> Option<&TagQueryResponseState> {
        self.debug_query.last_tag_query.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnbtFormat {
    Compact,
    Pretty,
}

#[derive(Debug, Clone, PartialEq)]
enum NbtDebugValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtDebugValue>),
    Compound(Vec<(String, NbtDebugValue)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

fn decode_tag_query_nbt_root(raw_nbt: &[u8]) -> Result<Option<NbtDebugValue>> {
    let mut decoder = Decoder::new(raw_nbt);
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        if !decoder.is_empty() {
            return Err(ProtocolError::InvalidData(
                "trailing bytes after null tag query nbt".to_string(),
            )
            .into());
        }
        return Ok(None);
    }
    if tag_id != 10 {
        return Err(ProtocolError::InvalidData(format!(
            "tag query nbt root must be compound or end tag, got {tag_id}"
        ))
        .into());
    }

    let value = read_nbt_payload(&mut decoder, tag_id, 0)?;
    if !decoder.is_empty() {
        return Err(
            ProtocolError::InvalidData("trailing bytes after tag query nbt".to_string()).into(),
        );
    }
    Ok(Some(value))
}

fn read_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Result<NbtDebugValue> {
    if depth > MAX_TAG_QUERY_NBT_DEPTH {
        return Err(
            ProtocolError::InvalidData("tag query nbt exceeded max depth".to_string()).into(),
        );
    }

    match tag_id {
        1 => Ok(NbtDebugValue::Byte(decoder.read_i8()?)),
        2 => Ok(NbtDebugValue::Short(decoder.read_i16()?)),
        3 => Ok(NbtDebugValue::Int(decoder.read_i32()?)),
        4 => Ok(NbtDebugValue::Long(decoder.read_i64()?)),
        5 => Ok(NbtDebugValue::Float(decoder.read_f32()?)),
        6 => Ok(NbtDebugValue::Double(decoder.read_f64()?)),
        7 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_TAG_QUERY_NBT_LIST_ITEMS {
                return Err(
                    ProtocolError::PacketTooLarge(len, MAX_TAG_QUERY_NBT_LIST_ITEMS).into(),
                );
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i8()?);
            }
            Ok(NbtDebugValue::ByteArray(values))
        }
        8 => Ok(NbtDebugValue::String(read_modified_utf8(decoder)?)),
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_TAG_QUERY_NBT_LIST_ITEMS {
                return Err(
                    ProtocolError::PacketTooLarge(len, MAX_TAG_QUERY_NBT_LIST_ITEMS).into(),
                );
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty tag query nbt list has end tag element type".to_string(),
                )
                .into());
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(read_nbt_payload(decoder, element_type, depth + 1)?);
            }
            Ok(NbtDebugValue::List(values))
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
            Ok(NbtDebugValue::Compound(entries))
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_TAG_QUERY_NBT_LIST_ITEMS {
                return Err(
                    ProtocolError::PacketTooLarge(len, MAX_TAG_QUERY_NBT_LIST_ITEMS).into(),
                );
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i32()?);
            }
            Ok(NbtDebugValue::IntArray(values))
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            if len > MAX_TAG_QUERY_NBT_LIST_ITEMS {
                return Err(
                    ProtocolError::PacketTooLarge(len, MAX_TAG_QUERY_NBT_LIST_ITEMS).into(),
                );
            }
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(decoder.read_i64()?);
            }
            Ok(NbtDebugValue::LongArray(values))
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

fn write_snbt_compound(
    entries: &[(String, NbtDebugValue)],
    removed_keys: &[&str],
    format: SnbtFormat,
) -> String {
    let mut entries = entries
        .iter()
        .filter(|(key, _)| !removed_keys.iter().any(|removed| key == removed))
        .collect::<Vec<_>>();
    if matches!(format, SnbtFormat::Compact) {
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    }

    let mut out = String::new();
    out.push('{');
    for (index, (key, value)) in entries.into_iter().enumerate() {
        if index > 0 {
            out.push(',');
            if matches!(format, SnbtFormat::Pretty) {
                out.push(' ');
            }
        }
        write_snbt_key(&mut out, key, format);
        out.push(':');
        if matches!(format, SnbtFormat::Pretty) {
            out.push(' ');
        }
        write_snbt_value(&mut out, value, format);
    }
    out.push('}');
    out
}

fn write_snbt_value(out: &mut String, value: &NbtDebugValue, format: SnbtFormat) {
    match value {
        NbtDebugValue::Byte(value) => out.push_str(&format!("{value}b")),
        NbtDebugValue::Short(value) => out.push_str(&format!("{value}s")),
        NbtDebugValue::Int(value) => out.push_str(&value.to_string()),
        NbtDebugValue::Long(value) => out.push_str(&format!("{value}L")),
        NbtDebugValue::Float(value) => out.push_str(&format!("{}f", java_float_string(*value))),
        NbtDebugValue::Double(value) => out.push_str(&format!("{}d", java_double_string(*value))),
        NbtDebugValue::ByteArray(values) => {
            out.push_str("[B;");
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                    if matches!(format, SnbtFormat::Pretty) {
                        out.push(' ');
                    }
                } else if matches!(format, SnbtFormat::Pretty) && !values.is_empty() {
                    out.push(' ');
                }
                out.push_str(&format!("{value}B"));
            }
            out.push(']');
        }
        NbtDebugValue::String(value) => quote_snbt_string(out, value),
        NbtDebugValue::List(values) => {
            out.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                    if matches!(format, SnbtFormat::Pretty) {
                        out.push(' ');
                    }
                }
                write_snbt_value(out, value, format);
            }
            out.push(']');
        }
        NbtDebugValue::Compound(entries) => {
            out.push_str(&write_snbt_compound(entries, &[], format));
        }
        NbtDebugValue::IntArray(values) => {
            out.push_str("[I;");
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                    if matches!(format, SnbtFormat::Pretty) {
                        out.push(' ');
                    }
                } else if matches!(format, SnbtFormat::Pretty) && !values.is_empty() {
                    out.push(' ');
                }
                out.push_str(&value.to_string());
            }
            out.push(']');
        }
        NbtDebugValue::LongArray(values) => {
            out.push_str("[L;");
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                    if matches!(format, SnbtFormat::Pretty) {
                        out.push(' ');
                    }
                } else if matches!(format, SnbtFormat::Pretty) && !values.is_empty() {
                    out.push(' ');
                }
                out.push_str(&format!("{value}L"));
            }
            out.push(']');
        }
    }
}

fn write_snbt_key(out: &mut String, key: &str, format: SnbtFormat) {
    let simple = match format {
        SnbtFormat::Compact => is_compact_unquoted_key(key),
        SnbtFormat::Pretty => is_pretty_unquoted_key(key),
    };
    if simple {
        out.push_str(key);
    } else {
        quote_snbt_string(out, key);
    }
}

fn is_compact_unquoted_key(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '.' || first == '_') {
        return false;
    }
    if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

fn is_pretty_unquoted_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

fn quote_snbt_string(out: &mut String, value: &str) {
    let mut quote = None;
    let mut body = String::new();
    for c in value.chars() {
        match c {
            '\\' => body.push_str("\\\\"),
            '"' | '\'' => {
                if quote.is_none() {
                    quote = Some(if c == '"' { '\'' } else { '"' });
                }
                if quote == Some(c) {
                    body.push('\\');
                }
                body.push(c);
            }
            '\u{0008}' => body.push_str("\\b"),
            '\t' => body.push_str("\\t"),
            '\n' => body.push_str("\\n"),
            '\u{000c}' => body.push_str("\\f"),
            '\r' => body.push_str("\\r"),
            c if c < ' ' => body.push_str(&format!("\\x{:02x}", c as u8)),
            _ => body.push(c),
        }
    }
    let quote = quote.unwrap_or('"');
    out.push(quote);
    out.push_str(&body);
    out.push(quote);
}

fn java_float_string(value: f32) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f32::INFINITY {
        "Infinity".to_string()
    } else if value == f32::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        format!("{value:?}")
    }
}

fn java_double_string(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        format!("{value:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::packets::TagQuery;

    #[test]
    fn tag_query_stores_latest_response_and_counter() {
        let mut store = WorldStore::new();

        store.apply_tag_query(TagQuery {
            transaction_id: 7,
            tag_present: true,
            raw_nbt: vec![10, 0, 0],
        });

        assert_eq!(
            store.last_tag_query(),
            Some(&TagQueryResponseState {
                transaction_id: 7,
                tag_present: true,
                raw_nbt: vec![10, 0, 0],
            })
        );
        assert_eq!(store.last_tag_query().unwrap().raw_nbt_len(), 3);
        assert_eq!(store.counters().tag_query_packets, 1);

        store.apply_tag_query(TagQuery {
            transaction_id: 8,
            tag_present: false,
            raw_nbt: vec![0],
        });

        assert_eq!(
            store.client_debug_query().last_tag_query,
            Some(TagQueryResponseState {
                transaction_id: 8,
                tag_present: false,
                raw_nbt: vec![0],
            })
        );
        assert_eq!(store.counters().tag_query_packets, 2);
    }

    #[test]
    fn tag_query_compact_snbt_matches_vanilla_compound_string_shape() {
        let response = TagQueryResponseState {
            transaction_id: 3,
            tag_present: true,
            raw_nbt: compound_entries(vec![
                nbt_int("z", 2),
                nbt_string("name", "Chest"),
                nbt_byte_array("bytes", &[1, -2]),
                nbt_compound("nested", vec![nbt_short("value", 4)]),
                nbt_string("quote", "a\"b"),
            ]),
        };

        assert_eq!(
            response.compact_snbt().unwrap().as_deref(),
            Some("{bytes:[B;1B,-2B],name:\"Chest\",nested:{value:4s},quote:'a\"b',z:2}")
        );
    }

    #[test]
    fn tag_query_pretty_snbt_removes_entity_recreate_root_keys() {
        let response = TagQueryResponseState {
            transaction_id: 4,
            tag_present: true,
            raw_nbt: compound_entries(vec![
                nbt_string("id", "minecraft:creeper"),
                nbt_int_array("Pos", &[1, 2, 3]),
                nbt_long_array("UUID", &[11, 12]),
                nbt_list("Motion", 6, vec![1.25f64.to_be_bytes().to_vec()]),
                nbt_string("custom key", "quoted"),
            ]),
        };

        assert_eq!(
            response
                .pretty_snbt_without_root_keys(&["UUID", "Pos"])
                .unwrap()
                .as_deref(),
            Some("{id: \"minecraft:creeper\", Motion: [1.25d], \"custom key\": \"quoted\"}")
        );
    }

    #[test]
    fn null_tag_query_has_no_snbt() {
        let response = TagQueryResponseState {
            transaction_id: 5,
            tag_present: false,
            raw_nbt: vec![0],
        };

        assert_eq!(response.compact_snbt().unwrap(), None);
        assert_eq!(
            response
                .pretty_snbt_without_root_keys(&["UUID", "Pos"])
                .unwrap(),
            None
        );
    }

    fn compound_entries(entries: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = vec![10];
        for entry in entries {
            out.extend_from_slice(&entry);
        }
        out.push(0);
        out
    }

    fn nbt_byte_array(name: &str, values: &[i8]) -> Vec<u8> {
        let mut out = vec![7];
        write_mutf8(&mut out, name);
        out.extend_from_slice(&(values.len() as i32).to_be_bytes());
        for value in values {
            out.push(*value as u8);
        }
        out
    }

    fn nbt_string(name: &str, value: &str) -> Vec<u8> {
        let mut out = vec![8];
        write_mutf8(&mut out, name);
        write_mutf8(&mut out, value);
        out
    }

    fn nbt_list(name: &str, element_type: u8, values: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = vec![9];
        write_mutf8(&mut out, name);
        out.push(element_type);
        out.extend_from_slice(&(values.len() as i32).to_be_bytes());
        for value in values {
            out.extend_from_slice(&value);
        }
        out
    }

    fn nbt_compound(name: &str, entries: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = vec![10];
        write_mutf8(&mut out, name);
        for entry in entries {
            out.extend_from_slice(&entry);
        }
        out.push(0);
        out
    }

    fn nbt_int_array(name: &str, values: &[i32]) -> Vec<u8> {
        let mut out = vec![11];
        write_mutf8(&mut out, name);
        out.extend_from_slice(&(values.len() as i32).to_be_bytes());
        for value in values {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out
    }

    fn nbt_long_array(name: &str, values: &[i64]) -> Vec<u8> {
        let mut out = vec![12];
        write_mutf8(&mut out, name);
        out.extend_from_slice(&(values.len() as i32).to_be_bytes());
        for value in values {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out
    }

    fn nbt_short(name: &str, value: i16) -> Vec<u8> {
        let mut out = vec![2];
        write_mutf8(&mut out, name);
        out.extend_from_slice(&value.to_be_bytes());
        out
    }

    fn nbt_int(name: &str, value: i32) -> Vec<u8> {
        let mut out = vec![3];
        write_mutf8(&mut out, name);
        out.extend_from_slice(&value.to_be_bytes());
        out
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let mut payload = Encoder::new();
        for unit in value.encode_utf16() {
            if unit == 0 {
                payload.write_bytes(&[0xc0, 0x80]);
            } else if unit <= 0x7f {
                payload.write_u8(unit as u8);
            } else if unit <= 0x7ff {
                payload.write_u8((0xc0 | ((unit >> 6) & 0x1f)) as u8);
                payload.write_u8((0x80 | (unit & 0x3f)) as u8);
            } else {
                payload.write_u8((0xe0 | ((unit >> 12) & 0x0f)) as u8);
                payload.write_u8((0x80 | ((unit >> 6) & 0x3f)) as u8);
                payload.write_u8((0x80 | (unit & 0x3f)) as u8);
            }
        }
        let bytes = payload.into_inner();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(&bytes);
    }
}
