//! Primitive reads matching `TagValueInput` and NBT-backed codecs.

pub(super) use bbb_protocol::nbt_summary::{
    codec_float_value as numeric_float, codec_int_value as codec_int,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::unique_field;

/// `TagValueInput` delegates every primitive getter to `NumericTag`
/// (`TagValueInput.java:143-193`).
pub(super) fn byte_or(record: &[NbtSummaryEntry], name: &str, default: i8) -> i8 {
    unique_field(record, name)
        .and_then(numeric_byte)
        .unwrap_or(default)
}

pub(super) fn short_or(record: &[NbtSummaryEntry], name: &str, default: i32) -> i32 {
    unique_field(record, name)
        .and_then(numeric_short)
        .unwrap_or(default)
}

pub(super) fn int_or(record: &[NbtSummaryEntry], name: &str, default: i32) -> i32 {
    unique_field(record, name)
        .and_then(numeric_int)
        .unwrap_or(default)
}

pub(super) fn long_or(record: &[NbtSummaryEntry], name: &str, default: i64) -> i64 {
    unique_field(record, name)
        .and_then(numeric_long)
        .unwrap_or(default)
}

pub(super) fn float_or(record: &[NbtSummaryEntry], name: &str, default: f32) -> f32 {
    unique_field(record, name)
        .and_then(numeric_float)
        .unwrap_or(default)
}

pub(super) fn boolean_or(record: &[NbtSummaryEntry], name: &str, default: bool) -> bool {
    unique_field(record, name)
        .and_then(numeric_boolean)
        .unwrap_or(default)
}

pub(super) fn numeric_byte(value: &NbtSummaryValue) -> Option<i8> {
    numeric_int(value).map(|value| value as i8)
}

pub(super) fn numeric_short(value: &NbtSummaryValue) -> Option<i32> {
    numeric_int(value).map(|value| i32::from(value as i16))
}

pub(super) fn numeric_int(value: &NbtSummaryValue) -> Option<i32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => i32::from(*value),
        NbtSummaryValue::Short(value) => i32::from(*value),
        NbtSummaryValue::Int(value) => *value,
        NbtSummaryValue::Long(value) => *value as i32,
        NbtSummaryValue::Float(bits) => java_floor_i32(f32::from_bits(*bits) as f64),
        NbtSummaryValue::Double(bits) => java_floor_i32(f64::from_bits(*bits)),
        _ => return None,
    })
}

pub(super) fn numeric_long(value: &NbtSummaryValue) -> Option<i64> {
    match value {
        NbtSummaryValue::Double(bits) => Some(f64::from_bits(*bits).floor() as i64),
        _ => bbb_protocol::nbt_summary::codec_long_value(value),
    }
}

pub(super) fn numeric_boolean(value: &NbtSummaryValue) -> Option<bool> {
    numeric_byte(value).map(|value| value != 0)
}

/// Primitive codecs use the boxed number exposed by `NbtOps`
/// (`NbtOps.java:65-66`), whose floating conversions truncate instead.
pub(super) fn codec_byte(value: &NbtSummaryValue) -> Option<i8> {
    codec_int(value).map(|value| value as i8)
}

pub(super) fn codec_boolean(value: &NbtSummaryValue) -> Option<bool> {
    codec_byte(value).map(|value| value != 0)
}

fn java_floor_i32(value: f64) -> i32 {
    value.floor() as i32
}
