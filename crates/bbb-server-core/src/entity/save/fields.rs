use bbb_protocol::nbt_summary::codec_double_value as numeric_double;
pub(crate) use bbb_protocol::nbt_summary::codec_float_value as numeric_float;
pub(crate) use bbb_protocol::nbt_summary::compound_field as optional_field;
use bbb_protocol::nbt_summary::uuid_from_words;
pub(crate) use bbb_protocol::nbt_summary::uuid_nbt as uuid_int_array;

use super::*;

/// `BlockState.CODEC`: an unknown block fails the read; bad properties keep their defaults.
pub(crate) fn read_block_state_from_value_with_registry(
    value: &NbtSummaryValue,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<i32> {
    bbb_registry_codec::registry_codec::decode_block_state_id_nbt_with_content_registry(
        value,
        "",
        Default::default(),
        content_registry,
    )
    .ok()
}

// DoubleTag.valueOf folds -0.0 into the shared zero.
pub(super) fn double_list(value: Vec3d) -> NbtSummaryValue {
    NbtSummaryValue::List(
        [value.x, value.y, value.z]
            .map(|value| {
                NbtSummaryValue::Double(bbb_protocol::nbt_summary::canonical_double_tag_bits(value))
            })
            .into(),
    )
}

pub(crate) fn byte_bool(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(i8::from(value))
}

/// `Vec3.CODEC`: exactly three numbers.
pub(super) fn optional_vec3(entries: &[NbtSummaryEntry], name: &str) -> Option<Vec3d> {
    let NbtSummaryValue::List(values) = optional_field(entries, name)? else {
        return None;
    };
    let [x, y, z] = &values[..] else {
        return None;
    };
    Some(Vec3d {
        x: numeric_double(x)?,
        y: numeric_double(y)?,
        z: numeric_double(z)?,
    })
}

/// `Vec2.CODEC`: exactly two numbers.
pub(super) fn optional_float_pair(entries: &[NbtSummaryEntry], name: &str) -> Option<[f32; 2]> {
    let NbtSummaryValue::List(values) = optional_field(entries, name)? else {
        return None;
    };
    let [x, y] = &values[..] else {
        return None;
    };
    Some([numeric_float(x)?, numeric_float(y)?])
}

// TagValueInput.java:143-188: a missing or non-numeric tag answers the caller's default.
pub(crate) fn optional_int(entries: &[NbtSummaryEntry], name: &str) -> Option<i32> {
    optional_field(entries, name).and_then(super::numeric_tag_int_value)
}

pub(crate) fn optional_long(entries: &[NbtSummaryEntry], name: &str) -> Option<i64> {
    optional_field(entries, name).and_then(super::numeric_tag_long_value)
}

pub(crate) fn optional_float(entries: &[NbtSummaryEntry], name: &str) -> Option<f32> {
    optional_field(entries, name).and_then(numeric_float)
}

pub(crate) fn optional_double(entries: &[NbtSummaryEntry], name: &str) -> Option<f64> {
    optional_field(entries, name).and_then(numeric_double)
}

pub(crate) fn optional_byte(entries: &[NbtSummaryEntry], name: &str) -> Option<i8> {
    optional_field(entries, name).and_then(super::numeric_tag_byte_value)
}

pub(crate) fn optional_short(entries: &[NbtSummaryEntry], name: &str) -> Option<i16> {
    optional_field(entries, name).and_then(numeric_short)
}

pub(crate) fn optional_bool(entries: &[NbtSummaryEntry], name: &str) -> Option<bool> {
    optional_byte(entries, name).map(|value| value != 0)
}

pub(crate) fn optional_uuid(entries: &[NbtSummaryEntry], name: &str) -> Option<Uuid> {
    optional_field(entries, name).and_then(read_uuid_codec_input)
}

pub(crate) fn optional_block_pos(
    entries: &[NbtSummaryEntry],
    name: &str,
) -> Option<bbb_protocol::packets::BlockPos> {
    optional_field(entries, name).and_then(read_block_pos_codec_input)
}

// UUIDUtil.java:23-24; Util.java:899-906: an overlong numeric stream retains its prefix.
pub(crate) fn read_uuid_codec_input(value: &NbtSummaryValue) -> Option<Uuid> {
    read_codec_int_stream::<4>(value).map(uuid_from_words)
}

// BlockPos.java:34-38; TagValueInput.java:49-62.
pub(crate) fn read_block_pos_codec_input(
    value: &NbtSummaryValue,
) -> Option<bbb_protocol::packets::BlockPos> {
    let [x, y, z] = read_codec_int_stream::<3>(value)?;
    Some(bbb_protocol::packets::BlockPos { x, y, z })
}

fn read_codec_int_stream<const N: usize>(value: &NbtSummaryValue) -> Option<[i32; N]> {
    match value {
        NbtSummaryValue::IntArray(words) => words.get(..N)?.try_into().ok(),
        NbtSummaryValue::ByteArray(words) => {
            let words = words.get(..N)?;
            Some(std::array::from_fn(|i| i32::from(words[i])))
        }
        NbtSummaryValue::LongArray(words) => {
            let words = words.get(..N)?;
            Some(std::array::from_fn(|i| words[i] as i32))
        }
        // DynamicOps validates every numeric list item before fixedSize takes its prefix.
        NbtSummaryValue::List(words) => {
            let mut result = [0; N];
            for (i, word) in words.iter().enumerate() {
                let word = bbb_protocol::nbt_summary::codec_int_value(word)?;
                if i < N {
                    result[i] = word;
                }
            }
            (words.len() >= N).then_some(result)
        }
        _ => None,
    }
}

// NbtOps.java:234-240: list codecs also accept the three primitive arrays; items
// that fail to decode are dropped.
pub(crate) fn read_list_codec<T>(
    value: &NbtSummaryValue,
    mut decode: impl FnMut(&NbtSummaryValue) -> Option<T>,
) -> Option<Vec<T>> {
    Some(match value {
        NbtSummaryValue::List(values) => values.iter().filter_map(decode).collect(),
        NbtSummaryValue::ByteArray(values) => values
            .iter()
            .filter_map(|v| decode(&NbtSummaryValue::Byte(*v)))
            .collect(),
        NbtSummaryValue::IntArray(values) => values
            .iter()
            .filter_map(|v| decode(&NbtSummaryValue::Int(*v)))
            .collect(),
        NbtSummaryValue::LongArray(values) => values
            .iter()
            .filter_map(|v| decode(&NbtSummaryValue::Long(*v)))
            .collect(),
        _ => return None,
    })
}

/// `Entity.load`'s `Tags` list (`Entity.java:2089-2090`): non-string items are dropped.
pub(super) fn optional_string_list(
    entries: &[NbtSummaryEntry],
    name: &str,
) -> Option<Vec<bbb_protocol::JavaString>> {
    let NbtSummaryValue::List(values) = optional_field(entries, name)? else {
        return None;
    };
    Some(
        values
            .iter()
            .filter_map(|value| match value {
                NbtSummaryValue::String(value) => Some(value.clone()),
                _ => None,
            })
            .take(crate::entity::MAX_ENTITY_TAG_COUNT)
            .collect(),
    )
}

fn numeric_short(value: &NbtSummaryValue) -> Option<i16> {
    super::numeric_tag_int_value(value).map(|value| value as i16)
}
