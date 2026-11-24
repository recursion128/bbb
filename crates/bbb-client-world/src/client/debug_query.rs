use bbb_protocol::{
    codec::{Decoder, ProtocolError},
    nbt_summary::{canonical_snbt_key_units, canonical_snbt_string},
    packets::TagQuery as ProtocolTagQuery,
    JavaString,
};
use serde::{Deserialize, Serialize};

use crate::{
    nbt::{read_nbt_payload, NbtDecodeOptions, NbtValue},
    Result, WorldStore,
};

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
        let NbtValue::Compound(entries) = value else {
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

fn decode_tag_query_nbt_root(raw_nbt: &[u8]) -> Result<Option<NbtValue<'_>>> {
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

    let value = read_nbt_payload(
        &mut decoder,
        tag_id,
        0,
        NbtDecodeOptions {
            max_list_items: MAX_TAG_QUERY_NBT_LIST_ITEMS,
            max_array_items: MAX_TAG_QUERY_NBT_LIST_ITEMS,
            deduplicate_compound_keys: true,
        },
    )?;
    if !decoder.is_empty() {
        return Err(
            ProtocolError::InvalidData("trailing bytes after tag query nbt".to_string()).into(),
        );
    }
    Ok(Some(value))
}

fn write_snbt_compound(
    entries: &[(JavaString, NbtValue<'_>)],
    removed_keys: &[&str],
    format: SnbtFormat,
) -> String {
    let mut entries = entries
        .iter()
        .filter(|(key, _)| !removed_keys.iter().any(|removed| key == removed))
        .collect::<Vec<_>>();
    if matches!(format, SnbtFormat::Compact) {
        entries.sort_by(|(left, _), (right, _)| left.units().cmp(right.units()));
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

fn write_snbt_value(out: &mut String, value: &NbtValue<'_>, format: SnbtFormat) {
    match value {
        NbtValue::Byte(value) => out.push_str(&format!("{value}b")),
        NbtValue::Short(value) => out.push_str(&format!("{value}s")),
        NbtValue::Int(value) => out.push_str(&value.to_string()),
        NbtValue::Long(value) => out.push_str(&format!("{value}L")),
        NbtValue::Float(value) => out.push_str(&format!("{}f", java_float_string(*value))),
        NbtValue::Double(value) => out.push_str(&format!("{}d", java_double_string(*value))),
        NbtValue::ByteArray(values) => {
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
                out.push_str(&format!("{}B", *value as i8));
            }
            out.push(']');
        }
        NbtValue::String(value) => out.push_str(&canonical_snbt_string(&value.to_nbt())),
        NbtValue::List(values) => {
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
        NbtValue::Compound(entries) => {
            out.push_str(&write_snbt_compound(entries, &[], format));
        }
        NbtValue::IntArray(values) => {
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
        NbtValue::LongArray(values) => {
            out.push_str("[L;");
            for (index, &value) in values.as_chunks::<8>().0.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                    if matches!(format, SnbtFormat::Pretty) {
                        out.push(' ');
                    }
                } else if matches!(format, SnbtFormat::Pretty) && !values.is_empty() {
                    out.push(' ');
                }
                let value = i64::from_be_bytes(value);
                out.push_str(&format!("{value}L"));
            }
            out.push(']');
        }
    }
}

fn write_snbt_key(out: &mut String, key: &JavaString, format: SnbtFormat) {
    if matches!(format, SnbtFormat::Pretty) {
        if let Ok(text) = String::from_utf16(key.units()) {
            if is_pretty_unquoted_key(&text) {
                out.push_str(&text);
                return;
            }
        }
    }
    out.push_str(&canonical_snbt_key_units(key));
}

fn is_pretty_unquoted_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
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
    fn tag_query_duplicate_keys_keep_last_value_and_first_output_position() {
        let response = TagQueryResponseState {
            transaction_id: 5,
            tag_present: true,
            raw_nbt: compound_entries(vec![
                nbt_int("z", 1),
                nbt_int("a", 2),
                nbt_long_array("z", &[-1, 2]),
                nbt_compound("nested", vec![nbt_int("x", 3), nbt_int("x", 4)]),
            ]),
        };

        assert_eq!(
            response.compact_snbt().unwrap().as_deref(),
            Some("{a:2,nested:{x:4},z:[L;-1L,2L]}")
        );
        assert_eq!(
            response
                .pretty_snbt_without_root_keys(&[])
                .unwrap()
                .as_deref(),
            Some("{z: [L; -1L, 2L], a: 2, nested: {x: 4}}")
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
        nbt_string_units(
            &name.encode_utf16().collect::<Vec<_>>(),
            &value.encode_utf16().collect::<Vec<_>>(),
        )
    }

    fn nbt_string_units(name: &[u16], value: &[u16]) -> Vec<u8> {
        let mut out = vec![8];
        write_mutf8_units(&mut out, name);
        write_mutf8_units(&mut out, value);
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
        write_mutf8_units(out, &value.encode_utf16().collect::<Vec<_>>());
    }

    fn write_mutf8_units(out: &mut Vec<u8>, units: &[u16]) {
        let mut payload = Encoder::new();
        for &unit in units {
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
