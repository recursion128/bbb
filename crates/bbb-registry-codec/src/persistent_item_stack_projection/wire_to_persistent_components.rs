//! The wire-to-persistent direction: one dispatch that rebuilds each
//! component's own persistent codec NBT from the summary's typed fields, then
//! re-validates it through the same validator the save decoder owns.

use std::collections::{BTreeMap, BTreeSet};

use bbb_facts::builtin_enum_component_values::{
    builtin_enum_component_value_name, normalize_builtin_enum_component_value_id,
};
use bbb_facts::data_component_types::{
    vanilla_data_component_type, VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
    VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
    VANILLA_DATA_COMPONENT_LORE_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID, VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
    VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
    VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID, VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
    VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    VANILLA_DATA_COMPONENT_TRIM_TYPE_ID, VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_facts::item_types::VANILLA_ITEM_BRICK_ID;
use bbb_facts::mob_effect_types::vanilla_mob_effect_type;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::NbtSummaryDecodeLimits;
use bbb_protocol::packets::{
    DataComponentPatchSummary, ItemEnchantmentSummary, SuspiciousStewEffectSummary,
    ToolRuleSummary, ToolSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::{
    attribute_modifier_components, builtin_enum_components, component_matchers,
    consumable_components, embedded_record_components, equippable_components,
    error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result},
    holder::{
        block_resource_id_in_registry, item_resource_id_in_registry,
        persistent_inline_trim_material_nbt, persistent_inline_trim_pattern_nbt, BLOCK_REGISTRY,
        ENCHANTMENT_REGISTRY, ITEM_REGISTRY, MOB_EFFECT_REGISTRY, TRIM_MATERIAL_REGISTRY,
        TRIM_PATTERN_REGISTRY,
    },
    holder_component_projection,
    nbt_support::{
        checked_float, persistent_flag_nbt, persistent_float_nbt, ComponentReader,
        PERSISTENT_POT_DECORATION_SIDES, PERSISTENT_SUSPICIOUS_STEW_DEFAULT_DURATION,
    },
    nested_item_stack, potion_contents, profile_components, simple_record_components,
    tag_and_data_components, variant_components, weapon_and_block_attacks,
};

/// The persistent codec NBT [`TYPED_PERSISTENT_ENCODED_COMPONENT_TYPE_IDS`]
/// transcribes from the summary's typed fields, or `None` for a component that
/// is not typed-encoded.
///
/// `depth` counts the enclosing `minecraft:can_break` / `minecraft:can_place_on`
/// predicates, because an exact-component map may pin one of them again
/// (`component_matchers.rs`). `item_depth` counts enclosing nested item stack
/// templates (`container`, `bundle_contents`, `charged_projectiles`,
/// `use_remainder`).
#[allow(clippy::too_many_arguments)]
pub(super) fn typed_persistent_component_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    depth: usize,
    item_depth: usize,
) -> Option<Result<NbtSummaryValue>> {
    let missing = || ItemStackPersistentProjectionError::UnsupportedComponent {
        type_id,
        resource_id,
    };
    let reader = ComponentReader::new(type_id, resource_id);
    Some(match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => patch
            .custom_name_nbt
            .clone()
            .ok_or_else(missing)
            .and_then(|value| {
                reader.text_with_registry(&value, "custom name", content_registry)?;
                Ok(value)
            }),
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => patch
            .item_name_nbt
            .clone()
            .ok_or_else(missing)
            .and_then(|value| {
                reader.text_with_registry(&value, "item name", content_registry)?;
                Ok(value)
            }),
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => (|| {
            if patch.lore_nbt.len() > 256 {
                return Err(reader.error(format!(
                    "lore has {} lines, more than 256",
                    patch.lore_nbt.len()
                )));
            }
            for line in &patch.lore_nbt {
                reader.text_with_registry(line, "lore line", content_registry)?;
            }
            Ok(NbtSummaryValue::List(patch.lore_nbt.clone()))
        })(),
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
            patch.max_stack_size.ok_or_else(missing).and_then(|value| {
                if !(1..=99).contains(&value) {
                    return Err(reader.error(format!("max stack size {value} is outside 1..=99")));
                }
                Ok(NbtSummaryValue::Int(value))
            })
        }
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => {
            patch.max_damage.ok_or_else(missing).and_then(|value| {
                if value < 1 {
                    return Err(
                        reader.error(format!("max damage {value} is outside 1..={}", i32::MAX))
                    );
                }
                Ok(NbtSummaryValue::Int(value))
            })
        }
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => {
            patch.repair_cost.ok_or_else(missing).and_then(|value| {
                if value < 0 {
                    return Err(
                        reader.error(format!("repair cost {value} is outside 0..={}", i32::MAX))
                    );
                }
                Ok(NbtSummaryValue::Int(value))
            })
        }
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => {
            if patch.unbreakable {
                Ok(NbtSummaryValue::Compound(Vec::new()))
            } else {
                Err(missing())
            }
        }
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => patch
            .enchantment_glint_override
            .map(persistent_flag_nbt)
            .ok_or_else(missing),
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => patch
            .item_model
            .as_deref()
            .ok_or_else(missing)
            .and_then(|value| {
                bbb_protocol::normalize_resource_location(value)
                    .map_err(|error| reader.error(error.to_string()))?;
                Ok(NbtSummaryValue::string(value))
            }),
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => patch
            .tooltip_style
            .as_deref()
            .ok_or_else(missing)
            .and_then(|value| {
                bbb_protocol::normalize_resource_location(value)
                    .map_err(|error| reader.error(error.to_string()))?;
                Ok(NbtSummaryValue::string(value))
            }),
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => patch
            .note_block_sound
            .as_deref()
            .ok_or_else(missing)
            .and_then(|value| {
                bbb_protocol::normalize_resource_location(value)
                    .map_err(|error| reader.error(error.to_string()))?;
                Ok(NbtSummaryValue::string(value))
            }),
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => patch
            .ominous_bottle_amplifier
            .ok_or_else(missing)
            .and_then(|value| {
                if !(0..=4).contains(&value) {
                    return Err(reader.error(format!("amplifier {value} is outside 0..=4")));
                }
                Ok(NbtSummaryValue::Int(value))
            }),
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => patch
            .map_color
            .map(NbtSummaryValue::Int)
            .ok_or_else(missing),
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => patch
            .minimum_attack_charge_bits
            .ok_or_else(missing)
            .and_then(|bits| {
                checked_float(
                    type_id,
                    resource_id,
                    f32::from_bits(bits),
                    0.0,
                    1.0,
                    "attack charge",
                )?;
                Ok(persistent_float_nbt(bits))
            }),
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => patch
            .potion_duration_scale_bits
            .ok_or_else(missing)
            .and_then(|bits| {
                checked_float(
                    type_id,
                    resource_id,
                    f32::from_bits(bits),
                    0.0,
                    f32::MAX,
                    "duration scale",
                )?;
                Ok(persistent_float_nbt(bits))
            }),
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID => {
            if patch.intangible_projectile {
                Ok(NbtSummaryValue::Compound(Vec::new()))
            } else {
                Err(missing())
            }
        }
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => Ok(NbtSummaryValue::Compound(Vec::new())),
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => {
            persistent_trim_nbt(type_id, resource_id, patch, holder_resource_id)
        }
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => {
            persistent_block_state_nbt(&patch.block_state_properties)
        }
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => {
            persistent_tooltip_display_nbt(type_id, resource_id, patch)
        }
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => persistent_custom_model_data_nbt(patch),
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => patch
            .use_cooldown_seconds_bits
            .ok_or_else(missing)
            .and_then(|bits| {
                persistent_use_cooldown_nbt(
                    type_id,
                    resource_id,
                    bits,
                    patch.use_cooldown_group.as_deref(),
                )
            }),
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => patch
            .weapon_item_damage_per_attack
            .zip(patch.weapon_disable_blocking_seconds_bits)
            .ok_or_else(missing)
            .and_then(|(damage, seconds_bits)| {
                persistent_weapon_nbt(type_id, resource_id, damage, seconds_bits)
            }),
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => patch
            .firework_explosion
            .as_ref()
            .ok_or_else(missing)
            .map(super::firework_components::write_explosion),
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => patch
            .fireworks_flight_duration
            .ok_or_else(missing)
            .and_then(|flight_duration| {
                super::firework_components::write_fireworks(
                    type_id,
                    resource_id,
                    flight_duration,
                    &patch.fireworks_explosions,
                )
            }),
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => patch
            .rarity
            .map(|rarity| NbtSummaryValue::String(rarity.as_str().into()))
            .ok_or_else(missing),
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => patch
            .enchantable_value
            .ok_or_else(missing)
            .and_then(|value| {
                if value < 1 {
                    return Err(reader.error(format!(
                        "enchantable value {value} is outside 1..={}",
                        i32::MAX
                    )));
                }
                Ok(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "value".into(),
                    value: NbtSummaryValue::Int(value),
                }]))
            }),
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => persistent_enchantments_nbt(
            type_id,
            resource_id,
            &patch.stored_enchantments,
            holder_resource_id,
        ),
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => potion_contents::write(
            &patch.potion_contents,
            NbtSummaryDecodeLimits::default().max_depth,
        ),
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => persistent_container_nbt(
            type_id,
            resource_id,
            patch,
            holder_resource_id,
            content_registry,
            depth,
            item_depth,
        ),
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
            nested_item_stack::persistent_item_stack_template_list_nbt(
                type_id,
                resource_id,
                &patch.bundle_contents_items,
                holder_resource_id,
                content_registry,
                depth,
                item_depth,
            )
        }
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
            nested_item_stack::persistent_item_stack_template_list_nbt(
                type_id,
                resource_id,
                &patch.charged_projectiles_items,
                holder_resource_id,
                content_registry,
                depth,
                item_depth,
            )
        }
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => patch
            .use_remainder
            .as_deref()
            .ok_or_else(missing)
            .and_then(|remainder| {
                nested_item_stack::persistent_item_stack_template_nbt(
                    type_id,
                    resource_id,
                    remainder,
                    holder_resource_id,
                    content_registry,
                    depth,
                    item_depth,
                )
            }),
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => {
            patch.map_id.map(NbtSummaryValue::Int).ok_or_else(missing)
        }
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => patch
            .damage
            .ok_or_else(missing)
            .and_then(|damage| persistent_damage_nbt(type_id, resource_id, damage)),
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => persistent_enchantments_nbt(
            type_id,
            resource_id,
            &patch.enchantments,
            holder_resource_id,
        ),
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => patch
            .tool
            .as_ref()
            .ok_or_else(missing)
            .and_then(|tool| persistent_tool_nbt(type_id, resource_id, tool, content_registry)),
        // `Codec.withAlternative` encodes through its first codec
        // so a dyed colour is written as a bare int.
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => patch
            .dyed_color
            .map(NbtSummaryValue::Int)
            .ok_or_else(missing),
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => {
            persistent_suspicious_stew_effects_nbt(
                type_id,
                resource_id,
                &patch.suspicious_stew_effects,
            )
        }
        VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => patch
            .base_color_id
            .ok_or_else(missing)
            .and_then(|color_id| persistent_dye_color_nbt(type_id, resource_id, color_id)),
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => persistent_pot_decorations_nbt(
            type_id,
            resource_id,
            &patch.pot_decorations_item_ids,
            content_registry,
        ),
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => {
            super::book_components::write_writable(
                type_id,
                resource_id,
                &patch.writable_book_pages,
                &patch.writable_book_page_filters,
            )
        }
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => patch
            .written_book
            .as_ref()
            .ok_or_else(missing)
            .and_then(|content| {
                super::book_components::write_written(
                    type_id,
                    resource_id,
                    content,
                    &patch.written_book_pages_nbt,
                    &patch.written_book_page_filters_nbt,
                )
            }),
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => {
            attribute_modifier_components::persistent_attribute_modifiers_nbt(
                type_id,
                resource_id,
                &patch.attribute_modifiers,
            )
        }
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => patch
            .profile
            .as_ref()
            .ok_or_else(missing)
            .and_then(|profile| {
                profile_components::persistent_profile_nbt(type_id, resource_id, profile)
            }),
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => {
            consumable_components::persistent_food_nbt(type_id, resource_id, patch)
        }
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => {
            consumable_components::persistent_consumable_nbt(
                type_id,
                resource_id,
                patch,
                content_registry,
            )
        }
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => {
            holder_component_projection::persistent_break_sound_nbt(
                type_id,
                resource_id,
                patch,
                content_registry,
            )
        }
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => {
            holder_component_projection::persistent_piercing_weapon_nbt(
                type_id,
                resource_id,
                patch,
                content_registry,
            )
        }
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID
        | VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
            weapon_and_block_attacks::persistent_weapon_and_block_attacks_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
                content_registry,
            )
        }
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => {
            holder_component_projection::persistent_repairable_nbt(
                type_id,
                resource_id,
                patch,
                content_registry,
            )
        }
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => {
            // The current publication owns these holder names, not the built-in facts.
            holder_component_projection::persistent_damage_resistant_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
            )
        }
        // `TrimMaterial.CODEC` names a datapack-registry holder the publication
        // just answered with, so re-checking it against the static 26.1 facts
        // could only refuse a material the session legitimately issued, the
        // same reason `minecraft:trim` skips that check.
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => {
            holder_component_projection::persistent_provides_trim_material_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
            )
        }
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => {
            embedded_record_components::persistent_lock_nbt(type_id, resource_id, patch)
        }
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => {
            embedded_record_components::persistent_lodestone_tracker_nbt(
                type_id,
                resource_id,
                patch,
            )
        }
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => {
            equippable_components::persistent_equippable_nbt(
                type_id,
                resource_id,
                patch,
                content_registry,
            )
        }
        type_id if builtin_enum_components::is_builtin_enum_component(type_id) => {
            builtin_enum_components::persistent_builtin_enum_nbt(type_id, resource_id, patch)
        }
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID
        | VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => {
            builtin_enum_components::persistent_builtin_enum_nbt(type_id, resource_id, patch)
        }
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID
        | VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID
        | VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID
        | VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => {
            simple_record_components::persistent_simple_record_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
                content_registry,
            )
        }
        // `minecraft:damage_type`, `minecraft:instrument` and
        // `minecraft:jukebox_playable` name a datapack-registry holder the
        // publication just answered with, so like `minecraft:trim` they skip the
        // static 26.1 re-check, which could only refuse an entry the session
        // legitimately issued.
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID
        | VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID
        | VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => {
            simple_record_components::persistent_simple_record_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
                content_registry,
            )
        }
        // `minecraft:banner_pattern` is a datapack registry, so this skips the
        // static 26.1 re-check for the same reason `minecraft:trim` does.
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => {
            tag_and_data_components::persistent_banner_patterns_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
            )
        }
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => {
            tag_and_data_components::persistent_provides_banner_patterns_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
            )
        }
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => {
            tag_and_data_components::persistent_debug_stick_state_nbt(
                type_id,
                resource_id,
                patch,
                content_registry,
            )
        }
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID
        | VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
            tag_and_data_components::persistent_typed_data_nbt(
                type_id,
                resource_id,
                patch,
                content_registry,
            )
        }
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID => tag_and_data_components::persistent_bees_nbt(
            type_id,
            resource_id,
            patch,
            content_registry,
        ),
        // Like `minecraft:trim` and `minecraft:enchantments`, these name a
        // datapack-registry holder the publication just answered with, so
        // re-checking the name against the static 26.1 snapshot could only
        // refuse a variant the session legitimately issued.
        type_id if variant_components::variant_registry(type_id).is_some() => {
            variant_components::persistent_variant_nbt(
                type_id,
                resource_id,
                patch,
                holder_resource_id,
            )
        }
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => {
            let predicate = if type_id == VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID {
                patch.can_break_persistent.as_ref()
            } else {
                patch.can_place_on_persistent.as_ref()
            };
            predicate.ok_or_else(missing).and_then(|predicate| {
                component_matchers::persistent_adventure_mode_predicate_nbt(
                    type_id,
                    resource_id,
                    predicate,
                    holder_resource_id,
                    content_registry,
                    depth,
                    item_depth,
                )
            })
        }
        _ => return None,
    })
}

/// `Codec.intRange(1, 255)` guards every `ItemEnchantments` level
/// (`ItemEnchantments.java:32`).
const PERSISTENT_ENCHANTMENT_LEVELS: std::ops::RangeInclusive<i32> = 1..=255;

/// `ItemContainerContents.CODEC` writes
/// one `{slot, item}` record per occupied slot, in the ascending index order
/// `asSlots` walks.
///
/// The wire list also carries trailing empty slots (`container_slot_count`);
/// `fromSlots` and `asSlots` drop them on both sides
/// so they are not part of the save form,
/// but a summary whose slot span and slot list disagree is refused rather than
/// silently narrowed.
#[allow(clippy::too_many_arguments)]
fn persistent_container_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    predicate_depth: usize,
    item_depth: usize,
) -> Result<NbtSummaryValue> {
    let slot_count = patch.container.slot_count.ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "container is missing its exact slot count".to_string(),
        )
    })?;
    if slot_count > PERSISTENT_CONTAINER_SLOTS
        || patch.container.slots.len() > PERSISTENT_CONTAINER_SLOTS
    {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("container carries {slot_count} slots, at most {PERSISTENT_CONTAINER_SLOTS}"),
        ));
    }
    let mut previous_slot = None;
    let mut slots = Vec::with_capacity(patch.container.slots.len());
    for slot in &patch.container.slots {
        if !PERSISTENT_CONTAINER_SLOT_INDEXES.contains(&slot.slot) || slot.slot >= slot_count {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                format!("container slot {} is outside 0..=255", slot.slot),
            ));
        }
        if previous_slot.is_some_and(|previous| previous >= slot.slot) {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                "container slots must be strictly increasing".to_string(),
            ));
        }
        previous_slot = Some(slot.slot);
        let item = nested_item_stack::persistent_item_stack_template_nbt(
            type_id,
            resource_id,
            &slot.item,
            holder_resource_id,
            content_registry,
            predicate_depth,
            item_depth,
        )?;
        slots.push(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "slot".into(),
                value: NbtSummaryValue::Int(slot.slot as i32),
            },
            NbtSummaryEntry {
                name: "item".into(),
                value: item,
            },
        ]));
    }
    Ok(NbtSummaryValue::List(slots))
}

/// `ItemContainerContents.CODEC` is
/// `Slot.CODEC.sizeLimitedListOf(256)` (`ItemContainerContents.java:29`) over a
/// `Codec.intRange(0, 255)` index (`ItemContainerContents.java:178`).
const PERSISTENT_CONTAINER_SLOTS: usize = 256;
const PERSISTENT_CONTAINER_SLOT_INDEXES: std::ops::RangeInclusive<usize> = 0..=255;

/// `ExtraCodecs.NON_NEGATIVE_INT` (`ExtraCodecs.java:148`) validates while
/// encoding too, because `Codec.validate` runs on both directions
fn persistent_damage_nbt(
    type_id: i32,
    resource_id: &'static str,
    damage: i32,
) -> Result<NbtSummaryValue> {
    if damage < 0 {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("value must be non-negative: {damage}"),
        ));
    }
    Ok(NbtSummaryValue::Int(damage))
}

/// `ItemEnchantments.CODEC` is `Codec.unboundedMap(Enchantment.CODEC,
/// Codec.intRange(1, 255))` (`ItemEnchantments.java:32-37`), so the save form is
/// a compound keyed by each holder's identifier.
///
/// A `Object2IntOpenHashMap` cannot hold one holder twice
/// and the compound's own key order is not part of
/// the format, so this writes the keys sorted and
/// refuses a repeated one.
fn persistent_enchantments_nbt(
    type_id: i32,
    resource_id: &'static str,
    enchantments: &[ItemEnchantmentSummary],
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    let mut levels: BTreeMap<String, i32> = BTreeMap::new();
    for enchantment in enchantments {
        let name = holder_resource_id(ENCHANTMENT_REGISTRY, enchantment.holder_id).ok_or(
            ItemStackPersistentProjectionError::UnresolvedRegistryHolderName {
                registry_id: ENCHANTMENT_REGISTRY,
                holder_id: enchantment.holder_id,
            },
        )?;
        let name = bbb_protocol::normalize_resource_location(name).map_err(|error| {
            invalid_persistent_component(type_id, resource_id, error.to_string())
        })?;
        if !PERSISTENT_ENCHANTMENT_LEVELS.contains(&enchantment.level) {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                format!(
                    "enchantment level must be within range [1;255]: {}",
                    enchantment.level
                ),
            ));
        }
        if levels.insert(name.clone(), enchantment.level).is_some() {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                format!("duplicate enchantment {name}"),
            ));
        }
    }
    Ok(NbtSummaryValue::Compound(
        levels
            .into_iter()
            .map(|(name, level)| NbtSummaryEntry {
                name: name.into(),
                value: NbtSummaryValue::Int(level),
            })
            .collect(),
    ))
}

/// `SuspiciousStewEffects.CODEC` writes a
/// list of `Entry` records keyed by the effect's identifier
///
/// `lenientOptionalFieldOf(name, default)` drops a field that equals its
/// default while encoding, the rule [`persistent_tool_nbt`] already records, so
/// an entry at `DEFAULT_DURATION` writes `id` alone
fn persistent_suspicious_stew_effects_nbt(
    type_id: i32,
    resource_id: &'static str,
    effects: &[SuspiciousStewEffectSummary],
) -> Result<NbtSummaryValue> {
    let mut entries = Vec::with_capacity(effects.len());
    for effect in effects {
        let named = vanilla_mob_effect_type(effect.effect_id).ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!(
                    "unknown {MOB_EFFECT_REGISTRY} holder id {}",
                    effect.effect_id
                ),
            )
        })?;
        let mut fields = vec![NbtSummaryEntry {
            name: "id".into(),
            value: NbtSummaryValue::String(named.resource_id.into()),
        }];
        if effect.duration != PERSISTENT_SUSPICIOUS_STEW_DEFAULT_DURATION {
            fields.push(NbtSummaryEntry {
                name: "duration".into(),
                value: NbtSummaryValue::Int(effect.duration),
            });
        }
        entries.push(NbtSummaryValue::Compound(fields));
    }
    Ok(NbtSummaryValue::List(entries))
}

/// `DyeColor.CODEC` is `StringRepresentable.fromEnum`, so
/// the save form is the constant's `getSerializedName`
///
/// The catalog names a non-canonical id by clamping it onto a real constant,
/// which would invent a colour, so only an id that names itself is written.
fn persistent_dye_color_nbt(
    type_id: i32,
    resource_id: &'static str,
    color_id: i32,
) -> Result<NbtSummaryValue> {
    normalize_builtin_enum_component_value_id(type_id, color_id)
        .filter(|normalized| *normalized == color_id)
        .and_then(|color_id| builtin_enum_component_value_name(type_id, color_id))
        .map(|name| NbtSummaryValue::String(name.into()))
        .ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("dye color id {color_id} names no DyeColor constant"),
            )
        })
}

/// `PotDecorations.ordered` writes all four sides, `minecraft:brick` standing
/// for an empty one, and the list codec refuses a
/// fifth.
fn persistent_pot_decorations_nbt(
    type_id: i32,
    resource_id: &'static str,
    item_ids: &[i32],
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    if item_ids.len() > PERSISTENT_POT_DECORATION_SIDES {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!(
                "pot decorations carry {} sides, at most {PERSISTENT_POT_DECORATION_SIDES}",
                item_ids.len()
            ),
        ));
    }
    let mut sides = Vec::with_capacity(PERSISTENT_POT_DECORATION_SIDES);
    for index in 0..PERSISTENT_POT_DECORATION_SIDES {
        let item_id = item_ids
            .get(index)
            .copied()
            .unwrap_or(VANILLA_ITEM_BRICK_ID);
        let item = item_resource_id_in_registry(item_id, content_registry).ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("unknown {ITEM_REGISTRY} holder id {item_id}"),
            )
        })?;
        sides.push(NbtSummaryValue::String(item.into()));
    }
    Ok(NbtSummaryValue::List(sides))
}

/// `Tool.CODEC` over `Tool.Rule.CODEC`.
///
/// `optionalFieldOf(name, default)` drops a field that equals its default while
/// encoding, so a stock tool writes `rules` alone; `Codec.BOOL` reaches NBT as a
/// `ByteTag` and `FloatTag.valueOf` folds `-0.0` onto `+0.0`
fn persistent_tool_nbt(
    type_id: i32,
    resource_id: &'static str,
    tool: &ToolSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let mut rules = Vec::with_capacity(tool.rules.len());
    for rule in &tool.rules {
        let mut fields = vec![NbtSummaryEntry {
            name: "blocks".into(),
            value: persistent_tool_rule_blocks_nbt(type_id, resource_id, rule, content_registry)?,
        }];
        if let Some(bits) = rule.speed_bits {
            let speed = f32::from_bits(bits);
            // `ExtraCodecs.POSITIVE_FLOAT` (`ExtraCodecs`),
            // which also rejects NaN and infinity.
            if !(speed > 0.0 && speed <= f32::MAX) {
                return Err(invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("tool rule speed must be positive: {speed}"),
                ));
            }
            fields.push(NbtSummaryEntry {
                name: "speed".into(),
                value: NbtSummaryValue::Float(bits),
            });
        }
        if let Some(correct_for_drops) = rule.correct_for_drops {
            fields.push(NbtSummaryEntry {
                name: "correct_for_drops".into(),
                value: persistent_flag_nbt(correct_for_drops),
            });
        }
        rules.push(NbtSummaryValue::Compound(fields));
    }

    let mut fields = vec![NbtSummaryEntry {
        name: "rules".into(),
        value: NbtSummaryValue::List(rules),
    }];
    if tool.default_mining_speed_bits != 1.0_f32.to_bits() {
        fields.push(NbtSummaryEntry {
            name: "default_mining_speed".into(),
            value: persistent_float_nbt(tool.default_mining_speed_bits),
        });
    }
    if tool.damage_per_block < 0 {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("value must be non-negative: {}", tool.damage_per_block),
        ));
    }
    if tool.damage_per_block != 1 {
        fields.push(NbtSummaryEntry {
            name: "damage_per_block".into(),
            value: NbtSummaryValue::Int(tool.damage_per_block),
        });
    }
    if !tool.can_destroy_blocks_in_creative {
        fields.push(NbtSummaryEntry {
            name: "can_destroy_blocks_in_creative".into(),
            value: NbtSummaryValue::Byte(0),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// `RegistryCodecs.homogeneousList(Registries.BLOCK)`
/// which is `Codec.either` of a `"#tag"`
/// and
/// `ExtraCodecs.compactListCodec` -- the bare element for a one-holder set and a
/// list otherwise.
///
/// `Registries.BLOCK` is a built-in registry, so the summary's holder ids name
/// themselves without a session publication.
fn persistent_tool_rule_blocks_nbt(
    type_id: i32,
    resource_id: &'static str,
    rule: &ToolRuleSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    if let Some(tag) = &rule.block_tag {
        if !rule.block_registry_ids.is_empty() {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                format!("tool rule blocks carry both #{tag} and direct holders"),
            ));
        }
        let tag = bbb_protocol::normalize_resource_location(tag.as_str()).map_err(|error| {
            invalid_persistent_component(type_id, resource_id, error.to_string())
        })?;
        return Ok(NbtSummaryValue::String(format!("#{tag}").into()));
    }

    let mut blocks = Vec::with_capacity(rule.block_registry_ids.len());
    for &block_id in &rule.block_registry_ids {
        let block = block_resource_id_in_registry(block_id, content_registry).ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("unknown {BLOCK_REGISTRY} holder id {block_id}"),
            )
        })?;
        blocks.push(NbtSummaryValue::String(block.into()));
    }
    if blocks.len() == 1 {
        return Ok(blocks.pop().expect("one-element holder set"));
    }
    Ok(NbtSummaryValue::List(blocks))
}

fn list_field(name: &str, values: Vec<NbtSummaryValue>) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::List(values),
    }
}

/// `ArmorTrim.CODEC` requires a `material` and a
/// `pattern` holder. Both are `RegistryFileCodec`s, which write a named holder
/// as the element's identifier and an inline one as its record
///
/// The name written here is the one the publication that issued the summary's
/// holder id answers with, so membership is settled by construction.
fn persistent_trim_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    let material = match (
        patch.armor_trim_material_id,
        patch.armor_trim_material_direct.as_deref(),
    ) {
        (Some(material), None) => NbtSummaryValue::String(
            persistent_trim_holder_name(
                type_id,
                resource_id,
                TRIM_MATERIAL_REGISTRY,
                material,
                holder_resource_id,
            )?
            .into(),
        ),
        (None, Some(material)) => {
            persistent_inline_trim_material_nbt(type_id, resource_id, material)?
        }
        (None, None) => {
            return Err(ItemStackPersistentProjectionError::UnsupportedComponent {
                type_id,
                resource_id,
            })
        }
        (Some(_), Some(_)) => {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                "trim material has named and inline holders".to_string(),
            ))
        }
    };
    let pattern = match (
        patch.armor_trim_pattern_id,
        patch.armor_trim_pattern_direct.as_deref(),
    ) {
        (Some(pattern), None) => NbtSummaryValue::String(
            persistent_trim_holder_name(
                type_id,
                resource_id,
                TRIM_PATTERN_REGISTRY,
                pattern,
                holder_resource_id,
            )?
            .into(),
        ),
        (None, Some(pattern)) => persistent_inline_trim_pattern_nbt(type_id, resource_id, pattern)?,
        (None, None) => {
            return Err(ItemStackPersistentProjectionError::UnsupportedComponent {
                type_id,
                resource_id,
            })
        }
        (Some(_), Some(_)) => {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                "trim pattern has named and inline holders".to_string(),
            ))
        }
    };
    Ok(NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "material".into(),
            value: material,
        },
        NbtSummaryEntry {
            name: "pattern".into(),
            value: pattern,
        },
    ]))
}

fn persistent_trim_holder_name(
    type_id: i32,
    resource_id: &'static str,
    registry_id: &'static str,
    holder_id: i32,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<String> {
    let name = holder_resource_id(registry_id, holder_id).ok_or(
        ItemStackPersistentProjectionError::UnresolvedRegistryHolderName {
            registry_id,
            holder_id,
        },
    )?;
    bbb_protocol::normalize_resource_location(name)
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))
}

/// `BlockItemStateProperties.CODEC` is
/// `Codec.unboundedMap(Codec.STRING, Codec.STRING).xmap`
/// so the save form is one string field
/// per property and `EMPTY` is the empty compound.
fn persistent_block_state_nbt(
    properties: &bbb_protocol::packets::data_components::BlockStateProperties,
) -> Result<NbtSummaryValue> {
    Ok(NbtSummaryValue::Compound(
        properties
            .iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.clone(),
                value: value.to_nbt(),
            })
            .collect(),
    ))
}

/// `TooltipDisplay.CODEC`: `hide_tooltip`
/// defaults to `false` and `hidden_components` to the empty set, and
/// `optionalFieldOf` drops a field that holds its default while encoding.
///
/// `hiddenComponents` is a `SequencedSet`, so one
/// component cannot be listed twice; the summary keeps the wire decode's
/// first-seen order and a repeat is refused rather than collapsed.
fn persistent_tooltip_display_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
) -> Result<NbtSummaryValue> {
    let mut fields = Vec::new();
    if patch.tooltip_hide_tooltip {
        fields.push(NbtSummaryEntry {
            name: "hide_tooltip".into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    if !patch.tooltip_hidden_component_type_ids.is_empty() {
        let mut seen = BTreeSet::new();
        let mut names = Vec::with_capacity(patch.tooltip_hidden_component_type_ids.len());
        for &hidden_id in &patch.tooltip_hidden_component_type_ids {
            if !seen.insert(hidden_id) {
                return Err(invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("hidden component type id {hidden_id} is listed twice"),
                ));
            }
            let component = vanilla_data_component_type(hidden_id).ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("unknown data component type id {hidden_id}"),
                )
            })?;
            names.push(NbtSummaryValue::String(component.resource_id.into()));
        }
        fields.push(list_field("hidden_components", names));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// `CustomModelData.CODEC`: four optional lists
/// in the codec's declaration order, each of them dropped while encoding when
/// empty. `ExtraCodecs.RGB_COLOR_CODEC` encodes through its first codec
/// so a colour is a bare int.
fn persistent_custom_model_data_nbt(patch: &DataComponentPatchSummary) -> Result<NbtSummaryValue> {
    let mut fields = Vec::new();
    if !patch.custom_model_data_floats.is_empty() {
        fields.push(list_field(
            "floats",
            patch
                .custom_model_data_floats
                .iter()
                .map(|value| persistent_float_nbt(value.to_bits()))
                .collect(),
        ));
    }
    if !patch.custom_model_data_flags.is_empty() {
        fields.push(list_field(
            "flags",
            patch
                .custom_model_data_flags
                .iter()
                .copied()
                .map(persistent_flag_nbt)
                .collect(),
        ));
    }
    if !patch.custom_model_data_strings.is_empty() {
        fields.push(list_field(
            "strings",
            patch
                .custom_model_data_strings
                .iter()
                .map(|value| value.to_nbt())
                .collect(),
        ));
    }
    if !patch.custom_model_data_colors.is_empty() {
        fields.push(list_field(
            "colors",
            patch
                .custom_model_data_colors
                .iter()
                .map(|value| NbtSummaryValue::Int(*value))
                .collect(),
        ));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// `UseCooldown.CODEC`: `seconds` is required
/// (`ExtraCodecs.POSITIVE_FLOAT`) and `cooldown_group` is dropped when absent.
fn persistent_use_cooldown_nbt(
    type_id: i32,
    resource_id: &'static str,
    seconds_bits: u32,
    cooldown_group: Option<&str>,
) -> Result<NbtSummaryValue> {
    let seconds = f32::from_bits(seconds_bits);
    if !(seconds > 0.0 && seconds <= f32::MAX) {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("cooldown seconds {seconds} is not positive"),
        ));
    }
    let mut fields = vec![NbtSummaryEntry {
        name: "seconds".into(),
        value: persistent_float_nbt(seconds_bits),
    }];
    if let Some(group) = cooldown_group {
        let group = bbb_protocol::normalize_resource_location(group).map_err(|error| {
            invalid_persistent_component(type_id, resource_id, error.to_string())
        })?;
        fields.push(NbtSummaryEntry {
            name: "cooldown_group".into(),
            value: NbtSummaryValue::String(group.into()),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// `Weapon.CODEC`: `item_damage_per_attack` defaults to
/// `1` and `disable_blocking_for_seconds` to `0.0`, and `optionalFieldOf` drops
/// a field that holds its default while encoding.
///
/// `Float.equals` tells `-0.0` from `0.0`, so only the `0.0` bits are the
/// dropped default and the field written is the canonical `FloatTag` value
fn persistent_weapon_nbt(
    type_id: i32,
    resource_id: &'static str,
    item_damage_per_attack: i32,
    disable_blocking_seconds_bits: u32,
) -> Result<NbtSummaryValue> {
    if item_damage_per_attack < 0 {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!(
                "item damage per attack {item_damage_per_attack} is outside 0..={}",
                i32::MAX
            ),
        ));
    }
    checked_float(
        type_id,
        resource_id,
        f32::from_bits(disable_blocking_seconds_bits),
        0.0,
        f32::MAX,
        "disable blocking seconds",
    )?;
    let mut fields = Vec::new();
    if item_damage_per_attack != 1 {
        fields.push(NbtSummaryEntry {
            name: "item_damage_per_attack".into(),
            value: NbtSummaryValue::Int(item_damage_per_attack),
        });
    }
    if disable_blocking_seconds_bits != 0.0_f32.to_bits() {
        fields.push(NbtSummaryEntry {
            name: "disable_blocking_for_seconds".into(),
            value: persistent_float_nbt(disable_blocking_seconds_bits),
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}
