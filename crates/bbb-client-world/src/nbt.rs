use rustc_hash::FxHashMap;

use bbb_protocol::{
    codec::{Decoder, ProtocolError},
    nbt_summary::read_modified_utf8,
    JavaString,
};

use crate::error::{Result, WorldDecodeError};

const MAX_NBT_DEPTH: usize = 64;
const MAX_NBT_LIST_ITEMS: usize = 4096;

#[derive(Clone, Copy)]
pub(crate) struct NbtDecodeOptions {
    pub max_list_items: usize,
    pub max_array_items: usize,
    pub deduplicate_compound_keys: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NbtValue<'a> {
    Byte(i8),
    Short(i16),
    Int(i32),
    Float(f32),
    Double(f64),
    Long(i64),
    String(JavaString),
    List(Vec<NbtValue<'a>>),
    Compound(Vec<(JavaString, NbtValue<'a>)>),
    IntArray(Vec<i32>),
    ByteArray(&'a [u8]),
    LongArray(&'a [u8]),
}

pub fn decode_nbt_root(raw_nbt: &[u8]) -> Result<Option<NbtValue<'_>>> {
    let mut decoder = Decoder::new(raw_nbt);
    let root = read_nbt_any(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(WorldDecodeError::TrailingBlockEntityData {
            remaining: decoder.remaining_len(),
        });
    }
    Ok(root)
}

pub fn find_entry<'a, 'nbt>(
    entries: &'a [(JavaString, NbtValue<'nbt>)],
    key: &str,
) -> Option<&'a NbtValue<'nbt>> {
    entries
        .iter()
        .rev()
        .find(|(name, _)| name == key)
        .map(|(_, v)| v)
}

fn read_nbt_any<'a>(decoder: &mut Decoder<'a>) -> Result<Option<NbtValue<'a>>> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(None);
    }
    read_nbt_payload(
        decoder,
        tag_id,
        0,
        NbtDecodeOptions {
            max_list_items: MAX_NBT_LIST_ITEMS,
            max_array_items: usize::MAX,
            deduplicate_compound_keys: false,
        },
    )
    .map(Some)
}

pub(crate) fn read_nbt_payload<'a>(
    decoder: &mut Decoder<'a>,
    tag_id: u8,
    depth: usize,
    options: NbtDecodeOptions,
) -> Result<NbtValue<'a>> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData("nbt exceeded max depth".to_string()).into());
    }

    match tag_id {
        1 => Ok(NbtValue::Byte(decoder.read_i8()?)),
        2 => Ok(NbtValue::Short(decoder.read_i16()?)),
        3 => Ok(NbtValue::Int(decoder.read_i32()?)),
        5 => Ok(NbtValue::Float(decoder.read_f32()?)),
        4 => Ok(NbtValue::Long(decoder.read_i64()?)),
        6 => Ok(NbtValue::Double(decoder.read_f64()?)),
        7 => {
            let len = read_nbt_len(decoder, options.max_array_items)?;
            Ok(NbtValue::ByteArray(
                decoder.read_exact(len, "nbt byte array")?,
            ))
        }
        8 => Ok(NbtValue::String(read_modified_utf8(decoder)?)),
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder, options.max_list_items)?;
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty nbt list has end tag element type".to_string(),
                )
                .into());
            }
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(read_nbt_payload(decoder, element_type, depth + 1, options)?);
            }
            Ok(NbtValue::List(items))
        }
        10 => {
            let mut entries: Vec<(JavaString, NbtValue<'a>)> = Vec::new();
            let mut index_by_key: Option<FxHashMap<JavaString, usize>> =
                options.deduplicate_compound_keys.then(FxHashMap::default);
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    break;
                }
                let name = read_modified_utf8(decoder)?;
                let value = read_nbt_payload(decoder, nested_type, depth + 1, options)?;
                if let Some(index_by_key) = &mut index_by_key {
                    if let Some(&index) = index_by_key.get(&name) {
                        entries[index].1 = value;
                        continue;
                    }
                    index_by_key.insert(name.clone(), entries.len());
                }
                entries.push((name, value));
            }
            Ok(NbtValue::Compound(entries))
        }
        11 => {
            let len = read_nbt_len(decoder, options.max_array_items)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            let bytes = decoder.read_exact(byte_len, "nbt int array")?;
            let values = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .copied()
                .map(i32::from_be_bytes)
                .collect();
            Ok(NbtValue::IntArray(values))
        }
        12 => {
            let len = read_nbt_len(decoder, options.max_array_items)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            Ok(NbtValue::LongArray(
                decoder.read_exact(byte_len, "nbt long array")?,
            ))
        }
        other => Err(WorldDecodeError::InvalidNbtTag(other)),
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>, limit: usize) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(WorldDecodeError::NegativeNbtArrayLength(len));
    }
    let len = len as usize;
    if len > limit {
        return Err(ProtocolError::PacketTooLarge(len, limit).into());
    }
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_keys_keep_java_units_and_known_fields_use_last_write() {
        let mut raw = vec![10];
        for (name, value) in [
            ("雪", 1_i32),
            ("☃", 2),
            ("é", 3),
            ("?", 4),
            ("Target", 5),
            ("雪", 6),
            ("Target", 7),
        ] {
            raw.push(3);
            write_mutf8(&mut raw, &name.encode_utf16().collect::<Vec<_>>());
            raw.extend_from_slice(&value.to_be_bytes());
        }
        for (unit, value) in [(0xd800, 8_i32), (0xd801, 9), (0xfffd, 10)] {
            raw.push(3);
            write_mutf8(&mut raw, &[unit]);
            raw.extend_from_slice(&value.to_be_bytes());
        }
        raw.push(8);
        write_mutf8(&mut raw, &"value".encode_utf16().collect::<Vec<_>>());
        write_mutf8(&mut raw, &"😀".encode_utf16().collect::<Vec<_>>());
        raw.push(0);

        let Some(NbtValue::Compound(entries)) = decode_nbt_root(&raw).unwrap() else {
            panic!("expected compound");
        };
        assert_eq!(entries.len(), 11);
        for (unit, value) in [(0xd800, 8), (0xd801, 9), (0xfffd, 10)] {
            let (_, actual) = entries
                .iter()
                .find(|(name, _)| name.units() == [unit])
                .unwrap();
            assert_eq!(actual, &NbtValue::Int(value));
        }
        for (key, expected) in [("雪", 6), ("☃", 2), ("é", 3), ("?", 4)] {
            assert_eq!(find_entry(&entries, key), Some(&NbtValue::Int(expected)));
        }
        assert_eq!(find_entry(&entries, "Target"), Some(&NbtValue::Int(7)));
        assert_eq!(
            find_entry(&entries, "value"),
            Some(&NbtValue::String("😀".into()))
        );
    }

    fn write_mutf8(out: &mut Vec<u8>, units: &[u16]) {
        let mut bytes = Vec::new();
        for &unit in units {
            if unit == 0 {
                bytes.extend_from_slice(&[0xc0, 0x80]);
            } else if unit <= 0x7f {
                bytes.push(unit as u8);
            } else if unit <= 0x7ff {
                bytes.push((0xc0 | (unit >> 6)) as u8);
                bytes.push((0x80 | (unit & 0x3f)) as u8);
            } else {
                bytes.push((0xe0 | (unit >> 12)) as u8);
                bytes.push((0x80 | ((unit >> 6) & 0x3f)) as u8);
                bytes.push((0x80 | (unit & 0x3f)) as u8);
            }
        }
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(&bytes);
    }
}
