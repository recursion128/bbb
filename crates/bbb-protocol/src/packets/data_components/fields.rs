//! Field ownership and value operations for typed data components.

use serde::de::Error as _;
use serde::ser::SerializeMap as _;

use super::{DataComponentPatchSummary, FireworkExplosionSummary};
use crate::codec::{ProtocolError, Result};
use crate::data_component_types::*;
use crate::NbtSummaryValue;

macro_rules! component_fields {
    (
        flattened: [$($flat_id:ident => $flat:ident { $($key:ident => $member:ident),* }, [$($flat_map:ident),*];)*];
        maps: [$($all_map:ident),*];
        $($type_id:ident => [$($field:ident),*], [$($map:ident),*];)*
    ) => {
        impl serde::Serialize for DataComponentPatchSummary {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
                let mut output = serializer.serialize_map(None)?;
                output.serialize_entry("added", &self.added_type_ids.len())?;
                output.serialize_entry("added_type_ids", &self.added_type_ids)?;
                output.serialize_entry("removed_type_ids", &self.removed_type_ids)?;
                $(output.serialize_entry(stringify!($all_map), &self.$all_map)?;)*
                $($(output.serialize_entry(stringify!($field), &self.$field)?;)*)*
                $($(output.serialize_entry(stringify!($key), &self.$flat.$member)?;)*)*
                output.serialize_entry("can_break", &self.can_break_persistent.as_ref().map(super::catalog::adventure_mode_predicate_tooltip_summary))?;
                output.serialize_entry("can_place_on", &self.can_place_on_persistent.as_ref().map(super::catalog::adventure_mode_predicate_tooltip_summary))?;
                output.serialize_entry("container_items", &super::container::Items(&self.container.slots))?;
                output.serialize_entry("container_item_count", &self.container.slot_count.map(|_| self.container.slots.len()))?;
                output.serialize_entry("instrument_description", &self.instrument.direct.as_ref().map(|direct| &direct.description))?;
                output.serialize_entry("instrument_description_styled", &self.instrument.direct.as_ref().and_then(|direct| direct.description_styled.as_deref()))?;
                output.serialize_entry("use_cooldown_ticks", &self.use_cooldown_ticks())?;
                output.serialize_entry("potion_custom_effect_count", &self.added_type_ids.contains(&VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID).then_some(self.potion_contents.custom_effects.len()))?;
                output.serialize_entry("consumable_effect_count", &self.consumable.map(|_| self.consumable_effects.len()))?;
                output.serialize_entry("bundle_contents_item_count", &self.added_type_ids.contains(&VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID).then_some(self.bundle_contents_items.len()))?;
                output.serialize_entry("bees_count", &self.bees.len())?;
                output.serialize_entry("fireworks_explosions_count", &self.fireworks_flight_duration.map(|_| self.fireworks_explosions.len()))?;
                let (colors, fade_colors, shape, has_trail, has_twinkle) = match &self.firework_explosion {
                    Some(value) => (value.colors.as_slice(), value.fade_colors.as_slice(), Some(value.shape), Some(value.has_trail), Some(value.has_twinkle)),
                    None => (&[][..], &[][..], None, None, None),
                };
                output.serialize_entry("firework_explosion_colors", colors)?;
                output.serialize_entry("firework_explosion_fade_colors", fade_colors)?;
                output.serialize_entry("firework_explosion_shape", &shape)?;
                output.serialize_entry("firework_explosion_has_trail", &has_trail)?;
                output.serialize_entry("firework_explosion_has_twinkle", &has_twinkle)?;
                output.end()
            }
        }

        impl<'de> serde::Deserialize<'de> for DataComponentPatchSummary {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
                #[derive(serde::Deserialize)]
                #[serde(field_identifier)]
                #[allow(non_camel_case_types)]
                enum Field {
                    added, added_type_ids, removed_type_ids,
                    $($all_map,)*
                    $($($field,)*)*
                    $($($key,)*)*
                    firework_explosion_colors, firework_explosion_fade_colors,
                    firework_explosion_shape, firework_explosion_has_trail, firework_explosion_has_twinkle,
                    #[serde(other)]
                    Ignored,
                }
                struct PatchVisitor;
                impl<'de> serde::de::Visitor<'de> for PatchVisitor {
                    type Value = DataComponentPatchSummary;
                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("a data component patch")
                    }
                    fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> std::result::Result<Self::Value, M::Error> {
                        let mut patch = DataComponentPatchSummary::default();
                        let mut added: Option<usize> = None;
                        let mut removed_type_ids = None;
                        let mut colors = Vec::new();
                        let mut fade_colors = Vec::new();
                        let (mut shape, mut has_trail, mut has_twinkle) = (None, None, None);
                        while let Some(field) = map.next_key()? {
                            match field {
                                Field::added => added = Some(map.next_value()?),
                                Field::added_type_ids => patch.added_type_ids = map.next_value()?,
                                Field::removed_type_ids => removed_type_ids = Some(map.next_value()?),
                                $(Field::$all_map => patch.$all_map = map.next_value()?,)*
                                $($(Field::$field => patch.$field = map.next_value()?,)*)*
                                $($(Field::$key => patch.$flat.$member = map.next_value()?,)*)*
                                Field::firework_explosion_colors => colors = map.next_value()?,
                                Field::firework_explosion_fade_colors => fade_colors = map.next_value()?,
                                Field::firework_explosion_shape => shape = map.next_value()?,
                                Field::firework_explosion_has_trail => has_trail = map.next_value()?,
                                Field::firework_explosion_has_twinkle => has_twinkle = map.next_value()?,
                                Field::Ignored => { map.next_value::<serde::de::IgnoredAny>()?; }
                            }
                        }
                        let added = added.ok_or_else(|| M::Error::missing_field("added"))?;
                        if added != patch.added_type_ids.len() {
                            return Err(M::Error::custom("data component patch added count does not match type ids"));
                        }
                        patch.removed_type_ids = removed_type_ids.ok_or_else(|| M::Error::missing_field("removed_type_ids"))?;
                        patch.firework_explosion = match (shape, has_trail, has_twinkle) {
                            (None, None, None) if colors.is_empty() && fade_colors.is_empty() => None,
                            (Some(shape), Some(has_trail), Some(has_twinkle)) => Some(FireworkExplosionSummary {
                                shape, colors, fade_colors, has_trail, has_twinkle,
                            }),
                            _ => return Err(M::Error::custom("incomplete firework explosion fields")),
                        };
                        patch.container.validate().map_err(M::Error::custom)?;
                        Ok(patch)
                    }
                }
                deserializer.deserialize_map(PatchVisitor)
            }
        }
        component_fields! {
            maps: [$($all_map),*];
            $($flat_id => [$flat], [$($flat_map),*];)*
            $($type_id => [$($field),*], [$($map),*];)*
        }
    };
    (maps: [$($all_map:ident),*]; $($type_id:ident => [$($field:ident),*], [$($map:ident),*];)*) => {
        pub(super) fn fields_match_declared_components(
            patch: &DataComponentPatchSummary,
            membership: &[Option<bool>],
        ) -> bool {
            // Keep the ownership table exhaustive over the summary.
            let DataComponentPatchSummary {
                added_type_ids: _,
                removed_type_ids: _,
                $($all_map: _,)*
                $($($field: _,)*)*
            } = patch;
            $(let mut $all_map = 0_usize;)*
            $(
                if membership[$type_id as usize] == Some(false) {
                    $($map += usize::from(patch.$map.contains_key(&$type_id));)*
                } else if false $(|| !value_is_default(&patch.$field))* {
                    return false;
                }
            )*
            true $(&& $all_map == patch.$all_map.len())*
        }

        /// Copies a typed value, including retained persistent and resolved holder projections.
        /// The caller owns patch membership and input validation.
        pub fn copy_component_value(
            type_id: i32,
            source: &DataComponentPatchSummary,
            target: &mut DataComponentPatchSummary,
        ) -> Result<()> {
            match type_id {
                $($type_id => {
                    $(target.$field = source.$field.clone();)*
                    $(match source.$map.get(&type_id) {
                        Some(value) => { target.$map.insert(type_id, value.clone()); }
                        None => { target.$map.remove(&type_id); }
                    })*
                },)*
                _ => return Err(unknown_component(type_id)),
            }
            Ok(())
        }

        pub fn clear_component_value(type_id: i32, target: &mut DataComponentPatchSummary) -> Result<()> {
            match type_id {
                $($type_id => {
                    $(target.$field = Default::default();)*
                    $(target.$map.remove(&type_id);)*
                },)*
                _ => return Err(unknown_component(type_id)),
            }
            Ok(())
        }

        /// Compares one component without constructing or cloning patch summaries.
        pub fn component_values_equal(
            type_id: i32,
            left: &DataComponentPatchSummary,
            right: &DataComponentPatchSummary,
        ) -> Result<bool> {
            match type_id {
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID =>
                    return text_values_equal(left.custom_name_nbt.as_slice(), right.custom_name_nbt.as_slice()),
                VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID =>
                    return text_values_equal(left.item_name_nbt.as_slice(), right.item_name_nbt.as_slice()),
                VANILLA_DATA_COMPONENT_LORE_TYPE_ID => return text_values_equal(&left.lore_nbt, &right.lore_nbt),
                _ => {}
            }
            match type_id {
                $($type_id => Ok(true $(&& left.$field == right.$field)*
                    $(&& left.$map.get(&type_id) == right.$map.get(&type_id))*),)*
                _ => Err(unknown_component(type_id)),
            }
        }
    };
}

fn value_is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

fn unknown_component(type_id: i32) -> ProtocolError {
    ProtocolError::InvalidData(format!("unknown data component {type_id}"))
}

impl DataComponentPatchSummary {
    #[inline]
    pub fn use_cooldown_ticks(&self) -> Option<i32> {
        self.use_cooldown_seconds_bits
            .map(|bits| (f32::from_bits(bits) * 20.0) as i32)
    }

    #[inline]
    pub fn explicit_bundle_contents(&self) -> Option<&[crate::packets::ItemStackTemplateSummary]> {
        let type_id = VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID;
        (self.added_type_ids.contains(&type_id) && !self.removed_type_ids.contains(&type_id))
            .then_some(self.bundle_contents_items.as_slice())
    }
}

// Text codec forms such as "a" and {"text":"a"} represent the same value.
fn text_values_equal(left: &[NbtSummaryValue], right: &[NbtSummaryValue]) -> Result<bool> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (left, right) in left.iter().zip(right) {
        if !crate::component_codec_values_equal(left, right)? {
            return Ok(false);
        }
    }
    Ok(true)
}

component_fields! {
    flattened: [
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => container { container_slots => slots, container_slot_count => slot_count }, [];
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => instrument { instrument_id => registry_id, instrument_direct => direct }, [codec_backed_nbt];
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => firework_explosion {}, [];
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => potion_contents { potion_id => potion_id, potion_custom_color => custom_color, potion_custom_effects => custom_effects, potion_custom_name => custom_name }, [];
    ];
    maps: [codec_backed_nbt, builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => [lock_predicate], [];
    VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => [can_break_persistent], [];
    VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => [can_place_on_persistent], [];
    VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => [bundle_contents_items], [];
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => [charged_projectiles_items], [];
    VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => [use_remainder], [];
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => [profile], [];
    VANILLA_DATA_COMPONENT_BEES_TYPE_ID => [bees_present, bees], [];
    VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID => [container_loot, container_loot_table, container_loot_seed], [codec_backed_nbt];
    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => [enchantments], [];
    VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => [stored_enchantments], [];
    VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => [tool], [];
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => [blocks_attacks], [];
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => [kinetic_weapon], [];
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => [piercing_weapon], [];
    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => [equippable], [];
    VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => [attribute_modifiers], [];
    VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => [armor_trim_material_id, armor_trim_material_direct, armor_trim_pattern_id, armor_trim_pattern_direct], [codec_backed_nbt];
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => [provides_trim_material_id, provides_trim_material_direct], [codec_backed_nbt];
    VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => [jukebox_song_id, jukebox_direct_song], [];
    VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => [debug_stick_properties], [];
    VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID => [recipe_keys], [codec_backed_nbt];
    VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => [pot_decorations_item_ids], [];
    VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => [block_state_properties], [];
    VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID => [map_decorations], [codec_backed_nbt];
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => [damage_type_id], [];
    VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => [damage_resistant_type_registry_ids, damage_resistant_type_tag], [];
    VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => [repairable_item_registry_ids, repairable_item_tag], [];
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => [provides_banner_pattern_registry_ids, provides_banner_pattern_tag], [];
    VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => [break_sound], [];
    VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => [villager_variant_id], [];
    VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => [wolf_variant_id], [];
    VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => [wolf_sound_variant_id], [];
    VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => [pig_variant_id], [];
    VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => [pig_sound_variant_id], [];
    VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => [cow_variant_id], [];
    VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => [cow_sound_variant_id], [];
    VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => [chicken_variant_id], [];
    VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => [chicken_sound_variant_id], [];
    VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => [zombie_nautilus_variant_id], [];
    VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => [frog_variant_id], [];
    VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => [painting_variant_id, painting_variant_direct], [];
    VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => [cat_variant_id], [];
    VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => [cat_sound_variant_id], [];
    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => [custom_data], [];
    VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => [entity_data_entity_type_id, entity_data_tag], [];
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => [block_entity_data_type_id, block_entity_data_tag, block_entity_spawn_entity_type], [];
    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => [use_effects], [];
    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => [minimum_attack_charge_bits], [];
    VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => [custom_model_data_floats, custom_model_data_flags, custom_model_data_strings, custom_model_data_colors], [];
    VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => [tooltip_hide_tooltip, tooltip_hidden_component_type_ids], [];
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => [food_nutrition, food_saturation_bits, food_can_always_eat], [];
    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => [use_cooldown_seconds_bits, use_cooldown_group], [];
    VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => [weapon_item_damage_per_attack, weapon_disable_blocking_seconds_bits], [];
    VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => [attack_range], [];
    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => [swing_animation], [];
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => [potion_duration_scale_bits], [];
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => [writable_book_pages, writable_book_page_filters], [];
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => [fireworks_flight_duration, fireworks_explosions], [];
    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => [consumable, consumable_sound, consumable_has_consume_particles, consumable_effects], [];
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => [death_protection_effects], [];
    VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => [lodestone_target, lodestone_tracked], [];
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => [max_damage], [];
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => [damage], [];
    VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => [repair_cost], [];
    VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => [enchantable_value], [];
    VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => [map_color], [];
    VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => [map_id], [];
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => [ominous_bottle_amplifier], [];
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => [unbreakable], [];
    VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID => [intangible_projectile], [];
    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => [enchantment_glint_override], [];
    VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => [tooltip_style], [];
    VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => [note_block_sound], [];
    VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => [rarity], [];
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => [tropical_fish_pattern_id], [];
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => [tropical_fish_base_color_id], [];
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => [tropical_fish_pattern_color_id], [];
    VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID => [creative_slot_lock], [];
    VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID => [additional_trade_cost], [];
    VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID => [map_post_processing], [];
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => [suspicious_stew_effects], [];
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID => [dye_color_id], [];
    VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => [dyed_color], [];
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => [banner_pattern_layers], [];
    VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => [base_color_id], [];
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => [written_book, written_book_pages_nbt, written_book_page_filters_nbt], [];
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => [custom_name, custom_name_styled, custom_name_nbt], [];
    VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => [item_name, item_name_styled, item_name_nbt], [];
    VANILLA_DATA_COMPONENT_LORE_TYPE_ID => [lore, lore_styled, lore_nbt], [];
    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => [item_model], [];
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => [max_stack_size], [];
    VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => [], [codec_backed_nbt];
    VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => [], [];
    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID => [], [builtin_enum_component_ids];
    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID => [], [builtin_enum_component_ids];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_preserves_unrelated_component_values() {
        let source = DataComponentPatchSummary {
            max_stack_size: Some(16),
            damage: Some(7),
            ..DataComponentPatchSummary::default()
        };
        let mut target = DataComponentPatchSummary {
            repair_cost: Some(3),
            ..DataComponentPatchSummary::default()
        };

        copy_component_value(
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            &source,
            &mut target,
        )
        .unwrap();

        assert_eq!(target.max_stack_size, Some(16));
        assert_eq!(target.repair_cost, Some(3));
        assert_eq!(target.damage, None);
    }
}
