//! Pure `HashOps.CRC32C_INSTANCE` digest primitives and shared encoder backend.
//!
//! Anchors:
//! - `HashOps` (CRC32C tags, UTF-16 string hashing, and unsigned
//!   `HashCode::padToLong` map entry ordering).

use std::collections::BTreeMap;

use bbb_facts::sound_event_types::vanilla_sound_event_resource_id;
use bbb_protocol::packets::SoundEventSummary;
use bbb_protocol::{JavaString, NbtSummaryValue};

use crate::persistent_item_stack_projection::{
    consumable_components::ConsumableComponentEncoder,
    encoding::{ComponentEncoder, ComponentScalar},
    weapon_and_block_attacks::WeaponComponentEncoder,
};

use super::ItemHashLookup;

pub const HASH_TAG_EMPTY: u8 = 1;
pub const HASH_TAG_MAP_START: u8 = 2;
pub const HASH_TAG_MAP_END: u8 = 3;
pub const HASH_TAG_LIST_START: u8 = 4;
pub const HASH_TAG_LIST_END: u8 = 5;
pub const HASH_TAG_BYTE: u8 = 6;
pub const HASH_TAG_SHORT: u8 = 7;
pub const HASH_TAG_INT: u8 = 8;
pub const HASH_TAG_LONG: u8 = 9;
pub const HASH_TAG_FLOAT: u8 = 10;
pub const HASH_TAG_DOUBLE: u8 = 11;
pub const HASH_TAG_STRING: u8 = 12;
pub const HASH_TAG_BOOLEAN: u8 = 13;
pub const HASH_TAG_BYTE_ARRAY_START: u8 = 14;
pub const HASH_TAG_BYTE_ARRAY_END: u8 = 15;
pub const HASH_TAG_INT_ARRAY_START: u8 = 16;
pub const HASH_TAG_INT_ARRAY_END: u8 = 17;
pub const HASH_TAG_LONG_ARRAY_START: u8 = 18;
pub const HASH_TAG_LONG_ARRAY_END: u8 = 19;

const CRC32C_POLY: u32 = 0x82F6_3B78;

const CRC32C_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if (crc & 1) != 0 {
                (crc >> 1) ^ CRC32C_POLY
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

/// Guava `Hashing.crc32c()` digest over `bytes`. Pure Rust so native and `wasm32`
/// produce identical `u32` / `i32` digests.
#[inline]
pub fn crc32c(bytes: &[u8]) -> u32 {
    !update_crc32c(!0, bytes)
}

#[inline]
fn update_crc32c(mut crc: u32, bytes: &[u8]) -> u32 {
    for &byte in bytes {
        crc = CRC32C_TABLE[((crc ^ u32::from(byte)) & 0xff) as usize] ^ (crc >> 8);
    }
    crc
}

#[inline]
pub fn hash_empty() -> u32 {
    crc32c(&[HASH_TAG_EMPTY])
}

#[inline]
pub fn hash_empty_map() -> u32 {
    crc32c(&[HASH_TAG_MAP_START, HASH_TAG_MAP_END])
}

#[inline]
pub fn hash_empty_list() -> u32 {
    crc32c(&[HASH_TAG_LIST_START, HASH_TAG_LIST_END])
}

#[inline]
pub fn hash_byte(value: i8) -> u32 {
    crc32c(&[HASH_TAG_BYTE, value as u8])
}

#[inline]
pub fn hash_short(value: i16) -> u32 {
    let mut bytes = [0u8; 3];
    bytes[0] = HASH_TAG_SHORT;
    bytes[1..].copy_from_slice(&value.to_le_bytes());
    crc32c(&bytes)
}

#[inline]
pub fn hash_int(value: i32) -> u32 {
    let mut bytes = [0u8; 5];
    bytes[0] = HASH_TAG_INT;
    bytes[1..].copy_from_slice(&value.to_le_bytes());
    crc32c(&bytes)
}

/// Signed wire `i32` view of `HashOps.CRC32C_INSTANCE.createInt(value).asInt()`.
#[inline]
pub fn hash_ops_crc32c_int(value: i32) -> i32 {
    hash_int(value) as i32
}

#[inline]
pub fn hash_long(value: i64) -> u32 {
    let mut bytes = [0u8; 9];
    bytes[0] = HASH_TAG_LONG;
    bytes[1..].copy_from_slice(&value.to_le_bytes());
    crc32c(&bytes)
}

#[inline]
pub fn hash_float(bits: u32) -> u32 {
    let mut bytes = [0u8; 5];
    bytes[0] = HASH_TAG_FLOAT;
    bytes[1..].copy_from_slice(&bits.to_le_bytes());
    crc32c(&bytes)
}

#[inline]
pub fn hash_double(bits: u64) -> u32 {
    let mut bytes = [0u8; 9];
    bytes[0] = HASH_TAG_DOUBLE;
    bytes[1..].copy_from_slice(&bits.to_le_bytes());
    crc32c(&bytes)
}

#[inline]
pub fn hash_boolean(value: bool) -> u32 {
    crc32c(&[HASH_TAG_BOOLEAN, u8::from(value)])
}

pub fn hash_string(value: &str) -> u32 {
    let count = i32::try_from(value.encode_utf16().count()).unwrap_or(i32::MAX);
    let mut crc = update_crc32c(!0, &[HASH_TAG_STRING]);
    crc = update_crc32c(crc, &count.to_le_bytes());
    for unit in value.encode_utf16() {
        crc = update_crc32c(crc, &unit.to_le_bytes());
    }
    !crc
}

pub fn hash_java_string(value: &JavaString) -> u32 {
    let units = value.units();
    let count = i32::try_from(units.len()).unwrap_or(i32::MAX);
    let mut crc = update_crc32c(!0, &[HASH_TAG_STRING]);
    crc = update_crc32c(crc, &count.to_le_bytes());
    for &unit in units {
        crc = update_crc32c(crc, &unit.to_le_bytes());
    }
    !crc
}

/// `HashOps.hashMap`: orders entries by unsigned 32-bit
/// `HashCode.padToLong()` key digest then value digest.
pub fn hash_map(mut entries: Vec<(u32, u32)>) -> u32 {
    entries.sort_unstable();
    let mut crc = update_crc32c(!0, &[HASH_TAG_MAP_START]);
    for (key, value) in entries {
        crc = update_crc32c(crc, &key.to_le_bytes());
        crc = update_crc32c(crc, &value.to_le_bytes());
    }
    !update_crc32c(crc, &[HASH_TAG_MAP_END])
}

pub fn hash_record<'a>(entries: impl IntoIterator<Item = (&'a str, u32)>) -> u32 {
    hash_map(
        entries
            .into_iter()
            .map(|(name, value)| (hash_string(name), value))
            .collect(),
    )
}

pub fn hash_list(values: impl IntoIterator<Item = u32>) -> u32 {
    let mut crc = update_crc32c(!0, &[HASH_TAG_LIST_START]);
    for value in values {
        crc = update_crc32c(crc, &value.to_le_bytes());
    }
    !update_crc32c(crc, &[HASH_TAG_LIST_END])
}

pub fn hash_byte_array(values: &[i8]) -> u32 {
    let mut crc = update_crc32c(!0, &[HASH_TAG_BYTE_ARRAY_START]);
    for &value in values {
        crc = update_crc32c(crc, &[value as u8]);
    }
    !update_crc32c(crc, &[HASH_TAG_BYTE_ARRAY_END])
}

pub fn hash_int_array(values: impl IntoIterator<Item = i32>) -> u32 {
    let mut crc = update_crc32c(!0, &[HASH_TAG_INT_ARRAY_START]);
    for value in values {
        crc = update_crc32c(crc, &value.to_le_bytes());
    }
    !update_crc32c(crc, &[HASH_TAG_INT_ARRAY_END])
}

pub fn hash_long_array(values: &[i64]) -> u32 {
    let mut crc = update_crc32c(!0, &[HASH_TAG_LONG_ARRAY_START]);
    for &value in values {
        crc = update_crc32c(crc, &value.to_le_bytes());
    }
    !update_crc32c(crc, &[HASH_TAG_LONG_ARRAY_END])
}

/// `NbtOps.convertTo(HashOps.CRC32C_INSTANCE, tag)` (`NbtOps.java`).
pub fn hash_nbt(value: &NbtSummaryValue, byte_booleans: bool) -> u32 {
    match value {
        NbtSummaryValue::Byte(v) => {
            if byte_booleans {
                hash_boolean(*v != 0)
            } else {
                hash_byte(*v)
            }
        }
        NbtSummaryValue::Short(v) => hash_short(*v),
        NbtSummaryValue::Int(v) => hash_int(*v),
        NbtSummaryValue::Long(v) => hash_long(*v),
        NbtSummaryValue::Float(bits) => hash_float(*bits),
        NbtSummaryValue::Double(bits) => hash_double(*bits),
        NbtSummaryValue::ByteArray(values) => hash_byte_array(values),
        NbtSummaryValue::String(text) => hash_java_string(text),
        NbtSummaryValue::List(items) => {
            hash_list(items.iter().map(|item| hash_nbt(item, byte_booleans)))
        }
        NbtSummaryValue::Compound(entries) => {
            // `CompoundTag` stores keys in a `HashMap`, so a duplicate key keeps the last value.
            let mut dedup = BTreeMap::new();
            for entry in entries {
                dedup.insert(entry.name.units(), hash_nbt(&entry.value, byte_booleans));
            }
            let pairs = dedup
                .into_iter()
                .map(|(name_units, val_hash)| {
                    let count = i32::try_from(name_units.len()).unwrap_or(i32::MAX);
                    let mut crc = update_crc32c(!0, &[HASH_TAG_STRING]);
                    crc = update_crc32c(crc, &count.to_le_bytes());
                    for &unit in name_units {
                        crc = update_crc32c(crc, &unit.to_le_bytes());
                    }
                    (!crc, val_hash)
                })
                .collect();
            hash_map(pairs)
        }
        NbtSummaryValue::IntArray(values) => hash_int_array(values.iter().copied()),
        NbtSummaryValue::LongArray(values) => hash_long_array(values),
    }
}

/// Normalizes a `ResourceLocation` / `Identifier` leniently using default `"minecraft"`
/// namespace, delegating to [`bbb_protocol::normalize_resource_location`].
pub fn normalize_codec_identifier(raw: &str) -> Option<String> {
    bbb_protocol::normalize_resource_location(raw).ok()
}

pub(super) fn normalize_java_identifier(raw: &JavaString) -> Option<String> {
    raw.with_utf8(normalize_codec_identifier).flatten()
}

pub(super) fn hash_sound_event(
    lookup: &dyn ItemHashLookup,
    sound: &SoundEventSummary,
) -> Option<u32> {
    match (
        sound.registry_id,
        sound.sound_id.as_deref(),
        sound.fixed_range_bits,
    ) {
        (Some(holder_id), None, None) => lookup
            .resolve_holder_resource_id("minecraft:sound_event", holder_id)
            .map(|id| hash_string(&id)),
        (None, Some(sound_id), range_bits) => {
            let sound_id = normalize_codec_identifier(sound_id)?;
            let mut fields = vec![("sound_id", hash_string(&sound_id))];
            if let Some(bits) = range_bits {
                fields.push(("range", hash_float(bits)));
            }
            Some(hash_record(fields))
        }
        _ => None,
    }
}

pub(super) fn hash_holder_set_from_ids(
    lookup: &dyn ItemHashLookup,
    registry_id: &str,
    ids: &[i32],
    tag: Option<&str>,
) -> Option<u32> {
    match tag {
        Some(tag) => {
            if !ids.is_empty() {
                return None;
            }
            let normalized = normalize_codec_identifier(tag)?;
            Some(hash_string(&format!("#{normalized}")))
        }
        None => match ids {
            [single] => lookup
                .resolve_holder_resource_id(registry_id, *single)
                .map(|id| hash_string(&id)),
            many => {
                let mut hashes = Vec::with_capacity(many.len());
                for &id in many {
                    let resource_id = lookup.resolve_holder_resource_id(registry_id, id)?;
                    hashes.push(hash_string(&resource_id));
                }
                Some(hash_list(hashes))
            }
        },
    }
}

/// Shared encoder bridge allowing `persistent_item_stack_projection` component
/// schemas (`Consumable`, `BlocksAttacks`, `KineticWeapon`, etc.) to target `HashOps`.
pub(super) struct ComponentHashEncoder<'a> {
    pub(super) lookup: &'a dyn ItemHashLookup,
}

impl<'a> ComponentHashEncoder<'a> {
    pub(super) fn new(lookup: &'a dyn ItemHashLookup) -> Self {
        Self { lookup }
    }
}

impl ComponentEncoder for ComponentHashEncoder<'_> {
    type Value = u32;
    type Fields = Vec<(u32, u32)>;
    type Error = ();

    fn fields(&self, capacity: usize) -> Self::Fields {
        Vec::with_capacity(capacity)
    }

    fn field(&self, fields: &mut Self::Fields, name: &'static str, value: Self::Value) {
        fields.push((hash_string(name), value));
    }

    fn record(&self, fields: Self::Fields) -> Self::Value {
        if fields.is_empty() {
            return hash_empty_map();
        }
        hash_map(fields)
    }

    fn scalar(&self, fields: &mut Self::Fields, name: &'static str, value: ComponentScalar<'_>) {
        self.field(
            fields,
            name,
            match value {
                ComponentScalar::Int(v) => hash_int(v),
                ComponentScalar::Byte(v) => hash_byte(v),
                ComponentScalar::Boolean(v) => hash_boolean(v),
                ComponentScalar::Float(bits) => hash_float(bits),
                ComponentScalar::String(v) => hash_string(v),
            },
        );
    }

    fn list(
        &self,
        values: impl Iterator<Item = std::result::Result<Self::Value, Self::Error>>,
    ) -> std::result::Result<Self::Value, Self::Error> {
        let mut crc = update_crc32c(!0, &[HASH_TAG_LIST_START]);
        for value in values {
            crc = update_crc32c(crc, &value?.to_le_bytes());
        }
        Ok(!update_crc32c(crc, &[HASH_TAG_LIST_END]))
    }

    fn check(&self, valid: bool, _detail: &'static str) -> std::result::Result<(), Self::Error> {
        valid.then_some(()).ok_or(())
    }

    fn sound(&self, sound: &SoundEventSummary) -> std::result::Result<Self::Value, Self::Error> {
        hash_sound_event(self.lookup, sound).ok_or(())
    }
}

impl WeaponComponentEncoder for ComponentHashEncoder<'_> {
    fn non_negative_float(
        &self,
        _name: &str,
        bits: u32,
    ) -> std::result::Result<Self::Value, Self::Error> {
        let value = f32::from_bits(bits);
        ((0.0..=f32::MAX).contains(&value))
            .then(|| hash_float(bits))
            .ok_or(())
    }

    fn positive_float(
        &self,
        _name: &str,
        bits: u32,
    ) -> std::result::Result<Self::Value, Self::Error> {
        let value = f32::from_bits(bits);
        (value > 0.0 && value <= f32::MAX)
            .then(|| hash_float(bits))
            .ok_or(())
    }

    fn non_negative_int(
        &self,
        _name: &str,
        value: i32,
    ) -> std::result::Result<Self::Value, Self::Error> {
        (value >= 0).then(|| hash_int(value)).ok_or(())
    }

    fn reductions(
        &self,
        values: &[bbb_protocol::packets::DamageReductionSummary],
    ) -> std::result::Result<Self::Value, Self::Error> {
        self.list(
            values
                .iter()
                .map(|reduction| {
                    crate::persistent_item_stack_projection::weapon_and_block_attacks::encode_damage_reduction(self, reduction)
                }),
        )
    }

    fn damage_types(
        &self,
        types: &bbb_protocol::packets::HolderSetSummary,
    ) -> std::result::Result<Self::Value, Self::Error> {
        hash_holder_set_from_ids(
            self.lookup,
            "minecraft:damage_type",
            &types.registry_ids,
            types.tag.as_deref(),
        )
        .ok_or(())
    }
}

impl ConsumableComponentEncoder for ComponentHashEncoder<'_> {
    fn sound_is_default(&self, sound: &SoundEventSummary, resource_id: &str) -> bool {
        sound.sound_id.is_none()
            && sound.fixed_range_bits.is_none()
            && sound.registry_id.is_some_and(|holder_id| {
                self.lookup
                    .resolve_holder_resource_id("minecraft:sound_event", holder_id)
                    .as_deref()
                    == Some(resource_id)
                    || vanilla_sound_event_resource_id(holder_id) == Some(resource_id)
            })
    }

    fn mob_effect(&self, holder_id: i32) -> std::result::Result<Self::Value, Self::Error> {
        self.lookup
            .resolve_holder_resource_id("minecraft:mob_effect", holder_id)
            .map(|id| hash_string(&id))
            .ok_or(())
    }

    fn mob_effect_set(
        &self,
        ids: &[i32],
        tag: Option<&str>,
    ) -> std::result::Result<Self::Value, Self::Error> {
        hash_holder_set_from_ids(self.lookup, "minecraft:mob_effect", ids, tag).ok_or(())
    }
}
