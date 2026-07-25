//! Vanilla `HashOps.CRC32C_INSTANCE` encoding for predicted container clicks.
//!
//! `ServerboundContainerClickPacket` does not resend changed `ItemStack`
//! component values. It sends the persistent-codec hash of every added value,
//! so this module mirrors the codec shape as well as the primitive CRC32C
//! representation. Components whose exact persistent codec is not represented
//! here deliberately keep the enclosing click server-authoritative.

use std::collections::BTreeMap;

use bbb_protocol::{
    builtin_enum_component_values::builtin_enum_component_value_name,
    data_component_types::{
        vanilla_data_component_resource_id_for_type_id, vanilla_data_component_type_is_persistent,
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
    },
    packets::{
        validate_data_component_patch_summary, DataComponentPatchSummary, HashedComponentPatch,
        HashedItemStack, HashedStack, ItemStackSummary, NbtSummaryValue,
    },
    tropical_fish_patterns::vanilla_tropical_fish_pattern,
};

use super::{ContainerClickBuildError, ContainerSlot, InventoryCtx};

const HASH_TAG_MAP_START: u8 = 2;
const HASH_TAG_MAP_END: u8 = 3;
const HASH_TAG_LIST_START: u8 = 4;
const HASH_TAG_LIST_END: u8 = 5;
const HASH_TAG_BYTE: u8 = 6;
const HASH_TAG_SHORT: u8 = 7;
const HASH_TAG_INT: u8 = 8;
const HASH_TAG_LONG: u8 = 9;
const HASH_TAG_FLOAT: u8 = 10;
const HASH_TAG_DOUBLE: u8 = 11;
const HASH_TAG_STRING: u8 = 12;
const HASH_TAG_BOOLEAN: u8 = 13;
const HASH_TAG_BYTE_ARRAY_START: u8 = 14;
const HASH_TAG_BYTE_ARRAY_END: u8 = 15;
const HASH_TAG_INT_ARRAY_START: u8 = 16;
const HASH_TAG_INT_ARRAY_END: u8 = 17;
const HASH_TAG_LONG_ARRAY_START: u8 = 18;
const HASH_TAG_LONG_ARRAY_END: u8 = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Hash32(u32);

pub(super) fn hashed_stack_from_summary(
    ctx: InventoryCtx<'_>,
    stack: &ItemStackSummary,
) -> Option<HashedStack> {
    let (Some(item_id), true) = (stack.item_id, stack.count > 0) else {
        return Some(HashedStack::Empty);
    };
    let components = hashed_component_patch_from_summary(ctx, &stack.component_patch)?;
    Some(HashedStack::Item(HashedItemStack {
        item_id,
        count: stack.count,
        components,
    }))
}

pub(super) fn hashed_component_patch_from_summary(
    ctx: InventoryCtx<'_>,
    patch: &DataComponentPatchSummary,
) -> Option<HashedComponentPatch> {
    if patch == &DataComponentPatchSummary::default() {
        return Some(HashedComponentPatch::default());
    }
    validate_data_component_patch_summary(patch).ok()?;

    let mut added_components = BTreeMap::new();
    for type_id in &patch.added_type_ids {
        let hash = hash_component_value(ctx, patch, *type_id)?;
        added_components.insert(*type_id, hash.0 as i32);
    }
    Some(HashedComponentPatch {
        added_components,
        removed_components: patch.removed_type_ids.iter().copied().collect(),
    })
}

pub(super) fn component_patch_can_be_hashed_from_summary(
    ctx: InventoryCtx<'_>,
    patch: &DataComponentPatchSummary,
) -> bool {
    hashed_component_patch_from_summary(ctx, patch).is_some()
}

pub(super) fn changed_hashed_slots(
    ctx: InventoryCtx<'_>,
    before: &[ContainerSlot],
    after: &[ContainerSlot],
) -> Result<BTreeMap<i16, HashedStack>, ContainerClickBuildError> {
    let mut changed = BTreeMap::new();
    for slot in after {
        if before
            .iter()
            .find(|before| before.slot == slot.slot)
            .is_some_and(|before| before.item == slot.item)
        {
            continue;
        }
        let hashed = hashed_stack_from_summary(ctx, &slot.item)
            .ok_or(ContainerClickBuildError::UnhashableChangedSlot(slot.slot))?;
        changed.insert(slot.slot, hashed);
    }
    Ok(changed)
}

fn hash_component_value(
    _ctx: InventoryCtx<'_>,
    patch: &DataComponentPatchSummary,
    type_id: i32,
) -> Option<Hash32> {
    if !vanilla_data_component_type_is_persistent(type_id) {
        return None;
    }

    let hash = match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => {
            let value = patch.custom_data.as_ref()?;
            if !matches!(value, NbtSummaryValue::Compound(_)) {
                return None;
            }
            hash_nbt(value)
        }
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => hash_int(patch.max_stack_size?),
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => hash_int(patch.max_damage?),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => hash_int(patch.damage?),
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID
        | VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID
        | VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => hash_map(Vec::new()),
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => {
            let value = patch.use_effects?;
            let mut fields = Vec::new();
            push_non_default_bool(&mut fields, "can_sprint", value.can_sprint, false);
            push_non_default_bool(
                &mut fields,
                "interact_vibrations",
                value.interact_vibrations,
                true,
            );
            push_non_default_float(
                &mut fields,
                "speed_multiplier",
                value.speed_multiplier.to_bits(),
                0.2_f32.to_bits(),
            );
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => {
            hash_float(patch.minimum_attack_charge_bits?)
        }
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => hash_string(patch.item_model.as_deref()?),
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => hash_string(patch.rarity?.as_str()),
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => {
            let mut fields = Vec::new();
            if !patch.custom_model_data_floats.is_empty() {
                fields.push((
                    "floats",
                    hash_list(
                        patch
                            .custom_model_data_floats
                            .iter()
                            .map(|value| hash_float(value.to_bits())),
                    ),
                ));
            }
            if !patch.custom_model_data_flags.is_empty() {
                fields.push((
                    "flags",
                    hash_list(
                        patch
                            .custom_model_data_flags
                            .iter()
                            .map(|value| hash_boolean(*value)),
                    ),
                ));
            }
            if !patch.custom_model_data_strings.is_empty() {
                fields.push((
                    "strings",
                    hash_list(
                        patch
                            .custom_model_data_strings
                            .iter()
                            .map(|value| hash_string(value)),
                    ),
                ));
            }
            if !patch.custom_model_data_colors.is_empty() {
                fields.push((
                    "colors",
                    hash_list(
                        patch
                            .custom_model_data_colors
                            .iter()
                            .map(|value| hash_int(*value)),
                    ),
                ));
            }
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => {
            let mut fields = Vec::new();
            push_non_default_bool(
                &mut fields,
                "hide_tooltip",
                patch.tooltip_hide_tooltip,
                false,
            );
            if !patch.tooltip_hidden_component_type_ids.is_empty() {
                let hidden = patch
                    .tooltip_hidden_component_type_ids
                    .iter()
                    .map(|type_id| {
                        vanilla_data_component_resource_id_for_type_id(*type_id).map(hash_string)
                    })
                    .collect::<Option<Vec<_>>>()?;
                fields.push(("hidden_components", hash_list(hidden)));
            }
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => hash_int(patch.repair_cost?),
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            hash_boolean(patch.enchantment_glint_override?)
        }
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => {
            let mut fields = vec![
                ("nutrition", hash_int(patch.food_nutrition?)),
                ("saturation", hash_float(patch.food_saturation_bits?)),
            ];
            push_non_default_bool(
                &mut fields,
                "can_always_eat",
                patch.food_can_always_eat,
                false,
            );
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => {
            let mut fields = vec![("seconds", hash_float(patch.use_cooldown_seconds_bits?))];
            if let Some(group) = &patch.use_cooldown_group {
                fields.push(("cooldown_group", hash_string(group)));
            }
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => {
            let mut fields = Vec::new();
            push_non_default_int(
                &mut fields,
                "item_damage_per_attack",
                patch.weapon_item_damage_per_attack?,
                1,
            );
            push_non_default_float(
                &mut fields,
                "disable_blocking_for_seconds",
                patch.weapon_disable_blocking_seconds_bits?,
                0.0_f32.to_bits(),
            );
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => {
            let value = patch.attack_range?;
            let mut fields = Vec::new();
            push_non_default_float(
                &mut fields,
                "min_reach",
                value.min_reach.to_bits(),
                0.0_f32.to_bits(),
            );
            push_non_default_float(
                &mut fields,
                "max_reach",
                value.max_reach.to_bits(),
                3.0_f32.to_bits(),
            );
            push_non_default_float(
                &mut fields,
                "min_creative_reach",
                value.min_creative_reach.to_bits(),
                0.0_f32.to_bits(),
            );
            push_non_default_float(
                &mut fields,
                "max_creative_reach",
                value.max_creative_reach.to_bits(),
                5.0_f32.to_bits(),
            );
            push_non_default_float(
                &mut fields,
                "hitbox_margin",
                value.hitbox_margin.to_bits(),
                0.3_f32.to_bits(),
            );
            push_non_default_float(
                &mut fields,
                "mob_factor",
                value.mob_factor.to_bits(),
                1.0_f32.to_bits(),
            );
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => {
            hash_record(vec![("value", hash_int(patch.enchantable_value?))])
        }
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => {
            hash_string(patch.tooltip_style.as_deref()?)
        }
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => {
            let value = patch.swing_animation?;
            let mut fields = Vec::new();
            if value.animation_type.id() != 1 {
                fields.push(("type", hash_string(value.animation_type.as_str())));
            }
            push_non_default_int(&mut fields, "duration", value.duration, 6);
            hash_record(fields)
        }
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID => hash_string(builtin_enum_component_value_name(
            type_id,
            patch.dye_color_id?,
        )?),
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => hash_int(patch.dyed_color?),
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => hash_int(patch.map_color?),
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => hash_int(patch.map_id?),
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => {
            hash_float(patch.potion_duration_scale_bits?)
        }
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => {
            hash_int(patch.ominous_bottle_amplifier?)
        }
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => {
            hash_string(patch.note_block_sound.as_deref()?)
        }
        VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => hash_string(
            builtin_enum_component_value_name(type_id, patch.base_color_id?)?,
        ),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => hash_string(
            vanilla_tropical_fish_pattern(patch.tropical_fish_pattern_id?).serialized_name,
        ),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => hash_string(
            builtin_enum_component_value_name(type_id, patch.tropical_fish_base_color_id?)?,
        ),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => hash_string(
            builtin_enum_component_value_name(type_id, patch.tropical_fish_pattern_color_id?)?,
        ),
        _ => {
            let value_id = patch.builtin_enum_component_ids.get(&type_id)?;
            hash_string(builtin_enum_component_value_name(type_id, *value_id)?)
        }
    };
    Some(hash)
}

fn push_non_default_bool<'a>(
    fields: &mut Vec<(&'a str, Hash32)>,
    name: &'a str,
    value: bool,
    default: bool,
) {
    if value != default {
        fields.push((name, hash_boolean(value)));
    }
}

fn push_non_default_int<'a>(
    fields: &mut Vec<(&'a str, Hash32)>,
    name: &'a str,
    value: i32,
    default: i32,
) {
    if value != default {
        fields.push((name, hash_int(value)));
    }
}

fn push_non_default_float<'a>(
    fields: &mut Vec<(&'a str, Hash32)>,
    name: &'a str,
    value_bits: u32,
    default_bits: u32,
) {
    if value_bits != default_bits {
        fields.push((name, hash_float(value_bits)));
    }
}

fn hash_record(fields: Vec<(&str, Hash32)>) -> Hash32 {
    hash_map(
        fields
            .into_iter()
            .map(|(name, value)| (hash_string(name), value))
            .collect(),
    )
}

fn hash_nbt(value: &NbtSummaryValue) -> Hash32 {
    match value {
        NbtSummaryValue::Byte(value) => hash_tagged_bytes(HASH_TAG_BYTE, &value.to_le_bytes()),
        NbtSummaryValue::Short(value) => hash_tagged_bytes(HASH_TAG_SHORT, &value.to_le_bytes()),
        NbtSummaryValue::Int(value) => hash_int(*value),
        NbtSummaryValue::Long(value) => hash_tagged_bytes(HASH_TAG_LONG, &value.to_le_bytes()),
        NbtSummaryValue::Float(bits) => hash_float(*bits),
        NbtSummaryValue::Double(bits) => hash_tagged_bytes(HASH_TAG_DOUBLE, &bits.to_le_bytes()),
        NbtSummaryValue::ByteArray(values) => hash_delimited_bytes(
            HASH_TAG_BYTE_ARRAY_START,
            values.iter().map(|value| value.to_le_bytes()[0]),
            HASH_TAG_BYTE_ARRAY_END,
        ),
        NbtSummaryValue::String(value) => hash_string(value),
        NbtSummaryValue::List(values) => hash_list(values.iter().map(hash_nbt)),
        NbtSummaryValue::Compound(entries) => hash_map(
            entries
                .iter()
                .map(|entry| (hash_string(&entry.name), hash_nbt(&entry.value)))
                .collect(),
        ),
        NbtSummaryValue::IntArray(values) => {
            let mut payload = Vec::with_capacity(2 + values.len() * 4);
            payload.push(HASH_TAG_INT_ARRAY_START);
            for value in values {
                payload.extend_from_slice(&value.to_le_bytes());
            }
            payload.push(HASH_TAG_INT_ARRAY_END);
            hash_bytes(&payload)
        }
        NbtSummaryValue::LongArray(values) => {
            let mut payload = Vec::with_capacity(2 + values.len() * 8);
            payload.push(HASH_TAG_LONG_ARRAY_START);
            for value in values {
                payload.extend_from_slice(&value.to_le_bytes());
            }
            payload.push(HASH_TAG_LONG_ARRAY_END);
            hash_bytes(&payload)
        }
    }
}

#[cfg(test)]
pub(super) fn hash_ops_crc32c_int(value: i32) -> i32 {
    hash_int(value).0 as i32
}

fn hash_int(value: i32) -> Hash32 {
    hash_tagged_bytes(HASH_TAG_INT, &value.to_le_bytes())
}

fn hash_float(bits: u32) -> Hash32 {
    hash_tagged_bytes(HASH_TAG_FLOAT, &bits.to_le_bytes())
}

fn hash_boolean(value: bool) -> Hash32 {
    hash_bytes(&[HASH_TAG_BOOLEAN, u8::from(value)])
}

fn hash_string(value: &str) -> Hash32 {
    let units: Vec<_> = value.encode_utf16().collect();
    let mut payload = Vec::with_capacity(5 + units.len() * 2);
    payload.push(HASH_TAG_STRING);
    payload.extend_from_slice(&(units.len() as i32).to_le_bytes());
    for unit in units {
        payload.extend_from_slice(&unit.to_le_bytes());
    }
    hash_bytes(&payload)
}

fn hash_map(mut entries: Vec<(Hash32, Hash32)>) -> Hash32 {
    entries.sort_unstable();
    let mut payload = Vec::with_capacity(2 + entries.len() * 8);
    payload.push(HASH_TAG_MAP_START);
    for (key, value) in entries {
        payload.extend_from_slice(&key.0.to_le_bytes());
        payload.extend_from_slice(&value.0.to_le_bytes());
    }
    payload.push(HASH_TAG_MAP_END);
    hash_bytes(&payload)
}

fn hash_list(values: impl IntoIterator<Item = Hash32>) -> Hash32 {
    let values = values.into_iter();
    let (lower, _) = values.size_hint();
    let mut payload = Vec::with_capacity(2 + lower * 4);
    payload.push(HASH_TAG_LIST_START);
    for value in values {
        payload.extend_from_slice(&value.0.to_le_bytes());
    }
    payload.push(HASH_TAG_LIST_END);
    hash_bytes(&payload)
}

fn hash_tagged_bytes(tag: u8, value: &[u8]) -> Hash32 {
    let mut payload = Vec::with_capacity(value.len() + 1);
    payload.push(tag);
    payload.extend_from_slice(value);
    hash_bytes(&payload)
}

fn hash_delimited_bytes(start: u8, values: impl IntoIterator<Item = u8>, end: u8) -> Hash32 {
    let values = values.into_iter();
    let (lower, _) = values.size_hint();
    let mut payload = Vec::with_capacity(2 + lower);
    payload.push(start);
    payload.extend(values);
    payload.push(end);
    hash_bytes(&payload)
}

fn hash_bytes(bytes: &[u8]) -> Hash32 {
    Hash32(crc32c(bytes))
}

pub(super) fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0x82f6_3b78 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        packets::{ItemRaritySummary, UseEffectsSummary},
        NbtSummaryEntry,
    };

    use super::*;

    fn test_ctx(store: &crate::WorldStore) -> InventoryCtx<'_> {
        InventoryCtx {
            items: &store.items,
            local_player: &store.local_player,
            entities: &store.entities,
            recipes: &store.recipes,
            recipe_book: &store.recipe_book,
            registries: &store.registries,
        }
    }

    #[test]
    fn crc32c_int_matches_guava_hash_ops_fixture() {
        assert_eq!(hash_ops_crc32c_int(7), -1_726_626_450);
    }

    #[test]
    fn strings_use_java_utf16_length_and_unencoded_chars() {
        let encoded_surrogate_pair = hash_string("😀");
        let payload = [HASH_TAG_STRING, 2, 0, 0, 0, 0x3d, 0xd8, 0x00, 0xde];
        assert_eq!(encoded_surrogate_pair, hash_bytes(&payload));
        assert_ne!(encoded_surrogate_pair, hash_string("\u{fffd}"));
    }

    #[test]
    fn maps_sort_by_unsigned_key_then_value_hash() {
        let left = hash_map(vec![
            (hash_string("z"), hash_int(1)),
            (hash_string("a"), hash_int(2)),
        ]);
        let right = hash_map(vec![
            (hash_string("a"), hash_int(2)),
            (hash_string("z"), hash_int(1)),
        ]);
        assert_eq!(left, right);
    }

    #[test]
    fn custom_data_compounds_use_nbt_primitive_and_container_tags() {
        let left = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "flag".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "values".to_string(),
                value: NbtSummaryValue::IntArray(vec![1, 2]),
            },
        ]);
        let right = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "values".to_string(),
                value: NbtSummaryValue::IntArray(vec![1, 2]),
            },
            NbtSummaryEntry {
                name: "flag".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        assert_eq!(hash_nbt(&left), hash_nbt(&right));
        assert_ne!(hash_nbt(&NbtSummaryValue::Byte(1)), hash_boolean(true));
        assert_ne!(
            hash_nbt(&NbtSummaryValue::IntArray(vec![1])),
            hash_list([hash_int(1)])
        );
    }

    #[test]
    fn hashes_primitive_unit_and_enum_component_codecs() {
        let custom_data = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "value".to_string(),
            value: NbtSummaryValue::Int(7),
        }]);
        let patch = DataComponentPatchSummary {
            added: 7,
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
                VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
                VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
            ],
            custom_data: Some(custom_data.clone()),
            unbreakable: true,
            use_effects: Some(UseEffectsSummary {
                can_sprint: false,
                interact_vibrations: true,
                speed_multiplier: 0.2,
            }),
            item_model: Some("minecraft:diamond_sword".to_string()),
            rarity: Some(ItemRaritySummary::Rare),
            enchantment_glint_override: Some(true),
            dye_color_id: Some(14),
            ..DataComponentPatchSummary::default()
        };
        let store = crate::WorldStore::new();

        let hashed = hashed_component_patch_from_summary(test_ctx(&store), &patch).unwrap();

        assert_eq!(
            hashed.added_components,
            BTreeMap::from([
                (
                    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                    hash_nbt(&custom_data).0 as i32,
                ),
                (
                    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                    hash_map(Vec::new()).0 as i32,
                ),
                (
                    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                    hash_map(Vec::new()).0 as i32,
                ),
                (
                    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
                    hash_string("minecraft:diamond_sword").0 as i32,
                ),
                (
                    VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                    hash_string("rare").0 as i32,
                ),
                (
                    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
                    hash_boolean(true).0 as i32,
                ),
                (
                    VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                    hash_string("red").0 as i32,
                ),
            ])
        );
    }

    #[test]
    fn keeps_transient_and_unimplemented_registry_codec_components_server_authoritative() {
        let transient_patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
            ],
            creative_slot_lock: true,
            ..DataComponentPatchSummary::default()
        };
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
            ],
            damage_type_id: Some(0),
            ..DataComponentPatchSummary::default()
        };
        let store = crate::WorldStore::new();

        assert_eq!(
            hashed_component_patch_from_summary(test_ctx(&store), &transient_patch),
            None
        );
        assert_eq!(
            hashed_component_patch_from_summary(test_ctx(&store), &patch),
            None
        );
    }
}
