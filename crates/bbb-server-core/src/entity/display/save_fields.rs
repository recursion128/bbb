//! `ValueInput` numeric readers as `TagValueInput` answers them
//! (`TagValueInput.java:94-104`, `:143-188`): a field of any numeric tag is
//! converted with the `NumericTag` narrowing of its own type, and a missing
//! or non-numeric field yields the caller's default.

pub(crate) use bbb_protocol::nbt_summary::codec_float_value as numeric_float;
/// `Codec.LONG` over `NbtOps`: `Number.longValue()` of the tag's boxed
/// value, which truncates a float or double toward zero.
pub(crate) use bbb_protocol::nbt_summary::codec_long_value as codec_long;
pub(crate) use bbb_protocol::nbt_summary::compound_field as optional_field;
pub(crate) use bbb_protocol::nbt_summary::entry;
use bbb_protocol::nbt_summary::uuid_from_words;
pub(crate) use bbb_protocol::nbt_summary::uuid_nbt as uuid_int_array;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

pub(crate) fn byte_bool(value: bool) -> NbtSummaryValue {
    NbtSummaryValue::Byte(i8::from(value))
}

pub(crate) fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

pub(crate) fn float_list(values: &[f32]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.iter().map(|value| float(*value)).collect())
}

/// `Mth.floor(double)` (`Mth.java:65-67`) over the narrowing `(int)` cast.
fn java_floor_to_int(value: f64) -> i32 {
    value.floor() as i32
}

/// `NumericTag.intValue()` per tag type (`ByteTag.java:89-91`,
/// `ShortTag.java:83-85`, `IntTag.java:83-85`, `LongTag.java:83-85`,
/// `FloatTag.java:85-87`, `DoubleTag.java:85-87`).
pub(crate) fn numeric_int(value: &NbtSummaryValue) -> Option<i32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => i32::from(*value),
        NbtSummaryValue::Short(value) => i32::from(*value),
        NbtSummaryValue::Int(value) => *value,
        NbtSummaryValue::Long(value) => *value as i32,
        NbtSummaryValue::Float(bits) => java_floor_to_int(f64::from(f32::from_bits(*bits))),
        NbtSummaryValue::Double(bits) => java_floor_to_int(f64::from_bits(*bits)),
        _ => return None,
    })
}

/// `NumericTag.byteValue()` per tag type (`ByteTag.java:99-101`,
/// `ShortTag.java:93-95`, `IntTag.java:93-95`, `LongTag.java:93-95`,
/// `FloatTag.java:95-97`, `DoubleTag.java:95-97`).
pub(crate) fn numeric_byte(value: &NbtSummaryValue) -> Option<i8> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value,
        NbtSummaryValue::Short(value) => *value as i8,
        NbtSummaryValue::Int(value) => *value as i8,
        NbtSummaryValue::Long(value) => *value as i8,
        NbtSummaryValue::Float(bits) => java_floor_to_int(f64::from(f32::from_bits(*bits))) as i8,
        NbtSummaryValue::Double(bits) => java_floor_to_int(f64::from_bits(*bits)) as i8,
        _ => return None,
    })
}

/// `ValueInput.getFloatOr` (`TagValueInput.java:185-188`).
pub(crate) fn float_or(entries: &[NbtSummaryEntry], name: &str, default: f32) -> f32 {
    optional_field(entries, name)
        .and_then(numeric_float)
        .unwrap_or(default)
}

/// `ValueInput.getIntOr` (`TagValueInput.java:167-170`).
pub(crate) fn int_or(entries: &[NbtSummaryEntry], name: &str, default: i32) -> i32 {
    optional_field(entries, name)
        .and_then(numeric_int)
        .unwrap_or(default)
}

/// `ValueInput.getByteOr` (`TagValueInput.java:149-152`).
pub(crate) fn byte_or(entries: &[NbtSummaryEntry], name: &str, default: i8) -> i8 {
    optional_field(entries, name)
        .and_then(numeric_byte)
        .unwrap_or(default)
}

/// `ValueInput.getBooleanOr` (`TagValueInput.java:143-146`).
pub(crate) fn bool_or(entries: &[NbtSummaryEntry], name: &str, default: bool) -> bool {
    optional_field(entries, name)
        .and_then(numeric_byte)
        .map_or(default, |value| value != 0)
}

/// `Codec.FLOAT.listOf()` fixed to `len` entries (`ExtraCodecs.java:94-98`,
/// `:110-114`): `None` for anything but a list of exactly `len` numbers, each
/// read as `Number.floatValue()` of the boxed tag value.
pub(crate) fn fixed_float_list(value: &NbtSummaryValue, len: usize) -> Option<Vec<f32>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    if values.len() != len {
        return None;
    }
    values.iter().map(numeric_float).collect()
}

/// `UUIDUtil.CODEC` (`UUIDUtil.java:23-24`) read: the four big-endian ints of
/// an int array (`NbtOps.java:253-255`), or a list of four numbers through
/// `DynamicOps.getIntStream`.
pub(crate) fn codec_uuid(value: &NbtSummaryValue) -> Option<Uuid> {
    let words = match value {
        NbtSummaryValue::IntArray(words) => words.as_slice().try_into().ok()?,
        NbtSummaryValue::List(values) if values.len() == 4 => {
            let mut words = [0; 4];
            for (word, value) in words.iter_mut().zip(values) {
                *word = numeric_int(value)?;
            }
            words
        }
        _ => return None,
    };
    Some(uuid_from_words(words))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_round_trips_through_the_int_array_form() {
        let uuid = Uuid::from_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
        assert_eq!(codec_uuid(&uuid_int_array(uuid)), Some(uuid));
    }
}
