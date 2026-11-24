//! Dialog report fields projected from persistent component values.

use std::borrow::Cow;

use crate::registries::normalize_codec_identifier;
use bbb_protocol::{data_component_types::*, packets::DataComponentPatchSummary, NbtSummaryValue};
use rustc_hash::FxHashMap;

use super::{
    attribute_modifiers, bees, blocks_attacks, consume_effect, debug_stick, enchantments,
    equippable, holder_set, instrument, item_stack_collections, map_decorations, potion_contents,
    sound_event, tool, trim, typed_data, writable_book, written_book,
    DialogDataComponentReferenceState, DialogDataComponentValueState as Value, DialogDecodeContext,
    DialogStringMapEntryState,
};

type Projection = fn(
    &mut Cow<'_, DataComponentPatchSummary>,
    DialogDecodeContext<'_>,
    &NbtSummaryValue,
) -> Result<Value, String>;

fn missing_payload(component: &str) -> String {
    format!("missing dialog component payload for {component}")
}

fn missing_registry_access(component: &str) -> String {
    format!("missing registry access for {component}")
}

pub(super) fn component_projection(type_id: i32) -> Option<Projection> {
    let project: Projection = match type_id {
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => |patch, context, source| {
            item_stack_collections::project_item_stack_template_list(
                &patch.charged_projectiles_items,
                source,
                context,
            )
            .map(|items| Value::ChargedProjectiles { items })
        },
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => |patch, context, source| {
            item_stack_collections::project_item_stack_template_list(
                &patch.bundle_contents_items,
                source,
                context,
            )
            .map(|items| Value::BundleContents { items })
        },
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => |patch, context, source| {
            item_stack_collections::project_item_stack_template_slots(
                &patch.container,
                source,
                context,
            )
            .map(|slots| Value::Container { slots })
        },
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => |patch, context, source| {
            let remainder = patch
                .use_remainder
                .as_deref()
                .ok_or_else(|| missing_payload("minecraft:use_remainder"))?;
            super::super::project_item_stack_template(remainder, source, context)
                .map(|convert_into| Value::UseRemainder { convert_into })
        },
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => |patch, context, _| {
            holder_set::project_registry_holder_set(
                &patch.repairable_item_registry_ids,
                &patch.repairable_item_tag,
                "minecraft:item",
                context.registry_access,
            )
            .map(|items| Value::Repairable { items })
        },
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => |patch, context, _| {
            holder_set::project_registry_holder_set(
                &patch.provides_banner_pattern_registry_ids,
                &patch.provides_banner_pattern_tag,
                "minecraft:banner_pattern",
                context.registry_access,
            )
            .map(|patterns| Value::ProvidesBannerPatterns { patterns })
        },
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => |patch, context, _| {
            holder_set::project_registry_holder_set(
                &patch.damage_resistant_type_registry_ids,
                &patch.damage_resistant_type_tag,
                "minecraft:damage_type",
                context.registry_access,
            )
            .map(|types| Value::DamageResistant { types })
        },
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => |patch, context, _| {
            holder_set::project_banner_pattern_layers(
                &patch.banner_pattern_layers,
                context.registry_access,
            )
            .map(|layers| Value::BannerPatterns { layers })
        },
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => |patch, context, _| {
            let song_id = patch
                .jukebox_song_id
                .ok_or_else(|| missing_payload("minecraft:jukebox_playable"))?;
            let registry_access = context
                .registry_access
                .ok_or_else(|| missing_registry_access("minecraft:jukebox_playable"))?;
            holder_set::project_registry_holder(song_id, "minecraft:jukebox_song", registry_access)
                .map(|song| Value::JukeboxPlayable { song })
        },
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => |patch, context, _| {
            let variant_id = patch
                .painting_variant_id
                .ok_or_else(|| missing_payload("minecraft:painting_variant"))?;
            let registry_access = context
                .registry_access
                .ok_or_else(|| missing_registry_access("minecraft:painting_variant"))?;
            holder_set::project_registry_holder(
                variant_id,
                "minecraft:painting_variant",
                registry_access,
            )
            .map(|variant| Value::PaintingVariant { variant })
        },
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => |patch, context, _| {
            let damage_type_id = patch
                .damage_type_id
                .ok_or_else(|| missing_payload("minecraft:damage_type"))?;
            let registry_access = context
                .registry_access
                .ok_or_else(|| missing_registry_access("minecraft:damage_type"))?;
            holder_set::project_registry_holder(
                damage_type_id,
                "minecraft:damage_type",
                registry_access,
            )
            .map(|damage_type| Value::DamageType { damage_type })
        },
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => |patch, _, _| {
            Ok(Value::SuspiciousStewEffects {
                effects: holder_set::project_suspicious_stew_effects(
                    &patch.suspicious_stew_effects,
                )?,
            })
        },
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => |patch, context, _| {
            let content_registry = context.registry_access.map_or(
                bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
                |registries| registries.content_registry(),
            );
            Ok(Value::PotDecorations {
                decorations: holder_set::project_pot_decorations(
                    &patch.pot_decorations_item_ids,
                    content_registry,
                )?,
            })
        },
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => |patch, context, _| {
            trim::project_trim_material_holder(
                patch.provides_trim_material_id,
                patch.provides_trim_material_direct.as_deref(),
                context,
            )
            .map(|material| Value::ProvidesTrimMaterial { material })
        },
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => |patch, context, _| {
            Ok(Value::Trim {
                material: trim::project_trim_material_holder(
                    patch.armor_trim_material_id,
                    patch.armor_trim_material_direct.as_deref(),
                    context,
                )?,
                pattern: trim::project_trim_pattern_holder(
                    patch.armor_trim_pattern_id,
                    patch.armor_trim_pattern_direct.as_deref(),
                    context,
                )?,
            })
        },
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => |patch, context, _| {
            instrument::project_instrument_holder(&patch.instrument, context)
                .map(|instrument| Value::Instrument { instrument })
        },
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => |patch, context, _| {
            attribute_modifiers::project_attribute_modifiers(&patch.attribute_modifiers, context)
                .map(|modifiers| Value::AttributeModifiers { modifiers })
        },
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => |patch, _, _| {
            let profile = match patch {
                Cow::Borrowed(patch) => patch.profile.as_ref().cloned(),
                Cow::Owned(patch) => patch.profile.take(),
            }
            .ok_or_else(|| missing_payload("minecraft:profile"))?;
            Ok(Value::Profile { profile })
        },
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => |patch, context, _| {
            match patch {
                Cow::Borrowed(patch) => sound_event::project_sound_event_holder(
                    patch
                        .break_sound
                        .as_ref()
                        .ok_or_else(|| missing_payload("minecraft:break_sound"))?,
                    context.registry_access,
                ),
                Cow::Owned(patch) => sound_event::project_owned_sound_event_holder(
                    patch
                        .break_sound
                        .take()
                        .ok_or_else(|| missing_payload("minecraft:break_sound"))?,
                    context.registry_access,
                ),
            }
            .map(|sound| Value::BreakSound { sound })
        },
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => |patch, context, _| {
            written_book::project_written_book_content(patch, context)
                .map(|content| Value::WrittenBookContent { content })
        },
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => |patch, _, _| {
            Ok(Value::Integer {
                value: patch
                    .max_stack_size
                    .ok_or_else(|| missing_payload("minecraft:max_stack_size"))?,
            })
        },
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => |patch, _, _| {
            Ok(Value::Integer {
                value: patch
                    .max_damage
                    .ok_or_else(|| missing_payload("minecraft:max_damage"))?,
            })
        },
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => |patch, _, _| {
            Ok(Value::Integer {
                value: patch
                    .damage
                    .ok_or_else(|| missing_payload("minecraft:damage"))?,
            })
        },
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => |patch, _, _| {
            Ok(Value::Integer {
                value: patch
                    .repair_cost
                    .ok_or_else(|| missing_payload("minecraft:repair_cost"))?,
            })
        },
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => |patch, _, _| {
            Ok(Value::Integer {
                value: patch
                    .map_color
                    .ok_or_else(|| missing_payload("minecraft:map_color"))?,
            })
        },
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => |patch, _, _| {
            Ok(Value::Integer {
                value: patch
                    .map_id
                    .ok_or_else(|| missing_payload("minecraft:map_id"))?,
            })
        },
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => |patch, _, _| {
            Ok(Value::Integer {
                value: patch
                    .ominous_bottle_amplifier
                    .ok_or_else(|| missing_payload("minecraft:ominous_bottle_amplifier"))?,
            })
        },
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => |patch, _, _| {
            Ok(Value::Boolean {
                value: patch
                    .enchantment_glint_override
                    .ok_or_else(|| missing_payload("minecraft:enchantment_glint_override"))?,
            })
        },
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID
        | VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID
        | VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => |_, _, _| Ok(Value::Unit),
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => |patch, _, _| {
            Ok(Value::ResourceId {
                resource_id: patch
                    .item_model
                    .clone()
                    .ok_or_else(|| missing_payload("minecraft:item_model"))?,
            })
        },
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => |patch, _, _| {
            Ok(Value::ResourceId {
                resource_id: patch
                    .tooltip_style
                    .clone()
                    .ok_or_else(|| missing_payload("minecraft:tooltip_style"))?,
            })
        },
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => |patch, _, _| {
            Ok(Value::ResourceId {
                resource_id: patch
                    .note_block_sound
                    .clone()
                    .ok_or_else(|| missing_payload("minecraft:note_block_sound"))?,
            })
        },
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => |patch, _, _| {
            Ok(Value::Rarity {
                value: patch
                    .rarity
                    .ok_or_else(|| missing_payload("minecraft:rarity"))?,
            })
        },
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => |patch, _, _| {
            let value = patch
                .swing_animation
                .ok_or_else(|| missing_payload("minecraft:swing_animation"))?;
            Ok(Value::SwingAnimation {
                animation_type: value.animation_type,
                duration: value.duration,
            })
        },
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => |patch, _, _| {
            Ok(Value::BlockState {
                properties: patch
                    .block_state_properties
                    .iter()
                    .map(|(key, value)| DialogStringMapEntryState {
                        key: key.clone(),
                        value: value.clone(),
                    })
                    .collect(),
            })
        },
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => |patch, _, _| {
            Ok(Value::Food {
                nutrition: patch
                    .food_nutrition
                    .ok_or_else(|| missing_payload("minecraft:food.nutrition"))?,
                saturation_bits: patch
                    .food_saturation_bits
                    .ok_or_else(|| missing_payload("minecraft:food.saturation"))?,
                can_always_eat: patch.food_can_always_eat,
            })
        },
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => |patch, _, _| {
            Ok(Value::UseCooldown {
                seconds_bits: patch
                    .use_cooldown_seconds_bits
                    .ok_or_else(|| missing_payload("minecraft:use_cooldown"))?,
                cooldown_group: patch.use_cooldown_group.clone(),
            })
        },
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => |patch, _, _| {
            Ok(Value::Enchantable {
                value: patch
                    .enchantable_value
                    .ok_or_else(|| missing_payload("minecraft:enchantable"))?,
            })
        },
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => |patch, _, _| {
            let hidden_components = patch
                .tooltip_hidden_component_type_ids
                .iter()
                .map(|&type_id| {
                    let component = vanilla_data_component_type(type_id).ok_or_else(|| {
                        format!("unknown tooltip_display hidden component type id {type_id}")
                    })?;
                    Ok(DialogDataComponentReferenceState {
                        type_id,
                        resource_id: component.resource_id.to_string(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(Value::TooltipDisplay {
                hide_tooltip: patch.tooltip_hide_tooltip,
                hidden_components,
            })
        },
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => |patch, _, _| {
            Ok(Value::RgbColor {
                value: patch
                    .dyed_color
                    .ok_or_else(|| missing_payload("minecraft:dyed_color"))?,
            })
        },
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => |patch, _, _| {
            Ok(Value::CustomModelData {
                floats_bits: patch
                    .custom_model_data_floats
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
                flags: patch.custom_model_data_flags.clone(),
                strings: patch.custom_model_data_strings.clone(),
                colors: patch.custom_model_data_colors.clone(),
            })
        },
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => |patch, _, _| {
            Ok(Value::FireworkExplosion {
                explosion: patch
                    .firework_explosion
                    .clone()
                    .ok_or_else(|| missing_payload("minecraft:firework_explosion"))?,
            })
        },
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => |patch, _, _| {
            Ok(Value::Fireworks {
                flight_duration: patch
                    .fireworks_flight_duration
                    .ok_or_else(|| missing_payload("minecraft:fireworks"))?,
                explosions: patch.fireworks_explosions.clone(),
            })
        },
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => |patch, _, _| {
            Ok(Value::Float {
                bits: patch
                    .minimum_attack_charge_bits
                    .ok_or_else(|| missing_payload("minecraft:minimum_attack_charge"))?,
            })
        },
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => |patch, _, _| {
            Ok(Value::Float {
                bits: patch
                    .potion_duration_scale_bits
                    .ok_or_else(|| missing_payload("minecraft:potion_duration_scale"))?,
            })
        },
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => |patch, _, _| {
            let effects = patch
                .use_effects
                .as_ref()
                .ok_or_else(|| missing_payload("minecraft:use_effects"))?;
            Ok(Value::UseEffects {
                can_sprint: effects.can_sprint,
                interact_vibrations: effects.interact_vibrations,
                speed_multiplier_bits: effects.speed_multiplier.to_bits(),
            })
        },
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => |patch, _, _| {
            Ok(Value::Weapon {
                item_damage_per_attack: patch
                    .weapon_item_damage_per_attack
                    .ok_or_else(|| missing_payload("minecraft:weapon.item_damage_per_attack"))?,
                disable_blocking_for_seconds_bits: patch
                    .weapon_disable_blocking_seconds_bits
                    .ok_or_else(|| {
                        missing_payload("minecraft:weapon.disable_blocking_for_seconds")
                    })?,
            })
        },
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => |patch, _, _| {
            let range = patch
                .attack_range
                .as_ref()
                .ok_or_else(|| missing_payload("minecraft:attack_range"))?;
            Ok(Value::AttackRange {
                min_reach_bits: range.min_reach.to_bits(),
                max_reach_bits: range.max_reach.to_bits(),
                min_creative_reach_bits: range.min_creative_reach.to_bits(),
                max_creative_reach_bits: range.max_creative_reach.to_bits(),
                hitbox_margin_bits: range.hitbox_margin.to_bits(),
                mob_factor_bits: range.mob_factor.to_bits(),
            })
        },
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID => |patch, _, _| {
            Ok(Value::Recipes {
                resource_ids: patch
                    .recipe_keys
                    .clone()
                    .ok_or_else(|| missing_payload("minecraft:recipes"))?,
            })
        },
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID => |patch, _, _| {
            Ok(Value::ContainerLoot {
                loot_table: patch
                    .container_loot_table
                    .clone()
                    .ok_or_else(|| missing_payload("minecraft:container_loot.loot_table"))?,
                seed: patch
                    .container_loot_seed
                    .ok_or_else(|| missing_payload("minecraft:container_loot.seed"))?,
            })
        },
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => |patch, _, _| {
            Ok(Value::LodestoneTracker {
                target: patch.lodestone_target.clone(),
                tracked: patch
                    .lodestone_tracked
                    .ok_or_else(|| missing_payload("minecraft:lodestone_tracker"))?,
            })
        },
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => |patch, context, _| {
            sound_event::project_piercing_weapon(
                patch
                    .piercing_weapon
                    .as_ref()
                    .ok_or_else(|| missing_payload("minecraft:piercing_weapon"))?,
                context.registry_access,
            )
            .map(|weapon| Value::PiercingWeapon { weapon })
        },
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => |patch, context, _| {
            sound_event::project_kinetic_weapon(
                patch
                    .kinetic_weapon
                    .as_ref()
                    .ok_or_else(|| missing_payload("minecraft:kinetic_weapon"))?,
                context.registry_access,
            )
            .map(|weapon| Value::KineticWeapon { weapon })
        },
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => |patch, context, _| {
            blocks_attacks::project_blocks_attacks(
                patch
                    .blocks_attacks
                    .as_ref()
                    .ok_or_else(|| missing_payload("minecraft:blocks_attacks"))?,
                context.registry_access,
            )
            .map(|blocks_attacks| Value::BlocksAttacks { blocks_attacks })
        },
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID => |patch, context, _| {
            let content_registry = context.registry_access.map_or(
                bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
                |registries| registries.content_registry(),
            );
            Ok(Value::Bees {
                occupants: bees::project_bees(&patch.bees, content_registry)?,
            })
        },
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID => |patch, _, _| {
            Ok(Value::MapDecorations {
                decorations: map_decorations::project_map_decorations(
                    patch
                        .map_decorations
                        .as_deref()
                        .ok_or_else(|| missing_payload("minecraft:map_decorations"))?,
                ),
            })
        },
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => |patch, context, source| {
            let content_registry = context.registry_access.map_or(
                bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
                |registries| registries.content_registry(),
            );
            Ok(Value::DebugStickState {
                properties: source_map_order(
                    debug_stick::project_debug_stick_state(
                        &patch.debug_stick_properties,
                        content_registry,
                    )?,
                    source,
                    |row| &row.block.resource_id,
                )?,
            })
        },
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => |patch, context, source| {
            Ok(Value::Enchantments {
                entries: source_map_order(
                    enchantments::project_enchantments(
                        &patch.enchantments,
                        context.registry_access,
                    )?,
                    source,
                    |row| &row.enchantment.resource_id,
                )?,
            })
        },
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => |patch, context, source| {
            Ok(Value::StoredEnchantments {
                entries: source_map_order(
                    enchantments::project_enchantments(
                        &patch.stored_enchantments,
                        context.registry_access,
                    )?,
                    source,
                    |row| &row.enchantment.resource_id,
                )?,
            })
        },
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => |patch, context, _| {
            let content_registry = context.registry_access.map_or(
                bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
                |registries| registries.content_registry(),
            );
            Ok(Value::EntityData {
                entity: typed_data::project_entity_data(patch, content_registry)?,
            })
        },
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => |patch, _, _| {
            Ok(Value::BlockEntityData {
                block_entity: typed_data::project_block_entity_data(patch)?,
            })
        },
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => |patch, _, _| {
            Ok(Value::CustomData {
                data: patch
                    .custom_data
                    .as_ref()
                    .ok_or_else(|| missing_payload("minecraft:custom_data"))?
                    .clone(),
            })
        },
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => |patch, _, _| {
            Ok(Value::BucketEntityData {
                data: patch
                    .codec_backed_nbt
                    .get(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID)
                    .cloned()
                    .ok_or_else(|| missing_payload("minecraft:bucket_entity_data"))?,
            })
        },
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => |patch, context, _| {
            consume_effect::project_consumable(patch, context.registry_access)
                .map(|consumable| Value::Consumable { consumable })
        },
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => |patch, context, _| {
            consume_effect::project_effects(
                &patch.death_protection_effects,
                context.registry_access,
            )
            .map(|effects| Value::DeathProtection { effects })
        },
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => |patch, _, _| {
            Ok(Value::PotionContents {
                contents: potion_contents::project_potion_contents(patch),
            })
        },
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => |patch, context, _| {
            tool::project_tool(
                patch
                    .tool
                    .as_ref()
                    .ok_or_else(|| missing_payload("minecraft:tool"))?,
                context.registry_access,
            )
            .map(|tool| Value::Tool { tool })
        },
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => |patch, context, _| {
            equippable::project_equippable(
                patch
                    .equippable
                    .as_ref()
                    .ok_or_else(|| missing_payload("minecraft:equippable"))?,
                context.registry_access,
            )
            .map(|equippable| Value::Equippable { equippable })
        },
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => |patch, _, _| {
            Ok(Value::WritableBookContent {
                pages: writable_book::project_writable_book_content(patch),
            })
        },
        _ => return None,
    };
    Some(project)
}

// Canonical maps stay sorted; reports retain the source map's entry order.
fn source_map_order<T>(
    mut rows: Vec<T>,
    source: &NbtSummaryValue,
    resource_id: impl Fn(&T) -> &str,
) -> Result<Vec<T>, String> {
    if rows.len() < 2 {
        return Ok(rows);
    }
    let NbtSummaryValue::Compound(entries) = source else {
        return Err("expected compound source for canonical map".to_string());
    };
    let mut order = FxHashMap::with_capacity_and_hasher(entries.len(), Default::default());
    for (index, entry) in entries.iter().enumerate() {
        let name = entry
            .name
            .with_utf8(normalize_codec_identifier)
            .flatten()
            .ok_or_else(|| "invalid codec identifier in canonical map source".to_string())?;
        order.entry(name).or_insert(index);
    }
    rows.sort_unstable_by_key(|row| order.get(resource_id(row)).copied().unwrap_or(usize::MAX));
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_malformed_component_payloads_return_errors_instead_of_panicking() {
        let empty = DataComponentPatchSummary::default();
        let source = NbtSummaryValue::Compound(Vec::new());
        let context = DialogDecodeContext::with_registry_access(None, 0);
        let payload_requiring_types = [
            VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
            VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
            VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
            VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
            VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
            VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        ];
        for type_id in payload_requiring_types {
            let project = component_projection(type_id).expect("projection registered");
            let mut borrowed = Cow::Borrowed(&empty);
            assert!(
                project(&mut borrowed, context, &source).is_err(),
                "expected Err for empty borrowed patch on component type {type_id}"
            );
            let mut owned = Cow::Owned(DataComponentPatchSummary::default());
            assert!(
                project(&mut owned, context, &source).is_err(),
                "expected Err for empty owned patch on component type {type_id}"
            );
        }

        // Unknown hidden component id in tooltip_display returns Err instead of panicking.
        let mut bad_tooltip = Cow::Owned(DataComponentPatchSummary {
            tooltip_hidden_component_type_ids: vec![-999_999],
            ..DataComponentPatchSummary::default()
        });
        let tooltip_project = component_projection(VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID)
            .expect("tooltip_display projection");
        assert!(tooltip_project(&mut bad_tooltip, context, &source).is_err());

        // Non-compound source passed to source_map_order returns Err instead of panicking.
        let rows = vec!["minecraft:a", "minecraft:b"];
        assert!(source_map_order(rows, &NbtSummaryValue::Int(0), |row| row).is_err());
    }
}
