use super::*;
use std::sync::OnceLock;

use bbb_facts::{vanilla_map_decoration_type, vanilla_map_decoration_type_for_resource_id};

use crate::block_entity_types::vanilla_block_entity_type;
use crate::block_state_types::vanilla_block_has_property;
use crate::block_types::{vanilla_block_type, vanilla_block_type_for_resource_id};
use crate::data_component_types::{
    DataComponentTypeCatalogEntry, VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
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
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID, VANILLA_DATA_COMPONENT_TYPES,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
};
use crate::item_types::{vanilla_item_type, VANILLA_ITEM_BRICK_ID};
use crate::nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH};

type WireDecodeHandler =
    fn(i32, &mut Decoder<'_>, &mut DataComponentPatchSummary, &ContentRegistry) -> Result<()>;

#[derive(Clone, Copy)]
enum WireEncodeHandler {
    Vanilla(fn(i32, &DataComponentPatchSummary, &mut Encoder) -> Result<()>),
    Registry(fn(i32, &DataComponentPatchSummary, &mut Encoder, &ContentRegistry) -> Result<()>),
}

#[derive(Clone, Copy)]
pub struct DataComponentDescriptor {
    metadata: &'static DataComponentTypeCatalogEntry,
    decode: WireDecodeHandler,
    encode: WireEncodeHandler,
}

impl DataComponentDescriptor {
    pub const fn type_id(self) -> i32 {
        self.metadata.type_id
    }

    pub const fn resource_id(self) -> &'static str {
        self.metadata.resource_id
    }

    pub const fn is_persistent(self) -> bool {
        self.metadata.persistent
    }

    #[cfg(test)]
    pub(super) fn decode(
        self,
        decoder: &mut Decoder<'_>,
        summary: &mut DataComponentPatchSummary,
    ) -> Result<()> {
        self.decode_with_registry(decoder, summary, ContentRegistry::vanilla())
    }

    pub(super) fn decode_with_registry(
        self,
        decoder: &mut Decoder<'_>,
        summary: &mut DataComponentPatchSummary,
        registry: &ContentRegistry,
    ) -> Result<()> {
        (self.decode)(self.metadata.type_id, decoder, summary, registry)
    }

    #[cfg(test)]
    pub(super) fn encode(
        self,
        summary: &DataComponentPatchSummary,
        out: &mut Encoder,
    ) -> Result<()> {
        self.encode_with_registry(summary, out, ContentRegistry::vanilla())
    }

    pub(super) fn encode_with_registry(
        self,
        summary: &DataComponentPatchSummary,
        out: &mut Encoder,
        registry: &ContentRegistry,
    ) -> Result<()> {
        match self.encode {
            WireEncodeHandler::Vanilla(encode) => encode(self.metadata.type_id, summary, out),
            WireEncodeHandler::Registry(encode) => {
                encode(self.metadata.type_id, summary, out, registry)
            }
        }
    }
}

static WIRE_DECODE_HANDLERS: &[WireDecodeHandler] = &[
    decode_custom_data,
    decode_max_stack_size,
    decode_max_damage,
    decode_damage,
    decode_unbreakable,
    decode_use_effects_component,
    decode_custom_name,
    decode_minimum_attack_charge,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID>,
    decode_item_name,
    decode_item_model,
    decode_lore_component,
    decode_rarity,
    decode_enchantments,
    decode_can_place_on,
    decode_can_break,
    decode_attribute_modifiers_component,
    decode_custom_model_data_component,
    decode_tooltip_display_component,
    decode_repair_cost,
    decode_creative_slot_lock,
    decode_enchantment_glint_override,
    decode_intangible_projectile,
    decode_food_component,
    decode_consumable_component,
    decode_use_remainder_component,
    decode_use_cooldown_component,
    decode_damage_resistant,
    decode_tool_component,
    decode_weapon_component,
    decode_attack_range_component,
    decode_enchantable,
    decode_equippable_component,
    decode_repairable,
    decode_glider,
    decode_tooltip_style,
    decode_death_protection_component,
    decode_blocks_attacks_component,
    decode_piercing_weapon_component,
    decode_kinetic_weapon_component,
    decode_swing_animation_component,
    decode_additional_trade_cost,
    decode_stored_enchantments,
    decode_dye,
    decode_dyed_color,
    decode_map_color,
    decode_map_id,
    decode_map_decorations,
    decode_map_post_processing_component,
    decode_charged_projectiles,
    decode_bundle_contents,
    decode_potion_contents_component,
    decode_potion_duration_scale,
    decode_suspicious_stew_effects_component,
    decode_writable_book_content_component,
    decode_written_book_content_component,
    decode_trim,
    decode_debug_stick_state,
    decode_entity_data,
    decode_bucket_entity_data,
    decode_block_entity_data,
    decode_instrument,
    decode_provides_trim_material,
    decode_ominous_bottle_amplifier,
    decode_jukebox_playable_component,
    decode_provides_banner_patterns,
    decode_recipes,
    decode_lodestone_tracker_component,
    decode_firework_explosion_component,
    decode_fireworks_component,
    decode_profile,
    decode_note_block_sound,
    decode_banner_patterns,
    decode_base_color,
    decode_pot_decorations_component,
    decode_container,
    decode_block_state,
    decode_bees_component,
    decode_lock,
    decode_container_loot,
    decode_break_sound,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID>,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_tropical_fish_pattern,
    decode_tropical_fish_base_color,
    decode_tropical_fish_pattern_color,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID>,
    decode_builtin_enum,
    decode_painting_variant,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID>,
    decode_registry_holder_component::<VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID>,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
];

static VANILLA_DATA_COMPONENTS: OnceLock<Vec<DataComponentDescriptor>> = OnceLock::new();

fn initialized_data_component_descriptors() -> &'static [DataComponentDescriptor] {
    VANILLA_DATA_COMPONENTS
        .get_or_init(|| {
            assert_eq!(
                VANILLA_DATA_COMPONENT_TYPES.len(),
                WIRE_DECODE_HANDLERS.len()
            );
            VANILLA_DATA_COMPONENT_TYPES
                .iter()
                .zip(WIRE_DECODE_HANDLERS.iter().copied())
                .map(|(metadata, decode)| DataComponentDescriptor {
                    metadata,
                    decode,
                    encode: wire_encode_handler(metadata.type_id),
                })
                .collect()
        })
        .as_slice()
}

pub fn vanilla_data_component_descriptors() -> &'static [DataComponentDescriptor] {
    initialized_data_component_descriptors()
}

pub fn vanilla_data_component_descriptor(type_id: i32) -> Option<&'static DataComponentDescriptor> {
    let index = usize::try_from(type_id).ok()?;
    let descriptor = initialized_data_component_descriptors().get(index)?;
    (descriptor.type_id() == type_id).then_some(descriptor)
}

/// Encodes exactly one component value through the descriptor's Vanilla
/// network codec, without the surrounding patch counts/type id.
///
/// Component-select equality uses this only after its persistent case value
/// has been decoded into the same typed summary. The component catalog remains
/// the single dispatch table for both ordinary packet encoding and this
/// value-level projection.
pub fn encode_data_component_value_summary(
    type_id: i32,
    summary: &DataComponentPatchSummary,
) -> Result<Vec<u8>> {
    encode_data_component_value_summary_with_registry(type_id, summary, ContentRegistry::vanilla())
}

pub fn encode_data_component_value_summary_with_registry(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<Vec<u8>> {
    let descriptor = vanilla_data_component_descriptor(type_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("unknown data component type id {type_id}"))
    })?;
    let mut out = Encoder::new();
    descriptor.encode_with_registry(summary, &mut out, registry)?;
    Ok(out.into_inner())
}

fn wire_encode_handler(type_id: i32) -> WireEncodeHandler {
    use WireEncodeHandler::{Registry as R, Vanilla as V};
    match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => V(encode_custom_data),
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => V(encode_max_stack_size),
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => V(encode_max_damage),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => V(encode_damage),
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => V(encode_unbreakable),
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => V(encode_use_effects),
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID | VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
            R(encode_name_component)
        }
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => V(encode_minimum_attack_charge),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => V(encode_item_model),
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => V(encode_tooltip_style),
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => R(encode_lore),
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => V(encode_rarity),
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
        | VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => V(encode_item_enchantments),
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
            R(encode_adventure_mode_predicate)
        }
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => R(encode_attribute_modifiers),
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => V(encode_custom_model_data),
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => V(encode_tooltip_display),
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => V(encode_repair_cost),
        VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID => V(encode_creative_slot_lock),
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            V(encode_enchantment_glint_override)
        }
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID => V(encode_intangible_projectile),
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => V(encode_food),
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => V(encode_consumable),
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => R(encode_use_remainder),
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => V(encode_use_cooldown),
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => V(encode_damage_resistant),
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => R(encode_tool),
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => V(encode_weapon),
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => V(encode_attack_range),
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => V(encode_enchantable),
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => R(encode_equippable),
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => R(encode_repairable),
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => V(encode_glider),
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => V(encode_death_protection),
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => V(encode_blocks_attacks),
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => V(encode_piercing_weapon),
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => V(encode_kinetic_weapon),
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => V(encode_swing_animation),
        VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID => V(encode_additional_trade_cost),
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => V(encode_writable_book_content),
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => R(encode_written_book_content),
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => R(encode_trim),
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => R(encode_debug_stick_state),
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => V(encode_firework_explosion),
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => V(encode_fireworks),
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => R(encode_charged_projectiles),
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => R(encode_bundle_contents),
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => V(encode_potion_contents),
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => V(encode_potion_duration_scale),
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => V(encode_suspicious_stew_effects),
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => R(encode_entity_data),
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => V(encode_bucket_entity_data),
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => R(encode_block_entity_data),
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => R(encode_instrument),
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => R(encode_jukebox_playable),
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => V(encode_lodestone_tracker),
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => V(encode_profile),
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID => V(encode_dye),
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => V(encode_dyed_color),
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => V(encode_map_color),
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => V(encode_map_id),
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID => V(encode_map_decorations),
        VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID => V(encode_map_post_processing),
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => V(encode_note_block_sound),
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => V(encode_banner_patterns),
        VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => V(encode_base_color),
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => {
            V(encode_provides_banner_patterns)
        }
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID => V(encode_recipes),
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => R(encode_pot_decorations),
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => R(encode_container),
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => V(encode_block_state),
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID => R(encode_bees),
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => R(encode_lock),
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID => V(encode_container_loot),
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => R(encode_painting_variant),
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<
                VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID,
            >)
        }
        VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => V(encode_tropical_fish_pattern),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => {
            V(encode_tropical_fish_base_color)
        }
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => {
            V(encode_tropical_fish_pattern_color)
        }
        VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<
                VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
            >)
        }
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => {
            V(encode_registry_holder_component::<VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID>)
        }
        VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID => V(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => R(encode_provides_trim_material),
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => {
            V(encode_ominous_bottle_amplifier)
        }
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => V(encode_break_sound),
        _ => unreachable!("known data component metadata must have an encoder"),
    }
}

mod encode;
pub use encode::adventure_mode_predicate_tooltip_summary;
pub(crate) use encode::*;

mod decode;
pub use decode::*;

#[cfg(test)]
mod tests;
