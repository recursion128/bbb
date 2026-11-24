//! Per-component `HashOps.CRC32C_INSTANCE` digest encoders for all Vanilla
//! 26.1 `DataComponentType` IDs (`0..=109`).

use std::collections::{BTreeMap, BTreeSet};

use bbb_facts::attribute_modifier_types::{
    attribute_modifier_display_name, attribute_modifier_operation_name,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
};
use bbb_facts::attribute_types::vanilla_attribute_type;
use bbb_facts::block_entity_types::vanilla_block_entity_type;
use bbb_facts::builtin_enum_component_values::{
    builtin_enum_component_value_name, normalize_builtin_enum_component_value_id,
};
use bbb_facts::data_component_types::{
    vanilla_data_component_type, VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
    VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID, VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
    VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID, VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
    VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID, VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
    VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID, VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
    VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
    VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID, VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
    VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID, VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
    VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    VANILLA_DATA_COMPONENT_TRIM_TYPE_ID, VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
};
use bbb_facts::equipment_slots::{vanilla_equipment_slot_by_id, vanilla_equipment_slot_group};
use bbb_facts::item_component_value_types::SwingAnimationTypeSummary;
use bbb_facts::item_types::VANILLA_ITEM_AIR_ID;
use bbb_facts::tropical_fish_patterns::vanilla_tropical_fish_pattern;
use bbb_protocol::packets::data_components::catalog::{
    canonical_container_loot_nbt, canonical_map_decorations_nbt, canonical_recipe_keys_nbt,
    parse_container_loot_nbt, parse_map_decorations_nbt, parse_recipe_keys_nbt,
};
use bbb_protocol::packets::data_components::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH;
use bbb_protocol::packets::{
    AttributeModifierSummary, BannerPatternLayerSummary, DataComponentPatchSummary,
    FireworkExplosionSummary, InstrumentSummary, ItemEnchantmentSummary, ItemStackTemplateSummary,
    TrimMaterialSummary, TrimPatternSummary, WrittenBookContentSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::persistent_item_stack_projection::{
    consumable_components::{encode_consumable, encode_consume_effect, encode_mob_effect_instance},
    weapon_and_block_attacks::{encode_blocks_attacks, encode_kinetic_weapon},
};

use super::components_predicates::{
    hash_adventure_mode_predicate, hash_persistent_item_predicate_at_depth,
};
use super::components_rich::{hash_component_at_depth, hash_resolvable_profile};
use super::ops::{
    hash_boolean, hash_byte, hash_empty_map, hash_float, hash_holder_set_from_ids, hash_int,
    hash_int_array, hash_java_string, hash_list, hash_map, hash_nbt, hash_record, hash_sound_event,
    hash_string, normalize_codec_identifier, ComponentHashEncoder,
};
use super::ItemHashLookup;

const MAX_ITEM_STACK_HASH_DEPTH: usize = MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH;
const MAX_CONTAINER_SLOTS: usize = 256;

/// `HashOps.CRC32C_INSTANCE` digest of one component entry in `patch`.
pub fn hash_component_value(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    type_id: i32,
) -> Option<u32> {
    hash_component_value_at_depth(lookup, patch, type_id, 0)
}

pub(super) fn hash_component_value_at_depth(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    type_id: i32,
    depth: usize,
) -> Option<u32> {
    if depth > MAX_ITEM_STACK_HASH_DEPTH {
        return None;
    }
    match type_id {
        // Transient network-only or server-authoritative components carry no persistent hash.
        VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID
        | VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID
        | VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID => None,

        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => patch
            .custom_data
            .as_ref()
            .filter(|nbt| matches!(nbt, NbtSummaryValue::Compound(_)))
            .map(|nbt| hash_nbt(nbt, false)),
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => patch
            .codec_backed_nbt
            .get(&type_id)
            .filter(|nbt| matches!(nbt, NbtSummaryValue::Compound(_)))
            .map(|nbt| hash_nbt(nbt, false)),
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => patch
            .max_stack_size
            .filter(|v| (1..=99).contains(v))
            .map(hash_int),
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => {
            patch.max_damage.filter(|v| *v > 0).map(hash_int)
        }
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => patch.damage.filter(|v| *v >= 0).map(hash_int),
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => {
            patch.repair_cost.filter(|v| *v >= 0).map(hash_int)
        }
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => patch.map_id.map(hash_int),
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => patch.map_color.map(hash_int),
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => patch.dyed_color.map(hash_int),
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => patch
            .ominous_bottle_amplifier
            .filter(|v| (0..=4).contains(v))
            .map(hash_int),
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => patch
            .enchantable_value
            .filter(|v| *v > 0)
            .map(|v| hash_record([("value", hash_int(v))])),
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => {
            patch.minimum_attack_charge_bits.and_then(|bits| {
                let v = f32::from_bits(bits);
                (0.0..=1.0).contains(&v).then(|| hash_float(bits))
            })
        }
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => {
            patch.potion_duration_scale_bits.and_then(|bits| {
                let v = f32::from_bits(bits);
                (0.0..=f32::MAX).contains(&v).then(|| hash_float(bits))
            })
        }
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => patch.unbreakable.then(hash_empty_map),
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID => {
            patch.intangible_projectile.then(hash_empty_map)
        }
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => Some(hash_empty_map()),
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            patch.enchantment_glint_override.map(hash_boolean)
        }
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => patch
            .item_model
            .as_deref()
            .and_then(normalize_codec_identifier)
            .map(|id| hash_string(&id)),
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => patch
            .tooltip_style
            .as_deref()
            .and_then(normalize_codec_identifier)
            .map(|id| hash_string(&id)),
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => patch
            .note_block_sound
            .as_deref()
            .and_then(normalize_codec_identifier)
            .map(|id| hash_string(&id)),
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => {
            patch.rarity.map(|rarity| hash_string(rarity.as_str()))
        }
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => patch
            .custom_name_nbt
            .as_ref()
            .and_then(|nbt| hash_component_at_depth(lookup, nbt, depth)),
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => patch
            .item_name_nbt
            .as_ref()
            .and_then(|nbt| hash_component_at_depth(lookup, nbt, depth)),
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => {
            let mut lines = Vec::with_capacity(patch.lore_nbt.len());
            for line in &patch.lore_nbt {
                lines.push(hash_component_at_depth(lookup, line, depth)?);
            }
            Some(hash_list(lines))
        }
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => hash_use_effects(patch),
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => hash_attack_range(patch),
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => hash_swing_animation(patch),
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => hash_use_cooldown(patch),
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => hash_weapon(patch),
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => hash_custom_model_data(patch),
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => hash_tooltip_display(patch),
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => hash_food(patch),
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => hash_consumable(lookup, patch),
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => hash_death_protection(lookup, patch),
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => patch
            .blocks_attacks
            .as_ref()
            .and_then(|v| encode_blocks_attacks(&ComponentHashEncoder::new(lookup), v).ok()),
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => patch
            .kinetic_weapon
            .as_ref()
            .and_then(|v| encode_kinetic_weapon(&ComponentHashEncoder::new(lookup), v).ok()),
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => hash_piercing_weapon(lookup, patch),
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => hash_equippable(lookup, patch),
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => hash_tool(lookup, patch),
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => patch
            .use_remainder
            .as_deref()
            .and_then(|t| hash_item_stack_template(lookup, t, depth)),
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
            hash_template_list(lookup, &patch.bundle_contents_items, depth)
        }
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
            hash_template_list(lookup, &patch.charged_projectiles_items, depth)
        }
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => hash_container(lookup, patch, depth),
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => hash_holder_set_from_ids(
            lookup,
            "minecraft:item",
            &patch.repairable_item_registry_ids,
            patch.repairable_item_tag.as_deref(),
        )
        .map(|items| hash_record([("items", items)])),
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => hash_holder_set_from_ids(
            lookup,
            "minecraft:damage_type",
            &patch.damage_resistant_type_registry_ids,
            patch.damage_resistant_type_tag.as_deref(),
        )
        .map(|types| hash_record([("types", types)])),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => patch
            .damage_type_id
            .and_then(|id| lookup.resolve_holder_resource_id("minecraft:damage_type", id))
            .map(|id| hash_string(&id)),
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => patch
            .break_sound
            .as_ref()
            .and_then(|sound| hash_sound_event(lookup, sound)),
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => {
            hash_enchantments(lookup, &patch.enchantments)
        }
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => {
            hash_enchantments(lookup, &patch.stored_enchantments)
        }
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => {
            hash_attribute_modifiers(lookup, &patch.attribute_modifiers, depth)
        }
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => hash_potion_contents(lookup, patch),
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => {
            hash_suspicious_stew_effects(lookup, patch)
        }
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => hash_writable_book(patch),
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => {
            hash_written_book(lookup, patch, depth)
        }
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => patch
            .firework_explosion
            .as_ref()
            .map(hash_firework_explosion),
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => hash_fireworks(patch),
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => hash_lodestone_tracker(patch),
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => Some(hash_map(
            patch
                .block_state_properties
                .iter()
                .map(|(k, v)| (hash_java_string(k), hash_java_string(v)))
                .collect(),
        )),
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => hash_debug_stick(lookup, patch),
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => hash_typed_entity_data(
            lookup,
            "minecraft:entity_type",
            patch.entity_data_entity_type_id?,
            patch.entity_data_tag.as_ref()?,
        ),
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
            let id = patch.block_entity_data_type_id?;
            let named = lookup
                .resolve_holder_resource_id("minecraft:block_entity_type", id)
                .or_else(|| vanilla_block_entity_type(id).map(|b| b.resource_id.to_owned()))?;
            hash_typed_entity_data_named(&named, patch.block_entity_data_tag.as_ref()?)
        }
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID => hash_bees(lookup, patch),
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => {
            patch.profile.as_ref().and_then(hash_resolvable_profile)
        }
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => hash_trim(lookup, patch, depth),
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => {
            hash_provides_trim_material(lookup, patch, depth)
        }
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => hash_instrument(lookup, patch, depth),
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => {
            if patch.jukebox_direct_song.is_some() {
                return None;
            }
            patch
                .jukebox_song_id
                .and_then(|id| lookup.resolve_holder_resource_id("minecraft:jukebox_song", id))
                .map(|id| hash_string(&id))
        }
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => {
            hash_banner_patterns(lookup, &patch.banner_pattern_layers)
        }
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => hash_holder_set_from_ids(
            lookup,
            "minecraft:banner_pattern",
            &patch.provides_banner_pattern_registry_ids,
            patch.provides_banner_pattern_tag.as_deref(),
        ),
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => {
            if patch.pot_decorations_item_ids.len() > 4 {
                return None;
            }
            let mut sides = Vec::with_capacity(4);
            for i in 0..4 {
                let item_id = patch
                    .pot_decorations_item_ids
                    .get(i)
                    .copied()
                    .unwrap_or(bbb_facts::item_types::VANILLA_ITEM_BRICK_ID);
                let name = lookup.item_resource_id(item_id)?;
                sides.push(hash_string(&name));
            }
            Some(hash_list(sides))
        }
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID => {
            if let Some(keys) = &patch.recipe_keys {
                let canonical = canonical_recipe_keys_nbt(keys);
                Some(hash_nbt(&canonical, false))
            } else if let Some(nbt) = patch.codec_backed_nbt.get(&type_id) {
                let keys = parse_recipe_keys_nbt(nbt).ok()?;
                Some(hash_nbt(&canonical_recipe_keys_nbt(&keys), false))
            } else {
                None
            }
        }
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID => {
            if let Some(decorations) = &patch.map_decorations {
                Some(hash_nbt(&canonical_map_decorations_nbt(decorations), false))
            } else if let Some(nbt) = patch.codec_backed_nbt.get(&type_id) {
                let decorations = parse_map_decorations_nbt(nbt).ok()?;
                Some(hash_nbt(
                    &canonical_map_decorations_nbt(&decorations),
                    false,
                ))
            } else {
                None
            }
        }
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID => {
            if let Some(table) = &patch.container_loot_table {
                let canonical = canonical_container_loot_nbt(
                    &normalize_codec_identifier(table)?,
                    patch.container_loot_seed.unwrap_or(0),
                );
                Some(hash_nbt(&canonical, false))
            } else if let Some(nbt) = patch.codec_backed_nbt.get(&type_id) {
                let (table, seed) = parse_container_loot_nbt(nbt).ok()?;
                Some(hash_nbt(&canonical_container_loot_nbt(&table, seed), false))
            } else {
                None
            }
        }
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => patch
            .lock_predicate
            .as_ref()
            .and_then(|p| hash_persistent_item_predicate_at_depth(lookup, p, depth)),
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => patch
            .can_place_on_persistent
            .as_ref()
            .and_then(|p| hash_adventure_mode_predicate(lookup, p, depth)),
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => patch
            .can_break_persistent
            .as_ref()
            .and_then(|p| hash_adventure_mode_predicate(lookup, p, depth)),

        VANILLA_DATA_COMPONENT_DYE_TYPE_ID | VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => {
            let color_id = match type_id {
                VANILLA_DATA_COMPONENT_DYE_TYPE_ID => patch.dye_color_id?,
                VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => patch.base_color_id?,
                _ => return None,
            };
            hash_checked_builtin_enum(type_id, color_id)
        }
        // `DataComponents.java:349-351`, `TropicalFish.java:281-308`: sparse packed pattern IDs.
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => {
            let packed_id = patch.tropical_fish_pattern_id?;
            let pattern = vanilla_tropical_fish_pattern(packed_id);
            (pattern.packed_id == packed_id).then(|| hash_string(pattern.serialized_name))
        }
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => {
            hash_checked_builtin_enum(type_id, patch.tropical_fish_base_color_id?)
        }
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => {
            hash_checked_builtin_enum(type_id, patch.tropical_fish_pattern_color_id?)
        }
        VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID
        | VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID
        | VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID
        | VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID
        | VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID
        | VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID
        | VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID
        | VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID
        | VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID
        | VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID
        | VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID
        | VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID => {
            let id = patch.builtin_enum_component_ids.get(&type_id).copied()?;
            hash_checked_builtin_enum(type_id, id)
        }
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => {
            if patch.painting_variant_direct.is_some() {
                return None;
            }
            patch
                .painting_variant_id
                .and_then(|id| lookup.resolve_holder_resource_id("minecraft:painting_variant", id))
                .map(|id| hash_string(&id))
        }
        _ => variant_registry(type_id).and_then(|registry| {
            let holder_id = variant_holder_id(type_id, patch)?;
            lookup
                .resolve_holder_resource_id(registry, holder_id)
                .map(|id| hash_string(&id))
        }),
    }
}

fn hash_checked_builtin_enum(type_id: i32, raw_id: i32) -> Option<u32> {
    let normalized = normalize_builtin_enum_component_value_id(type_id, raw_id)?;
    (normalized == raw_id)
        .then(|| builtin_enum_component_value_name(type_id, normalized))
        .flatten()
        .map(hash_string)
}

fn variant_registry(type_id: i32) -> Option<&'static str> {
    Some(match type_id {
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => "minecraft:villager_type",
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => "minecraft:wolf_variant",
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => "minecraft:wolf_sound_variant",
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => "minecraft:pig_variant",
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => "minecraft:pig_sound_variant",
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => "minecraft:cow_variant",
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => "minecraft:cow_sound_variant",
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => "minecraft:chicken_variant",
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => "minecraft:chicken_sound_variant",
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            "minecraft:zombie_nautilus_variant"
        }
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => "minecraft:frog_variant",
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => "minecraft:cat_variant",
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => "minecraft:cat_sound_variant",
        _ => return None,
    })
}

fn variant_holder_id(type_id: i32, patch: &DataComponentPatchSummary) -> Option<i32> {
    match type_id {
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => patch.villager_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => patch.wolf_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => patch.wolf_sound_variant_id,
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => patch.pig_variant_id,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => patch.pig_sound_variant_id,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => patch.cow_variant_id,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => patch.cow_sound_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => patch.chicken_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => patch.chicken_sound_variant_id,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => patch.zombie_nautilus_variant_id,
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => patch.frog_variant_id,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => patch.cat_variant_id,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => patch.cat_sound_variant_id,
        _ => None,
    }
}

fn hash_use_effects(patch: &DataComponentPatchSummary) -> Option<u32> {
    let effects = patch.use_effects.as_ref()?;
    let speed = effects.speed_multiplier;
    if !(0.0..=1.0).contains(&speed) {
        return None;
    }
    let mut fields = Vec::new();
    if effects.can_sprint {
        fields.push(("can_sprint", hash_boolean(true)));
    }
    if !effects.interact_vibrations {
        fields.push(("interact_vibrations", hash_boolean(false)));
    }
    if speed.to_bits() != 0.2_f32.to_bits() {
        fields.push(("speed_multiplier", hash_float(speed.to_bits())));
    }
    Some(hash_record(fields))
}

fn hash_attack_range(patch: &DataComponentPatchSummary) -> Option<u32> {
    let r = patch.attack_range.as_ref()?;
    let spec = [
        ("min_reach", r.min_reach, 0.0_f32, 0.0_f32, 64.0_f32),
        ("max_reach", r.max_reach, 3.0, 0.0, 64.0),
        ("min_creative_reach", r.min_creative_reach, 0.0, 0.0, 64.0),
        ("max_creative_reach", r.max_creative_reach, 5.0, 0.0, 64.0),
        ("hitbox_margin", r.hitbox_margin, 0.3, 0.0, 1.0),
        ("mob_factor", r.mob_factor, 1.0, 0.0, 2.0),
    ];
    let mut fields = Vec::new();
    for (name, val, default, min, max) in spec {
        if !(min..=max).contains(&val) {
            return None;
        }
        if val.to_bits() != default.to_bits() {
            fields.push((name, hash_float(val.to_bits())));
        }
    }
    Some(hash_record(fields))
}

fn hash_swing_animation(patch: &DataComponentPatchSummary) -> Option<u32> {
    let anim = patch.swing_animation.as_ref()?;
    if SwingAnimationTypeSummary::from_id(anim.animation_type.id()) != anim.animation_type {
        return None;
    }
    let mut fields = Vec::new();
    if anim.animation_type != SwingAnimationTypeSummary::Whack {
        fields.push(("type", hash_string(anim.animation_type.as_str())));
    }
    if anim.duration != 6 {
        if anim.duration <= 0 {
            return None;
        }
        fields.push(("duration", hash_int(anim.duration)));
    }
    Some(hash_record(fields))
}

fn hash_use_cooldown(patch: &DataComponentPatchSummary) -> Option<u32> {
    let bits = patch.use_cooldown_seconds_bits?;
    let seconds = f32::from_bits(bits);
    if !(seconds > 0.0 && seconds <= f32::MAX) {
        return None;
    }
    let mut fields = vec![("seconds", hash_float(bits))];
    if let Some(group) = &patch.use_cooldown_group {
        fields.push((
            "cooldown_group",
            hash_string(&normalize_codec_identifier(group)?),
        ));
    }
    Some(hash_record(fields))
}

fn hash_weapon(patch: &DataComponentPatchSummary) -> Option<u32> {
    let damage = patch.weapon_item_damage_per_attack?;
    let secs_bits = patch.weapon_disable_blocking_seconds_bits?;
    let secs = f32::from_bits(secs_bits);
    if damage < 0 || !(0.0..=f32::MAX).contains(&secs) {
        return None;
    }
    let mut fields = Vec::new();
    if damage != 1 {
        fields.push(("item_damage_per_attack", hash_int(damage)));
    }
    if secs_bits != 0.0_f32.to_bits() {
        fields.push(("disable_blocking_for_seconds", hash_float(secs_bits)));
    }
    Some(hash_record(fields))
}

fn hash_custom_model_data(patch: &DataComponentPatchSummary) -> Option<u32> {
    let mut fields = Vec::new();
    if !patch.custom_model_data_floats.is_empty() {
        fields.push((
            "floats",
            super::ops::hash_list(
                patch
                    .custom_model_data_floats
                    .iter()
                    .map(|v| hash_float(v.to_bits())),
            ),
        ));
    }
    if !patch.custom_model_data_flags.is_empty() {
        fields.push((
            "flags",
            super::ops::hash_list(
                patch
                    .custom_model_data_flags
                    .iter()
                    .copied()
                    .map(hash_boolean),
            ),
        ));
    }
    if !patch.custom_model_data_strings.is_empty() {
        fields.push((
            "strings",
            super::ops::hash_list(patch.custom_model_data_strings.iter().map(hash_java_string)),
        ));
    }
    if !patch.custom_model_data_colors.is_empty() {
        fields.push((
            "colors",
            super::ops::hash_list(patch.custom_model_data_colors.iter().copied().map(hash_int)),
        ));
    }
    Some(hash_record(fields))
}

fn hash_tooltip_display(patch: &DataComponentPatchSummary) -> Option<u32> {
    let mut fields = Vec::new();
    if patch.tooltip_hide_tooltip {
        fields.push(("hide_tooltip", hash_boolean(true)));
    }
    if !patch.tooltip_hidden_component_type_ids.is_empty() {
        let mut seen = BTreeSet::new();
        let mut names = Vec::with_capacity(patch.tooltip_hidden_component_type_ids.len());
        for &id in &patch.tooltip_hidden_component_type_ids {
            if !seen.insert(id) {
                return None;
            }
            let comp = vanilla_data_component_type(id)?;
            names.push(hash_string(comp.resource_id));
        }
        fields.push(("hidden_components", hash_list(names)));
    }
    Some(hash_record(fields))
}

fn hash_food(patch: &DataComponentPatchSummary) -> Option<u32> {
    let nutrition = patch.food_nutrition.filter(|v| *v >= 0)?;
    let saturation_bits = patch.food_saturation_bits?;
    let mut fields = vec![
        ("nutrition", hash_int(nutrition)),
        ("saturation", hash_float(saturation_bits)),
    ];
    if patch.food_can_always_eat {
        fields.push(("can_always_eat", hash_boolean(true)));
    }
    Some(hash_record(fields))
}

fn hash_consumable(lookup: &dyn ItemHashLookup, patch: &DataComponentPatchSummary) -> Option<u32> {
    let consumable = patch.consumable?;
    let sound = patch.consumable_sound.as_ref()?;
    let has_particles = patch.consumable_has_consume_particles?;
    encode_consumable(
        &ComponentHashEncoder::new(lookup),
        consumable,
        sound,
        has_particles,
        &patch.consumable_effects,
    )
    .ok()
}

fn hash_death_protection(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
) -> Option<u32> {
    if patch.death_protection_effects.is_empty() {
        return Some(hash_empty_map());
    }
    let encoder = ComponentHashEncoder::new(lookup);
    let mut effects = Vec::with_capacity(patch.death_protection_effects.len());
    for effect in &patch.death_protection_effects {
        effects.push(encode_consume_effect(&encoder, effect).ok()?);
    }
    Some(hash_record([("death_effects", hash_list(effects))]))
}

fn hash_piercing_weapon(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
) -> Option<u32> {
    let pw = patch.piercing_weapon.as_ref()?;
    let mut fields = Vec::new();
    if !pw.deals_knockback {
        fields.push(("deals_knockback", hash_boolean(false)));
    }
    if pw.dismounts {
        fields.push(("dismounts", hash_boolean(true)));
    }
    if let Some(sound) = &pw.sound {
        fields.push(("sound", hash_sound_event(lookup, sound)?));
    }
    if let Some(hit_sound) = &pw.hit_sound {
        fields.push(("hit_sound", hash_sound_event(lookup, hit_sound)?));
    }
    Some(hash_record(fields))
}

fn hash_equippable(lookup: &dyn ItemHashLookup, patch: &DataComponentPatchSummary) -> Option<u32> {
    let eq = patch.equippable.as_ref()?;
    let slot = vanilla_equipment_slot_by_id(eq.slot_id)?;
    let mut fields = vec![("slot", hash_string(slot.serialized_name))];
    let sound_is = |sound: &bbb_protocol::packets::SoundEventSummary, expected: &str| {
        sound.sound_id.is_none()
            && sound.fixed_range_bits.is_none()
            && sound.registry_id.is_some_and(|id| {
                lookup
                    .resolve_holder_resource_id("minecraft:sound_event", id)
                    .as_deref()
                    == Some(expected)
            })
    };
    if !sound_is(&eq.equip_sound, "minecraft:item.armor.equip_generic") {
        fields.push(("equip_sound", hash_sound_event(lookup, &eq.equip_sound)?));
    }
    if let Some(asset_id) = &eq.asset_id {
        fields.push((
            "asset_id",
            hash_string(&normalize_codec_identifier(asset_id)?),
        ));
    }
    if let Some(overlay) = &eq.camera_overlay {
        fields.push((
            "camera_overlay",
            hash_string(&normalize_codec_identifier(overlay)?),
        ));
    }
    if let Some(entities) = &eq.allowed_entities {
        fields.push((
            "allowed_entities",
            hash_holder_set_from_ids(
                lookup,
                "minecraft:entity_type",
                &entities.registry_ids,
                entities.tag.as_deref(),
            )?,
        ));
    }
    if !eq.dispensable {
        fields.push(("dispensable", hash_boolean(false)));
    }
    if !eq.swappable {
        fields.push(("swappable", hash_boolean(false)));
    }
    if !eq.damage_on_hurt {
        fields.push(("damage_on_hurt", hash_boolean(false)));
    }
    if eq.equip_on_interact {
        fields.push(("equip_on_interact", hash_boolean(true)));
    }
    if eq.can_be_sheared {
        fields.push(("can_be_sheared", hash_boolean(true)));
    }
    if !sound_is(&eq.shearing_sound, "minecraft:item.shears.snip") {
        fields.push((
            "shearing_sound",
            hash_sound_event(lookup, &eq.shearing_sound)?,
        ));
    }
    Some(hash_record(fields))
}

fn hash_tool(lookup: &dyn ItemHashLookup, patch: &DataComponentPatchSummary) -> Option<u32> {
    let tool = patch.tool.as_ref()?;
    if tool.damage_per_block < 0 {
        return None;
    }
    let mut rules = Vec::with_capacity(tool.rules.len());
    for rule in &tool.rules {
        let blocks = hash_holder_set_from_ids(
            lookup,
            "minecraft:block",
            &rule.block_registry_ids,
            rule.block_tag.as_deref(),
        )?;
        let mut r_fields = vec![("blocks", blocks)];
        if let Some(speed_bits) = rule.speed_bits {
            let speed = f32::from_bits(speed_bits);
            if !(speed > 0.0 && speed <= f32::MAX) {
                return None;
            }
            r_fields.push(("speed", hash_float(speed_bits)));
        }
        if let Some(correct) = rule.correct_for_drops {
            r_fields.push(("correct_for_drops", hash_boolean(correct)));
        }
        rules.push(hash_record(r_fields));
    }
    let mut fields = vec![("rules", hash_list(rules))];
    if tool.default_mining_speed_bits != 1.0_f32.to_bits() {
        fields.push((
            "default_mining_speed",
            hash_float(tool.default_mining_speed_bits),
        ));
    }
    if tool.damage_per_block != 1 {
        fields.push(("damage_per_block", hash_int(tool.damage_per_block)));
    }
    if !tool.can_destroy_blocks_in_creative {
        fields.push(("can_destroy_blocks_in_creative", hash_boolean(false)));
    }
    Some(hash_record(fields))
}

fn hash_item_stack_template(
    lookup: &dyn ItemHashLookup,
    template: &ItemStackTemplateSummary,
    depth: usize,
) -> Option<u32> {
    if depth >= MAX_ITEM_STACK_HASH_DEPTH
        || !(1..=99).contains(&template.count)
        || template.item_id == VANILLA_ITEM_AIR_ID
    {
        return None;
    }
    let item_name = lookup.item_resource_id(template.item_id)?;
    let mut fields = vec![("id", hash_string(&item_name))];
    if template.count != 1 {
        fields.push(("count", hash_int(template.count)));
    }
    let patch = &template.component_patch;
    if !patch.added_type_ids.is_empty() || !patch.removed_type_ids.is_empty() {
        let mut comp_pairs =
            Vec::with_capacity(patch.added_type_ids.len() + patch.removed_type_ids.len());
        // `DataComponentPatch.CODEC` (`DataComponentPatch.java:46`) skips transient entries.
        for &type_id in &patch.added_type_ids {
            let comp = vanilla_data_component_type(type_id)?;
            if !comp.persistent {
                continue;
            }
            let val_hash = hash_component_value_at_depth(lookup, patch, type_id, depth + 1)?;
            comp_pairs.push((hash_string(comp.resource_id), val_hash));
        }
        for &type_id in &patch.removed_type_ids {
            let comp = vanilla_data_component_type(type_id)?;
            if !comp.persistent {
                continue;
            }
            comp_pairs.push((
                hash_string(&format!("!{}", comp.resource_id)),
                hash_empty_map(),
            ));
        }
        fields.push(("components", hash_map(comp_pairs)));
    }
    Some(hash_record(fields))
}

fn hash_template_list(
    lookup: &dyn ItemHashLookup,
    templates: &[ItemStackTemplateSummary],
    depth: usize,
) -> Option<u32> {
    let mut items = Vec::with_capacity(templates.len());
    for t in templates {
        items.push(hash_item_stack_template(lookup, t, depth)?);
    }
    Some(hash_list(items))
}

fn hash_container(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    depth: usize,
) -> Option<u32> {
    let slot_count = patch.container.slot_count?;
    if slot_count > MAX_CONTAINER_SLOTS || patch.container.slots.len() > MAX_CONTAINER_SLOTS {
        return None;
    }
    let mut prev: Option<usize> = None;
    let mut slots = Vec::with_capacity(patch.container.slots.len());
    for entry in &patch.container.slots {
        if entry.slot >= slot_count || prev.is_some_and(|p| p >= entry.slot) {
            return None;
        }
        prev = Some(entry.slot);
        slots.push(hash_record([
            ("slot", hash_int(i32::try_from(entry.slot).ok()?)),
            (
                "item",
                hash_item_stack_template(lookup, &entry.item, depth)?,
            ),
        ]));
    }
    Some(hash_list(slots))
}

fn hash_enchantments(
    lookup: &dyn ItemHashLookup,
    enchantments: &[ItemEnchantmentSummary],
) -> Option<u32> {
    let mut by_id = BTreeMap::new();
    for ench in enchantments {
        if !(1..=255).contains(&ench.level) {
            return None;
        }
        let name = lookup.resolve_holder_resource_id("minecraft:enchantment", ench.holder_id)?;
        by_id.insert(name, hash_int(ench.level));
    }
    Some(hash_map(
        by_id
            .into_iter()
            .map(|(name, level)| (hash_string(&name), level))
            .collect(),
    ))
}

fn hash_attribute_modifiers(
    lookup: &dyn ItemHashLookup,
    modifiers: &[AttributeModifierSummary],
    depth: usize,
) -> Option<u32> {
    let mut items = Vec::with_capacity(modifiers.len());
    for m in modifiers {
        let attr_name = lookup
            .resolve_holder_resource_id("minecraft:attribute", m.attribute_id)
            .or_else(|| vanilla_attribute_type(m.attribute_id).map(|a| a.resource_id.to_owned()))?;
        let op_name = attribute_modifier_operation_name(m.operation_id)?;
        let slot_name = vanilla_equipment_slot_group(m.slot_id)?.serialized_name;
        let mut fields = vec![
            ("type", hash_string(&attr_name)),
            (
                "id",
                hash_string(&normalize_codec_identifier(&m.modifier_id)?),
            ),
            ("amount", super::ops::hash_double(m.amount_bits)),
            ("operation", hash_string(op_name)),
        ];
        if slot_name != "any" {
            fields.push(("slot", hash_string(slot_name)));
        }
        if m.display_id != VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID {
            let disp_name = attribute_modifier_display_name(m.display_id)?;
            let disp = match m.display_id {
                VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID => {
                    hash_record([("type", hash_string(disp_name))])
                }
                VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID => hash_record([
                    ("type", hash_string(disp_name)),
                    (
                        "value",
                        hash_component_at_depth(lookup, m.display_text_nbt.as_ref()?, depth)?,
                    ),
                ]),
                _ => return None,
            };
            fields.push(("display", disp));
        }
        items.push(hash_record(fields));
    }
    Some(hash_list(items))
}

fn hash_potion_contents(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
) -> Option<u32> {
    let pc = &patch.potion_contents;
    let mut fields = Vec::new();
    if let Some(potion_id) = pc.potion_id {
        let name = lookup.resolve_holder_resource_id("minecraft:potion", potion_id)?;
        fields.push(("potion", hash_string(&name)));
    }
    if let Some(color) = pc.custom_color {
        fields.push(("custom_color", hash_int(color)));
    }
    if let Some(name) = &pc.custom_name {
        fields.push(("custom_name", hash_java_string(name)));
    }
    if !pc.custom_effects.is_empty() {
        let encoder = ComponentHashEncoder::new(lookup);
        let mut effects = Vec::with_capacity(pc.custom_effects.len());
        // `PotionContents.FULL_CODEC` (`PotionContents.java:51`) reads via `PotionContents::customEffects`
        // (`PotionContents.java:150-152`), which copies each entry with `MobEffectInstance::new`
        // (`MobEffectInstance.java:88-91`, `setDetailsFrom` at `124-130`), dropping `hiddenEffect`.
        for effect in &pc.custom_effects {
            effects.push(encode_mob_effect_instance(&encoder, effect, None, 16).ok()?);
        }
        fields.push(("custom_effects", hash_list(effects)));
    }
    Some(hash_record(fields))
}

fn hash_suspicious_stew_effects(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
) -> Option<u32> {
    let mut entries = Vec::with_capacity(patch.suspicious_stew_effects.len());
    for effect in &patch.suspicious_stew_effects {
        let name = lookup.resolve_holder_resource_id("minecraft:mob_effect", effect.effect_id)?;
        let mut fields = vec![("id", hash_string(&name))];
        if effect.duration != 160 {
            fields.push(("duration", hash_int(effect.duration)));
        }
        entries.push(hash_record(fields));
    }
    Some(hash_list(entries))
}

fn hash_writable_book(patch: &DataComponentPatchSummary) -> Option<u32> {
    if patch.writable_book_pages.len() != patch.writable_book_page_filters.len()
        || patch.writable_book_pages.len() > 100
    {
        return None;
    }
    let mut pages = Vec::with_capacity(patch.writable_book_pages.len());
    for (raw, filtered) in patch
        .writable_book_pages
        .iter()
        .zip(&patch.writable_book_page_filters)
    {
        if raw.units().len() > 1024 || filtered.as_ref().is_some_and(|f| f.units().len() > 1024) {
            return None;
        }
        let mut fields = vec![("raw", hash_java_string(raw))];
        if let Some(filtered) = filtered {
            fields.push(("filtered", hash_java_string(filtered)));
        }
        pages.push(hash_record(fields));
    }
    if pages.is_empty() {
        Some(hash_empty_map())
    } else {
        Some(hash_record([("pages", hash_list(pages))]))
    }
}

fn hash_written_book(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    depth: usize,
) -> Option<u32> {
    let book: &WrittenBookContentSummary = patch.written_book.as_ref()?;
    if !(0..=3).contains(&book.generation)
        || book.title.units().len() > 32
        || book
            .title_filter
            .as_ref()
            .is_some_and(|t| t.units().len() > 32)
        || patch.written_book_pages_nbt.len() != book.pages.len()
        || patch.written_book_page_filters_nbt.len() != book.pages.len()
    {
        return None;
    }
    let mut title_fields = vec![("raw", hash_java_string(&book.title))];
    if let Some(filtered) = &book.title_filter {
        title_fields.push(("filtered", hash_java_string(filtered)));
    }
    let mut fields = vec![
        ("title", hash_record(title_fields)),
        ("author", hash_java_string(&book.author)),
    ];
    if book.generation != 0 {
        fields.push(("generation", hash_int(book.generation)));
    }
    if !book.pages.is_empty() {
        let mut pages = Vec::with_capacity(book.pages.len());
        for (raw_nbt, filtered_nbt) in patch
            .written_book_pages_nbt
            .iter()
            .zip(&patch.written_book_page_filters_nbt)
        {
            let raw_hash = hash_component_at_depth(lookup, raw_nbt.as_ref()?, depth)?;
            let mut page_fields = vec![("raw", raw_hash)];
            if let Some(filtered_nbt) = filtered_nbt {
                page_fields.push((
                    "filtered",
                    hash_component_at_depth(lookup, filtered_nbt, depth)?,
                ));
            }
            pages.push(hash_record(page_fields));
        }
        fields.push(("pages", hash_list(pages)));
    }
    if book.resolved {
        fields.push(("resolved", hash_boolean(true)));
    }
    Some(hash_record(fields))
}

pub fn hash_firework_explosion(explosion: &FireworkExplosionSummary) -> u32 {
    let mut fields = vec![("shape", hash_string(explosion.shape.as_str()))];
    if !explosion.colors.is_empty() {
        fields.push((
            "colors",
            hash_list(explosion.colors.iter().copied().map(hash_int)),
        ));
    }
    if !explosion.fade_colors.is_empty() {
        fields.push((
            "fade_colors",
            hash_list(explosion.fade_colors.iter().copied().map(hash_int)),
        ));
    }
    if explosion.has_trail {
        fields.push(("has_trail", hash_boolean(true)));
    }
    if explosion.has_twinkle {
        fields.push(("has_twinkle", hash_boolean(true)));
    }
    hash_record(fields)
}

fn hash_fireworks(patch: &DataComponentPatchSummary) -> Option<u32> {
    let flight = patch.fireworks_flight_duration?;
    if !(0..=255).contains(&flight) || patch.fireworks_explosions.len() > 256 {
        return None;
    }
    let mut fields = Vec::new();
    // `Fireworks.java:22`, `ExtraCodecs.java:143-147`, `HashOps.java:99-101`:
    // `UNSIGNED_BYTE` encodes via `Codec.BYTE` (`createByte`).
    if flight != 0 {
        fields.push(("flight_duration", hash_byte(flight as i8)));
    }
    if !patch.fireworks_explosions.is_empty() {
        fields.push((
            "explosions",
            hash_list(
                patch
                    .fireworks_explosions
                    .iter()
                    .map(hash_firework_explosion),
            ),
        ));
    }
    Some(hash_record(fields))
}

fn hash_lodestone_tracker(patch: &DataComponentPatchSummary) -> Option<u32> {
    if patch.lodestone_target.is_none() && patch.lodestone_tracked.is_none() {
        return None;
    }
    let mut fields = Vec::new();
    if let Some(target) = &patch.lodestone_target {
        fields.push((
            "target",
            hash_record([
                (
                    "dimension",
                    hash_string(&normalize_codec_identifier(&target.dimension)?),
                ),
                (
                    "pos",
                    hash_int_array([target.pos.x, target.pos.y, target.pos.z]),
                ),
            ]),
        ));
    }
    if patch.lodestone_tracked == Some(false) {
        fields.push(("tracked", hash_boolean(false)));
    }
    Some(hash_record(fields))
}

fn hash_debug_stick(lookup: &dyn ItemHashLookup, patch: &DataComponentPatchSummary) -> Option<u32> {
    let mut pairs = Vec::with_capacity(patch.debug_stick_properties.len());
    for prop in &patch.debug_stick_properties {
        let block_id =
            lookup.resolve_holder_resource_id("minecraft:block", prop.block_registry_id)?;
        pairs.push((hash_string(&block_id), hash_string(&prop.property)));
    }
    Some(hash_map(pairs))
}

fn hash_typed_entity_data(
    lookup: &dyn ItemHashLookup,
    registry_id: &str,
    holder_id: i32,
    tag: &NbtSummaryValue,
) -> Option<u32> {
    let named = lookup.resolve_holder_resource_id(registry_id, holder_id)?;
    hash_typed_entity_data_named(&named, tag)
}

fn hash_typed_entity_data_named(named: &str, tag: &NbtSummaryValue) -> Option<u32> {
    let NbtSummaryValue::Compound(entries) = tag else {
        return None;
    };
    if entries.iter().any(|e| e.name == "id") {
        return None;
    }
    let mut combined = Vec::with_capacity(entries.len() + 1);
    combined.push(NbtSummaryEntry {
        name: "id".into(),
        value: NbtSummaryValue::string(named),
    });
    combined.extend(entries.iter().cloned());
    Some(hash_nbt(&NbtSummaryValue::Compound(combined), false))
}

fn hash_bees(lookup: &dyn ItemHashLookup, patch: &DataComponentPatchSummary) -> Option<u32> {
    if !patch.bees_present {
        return None;
    }
    let mut occupants = Vec::with_capacity(patch.bees.len());
    for occ in &patch.bees {
        let entity_data = hash_typed_entity_data(
            lookup,
            "minecraft:entity_type",
            occ.entity_type_id,
            occ.entity_data_tag.as_ref()?,
        )?;
        occupants.push(hash_record([
            ("entity_data", entity_data),
            ("ticks_in_hive", hash_int(occ.ticks_in_hive)),
            ("min_ticks_in_hive", hash_int(occ.min_ticks_in_hive)),
        ]));
    }
    Some(hash_list(occupants))
}

fn hash_trim(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    depth: usize,
) -> Option<u32> {
    let material = if let Some(mat) = patch.armor_trim_material_direct.as_deref() {
        if patch.armor_trim_material_id.is_some() {
            return None;
        }
        hash_inline_trim_material(lookup, mat, depth)?
    } else {
        let id = patch.armor_trim_material_id?;
        hash_string(&lookup.resolve_holder_resource_id("minecraft:trim_material", id)?)
    };
    let pattern = if let Some(pat) = patch.armor_trim_pattern_direct.as_deref() {
        if patch.armor_trim_pattern_id.is_some() {
            return None;
        }
        hash_inline_trim_pattern(lookup, pat, depth)?
    } else {
        let id = patch.armor_trim_pattern_id?;
        hash_string(&lookup.resolve_holder_resource_id("minecraft:trim_pattern", id)?)
    };
    Some(hash_record([("material", material), ("pattern", pattern)]))
}

fn hash_provides_trim_material(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    depth: usize,
) -> Option<u32> {
    if let Some(mat) = patch.provides_trim_material_direct.as_deref() {
        if patch.provides_trim_material_id.is_some() {
            return None;
        }
        return hash_inline_trim_material(lookup, mat, depth);
    }
    let id = patch.provides_trim_material_id?;
    Some(hash_string(
        &lookup.resolve_holder_resource_id("minecraft:trim_material", id)?,
    ))
}

fn hash_inline_trim_material(
    lookup: &dyn ItemHashLookup,
    mat: &TrimMaterialSummary,
    depth: usize,
) -> Option<u32> {
    let mut fields = vec![("asset_name", hash_string(&mat.asset_name))];
    if !mat.override_armor_assets.is_empty() {
        let mut overrides = Vec::with_capacity(mat.override_armor_assets.len());
        for (k, v) in &mat.override_armor_assets {
            overrides.push((hash_string(&normalize_codec_identifier(k)?), hash_string(v)));
        }
        fields.push(("override_armor_assets", hash_map(overrides)));
    }
    fields.push((
        "description",
        hash_component_at_depth(lookup, mat.description_nbt.as_deref()?, depth)?,
    ));
    Some(hash_record(fields))
}

fn hash_inline_trim_pattern(
    lookup: &dyn ItemHashLookup,
    pat: &TrimPatternSummary,
    depth: usize,
) -> Option<u32> {
    // `TrimPattern.DIRECT_CODEC` (`TrimPattern.java:20`) uses `fieldOf("decal").orElse(false)`,
    // so the fallback applies only on decode and encoding always writes `"decal"`.
    Some(hash_record([
        (
            "asset_id",
            hash_string(&normalize_codec_identifier(&pat.asset_id)?),
        ),
        (
            "description",
            hash_component_at_depth(lookup, pat.description_nbt.as_deref()?, depth)?,
        ),
        ("decal", hash_boolean(pat.decal)),
    ]))
}

fn hash_instrument(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    depth: usize,
) -> Option<u32> {
    match (
        &patch.instrument.registry_id,
        patch.instrument.direct.as_deref(),
    ) {
        (Some(id), None) => Some(hash_string(
            &lookup.resolve_holder_resource_id("minecraft:instrument", *id)?,
        )),
        (None, Some(inst)) => hash_inline_instrument(lookup, inst, depth),
        _ => None,
    }
}

fn hash_inline_instrument(
    lookup: &dyn ItemHashLookup,
    inst: &InstrumentSummary,
    depth: usize,
) -> Option<u32> {
    let use_dur = f32::from_bits(inst.use_duration_bits);
    let range = f32::from_bits(inst.range_bits);
    if !(use_dur > 0.0 && use_dur <= f32::MAX) || !(range > 0.0 && range <= f32::MAX) {
        return None;
    }
    Some(hash_record([
        ("sound_event", hash_sound_event(lookup, &inst.sound_event)?),
        ("use_duration", hash_float(inst.use_duration_bits)),
        ("range", hash_float(inst.range_bits)),
        (
            "description",
            hash_component_at_depth(lookup, inst.description_nbt.as_deref()?, depth)?,
        ),
    ]))
}

fn hash_banner_patterns(
    lookup: &dyn ItemHashLookup,
    layers: &[BannerPatternLayerSummary],
) -> Option<u32> {
    let mut encoded = Vec::with_capacity(layers.len());
    for layer in layers {
        let color = hash_checked_builtin_enum(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, layer.color_id)?;
        let pattern = match (layer.registry_id, layer.asset_id.as_deref()) {
            (Some(id), None) => {
                hash_string(&lookup.resolve_holder_resource_id("minecraft:banner_pattern", id)?)
            }
            (None, Some(asset_id)) => {
                let translation_key = layer.translation_key.as_ref()?;
                hash_record([
                    (
                        "asset_id",
                        hash_string(&normalize_codec_identifier(asset_id)?),
                    ),
                    ("translation_key", hash_java_string(translation_key)),
                ])
            }
            _ => return None,
        };
        encoded.push(hash_record([("pattern", pattern), ("color", color)]));
    }
    Some(hash_list(encoded))
}
