use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

use hashbrown::{hash_table::Entry, HashTable};
use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};
use crate::java_string::JavaString;

mod json_ops;
mod snbt;
mod snbt_encode;

pub use json_ops::{
    json_number_to_javaops, json_number_to_nbtops, JsonOpsNumber, JsonOpsNumberError,
};
pub use snbt::{
    parse_snbt_compound, parse_snbt_compound_prefix, parse_snbt_compound_prefix_utf16,
    parse_snbt_compound_utf16, parse_snbt_prefix_utf16, SnbtParseFailure,
};
pub(crate) use snbt::{parse_snbt_prefix, validate_snbt_compound_prefix, validate_snbt_prefix};
pub use snbt_encode::{canonical_snbt_java_string, canonical_snbt_string};
pub use snbt_encode::{canonical_snbt_key_java_string, canonical_snbt_key_units};

pub(crate) const MAX_NBT_DEPTH: usize = 64;
pub(crate) const MAX_NBT_ARRAY_ITEMS: usize = 4096;
pub(crate) const MAX_NBT_COMPOUND_ENTRIES: usize = 4096;
pub(crate) const MAX_NBT_LIST_ITEMS: usize = 4096;
/// `NbtAccounter.DEFAULT_NBT_QUOTA` (`NbtAccounter.java:6`).
pub const VANILLA_DEFAULT_NBT_QUOTA: u64 = 2_097_152;
/// `NbtAccounter.MAX_STACK_DEPTH` (`NbtAccounter.java:8`).
pub const VANILLA_DEFAULT_NBT_MAX_DEPTH: usize = 512;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum NbtSummaryValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    ByteArray(Vec<i8>),
    String(JavaString),
    List(Vec<NbtSummaryValue>),
    Compound(Vec<NbtSummaryEntry>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl PartialEq for NbtSummaryValue {
    fn eq(&self, other: &Self) -> bool {
        use NbtSummaryValue as V;
        match (self, other) {
            (V::Byte(a), V::Byte(b)) => a == b,
            (V::Short(a), V::Short(b)) => a == b,
            (V::Int(a), V::Int(b)) => a == b,
            (V::Long(a), V::Long(b)) => a == b,
            (V::Float(a), V::Float(b)) => a == b,
            (V::Double(a), V::Double(b)) => a == b,
            (V::ByteArray(a), V::ByteArray(b)) => a == b,
            (V::String(a), V::String(b)) => a == b,
            (V::List(a), V::List(b)) => a == b,
            // CompoundTag.java:406-407; ListTag.java:372-373.
            (V::Compound(a), V::Compound(b)) => compound_entries_eq(a, b),
            (V::IntArray(a), V::IntArray(b)) => a == b,
            (V::LongArray(a), V::LongArray(b)) => a == b,
            _ => false,
        }
    }
}

impl NbtSummaryValue {
    pub fn string(value: impl Into<JavaString>) -> Self {
        Self::String(value.into())
    }

    pub fn string_from_utf16_slice(units: &[u16]) -> Self {
        Self::String(units.to_vec().into())
    }

    pub fn string_from_utf16(units: Vec<u16>) -> Self {
        Self::String(units.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NbtSummaryEntry {
    pub name: JavaString,
    pub value: NbtSummaryValue,
}

const TINY_COMPOUND_MAX_ENTRIES: usize = 8;

fn compound_entries_eq(mut a: &[NbtSummaryEntry], mut b: &[NbtSummaryEntry]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    while let ([first_a, rest_a @ ..], [first_b, rest_b @ ..]) = (a, b) {
        if first_a.name != first_b.name {
            break;
        }
        if first_a.value == first_b.value {
            a = rest_a;
            b = rest_b;
            continue;
        }
        if !rest_b.iter().any(|entry| entry.name == first_a.name) {
            return false;
        }
        break;
    }
    if a.is_empty() {
        return true;
    }
    if a.len() <= TINY_COMPOUND_MAX_ENTRIES {
        compound_entries_eq_tiny(a, b)
    } else {
        compound_entries_eq_sorted(a, b)
    }
}

fn compound_entries_eq_tiny(a: &[NbtSummaryEntry], b: &[NbtSummaryEntry]) -> bool {
    debug_assert_eq!(a.len(), b.len());
    debug_assert!(a.len() <= TINY_COMPOUND_MAX_ENTRIES);
    let mut visited = 0u8;
    for entry in a {
        let mut match_index = None;
        for (index, candidate) in b.iter().enumerate() {
            let bit = 1u8 << index;
            if visited & bit != 0 || candidate.name != entry.name {
                continue;
            }
            if match_index.is_some() {
                return compound_entries_eq_sorted(a, b);
            }
            match_index = Some(index);
        }
        let Some(index) = match_index else {
            return false;
        };
        if b[index].value != entry.value {
            return false;
        }
        visited |= 1u8 << index;
    }
    true
}

fn compound_entries_eq_sorted(a: &[NbtSummaryEntry], b: &[NbtSummaryEntry]) -> bool {
    debug_assert_eq!(a.len(), b.len());
    let mut left: Vec<&NbtSummaryEntry> = a.iter().collect();
    let mut right: Vec<&NbtSummaryEntry> = b.iter().collect();
    left.sort_unstable_by(|x, y| x.name.cmp(&y.name));
    right.sort_unstable_by(|x, y| x.name.cmp(&y.name));
    if left.iter().zip(&right).any(|(x, y)| x.name != y.name) {
        return false;
    }
    let mut start = 0;
    while start < left.len() {
        let key = &left[start].name;
        let mut end = start + 1;
        while end < left.len() && left[end].name == *key {
            end += 1;
        }
        if end - start == 1 {
            if left[start].value != right[start].value {
                return false;
            }
        } else {
            let mut left_run: Vec<CanonicalNbtValue<'_>> = left[start..end]
                .iter()
                .map(|entry| CanonicalNbtValue::from_value(&entry.value))
                .collect();
            let mut right_run: Vec<CanonicalNbtValue<'_>> = right[start..end]
                .iter()
                .map(|entry| CanonicalNbtValue::from_value(&entry.value))
                .collect();
            left_run.sort_unstable();
            right_run.sort_unstable();
            if left_run != right_run {
                return false;
            }
        }
        start = end;
    }
    true
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum CanonicalNbtValue<'a> {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    ByteArray(&'a [i8]),
    String(&'a JavaString),
    List(Vec<CanonicalNbtValue<'a>>),
    Compound(Vec<(&'a JavaString, CanonicalNbtValue<'a>)>),
    IntArray(&'a [i32]),
    LongArray(&'a [i64]),
}

impl<'a> CanonicalNbtValue<'a> {
    fn from_value(value: &'a NbtSummaryValue) -> Self {
        match value {
            NbtSummaryValue::Byte(v) => Self::Byte(*v),
            NbtSummaryValue::Short(v) => Self::Short(*v),
            NbtSummaryValue::Int(v) => Self::Int(*v),
            NbtSummaryValue::Long(v) => Self::Long(*v),
            NbtSummaryValue::Float(v) => Self::Float(*v),
            NbtSummaryValue::Double(v) => Self::Double(*v),
            NbtSummaryValue::ByteArray(v) => Self::ByteArray(v),
            NbtSummaryValue::String(v) => Self::String(v),
            NbtSummaryValue::List(values) => {
                Self::List(values.iter().map(Self::from_value).collect())
            }
            NbtSummaryValue::Compound(entries) => {
                let mut canonical: Vec<(&'a JavaString, CanonicalNbtValue<'a>)> = entries
                    .iter()
                    .map(|entry| (&entry.name, Self::from_value(&entry.value)))
                    .collect();
                canonical.sort_unstable();
                Self::Compound(canonical)
            }
            NbtSummaryValue::IntArray(v) => Self::IntArray(v),
            NbtSummaryValue::LongArray(v) => Self::LongArray(v),
        }
    }
}

/// The first field with `name`, for lenient readers.
#[inline]
pub fn compound_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[inline]
pub fn compound_last_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[inline]
pub fn value_field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    compound_field(entries, name)
}

#[inline]
pub fn value_last_field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    compound_last_field(entries, name)
}

pub fn entry(name: impl Into<JavaString>, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NbtSummaryDecodeLimits {
    pub max_depth: usize,
    pub max_byte_array_items: usize,
    pub max_int_array_items: usize,
    pub max_long_array_items: usize,
    pub max_list_items: usize,
    pub max_compound_entries: usize,
}

impl NbtSummaryDecodeLimits {
    /// `NbtAccounter.unlimitedHeap()`: no size caps, depth 512 (`NbtAccounter.java:8,31-33`).
    pub const SAVED_FILE: Self = Self {
        max_depth: VANILLA_DEFAULT_NBT_MAX_DEPTH,
        max_byte_array_items: usize::MAX,
        max_int_array_items: usize::MAX,
        max_long_array_items: usize::MAX,
        max_list_items: usize::MAX,
        max_compound_entries: usize::MAX,
    };

    /// `NbtAccounter.defaultQuota()` / `ClientboundRegistryDataPacket` (`NbtAccounter.java:6-8,23-25`):
    /// depth 512 with collection bounds governed by the cumulative accounted-byte quota.
    pub const NETWORK_QUOTA: Self = Self {
        max_depth: VANILLA_DEFAULT_NBT_MAX_DEPTH,
        max_byte_array_items: usize::MAX,
        max_int_array_items: usize::MAX,
        max_long_array_items: usize::MAX,
        max_list_items: 1_000_000,
        max_compound_entries: usize::MAX,
    };
}

/// Vanilla-aligned cumulative accounted-byte budget (`NbtAccounter.java:5-51`).
#[derive(Clone, Copy)]
struct NbtAccounter {
    quota: u64,
    usage: u64,
}

impl NbtAccounter {
    const UNLIMITED: Self = Self {
        quota: u64::MAX,
        usage: 0,
    };

    const fn new(quota: u64) -> Self {
        Self { quota, usage: 0 }
    }

    #[inline]
    fn account_bytes(&mut self, size: u64) -> Result<()> {
        if self.quota == u64::MAX {
            return Ok(());
        }
        let next = self.usage.saturating_add(size);
        if next > self.quota {
            return Err(ProtocolError::InvalidData(format!(
                "network nbt exceeded byte quota ({next} > {})",
                self.quota
            )));
        }
        self.usage = next;
        Ok(())
    }

    #[inline]
    fn account_items(&mut self, bytes_per_item: u64, count: usize) -> Result<()> {
        if self.quota == u64::MAX {
            return Ok(());
        }
        self.account_bytes(bytes_per_item.saturating_mul(count as u64))
    }
}

impl Default for NbtSummaryDecodeLimits {
    fn default() -> Self {
        Self {
            max_depth: MAX_NBT_DEPTH,
            max_byte_array_items: MAX_NBT_ARRAY_ITEMS,
            max_int_array_items: MAX_NBT_ARRAY_ITEMS,
            max_long_array_items: MAX_NBT_ARRAY_ITEMS,
            max_list_items: MAX_NBT_LIST_ITEMS,
            max_compound_entries: MAX_NBT_COMPOUND_ENTRIES,
        }
    }
}

/// Returns the bits stored by `FloatTag.valueOf` and `NbtOps.createFloat`.
///
/// Vanilla interns either signed zero as the positive-zero tag while leaving
/// every nonzero value, including NaN payloads, untouched
/// (`FloatTag.java:46-48`).
pub fn canonical_float_tag_bits(value: f32) -> u32 {
    if value == 0.0 {
        0.0_f32.to_bits()
    } else {
        value.to_bits()
    }
}

/// Returns the bits stored by `DoubleTag.valueOf` and `NbtOps.createDouble`.
///
/// This is the double-width counterpart of [`canonical_float_tag_bits`]
/// (`DoubleTag.java:46-48`).
pub fn canonical_double_tag_bits(value: f64) -> u64 {
    if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    }
}

/// Reads one NBT value the way `Codec.INT` reads it through `NbtOps`.
///
/// `NbtOps.getNumberValue` delegates to `Tag.asNumber` (`NbtOps.java:65-67`),
/// which every `NumericTag` answers with its boxed `Number`
/// (`NumericTag.java:5-23`) while every other tag answers `Optional.empty`
/// (`Tag.java:55-57`). `Codec.INT` then narrows that `Number` with
/// `Number::intValue`, so a byte, short, long, float or double `count` is
/// accepted and narrowed rather than refused, and a list, array, string or
/// compound is not a number at all.
///
/// Measured against the 26.1 server jar through `ItemStack.CODEC.parse`:
/// `count:3b` and `count:3L` both read `3`, `count:3.9d` reads `3`,
/// `count:4294967298L` reads `2` (the low 32 bits), and `count:[I;3]` is not a
/// number.
#[inline]
pub fn codec_int_value(value: &NbtSummaryValue) -> Option<i32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => i32::from(*value),
        NbtSummaryValue::Short(value) => i32::from(*value),
        NbtSummaryValue::Int(value) => *value,
        NbtSummaryValue::Long(value) => *value as i32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) as i32,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as i32,
        _ => return None,
    })
}

#[inline]
pub fn checked_int_value(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

pub fn integer_bool_value(value: &NbtSummaryValue) -> Option<bool> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value != 0,
        NbtSummaryValue::Short(value) => *value != 0,
        NbtSummaryValue::Int(value) => *value != 0,
        NbtSummaryValue::Long(value) => *value != 0,
        _ => return None,
    })
}

pub fn codec_long_value(value: &NbtSummaryValue) -> Option<i64> {
    Some(match value {
        NbtSummaryValue::Byte(value) => i64::from(*value),
        NbtSummaryValue::Short(value) => i64::from(*value),
        NbtSummaryValue::Int(value) => i64::from(*value),
        NbtSummaryValue::Long(value) => *value,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) as i64,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as i64,
        _ => return None,
    })
}

/// [`codec_int_value`] over `Codec.FLOAT`, which narrows the same `Number`
/// through `Number::floatValue` instead.
#[inline]
pub fn codec_float_value(value: &NbtSummaryValue) -> Option<f32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

#[inline]
pub fn codec_double_value(value: &NbtSummaryValue) -> Option<f64> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f64::from(*value),
        NbtSummaryValue::Short(value) => f64::from(*value),
        NbtSummaryValue::Int(value) => f64::from(*value),
        NbtSummaryValue::Long(value) => *value as f64,
        NbtSummaryValue::Float(bits) => f64::from(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits),
        _ => return None,
    })
}

pub fn codec_int_array<const N: usize>(value: &NbtSummaryValue) -> Option<[i32; N]> {
    match value {
        NbtSummaryValue::IntArray(values) => values.as_slice().try_into().ok(),
        NbtSummaryValue::List(values) if values.len() == N => {
            let mut words = [0; N];
            for (word, value) in words.iter_mut().zip(values) {
                *word = codec_int_value(value)?;
            }
            Some(words)
        }
        _ => None,
    }
}

pub fn uuid_from_words([first, second, third, fourth]: [i32; 4]) -> uuid::Uuid {
    let high = u64::from(first as u32) << 32 | u64::from(second as u32);
    let low = u64::from(third as u32) << 32 | u64::from(fourth as u32);
    uuid::Uuid::from_u64_pair(high, low)
}

pub fn uuid_int_array(uuid: uuid::Uuid) -> Vec<i32> {
    let (high, low) = uuid.as_u64_pair();
    vec![
        (high >> 32) as i32,
        high as i32,
        (low >> 32) as i32,
        low as i32,
    ]
}

pub fn uuid_nbt(uuid: uuid::Uuid) -> NbtSummaryValue {
    NbtSummaryValue::IntArray(uuid_int_array(uuid))
}

pub fn codec_uuid(value: &NbtSummaryValue) -> Option<uuid::Uuid> {
    codec_int_array(value).map(uuid_from_words)
}

/// Projects a `JsonOps` value into the primitive NBT shape used when a
/// persistent Vanilla codec is parsed from resource-pack JSON.
///
/// The conversion keeps integer and floating-point domains distinct and
/// applies the same depth/collection bounds as network NBT. Codec owners still
/// validate the resulting value; this helper only owns the shared ops bridge.
pub fn json_value_to_nbt_summary(value: &serde_json::Value) -> Result<NbtSummaryValue> {
    json_value_to_nbt_summary_at_depth(value, 0)
}

fn json_value_to_nbt_summary_at_depth(
    value: &serde_json::Value,
    depth: usize,
) -> Result<NbtSummaryValue> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "json-to-nbt value exceeded max depth".to_string(),
        ));
    }
    Ok(match value {
        serde_json::Value::Null => {
            return Err(ProtocolError::InvalidData(
                "null has no NBT value representation".to_string(),
            ));
        }
        serde_json::Value::Bool(value) => NbtSummaryValue::Byte(i8::from(*value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                match i32::try_from(value) {
                    Ok(value) => NbtSummaryValue::Int(value),
                    Err(_) => NbtSummaryValue::Long(value),
                }
            } else if let Some(value) = value.as_u64() {
                let value = i64::try_from(value).map_err(|_| {
                    ProtocolError::InvalidData(
                        "json integer exceeds the signed NBT domain".to_string(),
                    )
                })?;
                match i32::try_from(value) {
                    Ok(value) => NbtSummaryValue::Int(value),
                    Err(_) => NbtSummaryValue::Long(value),
                }
            } else {
                let value = value.as_f64().ok_or_else(|| {
                    ProtocolError::InvalidData(
                        "json number has no finite f64 representation".to_string(),
                    )
                })?;
                NbtSummaryValue::Double(value.to_bits())
            }
        }
        serde_json::Value::String(value) => NbtSummaryValue::string(value),
        serde_json::Value::Array(values) => {
            if values.len() > MAX_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    values.len(),
                    MAX_NBT_LIST_ITEMS,
                ));
            }
            NbtSummaryValue::List(
                values
                    .iter()
                    .map(|value| json_value_to_nbt_summary_at_depth(value, depth + 1))
                    .collect::<Result<_>>()?,
            )
        }
        serde_json::Value::Object(entries) => {
            if entries.len() > MAX_NBT_COMPOUND_ENTRIES {
                return Err(ProtocolError::PacketTooLarge(
                    entries.len(),
                    MAX_NBT_COMPOUND_ENTRIES,
                ));
            }
            NbtSummaryValue::Compound(
                entries
                    .iter()
                    .map(|(name, value)| {
                        Ok(NbtSummaryEntry {
                            name: name.clone().into(),
                            value: json_value_to_nbt_summary_at_depth(value, depth + 1)?,
                        })
                    })
                    .collect::<Result<_>>()?,
            )
        }
    })
}

/// The `JsonOps` view of a value, for a component a JSON-only packet carries
/// (`ClientboundLoginDisconnectPacket` writes `ComponentSerialization.CODEC`
/// through JSON). A non-finite float has no JSON form and becomes `null`.
pub fn nbt_summary_to_json_value(value: &NbtSummaryValue) -> serde_json::Value {
    use serde_json::Value as Json;
    let float = |value: f64| serde_json::Number::from_f64(value).map_or(Json::Null, Json::Number);
    match value {
        NbtSummaryValue::Byte(value) => Json::from(*value),
        NbtSummaryValue::Short(value) => Json::from(*value),
        NbtSummaryValue::Int(value) => Json::from(*value),
        NbtSummaryValue::Long(value) => Json::from(*value),
        NbtSummaryValue::Float(bits) => float(f64::from(f32::from_bits(*bits))),
        NbtSummaryValue::Double(bits) => float(f64::from_bits(*bits)),
        NbtSummaryValue::ByteArray(values) => values.iter().copied().map(Json::from).collect(),
        NbtSummaryValue::String(value) => Json::String(value.to_display_string()),
        NbtSummaryValue::List(values) => values.iter().map(nbt_summary_to_json_value).collect(),
        NbtSummaryValue::Compound(entries) => Json::Object(
            entries
                .iter()
                .map(|entry| {
                    (
                        entry.name.to_jdk_string(),
                        nbt_summary_to_json_value(&entry.value),
                    )
                })
                .collect(),
        ),
        NbtSummaryValue::IntArray(values) => values.iter().copied().map(Json::from).collect(),
        NbtSummaryValue::LongArray(values) => values.iter().copied().map(Json::from).collect(),
    }
}

pub(crate) fn decode_nbt_summary_from_decoder_with_limits(
    decoder: &mut Decoder<'_>,
    limits: &NbtSummaryDecodeLimits,
    max_accounted_bytes: u64,
) -> Result<Option<NbtSummaryValue>> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(None);
    }
    let mut accounter = NbtAccounter::new(max_accounted_bytes);
    read_nbt_payload_inner::<false>(decoder, tag_id, 0, limits, &mut 0, &mut accounter).map(Some)
}

pub(crate) fn decode_nbt_summary_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Option<NbtSummaryValue>> {
    decode_nbt_summary_from_decoder_with_limits(
        decoder,
        &NbtSummaryDecodeLimits::default(),
        VANILLA_DEFAULT_NBT_QUOTA,
    )
}

pub fn decode_nbt_summary_with_limits(
    bytes: &[u8],
    limits: NbtSummaryDecodeLimits,
    max_accounted_bytes: u64,
) -> Result<Option<NbtSummaryValue>> {
    let mut decoder = Decoder::new(bytes);
    let value =
        decode_nbt_summary_from_decoder_with_limits(&mut decoder, &limits, max_accounted_bytes)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after network nbt root".to_string(),
        ));
    }
    Ok(value)
}

pub fn decode_nbt_summary(bytes: &[u8]) -> Result<Option<NbtSummaryValue>> {
    decode_nbt_summary_with_limits(
        bytes,
        NbtSummaryDecodeLimits::default(),
        VANILLA_DEFAULT_NBT_QUOTA,
    )
}

/// Decodes the named-root NBT shape used by Vanilla files such as
/// `servers.dat`. Network NBT omits the root name and continues to use
/// [`decode_nbt_summary`].
pub fn decode_named_nbt_summary(bytes: &[u8]) -> Result<Option<(JavaString, NbtSummaryValue)>> {
    decode_named_nbt_summary_with_limits(
        bytes,
        NbtSummaryDecodeLimits::default(),
        VANILLA_DEFAULT_NBT_QUOTA,
        RootNamePolicy::Read,
    )
}

/// Decodes a save file's named root the way `NbtIo.read` with
/// `NbtAccounter.unlimitedHeap()` does: later duplicate keys win and only the
/// 512-level depth limit applies.
pub fn decode_named_saved_nbt_summary(
    bytes: &[u8],
) -> Result<Option<(JavaString, NbtSummaryValue)>> {
    decode_named_nbt_summary_with_limits(
        bytes,
        NbtSummaryDecodeLimits::SAVED_FILE,
        u64::MAX,
        RootNamePolicy::Read,
    )
}

/// Reads a file NBT header while ignoring the root name bytes as `NbtIo.readUnnamedTag` does.
pub fn decode_file_nbt_summary(bytes: &[u8]) -> Result<Option<NbtSummaryValue>> {
    decode_named_nbt_summary_with_limits(
        bytes,
        NbtSummaryDecodeLimits::default(),
        VANILLA_DEFAULT_NBT_QUOTA,
        RootNamePolicy::Skip,
    )
    .map(|value| value.map(|(_, root)| root))
}

/// [`decode_named_saved_nbt_summary`] without the root name.
pub fn decode_file_saved_nbt_summary(bytes: &[u8]) -> Result<Option<NbtSummaryValue>> {
    decode_named_nbt_summary_with_limits(
        bytes,
        NbtSummaryDecodeLimits::SAVED_FILE,
        u64::MAX,
        RootNamePolicy::Skip,
    )
    .map(|value| value.map(|(_, root)| root))
}

/// Reads the first file root with explicit budgets, as `NbtIo.readCompressed` does.
pub fn decode_file_nbt_summary_prefix(
    bytes: &[u8],
    limits: NbtSummaryDecodeLimits,
    mut max_nodes: usize,
) -> Result<Option<NbtSummaryValue>> {
    let mut accounter = NbtAccounter::UNLIMITED;
    read_named_nbt_summary_with_limits::<true>(
        &mut Decoder::new(bytes),
        &limits,
        RootNamePolicy::Skip,
        &mut max_nodes,
        &mut accounter,
    )
    .map(|value| value.map(|(_, root)| root))
}

fn decode_named_nbt_summary_with_limits(
    bytes: &[u8],
    limits: NbtSummaryDecodeLimits,
    max_accounted_bytes: u64,
    root_name_policy: RootNamePolicy,
) -> Result<Option<(JavaString, NbtSummaryValue)>> {
    let mut decoder = Decoder::new(bytes);
    let mut accounter = NbtAccounter::new(max_accounted_bytes);
    let value = read_named_nbt_summary_with_limits::<false>(
        &mut decoder,
        &limits,
        root_name_policy,
        &mut 0,
        &mut accounter,
    )?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after named nbt root".to_string(),
        ));
    }
    Ok(value)
}

fn read_named_nbt_summary_with_limits<const COUNT_NODES: bool>(
    decoder: &mut Decoder<'_>,
    limits: &NbtSummaryDecodeLimits,
    root_name_policy: RootNamePolicy,
    remaining: &mut usize,
    accounter: &mut NbtAccounter,
) -> Result<Option<(JavaString, NbtSummaryValue)>> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(None);
    }
    let name = match root_name_policy {
        RootNamePolicy::Read => read_modified_utf8(decoder)?,
        RootNamePolicy::Skip => {
            let len = usize::from(decoder.read_u16()?);
            decoder.read_exact(len, "nbt root name")?;
            JavaString::default()
        }
    };
    let value =
        read_nbt_payload_inner::<COUNT_NODES>(decoder, tag_id, 0, limits, remaining, accounter)?;
    Ok(Some((name, value)))
}

#[derive(Clone, Copy)]
enum RootNamePolicy {
    Read,
    Skip,
}

fn read_nbt_payload_inner<const COUNT_NODES: bool>(
    decoder: &mut Decoder<'_>,
    tag_id: u8,
    depth: usize,
    limits: &NbtSummaryDecodeLimits,
    remaining: &mut usize,
    accounter: &mut NbtAccounter,
) -> Result<NbtSummaryValue> {
    #[cfg(test)]
    crate::test_stack_budget::record();

    if depth > limits.max_depth {
        return Err(ProtocolError::InvalidData(
            "network nbt exceeded max depth".to_string(),
        ));
    }
    if COUNT_NODES {
        *remaining = remaining.checked_sub(1).ok_or_else(|| {
            ProtocolError::InvalidData("nbt tree exceeded node budget".to_string())
        })?;
    }

    match tag_id {
        1 => {
            accounter.account_bytes(9)?;
            Ok(NbtSummaryValue::Byte(decoder.read_i8()?))
        }
        2 => {
            accounter.account_bytes(10)?;
            Ok(NbtSummaryValue::Short(decoder.read_i16()?))
        }
        3 => {
            accounter.account_bytes(12)?;
            Ok(NbtSummaryValue::Int(decoder.read_i32()?))
        }
        4 => {
            accounter.account_bytes(16)?;
            Ok(NbtSummaryValue::Long(decoder.read_i64()?))
        }
        // Full-tree decoding follows TagType.load, whose FloatTag/DoubleTag
        // factories canonicalize either signed zero (`FloatTag.java:12-24`,
        // `DoubleTag.java:12-24`).
        5 => {
            accounter.account_bytes(12)?;
            Ok(NbtSummaryValue::Float(canonical_float_tag_bits(
                decoder.read_f32()?,
            )))
        }
        6 => {
            accounter.account_bytes(16)?;
            Ok(NbtSummaryValue::Double(canonical_double_tag_bits(
                decoder.read_f64()?,
            )))
        }
        7 => {
            accounter.account_bytes(24)?;
            let len = read_nbt_len(decoder)?;
            if len > limits.max_byte_array_items {
                return Err(ProtocolError::PacketTooLarge(
                    len,
                    limits.max_byte_array_items,
                ));
            }
            accounter.account_items(1, len)?;
            let raw = decoder.read_exact(len, "nbt byte array")?;
            let values = raw.iter().map(|&b| b as i8).collect();
            Ok(NbtSummaryValue::ByteArray(values))
        }
        8 => {
            accounter.account_bytes(36)?;
            let value = read_modified_utf8(decoder)?;
            accounter.account_items(2, value.units().len())?;
            Ok(NbtSummaryValue::String(value))
        }
        9 => {
            if depth >= limits.max_depth {
                return Err(ProtocolError::InvalidData(
                    "network nbt exceeded max depth".to_string(),
                ));
            }
            accounter.account_bytes(36)?;
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > limits.max_list_items {
                return Err(ProtocolError::PacketTooLarge(len, limits.max_list_items));
            }
            if COUNT_NODES && len > *remaining {
                return Err(ProtocolError::InvalidData(
                    "nbt list exceeded remaining node budget".to_string(),
                ));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty nbt list has end tag element type".to_string(),
                ));
            }
            accounter.account_items(4, len)?;
            let initial_cap =
                decoder.bounded_capacity(len, min_nbt_payload_wire_bytes(element_type));
            let mut items = Vec::with_capacity(initial_cap);
            for _ in 0..len {
                let value = read_nbt_payload_inner::<COUNT_NODES>(
                    decoder,
                    element_type,
                    depth + 1,
                    limits,
                    remaining,
                    accounter,
                )?;
                items.push(unwrap_list_element(value));
            }
            Ok(NbtSummaryValue::List(items))
        }
        10 => {
            if depth >= limits.max_depth {
                return Err(ProtocolError::InvalidData(
                    "network nbt exceeded max depth".to_string(),
                ));
            }
            accounter.account_bytes(48)?;
            let mut entries = CompoundEntries::new(Vec::new());
            let mut entry_count = 0usize;
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    return Ok(NbtSummaryValue::Compound(entries.entries));
                }
                if entry_count >= limits.max_compound_entries {
                    return Err(ProtocolError::PacketTooLarge(
                        entry_count + 1,
                        limits.max_compound_entries,
                    ));
                }
                entry_count += 1;
                let name = read_modified_utf8(decoder)?;
                // CompoundTag.java:130-131,60,108: charge key and map-entry overhead per entry
                // (including duplicates so repeated-key payloads cannot bypass the quota).
                accounter.account_bytes(28 + 36)?;
                accounter.account_items(2, name.units().len())?;
                let value = read_nbt_payload_inner::<COUNT_NODES>(
                    decoder,
                    nested_type,
                    depth + 1,
                    limits,
                    remaining,
                    accounter,
                )?;
                entries.push(NbtSummaryEntry { name, value });
            }
        }
        11 => {
            accounter.account_bytes(24)?;
            let len = read_nbt_len(decoder)?;
            if len > limits.max_int_array_items {
                return Err(ProtocolError::PacketTooLarge(
                    len,
                    limits.max_int_array_items,
                ));
            }
            accounter.account_items(4, len)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            let raw = decoder.read_exact(byte_len, "nbt int array")?;
            let values = raw
                .as_chunks::<4>()
                .0
                .iter()
                .copied()
                .map(i32::from_be_bytes)
                .collect();
            Ok(NbtSummaryValue::IntArray(values))
        }
        12 => {
            accounter.account_bytes(24)?;
            let len = read_nbt_len(decoder)?;
            if len > limits.max_long_array_items {
                return Err(ProtocolError::PacketTooLarge(
                    len,
                    limits.max_long_array_items,
                ));
            }
            accounter.account_items(8, len)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            let raw = decoder.read_exact(byte_len, "nbt long array")?;
            let values = raw
                .as_chunks::<8>()
                .0
                .iter()
                .copied()
                .map(i64::from_be_bytes)
                .collect();
            Ok(NbtSummaryValue::LongArray(values))
        }
        other => Err(ProtocolError::InvalidData(format!(
            "invalid network nbt tag id {other}"
        ))),
    }
}

const fn min_nbt_payload_wire_bytes(tag_id: u8) -> usize {
    match tag_id {
        1 | 10 => 1,
        2 | 8 => 2,
        3 | 5 | 7 | 11 | 12 => 4,
        9 => 5,
        4 | 6 => 8,
        _ => 1,
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(ProtocolError::NegativeLength(len));
    }
    Ok(len as usize)
}

/// A key or string read the way `DataInput.readUTF` reads it: every compound
/// key and every `StringTag` shares this decoder (`CompoundTag.java:128-129`,
/// `StringTag.java:20-24`), so a key keeps units no `String` can hold.
pub fn read_modified_utf8(decoder: &mut Decoder<'_>) -> Result<JavaString> {
    let len = decoder.read_u16()? as usize;
    let bytes = decoder.read_exact(len, "nbt string")?;
    if bytes.is_ascii() {
        return Ok(JavaString::from_ascii_bytes(bytes));
    }
    Ok(decode_modified_utf8_bytes(bytes)?.into())
}

fn decode_modified_utf8_bytes(bytes: &[u8]) -> Result<Vec<u16>> {
    let mut units = Vec::with_capacity(bytes.len());
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

    Ok(units)
}

pub(crate) fn skip_modified_utf8(decoder: &mut Decoder<'_>, validate: bool) -> Result<()> {
    let len = decoder.read_u16()? as usize;
    let bytes = decoder.read_exact(len, "nbt string")?;
    if !validate || bytes.is_ascii() {
        return Ok(());
    }
    let mut cursor = 0;
    while cursor < bytes.len() {
        let b0 = bytes[cursor];
        if b0 & 0x80 == 0 {
            cursor += 1;
        } else if b0 & 0xe0 == 0xc0 {
            if cursor + 1 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            if bytes[cursor + 1] & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            cursor += 2;
        } else if b0 & 0xf0 == 0xe0 {
            if cursor + 2 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            if bytes[cursor + 1] & 0xc0 != 0x80 || bytes[cursor + 2] & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            cursor += 3;
        } else {
            return Err(ProtocolError::InvalidData(
                "invalid modified utf-8 leading byte".to_string(),
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) struct SkipNbtLimits {
    pub(crate) max_depth: usize,
    pub(crate) max_list_items: usize,
    pub(crate) max_array_items: usize,
    pub(crate) max_compound_entries: usize,
    pub(crate) max_total_bytes: Option<usize>,
    pub(crate) validate_mutf8: bool,
    pub(crate) label: &'static str,
}

impl SkipNbtLimits {
    pub(crate) const NETWORK: Self = Self {
        max_depth: MAX_NBT_DEPTH,
        max_list_items: MAX_NBT_LIST_ITEMS,
        max_array_items: MAX_NBT_ARRAY_ITEMS,
        max_compound_entries: MAX_NBT_COMPOUND_ENTRIES,
        max_total_bytes: None,
        validate_mutf8: true,
        label: "network nbt",
    };
}

/// Zero-allocation network NBT payload validator and skipper shared by packet
/// decoders that retain raw NBT bytes or discard unused tag payloads.
pub(crate) fn skip_network_nbt_payload(
    decoder: &mut Decoder<'_>,
    tag_id: u8,
    limits: SkipNbtLimits,
) -> Result<()> {
    let start_pos = decoder.position().saturating_sub(1);
    skip_network_nbt_payload_inner(decoder, tag_id, 0, start_pos, limits)
}

/// Zero-allocation network NBT root tag skipper (accepts `TAG_End` as absent).
pub(crate) fn skip_network_nbt_tag(decoder: &mut Decoder<'_>, limits: SkipNbtLimits) -> Result<()> {
    let start_pos = decoder.position();
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(());
    }
    skip_network_nbt_payload_inner(decoder, tag_id, 0, start_pos, limits)
}

fn check_skip_nbt_bytes(
    decoder: &Decoder<'_>,
    start_pos: usize,
    limits: SkipNbtLimits,
) -> Result<()> {
    if let Some(max_bytes) = limits.max_total_bytes {
        let consumed = decoder.position().saturating_sub(start_pos);
        if consumed > max_bytes {
            return Err(ProtocolError::PacketTooLarge(consumed, max_bytes));
        }
    }
    Ok(())
}

fn skip_network_nbt_payload_inner(
    decoder: &mut Decoder<'_>,
    tag_id: u8,
    depth: usize,
    start_pos: usize,
    limits: SkipNbtLimits,
) -> Result<()> {
    #[cfg(test)]
    crate::test_stack_budget::record();

    if depth > limits.max_depth {
        return Err(ProtocolError::InvalidData(format!(
            "{} exceeded max depth",
            limits.label
        )));
    }

    match tag_id {
        1 => {
            decoder.read_exact(1, "nbt byte")?;
        }
        2 => {
            decoder.read_exact(2, "nbt short")?;
        }
        3 | 5 => {
            decoder.read_exact(4, "nbt int/float")?;
        }
        4 | 6 => {
            decoder.read_exact(8, "nbt long/double")?;
        }
        7 => {
            let len = read_nbt_len(decoder)?;
            if len > limits.max_array_items {
                return Err(ProtocolError::PacketTooLarge(len, limits.max_array_items));
            }
            if let Some(max_bytes) = limits.max_total_bytes {
                let next_len = decoder
                    .position()
                    .saturating_sub(start_pos)
                    .checked_add(len)
                    .ok_or_else(|| {
                        ProtocolError::InvalidData(format!("{} length overflow", limits.label))
                    })?;
                if next_len > max_bytes {
                    return Err(ProtocolError::PacketTooLarge(next_len, max_bytes));
                }
            }
            decoder.read_exact(len, "nbt byte array")?;
        }
        8 => {
            skip_modified_utf8(decoder, limits.validate_mutf8)?;
        }
        9 => {
            if depth >= limits.max_depth {
                return Err(ProtocolError::InvalidData(format!(
                    "{} exceeded max depth",
                    limits.label
                )));
            }
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > limits.max_list_items {
                return Err(ProtocolError::PacketTooLarge(len, limits.max_list_items));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(format!(
                    "non-empty {} list has end tag element type",
                    limits.label
                )));
            }
            check_skip_nbt_bytes(decoder, start_pos, limits)?;
            for _ in 0..len {
                skip_network_nbt_payload_inner(
                    decoder,
                    element_type,
                    depth + 1,
                    start_pos,
                    limits,
                )?;
            }
        }
        10 => {
            if depth >= limits.max_depth {
                return Err(ProtocolError::InvalidData(format!(
                    "{} exceeded max depth",
                    limits.label
                )));
            }
            let mut entries = 0usize;
            loop {
                let nested_type = decoder.read_u8()?;
                check_skip_nbt_bytes(decoder, start_pos, limits)?;
                if nested_type == 0 {
                    break;
                }
                entries += 1;
                if entries > limits.max_compound_entries {
                    return Err(ProtocolError::PacketTooLarge(
                        entries,
                        limits.max_compound_entries,
                    ));
                }
                skip_modified_utf8(decoder, limits.validate_mutf8)?;
                check_skip_nbt_bytes(decoder, start_pos, limits)?;
                skip_network_nbt_payload_inner(decoder, nested_type, depth + 1, start_pos, limits)?;
            }
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            if len > limits.max_array_items {
                return Err(ProtocolError::PacketTooLarge(len, limits.max_array_items));
            }
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            if let Some(max_bytes) = limits.max_total_bytes {
                let next_len = decoder
                    .position()
                    .saturating_sub(start_pos)
                    .checked_add(byte_len)
                    .ok_or_else(|| {
                        ProtocolError::InvalidData(format!("{} length overflow", limits.label))
                    })?;
                if next_len > max_bytes {
                    return Err(ProtocolError::PacketTooLarge(next_len, max_bytes));
                }
            }
            decoder.read_exact(byte_len, "nbt int array")?;
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            if len > limits.max_array_items {
                return Err(ProtocolError::PacketTooLarge(len, limits.max_array_items));
            }
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            if let Some(max_bytes) = limits.max_total_bytes {
                let next_len = decoder
                    .position()
                    .saturating_sub(start_pos)
                    .checked_add(byte_len)
                    .ok_or_else(|| {
                        ProtocolError::InvalidData(format!("{} length overflow", limits.label))
                    })?;
                if next_len > max_bytes {
                    return Err(ProtocolError::PacketTooLarge(next_len, max_bytes));
                }
            }
            decoder.read_exact(byte_len, "nbt long array")?;
        }
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid {} tag id {other}",
                limits.label
            )));
        }
    }
    check_skip_nbt_bytes(decoder, start_pos, limits)
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

/// Encodes one value as a network-NBT root: the tag id, no root name, then the
/// payload. This is the shape every in-packet NBT field carries, and the one a
/// producer outside this crate — a server building a block entity's update tag
/// — needs to hand back as raw bytes.
pub fn encode_nbt_summary(value: &NbtSummaryValue, max_len: usize) -> Result<Vec<u8>> {
    encode_nbt_summary_bounded(Some(value), max_len, MAX_NBT_DEPTH)
}

/// Encodes one network-NBT root under Vanilla's `NbtAccounter.defaultQuota()`
/// (`NbtAccounter.java:6-8,23-25`): 2 MiB total byte quota, 512 depth limit,
/// and no artificial 4096 compound entry cap.
pub fn encode_nbt_summary_network_quota(value: &NbtSummaryValue) -> Result<Vec<u8>> {
    let mut encoder = BoundedNbtEncoder::new(VANILLA_DEFAULT_NBT_QUOTA as usize);
    encoder.max_compound_entries = usize::MAX;
    encoder.write_u8(nbt_tag_id(value))?;
    encode_nbt_payload(&mut encoder, value, 0, VANILLA_DEFAULT_NBT_MAX_DEPTH)?;
    Ok(encoder.into_inner())
}

/// Encodes the named-root NBT shape used by Vanilla files while retaining the
/// same bounded collection and depth rules as protocol NBT.
pub fn encode_named_nbt_summary(
    root_name: &str,
    value: &NbtSummaryValue,
    max_len: usize,
) -> Result<Vec<u8>> {
    encode_named_nbt_summary_units(root_name.encode_utf16(), value, max_len)
}

pub fn encode_named_nbt_summary_java_string(
    root_name: &JavaString,
    value: &NbtSummaryValue,
    max_len: usize,
) -> Result<Vec<u8>> {
    encode_named_nbt_summary_units(root_name.units().iter().copied(), value, max_len)
}

pub fn encode_named_nbt_summary_saved_file(
    root_name: &str,
    value: &NbtSummaryValue,
    max_len: usize,
) -> Result<Vec<u8>> {
    encode_named_nbt_summary_units_with_limits(
        root_name.encode_utf16(),
        value,
        max_len,
        NbtSummaryDecodeLimits::SAVED_FILE.max_depth,
        NbtSummaryDecodeLimits::SAVED_FILE.max_compound_entries,
    )
}

pub fn encode_named_nbt_summary_java_string_saved_file(
    root_name: &JavaString,
    value: &NbtSummaryValue,
    max_len: usize,
) -> Result<Vec<u8>> {
    encode_named_nbt_summary_units_with_limits(
        root_name.units().iter().copied(),
        value,
        max_len,
        NbtSummaryDecodeLimits::SAVED_FILE.max_depth,
        NbtSummaryDecodeLimits::SAVED_FILE.max_compound_entries,
    )
}

fn encode_named_nbt_summary_units(
    root_name: impl Iterator<Item = u16> + Clone,
    value: &NbtSummaryValue,
    max_len: usize,
) -> Result<Vec<u8>> {
    encode_named_nbt_summary_units_with_limits(
        root_name,
        value,
        max_len,
        MAX_NBT_DEPTH,
        MAX_NBT_COMPOUND_ENTRIES,
    )
}

pub fn encode_named_nbt_summary_with_limits(
    root_name: &str,
    value: &NbtSummaryValue,
    max_len: usize,
    max_depth: usize,
    max_compound_entries: usize,
) -> Result<Vec<u8>> {
    encode_named_nbt_summary_units_with_limits(
        root_name.encode_utf16(),
        value,
        max_len,
        max_depth,
        max_compound_entries,
    )
}

fn encode_named_nbt_summary_units_with_limits(
    root_name: impl Iterator<Item = u16> + Clone,
    value: &NbtSummaryValue,
    max_len: usize,
    max_depth: usize,
    max_compound_entries: usize,
) -> Result<Vec<u8>> {
    let mut encoder = BoundedNbtEncoder::new(max_len);
    encoder.max_compound_entries = max_compound_entries;
    encoder.write_u8(nbt_tag_id(value))?;
    encode_modified_utf8_units(&mut encoder, root_name)?;
    // NbtIo.java:78-90; FloatTag.java:51-53; DoubleTag.java:51-53: DataOutputStream canonicalizes NaN bits.
    encoder.file_numbers = true;
    encode_nbt_payload(&mut encoder, value, 0, max_depth)?;
    Ok(encoder.into_inner())
}

fn encode_nbt_payload(
    encoder: &mut BoundedNbtEncoder,
    value: &NbtSummaryValue,
    depth: usize,
    max_depth: usize,
) -> Result<()> {
    if depth > max_depth {
        return Err(ProtocolError::InvalidData(format!(
            "network nbt exceeded max depth {max_depth}"
        )));
    }
    match value {
        NbtSummaryValue::Byte(value) => encoder.write_u8(*value as u8),
        NbtSummaryValue::Short(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::Int(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::Long(value) => encoder.write_bytes(&value.to_be_bytes()),
        NbtSummaryValue::Float(value) => {
            let bits = if encoder.file_numbers && f32::from_bits(*value).is_nan() {
                f32::NAN.to_bits()
            } else {
                *value
            };
            encoder.write_bytes(&bits.to_be_bytes())
        }
        NbtSummaryValue::Double(value) => {
            let bits = if encoder.file_numbers && f64::from_bits(*value).is_nan() {
                f64::NAN.to_bits()
            } else {
                *value
            };
            encoder.write_bytes(&bits.to_be_bytes())
        }
        NbtSummaryValue::ByteArray(values) => {
            encode_nbt_collection_len(encoder, values.len())?;
            encoder.reserve_additional(values.len())?;
            encoder
                .bytes
                .extend(values.iter().map(|&value| value as u8));
            Ok(())
        }
        NbtSummaryValue::String(value) => {
            encode_modified_utf8_units(encoder, value.units().iter().copied())
        }
        NbtSummaryValue::List(values) => {
            let nested_depth = enter_nbt_container(depth, max_depth)?;
            let element_type = list_wire_element_type(values);
            encoder.write_u8(element_type)?;
            encode_nbt_collection_len(encoder, values.len())?;
            for value in values {
                encode_nbt_list_element(encoder, value, element_type, nested_depth, max_depth)?;
            }
            Ok(())
        }
        NbtSummaryValue::Compound(entries) => {
            let nested_depth = enter_nbt_container(depth, max_depth)?;
            if entries.len() > encoder.max_compound_entries {
                return Err(ProtocolError::PacketTooLarge(
                    entries.len(),
                    encoder.max_compound_entries,
                ));
            }
            with_last_wins_compound_entries(entries, |entries| {
                for entry in entries {
                    encoder.write_u8(nbt_tag_id(&entry.value))?;
                    encode_modified_utf8_units(encoder, entry.name.units().iter().copied())?;
                    encode_nbt_payload(encoder, &entry.value, nested_depth, max_depth)?;
                }
                encoder.write_u8(0)
            })
        }
        NbtSummaryValue::IntArray(values) => {
            encode_nbt_collection_len(encoder, values.len())?;
            let byte_len = values.len().checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("network nbt encoded length overflow".to_string())
            })?;
            encoder.reserve_additional(byte_len)?;
            for &value in values {
                encoder.bytes.extend_from_slice(&value.to_be_bytes());
            }
            Ok(())
        }
        NbtSummaryValue::LongArray(values) => {
            encode_nbt_collection_len(encoder, values.len())?;
            let byte_len = values.len().checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("network nbt encoded length overflow".to_string())
            })?;
            encoder.reserve_additional(byte_len)?;
            for &value in values {
                encoder.bytes.extend_from_slice(&value.to_be_bytes());
            }
            Ok(())
        }
    }
}

pub fn clone_nbt_with_last_wins_compounds(value: &NbtSummaryValue) -> NbtSummaryValue {
    match value {
        NbtSummaryValue::List(values) => NbtSummaryValue::List(
            values
                .iter()
                .map(clone_nbt_with_last_wins_compounds)
                .collect(),
        ),
        NbtSummaryValue::Compound(entries) => with_last_wins_compound_entries(entries, |entries| {
            NbtSummaryValue::Compound(
                entries
                    .iter()
                    .map(|entry| NbtSummaryEntry {
                        name: entry.name.clone(),
                        value: clone_nbt_with_last_wins_compounds(&entry.value),
                    })
                    .collect(),
            )
        }),
        _ => value.clone(),
    }
}

pub(crate) fn codec_values_equal(left: &NbtSummaryValue, right: &NbtSummaryValue) -> bool {
    match (left, right) {
        (NbtSummaryValue::List(left), NbtSummaryValue::List(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(a, b)| codec_values_equal(a, b))
        }
        (NbtSummaryValue::Compound(left), NbtSummaryValue::Compound(right)) => {
            with_last_wins_compound_entries(left, |left| {
                with_last_wins_compound_entries(right, |right| {
                    left.len() == right.len() && {
                        left.sort_unstable_by(|a, b| a.name.cmp(&b.name));
                        right.sort_unstable_by(|a, b| a.name.cmp(&b.name));
                        left.iter().zip(right.iter()).all(|(a, b)| {
                            a.name == b.name && codec_values_equal(&a.value, &b.value)
                        })
                    }
                })
            })
        }
        _ => left == right,
    }
}

const LINEAR_COMPOUND_ENTRIES: usize = 32;

struct CompoundEntries<T: Borrow<NbtSummaryEntry>> {
    entries: Vec<T>,
    positions: Option<(HashTable<usize>, RandomState)>,
}

impl<T: Borrow<NbtSummaryEntry>> CompoundEntries<T> {
    fn new(entries: Vec<T>) -> Self {
        Self {
            entries,
            positions: None,
        }
    }

    fn push(&mut self, incoming: T) {
        if self.entries.len() < LINEAR_COMPOUND_ENTRIES {
            if let Some(existing) = find_compound_entry(&mut self.entries, &incoming.borrow().name)
            {
                *existing = incoming;
            } else {
                self.entries.push(incoming);
            }
            return;
        }
        let name = &incoming.borrow().name;
        let (positions, hasher) = self.positions.get_or_insert_with(|| {
            let hasher = RandomState::new();
            // Cover the Vec's next growth without rehashing before it fills.
            let capacity = self.entries.capacity().max(self.entries.len() * 2);
            let mut positions = HashTable::with_capacity(capacity);
            for (index, entry) in self.entries.iter().enumerate() {
                positions.insert_unique(hasher.hash_one(&entry.borrow().name), index, |index| {
                    hasher.hash_one(&self.entries[*index].borrow().name)
                });
            }
            (positions, hasher)
        });
        match positions.entry(
            hasher.hash_one(name),
            |index| self.entries[*index].borrow().name == *name,
            |index| hasher.hash_one(&self.entries[*index].borrow().name),
        ) {
            Entry::Occupied(position) => self.entries[*position.get()] = incoming,
            Entry::Vacant(position) => {
                position.insert(self.entries.len());
                self.entries.push(incoming);
            }
        }
    }
}

fn unwrap_list_element(value: NbtSummaryValue) -> NbtSummaryValue {
    let NbtSummaryValue::Compound(mut entries) = value else {
        return value;
    };
    if entries.len() == 1 && entries[0].name.is_empty() {
        entries.pop().expect("one list wrapper entry").value
    } else {
        NbtSummaryValue::Compound(entries)
    }
}

fn list_wire_element_type(values: &[NbtSummaryValue]) -> u8 {
    let Some(first) = values.first() else {
        return 0;
    };
    let first_type = nbt_tag_id(first);
    if values.iter().all(|value| nbt_tag_id(value) == first_type) {
        first_type
    } else {
        10
    }
}

fn encode_nbt_list_element(
    encoder: &mut BoundedNbtEncoder,
    value: &NbtSummaryValue,
    element_type: u8,
    depth: usize,
    max_depth: usize,
) -> Result<()> {
    if element_type != 10 || is_unwrapped_list_compound(value) {
        return encode_nbt_payload(encoder, value, depth, max_depth);
    }

    let wrapper_depth = enter_nbt_container(depth, max_depth)?;
    encoder.write_u8(nbt_tag_id(value))?;
    encode_modified_utf8_units(encoder, std::iter::empty())?;
    encode_nbt_payload(encoder, value, wrapper_depth, max_depth)?;
    encoder.write_u8(0)
}

fn is_unwrapped_list_compound(value: &NbtSummaryValue) -> bool {
    matches!(
        value,
        NbtSummaryValue::Compound(entries)
            if !(entries.len() == 1 && entries[0].name.is_empty())
    )
}

fn find_compound_entry<'a, T: Borrow<NbtSummaryEntry>>(
    entries: &'a mut [T],
    name: &JavaString,
) -> Option<&'a mut T> {
    entries
        .iter_mut()
        .find(|entry| T::borrow(entry).name == *name)
}

pub fn with_last_wins_compound_entries<'a, R>(
    entries: &'a [NbtSummaryEntry],
    consume: impl FnOnce(&mut [&'a NbtSummaryEntry]) -> R,
) -> R {
    if entries.len() > LINEAR_COMPOUND_ENTRIES {
        return consume(&mut last_wins_wide_compound_entries(entries));
    }
    let Some(first) = entries.first() else {
        return consume(&mut []);
    };
    let mut normalized = [first; LINEAR_COMPOUND_ENTRIES];
    let mut len = 0;
    for entry in entries {
        if let Some(existing) = find_compound_entry(&mut normalized[..len], &entry.name) {
            *existing = entry;
        } else {
            normalized[len] = entry;
            len += 1;
        }
    }
    consume(&mut normalized[..len])
}

// Keep index scratch out of small-compound normalization.
#[inline(never)]
fn last_wins_wide_compound_entries(entries: &[NbtSummaryEntry]) -> Vec<&NbtSummaryEntry> {
    let mut normalized = CompoundEntries::new(Vec::with_capacity(entries.len()));
    for entry in entries {
        normalized.push(entry);
    }
    normalized.entries
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

fn encode_modified_utf8_units(
    encoder: &mut BoundedNbtEncoder,
    units: impl Iterator<Item = u16> + Clone,
) -> Result<()> {
    let encoded_len = units.clone().try_fold(0usize, |len, unit| {
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
    encoder.reserve_additional(2 + encoded_len)?;
    encoder
        .bytes
        .extend_from_slice(&encoded_len_u16.to_be_bytes());
    for unit in units {
        if unit == 0 {
            encoder.bytes.extend_from_slice(&[0xc0, 0x80]);
        } else if unit <= 0x7f {
            encoder.bytes.push(unit as u8);
        } else if unit <= 0x7ff {
            encoder.bytes.extend_from_slice(&[
                (0xc0 | ((unit >> 6) & 0x1f)) as u8,
                (0x80 | (unit & 0x3f)) as u8,
            ]);
        } else {
            encoder.bytes.extend_from_slice(&[
                (0xe0 | ((unit >> 12) & 0x0f)) as u8,
                (0x80 | ((unit >> 6) & 0x3f)) as u8,
                (0x80 | (unit & 0x3f)) as u8,
            ]);
        }
    }
    Ok(())
}

struct BoundedNbtEncoder {
    bytes: Vec<u8>,
    max_len: usize,
    file_numbers: bool,
    max_compound_entries: usize,
}

impl BoundedNbtEncoder {
    fn new(max_len: usize) -> Self {
        Self {
            bytes: Vec::new(),
            max_len,
            file_numbers: false,
            max_compound_entries: MAX_NBT_COMPOUND_ENTRIES,
        }
    }

    fn reserve_additional(&mut self, additional: usize) -> Result<()> {
        let next_len = self.bytes.len().checked_add(additional).ok_or_else(|| {
            ProtocolError::InvalidData("network nbt encoded length overflow".to_string())
        })?;
        if next_len > self.max_len {
            return Err(ProtocolError::PacketTooLarge(next_len, self.max_len));
        }
        self.bytes.reserve(additional);
        Ok(())
    }

    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.write_bytes(&[value])
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.reserve_additional(bytes.len())?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod file_number_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_root_keeps_java_units_and_file_reader_skips_name_bytes() {
        let root = NbtSummaryValue::Compound(Vec::new());
        for units in [
            vec![0xd800],
            vec![0xd801],
            vec![0xfffd],
            vec![b'?' as u16],
            "plain\0😀".encode_utf16().collect(),
        ] {
            let name = JavaString::from(units.clone());
            assert_eq!(name.units(), units);
            let encoded = encode_named_nbt_summary_java_string(&name, &root, 128).unwrap();
            assert_eq!(
                decode_named_nbt_summary(&encoded).unwrap(),
                Some((name.clone(), root.clone()))
            );
            assert_eq!(
                decode_named_saved_nbt_summary(&encoded).unwrap(),
                Some((name, root.clone()))
            );
            assert_eq!(
                decode_file_nbt_summary(&encoded).unwrap(),
                Some(root.clone())
            );
            assert_eq!(
                decode_file_saved_nbt_summary(&encoded).unwrap(),
                Some(root.clone())
            );
        }
        let normal = encode_named_nbt_summary("plain\0😀", &root, 128).unwrap();
        assert_eq!(
            decode_named_nbt_summary(&normal).unwrap().unwrap().0,
            "plain\0😀"
        );
    }

    #[test]
    fn file_root_skip_does_not_validate_utf8_but_keeps_bounds_and_payload_checks() {
        let malformed_name = [10, 0, 2, 0xff, 0xff, 0];
        assert_eq!(
            decode_file_saved_nbt_summary(&malformed_name).unwrap(),
            Some(NbtSummaryValue::Compound(Vec::new()))
        );
        assert!(decode_named_saved_nbt_summary(&malformed_name).is_err());
        assert!(decode_file_saved_nbt_summary(&malformed_name[..4]).is_err());
        let mut trailing = malformed_name.to_vec();
        trailing.push(1);
        assert!(decode_file_saved_nbt_summary(&trailing).is_err());
        let duplicate = [10, 0, 2, 0xff, 0xff, 1, 0, 1, b'a', 1, 1, 0, 1, b'a', 2, 0];
        assert_eq!(
            decode_file_saved_nbt_summary(&duplicate).unwrap(),
            Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "a".into(),
                value: NbtSummaryValue::Byte(2)
            },]))
        );
    }

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

    #[test]
    fn saved_file_decoder_has_no_array_size_cap() {
        let mut bytes = vec![10, 0, 0, 7, 0, 6];
        bytes.extend_from_slice(b"colors");
        bytes.extend_from_slice(&(16_384_i32).to_be_bytes());
        bytes.extend(std::iter::repeat_n(0, 16_384));
        bytes.push(0);

        let decoded = decode_named_saved_nbt_summary(&bytes).unwrap();
        let Some((_, NbtSummaryValue::Compound(entries))) = decoded else {
            panic!("expected a named compound root");
        };
        assert!(matches!(
            entries.as_slice(),
            [NbtSummaryEntry {
                name,
                value: NbtSummaryValue::ByteArray(values),
            }] if name == "colors" && values.len() == 16_384
        ));
    }

    #[test]
    fn full_tree_float_load_normalizes_zero_and_preserves_nan_payloads() {
        for (tag, negative_zero, positive_zero, nan) in [
            (
                5,
                (-0.0_f32).to_bits().to_be_bytes().to_vec(),
                NbtSummaryValue::Float(0.0_f32.to_bits()),
                0xffc0_1234_u32.to_be_bytes().to_vec(),
            ),
            (
                6,
                (-0.0_f64).to_bits().to_be_bytes().to_vec(),
                NbtSummaryValue::Double(0.0_f64.to_bits()),
                0xfff8_0000_0000_1234_u64.to_be_bytes().to_vec(),
            ),
        ] {
            let mut zero_wire = vec![tag];
            zero_wire.extend_from_slice(&negative_zero);
            assert_eq!(decode_nbt_summary(&zero_wire).unwrap(), Some(positive_zero));

            let mut nan_wire = vec![tag];
            nan_wire.extend_from_slice(&nan);
            let expected_nan = if tag == 5 {
                NbtSummaryValue::Float(u32::from_be_bytes(nan.try_into().unwrap()))
            } else {
                NbtSummaryValue::Double(u64::from_be_bytes(nan.try_into().unwrap()))
            };
            assert_eq!(decode_nbt_summary(&nan_wire).unwrap(), Some(expected_nan));
        }
    }

    #[test]
    fn heterogeneous_network_lists_use_vanilla_compound_wrappers() {
        // Vanilla 26.1 ListTag writes a heterogeneous list as compounds. A
        // non-compound element is wrapped under the empty-string marker and
        // ListTag.load removes that marker again.
        let value = NbtSummaryValue::List(vec![
            NbtSummaryValue::String("a".into()),
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::String("b".into()),
            }]),
        ]);
        let encoded = encode_nbt_summary_bounded(Some(&value), 1024, MAX_NBT_DEPTH).unwrap();
        assert_eq!(
            encoded,
            vec![
                9, 10, 0, 0, 0, 2, // list<compound>, two elements
                8, 0, 0, 0, 1, b'a', 0, // {"": "a"}
                8, 0, 4, b't', b'e', b'x', b't', 0, 1, b'b', 0,
            ]
        );
        assert_eq!(decode_nbt_summary(&encoded).unwrap(), Some(value));
    }

    #[test]
    fn compound_list_element_equal_to_wrapper_marker_is_double_wrapped() {
        let value = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: String::new().into(),
            value: NbtSummaryValue::String("literal compound".into()),
        }])]);
        let encoded = encode_nbt_summary_bounded(Some(&value), 1024, MAX_NBT_DEPTH).unwrap();
        assert_eq!(decode_nbt_summary(&encoded).unwrap(), Some(value));
    }

    #[test]
    fn network_compounds_decode_and_encode_duplicate_keys_last_wins() {
        let value = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "key".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "key".into(),
                value: NbtSummaryValue::String("last".into()),
            },
        ]);
        let encoded = encode_nbt_summary_bounded(Some(&value), 1024, MAX_NBT_DEPTH).unwrap();
        assert_eq!(
            decode_nbt_summary(&encoded).unwrap(),
            Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "key".into(),
                value: NbtSummaryValue::String("last".into()),
            }]))
        );

        let mut duplicate_wire = vec![10, 3, 0, 3, b'k', b'e', b'y'];
        duplicate_wire.extend_from_slice(&1i32.to_be_bytes());
        duplicate_wire.extend_from_slice(&[8, 0, 3, b'k', b'e', b'y', 0, 4]);
        duplicate_wire.extend_from_slice(b"last");
        duplicate_wire.push(0);
        assert_eq!(
            decode_nbt_summary(&duplicate_wire).unwrap(),
            decode_nbt_summary(&encoded).unwrap()
        );

        let mut large_wire = vec![10];
        let mut expected = Vec::new();
        for index in 0_i32..64 {
            let name = format!("key{index}");
            large_wire.push(3);
            large_wire.extend_from_slice(&(name.len() as u16).to_be_bytes());
            large_wire.extend_from_slice(name.as_bytes());
            large_wire.extend_from_slice(&index.to_be_bytes());
            expected.push(NbtSummaryEntry {
                name: name.into(),
                value: NbtSummaryValue::Int(index),
            });
        }
        let mut large_raw_keys = large_wire.clone();
        large_wire.extend_from_slice(&[3, 0, 4, b'k', b'e', b'y', b'0', 0, 0, 0, 99, 0]);
        expected[0].value = NbtSummaryValue::Int(99);
        let expected = Some(NbtSummaryValue::Compound(expected));
        assert_eq!(decode_nbt_summary(&large_wire).unwrap(), expected);
        let mut file = vec![10, 0, 0];
        file.extend_from_slice(&large_wire[1..]);
        file.push(99); // file loading stops after the root
        let limits = NbtSummaryDecodeLimits::default();
        assert_eq!(
            decode_file_nbt_summary_prefix(&file, limits, 66).unwrap(),
            expected
        );
        assert!(decode_file_nbt_summary_prefix(&file, limits, 65).is_err());

        let mut raw_keys = vec![10];
        let raw_entries = [
            ([0xed, 0xa0, 0x80], 1_i32),
            ([0xed, 0xa0, 0x81], 2),
            ([0xef, 0xbf, 0xbd], 3),
        ];
        for (key, value) in raw_entries {
            raw_keys.extend_from_slice(&[3, 0, 3]);
            raw_keys.extend_from_slice(&key);
            raw_keys.extend_from_slice(&value.to_be_bytes());
        }
        raw_keys.push(0);
        let decoded = decode_nbt_summary(&raw_keys).unwrap().unwrap();
        let NbtSummaryValue::Compound(entries) = &decoded else {
            panic!("compound expected");
        };
        assert_eq!(entries.len(), 3);
        for (entry, (unit, value)) in entries.iter().zip([(0xd800, 1), (0xd801, 2), (0xfffd, 3)]) {
            assert_eq!(entry.name.units(), &[unit]);
            assert_eq!(entry.value, NbtSummaryValue::Int(value));
        }
        assert_eq!(encode_nbt_summary(&decoded, 128).unwrap(), raw_keys);
        let mut large_entries: Vec<_> = (0..64)
            .map(|index| NbtSummaryEntry {
                name: format!("key{index}").into(),
                value: NbtSummaryValue::Int(index),
            })
            .collect();
        large_entries.extend_from_slice(entries);
        let mut normalized = large_entries.clone();
        for (index, entry) in entries.iter().enumerate() {
            let replacement = NbtSummaryEntry {
                name: entry.name.clone(),
                value: NbtSummaryValue::Int(index as i32 + 4),
            };
            normalized[64 + index] = replacement.clone();
            large_entries.push(replacement);
        }
        large_raw_keys.extend_from_slice(&raw_keys[1..raw_keys.len() - 1]);
        for (key, value) in raw_entries {
            large_raw_keys.extend_from_slice(&[3, 0, 3]);
            large_raw_keys.extend_from_slice(&key);
            large_raw_keys.extend_from_slice(&(value + 3).to_be_bytes());
        }
        large_raw_keys.push(0);
        let Some(NbtSummaryValue::Compound(entries)) = decode_nbt_summary(&large_raw_keys).unwrap()
        else {
            panic!("compound expected");
        };
        assert_eq!(entries, normalized);
        let normalized = NbtSummaryValue::Compound(normalized);
        let large = NbtSummaryValue::Compound(large_entries);
        assert_eq!(
            encode_nbt_summary(&large, 4096).unwrap(),
            encode_nbt_summary(&normalized, 4096).unwrap()
        );
        assert_eq!(
            canonical_snbt_java_string(&large),
            canonical_snbt_java_string(&normalized)
        );
        assert_eq!(
            parse_snbt_compound_utf16(canonical_snbt_java_string(&large).units()).unwrap(),
            normalized
        );
        let mut named = vec![10, 0, 0];
        named.extend_from_slice(&raw_keys[1..]);
        assert_eq!(
            decode_named_saved_nbt_summary(&named).unwrap(),
            Some(("".into(), decoded))
        );
        for (bytes, unit) in [
            ([0xed, 0xa0, 0x80], 0xd800),
            ([0xed, 0xa0, 0x81], 0xd801),
            ([0xef, 0xbf, 0xbd], 0xfffd),
        ] {
            let wire = [8, 0, 3, bytes[0], bytes[1], bytes[2]];
            let decoded = decode_nbt_summary(&wire).unwrap().unwrap();
            assert_eq!(JavaString::from_nbt(&decoded).unwrap().units(), &[unit]);
            assert_eq!(encode_nbt_summary(&decoded, 128).unwrap(), wire);
        }
    }

    #[test]
    fn named_nbt_root_roundtrips_vanilla_file_shape() {
        let value = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "servers".into(),
            value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "name".into(),
                    value: NbtSummaryValue::String("Native".into()),
                },
                NbtSummaryEntry {
                    name: "hidden".into(),
                    value: NbtSummaryValue::Byte(0),
                },
            ])]),
        }]);
        let encoded = encode_named_nbt_summary("", &value, 4096).unwrap();
        assert_eq!(
            decode_named_nbt_summary(&encoded).unwrap(),
            Some(("".into(), value))
        );
    }

    #[test]
    fn named_nbt_root_rejects_trailing_bytes() {
        assert!(decode_named_nbt_summary(&[10, 0, 0, 0, 1])
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after named nbt root"));
    }

    #[test]
    fn duplicate_network_compound_keys_still_count_toward_the_entry_limit() {
        let mut bytes = vec![10];
        for _ in 0..=MAX_NBT_COMPOUND_ENTRIES {
            bytes.extend_from_slice(&[1, 0, 3, b'k', b'e', b'y', 0]);
        }
        bytes.push(0);
        assert!(matches!(
            decode_nbt_summary(&bytes),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_NBT_COMPOUND_ENTRIES + 1 && max == MAX_NBT_COMPOUND_ENTRIES
        ));
    }

    #[test]
    fn json_ops_bridge_preserves_numeric_domains_and_bounds_collections() {
        let value = serde_json::json!({
            "integer": 42,
            "long": 4_294_967_296_u64,
            "fraction": 1.5,
            "enabled": true,
            "items": ["a", "b"]
        });
        assert_eq!(
            json_value_to_nbt_summary(&value).unwrap(),
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "enabled".into(),
                    value: NbtSummaryValue::Byte(1),
                },
                NbtSummaryEntry {
                    name: "fraction".into(),
                    value: NbtSummaryValue::Double(1.5_f64.to_bits()),
                },
                NbtSummaryEntry {
                    name: "integer".into(),
                    value: NbtSummaryValue::Int(42),
                },
                NbtSummaryEntry {
                    name: "items".into(),
                    value: NbtSummaryValue::List(vec![
                        NbtSummaryValue::String("a".into()),
                        NbtSummaryValue::String("b".into()),
                    ]),
                },
                NbtSummaryEntry {
                    name: "long".into(),
                    value: NbtSummaryValue::Long(4_294_967_296),
                },
            ])
        );
        assert!(json_value_to_nbt_summary(&serde_json::Value::Null).is_err());
        assert!(matches!(
            json_value_to_nbt_summary(&serde_json::Value::Array(vec![
                serde_json::Value::Null;
                MAX_NBT_LIST_ITEMS + 1
            ])),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_NBT_LIST_ITEMS + 1 && max == MAX_NBT_LIST_ITEMS
        ));
    }

    #[test]
    fn compound_multiset_equality_handles_reordered_4096_entries_in_milliseconds() {
        let forward: Vec<NbtSummaryEntry> = (0..MAX_NBT_COMPOUND_ENTRIES)
            .map(|index| NbtSummaryEntry {
                name: format!("entry_{index:04}").into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "id".into(),
                        value: NbtSummaryValue::Int(index as i32),
                    },
                    NbtSummaryEntry {
                        name: "flag".into(),
                        value: NbtSummaryValue::Byte((index & 1) as i8),
                    },
                ]),
            })
            .collect();
        let mut reordered: Vec<NbtSummaryEntry> = forward
            .iter()
            .rev()
            .map(|entry| {
                let NbtSummaryValue::Compound(inner) = &entry.value else {
                    unreachable!();
                };
                NbtSummaryEntry {
                    name: entry.name.clone(),
                    value: NbtSummaryValue::Compound(inner.iter().rev().cloned().collect()),
                }
            })
            .collect();

        let left = NbtSummaryValue::Compound(forward);
        let right = NbtSummaryValue::Compound(reordered.clone());

        let started = std::time::Instant::now();
        assert_eq!(left, right);
        if let NbtSummaryValue::Compound(inner) = &mut reordered[2048].value {
            inner[0].value = NbtSummaryValue::Byte(99);
        }
        let modified = NbtSummaryValue::Compound(reordered);
        assert_ne!(left, modified);
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(200),
            "4096-entry reordered compound comparison took {elapsed:?}"
        );

        let dup_a = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "a".into(),
                        value: NbtSummaryValue::Int(1),
                    },
                    NbtSummaryEntry {
                        name: "b".into(),
                        value: NbtSummaryValue::Int(2),
                    },
                ]),
            },
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "b".into(),
                        value: NbtSummaryValue::Int(3),
                    },
                    NbtSummaryEntry {
                        name: "a".into(),
                        value: NbtSummaryValue::Int(1),
                    },
                ]),
            },
        ]);
        let dup_b = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "a".into(),
                        value: NbtSummaryValue::Int(1),
                    },
                    NbtSummaryEntry {
                        name: "b".into(),
                        value: NbtSummaryValue::Int(3),
                    },
                ]),
            },
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "b".into(),
                        value: NbtSummaryValue::Int(2),
                    },
                    NbtSummaryEntry {
                        name: "a".into(),
                        value: NbtSummaryValue::Int(1),
                    },
                ]),
            },
        ]);
        let dup_mismatch = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Int(1),
            },
        ]);
        let dup_mismatch_other = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "dup".into(),
                value: NbtSummaryValue::Int(2),
            },
        ]);
        assert_eq!(dup_a, dup_b);
        assert_ne!(dup_mismatch, dup_mismatch_other);
    }

    #[test]
    fn saved_file_named_encode_supports_large_compounds_and_deep_nesting_round_trip() {
        let wide = NbtSummaryValue::Compound(
            (0..4200)
                .map(|idx| entry(format!("k_{idx}"), NbtSummaryValue::Int(idx)))
                .collect(),
        );
        assert!(encode_named_nbt_summary("", &wide, usize::MAX).is_err());
        let wide_bytes = encode_named_nbt_summary_saved_file("", &wide, usize::MAX)
            .expect("saved file encode wide compound");
        let (decoded_name, decoded_wide) = decode_named_saved_nbt_summary(&wide_bytes)
            .expect("saved file decode")
            .unwrap();
        assert_eq!(decoded_name, JavaString::from(""));
        assert_eq!(decoded_wide, wide);

        let mut deep = NbtSummaryValue::Int(7);
        for _ in 0..80 {
            deep = NbtSummaryValue::Compound(vec![entry("inner", deep)]);
        }
        assert!(encode_named_nbt_summary("", &deep, usize::MAX).is_err());
        let root_name = JavaString::from("level_root");
        let deep_bytes =
            encode_named_nbt_summary_java_string_saved_file(&root_name, &deep, usize::MAX)
                .expect("saved file encode deep compound");
        let (decoded_name, decoded_deep) = decode_named_saved_nbt_summary(&deep_bytes)
            .expect("saved file decode")
            .unwrap();
        assert_eq!(decoded_name, root_name);
        assert_eq!(decoded_deep, deep);
    }

    #[test]
    fn network_nbt_enforces_vanilla_accounted_byte_quota_and_registry_limits() {
        // Build a compact heap-amplification payload: 64 lists of 1000 empty compounds.
        // Wire size is ~64 KiB, but accounted heap bytes exceed 2 MiB (~64 * 1000 * 52 = 3.3 MiB).
        let mut bomb = Vec::new();
        bomb.push(9); // outer List
        bomb.push(9); // element type = List
        bomb.extend_from_slice(&64_i32.to_be_bytes());
        for _ in 0..64 {
            bomb.push(10); // element type = Compound
            bomb.extend_from_slice(&1000_i32.to_be_bytes());
            bomb.extend(std::iter::repeat_n(0_u8, 1000));
        }
        let err = decode_nbt_summary(&bomb).unwrap_err();
        assert!(err.to_string().contains("exceeded byte quota"), "got {err}");

        // NETWORK_QUOTA matches ClientboundRegistryDataPacket: allows >4096 entries and depth >64
        // (up to 512) while staying inside the 2 MiB accounted quota.
        let mut wide_raw = Vec::new();
        wide_raw.push(10);
        for idx in 0..4200u16 {
            wide_raw.push(1);
            let key = format!("k{idx}");
            wide_raw.extend_from_slice(&(key.len() as u16).to_be_bytes());
            wide_raw.extend_from_slice(key.as_bytes());
            wide_raw.push(1);
        }
        wide_raw.push(0);
        assert!(decode_nbt_summary(&wide_raw).is_err());
        let decoded = decode_nbt_summary_with_limits(
            &wide_raw,
            NbtSummaryDecodeLimits::NETWORK_QUOTA,
            VANILLA_DEFAULT_NBT_QUOTA,
        )
        .expect("NETWORK_QUOTA accepts >4096 entries within 2 MiB")
        .unwrap();
        let NbtSummaryValue::Compound(entries) = decoded else {
            panic!("expected compound");
        };
        assert_eq!(entries.len(), 4200);

        let mut deep_raw = vec![10_u8]; // root TAG_Compound (depth 0)
        for _ in 0..80 {
            deep_raw.push(10); // nested TAG_Compound entry
            deep_raw.extend_from_slice(&1u16.to_be_bytes());
            deep_raw.push(b'a');
        }
        deep_raw.push(1); // leaf TAG_Byte entry
        deep_raw.extend_from_slice(&1u16.to_be_bytes());
        deep_raw.push(b'v');
        deep_raw.push(7);
        deep_raw.extend(std::iter::repeat_n(0_u8, 81));
        assert!(decode_nbt_summary(&deep_raw).is_err());
        assert!(decode_nbt_summary_with_limits(
            &deep_raw,
            NbtSummaryDecodeLimits::NETWORK_QUOTA,
            VANILLA_DEFAULT_NBT_QUOTA,
        )
        .is_ok());
    }
}
