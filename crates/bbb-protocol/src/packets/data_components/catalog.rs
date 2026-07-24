use super::*;
use std::sync::OnceLock;

use bbb_render_types::{vanilla_map_decoration_type, vanilla_map_decoration_type_for_resource_id};

use crate::block_entity_types::vanilla_block_entity_type;
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
    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
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
    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
    VANILLA_DATA_COMPONENT_LORE_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
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

type WireDecodeHandler = fn(i32, &mut Decoder<'_>, &mut DataComponentPatchSummary) -> Result<()>;
type WireEncodeHandler = fn(i32, &DataComponentPatchSummary, &mut Encoder) -> Result<()>;

#[derive(Clone, Copy)]
pub struct DataComponentDescriptor {
    metadata: &'static DataComponentTypeCatalogEntry,
    decode: WireDecodeHandler,
    encode: Option<WireEncodeHandler>,
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

    pub(super) fn decode(
        self,
        decoder: &mut Decoder<'_>,
        summary: &mut DataComponentPatchSummary,
    ) -> Result<()> {
        (self.decode)(self.metadata.type_id, decoder, summary)
    }

    pub(crate) const fn can_encode(self) -> bool {
        self.encode.is_some()
    }

    pub(super) fn encode(
        self,
        summary: &DataComponentPatchSummary,
        out: &mut Encoder,
    ) -> Result<()> {
        let encode = self.encode.ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "cannot encode unsupported data component {} ({})",
                self.resource_id(),
                self.type_id()
            ))
        })?;
        encode(self.metadata.type_id, summary, out)
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
    decode_damage_type,
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
    skip_unit,
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
    decode_codec_backed_nbt,
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
    decode_codec_backed_nbt,
    decode_container_loot,
    decode_break_sound,
    decode_villager_variant,
    decode_wolf_variant,
    decode_wolf_sound_variant,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_tropical_fish_pattern,
    decode_tropical_fish_base_color,
    decode_tropical_fish_pattern_color,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_pig_variant,
    decode_pig_sound_variant,
    decode_cow_variant,
    decode_cow_sound_variant,
    decode_chicken_variant,
    decode_chicken_sound_variant,
    decode_zombie_nautilus_variant,
    decode_frog_variant,
    decode_builtin_enum,
    decode_painting_variant,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_cat_variant,
    decode_cat_sound_variant,
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

const fn wire_encode_handler(type_id: i32) -> Option<WireEncodeHandler> {
    match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => Some(encode_custom_data),
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => Some(encode_max_stack_size),
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => Some(encode_max_damage),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => Some(encode_damage),
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => Some(encode_unbreakable),
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => Some(encode_use_effects),
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID | VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
            Some(encode_name_component)
        }
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => Some(encode_minimum_attack_charge),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => Some(encode_damage_type),
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => Some(encode_item_model),
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => Some(encode_tooltip_style),
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => Some(encode_lore),
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => Some(encode_rarity),
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
        | VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => Some(encode_item_enchantments),
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
            Some(encode_adventure_mode_predicate)
        }
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => Some(encode_attribute_modifiers),
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => Some(encode_custom_model_data),
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => Some(encode_tooltip_display),
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => Some(encode_repair_cost),
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            Some(encode_enchantment_glint_override)
        }
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID => Some(encode_intangible_projectile),
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => Some(encode_food),
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => Some(encode_consumable),
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => Some(encode_use_remainder),
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => Some(encode_use_cooldown),
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => Some(encode_damage_resistant),
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => Some(encode_tool),
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => Some(encode_weapon),
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => Some(encode_attack_range),
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => Some(encode_enchantable),
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => Some(encode_equippable),
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => Some(encode_repairable),
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => Some(encode_glider),
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => Some(encode_death_protection),
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => Some(encode_blocks_attacks),
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => Some(encode_piercing_weapon),
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => Some(encode_kinetic_weapon),
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => Some(encode_swing_animation),
        VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID => Some(encode_additional_trade_cost),
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => Some(encode_writable_book_content),
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => Some(encode_written_book_content),
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => Some(encode_trim),
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => Some(encode_firework_explosion),
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => Some(encode_fireworks),
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => Some(encode_charged_projectiles),
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => Some(encode_bundle_contents),
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => Some(encode_potion_contents),
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => Some(encode_potion_duration_scale),
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => {
            Some(encode_suspicious_stew_effects)
        }
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => Some(encode_entity_data),
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => Some(encode_bucket_entity_data),
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => Some(encode_block_entity_data),
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => Some(encode_instrument),
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => Some(encode_jukebox_playable),
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => Some(encode_lodestone_tracker),
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => Some(encode_profile),
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID => Some(encode_dye),
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => Some(encode_dyed_color),
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => Some(encode_map_color),
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => Some(encode_map_id),
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID => Some(encode_map_decorations),
        VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID => Some(encode_map_post_processing),
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => Some(encode_note_block_sound),
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => Some(encode_banner_patterns),
        VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => Some(encode_base_color),
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => {
            Some(encode_provides_banner_patterns)
        }
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID => Some(encode_recipes),
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => Some(encode_pot_decorations),
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => Some(encode_container),
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => Some(encode_block_state),
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID => Some(encode_bees),
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID => Some(encode_container_loot),
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => Some(encode_painting_variant),
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => Some(encode_villager_variant),
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => Some(encode_wolf_variant),
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => Some(encode_wolf_sound_variant),
        VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => Some(encode_pig_variant),
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => Some(encode_pig_sound_variant),
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => Some(encode_cow_variant),
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => Some(encode_cow_sound_variant),
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => Some(encode_chicken_variant),
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => Some(encode_chicken_sound_variant),
        VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => Some(encode_tropical_fish_pattern),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => {
            Some(encode_tropical_fish_base_color)
        }
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => {
            Some(encode_tropical_fish_pattern_color)
        }
        VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            Some(encode_zombie_nautilus_variant)
        }
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => Some(encode_frog_variant),
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => Some(encode_cat_variant),
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => Some(encode_cat_sound_variant),
        VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID => Some(encode_builtin_enum),
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => {
            Some(encode_provides_trim_material)
        }
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => {
            Some(encode_ominous_bottle_amplifier)
        }
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => Some(encode_break_sound),
        _ => None,
    }
}

fn decode_custom_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.custom_data = Some(decode_required_compound_tag(decoder, "custom_data")?);
    Ok(())
}

fn encode_custom_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_required_compound_tag(summary.custom_data.as_ref(), "custom_data", out)
}

fn encode_name_component(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let (text, styled, nbt, what) = match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => (
            summary.custom_name.as_deref(),
            summary.custom_name_styled.as_deref(),
            summary.custom_name_nbt.as_ref(),
            "custom name",
        ),
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => (
            summary.item_name.as_deref(),
            summary.item_name_styled.as_deref(),
            summary.item_name_nbt.as_ref(),
            "item name",
        ),
        _ => unreachable!("name component encoder registered for type id {type_id}"),
    };
    let text = text.ok_or_else(|| ProtocolError::InvalidData(format!("{what} text is missing")))?;
    out.write_bytes(&encode_canonical_component_projection(
        nbt, text, styled, what,
    )?);
    Ok(())
}

fn encode_use_effects(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let effects = summary.use_effects.ok_or_else(|| {
        ProtocolError::InvalidData("missing use_effects component value".to_string())
    })?;
    out.write_bool(effects.can_sprint);
    out.write_bool(effects.interact_vibrations);
    out.write_f32(effects.speed_multiplier);
    Ok(())
}

fn encode_minimum_attack_charge(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let bits = summary.minimum_attack_charge_bits.ok_or_else(|| {
        ProtocolError::InvalidData("missing minimum_attack_charge component value".to_string())
    })?;
    out.write_f32(f32::from_bits(bits));
    Ok(())
}

fn encode_damage_type(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.damage_type_id, "damage_type", out)
}

fn encode_registry_holder_id(
    holder_id: Option<i32>,
    component: &'static str,
    out: &mut Encoder,
) -> Result<()> {
    let holder_id = holder_id.ok_or_else(|| {
        ProtocolError::InvalidData(format!("missing {component} component value"))
    })?;
    if holder_id < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "{component} has negative holder id {holder_id}"
        )));
    }
    out.write_var_i32(holder_id);
    Ok(())
}

fn encode_item_model(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let item_model = summary.item_model.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing item_model component value".to_string())
    })?;
    validate_canonical_resource_location(item_model, "item_model")?;

    let mut encoded = Encoder::new();
    encode_java_string(&mut encoded, item_model, MAX_STRING_CHARS)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_tooltip_style(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let tooltip_style = summary.tooltip_style.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing tooltip_style component value".to_string())
    })?;
    validate_canonical_resource_location(tooltip_style, "tooltip_style")?;

    let mut encoded = Encoder::new();
    encode_java_string(&mut encoded, tooltip_style, MAX_STRING_CHARS)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_note_block_sound(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let note_block_sound = summary.note_block_sound.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing note_block_sound component value".to_string())
    })?;
    validate_canonical_resource_location(note_block_sound, "note_block_sound")?;

    let mut encoded = Encoder::new();
    encode_java_string(&mut encoded, note_block_sound, MAX_STRING_CHARS)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_rarity(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let rarity = summary
        .rarity
        .ok_or_else(|| ProtocolError::InvalidData("missing rarity component value".to_string()))?;
    out.write_var_i32(rarity.id());
    Ok(())
}

fn encode_item_enchantments(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let (entries, what) = match type_id {
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => {
            (summary.enchantments.as_slice(), "enchantments")
        }
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => (
            summary.stored_enchantments.as_slice(),
            "stored_enchantments",
        ),
        _ => unreachable!("item enchantments encoder registered for type id {type_id}"),
    };
    if entries.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            entries.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(entries.len() as i32);
    let mut holder_ids = BTreeSet::new();
    for entry in entries {
        if entry.holder_id < 0 {
            return Err(ProtocolError::InvalidData(format!(
                "{what} has negative holder id {}",
                entry.holder_id
            )));
        }
        if !holder_ids.insert(entry.holder_id) {
            return Err(ProtocolError::InvalidData(format!(
                "{what} has duplicate holder id {}",
                entry.holder_id
            )));
        }
        if !(0..=u8::MAX.into()).contains(&entry.level) {
            return Err(ProtocolError::InvalidData(format!(
                "{what} level {} is outside network range 0..=255",
                entry.level
            )));
        }
        encoded.write_var_i32(entry.holder_id);
        encoded.write_var_i32(entry.level);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_custom_model_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    for (what, len) in [
        ("floats", summary.custom_model_data_floats.len()),
        ("flags", summary.custom_model_data_flags.len()),
        ("strings", summary.custom_model_data_strings.len()),
        ("colors", summary.custom_model_data_colors.len()),
    ] {
        if len > MAX_DATA_COMPONENT_LIST_ITEMS {
            return Err(ProtocolError::InvalidData(format!(
                "custom_model_data {what} list has {len} entries, maximum is \
                 {MAX_DATA_COMPONENT_LIST_ITEMS}"
            )));
        }
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(summary.custom_model_data_floats.len() as i32);
    for value in summary.custom_model_data_floats.iter().copied() {
        encoded.write_f32(value);
    }
    encoded.write_var_i32(summary.custom_model_data_flags.len() as i32);
    for value in &summary.custom_model_data_flags {
        encoded.write_bool(*value);
    }
    encoded.write_var_i32(summary.custom_model_data_strings.len() as i32);
    for value in &summary.custom_model_data_strings {
        encode_java_string(&mut encoded, value, MAX_STRING_CHARS)?;
    }
    encoded.write_var_i32(summary.custom_model_data_colors.len() as i32);
    for value in &summary.custom_model_data_colors {
        encoded.write_i32(*value);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_tooltip_display(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let hidden_components = &summary.tooltip_hidden_component_type_ids;
    if hidden_components.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            hidden_components.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut seen = BTreeSet::new();
    for type_id in hidden_components {
        if crate::data_component_types::vanilla_data_component_type(*type_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "tooltip_display has invalid hidden component type id {type_id}"
            )));
        }
        if !seen.insert(*type_id) {
            return Err(ProtocolError::InvalidData(format!(
                "tooltip_display has duplicate hidden component type id {type_id}"
            )));
        }
    }

    let mut encoded = Encoder::new();
    encoded.write_bool(summary.tooltip_hide_tooltip);
    encoded.write_var_i32(hidden_components.len() as i32);
    for type_id in hidden_components {
        encoded.write_var_i32(*type_id);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_enchantment_glint_override(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let override_value = summary.enchantment_glint_override.ok_or_else(|| {
        ProtocolError::InvalidData("missing enchantment_glint_override component value".to_string())
    })?;
    out.write_bool(override_value);
    Ok(())
}

fn encode_intangible_projectile(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if !summary.intangible_projectile {
        return Err(ProtocolError::InvalidData(
            "missing intangible_projectile component value".to_string(),
        ));
    }
    let unit = NbtSummaryValue::Compound(Vec::new());
    let encoded = encode_nbt_summary_bounded(Some(&unit), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

fn encode_tool(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let tool = summary
        .tool
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing tool component value".to_string()))?;
    if tool.rules.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            tool.rules.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    if tool.damage_per_block < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "tool damage_per_block {} is negative",
            tool.damage_per_block
        )));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(tool.rules.len() as i32);
    for (index, rule) in tool.rules.iter().enumerate() {
        encode_holder_set_summary(
            &mut encoded,
            &rule.block_registry_ids,
            rule.block_tag.as_deref(),
            NetworkHolderSetRegistry::Block,
            "tool rule block holder set",
        )?;
        match rule.speed_bits {
            Some(bits) => {
                let speed = f32::from_bits(bits);
                if !speed.is_finite() || speed <= 0.0 {
                    return Err(ProtocolError::InvalidData(format!(
                        "tool rule {index} speed {speed} is not a positive finite float"
                    )));
                }
                encoded.write_bool(true);
                encoded.write_f32(speed);
            }
            None => encoded.write_bool(false),
        }
        match rule.correct_for_drops {
            Some(correct_for_drops) => {
                encoded.write_bool(true);
                encoded.write_bool(correct_for_drops);
            }
            None => encoded.write_bool(false),
        }
    }
    encoded.write_f32(f32::from_bits(tool.default_mining_speed_bits));
    encoded.write_var_i32(tool.damage_per_block);
    encoded.write_bool(tool.can_destroy_blocks_in_creative);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_attack_range(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let range = summary.attack_range.ok_or_else(|| {
        ProtocolError::InvalidData("missing attack_range component value".to_string())
    })?;
    for (field, value, max) in [
        ("min_reach", range.min_reach, 64.0),
        ("max_reach", range.max_reach, 64.0),
        ("min_creative_reach", range.min_creative_reach, 64.0),
        ("max_creative_reach", range.max_creative_reach, 64.0),
        ("hitbox_margin", range.hitbox_margin, 1.0),
        ("mob_factor", range.mob_factor, 2.0),
    ] {
        if !value.is_finite() || value.total_cmp(&0.0).is_lt() || value.total_cmp(&max).is_gt() {
            return Err(ProtocolError::InvalidData(format!(
                "attack_range {field} {value} is outside persistent range 0..={max}"
            )));
        }
    }

    let mut encoded = Encoder::new();
    encoded.write_f32(range.min_reach);
    encoded.write_f32(range.max_reach);
    encoded.write_f32(range.min_creative_reach);
    encoded.write_f32(range.max_creative_reach);
    encoded.write_f32(range.hitbox_margin);
    encoded.write_f32(range.mob_factor);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_glider(
    _type_id: i32,
    _summary: &DataComponentPatchSummary,
    _out: &mut Encoder,
) -> Result<()> {
    Ok(())
}

fn encode_equippable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let equippable = summary.equippable.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing equippable component value".to_string())
    })?;
    if vanilla_equipment_slot_by_id(equippable.slot_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "invalid equippable slot id {}",
            equippable.slot_id
        )));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(equippable.slot_id);
    encode_sound_event_holder_summary(&mut encoded, &equippable.equip_sound)?;
    encode_optional_resource_location(
        &mut encoded,
        equippable.asset_id.as_deref(),
        "equippable asset_id",
    )?;
    encode_optional_resource_location(
        &mut encoded,
        equippable.camera_overlay.as_deref(),
        "equippable camera_overlay",
    )?;
    encode_optional_holder_set_summary(
        &mut encoded,
        equippable.allowed_entities.as_ref().map(|entities| {
            (
                entities.registry_ids.as_slice(),
                entities.tag.as_deref(),
                "equippable allowed_entities",
            )
        }),
        NetworkHolderSetRegistry::EntityType,
    )?;
    encoded.write_bool(equippable.dispensable);
    encoded.write_bool(equippable.swappable);
    encoded.write_bool(equippable.damage_on_hurt);
    encoded.write_bool(equippable.equip_on_interact);
    encoded.write_bool(equippable.can_be_sheared);
    encode_sound_event_holder_summary(&mut encoded, &equippable.shearing_sound)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_optional_resource_location(
    out: &mut Encoder,
    resource_id: Option<&str>,
    what: &str,
) -> Result<()> {
    let Some(resource_id) = resource_id else {
        out.write_bool(false);
        return Ok(());
    };
    validate_canonical_resource_location(resource_id, what)?;
    out.write_bool(true);
    encode_java_string(out, resource_id, MAX_STRING_CHARS)
}

fn encode_lore(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let line_count = summary.lore.len();
    if line_count > MAX_LORE_LINES {
        return Err(ProtocolError::PacketTooLarge(line_count, MAX_LORE_LINES));
    }
    if summary.lore_styled.len() != line_count || summary.lore_nbt.len() != line_count {
        return Err(ProtocolError::InvalidData(
            "lore component projections are not aligned".to_string(),
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(line_count as i32);
    for ((line, styled), nbt) in summary
        .lore
        .iter()
        .zip(&summary.lore_styled)
        .zip(&summary.lore_nbt)
    {
        encoded.write_bytes(&encode_canonical_component_projection(
            Some(nbt),
            line,
            Some(styled),
            "lore line",
        )?);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_adventure_mode_predicate(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let (tooltip, predicate, what) = match type_id {
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => (
            summary.can_place_on.as_ref(),
            summary.can_place_on_persistent.as_ref(),
            "can_place_on",
        ),
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => (
            summary.can_break.as_ref(),
            summary.can_break_persistent.as_ref(),
            "can_break",
        ),
        _ => unreachable!("adventure mode predicate encoder registered for type id {type_id}"),
    };
    let tooltip = tooltip
        .ok_or_else(|| ProtocolError::InvalidData(format!("missing {what} tooltip projection")))?;
    let predicate = predicate
        .ok_or_else(|| ProtocolError::InvalidData(format!("missing {what} component value")))?;
    let canonical_tooltip = adventure_mode_predicate_tooltip_summary(predicate);
    if tooltip != &canonical_tooltip {
        return Err(ProtocolError::InvalidData(format!(
            "{what} contains a noncanonical tooltip projection"
        )));
    }
    validate_adventure_mode_predicate_nesting(predicate, 0)?;

    let mut encoded = Encoder::new();
    encode_adventure_mode_predicate_value(&mut encoded, predicate)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn adventure_mode_predicate_tooltip_summary(
    predicate: &AdventureModePredicateDataSummary,
) -> AdventureModePredicateSummary {
    let mut summary = AdventureModePredicateSummary::default();
    for predicate in &predicate.predicates {
        let Some(blocks) = &predicate.blocks else {
            summary.unknown = true;
            continue;
        };
        summary.unresolved_block_tag_count += usize::from(blocks.tag.is_some());
        for registry_id in &blocks.registry_ids {
            if !summary.block_registry_ids.contains(registry_id) {
                summary.block_registry_ids.push(*registry_id);
            }
        }
    }
    summary
}

fn validate_adventure_mode_predicate_nesting(
    predicate: &AdventureModePredicateDataSummary,
    depth: usize,
) -> Result<()> {
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return Err(ProtocolError::InvalidData(format!(
            "adventure mode predicate nesting exceeds \
             {MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH}"
        )));
    }
    for block in &predicate.predicates {
        let Some(exact) = block.exact_components.as_deref() else {
            continue;
        };
        if let Some(nested) = exact.can_place_on_persistent.as_ref() {
            validate_adventure_mode_predicate_nesting(nested, depth + 1)?;
        }
        if let Some(nested) = exact.can_break_persistent.as_ref() {
            validate_adventure_mode_predicate_nesting(nested, depth + 1)?;
        }
    }
    Ok(())
}

fn encode_adventure_mode_predicate_value(
    out: &mut Encoder,
    predicate: &AdventureModePredicateDataSummary,
) -> Result<()> {
    if predicate.predicates.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            predicate.predicates.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    out.write_var_i32(predicate.predicates.len() as i32);
    for block in &predicate.predicates {
        encode_block_predicate(out, block)?;
    }
    Ok(())
}

fn encode_block_predicate(out: &mut Encoder, predicate: &BlockPredicateSummary) -> Result<()> {
    match &predicate.blocks {
        Some(blocks) => {
            out.write_bool(true);
            encode_holder_set_summary(
                out,
                &blocks.registry_ids,
                blocks.tag.as_deref(),
                NetworkHolderSetRegistry::Block,
                "block predicate holder set",
            )?;
        }
        None => out.write_bool(false),
    }

    match &predicate.state {
        Some(properties) => {
            if properties.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    properties.len(),
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                ));
            }
            out.write_bool(true);
            out.write_var_i32(properties.len() as i32);
            for property in properties {
                encode_java_string(out, &property.name, MAX_STRING_CHARS)?;
                match &property.value {
                    StatePropertyValueMatcherSummary::Exact(value) => {
                        out.write_bool(true);
                        encode_java_string(out, value, MAX_STRING_CHARS)?;
                    }
                    StatePropertyValueMatcherSummary::Range { min, max } => {
                        out.write_bool(false);
                        encode_optional_java_string(out, min.as_deref(), MAX_STRING_CHARS)?;
                        encode_optional_java_string(out, max.as_deref(), MAX_STRING_CHARS)?;
                    }
                }
            }
        }
        None => out.write_bool(false),
    }

    match predicate.nbt.as_ref() {
        Some(nbt @ NbtSummaryValue::Compound(_)) => {
            out.write_bool(true);
            out.write_bytes(&encode_nbt_summary_bounded(
                Some(nbt),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )?);
        }
        Some(_) => {
            return Err(ProtocolError::InvalidData(
                "block predicate nbt must be a compound tag".to_string(),
            ));
        }
        None => out.write_bool(false),
    }

    encode_data_component_matchers(
        out,
        predicate.exact_components.as_deref(),
        &predicate.partial_components,
    )
}

fn encode_data_component_matchers(
    out: &mut Encoder,
    exact: Option<&DataComponentPatchSummary>,
    partial: &[PartialDataComponentPredicateSummary],
) -> Result<()> {
    if let Some(exact) = exact {
        if !exact.removed_type_ids.is_empty() {
            return Err(ProtocolError::InvalidData(
                "exact data component predicate cannot remove components".to_string(),
            ));
        }
        if exact.added == 0 {
            return Err(ProtocolError::InvalidData(
                "empty exact data component predicate must use the absent projection".to_string(),
            ));
        }
        if exact.added > MAX_DATA_COMPONENT_PREDICATE_ENTRIES {
            return Err(ProtocolError::PacketTooLarge(
                exact.added,
                MAX_DATA_COMPONENT_PREDICATE_ENTRIES,
            ));
        }
        let descriptors = validate_encodable_data_component_patch_summary(exact)?;
        out.write_var_i32(exact.added as i32);
        for (type_id, descriptor) in descriptors {
            out.write_var_i32(type_id);
            descriptor.encode(exact, out)?;
        }
    } else {
        out.write_var_i32(0);
    }

    if partial.len() > MAX_PARTIAL_DATA_COMPONENT_PREDICATES {
        return Err(ProtocolError::PacketTooLarge(
            partial.len(),
            MAX_PARTIAL_DATA_COMPONENT_PREDICATES,
        ));
    }
    out.write_var_i32(partial.len() as i32);
    let mut seen = BTreeSet::new();
    for predicate in partial {
        let (concrete, type_id) = match predicate.predicate_type {
            DataComponentPredicateTypeSummary::Concrete(type_id) => {
                if crate::data_component_types::vanilla_data_component_predicate_type(type_id)
                    .is_none()
                {
                    return Err(ProtocolError::InvalidData(format!(
                        "invalid data component predicate type id {type_id}"
                    )));
                }
                (true, type_id)
            }
            DataComponentPredicateTypeSummary::AnyComponent(type_id) => {
                data_component_descriptor_or_error(type_id)?;
                (false, type_id)
            }
        };
        if !seen.insert((concrete, type_id)) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate partial data component predicate type id {type_id}"
            )));
        }
        let value = predicate.wire_value.as_ref().ok_or_else(|| {
            ProtocolError::InvalidData(
                "partial data component predicate is missing non-null nbt".to_string(),
            )
        })?;
        if !concrete && !matches!(value, NbtSummaryValue::Compound(entries) if entries.is_empty()) {
            return Err(ProtocolError::InvalidData(format!(
                "any-component predicate type id {type_id} must use an empty compound value"
            )));
        }
        out.write_bool(concrete);
        out.write_var_i32(type_id);
        out.write_bytes(&encode_nbt_summary_bounded(
            Some(value),
            i32::MAX as usize,
            MAX_NBT_DEPTH,
        )?);
    }
    Ok(())
}

fn encode_attribute_modifiers(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if summary.attribute_modifiers.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.attribute_modifiers.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(summary.attribute_modifiers.len() as i32);
    for modifier in &summary.attribute_modifiers {
        if crate::attribute_types::vanilla_attribute_type(modifier.attribute_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla attribute holder id {}",
                modifier.attribute_id
            )));
        }
        if normalize_resource_location(modifier.modifier_id.clone())? != modifier.modifier_id {
            return Err(ProtocolError::InvalidData(format!(
                "noncanonical attribute modifier resource location `{}`",
                modifier.modifier_id
            )));
        }
        if crate::attribute_modifier_types::attribute_modifier_operation_name(modifier.operation_id)
            .is_none()
        {
            return Err(ProtocolError::InvalidData(format!(
                "invalid attribute modifier operation id {}",
                modifier.operation_id
            )));
        }
        if vanilla_equipment_slot_group(modifier.slot_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid equipment slot group id {}",
                modifier.slot_id
            )));
        }

        encoded.write_var_i32(modifier.attribute_id);
        encoded.write_string(&modifier.modifier_id);
        encoded.write_f64(f64::from_bits(modifier.amount_bits));
        encoded.write_var_i32(modifier.operation_id);
        encoded.write_var_i32(modifier.slot_id);
        encode_attribute_modifier_display(modifier, &mut encoded)?;
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_attribute_modifier_display(
    modifier: &AttributeModifierSummary,
    out: &mut Encoder,
) -> Result<()> {
    use crate::attribute_modifier_types::{
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
    };

    match modifier.display_id {
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID
        | VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID => {
            if modifier.display_text.is_some()
                || modifier.display_text_styled.is_some()
                || modifier.display_text_nbt.is_some()
            {
                return Err(ProtocolError::InvalidData(format!(
                    "attribute modifier display {} cannot contain override text",
                    modifier.display_id
                )));
            }
            out.write_var_i32(modifier.display_id);
        }
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID => {
            let nbt = modifier.display_text_nbt.as_ref().ok_or_else(|| {
                ProtocolError::InvalidData(
                    "attribute modifier override display is missing component nbt".to_string(),
                )
            })?;
            let component =
                encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
            let (canonical_runs, canonical_nbt) =
                decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(
                    &component,
                ))?;
            let canonical_text = styled_runs_summary_text(&canonical_runs);
            if modifier.display_text.as_deref() != Some(canonical_text.as_str())
                || modifier.display_text_styled.as_deref() != Some(canonical_runs.as_slice())
                || canonical_nbt.as_ref() != Some(nbt)
            {
                return Err(ProtocolError::InvalidData(
                    "attribute modifier override display contains noncanonical component projections"
                        .to_string(),
                ));
            }
            out.write_var_i32(modifier.display_id);
            out.write_bytes(&component);
        }
        display_id => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid attribute modifier display id {display_id}"
            )));
        }
    }
    Ok(())
}

fn encode_consumable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let consumable = summary.consumable.ok_or_else(|| {
        ProtocolError::InvalidData("missing consumable component value".to_string())
    })?;
    let sound = summary.consumable_sound.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("consumable component is missing sound holder".to_string())
    })?;
    let has_consume_particles = summary.consumable_has_consume_particles.ok_or_else(|| {
        ProtocolError::InvalidData("consumable component is missing particle flag".to_string())
    })?;
    let effect_count = summary.consumable_effect_count.ok_or_else(|| {
        ProtocolError::InvalidData("consumable component is missing effect count".to_string())
    })?;
    if effect_count != summary.consumable_effects.len() {
        return Err(ProtocolError::InvalidData(format!(
            "consumable effect count {effect_count} does not match {} effects",
            summary.consumable_effects.len()
        )));
    }

    let mut encoded = Encoder::new();
    encoded.write_f32(f32::from_bits(consumable.consume_seconds.to_bits()));
    encoded.write_var_i32(consumable.animation.id());
    encode_sound_event_holder_summary(&mut encoded, sound)?;
    encoded.write_bool(has_consume_particles);
    encode_consume_effect_list(&mut encoded, &summary.consumable_effects)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_use_remainder(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let template = summary.use_remainder.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("use_remainder is missing item stack template".to_string())
    })?;
    validate_item_stack_template_forest(std::iter::once(template))?;
    encode_item_stack_template(out, template)
}

fn encode_use_cooldown(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let seconds_bits = summary.use_cooldown_seconds_bits.ok_or_else(|| {
        ProtocolError::InvalidData("use_cooldown is missing seconds projection".to_string())
    })?;
    let seconds = decode_persistent_positive_float(seconds_bits, "use_cooldown seconds")?;
    let ticks = vanilla_use_cooldown_ticks(seconds);
    if summary.use_cooldown_ticks != Some(ticks) {
        return Err(ProtocolError::InvalidData(format!(
            "use_cooldown ticks projection {:?} does not match {ticks}",
            summary.use_cooldown_ticks
        )));
    }

    let mut encoded = Encoder::new();
    encoded.write_f32(seconds);
    encoded.write_bool(summary.use_cooldown_group.is_some());
    if let Some(group) = &summary.use_cooldown_group {
        validate_canonical_resource_location(group, "use_cooldown group")?;
        encode_java_string(&mut encoded, group, MAX_STRING_CHARS)?;
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_death_protection(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_consume_effect_list(&mut encoded, &summary.death_protection_effects)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_consume_effect_list(out: &mut Encoder, effects: &[ConsumeEffectSummary]) -> Result<()> {
    if effects.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            effects.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    out.write_var_i32(effects.len() as i32);
    for effect in effects {
        encode_consume_effect(out, effect)?;
    }
    Ok(())
}

fn encode_consume_effect(out: &mut Encoder, effect: &ConsumeEffectSummary) -> Result<()> {
    use crate::consume_effect_types::{
        VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID, VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID,
    };

    match effect {
        ConsumeEffectSummary::ApplyEffects {
            effects,
            probability_bits,
        } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID);
            if effects.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    effects.len(),
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                ));
            }
            out.write_var_i32(effects.len() as i32);
            for effect in effects {
                encode_mob_effect_instance(out, effect)?;
            }
            out.write_f32(f32::from_bits(*probability_bits));
        }
        ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids,
            effect_tag,
        } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID);
            encode_holder_set_summary(
                out,
                effect_registry_ids,
                effect_tag.as_deref(),
                NetworkHolderSetRegistry::MobEffect,
                "remove consume effect",
            )?;
        }
        ConsumeEffectSummary::ClearAllEffects => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID);
        }
        ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID);
            out.write_f32(f32::from_bits(*diameter_bits));
        }
        ConsumeEffectSummary::PlaySound { sound } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
            encode_sound_event_holder_summary(out, sound)?;
        }
    }
    Ok(())
}

fn encode_mob_effect_instance(out: &mut Encoder, effect: &MobEffectInstanceSummary) -> Result<()> {
    client_state::validate_mob_effect_holder_id(effect.effect_id)?;
    out.write_var_i32(effect.effect_id);
    encode_mob_effect_details(
        out,
        effect.amplifier,
        effect.duration,
        effect.ambient,
        effect.show_particles,
        effect.show_icon,
        effect.hidden_effect.as_deref(),
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_mob_effect_details(
    out: &mut Encoder,
    amplifier: i32,
    duration: i32,
    ambient: bool,
    show_particles: bool,
    show_icon: bool,
    hidden_effect: Option<&MobEffectDetailsSummary>,
    depth: usize,
) -> Result<()> {
    if depth > MAX_MOB_EFFECT_DETAILS_DEPTH {
        return Err(ProtocolError::InvalidData(
            "mob effect details exceeded max depth".to_string(),
        ));
    }
    if !(0..=255).contains(&amplifier) {
        return Err(ProtocolError::InvalidData(format!(
            "mob effect amplifier {amplifier} is outside 0..=255"
        )));
    }
    out.write_var_i32(amplifier);
    out.write_var_i32(duration);
    out.write_bool(ambient);
    out.write_bool(show_particles);
    out.write_bool(show_icon);
    out.write_bool(hidden_effect.is_some());
    if let Some(hidden_effect) = hidden_effect {
        encode_mob_effect_details(
            out,
            hidden_effect.amplifier,
            hidden_effect.duration,
            hidden_effect.ambient,
            hidden_effect.show_particles,
            hidden_effect.show_icon,
            hidden_effect.hidden_effect.as_deref(),
            depth + 1,
        )?;
    }
    Ok(())
}

fn encode_potion_contents(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let custom_effect_count = summary.potion_custom_effect_count.ok_or_else(|| {
        ProtocolError::InvalidData("missing potion custom effect count".to_string())
    })?;
    if custom_effect_count != summary.potion_custom_effects.len() {
        return Err(ProtocolError::InvalidData(format!(
            "potion custom effect count {custom_effect_count} does not match {} effects",
            summary.potion_custom_effects.len()
        )));
    }
    if custom_effect_count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            custom_effect_count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_bool(summary.potion_id.is_some());
    if let Some(potion_id) = summary.potion_id {
        if vanilla_potion(potion_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla potion holder id {potion_id}"
            )));
        }
        encoded.write_var_i32(potion_id);
    }
    encoded.write_bool(summary.potion_custom_color.is_some());
    if let Some(custom_color) = summary.potion_custom_color {
        encoded.write_i32(custom_color);
    }
    encoded.write_var_i32(custom_effect_count as i32);
    for effect in &summary.potion_custom_effects {
        encode_mob_effect_instance(&mut encoded, effect)?;
    }
    encode_optional_java_string(
        &mut encoded,
        summary.potion_custom_name.as_deref(),
        MAX_STRING_CHARS,
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_potion_duration_scale(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let bits = summary.potion_duration_scale_bits.ok_or_else(|| {
        ProtocolError::InvalidData("missing potion_duration_scale component value".to_string())
    })?;
    out.write_f32(f32::from_bits(bits));
    Ok(())
}

fn encode_suspicious_stew_effects(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if summary.suspicious_stew_effects.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.suspicious_stew_effects.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(summary.suspicious_stew_effects.len() as i32);
    for effect in &summary.suspicious_stew_effects {
        client_state::validate_mob_effect_holder_id(effect.effect_id)?;
        encoded.write_var_i32(effect.effect_id);
        encoded.write_var_i32(effect.duration);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_writable_book_content(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let page_count = summary.writable_book_pages.len();
    if page_count > MAX_BOOK_PAGES {
        return Err(ProtocolError::PacketTooLarge(page_count, MAX_BOOK_PAGES));
    }
    if summary.writable_book_page_filters.len() != page_count {
        return Err(ProtocolError::InvalidData(
            "writable book page projections are not aligned".to_string(),
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(page_count as i32);
    for (page, filtered) in summary
        .writable_book_pages
        .iter()
        .zip(&summary.writable_book_page_filters)
    {
        encode_filterable_java_string(
            &mut encoded,
            page,
            filtered.as_deref(),
            MAX_WRITABLE_BOOK_PAGE_CHARS,
        )?;
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_piercing_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let weapon = summary.piercing_weapon.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing piercing_weapon component value".to_string())
    })?;
    let mut encoded = Encoder::new();
    encoded.write_bool(weapon.deals_knockback);
    encoded.write_bool(weapon.dismounts);
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.sound.as_ref())?;
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.hit_sound.as_ref())?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_kinetic_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let weapon = summary.kinetic_weapon.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing kinetic_weapon component value".to_string())
    })?;
    let mut encoded = Encoder::new();
    encoded.write_var_i32(weapon.contact_cooldown_ticks);
    encoded.write_var_i32(weapon.delay_ticks);
    encode_optional_kinetic_weapon_condition(&mut encoded, weapon.dismount_conditions)?;
    encode_optional_kinetic_weapon_condition(&mut encoded, weapon.knockback_conditions)?;
    encode_optional_kinetic_weapon_condition(&mut encoded, weapon.damage_conditions)?;
    encoded.write_f32(f32::from_bits(weapon.forward_movement_bits));
    encoded.write_f32(f32::from_bits(weapon.damage_multiplier_bits));
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.sound.as_ref())?;
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.hit_sound.as_ref())?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_optional_kinetic_weapon_condition(
    out: &mut Encoder,
    condition: Option<KineticWeaponConditionSummary>,
) -> Result<()> {
    let Some(condition) = condition else {
        out.write_bool(false);
        return Ok(());
    };
    out.write_bool(true);
    out.write_var_i32(condition.max_duration_ticks);
    out.write_f32(f32::from_bits(condition.min_speed_bits));
    out.write_f32(f32::from_bits(condition.min_relative_speed_bits));
    Ok(())
}

fn encode_swing_animation(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let animation = summary.swing_animation.ok_or_else(|| {
        ProtocolError::InvalidData("missing swing_animation component value".to_string())
    })?;
    let mut encoded = Encoder::new();
    encoded.write_var_i32(animation.animation_type.id());
    encoded.write_var_i32(animation.duration);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_blocks_attacks(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary.blocks_attacks.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing blocks_attacks component value".to_string())
    })?;
    if value.damage_reductions.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            value.damage_reductions.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_f32(f32::from_bits(value.block_delay_seconds_bits));
    encoded.write_f32(f32::from_bits(value.disable_cooldown_scale_bits));
    encoded.write_var_i32(value.damage_reductions.len() as i32);
    for reduction in &value.damage_reductions {
        encoded.write_f32(f32::from_bits(reduction.horizontal_blocking_angle_bits));
        encode_optional_holder_set_summary(
            &mut encoded,
            reduction.damage_types.as_ref().map(|types| {
                (
                    types.registry_ids.as_slice(),
                    types.tag.as_deref(),
                    "blocks_attacks damage reduction type",
                )
            }),
            NetworkHolderSetRegistry::DamageType,
        )?;
        encoded.write_f32(f32::from_bits(reduction.base_bits));
        encoded.write_f32(f32::from_bits(reduction.factor_bits));
    }
    encoded.write_f32(f32::from_bits(value.item_damage.threshold_bits));
    encoded.write_f32(f32::from_bits(value.item_damage.base_bits));
    encoded.write_f32(f32::from_bits(value.item_damage.factor_bits));
    encode_optional_holder_set_summary(
        &mut encoded,
        value.bypassed_by.as_ref().map(|types| {
            (
                types.registry_ids.as_slice(),
                types.tag.as_deref(),
                "blocks_attacks bypassed_by",
            )
        }),
        NetworkHolderSetRegistry::DamageType,
    )?;
    encode_optional_sound_event_holder_summary(&mut encoded, value.block_sound.as_ref())?;
    encode_optional_sound_event_holder_summary(&mut encoded, value.disable_sound.as_ref())?;

    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_optional_holder_set_summary(
    out: &mut Encoder,
    value: Option<(&[i32], Option<&str>, &'static str)>,
    registry: NetworkHolderSetRegistry,
) -> Result<()> {
    let Some((registry_ids, tag, what)) = value else {
        out.write_bool(false);
        return Ok(());
    };
    out.write_bool(true);
    encode_holder_set_summary(out, registry_ids, tag, registry, what)
}

fn encode_holder_set_summary(
    out: &mut Encoder,
    registry_ids: &[i32],
    tag: Option<&str>,
    registry: NetworkHolderSetRegistry,
    what: &str,
) -> Result<()> {
    if registry_ids.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            registry_ids.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    match (registry_ids.is_empty(), tag) {
        (true, Some(tag)) => {
            if normalize_resource_location(tag.to_string())? != tag {
                return Err(ProtocolError::InvalidData(format!(
                    "{what} has noncanonical tag resource location `{tag}`"
                )));
            }
            out.write_var_i32(0);
            encode_java_string(out, tag, MAX_STRING_CHARS)
        }
        (_, None) => {
            out.write_var_i32(registry_ids.len() as i32 + 1);
            for holder_id in registry_ids {
                registry.validate_holder_id(*holder_id)?;
                out.write_var_i32(*holder_id);
            }
            Ok(())
        }
        (false, Some(_)) => Err(ProtocolError::InvalidData(format!(
            "{what} cannot contain both holder ids and a tag"
        ))),
    }
}

fn encode_optional_sound_event_holder_summary(
    out: &mut Encoder,
    sound: Option<&SoundEventSummary>,
) -> Result<()> {
    let Some(sound) = sound else {
        out.write_bool(false);
        return Ok(());
    };
    out.write_bool(true);
    encode_sound_event_holder_summary(out, sound)
}

fn encode_sound_event_holder_summary(out: &mut Encoder, sound: &SoundEventSummary) -> Result<()> {
    match (
        sound.registry_id,
        sound.sound_id.as_deref(),
        sound.fixed_range_bits,
    ) {
        (Some(registry_id), None, None) if registry_id >= 0 && registry_id < i32::MAX => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(sound_id), fixed_range_bits) => {
            if normalize_resource_location(sound_id.to_string())? != sound_id {
                return Err(ProtocolError::InvalidData(format!(
                    "sound event has noncanonical resource location `{sound_id}`"
                )));
            }
            out.write_var_i32(0);
            encode_java_string(out, sound_id, MAX_STRING_CHARS)?;
            match fixed_range_bits {
                Some(range_bits) => {
                    out.write_bool(true);
                    out.write_f32(f32::from_bits(range_bits));
                }
                None => out.write_bool(false),
            }
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "sound event holder projections are not canonical".to_string(),
        )),
    }
}

fn encode_written_book_content(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let book = summary.written_book.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing written_book_content component value".to_string())
    })?;
    if !(0..=3).contains(&book.generation) {
        return Err(ProtocolError::InvalidData(format!(
            "written book generation {} is outside 0..=3",
            book.generation
        )));
    }
    let page_count = book.pages.len();
    if page_count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            page_count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    if book.page_filters.len() != page_count
        || summary.written_book_pages_nbt.len() != page_count
        || summary.written_book_page_filters_nbt.len() != page_count
    {
        return Err(ProtocolError::InvalidData(
            "written book page projections are not aligned".to_string(),
        ));
    }

    let mut encoded = Encoder::new();
    encode_filterable_java_string(
        &mut encoded,
        &book.title,
        book.title_filter.as_deref(),
        MAX_WRITTEN_BOOK_TITLE_CHARS,
    )?;
    encode_java_string(&mut encoded, &book.author, MAX_STRING_CHARS)?;
    encoded.write_var_i32(book.generation);
    encoded.write_var_i32(page_count as i32);
    for index in 0..page_count {
        let raw = summary.written_book_pages_nbt[index]
            .as_ref()
            .ok_or_else(|| {
                ProtocolError::InvalidData(format!(
                    "written book page {index} is missing raw component nbt"
                ))
            })?;
        let raw = encode_canonical_written_book_page(
            raw,
            &book.pages[index],
            &format!("written book page {index} raw"),
        )?;
        encoded.write_bytes(&raw);

        match (
            book.page_filters[index].as_deref(),
            summary.written_book_page_filters_nbt[index].as_ref(),
        ) {
            (None, None) => encoded.write_bool(false),
            (Some(text), Some(nbt)) => {
                let filtered = encode_canonical_written_book_page(
                    nbt,
                    text,
                    &format!("written book page {index} filtered"),
                )?;
                encoded.write_bool(true);
                encoded.write_bytes(&filtered);
            }
            _ => {
                return Err(ProtocolError::InvalidData(format!(
                    "written book page {index} filtered projections are not aligned"
                )));
            }
        }
    }
    encoded.write_bool(book.resolved);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_trim(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_trim_material_holder(
        &mut encoded,
        summary.armor_trim_material_id,
        summary.armor_trim_material_direct.as_deref(),
    )?;
    encode_trim_pattern_holder(
        &mut encoded,
        summary.armor_trim_pattern_id,
        summary.armor_trim_pattern_direct.as_deref(),
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_provides_trim_material(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_trim_material_holder(
        &mut encoded,
        summary.provides_trim_material_id,
        summary.provides_trim_material_direct.as_deref(),
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_ominous_bottle_amplifier(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let amplifier = summary.ominous_bottle_amplifier.ok_or_else(|| {
        ProtocolError::InvalidData("missing ominous_bottle_amplifier component value".to_string())
    })?;
    if !(0..=4).contains(&amplifier) {
        return Err(ProtocolError::InvalidData(format!(
            "ominous_bottle_amplifier {amplifier} is outside persistent range 0..=4"
        )));
    }
    out.write_var_i32(amplifier);
    Ok(())
}

fn encode_break_sound(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let sound = summary
        .break_sound
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing break_sound value".to_string()))?;
    let mut encoded = Encoder::new();
    encode_sound_event_holder_summary(&mut encoded, sound)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_firework_explosion(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let shape = summary.firework_explosion_shape.ok_or_else(|| {
        ProtocolError::InvalidData("missing firework_explosion shape".to_string())
    })?;
    let has_trail = summary.firework_explosion_has_trail.ok_or_else(|| {
        ProtocolError::InvalidData("missing firework_explosion trail flag".to_string())
    })?;
    let has_twinkle = summary.firework_explosion_has_twinkle.ok_or_else(|| {
        ProtocolError::InvalidData("missing firework_explosion twinkle flag".to_string())
    })?;
    let mut encoded = Encoder::new();
    encode_firework_explosion_fields(
        &mut encoded,
        shape,
        &summary.firework_explosion_colors,
        &summary.firework_explosion_fade_colors,
        has_trail,
        has_twinkle,
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_fireworks(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let flight_duration = summary.fireworks_flight_duration.ok_or_else(|| {
        ProtocolError::InvalidData("missing fireworks flight duration".to_string())
    })?;
    let explosion_count = summary.fireworks_explosions_count.ok_or_else(|| {
        ProtocolError::InvalidData("missing fireworks explosion count".to_string())
    })?;
    if explosion_count != summary.fireworks_explosions.len() {
        return Err(ProtocolError::InvalidData(format!(
            "fireworks explosion count {explosion_count} does not match {} explosions",
            summary.fireworks_explosions.len()
        )));
    }
    if explosion_count > MAX_FIREWORK_EXPLOSIONS {
        return Err(ProtocolError::PacketTooLarge(
            explosion_count,
            MAX_FIREWORK_EXPLOSIONS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(flight_duration);
    encoded.write_var_i32(explosion_count as i32);
    for explosion in &summary.fireworks_explosions {
        encode_firework_explosion_value(&mut encoded, explosion)?;
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_charged_projectiles(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_item_stack_template_list(
        out,
        &summary.charged_projectiles_items,
        MAX_DATA_COMPONENT_LIST_ITEMS,
        "charged_projectiles",
    )
}

fn encode_bundle_contents(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let item_count = summary.bundle_contents_item_count.ok_or_else(|| {
        ProtocolError::InvalidData("bundle_contents is missing item count".to_string())
    })?;
    if item_count != summary.bundle_contents_items.len() {
        return Err(ProtocolError::InvalidData(format!(
            "bundle_contents item count {item_count} does not match {} items",
            summary.bundle_contents_items.len()
        )));
    }
    encode_item_stack_template_list(
        out,
        &summary.bundle_contents_items,
        MAX_DATA_COMPONENT_LIST_ITEMS,
        "bundle_contents",
    )
}

fn encode_firework_explosion_value(
    out: &mut Encoder,
    explosion: &FireworkExplosionSummary,
) -> Result<()> {
    encode_firework_explosion_fields(
        out,
        explosion.shape,
        &explosion.colors,
        &explosion.fade_colors,
        explosion.has_trail,
        explosion.has_twinkle,
    )
}

fn encode_firework_explosion_fields(
    out: &mut Encoder,
    shape: FireworkExplosionShapeSummary,
    colors: &[i32],
    fade_colors: &[i32],
    has_trail: bool,
    has_twinkle: bool,
) -> Result<()> {
    out.write_var_i32(shape.id());
    encode_firework_color_list(out, colors)?;
    encode_firework_color_list(out, fade_colors)?;
    out.write_bool(has_trail);
    out.write_bool(has_twinkle);
    Ok(())
}

fn encode_firework_color_list(out: &mut Encoder, colors: &[i32]) -> Result<()> {
    if colors.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            colors.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    out.write_var_i32(colors.len() as i32);
    for color in colors {
        out.write_i32(*color);
    }
    Ok(())
}

fn encode_trim_material_holder(
    out: &mut Encoder,
    registry_id: Option<i32>,
    direct: Option<&TrimMaterialSummary>,
) -> Result<()> {
    match (registry_id, direct) {
        (Some(registry_id), None) if (0..i32::MAX).contains(&registry_id) => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(material)) => {
            if material.override_armor_assets.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    material.override_armor_assets.len(),
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                ));
            }
            validate_trim_material_asset_suffix(&material.asset_name, "asset_name")?;
            out.write_var_i32(0);
            encode_java_string(out, &material.asset_name, MAX_STRING_CHARS)?;
            out.write_var_i32(material.override_armor_assets.len() as i32);
            for (equipment_asset, suffix) in &material.override_armor_assets {
                validate_canonical_resource_location(
                    equipment_asset,
                    "trim material equipment asset",
                )?;
                validate_trim_material_asset_suffix(suffix, "override_armor_assets suffix")?;
                encode_java_string(out, equipment_asset, MAX_STRING_CHARS)?;
                encode_java_string(out, suffix, MAX_STRING_CHARS)?;
            }
            let description = encode_canonical_component_projection(
                material.description_nbt.as_deref(),
                &material.description,
                material.description_styled.as_deref(),
                "trim material description",
            )?;
            out.write_bytes(&description);
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "trim material holder projections are not canonical".to_string(),
        )),
    }
}

fn encode_trim_pattern_holder(
    out: &mut Encoder,
    registry_id: Option<i32>,
    direct: Option<&TrimPatternSummary>,
) -> Result<()> {
    match (registry_id, direct) {
        (Some(registry_id), None) if (0..i32::MAX).contains(&registry_id) => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(pattern)) => {
            validate_canonical_resource_location(&pattern.asset_id, "trim pattern asset id")?;
            out.write_var_i32(0);
            encode_java_string(out, &pattern.asset_id, MAX_STRING_CHARS)?;
            let description = encode_canonical_component_projection(
                pattern.description_nbt.as_deref(),
                &pattern.description,
                pattern.description_styled.as_deref(),
                "trim pattern description",
            )?;
            out.write_bytes(&description);
            out.write_bool(pattern.decal);
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "trim pattern holder projections are not canonical".to_string(),
        )),
    }
}

fn validate_trim_material_asset_suffix(suffix: &str, field: &str) -> Result<()> {
    if !is_valid_resource_path(suffix) {
        return Err(ProtocolError::InvalidData(format!(
            "trim material {field} `{suffix}` is not a valid resource path"
        )));
    }
    Ok(())
}

fn validate_canonical_resource_location(resource_id: &str, what: &str) -> Result<()> {
    if normalize_resource_location(resource_id.to_string())? != resource_id {
        return Err(ProtocolError::InvalidData(format!(
            "{what} has noncanonical resource location `{resource_id}`"
        )));
    }
    Ok(())
}

fn encode_canonical_component_projection(
    nbt: Option<&NbtSummaryValue>,
    expected_text: &str,
    expected_styled: Option<&[StyledTextRun]>,
    what: &str,
) -> Result<Vec<u8>> {
    let nbt =
        nbt.ok_or_else(|| ProtocolError::InvalidData(format!("{what} is missing component nbt")))?;
    let expected_styled = expected_styled.ok_or_else(|| {
        ProtocolError::InvalidData(format!("{what} is missing styled projection"))
    })?;
    let encoded = encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
    let (canonical_runs, canonical_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(&encoded))?;
    if styled_runs_summary_text(&canonical_runs) != expected_text
        || canonical_runs != expected_styled
        || canonical_nbt.as_ref() != Some(nbt)
    {
        return Err(ProtocolError::InvalidData(format!(
            "{what} contains noncanonical component projections"
        )));
    }
    Ok(encoded)
}

fn encode_filterable_java_string(
    out: &mut Encoder,
    raw: &str,
    filtered: Option<&str>,
    max_utf16_units: usize,
) -> Result<()> {
    encode_java_string(out, raw, max_utf16_units)?;
    encode_optional_java_string(out, filtered, max_utf16_units)
}

fn encode_canonical_written_book_page(
    nbt: &NbtSummaryValue,
    expected_text: &str,
    what: &str,
) -> Result<Vec<u8>> {
    let encoded = encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
    let (runs, canonical_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(&encoded))?;
    if styled_runs_summary_text(&runs) != expected_text || canonical_nbt.as_ref() != Some(nbt) {
        return Err(ProtocolError::InvalidData(format!(
            "{what} contains noncanonical component projections"
        )));
    }
    Ok(encoded)
}

pub(super) fn decode_required_compound_tag(
    decoder: &mut Decoder<'_>,
    component: &'static str,
) -> Result<NbtSummaryValue> {
    let Some(value @ NbtSummaryValue::Compound(_)) = decode_nbt_summary_from_decoder(decoder)?
    else {
        return Err(ProtocolError::InvalidData(format!(
            "{component} network value must be a non-null compound tag"
        )));
    };
    Ok(value)
}

fn encode_required_compound_tag(
    value: Option<&NbtSummaryValue>,
    component: &'static str,
    out: &mut Encoder,
) -> Result<()> {
    let value = value.ok_or_else(|| {
        ProtocolError::InvalidData(format!("missing {component} component value"))
    })?;
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(format!(
            "{component} network value must be a non-null compound tag"
        )));
    }
    let encoded = encode_nbt_summary_bounded(Some(value), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

fn encode_entity_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_typed_entity_data(
        summary.entity_data_entity_type_id,
        summary.entity_data_tag.as_ref(),
        TypedEntityDataRegistry::EntityType,
        "entity_data",
        out,
    )
}

fn encode_bucket_entity_data(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_required_compound_tag(
        summary.codec_backed_nbt.get(&type_id),
        "bucket_entity_data",
        out,
    )
}

fn encode_block_entity_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_typed_entity_data(
        summary.block_entity_data_type_id,
        summary.block_entity_data_tag.as_ref(),
        TypedEntityDataRegistry::BlockEntityType,
        "block_entity_data",
        out,
    )
}

fn encode_typed_entity_data(
    type_id: Option<i32>,
    tag: Option<&NbtSummaryValue>,
    registry: TypedEntityDataRegistry,
    component: &'static str,
    out: &mut Encoder,
) -> Result<()> {
    let type_id = type_id.ok_or_else(|| {
        ProtocolError::InvalidData(format!("missing {component} registry holder"))
    })?;
    registry.validate(type_id)?;
    if let Some(NbtSummaryValue::Compound(entries)) = tag {
        if entries.iter().any(|entry| entry.name == "id") {
            return Err(ProtocolError::InvalidData(format!(
                "{component} canonical network tag must not contain top-level id"
            )));
        }
    }
    let mut encoded = Encoder::new();
    encoded.write_var_i32(type_id);
    encode_required_compound_tag(tag, component, &mut encoded)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_profile(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let profile = summary
        .profile
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing profile component value".to_string()))?;
    if canonical_resolvable_profile_summary(profile)? != *profile {
        return Err(ProtocolError::InvalidData(
            "profile component contains noncanonical derived texture values".to_string(),
        ));
    }

    let mut encoded = Encoder::new();
    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            let uuid = profile.uuid.ok_or_else(|| {
                ProtocolError::InvalidData("game profile component is missing its UUID".to_string())
            })?;
            let name = profile.name.as_deref().ok_or_else(|| {
                ProtocolError::InvalidData("game profile component is missing its name".to_string())
            })?;
            encoded.write_bool(true);
            encoded.write_uuid(uuid);
            encode_java_string(&mut encoded, name, MAX_PLAYER_NAME_CHARS)?;
        }
        ResolvableProfileKindSummary::Partial => {
            encoded.write_bool(false);
            encode_optional_java_string(
                &mut encoded,
                profile.name.as_deref(),
                MAX_PLAYER_NAME_CHARS,
            )?;
            encoded.write_bool(profile.uuid.is_some());
            if let Some(uuid) = profile.uuid {
                encoded.write_uuid(uuid);
            }
        }
    }
    encode_game_profile_properties(&mut encoded, &profile.properties)?;
    encode_player_skin_patch(&mut encoded, &profile.skin_patch)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_map_id(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let map_id = summary
        .map_id
        .ok_or_else(|| ProtocolError::InvalidData("missing map_id value".to_string()))?;
    out.write_var_i32(map_id);
    Ok(())
}

fn encode_dye(_type_id: i32, summary: &DataComponentPatchSummary, out: &mut Encoder) -> Result<()> {
    let color_id = summary
        .dye_color_id
        .ok_or_else(|| ProtocolError::InvalidData("missing dye value".to_string()))?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "dye color id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

fn encode_dyed_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color = summary
        .dyed_color
        .ok_or_else(|| ProtocolError::InvalidData("missing dyed_color value".to_string()))?;
    out.write_i32(color);
    Ok(())
}

fn encode_map_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color = summary
        .map_color
        .ok_or_else(|| ProtocolError::InvalidData("missing map_color value".to_string()))?;
    out.write_i32(color);
    Ok(())
}

fn encode_map_post_processing(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary.map_post_processing.ok_or_else(|| {
        ProtocolError::InvalidData("missing map_post_processing value".to_string())
    })?;
    out.write_var_i32(value.id());
    Ok(())
}

fn encode_jukebox_playable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    match (
        summary.jukebox_song_id,
        summary.jukebox_direct_song.as_deref(),
    ) {
        (Some(holder_id), None) if (0..i32::MAX).contains(&holder_id) => {
            encoded.write_var_i32(holder_id + 1);
        }
        (None, Some(song)) => {
            let length_in_seconds = decode_persistent_positive_float(
                song.length_in_seconds_bits,
                "jukebox song length_in_seconds",
            )?;
            if !(0..=15).contains(&song.comparator_output) {
                return Err(ProtocolError::InvalidData(format!(
                    "jukebox song comparator_output {} is outside 0..=15",
                    song.comparator_output
                )));
            }
            encoded.write_var_i32(0);
            encode_sound_event_holder_summary(&mut encoded, &song.sound_event)?;
            encoded.write_bytes(&encode_canonical_component_projection(
                song.description_nbt.as_deref(),
                &song.description,
                song.description_styled.as_deref(),
                "jukebox song description",
            )?);
            encoded.write_f32(length_in_seconds);
            encoded.write_var_i32(song.comparator_output);
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "jukebox_playable holder projections are not canonical".to_string(),
            ));
        }
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_instrument(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    match (summary.instrument_id, summary.instrument_direct.as_deref()) {
        (Some(holder_id), None)
            if (0..i32::MAX).contains(&holder_id)
                && summary.instrument_description.is_none()
                && summary.instrument_description_styled.is_none() =>
        {
            encoded.write_var_i32(holder_id + 1);
        }
        (None, Some(instrument)) => {
            if summary.instrument_description.as_deref() != Some(instrument.description.as_str())
                || summary.instrument_description_styled.as_deref()
                    != instrument.description_styled.as_deref()
            {
                return Err(ProtocolError::InvalidData(
                    "instrument tooltip projections do not match the direct description"
                        .to_string(),
                ));
            }
            let use_duration = decode_persistent_positive_float(
                instrument.use_duration_bits,
                "instrument use_duration",
            )?;
            let range =
                decode_persistent_positive_float(instrument.range_bits, "instrument range")?;
            encoded.write_var_i32(0);
            encode_sound_event_holder_summary(&mut encoded, &instrument.sound_event)?;
            encoded.write_f32(use_duration);
            encoded.write_f32(range);
            encoded.write_bytes(&encode_canonical_component_projection(
                instrument.description_nbt.as_deref(),
                &instrument.description,
                instrument.description_styled.as_deref(),
                "instrument description",
            )?);
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "instrument holder projections are not canonical".to_string(),
            ));
        }
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn decode_persistent_positive_float(bits: u32, what: &str) -> Result<f32> {
    let value = f32::from_bits(bits);
    if value > 0.0 && value <= f32::MAX {
        return Ok(value);
    }
    Err(ProtocolError::InvalidData(format!(
        "{what} must be positive, got {value}"
    )))
}

fn encode_banner_patterns(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if summary.banner_pattern_layers.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.banner_pattern_layers.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(summary.banner_pattern_layers.len() as i32);
    for layer in &summary.banner_pattern_layers {
        encode_banner_pattern_holder(&mut encoded, layer)?;
        if normalize_vanilla_dye_color_id(layer.color_id) != layer.color_id {
            return Err(ProtocolError::InvalidData(format!(
                "banner pattern has noncanonical dye color id {}",
                layer.color_id
            )));
        }
        encoded.write_var_i32(layer.color_id);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_banner_pattern_holder(
    out: &mut Encoder,
    layer: &BannerPatternLayerSummary,
) -> Result<()> {
    match (
        layer.registry_id,
        layer.asset_id.as_deref(),
        layer.translation_key.as_deref(),
    ) {
        (Some(registry_id), None, None) if (0..i32::MAX).contains(&registry_id) => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(asset_id), Some(translation_key)) => {
            validate_canonical_resource_location(asset_id, "banner pattern asset id")?;
            out.write_var_i32(0);
            encode_java_string(out, asset_id, MAX_STRING_CHARS)?;
            encode_java_string(out, translation_key, MAX_STRING_CHARS)
        }
        _ => Err(ProtocolError::InvalidData(
            "banner pattern holder projections are not canonical".to_string(),
        )),
    }
}

fn encode_lodestone_tracker(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let tracked = summary.lodestone_tracked.ok_or_else(|| {
        ProtocolError::InvalidData("lodestone_tracker is missing tracked projection".to_string())
    })?;
    let mut encoded = Encoder::new();
    encoded.write_bool(summary.lodestone_target.is_some());
    if let Some(target) = &summary.lodestone_target {
        validate_canonical_resource_location(
            &target.dimension,
            "lodestone_tracker target dimension",
        )?;
        encode_java_string(&mut encoded, &target.dimension, MAX_STRING_CHARS)?;
        let packed = chunks::encode_block_pos(target.pos);
        if chunks::decode_block_pos(packed) != target.pos {
            return Err(ProtocolError::InvalidData(format!(
                "lodestone_tracker target position ({}, {}, {}) is not representable as a packed block position",
                target.pos.x, target.pos.y, target.pos.z
            )));
        }
        encoded.write_i64(packed);
    }
    encoded.write_bool(tracked);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_container(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_item_container_contents(out, summary)
}

fn encode_bees(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if !summary.bees_present || summary.bees_count != summary.bees.len() {
        return Err(ProtocolError::InvalidData(
            "bees presence and count projections are not canonical".to_string(),
        ));
    }
    if summary.bees.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.bees.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(summary.bees.len() as i32);
    for bee in &summary.bees {
        encode_typed_entity_data(
            Some(bee.entity_type_id),
            bee.entity_data_tag.as_ref(),
            TypedEntityDataRegistry::EntityType,
            "bees entity_data",
            &mut encoded,
        )?;
        encoded.write_var_i32(bee.ticks_in_hive);
        encoded.write_var_i32(bee.min_ticks_in_hive);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_painting_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    match (
        summary.painting_variant_id,
        summary.painting_variant_direct.as_ref(),
    ) {
        (Some(holder_id), None) if (0..i32::MAX).contains(&holder_id) => {
            encoded.write_var_i32(holder_id + 1);
        }
        (None, Some(variant)) => {
            if !(1..=16).contains(&variant.width) || !(1..=16).contains(&variant.height) {
                return Err(ProtocolError::InvalidData(format!(
                    "painting variant dimensions {}x{} are outside 1..=16",
                    variant.width, variant.height
                )));
            }
            validate_canonical_resource_location(&variant.asset_id, "painting variant asset id")?;
            encoded.write_var_i32(0);
            encoded.write_var_i32(variant.width);
            encoded.write_var_i32(variant.height);
            encode_java_string(&mut encoded, &variant.asset_id, MAX_STRING_CHARS)?;
            encode_optional_painting_component(
                &mut encoded,
                variant.title.as_deref(),
                variant.title_styled.as_deref(),
                variant.title_nbt.as_deref(),
                "painting variant title",
            )?;
            encode_optional_painting_component(
                &mut encoded,
                variant.author.as_deref(),
                variant.author_styled.as_deref(),
                variant.author_nbt.as_deref(),
                "painting variant author",
            )?;
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "painting variant holder projections are not canonical".to_string(),
            ));
        }
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_optional_painting_component(
    out: &mut Encoder,
    text: Option<&str>,
    styled: Option<&[StyledTextRun]>,
    nbt: Option<&NbtSummaryValue>,
    what: &str,
) -> Result<()> {
    match (text, styled, nbt) {
        (None, None, None) => {
            out.write_bool(false);
            Ok(())
        }
        (Some(text), Some(styled), Some(nbt)) => {
            out.write_bool(true);
            out.write_bytes(&encode_canonical_component_projection(
                Some(nbt),
                text,
                Some(styled),
                what,
            )?);
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(format!(
            "{what} optional component projections are not aligned"
        ))),
    }
}

fn encode_game_profile_properties(
    out: &mut Encoder,
    properties: &[GameProfilePropertySummary],
) -> Result<()> {
    if properties.len() > MAX_PROFILE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            properties.len(),
            MAX_PROFILE_PROPERTIES,
        ));
    }
    out.write_var_i32(properties.len() as i32);
    for property in properties {
        encode_java_string(out, &property.name, MAX_PROFILE_PROPERTY_NAME_CHARS)?;
        encode_java_string(out, &property.value, MAX_STRING_CHARS)?;
        encode_optional_java_string(
            out,
            property.signature.as_deref(),
            MAX_PROFILE_SIGNATURE_CHARS,
        )?;
    }
    Ok(())
}

fn encode_player_skin_patch(out: &mut Encoder, patch: &PlayerSkinPatchSummary) -> Result<()> {
    for texture in [&patch.body, &patch.cape, &patch.elytra] {
        out.write_bool(texture.is_some());
        if let Some(texture) = texture {
            encode_java_string(out, &texture.asset_id, MAX_STRING_CHARS)?;
        }
    }
    out.write_bool(patch.model.is_some());
    if let Some(model) = patch.model {
        out.write_bool(model == PlayerModelTypeSummary::Slim);
    }
    Ok(())
}

fn encode_optional_java_string(
    out: &mut Encoder,
    value: Option<&str>,
    max_utf16_units: usize,
) -> Result<()> {
    out.write_bool(value.is_some());
    if let Some(value) = value {
        encode_java_string(out, value, max_utf16_units)?;
    }
    Ok(())
}

fn encode_java_string(out: &mut Encoder, value: &str, max_utf16_units: usize) -> Result<()> {
    validate_java_string_len(value, max_utf16_units)?;
    out.write_string(value);
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum TypedEntityDataRegistry {
    EntityType,
    BlockEntityType,
}

impl TypedEntityDataRegistry {
    pub(super) const fn component_name(self) -> &'static str {
        match self {
            Self::EntityType => "entity_data",
            Self::BlockEntityType => "block_entity_data",
        }
    }

    pub(super) fn validate(self, type_id: i32) -> Result<()> {
        let (valid, registry) = match self {
            Self::EntityType => (vanilla_entity_type(type_id).is_some(), "entity type"),
            Self::BlockEntityType => (
                vanilla_block_entity_type(type_id).is_some(),
                "block entity type",
            ),
        };
        if !valid {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla {registry} holder id {type_id}"
            )));
        }
        Ok(())
    }
}

macro_rules! decode_var_i32_field {
    ($name:ident, $field:ident) => {
        fn $name(
            _type_id: i32,
            decoder: &mut Decoder<'_>,
            summary: &mut DataComponentPatchSummary,
        ) -> Result<()> {
            summary.$field = Some(decoder.read_var_i32()?);
            Ok(())
        }
    };
}

decode_var_i32_field!(decode_max_stack_size, max_stack_size);
decode_var_i32_field!(decode_max_damage, max_damage);
decode_var_i32_field!(decode_damage, damage);
decode_var_i32_field!(decode_additional_trade_cost, additional_trade_cost);
decode_var_i32_field!(decode_map_id, map_id);
decode_var_i32_field!(decode_ominous_bottle_amplifier, ominous_bottle_amplifier);

fn decode_repair_cost(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let value = decoder.read_var_i32()?;
    if value < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "repair cost must be nonnegative, got {value}"
        )));
    }
    summary.repair_cost = Some(value);
    Ok(())
}

fn decode_enchantable(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let value = decoder.read_var_i32()?;
    if value <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "enchantable value must be positive, got {value}"
        )));
    }
    summary.enchantable_value = Some(value);
    Ok(())
}

macro_rules! encode_var_i32_field {
    ($name:ident, $field:ident) => {
        fn $name(
            type_id: i32,
            summary: &DataComponentPatchSummary,
            out: &mut Encoder,
        ) -> Result<()> {
            let value = summary.$field.ok_or_else(|| {
                ProtocolError::InvalidData(format!(
                    "data component type id {type_id} is missing {}",
                    stringify!($field)
                ))
            })?;
            out.write_var_i32(value);
            Ok(())
        }
    };
}

encode_var_i32_field!(encode_max_stack_size, max_stack_size);
encode_var_i32_field!(encode_max_damage, max_damage);
encode_var_i32_field!(encode_damage, damage);
encode_var_i32_field!(encode_additional_trade_cost, additional_trade_cost);

fn encode_repair_cost(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary
        .repair_cost
        .ok_or_else(|| ProtocolError::InvalidData("missing repair cost value".to_string()))?;
    if value < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "repair cost must be nonnegative, got {value}"
        )));
    }
    out.write_var_i32(value);
    Ok(())
}

fn encode_enchantable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary.enchantable_value.ok_or_else(|| {
        ProtocolError::InvalidData("missing enchantable component value".to_string())
    })?;
    if value <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "enchantable value must be positive, got {value}"
        )));
    }
    out.write_var_i32(value);
    Ok(())
}

fn decode_unbreakable(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.unbreakable = true;
    Ok(())
}

fn encode_unbreakable(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    _out: &mut Encoder,
) -> Result<()> {
    if !summary.unbreakable {
        return Err(ProtocolError::InvalidData(format!(
            "data component type id {type_id} is missing unbreakable"
        )));
    }
    Ok(())
}

fn decode_use_effects_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.use_effects = Some(decode_use_effects_summary(decoder)?);
    Ok(())
}

fn decode_styled_name(
    decoder: &mut Decoder<'_>,
) -> Result<(String, Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    let (runs, nbt) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    Ok((styled_runs_summary_text(&runs), runs, nbt))
}

fn decode_custom_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (text, runs, nbt) = decode_styled_name(decoder)?;
    summary.custom_name = Some(text);
    summary.custom_name_styled = Some(runs);
    summary.custom_name_nbt = nbt;
    Ok(())
}

fn decode_item_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (text, runs, nbt) = decode_styled_name(decoder)?;
    summary.item_name = Some(text);
    summary.item_name_styled = Some(runs);
    summary.item_name_nbt = nbt;
    Ok(())
}

fn decode_minimum_attack_charge(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.minimum_attack_charge_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

fn decode_damage_type(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.damage_type_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_item_model(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.item_model = Some(read_resource_location(decoder)?);
    Ok(())
}

fn decode_lore_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    (summary.lore_styled, summary.lore_nbt) = decode_lore(decoder)?;
    summary.lore = summary
        .lore_styled
        .iter()
        .map(|line| styled_runs_summary_text(line))
        .collect();
    Ok(())
}

fn decode_rarity(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.rarity = Some(decode_item_rarity(decoder)?);
    Ok(())
}

fn decode_enchantments(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.enchantments = decode_varint_map(decoder)?;
    Ok(())
}

fn decode_can_place_on(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (tooltip, persistent) = decode_adventure_mode_predicate_summary(decoder)?;
    summary.can_place_on = Some(tooltip);
    summary.can_place_on_persistent = Some(persistent);
    Ok(())
}

fn decode_can_break(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (tooltip, persistent) = decode_adventure_mode_predicate_summary(decoder)?;
    summary.can_break = Some(tooltip);
    summary.can_break_persistent = Some(persistent);
    Ok(())
}

fn decode_attribute_modifiers_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.attribute_modifiers = decode_attribute_modifiers(decoder)?;
    Ok(())
}

fn decode_custom_model_data_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (floats, flags, strings, colors) = decode_custom_model_data(decoder)?;
    summary.custom_model_data_floats = floats.into();
    summary.custom_model_data_flags = flags;
    summary.custom_model_data_strings = strings;
    summary.custom_model_data_colors = colors;
    Ok(())
}

fn decode_tooltip_display_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (hide_tooltip, hidden_components) = decode_tooltip_display_summary(decoder)?;
    summary.tooltip_hide_tooltip = hide_tooltip;
    summary.tooltip_hidden_component_type_ids = hidden_components;
    Ok(())
}

fn decode_enchantment_glint_override(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.enchantment_glint_override = Some(decoder.read_bool()?);
    Ok(())
}

fn decode_intangible_projectile(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.intangible_projectile = true;
    skip_nbt_tag_from_decoder(decoder)
}

fn decode_food_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.food_nutrition = Some(decoder.read_var_i32()?);
    summary.food_saturation_bits = Some(decoder.read_f32()?.to_bits());
    summary.food_can_always_eat = decoder.read_bool()?;
    Ok(())
}

fn encode_food(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let nutrition = summary.food_nutrition.ok_or_else(|| {
        ProtocolError::InvalidData("missing food component nutrition".to_string())
    })?;
    let saturation_bits = summary.food_saturation_bits.ok_or_else(|| {
        ProtocolError::InvalidData("missing food component saturation".to_string())
    })?;
    out.write_var_i32(nutrition);
    out.write_f32(f32::from_bits(saturation_bits));
    out.write_bool(summary.food_can_always_eat);
    Ok(())
}

fn decode_consumable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let consumable = decode_consumable_component_summary(decoder)?;
    summary.consumable = Some(consumable.summary);
    summary.consumable_sound = Some(consumable.sound);
    summary.consumable_has_consume_particles = Some(consumable.has_consume_particles);
    summary.consumable_effect_count = Some(consumable.effect_count);
    summary.consumable_effects = consumable.effects;
    Ok(())
}

fn decode_use_remainder_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.use_remainder = Some(Box::new(decode_item_stack_template(decoder)?));
    Ok(())
}

fn decode_use_cooldown_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let cooldown = decode_use_cooldown_summary(decoder)?;
    summary.use_cooldown_seconds_bits = Some(cooldown.seconds_bits);
    summary.use_cooldown_ticks = Some(cooldown.ticks);
    summary.use_cooldown_group = cooldown.cooldown_group;
    Ok(())
}

fn decode_damage_resistant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let types = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::DamageType)?;
    summary.damage_resistant_type_registry_ids = types.registry_ids;
    summary.damage_resistant_type_tag = types.tag;
    Ok(())
}

fn encode_damage_resistant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_holder_set_summary(
        &mut encoded,
        &summary.damage_resistant_type_registry_ids,
        summary.damage_resistant_type_tag.as_deref(),
        NetworkHolderSetRegistry::DamageType,
        "damage_resistant types",
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn decode_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.weapon_item_damage_per_attack = Some(decoder.read_var_i32()?);
    summary.weapon_disable_blocking_seconds_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

fn encode_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let item_damage_per_attack = summary.weapon_item_damage_per_attack.ok_or_else(|| {
        ProtocolError::InvalidData("missing weapon item_damage_per_attack".to_string())
    })?;
    if item_damage_per_attack < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "weapon item_damage_per_attack {item_damage_per_attack} is outside persistent range 0.."
        )));
    }
    let disable_blocking_seconds = f32::from_bits(
        summary
            .weapon_disable_blocking_seconds_bits
            .ok_or_else(|| {
                ProtocolError::InvalidData(
                    "missing weapon disable_blocking_for_seconds".to_string(),
                )
            })?,
    );
    if !disable_blocking_seconds.is_finite() || disable_blocking_seconds.total_cmp(&0.0).is_lt() {
        return Err(ProtocolError::InvalidData(format!(
            "weapon disable_blocking_for_seconds {disable_blocking_seconds} is outside persistent range 0.."
        )));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(item_damage_per_attack);
    encoded.write_f32(disable_blocking_seconds);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn decode_attack_range_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.attack_range = Some(decode_attack_range_summary(decoder)?);
    Ok(())
}

fn decode_repairable(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let items = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::Item)?;
    summary.repairable_item_registry_ids = items.registry_ids;
    summary.repairable_item_tag = items.tag;
    Ok(())
}

fn encode_repairable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_holder_set_summary(
        &mut encoded,
        &summary.repairable_item_registry_ids,
        summary.repairable_item_tag.as_deref(),
        NetworkHolderSetRegistry::Item,
        "repairable items",
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn decode_glider(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    Ok(())
}

fn decode_tooltip_style(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tooltip_style = Some(read_resource_location(decoder)?);
    Ok(())
}

fn decode_death_protection_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.death_protection_effects = decode_consume_effect_list_summary(decoder)?;
    Ok(())
}

fn decode_piercing_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.piercing_weapon = Some(decode_piercing_weapon_summary(decoder)?);
    Ok(())
}

fn decode_kinetic_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.kinetic_weapon = Some(decode_kinetic_weapon_summary(decoder)?);
    Ok(())
}

fn decode_swing_animation_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.swing_animation = Some(decode_swing_animation(decoder)?);
    Ok(())
}

fn decode_stored_enchantments(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.stored_enchantments = decode_varint_map(decoder)?;
    Ok(())
}

fn decode_dye(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.dye_color_id = Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

fn decode_dyed_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.dyed_color = Some(decoder.read_i32()?);
    Ok(())
}

fn decode_map_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.map_color = Some(decoder.read_i32()?);
    Ok(())
}

fn decode_map_decorations(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let value = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
        ProtocolError::InvalidData("map_decorations codec value is missing".to_string())
    })?;
    let decorations = parse_map_decorations_nbt(&value)?;
    let canonical = canonical_map_decorations_nbt(&decorations);
    summary.map_decorations = Some(decorations);
    summary.codec_backed_nbt.insert(type_id, canonical);
    Ok(())
}

fn encode_map_decorations(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let decorations = summary.map_decorations.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing map_decorations component value".to_string())
    })?;
    if decorations.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            decorations.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    let mut seen = BTreeSet::new();
    for decoration in decorations {
        if !seen.insert(decoration.id.as_str()) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate map_decorations id {:?}",
                decoration.id
            )));
        }
        let canonical_resource =
            normalize_resource_location(decoration.decoration_type_resource_id.clone())?;
        if canonical_resource != decoration.decoration_type_resource_id {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {:?} has noncanonical type resource location `{}`",
                decoration.id, decoration.decoration_type_resource_id
            )));
        }
        let Some(expected) = vanilla_map_decoration_type(decoration.decoration_type_id) else {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {:?} has unknown type holder id {}",
                decoration.id, decoration.decoration_type_id
            )));
        };
        if expected.resource_id != decoration.decoration_type_resource_id {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {:?} type holder id {} resolves to {}, not {}",
                decoration.id,
                decoration.decoration_type_id,
                expected.resource_id,
                decoration.decoration_type_resource_id
            )));
        }
    }

    let canonical = canonical_map_decorations_nbt(decorations);
    if let Some(retained) = summary.codec_backed_nbt.get(&type_id) {
        if retained != &canonical {
            return Err(ProtocolError::InvalidData(
                "map_decorations retained codec value does not match its typed entries".to_string(),
            ));
        }
    }
    let encoded = encode_nbt_summary_bounded(Some(&canonical), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

fn parse_map_decorations_nbt(value: &NbtSummaryValue) -> Result<Vec<ItemMapDecorationSummary>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(
            "map_decorations codec value is not a compound map".to_string(),
        ));
    };
    if entries.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            entries.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    entries
        .iter()
        .map(|entry| parse_map_decoration_entry(&entry.name, &entry.value))
        .collect()
}

fn parse_map_decoration_entry(
    id: &str,
    value: &NbtSummaryValue,
) -> Result<ItemMapDecorationSummary> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(format!(
            "map_decorations entry {id:?} is not a compound"
        )));
    };
    let field = |name: &str| {
        entries
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
            .ok_or_else(|| {
                ProtocolError::InvalidData(format!(
                    "map_decorations entry {id:?} is missing {name}"
                ))
            })
    };
    let raw_type = match field("type")? {
        NbtSummaryValue::String(raw_type) => raw_type,
        _ => {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {id:?} type is not a resource location"
            )));
        }
    };
    let resource_id = normalize_resource_location(raw_type.clone())?;
    let decoration_type =
        vanilla_map_decoration_type_for_resource_id(&resource_id).ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "map_decorations entry {id:?} references unknown type {resource_id}"
            ))
        })?;
    Ok(ItemMapDecorationSummary {
        id: id.to_string(),
        decoration_type_id: decoration_type.type_id,
        decoration_type_resource_id: decoration_type.resource_id.to_string(),
        x_bits: nbt_number_as_f64_bits(field("x")?, &format!("map_decorations entry {id:?} x"))?,
        z_bits: nbt_number_as_f64_bits(field("z")?, &format!("map_decorations entry {id:?} z"))?,
        rotation_bits: nbt_number_as_f32_bits(
            field("rotation")?,
            &format!("map_decorations entry {id:?} rotation"),
        )?,
    })
}

fn nbt_number_as_f64_bits(value: &NbtSummaryValue, what: &str) -> Result<u64> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f64::from(*value),
        NbtSummaryValue::Short(value) => f64::from(*value),
        NbtSummaryValue::Int(value) => f64::from(*value),
        NbtSummaryValue::Long(value) => *value as f64,
        NbtSummaryValue::Float(bits) => f64::from(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => return Ok(*bits),
        _ => return Err(ProtocolError::InvalidData(format!("{what} is not numeric"))),
    };
    Ok(value.to_bits())
}

fn nbt_number_as_f32_bits(value: &NbtSummaryValue, what: &str) -> Result<u32> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => return Ok(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return Err(ProtocolError::InvalidData(format!("{what} is not numeric"))),
    };
    Ok(value.to_bits())
}

fn canonical_map_decorations_nbt(decorations: &[ItemMapDecorationSummary]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        decorations
            .iter()
            .map(|decoration| NbtSummaryEntry {
                name: decoration.id.clone(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".to_string(),
                        value: NbtSummaryValue::String(
                            decoration.decoration_type_resource_id.clone(),
                        ),
                    },
                    NbtSummaryEntry {
                        name: "x".to_string(),
                        value: NbtSummaryValue::Double(decoration.x_bits),
                    },
                    NbtSummaryEntry {
                        name: "z".to_string(),
                        value: NbtSummaryValue::Double(decoration.z_bits),
                    },
                    NbtSummaryEntry {
                        name: "rotation".to_string(),
                        value: NbtSummaryValue::Float(decoration.rotation_bits),
                    },
                ]),
            })
            .collect(),
    )
}

fn decode_recipes(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let value = decode_nbt_summary_from_decoder(decoder)?
        .ok_or_else(|| ProtocolError::InvalidData("recipes codec value is missing".to_string()))?;
    let recipe_keys = parse_recipe_keys_nbt(&value)?;
    let canonical = canonical_recipe_keys_nbt(&recipe_keys);
    summary.recipe_keys = Some(recipe_keys);
    summary.codec_backed_nbt.insert(type_id, canonical);
    Ok(())
}

fn encode_recipes(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let recipe_keys = summary
        .recipe_keys
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing recipes component value".to_string()))?;
    if recipe_keys.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            recipe_keys.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    for (index, resource_id) in recipe_keys.iter().enumerate() {
        let canonical = normalize_resource_location(resource_id.clone())?;
        if canonical != *resource_id {
            return Err(ProtocolError::InvalidData(format!(
                "recipes[{index}] has noncanonical resource location `{resource_id}`"
            )));
        }
    }
    let canonical = canonical_recipe_keys_nbt(recipe_keys);
    if let Some(retained) = summary.codec_backed_nbt.get(&type_id) {
        if retained != &canonical {
            return Err(ProtocolError::InvalidData(
                "recipes retained codec value does not match its typed keys".to_string(),
            ));
        }
    }
    let encoded = encode_nbt_summary_bounded(Some(&canonical), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

fn parse_recipe_keys_nbt(value: &NbtSummaryValue) -> Result<Vec<String>> {
    let NbtSummaryValue::List(values) = value else {
        return Err(ProtocolError::InvalidData(
            "recipes codec value is not a list".to_string(),
        ));
    };
    if values.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            values.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let NbtSummaryValue::String(resource_id) = value else {
                return Err(ProtocolError::InvalidData(format!(
                    "recipes[{index}] is not a resource location"
                )));
            };
            normalize_resource_location(resource_id.clone())
        })
        .collect()
}

fn canonical_recipe_keys_nbt(recipe_keys: &[String]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        recipe_keys
            .iter()
            .cloned()
            .map(NbtSummaryValue::String)
            .collect(),
    )
}

fn decode_codec_backed_nbt(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    if let Some(value) = decode_nbt_summary_from_decoder(decoder)? {
        summary.codec_backed_nbt.insert(type_id, value);
    }
    Ok(())
}

fn decode_bucket_entity_data(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let value = decode_required_compound_tag(decoder, "bucket_entity_data")?;
    summary.codec_backed_nbt.insert(type_id, value);
    Ok(())
}

fn decode_map_post_processing_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.map_post_processing = Some(decode_map_post_processing(decoder)?);
    Ok(())
}

fn decode_charged_projectiles(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.charged_projectiles_items =
        decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    Ok(())
}

fn decode_bundle_contents(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.bundle_contents_items =
        decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    summary.bundle_contents_item_count = Some(summary.bundle_contents_items.len());
    Ok(())
}

fn decode_potion_contents_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let potion = decode_potion_contents(decoder)?;
    summary.potion_id = potion.potion_id;
    summary.potion_custom_color = potion.custom_color;
    summary.potion_custom_effect_count = Some(potion.custom_effects.len());
    summary.potion_custom_effects = potion.custom_effects;
    summary.potion_custom_name = potion.custom_name;
    Ok(())
}

fn decode_potion_duration_scale(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.potion_duration_scale_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

fn decode_suspicious_stew_effects_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.suspicious_stew_effects = decode_suspicious_stew_effects(decoder)?;
    Ok(())
}

fn decode_writable_book_content_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let writable_book = decode_writable_book_content(decoder)?;
    summary.writable_book_pages = writable_book.pages;
    summary.writable_book_page_filters = writable_book.page_filters;
    Ok(())
}

fn decode_written_book_content_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let written_book = decode_written_book_content(decoder)?;
    summary.written_book = Some(written_book.content);
    summary.written_book_pages_nbt = written_book.pages_nbt;
    summary.written_book_page_filters_nbt = written_book.page_filters_nbt;
    Ok(())
}

fn decode_trim(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let trim = decode_armor_trim(decoder)?;
    summary.armor_trim_material_id = trim.material_id;
    summary.armor_trim_material_direct = trim.material_direct.map(Box::new);
    summary.armor_trim_pattern_id = trim.pattern_id;
    summary.armor_trim_pattern_direct = trim.pattern_direct.map(Box::new);
    Ok(())
}

fn decode_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let entity_data =
        decode_typed_entity_data_summary(decoder, TypedEntityDataRegistry::EntityType)?;
    summary.entity_data_entity_type_id = Some(entity_data.type_id);
    summary.entity_data_tag = Some(entity_data.tag);
    Ok(())
}

fn decode_block_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let block_entity_data =
        decode_typed_entity_data_summary(decoder, TypedEntityDataRegistry::BlockEntityType)?;
    summary.block_entity_data_type_id = Some(block_entity_data.type_id);
    summary.block_entity_spawn_entity_type =
        typed_entity_data_spawn_entity_type(Some(&block_entity_data.tag));
    summary.block_entity_data_tag = Some(block_entity_data.tag);
    Ok(())
}

fn decode_instrument(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let instrument = decode_instrument_component_summary(decoder)?;
    summary.instrument_id = instrument.registry_id;
    if let Some(direct) = instrument.direct {
        summary.instrument_description = Some(direct.description.clone());
        summary.instrument_description_styled = direct
            .description_styled
            .as_deref()
            .map(<[StyledTextRun]>::to_vec);
        summary.instrument_direct = Some(Box::new(direct));
    }
    Ok(())
}

fn decode_jukebox_playable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let song = decode_jukebox_song_holder(decoder)?;
    summary.jukebox_song_id = song.registry_id;
    summary.jukebox_direct_song = song.direct.map(Box::new);
    Ok(())
}

fn decode_provides_banner_patterns(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let patterns = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::BannerPattern)?;
    summary.provides_banner_pattern_registry_ids = patterns.registry_ids;
    summary.provides_banner_pattern_tag = patterns.tag;
    Ok(())
}

fn encode_provides_banner_patterns(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_holder_set_summary(
        &mut encoded,
        &summary.provides_banner_pattern_registry_ids,
        summary.provides_banner_pattern_tag.as_deref(),
        NetworkHolderSetRegistry::BannerPattern,
        "provides_banner_patterns holder set",
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn decode_lodestone_tracker_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (target, tracked) = decode_lodestone_tracker(decoder)?;
    summary.lodestone_target = target;
    summary.lodestone_tracked = Some(tracked);
    Ok(())
}

fn decode_firework_explosion_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let explosion = decode_firework_explosion(decoder)?;
    summary.firework_explosion_colors = explosion.colors;
    summary.firework_explosion_fade_colors = explosion.fade_colors;
    summary.firework_explosion_shape = Some(explosion.shape);
    summary.firework_explosion_has_trail = Some(explosion.has_trail);
    summary.firework_explosion_has_twinkle = Some(explosion.has_twinkle);
    Ok(())
}

fn decode_fireworks_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let fireworks = decode_fireworks(decoder)?;
    summary.fireworks_flight_duration = Some(fireworks.flight_duration);
    summary.fireworks_explosions_count = Some(fireworks.explosions.len());
    summary.fireworks_explosions = fireworks.explosions;
    Ok(())
}

fn decode_profile(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.profile = Some(decode_resolvable_profile(decoder)?);
    Ok(())
}

fn decode_note_block_sound(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.note_block_sound = Some(read_resource_location(decoder)?);
    Ok(())
}

fn decode_banner_patterns(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.banner_pattern_layers = decode_banner_pattern_layers_summary(decoder)?;
    Ok(())
}

fn decode_base_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.base_color_id = Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

fn encode_base_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color_id = summary
        .base_color_id
        .ok_or_else(|| ProtocolError::InvalidData("missing base_color value".to_string()))?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "base_color dye color id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

fn decode_pot_decorations_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pot_decorations_item_ids = decode_pot_decorations_item_ids(decoder)?;
    Ok(())
}

fn encode_pot_decorations(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if summary.pot_decorations_item_ids.len() > MAX_POT_DECORATIONS {
        return Err(ProtocolError::PacketTooLarge(
            summary.pot_decorations_item_ids.len(),
            MAX_POT_DECORATIONS,
        ));
    }
    let mut item_ids = [VANILLA_ITEM_BRICK_ID; MAX_POT_DECORATIONS];
    for (target, item_id) in item_ids.iter_mut().zip(&summary.pot_decorations_item_ids) {
        if vanilla_item_type(*item_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid pot decoration item registry id {item_id}"
            )));
        }
        *target = *item_id;
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(MAX_POT_DECORATIONS as i32);
    for item_id in item_ids {
        encoded.write_var_i32(item_id);
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn decode_container(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let container = decode_item_container_contents(decoder)?;
    summary.container_item_count = Some(container.items.len());
    summary.container_items = container.items;
    summary.container_slots = container.slots;
    summary.container_slot_count = Some(container.slot_count);
    Ok(())
}

fn decode_block_state(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.block_state_properties = decode_string_map(decoder, MAX_BLOCK_STATE_PROPERTIES)?;
    Ok(())
}

fn encode_block_state(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let properties = &summary.block_state_properties;
    if properties.len() > MAX_BLOCK_STATE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            properties.len(),
            MAX_BLOCK_STATE_PROPERTIES,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(properties.len() as i32);
    for (name, value) in properties {
        encode_java_string(&mut encoded, name, MAX_STRING_CHARS)?;
        encode_java_string(&mut encoded, value, MAX_STRING_CHARS)?;
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn decode_bees_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.bees_present = true;
    summary.bees = decode_bees(decoder)?;
    summary.bees_count = summary.bees.len();
    Ok(())
}

fn decode_container_loot(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let value = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
        ProtocolError::InvalidData("container_loot codec value is missing".to_string())
    })?;
    let (loot_table, seed) = parse_container_loot_nbt(&value)?;
    let canonical = canonical_container_loot_nbt(&loot_table, seed);
    summary.container_loot = true;
    summary.container_loot_table = Some(loot_table);
    summary.container_loot_seed = Some(seed);
    summary.codec_backed_nbt.insert(type_id, canonical);
    Ok(())
}

fn encode_container_loot(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if !summary.container_loot {
        return Err(ProtocolError::InvalidData(
            "missing container_loot component value".to_string(),
        ));
    }
    let loot_table = summary.container_loot_table.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing container_loot loot_table".to_string())
    })?;
    let canonical_loot_table = normalize_resource_location(loot_table.to_string())?;
    if canonical_loot_table != loot_table {
        return Err(ProtocolError::InvalidData(format!(
            "container_loot has noncanonical loot_table resource location `{loot_table}`"
        )));
    }
    let seed = summary
        .container_loot_seed
        .ok_or_else(|| ProtocolError::InvalidData("missing container_loot seed".to_string()))?;
    let canonical = canonical_container_loot_nbt(loot_table, seed);
    if let Some(retained) = summary.codec_backed_nbt.get(&type_id) {
        if retained != &canonical {
            return Err(ProtocolError::InvalidData(
                "container_loot retained codec value does not match its typed fields".to_string(),
            ));
        }
    }
    let encoded = encode_nbt_summary_bounded(Some(&canonical), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

fn parse_container_loot_nbt(value: &NbtSummaryValue) -> Result<(String, i64)> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(
            "container_loot codec value is not a compound".to_string(),
        ));
    };
    let loot_table = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "loot_table")
        .ok_or_else(|| {
            ProtocolError::InvalidData("container_loot is missing loot_table".to_string())
        })?;
    let NbtSummaryValue::String(loot_table) = &loot_table.value else {
        return Err(ProtocolError::InvalidData(
            "container_loot loot_table is not a string".to_string(),
        ));
    };
    let loot_table = normalize_resource_location(loot_table.clone())?;
    let seed = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "seed")
        .map(|entry| nbt_number_as_i64(&entry.value, "container_loot seed"))
        .transpose()?
        .unwrap_or(0);
    Ok((loot_table, seed))
}

fn nbt_number_as_i64(value: &NbtSummaryValue, what: &str) -> Result<i64> {
    match value {
        NbtSummaryValue::Byte(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Short(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Int(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Long(value) => Ok(*value),
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits) as i64),
        NbtSummaryValue::Double(bits) => Ok(f64::from_bits(*bits) as i64),
        _ => Err(ProtocolError::InvalidData(format!("{what} is not numeric"))),
    }
}

fn canonical_container_loot_nbt(loot_table: &str, seed: i64) -> NbtSummaryValue {
    let mut entries = vec![NbtSummaryEntry {
        name: "loot_table".to_string(),
        value: NbtSummaryValue::String(loot_table.to_string()),
    }];
    if seed != 0 {
        entries.push(NbtSummaryEntry {
            name: "seed".to_string(),
            value: NbtSummaryValue::Long(seed),
        });
    }
    NbtSummaryValue::Compound(entries)
}

fn decode_break_sound(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.break_sound = Some(decode_sound_event_holder_summary(decoder)?);
    Ok(())
}

fn decode_villager_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.villager_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_villager_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.villager_variant_id, "villager_variant", out)
}

fn encode_wolf_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.wolf_variant_id, "wolf_variant", out)
}

fn encode_wolf_sound_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.wolf_sound_variant_id, "wolf_sound_variant", out)
}

fn decode_wolf_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.wolf_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_wolf_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.wolf_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_frog_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.frog_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_frog_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.frog_variant_id, "frog_variant", out)
}

fn decode_zombie_nautilus_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.zombie_nautilus_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_zombie_nautilus_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(
        summary.zombie_nautilus_variant_id,
        "zombie_nautilus_variant",
        out,
    )
}

fn decode_pig_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pig_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_pig_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.pig_variant_id, "pig_variant", out)
}

fn decode_pig_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pig_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_pig_sound_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.pig_sound_variant_id, "pig_sound_variant", out)
}

fn decode_cow_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cow_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_cow_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.cow_variant_id, "cow_variant", out)
}

fn decode_cow_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cow_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_cow_sound_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.cow_sound_variant_id, "cow_sound_variant", out)
}

fn decode_chicken_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.chicken_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_chicken_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.chicken_variant_id, "chicken_variant", out)
}

fn decode_chicken_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.chicken_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_chicken_sound_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(
        summary.chicken_sound_variant_id,
        "chicken_sound_variant",
        out,
    )
}

fn decode_cat_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cat_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_cat_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.cat_variant_id, "cat_variant", out)
}

fn decode_cat_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cat_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn encode_cat_sound_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_registry_holder_id(summary.cat_sound_variant_id, "cat_sound_variant", out)
}

fn decode_builtin_enum(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let id = decoder.read_var_i32()?;
    let id = normalize_builtin_enum_component_value_id(type_id, id).ok_or_else(|| {
        ProtocolError::InvalidData(format!(
            "unsupported built-in enum component type id {type_id}"
        ))
    })?;
    summary.builtin_enum_component_ids.insert(type_id, id);
    Ok(())
}

fn encode_builtin_enum(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value_id = summary
        .builtin_enum_component_ids
        .get(&type_id)
        .copied()
        .ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "built-in enum component type id {type_id} is missing"
            ))
        })?;
    let normalized =
        normalize_builtin_enum_component_value_id(type_id, value_id).ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "built-in enum component type id {type_id} has no value catalog"
            ))
        })?;
    if normalized != value_id {
        return Err(ProtocolError::InvalidData(format!(
            "built-in enum component type id {type_id} value id {value_id} is not canonical"
        )));
    }
    out.write_var_i32(value_id);
    Ok(())
}

fn decode_tropical_fish_pattern(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_pattern_id =
        Some(vanilla_tropical_fish_pattern(decoder.read_var_i32()?).packed_id);
    Ok(())
}

fn encode_tropical_fish_pattern(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let pattern_id = summary.tropical_fish_pattern_id.ok_or_else(|| {
        ProtocolError::InvalidData("tropical_fish_pattern is missing".to_string())
    })?;
    if vanilla_tropical_fish_pattern(pattern_id).packed_id != pattern_id {
        return Err(ProtocolError::InvalidData(format!(
            "unsupported tropical_fish_pattern packed id {pattern_id}"
        )));
    }
    out.write_var_i32(pattern_id);
    Ok(())
}

fn decode_tropical_fish_base_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_base_color_id =
        Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

fn encode_tropical_fish_base_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color_id = summary.tropical_fish_base_color_id.ok_or_else(|| {
        ProtocolError::InvalidData("tropical_fish_base_color is missing".to_string())
    })?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "tropical_fish_base_color dye id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

fn decode_tropical_fish_pattern_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_pattern_color_id =
        Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

fn encode_tropical_fish_pattern_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color_id = summary.tropical_fish_pattern_color_id.ok_or_else(|| {
        ProtocolError::InvalidData("tropical_fish_pattern_color is missing".to_string())
    })?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "tropical_fish_pattern_color dye id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

fn decode_painting_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let painting = decode_painting_variant_holder_summary(decoder)?;
    summary.painting_variant_id = painting.registry_id;
    summary.painting_variant_direct = painting.direct;
    Ok(())
}

fn skip_unit(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    Ok(())
}

fn decode_tool_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tool = Some(decode_tool(decoder)?);
    Ok(())
}

fn decode_equippable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.equippable = Some(decode_equippable(decoder)?);
    Ok(())
}

fn decode_blocks_attacks_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.blocks_attacks = Some(decode_blocks_attacks(decoder)?);
    Ok(())
}

fn decode_provides_trim_material(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let material = decode_trim_material_holder_summary(decoder)?;
    summary.provides_trim_material_id = material.registry_id;
    summary.provides_trim_material_direct = material.direct.map(Box::new);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use bbb_render_types::vanilla_map_decoration_types;

    use crate::{
        block_entity_types::{
            VANILLA_BLOCK_ENTITY_TYPES, VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
        },
        builtin_enum_component_values::{
            VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_RED_ID, VANILLA_DYE_COLOR_VALUES,
        },
        codec::Encoder,
        entity_types::{
            VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        },
    };

    use super::*;

    #[test]
    fn handler_registry_binds_every_generated_metadata_entry_in_order() {
        let descriptors = vanilla_data_component_descriptors();
        assert_eq!(descriptors.len(), 110);
        assert_eq!(WIRE_DECODE_HANDLERS.len(), 110);
        assert_eq!(descriptors.len(), VANILLA_DATA_COMPONENT_TYPES.len());
        let mut type_ids = HashSet::new();
        let mut resource_ids = HashSet::new();
        for (expected_id, (descriptor, metadata)) in descriptors
            .iter()
            .zip(VANILLA_DATA_COMPONENT_TYPES)
            .enumerate()
        {
            assert_eq!(descriptor.type_id(), expected_id as i32);
            assert_eq!(descriptor.metadata, metadata);
            assert!(type_ids.insert(descriptor.type_id()));
            assert!(resource_ids.insert(descriptor.resource_id()));
        }
        assert_eq!(
            descriptors
                .iter()
                .filter(|descriptor| !descriptor.is_persistent())
                .map(|descriptor| descriptor.type_id())
                .collect::<Vec<_>>(),
            vec![20, 41, 48]
        );
        assert_eq!(
            descriptors
                .iter()
                .filter(|descriptor| descriptor.can_encode())
                .map(|descriptor| descriptor.type_id())
                .collect::<Vec<_>>(),
            vec![
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44,
                45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 58, 59, 60, 61, 62, 63, 64, 65, 66,
                67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88,
                89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107,
                108, 109,
            ]
        );
    }

    #[test]
    fn custom_model_data_descriptor_round_trips_all_stream_lists_bit_exactly() {
        let summary = DataComponentPatchSummary {
            custom_model_data_floats: vec![f32::from_bits(1), -0.0, f32::from_bits(0x7fc0_0001)]
                .into(),
            custom_model_data_flags: vec![true, false],
            custom_model_data_strings: vec!["variant".to_string(), "😀".to_string()],
            custom_model_data_colors: vec![-1, 0x11_22_33],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(3);
        expected.write_f32(f32::from_bits(1));
        expected.write_f32(-0.0);
        expected.write_f32(f32::from_bits(0x7fc0_0001));
        expected.write_var_i32(2);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_var_i32(2);
        expected.write_string("variant");
        expected.write_string("😀");
        expected.write_var_i32(2);
        expected.write_i32(-1);
        expected.write_i32(0x11_22_33);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.custom_model_data_floats,
            summary.custom_model_data_floats
        );
        assert_eq!(
            decoded.custom_model_data_flags,
            summary.custom_model_data_flags
        );
        assert_eq!(
            decoded.custom_model_data_strings,
            summary.custom_model_data_strings
        );
        assert_eq!(
            decoded.custom_model_data_colors,
            summary.custom_model_data_colors
        );
    }

    #[test]
    fn repair_cost_descriptor_round_trips_nonnegative_var_int_values() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID).unwrap();
        for value in [0, i32::MAX] {
            let summary = DataComponentPatchSummary {
                repair_cost: Some(value),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(value);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.repair_cost, Some(value));
        }
    }

    #[test]
    fn repair_cost_descriptor_rejects_negative_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID).unwrap();
        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                repair_cost: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0x7f);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0x7f]);
        }

        for value in [-1, i32::MIN] {
            let mut encoded = Encoder::new();
            encoded.write_var_i32(value);
            let wire = encoded.into_inner();
            let mut decoded = DataComponentPatchSummary {
                repair_cost: Some(7),
                ..DataComponentPatchSummary::default()
            };
            assert!(descriptor
                .decode(&mut Decoder::new(&wire), &mut decoded)
                .is_err());
            assert_eq!(decoded.repair_cost, Some(7));
        }
    }

    #[test]
    fn additional_trade_cost_descriptor_round_trips_signed_var_int_values() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID)
                .unwrap();
        for value in [i32::MIN, 0, i32::MAX] {
            let summary = DataComponentPatchSummary {
                additional_trade_cost: Some(value),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(value);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.additional_trade_cost, Some(value));
        }
    }

    #[test]
    fn additional_trade_cost_descriptor_rejects_a_missing_value_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        encoded.write_u8(0x7f);
        assert!(descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .is_err());
        assert_eq!(encoded.into_inner(), [0x7f]);
    }

    #[test]
    fn damage_resistant_descriptor_round_trips_direct_and_named_holder_sets_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID)
                .unwrap();

        for summary in [
            DataComponentPatchSummary {
                damage_resistant_type_registry_ids: vec![2, 4],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.damage_resistant_type_registry_ids,
                summary.damage_resistant_type_registry_ids
            );
            assert_eq!(
                decoded.damage_resistant_type_tag,
                summary.damage_resistant_type_tag
            );
        }

        for summary in [
            DataComponentPatchSummary {
                damage_resistant_type_registry_ids: vec![-1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                damage_resistant_type_tag: Some("is_fire".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                damage_resistant_type_registry_ids: vec![2],
                damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn repairable_descriptor_round_trips_direct_and_named_holder_sets_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID).unwrap();

        for summary in [
            DataComponentPatchSummary {
                repairable_item_registry_ids: vec![1, 193],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                repairable_item_tag: Some("minecraft:repairs_leather_armor".to_string()),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.repairable_item_registry_ids,
                summary.repairable_item_registry_ids
            );
            assert_eq!(decoded.repairable_item_tag, summary.repairable_item_tag);
        }

        for summary in [
            DataComponentPatchSummary {
                repairable_item_registry_ids: vec![-1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                repairable_item_registry_ids: vec![i32::MAX],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                repairable_item_tag: Some("repairs_leather_armor".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                repairable_item_registry_ids: vec![1],
                repairable_item_tag: Some("minecraft:repairs_leather_armor".to_string()),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn weapon_descriptor_round_trips_wire_fields_and_rejects_invalid_persistent_values_atomically()
    {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID).unwrap();
        let summary = DataComponentPatchSummary {
            weapon_item_damage_per_attack: Some(3),
            weapon_disable_blocking_seconds_bits: Some(5.0_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(3);
        expected.write_f32(5.0);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.weapon_item_damage_per_attack, Some(3));
        assert_eq!(
            decoded.weapon_disable_blocking_seconds_bits,
            Some(5.0_f32.to_bits())
        );

        for summary in [
            DataComponentPatchSummary {
                weapon_disable_blocking_seconds_bits: Some(0.0_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                weapon_item_damage_per_attack: Some(1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                weapon_item_damage_per_attack: Some(-1),
                weapon_disable_blocking_seconds_bits: Some(0.0_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                weapon_item_damage_per_attack: Some(1),
                weapon_disable_blocking_seconds_bits: Some((-0.0_f32).to_bits()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                weapon_item_damage_per_attack: Some(1),
                weapon_disable_blocking_seconds_bits: Some(f32::NAN.to_bits()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                weapon_item_damage_per_attack: Some(1),
                weapon_disable_blocking_seconds_bits: Some(f32::INFINITY.to_bits()),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn container_loot_descriptor_canonicalizes_codec_nbt_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID)
                .unwrap();
        for summary in [
            DataComponentPatchSummary {
                container_loot: true,
                container_loot_table: Some("minecraft:empty".to_string()),
                container_loot_seed: Some(0),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                container_loot: true,
                container_loot_table: Some("minecraft:chests/simple_dungeon".to_string()),
                container_loot_seed: Some(12),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert!(decoded.container_loot);
            assert_eq!(decoded.container_loot_table, summary.container_loot_table);
            assert_eq!(decoded.container_loot_seed, summary.container_loot_seed);
            assert_eq!(
                decoded
                    .codec_backed_nbt
                    .get(&VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID),
                Some(&canonical_container_loot_nbt(
                    summary.container_loot_table.as_deref().unwrap(),
                    summary.container_loot_seed.unwrap()
                ))
            );
        }

        let shorthand = canonical_container_loot_nbt("empty", 0);
        let wire =
            encode_nbt_summary_bounded(Some(&shorthand), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&wire), &mut decoded)
            .unwrap();
        assert_eq!(
            decoded.container_loot_table.as_deref(),
            Some("minecraft:empty")
        );
        assert_eq!(
            decoded
                .codec_backed_nbt
                .get(&VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID),
            Some(&canonical_container_loot_nbt("minecraft:empty", 0))
        );

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                container_loot: true,
                container_loot_seed: Some(0),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                container_loot: true,
                container_loot_table: Some("empty".to_string()),
                container_loot_seed: Some(0),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                container_loot: true,
                container_loot_table: Some("minecraft:empty".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                container_loot: true,
                container_loot_table: Some("minecraft:empty".to_string()),
                container_loot_seed: Some(0),
                codec_backed_nbt: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
                    canonical_container_loot_nbt("minecraft:chests/simple_dungeon", 12),
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        for malformed in [
            NbtSummaryValue::String("minecraft:empty".to_string()),
            NbtSummaryValue::Compound(Vec::new()),
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "loot_table".to_string(),
                value: NbtSummaryValue::Int(1),
            }]),
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "loot_table".to_string(),
                    value: NbtSummaryValue::String("minecraft:empty".to_string()),
                },
                NbtSummaryEntry {
                    name: "seed".to_string(),
                    value: NbtSummaryValue::String("bad".to_string()),
                },
            ]),
        ] {
            let wire =
                encode_nbt_summary_bounded(Some(&malformed), i32::MAX as usize, MAX_NBT_DEPTH)
                    .unwrap();
            assert!(descriptor
                .decode(
                    &mut Decoder::new(&wire),
                    &mut DataComponentPatchSummary::default()
                )
                .is_err());
        }
        let missing = encode_nbt_summary_bounded(None, i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        assert!(descriptor
            .decode(
                &mut Decoder::new(&missing),
                &mut DataComponentPatchSummary::default()
            )
            .is_err());
    }

    #[test]
    fn custom_model_data_descriptor_enforces_collection_and_utf16_bounds_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID)
                .unwrap();
        let oversized_string = "😀".repeat(MAX_STRING_CHARS / 2 + 1);
        let cases = [
            DataComponentPatchSummary {
                custom_model_data_floats: vec![0.0; MAX_DATA_COMPONENT_LIST_ITEMS + 1].into(),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                custom_model_data_flags: vec![false; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                custom_model_data_strings: vec![String::new(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                custom_model_data_colors: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                custom_model_data_strings: vec![oversized_string.clone()],
                ..DataComponentPatchSummary::default()
            },
        ];
        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0x7f);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), vec![0x7f]);
        }

        let maximal_string = format!("{}a", "😀".repeat((MAX_STRING_CHARS.saturating_sub(1)) / 2));
        assert_eq!(maximal_string.encode_utf16().count(), MAX_STRING_CHARS);
        let maximal = DataComponentPatchSummary {
            custom_model_data_floats: vec![0.0; MAX_DATA_COMPONENT_LIST_ITEMS].into(),
            custom_model_data_strings: vec![maximal_string],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&maximal, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&wire), &mut decoded)
            .unwrap();
        assert_eq!(
            decoded.custom_model_data_floats.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS
        );
        assert_eq!(
            decoded.custom_model_data_strings[0].encode_utf16().count(),
            MAX_STRING_CHARS
        );

        let mut invalid_wire = Encoder::new();
        invalid_wire.write_var_i32(0);
        invalid_wire.write_var_i32(0);
        invalid_wire.write_var_i32(1);
        invalid_wire.write_string(&oversized_string);
        invalid_wire.write_var_i32(0);
        let wire = invalid_wire.into_inner();
        let error = descriptor
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::StringTooLong {
                actual,
                max: MAX_STRING_CHARS
            } if actual == MAX_STRING_CHARS + 1
        ));
    }

    #[test]
    fn tooltip_display_descriptor_round_trips_order_and_collapses_wire_duplicates() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID)
                .unwrap();
        let summary = DataComponentPatchSummary {
            tooltip_hide_tooltip: true,
            tooltip_hidden_component_type_ids: vec![11, 20, 109],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        assert_eq!(encoded.into_inner(), [1, 3, 11, 20, 109]);

        let mut wire = Encoder::new();
        wire.write_bool(false);
        wire.write_var_i32(5);
        for type_id in [13, 11, 13, 20, 11] {
            wire.write_var_i32(type_id);
        }
        let wire = wire.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert!(!decoded.tooltip_hide_tooltip);
        assert_eq!(decoded.tooltip_hidden_component_type_ids, [13, 11, 20]);
    }

    #[test]
    fn tooltip_display_descriptor_rejects_noncanonical_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID)
                .unwrap();
        for hidden_components in [
            vec![-1],
            vec![i32::try_from(VANILLA_DATA_COMPONENT_TYPES.len()).unwrap()],
            vec![11, 11],
            vec![11; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
        ] {
            let summary = DataComponentPatchSummary {
                tooltip_hidden_component_type_ids: hidden_components,
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            encoded.write_u8(0x7f);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0x7f]);
        }

        for invalid_type_id in [
            -1,
            i32::try_from(VANILLA_DATA_COMPONENT_TYPES.len()).unwrap(),
        ] {
            let mut wire = Encoder::new();
            wire.write_bool(true);
            wire.write_var_i32(2);
            wire.write_var_i32(11);
            wire.write_var_i32(invalid_type_id);
            let wire = wire.into_inner();
            let mut decoded = DataComponentPatchSummary::default();
            assert!(descriptor
                .decode(&mut Decoder::new(&wire), &mut decoded)
                .is_err());
            assert_eq!(decoded, DataComponentPatchSummary::default());
        }

        let mut oversized = Encoder::new();
        oversized.write_bool(false);
        oversized.write_var_i32((MAX_DATA_COMPONENT_LIST_ITEMS + 1) as i32);
        let wire = oversized.into_inner();
        assert!(matches!(
            descriptor.decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default()
            ),
            Err(ProtocolError::PacketTooLarge(
                _,
                MAX_DATA_COMPONENT_LIST_ITEMS
            ))
        ));
    }

    #[test]
    fn tool_descriptor_round_trips_rules_holders_and_raw_defaults_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID).unwrap();
        let summary = DataComponentPatchSummary {
            tool: Some(ToolSummary {
                rules: vec![
                    ToolRuleSummary {
                        block_registry_ids: Vec::new(),
                        block_tag: Some("minecraft:mineable/pickaxe".to_string()),
                        speed_bits: Some(8.0_f32.to_bits()),
                        correct_for_drops: Some(true),
                    },
                    ToolRuleSummary {
                        block_registry_ids: vec![1, 193],
                        block_tag: None,
                        speed_bits: None,
                        correct_for_drops: Some(false),
                    },
                ],
                default_mining_speed_bits: (-0.0_f32).to_bits(),
                damage_per_block: 2,
                can_destroy_blocks_in_creative: false,
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(2);
        expected.write_var_i32(0);
        expected.write_string("minecraft:mineable/pickaxe");
        expected.write_bool(true);
        expected.write_f32(8.0);
        expected.write_bool(true);
        expected.write_bool(true);
        expected.write_var_i32(3);
        expected.write_var_i32(1);
        expected.write_var_i32(193);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_f32(-0.0);
        expected.write_var_i32(2);
        expected.write_bool(false);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.tool, summary.tool);
    }

    #[test]
    fn tool_descriptor_rejects_persistent_range_and_holder_errors_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID).unwrap();
        let valid = ToolSummary {
            rules: vec![ToolRuleSummary {
                block_registry_ids: vec![1],
                block_tag: None,
                speed_bits: Some(1.0_f32.to_bits()),
                correct_for_drops: None,
            }],
            default_mining_speed_bits: 1.0_f32.to_bits(),
            damage_per_block: 1,
            can_destroy_blocks_in_creative: true,
        };
        let invalid_holder_id =
            i32::try_from(crate::block_types::VANILLA_BLOCK_TYPES.len()).unwrap();
        let cases = [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                tool: Some(ToolSummary {
                    rules: vec![valid.rules[0].clone(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                    ..valid.clone()
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tool: Some(ToolSummary {
                    damage_per_block: -1,
                    ..valid.clone()
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tool: Some(ToolSummary {
                    rules: vec![ToolRuleSummary {
                        speed_bits: Some(0.0_f32.to_bits()),
                        ..valid.rules[0].clone()
                    }],
                    ..valid.clone()
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tool: Some(ToolSummary {
                    rules: vec![ToolRuleSummary {
                        speed_bits: Some(f32::INFINITY.to_bits()),
                        ..valid.rules[0].clone()
                    }],
                    ..valid.clone()
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tool: Some(ToolSummary {
                    rules: vec![ToolRuleSummary {
                        block_registry_ids: vec![invalid_holder_id],
                        ..valid.rules[0].clone()
                    }],
                    ..valid.clone()
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tool: Some(ToolSummary {
                    rules: vec![ToolRuleSummary {
                        block_registry_ids: Vec::new(),
                        block_tag: Some("mineable/pickaxe".to_string()),
                        ..valid.rules[0].clone()
                    }],
                    ..valid.clone()
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tool: Some(ToolSummary {
                    rules: vec![ToolRuleSummary {
                        block_tag: Some("minecraft:mineable/pickaxe".to_string()),
                        ..valid.rules[0].clone()
                    }],
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
        ];
        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0x7f);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0x7f]);
        }
    }

    #[test]
    fn attack_range_descriptor_round_trips_six_raw_floats_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID).unwrap();
        let range = AttackRangeSummary {
            min_reach: 0.0,
            max_reach: 64.0,
            min_creative_reach: f32::from_bits(0x3f80_0001),
            max_creative_reach: 6.5,
            hitbox_margin: 0.125,
            mob_factor: 2.0,
        };
        let summary = DataComponentPatchSummary {
            attack_range: Some(range),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_f32(range.min_reach);
        expected.write_f32(range.max_reach);
        expected.write_f32(range.min_creative_reach);
        expected.write_f32(range.max_creative_reach);
        expected.write_f32(range.hitbox_margin);
        expected.write_f32(range.mob_factor);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.attack_range, Some(range));
    }

    #[test]
    fn attack_range_descriptor_rejects_persistent_ranges_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID).unwrap();
        let valid = AttackRangeSummary {
            min_reach: 0.0,
            max_reach: 3.0,
            min_creative_reach: 0.0,
            max_creative_reach: 5.0,
            hitbox_margin: 0.3,
            mob_factor: 1.0,
        };
        let cases = [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                attack_range: Some(AttackRangeSummary {
                    min_reach: -0.0,
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                attack_range: Some(AttackRangeSummary {
                    min_reach: -0.01,
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                attack_range: Some(AttackRangeSummary {
                    max_reach: 64.01,
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                attack_range: Some(AttackRangeSummary {
                    min_creative_reach: f32::NAN,
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                attack_range: Some(AttackRangeSummary {
                    max_creative_reach: f32::INFINITY,
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                attack_range: Some(AttackRangeSummary {
                    hitbox_margin: 1.01,
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                attack_range: Some(AttackRangeSummary {
                    mob_factor: 2.01,
                    ..valid
                }),
                ..DataComponentPatchSummary::default()
            },
        ];
        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0x7f);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0x7f]);
        }
    }

    #[test]
    fn enchantable_descriptor_round_trips_positive_var_int_values() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID).unwrap();
        for value in [1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                enchantable_value: Some(value),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(value);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.enchantable_value, Some(value));
        }
    }

    #[test]
    fn enchantable_descriptor_rejects_nonpositive_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID).unwrap();
        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                enchantable_value: Some(0),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                enchantable_value: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0x7f);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0x7f]);
        }

        for value in [0, -1] {
            let mut encoded = Encoder::new();
            encoded.write_var_i32(value);
            let wire = encoded.into_inner();
            let mut decoded = DataComponentPatchSummary {
                enchantable_value: Some(7),
                ..DataComponentPatchSummary::default()
            };
            assert!(descriptor
                .decode(&mut Decoder::new(&wire), &mut decoded)
                .is_err());
            assert_eq!(decoded.enchantable_value, Some(7));
        }
    }

    #[test]
    fn equippable_descriptor_round_trips_options_holders_and_flags_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID).unwrap();
        let summary = DataComponentPatchSummary {
            equippable: Some(EquippableSummary {
                slot_id: 4,
                equip_sound: SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: None,
                    fixed_range_bits: None,
                },
                asset_id: Some("minecraft:diamond".to_string()),
                camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
                allowed_entities: Some(EntityTypeHolderSetSummary {
                    registry_ids: vec![VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID],
                    tag: None,
                }),
                dispensable: true,
                swappable: false,
                damage_on_hurt: true,
                equip_on_interact: false,
                can_be_sheared: true,
                shearing_sound: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:item.shear".to_string()),
                    fixed_range_bits: Some(7.5_f32.to_bits()),
                },
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(4);
        expected.write_var_i32(1);
        expected.write_bool(true);
        expected.write_string("minecraft:diamond");
        expected.write_bool(true);
        expected.write_string("minecraft:misc/pumpkinblur");
        expected.write_bool(true);
        expected.write_var_i32(3);
        expected.write_var_i32(VANILLA_ENTITY_TYPE_BEE_ID);
        expected.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("example:item.shear");
        expected.write_bool(true);
        expected.write_f32(7.5);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);

        let tagged = DataComponentPatchSummary {
            equippable: Some(EquippableSummary {
                slot_id: 7,
                equip_sound: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:item.equip".to_string()),
                    fixed_range_bits: None,
                },
                asset_id: None,
                camera_overlay: None,
                allowed_entities: Some(EntityTypeHolderSetSummary {
                    registry_ids: Vec::new(),
                    tag: Some("minecraft:can_equip_saddle".to_string()),
                }),
                dispensable: false,
                swappable: true,
                damage_on_hurt: false,
                equip_on_interact: true,
                can_be_sheared: false,
                shearing_sound: SoundEventSummary {
                    registry_id: Some(1),
                    sound_id: None,
                    fixed_range_bits: None,
                },
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&tagged, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, tagged);
    }

    #[test]
    fn equippable_descriptor_rejects_noncanonical_projections_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID).unwrap();
        let valid = EquippableSummary {
            slot_id: 4,
            equip_sound: SoundEventSummary {
                registry_id: Some(0),
                sound_id: None,
                fixed_range_bits: None,
            },
            asset_id: Some("minecraft:diamond".to_string()),
            camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
            allowed_entities: Some(EntityTypeHolderSetSummary {
                registry_ids: vec![VANILLA_ENTITY_TYPE_BEE_ID],
                tag: None,
            }),
            dispensable: true,
            swappable: true,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared: false,
            shearing_sound: SoundEventSummary {
                registry_id: Some(1),
                sound_id: None,
                fixed_range_bits: None,
            },
        };
        let mut cases = vec![DataComponentPatchSummary::default()];
        for equippable in [
            EquippableSummary {
                slot_id: 8,
                ..valid.clone()
            },
            EquippableSummary {
                asset_id: Some("Example:diamond".to_string()),
                ..valid.clone()
            },
            EquippableSummary {
                camera_overlay: Some("bad path".to_string()),
                ..valid.clone()
            },
            EquippableSummary {
                allowed_entities: Some(EntityTypeHolderSetSummary {
                    registry_ids: vec![VANILLA_ENTITY_TYPE_BEE_ID],
                    tag: Some("minecraft:bees".to_string()),
                }),
                ..valid.clone()
            },
            EquippableSummary {
                allowed_entities: Some(EntityTypeHolderSetSummary {
                    registry_ids: vec![-1],
                    tag: None,
                }),
                ..valid.clone()
            },
            EquippableSummary {
                equip_sound: SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: Some("example:mixed".to_string()),
                    fixed_range_bits: None,
                },
                ..valid.clone()
            },
            EquippableSummary {
                shearing_sound: SoundEventSummary {
                    registry_id: None,
                    sound_id: None,
                    fixed_range_bits: None,
                },
                ..valid
            },
        ] {
            cases.push(DataComponentPatchSummary {
                equippable: Some(equippable),
                ..DataComponentPatchSummary::default()
            });
        }
        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0x7f);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0x7f]);
        }
    }

    #[test]
    fn use_cooldown_descriptor_round_trips_raw_seconds_and_optional_group_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID).unwrap();
        let ungrouped = DataComponentPatchSummary {
            use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
            use_cooldown_ticks: Some(25),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&ungrouped, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_f32(1.25);
        expected.write_bool(false);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, ungrouped);

        let grouped = DataComponentPatchSummary {
            use_cooldown_seconds_bits: Some(2.5_f32.to_bits()),
            use_cooldown_ticks: Some(50),
            use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&grouped, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_f32(2.5);
        expected.write_bool(true);
        expected.write_string("minecraft:ender_pearl");
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, grouped);

        for seconds in [f32::from_bits(1), f32::MAX] {
            let boundary = DataComponentPatchSummary {
                use_cooldown_seconds_bits: Some(seconds.to_bits()),
                use_cooldown_ticks: Some(vanilla_use_cooldown_ticks(seconds)),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&boundary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, boundary);
        }
    }

    #[test]
    fn use_cooldown_descriptor_rejects_noncanonical_projections_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID).unwrap();
        let valid = DataComponentPatchSummary {
            use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
            use_cooldown_ticks: Some(25),
            ..DataComponentPatchSummary::default()
        };
        let mut cases = vec![
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
                use_cooldown_ticks: Some(24),
                ..DataComponentPatchSummary::default()
            },
        ];
        for seconds in [0.0, -0.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            cases.push(DataComponentPatchSummary {
                use_cooldown_seconds_bits: Some(seconds.to_bits()),
                use_cooldown_ticks: Some(vanilla_use_cooldown_ticks(seconds)),
                ..DataComponentPatchSummary::default()
            });
        }
        for group in [
            "ender_pearl".to_string(),
            "Bad:group".to_string(),
            format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1)),
        ] {
            cases.push(DataComponentPatchSummary {
                use_cooldown_group: Some(group),
                ..valid.clone()
            });
        }

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
        descriptor.encode(&valid, &mut Encoder::new()).unwrap();
    }

    #[test]
    fn adventure_mode_predicate_descriptor_round_trips_all_stream_fields() {
        let value = AdventureModePredicateDataSummary {
            predicates: vec![
                BlockPredicateSummary {
                    blocks: Some(BlockHolderSetSummary {
                        registry_ids: vec![1],
                        tag: None,
                    }),
                    state: Some(vec![
                        StatePropertyMatcherSummary {
                            name: "facing".to_string(),
                            value: StatePropertyValueMatcherSummary::Exact("north".to_string()),
                        },
                        StatePropertyMatcherSummary {
                            name: "age".to_string(),
                            value: StatePropertyValueMatcherSummary::Range {
                                min: Some("1".to_string()),
                                max: Some("3".to_string()),
                            },
                        },
                    ]),
                    nbt: Some(NbtSummaryValue::Compound(Vec::new())),
                    exact_components: Some(Box::new(DataComponentPatchSummary {
                        added: 1,
                        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                        max_stack_size: Some(64),
                        ..DataComponentPatchSummary::default()
                    })),
                    partial_components: vec![
                        PartialDataComponentPredicateSummary {
                            predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
                                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                            ),
                            wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                        },
                        PartialDataComponentPredicateSummary {
                            predicate_type: DataComponentPredicateTypeSummary::Concrete(0),
                            wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                        },
                    ],
                },
                BlockPredicateSummary {
                    blocks: Some(BlockHolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:mineable/pickaxe".to_string()),
                    }),
                    ..BlockPredicateSummary::default()
                },
            ],
        };
        let tooltip = AdventureModePredicateSummary {
            unknown: false,
            block_registry_ids: vec![1],
            unresolved_block_tag_count: 1,
        };
        let summary = DataComponentPatchSummary {
            can_place_on: Some(tooltip.clone()),
            can_place_on_persistent: Some(value.clone()),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(2);
        expected.write_bool(true);
        expected.write_var_i32(2);
        expected.write_var_i32(1);
        expected.write_bool(true);
        expected.write_var_i32(2);
        expected.write_string("facing");
        expected.write_bool(true);
        expected.write_string("north");
        expected.write_string("age");
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_string("1");
        expected.write_bool(true);
        expected.write_string("3");
        expected.write_bool(true);
        expected.write_bytes(&[10, 0]);
        expected.write_var_i32(1);
        expected.write_var_i32(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
        expected.write_var_i32(64);
        expected.write_var_i32(2);
        expected.write_bool(false);
        expected.write_var_i32(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID);
        expected.write_bytes(&[10, 0]);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_bytes(&[10, 0]);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("minecraft:mineable/pickaxe");
        expected.write_bool(false);
        expected.write_bool(false);
        expected.write_var_i32(0);
        expected.write_var_i32(0);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.can_place_on, Some(tooltip));
        assert_eq!(decoded.can_place_on_persistent, Some(value));
    }

    #[test]
    fn adventure_mode_predicate_descriptor_rejects_invalid_values_atomically() {
        let base_value = AdventureModePredicateDataSummary {
            predicates: vec![BlockPredicateSummary {
                blocks: Some(BlockHolderSetSummary {
                    registry_ids: vec![1],
                    tag: None,
                }),
                ..BlockPredicateSummary::default()
            }],
        };
        let base_tooltip = AdventureModePredicateSummary {
            unknown: false,
            block_registry_ids: vec![1],
            unresolved_block_tag_count: 0,
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID).unwrap();
        let mut cases = Vec::new();

        cases.push((
            DataComponentPatchSummary {
                can_break: Some(AdventureModePredicateSummary {
                    unknown: true,
                    ..base_tooltip.clone()
                }),
                can_break_persistent: Some(base_value.clone()),
                ..DataComponentPatchSummary::default()
            },
            "noncanonical tooltip projection",
        ));

        let mut mixed_holder = base_value.clone();
        mixed_holder.predicates[0].blocks.as_mut().unwrap().tag =
            Some("minecraft:mineable/pickaxe".to_string());
        cases.push((
            DataComponentPatchSummary {
                can_break: Some(AdventureModePredicateSummary {
                    unresolved_block_tag_count: 1,
                    ..base_tooltip.clone()
                }),
                can_break_persistent: Some(mixed_holder),
                ..DataComponentPatchSummary::default()
            },
            "cannot contain both holder ids and a tag",
        ));

        let mut scalar_nbt = base_value.clone();
        scalar_nbt.predicates[0].nbt = Some(NbtSummaryValue::Int(1));
        cases.push((
            DataComponentPatchSummary {
                can_break: Some(base_tooltip.clone()),
                can_break_persistent: Some(scalar_nbt),
                ..DataComponentPatchSummary::default()
            },
            "nbt must be a compound tag",
        ));

        let mut missing_partial_value = base_value.clone();
        missing_partial_value.predicates[0].partial_components =
            vec![PartialDataComponentPredicateSummary {
                predicate_type: DataComponentPredicateTypeSummary::Concrete(0),
                wire_value: None,
            }];
        cases.push((
            DataComponentPatchSummary {
                can_break: Some(base_tooltip),
                can_break_persistent: Some(missing_partial_value),
                ..DataComponentPatchSummary::default()
            },
            "missing non-null nbt",
        ));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn adventure_mode_predicate_decode_and_encode_bound_recursive_exact_components() {
        let mut wire = Encoder::new();
        write_nested_adventure_mode_predicate(
            &mut wire,
            MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH + 1,
        );
        let wire = wire.into_inner();
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID).unwrap();
        let error = descriptor
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("adventure mode predicate nesting exceeds"),
            "{error}"
        );

        let nested =
            nested_adventure_mode_predicate_summary(MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH + 1);
        let summary = DataComponentPatchSummary {
            can_place_on: Some(AdventureModePredicateSummary {
                unknown: true,
                ..AdventureModePredicateSummary::default()
            }),
            can_place_on_persistent: Some(nested),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("adventure mode predicate nesting exceeds"),
            "{error}"
        );
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    fn write_nested_adventure_mode_predicate(out: &mut Encoder, depth: usize) {
        out.write_var_i32(1);
        out.write_bool(false);
        out.write_bool(false);
        out.write_bool(false);
        if depth > 1 {
            out.write_var_i32(1);
            out.write_var_i32(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID);
            write_nested_adventure_mode_predicate(out, depth - 1);
        } else {
            out.write_var_i32(0);
        }
        out.write_var_i32(0);
    }

    fn nested_adventure_mode_predicate_summary(depth: usize) -> AdventureModePredicateDataSummary {
        let exact_components = (depth > 1).then(|| {
            Box::new(DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
                can_place_on: Some(AdventureModePredicateSummary {
                    unknown: true,
                    ..AdventureModePredicateSummary::default()
                }),
                can_place_on_persistent: Some(nested_adventure_mode_predicate_summary(depth - 1)),
                ..DataComponentPatchSummary::default()
            })
        });
        AdventureModePredicateDataSummary {
            predicates: vec![BlockPredicateSummary {
                exact_components,
                ..BlockPredicateSummary::default()
            }],
        }
    }

    #[test]
    fn attribute_modifiers_descriptor_round_trips_exact_display_variants() {
        let display_text_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Override".to_string()),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        let display_text_styled = vec![StyledTextRun {
            text: "Override".to_string(),
            style: crate::ComponentStyle {
                bold: Some(true),
                ..crate::ComponentStyle::default()
            },
        }];
        let summary = DataComponentPatchSummary {
            attribute_modifiers: vec![
                AttributeModifierSummary {
                    attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                    modifier_id: "minecraft:test/default".to_string(),
                    amount_bits: (-0.0f64).to_bits(),
                    operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                    slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                    display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
                    display_text: None,
                    display_text_styled: None,
                    display_text_nbt: None,
                },
                AttributeModifierSummary {
                    attribute_id:
                        crate::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                    modifier_id: "minecraft:test/hidden".to_string(),
                    amount_bits: f64::INFINITY.to_bits(),
                    operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                    slot_id:
                        crate::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
                    display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
                    display_text: None,
                    display_text_styled: None,
                    display_text_nbt: None,
                },
                AttributeModifierSummary {
                    attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                    modifier_id: "example:override".to_string(),
                    amount_bits: 1.5f64.to_bits(),
                    operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                    slot_id: 7,
                    display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    display_text: Some("Override".to_string()),
                    display_text_styled: Some(display_text_styled),
                    display_text_nbt: Some(display_text_nbt.clone()),
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_var_i32(3);
        for (attribute, id, amount, operation, slot, display) in [
            (
                crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                "minecraft:test/default",
                -0.0,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
            ),
            (
                crate::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                "minecraft:test/hidden",
                f64::INFINITY,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                crate::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
            ),
            (
                crate::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                "example:override",
                1.5,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                7,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
            ),
        ] {
            expected.write_var_i32(attribute);
            expected.write_string(id);
            expected.write_f64(amount);
            expected.write_var_i32(operation);
            expected.write_var_i32(slot);
            expected.write_var_i32(display);
            if display
                == crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID
            {
                expected.write_bytes(
                    &encode_nbt_summary_bounded(
                        Some(&display_text_nbt),
                        i32::MAX as usize,
                        MAX_NBT_DEPTH,
                    )
                    .unwrap(),
                );
            }
        }
        assert_eq!(encoded, expected.into_inner());

        let mut decoder = Decoder::new(&encoded);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.attribute_modifiers, summary.attribute_modifiers);
    }

    #[test]
    fn attribute_modifiers_descriptor_rejects_noncanonical_summaries_atomically() {
        let valid = AttributeModifierSummary {
            attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            modifier_id: "minecraft:test".to_string(),
            amount_bits: 1.0f64.to_bits(),
            operation_id:
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
            display_id:
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        };
        let invalid = [
            (
                AttributeModifierSummary {
                    attribute_id: i32::try_from(crate::attribute_types::VANILLA_ATTRIBUTE_TYPES.len())
                        .unwrap(),
                    ..valid.clone()
                },
                "invalid vanilla attribute holder",
            ),
            (
                AttributeModifierSummary {
                    modifier_id: "test".to_string(),
                    ..valid.clone()
                },
                "noncanonical attribute modifier resource location",
            ),
            (
                AttributeModifierSummary {
                    operation_id: 3,
                    ..valid.clone()
                },
                "invalid attribute modifier operation",
            ),
            (
                AttributeModifierSummary {
                    slot_id: 99,
                    ..valid.clone()
                },
                "invalid equipment slot group",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
                    display_text: Some("dangling".to_string()),
                    ..valid.clone()
                },
                "cannot contain override text",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    ..valid.clone()
                },
                "missing component nbt",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    display_text: Some("stale".to_string()),
                    display_text_styled: Some(vec![StyledTextRun {
                        text: "wire".to_string(),
                        style: crate::ComponentStyle::default(),
                    }]),
                    display_text_nbt: Some(NbtSummaryValue::String("wire".to_string())),
                    ..valid.clone()
                },
                "noncanonical component projections",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    display_text: Some("invalid".to_string()),
                    display_text_styled: Some(Vec::new()),
                    display_text_nbt: Some(NbtSummaryValue::Int(1)),
                    ..valid.clone()
                },
                "is not a component",
            ),
            (
                AttributeModifierSummary {
                    display_id: 3,
                    ..valid.clone()
                },
                "invalid attribute modifier display",
            ),
        ];
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID)
                .unwrap();
        for (modifier, expected_error) in invalid {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor
                .encode(
                    &DataComponentPatchSummary {
                        attribute_modifiers: vec![modifier],
                        ..DataComponentPatchSummary::default()
                    },
                    &mut encoded,
                )
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor
            .encode(
                &DataComponentPatchSummary {
                    attribute_modifiers: vec![valid; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, max)
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn food_descriptor_round_trips_raw_network_values_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FOOD_TYPE_ID).unwrap();
        for (nutrition, saturation_bits, can_always_eat) in [
            (6, 7.2_f32.to_bits(), true),
            (-1, f32::from_bits(0x7fc0_1234).to_bits(), false),
        ] {
            let summary = DataComponentPatchSummary {
                food_nutrition: Some(nutrition),
                food_saturation_bits: Some(saturation_bits),
                food_can_always_eat: can_always_eat,
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(nutrition);
            expected.write_f32(f32::from_bits(saturation_bits));
            expected.write_bool(can_always_eat);
            let encoded = encoded.into_inner();
            assert_eq!(encoded, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&encoded);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.food_nutrition, Some(nutrition));
            assert_eq!(decoded.food_saturation_bits, Some(saturation_bits));
            assert_eq!(decoded.food_can_always_eat, can_always_eat);
        }
    }

    #[test]
    fn food_descriptor_rejects_incomplete_projection_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FOOD_TYPE_ID).unwrap();
        for summary in [
            DataComponentPatchSummary {
                food_saturation_bits: Some(0.0_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                food_nutrition: Some(0),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn glider_descriptor_round_trips_empty_unit_payload() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .unwrap();

        let encoded = encoded.into_inner();
        assert!(encoded.is_empty());
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, DataComponentPatchSummary::default());
    }

    #[test]
    fn consumable_descriptor_round_trips_all_effect_branches_in_wire_order() {
        let nan_bits = 0x7fc0_1234;
        let summary = DataComponentPatchSummary {
            consumable: Some(ConsumableSummary {
                consume_seconds: f32::from_bits(nan_bits),
                animation: ItemUseAnimationSummary::Spear,
            }),
            consumable_sound: Some(SoundEventSummary {
                registry_id: Some(8),
                sound_id: None,
                fixed_range_bits: None,
            }),
            consumable_has_consume_particles: Some(false),
            consumable_effect_count: Some(5),
            consumable_effects: vec![
                ConsumeEffectSummary::ApplyEffects {
                    effects: vec![MobEffectInstanceSummary {
                        effect_id: 1,
                        amplifier: 255,
                        duration: -5,
                        ambient: true,
                        show_particles: false,
                        show_icon: true,
                        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                            amplifier: 0,
                            duration: 10,
                            ambient: false,
                            show_particles: true,
                            show_icon: false,
                            hidden_effect: None,
                        })),
                    }],
                    probability_bits: nan_bits,
                },
                ConsumeEffectSummary::RemoveEffects {
                    effect_registry_ids: Vec::new(),
                    effect_tag: Some("minecraft:beneficial".to_string()),
                },
                ConsumeEffectSummary::ClearAllEffects,
                ConsumeEffectSummary::TeleportRandomly {
                    diameter_bits: (-0.0_f32).to_bits(),
                },
                ConsumeEffectSummary::PlaySound {
                    sound: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("example:consume".to_string()),
                        fixed_range_bits: Some(f32::INFINITY.to_bits()),
                    },
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_var_i32(ItemUseAnimationSummary::Spear.id());
        expected.write_var_i32(9);
        expected.write_bool(false);
        expected.write_var_i32(5);
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID,
        );
        expected.write_var_i32(1);
        expected.write_var_i32(1);
        expected.write_var_i32(255);
        expected.write_var_i32(-5);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_var_i32(10);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(false);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
        );
        expected.write_var_i32(0);
        expected.write_string("minecraft:beneficial");
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID,
        );
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID,
        );
        expected.write_f32(-0.0);
        expected
            .write_var_i32(crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
        expected.write_var_i32(0);
        expected.write_string("example:consume");
        expected.write_bool(true);
        expected.write_f32(f32::INFINITY);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.consumable, summary.consumable);
        assert_eq!(decoded.consumable_sound, summary.consumable_sound);
        assert_eq!(
            decoded.consumable_has_consume_particles,
            summary.consumable_has_consume_particles
        );
        assert_eq!(
            decoded.consumable_effect_count,
            summary.consumable_effect_count
        );
        assert_eq!(decoded.consumable_effects, summary.consumable_effects);
    }

    #[test]
    fn death_protection_descriptor_round_trips_direct_holders_and_empty_lists() {
        let summary = DataComponentPatchSummary {
            death_protection_effects: vec![
                ConsumeEffectSummary::RemoveEffects {
                    effect_registry_ids: vec![0, 3],
                    effect_tag: None,
                },
                ConsumeEffectSummary::PlaySound {
                    sound: SoundEventSummary {
                        registry_id: Some(12),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_var_i32(2);
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
        );
        expected.write_var_i32(3);
        expected.write_var_i32(0);
        expected.write_var_i32(3);
        expected
            .write_var_i32(crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
        expected.write_var_i32(13);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.death_protection_effects,
            summary.death_protection_effects
        );

        let mut empty = Encoder::new();
        descriptor
            .encode(&DataComponentPatchSummary::default(), &mut empty)
            .unwrap();
        assert_eq!(empty.into_inner(), [0]);
    }

    #[test]
    fn consume_effect_descriptors_enforce_bounds_and_canonical_values_atomically() {
        fn mob_effect() -> MobEffectInstanceSummary {
            MobEffectInstanceSummary {
                effect_id: 0,
                amplifier: 0,
                duration: 20,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect: None,
            }
        }

        fn valid_consumable() -> DataComponentPatchSummary {
            DataComponentPatchSummary {
                consumable: Some(ConsumableSummary {
                    consume_seconds: 1.6,
                    animation: ItemUseAnimationSummary::Eat,
                }),
                consumable_sound: Some(SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                consumable_has_consume_particles: Some(true),
                consumable_effect_count: Some(1),
                consumable_effects: vec![ConsumeEffectSummary::ClearAllEffects],
                ..DataComponentPatchSummary::default()
            }
        }

        let consumable_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID).unwrap();
        let death_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID)
                .unwrap();
        let mut cases = Vec::new();
        cases.push((
            DataComponentPatchSummary::default(),
            "missing consumable component value",
        ));
        let mut missing_sound = valid_consumable();
        missing_sound.consumable_sound = None;
        cases.push((missing_sound, "missing sound holder"));
        let mut missing_particles = valid_consumable();
        missing_particles.consumable_has_consume_particles = None;
        cases.push((missing_particles, "missing particle flag"));
        let mut missing_count = valid_consumable();
        missing_count.consumable_effect_count = None;
        cases.push((missing_count, "missing effect count"));
        let mut stale_count = valid_consumable();
        stale_count.consumable_effect_count = Some(0);
        cases.push((stale_count, "does not match 1 effects"));
        let mut malformed_sound = valid_consumable();
        malformed_sound.consumable_sound = Some(SoundEventSummary {
            registry_id: Some(0),
            sound_id: Some("minecraft:entity.generic.eat".to_string()),
            fixed_range_bits: None,
        });
        cases.push((malformed_sound, "sound event holder projections"));
        let mut invalid_effect_id = valid_consumable();
        invalid_effect_id.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                effect_id: i32::try_from(crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len())
                    .unwrap(),
                ..mob_effect()
            }],
            probability_bits: 1.0_f32.to_bits(),
        }];
        cases.push((invalid_effect_id, "invalid vanilla mob effect holder"));
        let mut invalid_amplifier = valid_consumable();
        invalid_amplifier.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                amplifier: 256,
                ..mob_effect()
            }],
            probability_bits: 1.0_f32.to_bits(),
        }];
        cases.push((invalid_amplifier, "amplifier 256 is outside"));
        let mut hidden_effect = None;
        for _ in 0..=MAX_MOB_EFFECT_DETAILS_DEPTH {
            hidden_effect = Some(Box::new(MobEffectDetailsSummary {
                amplifier: 0,
                duration: 1,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect,
            }));
        }
        let mut excessive_depth = valid_consumable();
        excessive_depth.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                hidden_effect,
                ..mob_effect()
            }],
            probability_bits: 1.0_f32.to_bits(),
        }];
        cases.push((excessive_depth, "exceeded max depth"));
        let mut mixed_holder_set = valid_consumable();
        mixed_holder_set.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: vec![0],
            effect_tag: Some("minecraft:beneficial".to_string()),
        }];
        cases.push((mixed_holder_set, "both holder ids and a tag"));
        let mut noncanonical_tag = valid_consumable();
        noncanonical_tag.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: Vec::new(),
            effect_tag: Some("beneficial".to_string()),
        }];
        cases.push((noncanonical_tag, "noncanonical tag resource location"));
        let mut negative_holder = valid_consumable();
        negative_holder.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: vec![-1],
            effect_tag: None,
        }];
        cases.push((negative_holder, "invalid mob effect holder id -1"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = consumable_descriptor
                .encode(&summary, &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut maximal_outer = valid_consumable();
        maximal_outer.consumable_effect_count = Some(MAX_DATA_COMPONENT_LIST_ITEMS);
        maximal_outer.consumable_effects =
            vec![ConsumeEffectSummary::ClearAllEffects; MAX_DATA_COMPONENT_LIST_ITEMS];
        consumable_descriptor
            .encode(&maximal_outer, &mut Encoder::new())
            .unwrap();
        let mut maximal_apply = valid_consumable();
        maximal_apply.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![mob_effect(); MAX_DATA_COMPONENT_LIST_ITEMS],
            probability_bits: 1.0_f32.to_bits(),
        }];
        consumable_descriptor
            .encode(&maximal_apply, &mut Encoder::new())
            .unwrap();
        let mut maximal_holder_set = valid_consumable();
        maximal_holder_set.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS],
            effect_tag: None,
        }];
        consumable_descriptor
            .encode(&maximal_holder_set, &mut Encoder::new())
            .unwrap();

        for mut oversized in [
            {
                let mut value = valid_consumable();
                value.consumable_effect_count = Some(MAX_DATA_COMPONENT_LIST_ITEMS + 1);
                value.consumable_effects =
                    vec![ConsumeEffectSummary::ClearAllEffects; MAX_DATA_COMPONENT_LIST_ITEMS + 1];
                value
            },
            {
                let mut value = valid_consumable();
                value.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
                    effects: vec![mob_effect(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                    probability_bits: 1.0_f32.to_bits(),
                }];
                value
            },
            {
                let mut value = valid_consumable();
                value.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
                    effect_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                    effect_tag: None,
                }];
                value
            },
        ] {
            oversized.consumable_effect_count = Some(oversized.consumable_effects.len());
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(matches!(
                consumable_descriptor.encode(&oversized, &mut encoded),
                Err(ProtocolError::PacketTooLarge(actual, max))
                    if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                        && max == MAX_DATA_COMPONENT_LIST_ITEMS
            ));
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized_death = DataComponentPatchSummary {
            death_protection_effects: vec![
                ConsumeEffectSummary::ClearAllEffects;
                MAX_DATA_COMPONENT_LIST_ITEMS + 1
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            death_descriptor.encode(&oversized_death, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn weapon_interaction_descriptors_round_trip_exact_wire_values() {
        let nan_bits = 0x7fc0_1234;
        let piercing = PiercingWeaponSummary {
            deals_knockback: false,
            dismounts: true,
            sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:pierce".to_string()),
                fixed_range_bits: Some(nan_bits),
            }),
            hit_sound: Some(SoundEventSummary {
                registry_id: Some(7),
                sound_id: None,
                fixed_range_bits: None,
            }),
        };
        let kinetic = KineticWeaponSummary {
            contact_cooldown_ticks: -1,
            delay_ticks: -2,
            dismount_conditions: Some(KineticWeaponConditionSummary {
                max_duration_ticks: -3,
                min_speed_bits: (-0.0_f32).to_bits(),
                min_relative_speed_bits: f32::INFINITY.to_bits(),
            }),
            knockback_conditions: None,
            damage_conditions: Some(KineticWeaponConditionSummary {
                max_duration_ticks: 4,
                min_speed_bits: nan_bits,
                min_relative_speed_bits: 0.25_f32.to_bits(),
            }),
            forward_movement_bits: nan_bits,
            damage_multiplier_bits: (-0.0_f32).to_bits(),
            sound: None,
            hit_sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:hit".to_string()),
                fixed_range_bits: None,
            }),
        };
        let swing = SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::Stab,
            duration: -4,
        };
        let summary = DataComponentPatchSummary {
            piercing_weapon: Some(piercing.clone()),
            kinetic_weapon: Some(kinetic.clone()),
            swing_animation: Some(swing),
            ..DataComponentPatchSummary::default()
        };

        let piercing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID)
                .unwrap();
        let kinetic_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
                .unwrap();
        let swing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID)
                .unwrap();

        let mut piercing_bytes = Encoder::new();
        piercing_descriptor
            .encode(&summary, &mut piercing_bytes)
            .unwrap();
        let piercing_bytes = piercing_bytes.into_inner();
        let mut expected = Encoder::new();
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("example:pierce");
        expected.write_bool(true);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_bool(true);
        expected.write_var_i32(8);
        assert_eq!(piercing_bytes, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&piercing_bytes);
        piercing_descriptor
            .decode(&mut decoder, &mut decoded)
            .unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.piercing_weapon, Some(piercing));

        let mut kinetic_bytes = Encoder::new();
        kinetic_descriptor
            .encode(&summary, &mut kinetic_bytes)
            .unwrap();
        let kinetic_bytes = kinetic_bytes.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(-1);
        expected.write_var_i32(-2);
        expected.write_bool(true);
        expected.write_var_i32(-3);
        expected.write_f32(-0.0);
        expected.write_f32(f32::INFINITY);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_var_i32(4);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_f32(0.25);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_f32(-0.0);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("example:hit");
        expected.write_bool(false);
        assert_eq!(kinetic_bytes, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&kinetic_bytes);
        kinetic_descriptor
            .decode(&mut decoder, &mut decoded)
            .unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.kinetic_weapon, Some(kinetic));

        let mut swing_bytes = Encoder::new();
        swing_descriptor.encode(&summary, &mut swing_bytes).unwrap();
        let swing_bytes = swing_bytes.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(SwingAnimationTypeSummary::Stab.id());
        expected.write_var_i32(-4);
        assert_eq!(swing_bytes, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&swing_bytes);
        swing_descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.swing_animation, Some(swing));
    }

    #[test]
    fn weapon_interaction_descriptors_reject_noncanonical_summaries_atomically() {
        let piercing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID)
                .unwrap();
        let kinetic_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
                .unwrap();
        let swing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID)
                .unwrap();

        for (descriptor, expected_error) in [
            (piercing_descriptor, "missing piercing_weapon"),
            (kinetic_descriptor, "missing kinetic_weapon"),
            (swing_descriptor, "missing swing_animation"),
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor
                .encode(&DataComponentPatchSummary::default(), &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let malformed_piercing = DataComponentPatchSummary {
            piercing_weapon: Some(PiercingWeaponSummary {
                deals_knockback: true,
                dismounts: false,
                sound: Some(SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: Some("minecraft:item.spear.attack".to_string()),
                    fixed_range_bits: None,
                }),
                hit_sound: None,
            }),
            ..DataComponentPatchSummary::default()
        };
        let malformed_kinetic = DataComponentPatchSummary {
            kinetic_weapon: Some(KineticWeaponSummary {
                contact_cooldown_ticks: 10,
                delay_ticks: 0,
                dismount_conditions: None,
                knockback_conditions: None,
                damage_conditions: None,
                forward_movement_bits: 0.0_f32.to_bits(),
                damage_multiplier_bits: 1.0_f32.to_bits(),
                sound: None,
                hit_sound: Some(SoundEventSummary {
                    registry_id: Some(-1),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
            }),
            ..DataComponentPatchSummary::default()
        };
        for (descriptor, summary) in [
            (piercing_descriptor, &malformed_piercing),
            (kinetic_descriptor, &malformed_kinetic),
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(summary, &mut encoded).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("sound event holder projections are not canonical"),
                "{error}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn blocks_attacks_descriptor_round_trips_holder_and_sound_unions_in_wire_order() {
        let nan_bits = 0x7fc0_1234;
        let summary = DataComponentPatchSummary {
            blocks_attacks: Some(BlocksAttacksSummary {
                block_delay_seconds_bits: (-0.0_f32).to_bits(),
                disable_cooldown_scale_bits: f32::INFINITY.to_bits(),
                damage_reductions: vec![
                    DamageReductionSummary {
                        horizontal_blocking_angle_bits: 45.0_f32.to_bits(),
                        damage_types: Some(DamageTypeHolderSetSummary {
                            registry_ids: Vec::new(),
                            tag: Some("minecraft:bypasses_shield".to_string()),
                        }),
                        base_bits: nan_bits,
                        factor_bits: (-1.25_f32).to_bits(),
                    },
                    DamageReductionSummary {
                        horizontal_blocking_angle_bits: 180.0_f32.to_bits(),
                        damage_types: Some(DamageTypeHolderSetSummary {
                            registry_ids: vec![0, 47],
                            tag: None,
                        }),
                        base_bits: 2.0_f32.to_bits(),
                        factor_bits: 0.5_f32.to_bits(),
                    },
                ],
                item_damage: ItemDamageFunctionSummary {
                    threshold_bits: (-2.0_f32).to_bits(),
                    base_bits: 3.0_f32.to_bits(),
                    factor_bits: nan_bits,
                },
                bypassed_by: Some(DamageTypeHolderSetSummary {
                    registry_ids: Vec::new(),
                    tag: None,
                }),
                block_sound: Some(SoundEventSummary {
                    registry_id: Some(11),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                disable_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:shield_disable".to_string()),
                    fixed_range_bits: Some((-0.0_f32).to_bits()),
                }),
            }),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_f32(-0.0);
        expected.write_f32(f32::INFINITY);
        expected.write_var_i32(2);
        expected.write_f32(45.0);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("minecraft:bypasses_shield");
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_f32(-1.25);
        expected.write_f32(180.0);
        expected.write_bool(true);
        expected.write_var_i32(3);
        expected.write_var_i32(0);
        expected.write_var_i32(47);
        expected.write_f32(2.0);
        expected.write_f32(0.5);
        expected.write_f32(-2.0);
        expected.write_f32(3.0);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_bool(true);
        expected.write_var_i32(1);
        expected.write_bool(true);
        expected.write_var_i32(12);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("example:shield_disable");
        expected.write_bool(true);
        expected.write_f32(-0.0);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.blocks_attacks, summary.blocks_attacks);
    }

    #[test]
    fn blocks_attacks_descriptor_enforces_bounds_and_canonical_unions_atomically() {
        let reduction = DamageReductionSummary {
            horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
            damage_types: None,
            base_bits: 0.0_f32.to_bits(),
            factor_bits: 1.0_f32.to_bits(),
        };
        let valid = DataComponentPatchSummary {
            blocks_attacks: Some(BlocksAttacksSummary {
                block_delay_seconds_bits: 0.0_f32.to_bits(),
                disable_cooldown_scale_bits: 1.0_f32.to_bits(),
                damage_reductions: vec![reduction.clone()],
                item_damage: ItemDamageFunctionSummary {
                    threshold_bits: 1.0_f32.to_bits(),
                    base_bits: 0.0_f32.to_bits(),
                    factor_bits: 1.0_f32.to_bits(),
                },
                bypassed_by: None,
                block_sound: None,
                disable_sound: None,
            }),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
                .unwrap();

        let mut empty = valid.clone();
        empty
            .blocks_attacks
            .as_mut()
            .unwrap()
            .damage_reductions
            .clear();
        descriptor.encode(&empty, &mut Encoder::new()).unwrap();
        let mut maximal = valid.clone();
        maximal.blocks_attacks.as_mut().unwrap().damage_reductions =
            vec![reduction; MAX_DATA_COMPONENT_LIST_ITEMS];
        descriptor.encode(&maximal, &mut Encoder::new()).unwrap();
        let mut maximal_holder_set = valid.clone();
        maximal_holder_set
            .blocks_attacks
            .as_mut()
            .unwrap()
            .bypassed_by = Some(DamageTypeHolderSetSummary {
            registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS],
            tag: None,
        });
        descriptor
            .encode(&maximal_holder_set, &mut Encoder::new())
            .unwrap();

        let mut cases = Vec::new();
        cases.push((
            DataComponentPatchSummary::default(),
            "missing blocks_attacks",
        ));
        let mut holder_union = valid.clone();
        holder_union.blocks_attacks.as_mut().unwrap().bypassed_by =
            Some(DamageTypeHolderSetSummary {
                registry_ids: vec![0],
                tag: Some("minecraft:bypasses_shield".to_string()),
            });
        cases.push((holder_union, "both holder ids and a tag"));
        let mut negative_holder = valid.clone();
        negative_holder
            .blocks_attacks
            .as_mut()
            .unwrap()
            .damage_reductions[0]
            .damage_types = Some(DamageTypeHolderSetSummary {
            registry_ids: vec![-1],
            tag: None,
        });
        cases.push((negative_holder, "invalid damage type holder id -1"));
        let mut noncanonical_tag = valid.clone();
        noncanonical_tag
            .blocks_attacks
            .as_mut()
            .unwrap()
            .bypassed_by = Some(DamageTypeHolderSetSummary {
            registry_ids: Vec::new(),
            tag: Some("bypasses_shield".to_string()),
        });
        cases.push((noncanonical_tag, "noncanonical tag resource location"));
        let mut mixed_sound = valid.clone();
        mixed_sound.blocks_attacks.as_mut().unwrap().block_sound = Some(SoundEventSummary {
            registry_id: Some(0),
            sound_id: Some("minecraft:item.shield.block".to_string()),
            fixed_range_bits: None,
        });
        cases.push((mixed_sound, "sound event holder projections"));
        let mut missing_sound = valid.clone();
        missing_sound.blocks_attacks.as_mut().unwrap().block_sound = Some(SoundEventSummary {
            registry_id: None,
            sound_id: None,
            fixed_range_bits: None,
        });
        cases.push((missing_sound, "sound event holder projections"));
        let mut invalid_sound_id = valid.clone();
        invalid_sound_id
            .blocks_attacks
            .as_mut()
            .unwrap()
            .block_sound = Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("Invalid Path".to_string()),
            fixed_range_bits: None,
        });
        cases.push((invalid_sound_id, "invalid resource"));
        let mut noncanonical_sound_id = valid.clone();
        noncanonical_sound_id
            .blocks_attacks
            .as_mut()
            .unwrap()
            .block_sound = Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("item.shield.block".to_string()),
            fixed_range_bits: None,
        });
        cases.push((noncanonical_sound_id, "noncanonical resource location"));
        let mut overflowing_sound_holder = valid.clone();
        overflowing_sound_holder
            .blocks_attacks
            .as_mut()
            .unwrap()
            .block_sound = Some(SoundEventSummary {
            registry_id: Some(i32::MAX),
            sound_id: None,
            fixed_range_bits: None,
        });
        cases.push((overflowing_sound_holder, "sound event holder projections"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut oversized_holder_set = valid.clone();
        oversized_holder_set
            .blocks_attacks
            .as_mut()
            .unwrap()
            .bypassed_by = Some(DamageTypeHolderSetSummary {
            registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            tag: None,
        });
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized_holder_set, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);

        let mut oversized = valid;
        oversized.blocks_attacks.as_mut().unwrap().damage_reductions = vec![
            DamageReductionSummary {
                horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                damage_types: None,
                base_bits: 0.0_f32.to_bits(),
                factor_bits: 1.0_f32.to_bits(),
            };
            MAX_DATA_COMPONENT_LIST_ITEMS
                + 1
        ];
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    fn direct_instrument_patch() -> DataComponentPatchSummary {
        let styled = vec![StyledTextRun {
            text: "Custom horn".to_string(),
            style: Default::default(),
        }];
        DataComponentPatchSummary {
            instrument_description: Some("Custom horn".to_string()),
            instrument_description_styled: Some(styled.clone()),
            instrument_direct: Some(Box::new(InstrumentSummary {
                sound_event: SoundEventSummary {
                    registry_id: Some(7),
                    sound_id: None,
                    fixed_range_bits: None,
                },
                use_duration_bits: 3.25_f32.to_bits(),
                range_bits: 96.0_f32.to_bits(),
                description: "Custom horn".to_string(),
                description_styled: Some(styled.into_boxed_slice()),
                description_nbt: Some(Box::new(NbtSummaryValue::String("Custom horn".to_string()))),
            })),
            ..DataComponentPatchSummary::default()
        }
    }

    #[test]
    fn instrument_descriptor_round_trips_reference_and_direct_holders() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID).unwrap();
        for holder_id in [0, 300] {
            let summary = DataComponentPatchSummary {
                instrument_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id + 1);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.instrument_id, Some(holder_id));
            assert!(decoded.instrument_direct.is_none());
        }

        let direct = direct_instrument_patch();
        let mut encoded = Encoder::new();
        descriptor.encode(&direct, &mut encoded).unwrap();
        assert_eq!(
            encoded.clone().into_inner(),
            [
                0, // direct instrument holder
                8, // sound event registry holder id 7 + 1
                0x40, 0x50, 0, 0, // use_duration = 3.25
                0x42, 0xc0, 0, 0, // range = 96.0
                8, 0, 11, b'C', b'u', b's', b't', b'o', b'm', b' ', b'h', b'o', b'r', b'n',
            ]
        );

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, direct);

        let mut boundary = direct_instrument_patch();
        let instrument = boundary.instrument_direct.as_mut().unwrap();
        instrument.sound_event = SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:signal".to_string()),
            fixed_range_bits: Some((-0.0_f32).to_bits()),
        };
        instrument.use_duration_bits = f32::MAX.to_bits();
        instrument.range_bits = f32::from_bits(1).to_bits();
        let mut encoded = Encoder::new();
        descriptor.encode(&boundary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, boundary);
    }

    #[test]
    fn instrument_descriptor_rejects_noncanonical_or_persistently_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID).unwrap();
        let valid = direct_instrument_patch();
        let mut cases = vec![
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                instrument_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                instrument_id: Some(i32::MAX),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                instrument_id: Some(0),
                instrument_description: Some("dangling".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                instrument_id: Some(0),
                ..valid.clone()
            },
        ];
        for field in ["use_duration", "range"] {
            for invalid_bits in [
                0.0_f32.to_bits(),
                (-0.0_f32).to_bits(),
                (-1.0_f32).to_bits(),
                f32::INFINITY.to_bits(),
                f32::NAN.to_bits(),
            ] {
                let mut invalid = valid.clone();
                let instrument = invalid.instrument_direct.as_mut().unwrap();
                match field {
                    "use_duration" => instrument.use_duration_bits = invalid_bits,
                    "range" => instrument.range_bits = invalid_bits,
                    _ => unreachable!(),
                }
                cases.push(invalid);
            }
        }
        let mut missing_tooltip = valid.clone();
        missing_tooltip.instrument_description = None;
        cases.push(missing_tooltip);
        let mut mismatched_tooltip = valid.clone();
        mismatched_tooltip.instrument_description = Some("Stale".to_string());
        cases.push(mismatched_tooltip);
        let mut mismatched_style = valid.clone();
        mismatched_style
            .instrument_description_styled
            .as_mut()
            .unwrap()[0]
            .style
            .bold = Some(true);
        cases.push(mismatched_style);
        let mut mixed_sound = valid.clone();
        mixed_sound
            .instrument_direct
            .as_mut()
            .unwrap()
            .sound_event
            .sound_id = Some("example:mixed".to_string());
        cases.push(mixed_sound);
        let mut missing_component = valid;
        missing_component
            .instrument_direct
            .as_mut()
            .unwrap()
            .description_nbt = None;
        cases.push(missing_component);

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    fn banner_pattern_layers() -> Vec<BannerPatternLayerSummary> {
        vec![
            BannerPatternLayerSummary {
                registry_id: Some(4),
                asset_id: None,
                translation_key: None,
                color_id: VANILLA_DYE_COLOR_RED_ID,
            },
            BannerPatternLayerSummary {
                registry_id: None,
                asset_id: Some("minecraft:stripe_bottom".to_string()),
                translation_key: Some("block.minecraft.banner.stripe_bottom".to_string()),
                color_id: VANILLA_DYE_COLOR_BLUE_ID,
            },
        ]
    }

    #[test]
    fn banner_patterns_descriptor_round_trips_reference_and_direct_layers_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID)
                .unwrap();
        let empty = DataComponentPatchSummary::default();
        let mut encoded = Encoder::new();
        descriptor.encode(&empty, &mut encoded).unwrap();
        assert_eq!(encoded.clone().into_inner(), [0]);
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, empty);

        let summary = DataComponentPatchSummary {
            banner_pattern_layers: banner_pattern_layers(),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(2);
        expected.write_var_i32(5);
        expected.write_var_i32(VANILLA_DYE_COLOR_RED_ID);
        expected.write_var_i32(0);
        expected.write_string("minecraft:stripe_bottom");
        expected.write_string("block.minecraft.banner.stripe_bottom");
        expected.write_var_i32(VANILLA_DYE_COLOR_BLUE_ID);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);

        let maximal_translation =
            format!("{}a", "😀".repeat((MAX_STRING_CHARS.saturating_sub(1)) / 2));
        assert_eq!(maximal_translation.encode_utf16().count(), MAX_STRING_CHARS);
        let boundary = DataComponentPatchSummary {
            banner_pattern_layers: vec![
                BannerPatternLayerSummary {
                    registry_id: Some(i32::MAX - 1),
                    color_id: VANILLA_DYE_COLOR_RED_ID,
                    ..BannerPatternLayerSummary::default()
                },
                BannerPatternLayerSummary {
                    asset_id: Some("example:signal".to_string()),
                    translation_key: Some(maximal_translation),
                    color_id: VANILLA_DYE_COLOR_BLUE_ID,
                    ..BannerPatternLayerSummary::default()
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&boundary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, boundary);

        let maximal = DataComponentPatchSummary {
            banner_pattern_layers: vec![
                BannerPatternLayerSummary {
                    registry_id: Some(0),
                    color_id: VANILLA_DYE_COLOR_RED_ID,
                    ..BannerPatternLayerSummary::default()
                };
                MAX_DATA_COMPONENT_LIST_ITEMS
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&maximal, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, maximal);
    }

    #[test]
    fn banner_patterns_descriptor_rejects_noncanonical_layers_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID)
                .unwrap();
        let valid = DataComponentPatchSummary {
            banner_pattern_layers: banner_pattern_layers(),
            ..DataComponentPatchSummary::default()
        };
        let mut cases = Vec::new();
        for invalid_layer in [
            BannerPatternLayerSummary {
                registry_id: Some(-1),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                registry_id: Some(i32::MAX),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                registry_id: Some(0),
                asset_id: Some("example:mixed".to_string()),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                asset_id: Some("example:missing_translation".to_string()),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                asset_id: Some("Bad:asset".to_string()),
                translation_key: Some("block.example.bad".to_string()),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                asset_id: Some("example:oversized".to_string()),
                translation_key: Some("x".repeat(MAX_STRING_CHARS + 1)),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                registry_id: Some(0),
                color_id: -1,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                registry_id: Some(0),
                color_id: 16,
                ..BannerPatternLayerSummary::default()
            },
        ] {
            cases.push(DataComponentPatchSummary {
                banner_pattern_layers: vec![banner_pattern_layers()[0].clone(), invalid_layer],
                ..DataComponentPatchSummary::default()
            });
        }

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized = DataComponentPatchSummary {
            banner_pattern_layers: vec![
                banner_pattern_layers()[0].clone();
                MAX_DATA_COMPONENT_LIST_ITEMS + 1
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(count, max))
                if count == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);

        descriptor.encode(&valid, &mut Encoder::new()).unwrap();
    }

    #[test]
    fn lodestone_tracker_descriptor_round_trips_optional_global_position_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
                .unwrap();
        let untargeted = DataComponentPatchSummary {
            lodestone_tracked: Some(true),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&untargeted, &mut encoded).unwrap();
        assert_eq!(encoded.clone().into_inner(), [0, 1]);
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, untargeted);

        let targeted = DataComponentPatchSummary {
            lodestone_target: Some(LodestoneTargetSummary {
                dimension: "minecraft:the_nether".to_string(),
                pos: chunks::BlockPos {
                    x: 12,
                    y: -64,
                    z: 34,
                },
            }),
            lodestone_tracked: Some(false),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&targeted, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_bool(true);
        expected.write_string("minecraft:the_nether");
        expected.write_i64(chunks::encode_block_pos(chunks::BlockPos {
            x: 12,
            y: -64,
            z: 34,
        }));
        expected.write_bool(false);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, targeted);

        for pos in [
            chunks::BlockPos {
                x: -33_554_432,
                y: -2_048,
                z: -33_554_432,
            },
            chunks::BlockPos {
                x: 33_554_431,
                y: 2_047,
                z: 33_554_431,
            },
        ] {
            let boundary = DataComponentPatchSummary {
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos,
                }),
                lodestone_tracked: Some(true),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&boundary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, boundary);
        }
    }

    #[test]
    fn lodestone_tracker_descriptor_rejects_noncanonical_projections_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
                .unwrap();
        let mut cases = vec![DataComponentPatchSummary::default()];
        for dimension in [
            "overworld".to_string(),
            "Bad:dimension".to_string(),
            format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1)),
        ] {
            cases.push(DataComponentPatchSummary {
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension,
                    pos: chunks::BlockPos { x: 0, y: 0, z: 0 },
                }),
                lodestone_tracked: Some(true),
                ..DataComponentPatchSummary::default()
            });
        }
        for pos in [
            chunks::BlockPos {
                x: -33_554_433,
                y: 0,
                z: 0,
            },
            chunks::BlockPos {
                x: 33_554_432,
                y: 0,
                z: 0,
            },
            chunks::BlockPos {
                x: 0,
                y: -2_049,
                z: 0,
            },
            chunks::BlockPos {
                x: 0,
                y: 2_048,
                z: 0,
            },
            chunks::BlockPos {
                x: 0,
                y: 0,
                z: -33_554_433,
            },
            chunks::BlockPos {
                x: 0,
                y: 0,
                z: 33_554_432,
            },
        ] {
            cases.push(DataComponentPatchSummary {
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos,
                }),
                lodestone_tracked: Some(false),
                ..DataComponentPatchSummary::default()
            });
        }

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    fn direct_painting_variant() -> PaintingVariantSummary {
        PaintingVariantSummary {
            width: 4,
            height: 2,
            asset_id: "minecraft:gallery".to_string(),
            title: Some("Gallery".to_string()),
            title_styled: Some(vec![StyledTextRun {
                text: "Gallery".to_string(),
                style: Default::default(),
            }]),
            title_nbt: Some(Box::new(NbtSummaryValue::String("Gallery".to_string()))),
            author: None,
            author_styled: None,
            author_nbt: None,
        }
    }

    #[test]
    fn painting_variant_descriptor_round_trips_reference_and_direct_holders_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, i32::MAX - 1] {
            let reference = DataComponentPatchSummary {
                painting_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&reference, &mut encoded).unwrap();
            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id + 1);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, reference);
        }

        let direct = DataComponentPatchSummary {
            painting_variant_direct: Some(direct_painting_variant()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&direct, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(0);
        expected.write_var_i32(4);
        expected.write_var_i32(2);
        expected.write_string("minecraft:gallery");
        expected.write_bool(true);
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                Some(&NbtSummaryValue::String("Gallery".to_string())),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        expected.write_bool(false);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, direct);

        for (width, height) in [(1, 16), (16, 1)] {
            let boundary = DataComponentPatchSummary {
                painting_variant_direct: Some(PaintingVariantSummary {
                    width,
                    height,
                    ..direct_painting_variant()
                }),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&boundary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, boundary);
        }
    }

    #[test]
    fn painting_variant_descriptor_rejects_noncanonical_holders_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID)
                .unwrap();
        let direct = direct_painting_variant();
        let mut cases = vec![
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                painting_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                painting_variant_id: Some(i32::MAX),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                painting_variant_id: Some(0),
                painting_variant_direct: Some(direct.clone()),
                ..DataComponentPatchSummary::default()
            },
        ];
        for invalid in [
            PaintingVariantSummary {
                width: 0,
                ..direct.clone()
            },
            PaintingVariantSummary {
                width: 17,
                ..direct.clone()
            },
            PaintingVariantSummary {
                height: 0,
                ..direct.clone()
            },
            PaintingVariantSummary {
                height: 17,
                ..direct.clone()
            },
            PaintingVariantSummary {
                asset_id: "gallery".to_string(),
                ..direct.clone()
            },
            PaintingVariantSummary {
                asset_id: "Bad:gallery".to_string(),
                ..direct.clone()
            },
            PaintingVariantSummary {
                asset_id: format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1)),
                ..direct.clone()
            },
            PaintingVariantSummary {
                title: Some("Stale".to_string()),
                ..direct.clone()
            },
            PaintingVariantSummary {
                title_styled: None,
                ..direct.clone()
            },
            PaintingVariantSummary {
                title_nbt: None,
                ..direct.clone()
            },
            PaintingVariantSummary {
                author_nbt: Some(Box::new(NbtSummaryValue::String("Dangling".to_string()))),
                ..direct.clone()
            },
        ] {
            cases.push(DataComponentPatchSummary {
                painting_variant_direct: Some(invalid),
                ..DataComponentPatchSummary::default()
            });
        }

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    fn bee_occupant() -> BeeOccupantSummary {
        BeeOccupantSummary {
            entity_type_id: VANILLA_ENTITY_TYPE_BEE_ID,
            entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
            ticks_in_hive: i32::MIN,
            min_ticks_in_hive: i32::MAX,
        }
    }

    #[test]
    fn bees_descriptor_round_trips_empty_and_populated_lists_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BEES_TYPE_ID).unwrap();
        let empty = DataComponentPatchSummary {
            bees_present: true,
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&empty, &mut encoded).unwrap();
        assert_eq!(encoded.clone().into_inner(), [0]);
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, empty);

        let summary = DataComponentPatchSummary {
            bees_present: true,
            bees_count: 1,
            bees: vec![bee_occupant()],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(1);
        expected.write_var_i32(VANILLA_ENTITY_TYPE_BEE_ID);
        expected.write_bytes(&[10, 0]);
        expected.write_var_i32(i32::MIN);
        expected.write_var_i32(i32::MAX);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);
    }

    #[test]
    fn bees_descriptor_rejects_noncanonical_nested_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BEES_TYPE_ID).unwrap();
        let valid = DataComponentPatchSummary {
            bees_present: true,
            bees_count: 1,
            bees: vec![bee_occupant()],
            ..DataComponentPatchSummary::default()
        };
        let mut cases = vec![
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                bees_present: true,
                bees_count: 1,
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                bees_count: 1,
                bees: vec![bee_occupant()],
                ..DataComponentPatchSummary::default()
            },
        ];
        for invalid_bee in [
            BeeOccupantSummary {
                entity_type_id: -1,
                ..bee_occupant()
            },
            BeeOccupantSummary {
                entity_type_id: VANILLA_ENTITY_TYPES.len() as i32,
                ..bee_occupant()
            },
            BeeOccupantSummary {
                entity_data_tag: None,
                ..bee_occupant()
            },
            BeeOccupantSummary {
                entity_data_tag: Some(NbtSummaryValue::String("not a compound".to_string())),
                ..bee_occupant()
            },
            BeeOccupantSummary {
                entity_data_tag: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "id".to_string(),
                    value: NbtSummaryValue::String("minecraft:bee".to_string()),
                }])),
                ..bee_occupant()
            },
        ] {
            cases.push(DataComponentPatchSummary {
                bees_present: true,
                bees_count: 2,
                bees: vec![bee_occupant(), invalid_bee],
                ..DataComponentPatchSummary::default()
            });
        }

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized = DataComponentPatchSummary {
            bees_present: true,
            bees_count: MAX_DATA_COMPONENT_LIST_ITEMS + 1,
            bees: vec![bee_occupant(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(count, max))
                if count == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);

        let mut encoded = Encoder::new();
        descriptor.encode(&valid, &mut encoded).unwrap();
        assert!(!encoded.into_inner().is_empty());

        let maximal = DataComponentPatchSummary {
            bees_present: true,
            bees_count: MAX_DATA_COMPONENT_LIST_ITEMS,
            bees: vec![bee_occupant(); MAX_DATA_COMPONENT_LIST_ITEMS],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&maximal, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, maximal);
    }

    fn direct_jukebox_playable_patch() -> DataComponentPatchSummary {
        DataComponentPatchSummary {
            jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
                sound_event: SoundEventSummary {
                    registry_id: Some(7),
                    sound_id: None,
                    fixed_range_bits: None,
                },
                description: "Custom song".to_string(),
                description_styled: Some(
                    vec![StyledTextRun {
                        text: "Custom song".to_string(),
                        style: Default::default(),
                    }]
                    .into_boxed_slice(),
                ),
                description_nbt: Some(Box::new(NbtSummaryValue::String("Custom song".to_string()))),
                length_in_seconds_bits: 60.0_f32.to_bits(),
                comparator_output: 7,
            })),
            ..DataComponentPatchSummary::default()
        }
    }

    #[test]
    fn jukebox_playable_descriptor_round_trips_reference_and_direct_song_holders() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID)
                .unwrap();
        for holder_id in [0, 300] {
            let summary = DataComponentPatchSummary {
                jukebox_song_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id + 1);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.jukebox_song_id, Some(holder_id));
            assert!(decoded.jukebox_direct_song.is_none());
        }

        let direct = direct_jukebox_playable_patch();
        let mut encoded = Encoder::new();
        descriptor.encode(&direct, &mut encoded).unwrap();
        assert_eq!(
            encoded.clone().into_inner(),
            [
                0, // direct jukebox song holder
                8, // sound event registry holder id 7 + 1
                8, 0, 11, b'C', b'u', b's', b't', b'o', b'm', b' ', b's', b'o', b'n', b'g', 0x42,
                0x70, 0, 0, // length_in_seconds = 60.0
                7, // comparator_output
            ]
        );

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, direct);

        let mut boundary = direct_jukebox_playable_patch();
        let song = boundary.jukebox_direct_song.as_mut().unwrap();
        song.sound_event = SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:music.custom".to_string()),
            fixed_range_bits: Some((-0.0_f32).to_bits()),
        };
        song.length_in_seconds_bits = f32::MAX.to_bits();
        song.comparator_output = 15;
        let mut encoded = Encoder::new();
        descriptor.encode(&boundary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, boundary);
    }

    #[test]
    fn jukebox_playable_descriptor_rejects_noncanonical_or_persistently_invalid_values_atomically()
    {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID)
                .unwrap();
        let valid = direct_jukebox_playable_patch();
        let mut cases = vec![
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                jukebox_song_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                jukebox_song_id: Some(i32::MAX),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                jukebox_song_id: Some(0),
                ..valid.clone()
            },
        ];
        for invalid_length_bits in [
            0.0_f32.to_bits(),
            (-0.0_f32).to_bits(),
            (-1.0_f32).to_bits(),
            f32::INFINITY.to_bits(),
            f32::NAN.to_bits(),
        ] {
            let mut invalid = valid.clone();
            invalid
                .jukebox_direct_song
                .as_mut()
                .unwrap()
                .length_in_seconds_bits = invalid_length_bits;
            cases.push(invalid);
        }
        for comparator_output in [-1, 16] {
            let mut invalid = valid.clone();
            invalid
                .jukebox_direct_song
                .as_mut()
                .unwrap()
                .comparator_output = comparator_output;
            cases.push(invalid);
        }
        let mut mixed_sound = valid.clone();
        mixed_sound
            .jukebox_direct_song
            .as_mut()
            .unwrap()
            .sound_event
            .sound_id = Some("example:mixed".to_string());
        cases.push(mixed_sound);
        let mut missing_component = valid;
        missing_component
            .jukebox_direct_song
            .as_mut()
            .unwrap()
            .description_nbt = None;
        cases.push(missing_component);

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    fn canonical_trim_description(nbt: &NbtSummaryValue) -> (String, Box<[StyledTextRun]>) {
        let encoded =
            encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        let (runs, canonical_nbt) =
            decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(&encoded))
                .unwrap();
        assert_eq!(canonical_nbt.as_ref(), Some(nbt));
        (styled_runs_summary_text(&runs), runs.into_boxed_slice())
    }

    fn direct_trim_patch() -> DataComponentPatchSummary {
        let material_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Ruby".to_string()),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("red".to_string()),
            },
        ]);
        let pattern_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Spiral".to_string()),
            },
            NbtSummaryEntry {
                name: "italic".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        let (material_description, material_styled) = canonical_trim_description(&material_nbt);
        let (pattern_description, pattern_styled) = canonical_trim_description(&pattern_nbt);
        DataComponentPatchSummary {
            armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                asset_name: "ruby".to_string(),
                override_armor_assets: BTreeMap::from([
                    ("minecraft:netherite".to_string(), "ruby_darker".to_string()),
                    ("example:opal".to_string(), "ruby_opal".to_string()),
                ]),
                description: material_description,
                description_styled: Some(material_styled),
                description_nbt: Some(Box::new(material_nbt)),
            })),
            armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                asset_id: "example:spiral".to_string(),
                description: pattern_description,
                description_styled: Some(pattern_styled),
                description_nbt: Some(Box::new(pattern_nbt)),
                decal: true,
            })),
            ..DataComponentPatchSummary::default()
        }
    }

    #[test]
    fn break_sound_descriptor_round_trips_reference_and_direct_holders_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID).unwrap();
        for holder_id in [0, i32::MAX - 1] {
            let referenced = DataComponentPatchSummary {
                break_sound: Some(SoundEventSummary {
                    registry_id: Some(holder_id),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&referenced, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id + 1);
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, referenced);
        }

        for fixed_range_bits in [None, Some(12.5_f32.to_bits())] {
            let direct = DataComponentPatchSummary {
                break_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:item.custom_break".to_string()),
                    fixed_range_bits,
                }),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&direct, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_var_i32(0);
            expected.write_string("example:item.custom_break");
            expected.write_bool(fixed_range_bits.is_some());
            if let Some(bits) = fixed_range_bits {
                expected.write_f32(f32::from_bits(bits));
            }
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, direct);
        }
    }

    #[test]
    fn break_sound_descriptor_rejects_noncanonical_holders_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID).unwrap();
        let cases = [
            None,
            Some(SoundEventSummary {
                registry_id: Some(-1),
                sound_id: None,
                fixed_range_bits: None,
            }),
            Some(SoundEventSummary {
                registry_id: Some(i32::MAX),
                sound_id: None,
                fixed_range_bits: None,
            }),
            Some(SoundEventSummary {
                registry_id: Some(0),
                sound_id: Some("example:item.custom_break".to_string()),
                fixed_range_bits: None,
            }),
            Some(SoundEventSummary {
                registry_id: Some(0),
                sound_id: None,
                fixed_range_bits: Some(12.5_f32.to_bits()),
            }),
            Some(SoundEventSummary {
                registry_id: None,
                sound_id: None,
                fixed_range_bits: Some(12.5_f32.to_bits()),
            }),
            Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("item.custom_break".to_string()),
                fixed_range_bits: None,
            }),
            Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("Bad:custom_break".to_string()),
                fixed_range_bits: None,
            }),
            Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some(format!("example:{}", "a".repeat(MAX_STRING_CHARS + 1))),
                fixed_range_bits: None,
            }),
        ];
        for break_sound in cases {
            let summary = DataComponentPatchSummary {
                break_sound,
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn map_id_descriptor_round_trips_the_full_signed_domain_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID).unwrap();
        for map_id in [i32::MIN, -1, 0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                map_id: Some(map_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_var_i32(map_id);
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, summary);
        }
    }

    #[test]
    fn map_id_descriptor_rejects_a_missing_value_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn map_decorations_descriptor_round_trips_all_builtin_holders_and_canonicalizes_numeric_nbt() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID)
                .unwrap();
        let decorations = vanilla_map_decoration_types()
            .enumerate()
            .map(|(index, decoration_type)| ItemMapDecorationSummary {
                id: format!("marker {index}"),
                decoration_type_id: decoration_type.type_id,
                decoration_type_resource_id: decoration_type.resource_id.to_string(),
                x_bits: (index as f64 - 12.5).to_bits(),
                z_bits: (64.0 - index as f64).to_bits(),
                rotation_bits: (index as f32 * 11.25).to_bits(),
            })
            .collect::<Vec<_>>();
        for expected in [Vec::new(), decorations] {
            let summary = DataComponentPatchSummary {
                map_decorations: Some(expected.clone()),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.map_decorations.as_deref(),
                Some(expected.as_slice())
            );
            assert_eq!(
                decoded
                    .codec_backed_nbt
                    .get(&VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID),
                Some(&canonical_map_decorations_nbt(&expected))
            );
        }

        let shorthand = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "home".to_string(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "ignored".to_string(),
                    value: NbtSummaryValue::Int(1),
                },
                NbtSummaryEntry {
                    name: "type".to_string(),
                    value: NbtSummaryValue::String("mansion".to_string()),
                },
                NbtSummaryEntry {
                    name: "x".to_string(),
                    value: NbtSummaryValue::Int(-12),
                },
                NbtSummaryEntry {
                    name: "z".to_string(),
                    value: NbtSummaryValue::Float(2.5_f32.to_bits()),
                },
                NbtSummaryEntry {
                    name: "rotation".to_string(),
                    value: NbtSummaryValue::Double(45.25_f64.to_bits()),
                },
            ]),
        }]);
        let wire =
            encode_nbt_summary_bounded(Some(&shorthand), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&wire), &mut decoded)
            .unwrap();
        let decoration = &decoded.map_decorations.as_ref().unwrap()[0];
        let mansion = vanilla_map_decoration_type_for_resource_id("minecraft:mansion").unwrap();
        assert_eq!(decoration.id, "home");
        assert_eq!(decoration.decoration_type_id, mansion.type_id);
        assert_eq!(decoration.decoration_type_resource_id, "minecraft:mansion");
        assert_eq!(f64::from_bits(decoration.x_bits), -12.0);
        assert_eq!(f64::from_bits(decoration.z_bits), 2.5);
        assert_eq!(f32::from_bits(decoration.rotation_bits), 45.25);
        assert_eq!(
            decoded
                .codec_backed_nbt
                .get(&VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID),
            Some(&canonical_map_decorations_nbt(
                decoded.map_decorations.as_deref().unwrap()
            ))
        );
    }

    #[test]
    fn map_decorations_descriptor_rejects_noncanonical_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID)
                .unwrap();
        let frame = vanilla_map_decoration_type_for_resource_id("minecraft:frame").unwrap();
        let valid = ItemMapDecorationSummary {
            id: "home".to_string(),
            decoration_type_id: frame.type_id,
            decoration_type_resource_id: frame.resource_id.to_string(),
            x_bits: 1.0_f64.to_bits(),
            z_bits: 2.0_f64.to_bits(),
            rotation_bits: 90.0_f32.to_bits(),
        };
        let mut cases = vec![
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                map_decorations: Some(vec![valid.clone(), valid.clone()]),
                ..DataComponentPatchSummary::default()
            },
        ];
        let mut unknown_id = valid.clone();
        unknown_id.decoration_type_id = i32::MAX;
        cases.push(DataComponentPatchSummary {
            map_decorations: Some(vec![unknown_id]),
            ..DataComponentPatchSummary::default()
        });
        let mut mismatched = valid.clone();
        mismatched.decoration_type_resource_id = "minecraft:mansion".to_string();
        cases.push(DataComponentPatchSummary {
            map_decorations: Some(vec![mismatched]),
            ..DataComponentPatchSummary::default()
        });
        let mut shorthand = valid.clone();
        shorthand.decoration_type_resource_id = "frame".to_string();
        cases.push(DataComponentPatchSummary {
            map_decorations: Some(vec![shorthand]),
            ..DataComponentPatchSummary::default()
        });
        cases.push(DataComponentPatchSummary {
            map_decorations: Some(vec![valid.clone()]),
            codec_backed_nbt: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
                canonical_map_decorations_nbt(&[]),
            )]),
            ..DataComponentPatchSummary::default()
        });
        cases.push(DataComponentPatchSummary {
            map_decorations: Some(vec![valid; MAX_DATA_COMPONENT_LIST_ITEMS + 1]),
            ..DataComponentPatchSummary::default()
        });

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn map_decorations_descriptor_rejects_malformed_codec_nbt() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID)
                .unwrap();
        let entry = |fields| {
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "home".to_string(),
                value: NbtSummaryValue::Compound(fields),
            }])
        };
        for malformed in [
            NbtSummaryValue::String("home".to_string()),
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "home".to_string(),
                value: NbtSummaryValue::Int(1),
            }]),
            entry(Vec::new()),
            entry(vec![
                NbtSummaryEntry {
                    name: "type".to_string(),
                    value: NbtSummaryValue::String("minecraft:missing".to_string()),
                },
                NbtSummaryEntry {
                    name: "x".to_string(),
                    value: NbtSummaryValue::Double(0.0_f64.to_bits()),
                },
                NbtSummaryEntry {
                    name: "z".to_string(),
                    value: NbtSummaryValue::Double(0.0_f64.to_bits()),
                },
                NbtSummaryEntry {
                    name: "rotation".to_string(),
                    value: NbtSummaryValue::Float(0.0_f32.to_bits()),
                },
            ]),
            entry(vec![
                NbtSummaryEntry {
                    name: "type".to_string(),
                    value: NbtSummaryValue::String("minecraft:frame".to_string()),
                },
                NbtSummaryEntry {
                    name: "x".to_string(),
                    value: NbtSummaryValue::String("near".to_string()),
                },
                NbtSummaryEntry {
                    name: "z".to_string(),
                    value: NbtSummaryValue::Double(0.0_f64.to_bits()),
                },
                NbtSummaryEntry {
                    name: "rotation".to_string(),
                    value: NbtSummaryValue::Float(0.0_f32.to_bits()),
                },
            ]),
        ] {
            let wire =
                encode_nbt_summary_bounded(Some(&malformed), i32::MAX as usize, MAX_NBT_DEPTH)
                    .unwrap();
            assert!(descriptor
                .decode(
                    &mut Decoder::new(&wire),
                    &mut DataComponentPatchSummary::default()
                )
                .is_err());
        }
        let missing = encode_nbt_summary_bounded(None, i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        assert!(descriptor
            .decode(
                &mut Decoder::new(&missing),
                &mut DataComponentPatchSummary::default()
            )
            .is_err());
    }

    #[test]
    fn recipes_descriptor_preserves_order_duplicates_and_canonicalizes_resource_keys() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID).unwrap();
        for expected in [
            Vec::new(),
            vec![
                "minecraft:bread".to_string(),
                "example:tools/hammer".to_string(),
                "minecraft:bread".to_string(),
            ],
        ] {
            let summary = DataComponentPatchSummary {
                recipe_keys: Some(expected.clone()),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.recipe_keys.as_deref(), Some(expected.as_slice()));
            assert_eq!(
                decoded
                    .codec_backed_nbt
                    .get(&VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID),
                Some(&canonical_recipe_keys_nbt(&expected))
            );
        }

        let shorthand = NbtSummaryValue::List(vec![
            NbtSummaryValue::String("bread".to_string()),
            NbtSummaryValue::String(":torch".to_string()),
            NbtSummaryValue::String("bread".to_string()),
        ]);
        let wire =
            encode_nbt_summary_bounded(Some(&shorthand), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&wire), &mut decoded)
            .unwrap();
        assert_eq!(
            decoded.recipe_keys.as_deref(),
            Some(
                [
                    "minecraft:bread".to_string(),
                    "minecraft:torch".to_string(),
                    "minecraft:bread".to_string(),
                ]
                .as_slice()
            )
        );
    }

    #[test]
    fn recipes_descriptor_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID).unwrap();
        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                recipe_keys: Some(vec!["bread".to_string()]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                recipe_keys: Some(vec!["Invalid".to_string()]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                recipe_keys: Some(vec!["minecraft:bread".to_string()]),
                codec_backed_nbt: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
                    canonical_recipe_keys_nbt(&[]),
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                recipe_keys: Some(vec![
                    "minecraft:bread".to_string();
                    MAX_DATA_COMPONENT_LIST_ITEMS + 1
                ]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        for malformed in [
            NbtSummaryValue::Compound(Vec::new()),
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            NbtSummaryValue::List(vec![NbtSummaryValue::String("Invalid".to_string())]),
        ] {
            let wire =
                encode_nbt_summary_bounded(Some(&malformed), i32::MAX as usize, MAX_NBT_DEPTH)
                    .unwrap();
            assert!(descriptor
                .decode(
                    &mut Decoder::new(&wire),
                    &mut DataComponentPatchSummary::default()
                )
                .is_err());
        }
        let missing = encode_nbt_summary_bounded(None, i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        assert!(descriptor
            .decode(
                &mut Decoder::new(&missing),
                &mut DataComponentPatchSummary::default()
            )
            .is_err());
    }

    #[test]
    fn map_post_processing_descriptor_round_trips_both_wire_ids() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID)
                .unwrap();
        for value in [
            MapPostProcessingSummary::Lock,
            MapPostProcessingSummary::Scale,
        ] {
            let summary = DataComponentPatchSummary {
                map_post_processing: Some(value),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_var_i32(value.id());
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, summary);
        }
    }

    #[test]
    fn map_post_processing_descriptor_rejects_a_missing_value_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn dye_descriptor_round_trips_all_canonical_wire_ids() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYE_TYPE_ID).unwrap();
        for value in VANILLA_DYE_COLOR_VALUES {
            let summary = DataComponentPatchSummary {
                dye_color_id: Some(value.id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_var_i32(value.id);
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, summary);
        }
    }

    #[test]
    fn dye_descriptor_rejects_missing_and_noncanonical_ids_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYE_TYPE_ID).unwrap();
        let past_end = i32::try_from(VANILLA_DYE_COLOR_VALUES.len()).unwrap();
        for dye_color_id in [None, Some(-1), Some(past_end)] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor
                .encode(
                    &DataComponentPatchSummary {
                        dye_color_id,
                        ..DataComponentPatchSummary::default()
                    },
                    &mut encoded,
                )
                .is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn dyed_color_descriptor_round_trips_the_full_signed_domain_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID).unwrap();
        for color in [i32::MIN, -1, 0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                dyed_color: Some(color),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_i32(color);
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, summary);
        }
    }

    #[test]
    fn dyed_color_descriptor_rejects_a_missing_value_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn map_color_descriptor_round_trips_the_full_signed_domain_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID).unwrap();
        for color in [i32::MIN, -1, 0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                map_color: Some(color),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_i32(color);
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, summary);
        }
    }

    #[test]
    fn map_color_descriptor_rejects_a_missing_value_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn provides_trim_material_descriptor_round_trips_reference_and_direct_holders_in_wire_order() {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        )
        .unwrap();
        for holder_id in [0, i32::MAX - 1] {
            let referenced = DataComponentPatchSummary {
                provides_trim_material_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&referenced, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id + 1);
            assert_eq!(wire, expected.into_inner());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&wire);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded, referenced);
        }

        let material = direct_trim_patch()
            .armor_trim_material_direct
            .expect("direct trim fixture has a material");
        let direct = DataComponentPatchSummary {
            provides_trim_material_direct: Some(material.clone()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&direct, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(0);
        expected.write_string(&material.asset_name);
        expected.write_var_i32(material.override_armor_assets.len() as i32);
        for (equipment_asset, suffix) in &material.override_armor_assets {
            expected.write_string(equipment_asset);
            expected.write_string(suffix);
        }
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                material.description_nbt.as_deref(),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, direct);
    }

    #[test]
    fn provides_trim_material_descriptor_rejects_noncanonical_values_atomically() {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        )
        .unwrap();
        let material = direct_trim_patch()
            .armor_trim_material_direct
            .expect("direct trim fixture has a material");
        let mut cases = vec![
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                provides_trim_material_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                provides_trim_material_id: Some(i32::MAX),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                provides_trim_material_id: Some(0),
                provides_trim_material_direct: Some(material.clone()),
                ..DataComponentPatchSummary::default()
            },
        ];
        for invalid in [
            TrimMaterialSummary {
                asset_name: "Ruby".to_string(),
                ..material.as_ref().clone()
            },
            TrimMaterialSummary {
                description_nbt: None,
                ..material.as_ref().clone()
            },
            TrimMaterialSummary {
                description: "Stale".to_string(),
                ..material.as_ref().clone()
            },
            TrimMaterialSummary {
                override_armor_assets: (0..=MAX_DATA_COMPONENT_LIST_ITEMS)
                    .map(|index| (format!("example:asset_{index}"), "ruby".to_string()))
                    .collect(),
                ..material.as_ref().clone()
            },
        ] {
            cases.push(DataComponentPatchSummary {
                provides_trim_material_direct: Some(Box::new(invalid)),
                ..DataComponentPatchSummary::default()
            });
        }

        for summary in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn trim_descriptor_round_trips_registry_and_direct_holders_in_wire_order() {
        let direct = direct_trim_patch();
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&direct, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let material = direct.armor_trim_material_direct.as_deref().unwrap();
        let pattern = direct.armor_trim_pattern_direct.as_deref().unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(0);
        expected.write_string("ruby");
        expected.write_var_i32(2);
        for (equipment_asset, suffix) in &material.override_armor_assets {
            expected.write_string(equipment_asset);
            expected.write_string(suffix);
        }
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                material.description_nbt.as_deref(),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        expected.write_var_i32(0);
        expected.write_string("example:spiral");
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                pattern.description_nbt.as_deref(),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        expected.write_bool(true);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.armor_trim_material_direct,
            direct.armor_trim_material_direct
        );
        assert_eq!(
            decoded.armor_trim_pattern_direct,
            direct.armor_trim_pattern_direct
        );

        let referenced = DataComponentPatchSummary {
            armor_trim_material_id: Some(0),
            armor_trim_pattern_id: Some(i32::MAX - 1),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&referenced, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(1);
        expected.write_var_i32(i32::MAX);
        assert_eq!(encoded, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&encoded), &mut decoded)
            .unwrap();
        assert_eq!(decoded.armor_trim_material_id, Some(0));
        assert_eq!(decoded.armor_trim_pattern_id, Some(i32::MAX - 1));

        let mixed = DataComponentPatchSummary {
            armor_trim_material_id: Some(41),
            armor_trim_pattern_direct: direct.armor_trim_pattern_direct.clone(),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&mixed, &mut encoded).unwrap();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&encoded.into_inner()), &mut decoded)
            .unwrap();
        assert_eq!(decoded.armor_trim_material_id, Some(41));
        assert_eq!(
            decoded.armor_trim_pattern_direct,
            mixed.armor_trim_pattern_direct
        );
    }

    #[test]
    fn trim_descriptor_rejects_noncanonical_holders_and_payloads_atomically() {
        let valid = direct_trim_patch();
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
        let mut cases = Vec::new();

        let mut missing_material = valid.clone();
        missing_material.armor_trim_material_direct = None;
        cases.push((missing_material, "trim material holder projections"));
        let mut mixed_material = valid.clone();
        mixed_material.armor_trim_material_id = Some(0);
        cases.push((mixed_material, "trim material holder projections"));
        for registry_id in [-1, i32::MAX] {
            let mut invalid = valid.clone();
            invalid.armor_trim_material_direct = None;
            invalid.armor_trim_material_id = Some(registry_id);
            cases.push((invalid, "trim material holder projections"));
        }
        let mut invalid_asset_name = valid.clone();
        invalid_asset_name
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .asset_name = "Ruby".to_string();
        cases.push((invalid_asset_name, "not a valid resource path"));
        let mut invalid_equipment_asset = valid.clone();
        invalid_equipment_asset
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets =
            BTreeMap::from([("Invalid Asset".to_string(), "ruby".to_string())]);
        cases.push((invalid_equipment_asset, "invalid resource location"));
        let mut noncanonical_equipment_asset = valid.clone();
        noncanonical_equipment_asset
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets =
            BTreeMap::from([("netherite".to_string(), "ruby".to_string())]);
        cases.push((
            noncanonical_equipment_asset,
            "noncanonical resource location",
        ));
        let mut invalid_override_suffix = valid.clone();
        invalid_override_suffix
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets =
            BTreeMap::from([("minecraft:netherite".to_string(), "Ruby".to_string())]);
        cases.push((invalid_override_suffix, "not a valid resource path"));
        let mut missing_material_nbt = valid.clone();
        missing_material_nbt
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description_nbt = None;
        cases.push((missing_material_nbt, "missing component nbt"));
        let mut missing_material_styled = valid.clone();
        missing_material_styled
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description_styled = None;
        cases.push((missing_material_styled, "missing styled projection"));
        let mut stale_material_text = valid.clone();
        stale_material_text
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description = "Stale".to_string();
        cases.push((stale_material_text, "noncanonical component projections"));
        let mut malformed_material_nbt = valid.clone();
        malformed_material_nbt
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description_nbt = Some(Box::new(NbtSummaryValue::Int(1)));
        cases.push((malformed_material_nbt, "is not a component"));
        let mut invalid_pattern_asset = valid.clone();
        invalid_pattern_asset
            .armor_trim_pattern_direct
            .as_mut()
            .unwrap()
            .asset_id = "Invalid Pattern".to_string();
        cases.push((invalid_pattern_asset, "invalid resource location"));
        let mut noncanonical_pattern_asset = valid.clone();
        noncanonical_pattern_asset
            .armor_trim_pattern_direct
            .as_mut()
            .unwrap()
            .asset_id = "spiral".to_string();
        cases.push((noncanonical_pattern_asset, "noncanonical resource location"));
        let mut mixed_pattern = valid.clone();
        mixed_pattern.armor_trim_pattern_id = Some(0);
        cases.push((mixed_pattern, "trim pattern holder projections"));
        for registry_id in [-1, i32::MAX] {
            let mut invalid = valid.clone();
            invalid.armor_trim_pattern_direct = None;
            invalid.armor_trim_pattern_id = Some(registry_id);
            cases.push((invalid, "trim pattern holder projections"));
        }
        let mut stale_pattern_styled = valid.clone();
        stale_pattern_styled
            .armor_trim_pattern_direct
            .as_mut()
            .unwrap()
            .description_styled
            .as_mut()
            .unwrap()[0]
            .text = "Stale".to_string();
        cases.push((stale_pattern_styled, "noncanonical component projections"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut oversized = valid;
        oversized
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets = (0..=MAX_DATA_COMPONENT_LIST_ITEMS)
            .map(|index| (format!("example:asset_{index}"), "ruby".to_string()))
            .collect();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn trim_descriptor_rejects_invalid_material_resource_paths_on_decode() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
        for (asset_name, suffix, expected_error) in [
            ("Ruby", "ruby", "asset_name"),
            ("ruby", "Ruby", "override_armor_assets suffix"),
        ] {
            let mut wire = Encoder::new();
            wire.write_var_i32(0);
            wire.write_string(asset_name);
            wire.write_var_i32(1);
            wire.write_string("minecraft:netherite");
            wire.write_string(suffix);
            wire.write_bytes(
                &encode_nbt_summary_bounded(
                    Some(&NbtSummaryValue::String("Ruby".to_string())),
                    i32::MAX as usize,
                    MAX_NBT_DEPTH,
                )
                .unwrap(),
            );
            wire.write_var_i32(1);
            let error = descriptor
                .decode(
                    &mut Decoder::new(&wire.into_inner()),
                    &mut DataComponentPatchSummary::default(),
                )
                .unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
        }
    }

    #[test]
    fn potion_effect_descriptors_round_trip_exact_wire_values() {
        let potion_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
                .unwrap();
        let potion = DataComponentPatchSummary {
            potion_id: Some(3),
            potion_custom_color: Some(i32::MIN),
            potion_custom_effect_count: Some(1),
            potion_custom_effects: vec![MobEffectInstanceSummary {
                effect_id: 2,
                amplifier: 255,
                duration: -1,
                ambient: true,
                show_particles: false,
                show_icon: true,
                hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                    amplifier: 1,
                    duration: i32::MAX,
                    ambient: false,
                    show_particles: true,
                    show_icon: false,
                    hidden_effect: None,
                })),
            }],
            potion_custom_name: Some("swift😀".to_string()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        potion_descriptor.encode(&potion, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_bool(true);
        expected.write_var_i32(3);
        expected.write_bool(true);
        expected.write_i32(i32::MIN);
        expected.write_var_i32(1);
        expected.write_var_i32(2);
        expected.write_var_i32(255);
        expected.write_var_i32(-1);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(true);
        expected.write_var_i32(1);
        expected.write_var_i32(i32::MAX);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_string("swift😀");
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        potion_descriptor
            .decode(&mut decoder, &mut decoded)
            .unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.potion_id, potion.potion_id);
        assert_eq!(decoded.potion_custom_color, potion.potion_custom_color);
        assert_eq!(
            decoded.potion_custom_effect_count,
            potion.potion_custom_effect_count
        );
        assert_eq!(decoded.potion_custom_effects, potion.potion_custom_effects);
        assert_eq!(decoded.potion_custom_name, potion.potion_custom_name);

        let stew_descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        )
        .unwrap();
        let stew = DataComponentPatchSummary {
            suspicious_stew_effects: vec![
                SuspiciousStewEffectSummary {
                    effect_id: 0,
                    duration: i32::MIN,
                },
                SuspiciousStewEffectSummary {
                    effect_id: 1,
                    duration: i32::MAX,
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        stew_descriptor.encode(&stew, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(2);
        expected.write_var_i32(0);
        expected.write_var_i32(i32::MIN);
        expected.write_var_i32(1);
        expected.write_var_i32(i32::MAX);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        stew_descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.suspicious_stew_effects,
            stew.suspicious_stew_effects
        );
    }

    #[test]
    fn potion_duration_scale_descriptor_round_trips_all_raw_float_bits() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID)
                .unwrap();
        for bits in [
            0.0_f32.to_bits(),
            (-0.0_f32).to_bits(),
            f32::INFINITY.to_bits(),
            f32::NEG_INFINITY.to_bits(),
            0x7fc1_2345,
        ] {
            let patch = DataComponentPatchSummary {
                potion_duration_scale_bits: Some(bits),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&patch, &mut encoded).unwrap();
            assert_eq!(encoded.into_inner(), bits.to_be_bytes());

            let mut decoded = DataComponentPatchSummary::default();
            let bytes = bits.to_be_bytes();
            let mut decoder = Decoder::new(&bytes);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.potion_duration_scale_bits, Some(bits));
        }

        let mut encoded = Encoder::new();
        let error = descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("missing potion_duration_scale component value"));
        assert!(encoded.into_inner().is_empty());
    }

    #[test]
    fn potion_effect_descriptors_enforce_bounds_and_presence_atomically() {
        fn hidden_effect_chain(depth: usize) -> Option<Box<MobEffectDetailsSummary>> {
            let mut hidden = None;
            for _ in 0..depth {
                hidden = Some(Box::new(MobEffectDetailsSummary {
                    amplifier: 0,
                    duration: 1,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                    hidden_effect: hidden,
                }));
            }
            hidden
        }

        let effect = MobEffectInstanceSummary {
            effect_id: 0,
            amplifier: 0,
            duration: 1,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        };
        let potion_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
                .unwrap();
        let empty_potion = DataComponentPatchSummary {
            potion_custom_effect_count: Some(0),
            ..DataComponentPatchSummary::default()
        };
        potion_descriptor
            .encode(&empty_potion, &mut Encoder::new())
            .unwrap();

        let maximal_name = format!("{}a", "😀".repeat((MAX_STRING_CHARS.saturating_sub(1)) / 2));
        assert_eq!(maximal_name.encode_utf16().count(), MAX_STRING_CHARS);
        let maximal_potion = DataComponentPatchSummary {
            potion_custom_effect_count: Some(MAX_DATA_COMPONENT_LIST_ITEMS),
            potion_custom_effects: vec![effect.clone(); MAX_DATA_COMPONENT_LIST_ITEMS],
            potion_custom_name: Some(maximal_name.clone()),
            ..DataComponentPatchSummary::default()
        };
        potion_descriptor
            .encode(&maximal_potion, &mut Encoder::new())
            .unwrap();

        let mut oversized_wire_name = Encoder::new();
        oversized_wire_name.write_bool(false);
        oversized_wire_name.write_bool(false);
        oversized_wire_name.write_var_i32(0);
        oversized_wire_name.write_bool(true);
        oversized_wire_name.write_string(&"😀".repeat(MAX_STRING_CHARS / 2 + 1));
        let decode_error = potion_descriptor
            .decode(
                &mut Decoder::new(&oversized_wire_name.into_inner()),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(decode_error.to_string().contains("max is 32767"));

        let mut potion_cases = Vec::new();
        let mut missing_count = empty_potion.clone();
        missing_count.potion_custom_effect_count = None;
        potion_cases.push((missing_count, "missing potion custom effect count"));
        let mut mismatched_count = empty_potion.clone();
        mismatched_count.potion_custom_effect_count = Some(1);
        potion_cases.push((mismatched_count, "does not match 0 effects"));
        let mut invalid_potion = empty_potion.clone();
        invalid_potion.potion_id = Some(crate::potion_types::VANILLA_POTIONS.len() as i32);
        potion_cases.push((invalid_potion, "invalid vanilla potion holder id"));
        let mut invalid_effect = empty_potion.clone();
        invalid_effect.potion_custom_effect_count = Some(1);
        invalid_effect.potion_custom_effects = vec![MobEffectInstanceSummary {
            effect_id: crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32,
            ..effect.clone()
        }];
        potion_cases.push((invalid_effect, "invalid vanilla mob effect holder id"));
        for amplifier in [-1, 256] {
            let mut invalid_amplifier = empty_potion.clone();
            invalid_amplifier.potion_custom_effect_count = Some(1);
            invalid_amplifier.potion_custom_effects = vec![MobEffectInstanceSummary {
                amplifier,
                ..effect.clone()
            }];
            potion_cases.push((invalid_amplifier, "outside 0..=255"));
        }
        let mut excessive_depth = empty_potion.clone();
        excessive_depth.potion_custom_effect_count = Some(1);
        excessive_depth.potion_custom_effects = vec![MobEffectInstanceSummary {
            hidden_effect: hidden_effect_chain(MAX_MOB_EFFECT_DETAILS_DEPTH + 1),
            ..effect.clone()
        }];
        potion_cases.push((excessive_depth, "exceeded max depth"));
        let mut oversized_name = empty_potion.clone();
        oversized_name.potion_custom_name = Some(format!("{maximal_name}b"));
        potion_cases.push((oversized_name, "max is 32767"));

        for (summary, expected_error) in potion_cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = potion_descriptor
                .encode(&summary, &mut encoded)
                .unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized_potion = DataComponentPatchSummary {
            potion_custom_effect_count: Some(MAX_DATA_COMPONENT_LIST_ITEMS + 1),
            potion_custom_effects: vec![effect; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            potion_descriptor.encode(&oversized_potion, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);

        let stew_descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        )
        .unwrap();
        let stew_effect = SuspiciousStewEffectSummary {
            effect_id: 0,
            duration: 160,
        };
        stew_descriptor
            .encode(
                &DataComponentPatchSummary {
                    suspicious_stew_effects: vec![
                        stew_effect.clone();
                        MAX_DATA_COMPONENT_LIST_ITEMS
                    ],
                    ..DataComponentPatchSummary::default()
                },
                &mut Encoder::new(),
            )
            .unwrap();

        let mut invalid_stew_effect = stew_effect.clone();
        invalid_stew_effect.effect_id =
            crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32;
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = stew_descriptor
            .encode(
                &DataComponentPatchSummary {
                    suspicious_stew_effects: vec![invalid_stew_effect],
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            )
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("invalid vanilla mob effect holder id"));
        assert_eq!(encoded.into_inner(), [0xaa]);

        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            stew_descriptor.encode(
                &DataComponentPatchSummary {
                    suspicious_stew_effects: vec![
                        stew_effect;
                        MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    ],
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            ),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn firework_descriptors_round_trip_exact_wire_values() {
        let explosion_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)
                .unwrap();
        let standalone = DataComponentPatchSummary {
            firework_explosion_shape: Some(FireworkExplosionShapeSummary::Burst),
            firework_explosion_colors: vec![i32::MIN, 0x11_22_33, i32::MAX],
            firework_explosion_fade_colors: vec![-1, 0x44_55_66],
            firework_explosion_has_trail: Some(true),
            firework_explosion_has_twinkle: Some(false),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        explosion_descriptor
            .encode(&standalone, &mut encoded)
            .unwrap();
        let encoded = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(FireworkExplosionShapeSummary::Burst.id());
        expected.write_var_i32(3);
        expected.write_i32(i32::MIN);
        expected.write_i32(0x11_22_33);
        expected.write_i32(i32::MAX);
        expected.write_var_i32(2);
        expected.write_i32(-1);
        expected.write_i32(0x44_55_66);
        expected.write_bool(true);
        expected.write_bool(false);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        explosion_descriptor
            .decode(&mut decoder, &mut decoded)
            .unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.firework_explosion_shape,
            standalone.firework_explosion_shape
        );
        assert_eq!(
            decoded.firework_explosion_colors,
            standalone.firework_explosion_colors
        );
        assert_eq!(
            decoded.firework_explosion_fade_colors,
            standalone.firework_explosion_fade_colors
        );
        assert_eq!(
            decoded.firework_explosion_has_trail,
            standalone.firework_explosion_has_trail
        );
        assert_eq!(
            decoded.firework_explosion_has_twinkle,
            standalone.firework_explosion_has_twinkle
        );

        let shapes = [
            FireworkExplosionShapeSummary::SmallBall,
            FireworkExplosionShapeSummary::LargeBall,
            FireworkExplosionShapeSummary::Star,
            FireworkExplosionShapeSummary::Creeper,
            FireworkExplosionShapeSummary::Burst,
        ];
        let explosions = shapes
            .into_iter()
            .enumerate()
            .map(|(index, shape)| FireworkExplosionSummary {
                shape,
                colors: vec![index as i32, -(index as i32) - 1],
                fade_colors: (index % 2 == 0)
                    .then_some(index as i32)
                    .into_iter()
                    .collect(),
                has_trail: index % 2 == 0,
                has_twinkle: index % 2 != 0,
            })
            .collect::<Vec<_>>();
        let fireworks = DataComponentPatchSummary {
            fireworks_flight_duration: Some(-1),
            fireworks_explosions_count: Some(explosions.len()),
            fireworks_explosions: explosions,
            ..DataComponentPatchSummary::default()
        };
        let fireworks_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        fireworks_descriptor
            .encode(&fireworks, &mut encoded)
            .unwrap();
        let encoded = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(-1);
        expected.write_var_i32(fireworks.fireworks_explosions.len() as i32);
        for explosion in &fireworks.fireworks_explosions {
            expected.write_var_i32(explosion.shape.id());
            expected.write_var_i32(explosion.colors.len() as i32);
            for color in &explosion.colors {
                expected.write_i32(*color);
            }
            expected.write_var_i32(explosion.fade_colors.len() as i32);
            for color in &explosion.fade_colors {
                expected.write_i32(*color);
            }
            expected.write_bool(explosion.has_trail);
            expected.write_bool(explosion.has_twinkle);
        }
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        fireworks_descriptor
            .decode(&mut decoder, &mut decoded)
            .unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.fireworks_flight_duration, Some(-1));
        assert_eq!(
            decoded.fireworks_explosions_count,
            fireworks.fireworks_explosions_count
        );
        assert_eq!(decoded.fireworks_explosions, fireworks.fireworks_explosions);
    }

    #[test]
    fn firework_descriptors_enforce_collection_bounds_and_presence_atomically() {
        let explosion_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)
                .unwrap();
        let valid_explosion = DataComponentPatchSummary {
            firework_explosion_shape: Some(FireworkExplosionShapeSummary::Star),
            firework_explosion_has_trail: Some(false),
            firework_explosion_has_twinkle: Some(true),
            ..DataComponentPatchSummary::default()
        };
        let mut maximal_colors = valid_explosion.clone();
        maximal_colors.firework_explosion_colors = vec![0; MAX_DATA_COMPONENT_LIST_ITEMS];
        explosion_descriptor
            .encode(&maximal_colors, &mut Encoder::new())
            .unwrap();

        let mut explosion_cases = Vec::new();
        let mut missing_shape = valid_explosion.clone();
        missing_shape.firework_explosion_shape = None;
        explosion_cases.push((missing_shape, "missing firework_explosion shape"));
        let mut missing_trail = valid_explosion.clone();
        missing_trail.firework_explosion_has_trail = None;
        explosion_cases.push((missing_trail, "missing firework_explosion trail"));
        let mut missing_twinkle = valid_explosion.clone();
        missing_twinkle.firework_explosion_has_twinkle = None;
        explosion_cases.push((missing_twinkle, "missing firework_explosion twinkle"));
        for (summary, expected_error) in explosion_cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = explosion_descriptor
                .encode(&summary, &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
        for colors_are_fades in [false, true] {
            let mut oversized = valid_explosion.clone();
            let colors = vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1];
            if colors_are_fades {
                oversized.firework_explosion_fade_colors = colors;
            } else {
                oversized.firework_explosion_colors = colors;
            }
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(matches!(
                explosion_descriptor.encode(&oversized, &mut encoded),
                Err(ProtocolError::PacketTooLarge(actual, max))
                    if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                        && max == MAX_DATA_COMPONENT_LIST_ITEMS
            ));
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let fireworks_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID).unwrap();
        let explosion = FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::SmallBall,
            colors: Vec::new(),
            fade_colors: Vec::new(),
            has_trail: false,
            has_twinkle: false,
        };
        let valid_fireworks = DataComponentPatchSummary {
            fireworks_flight_duration: Some(0),
            fireworks_explosions_count: Some(1),
            fireworks_explosions: vec![explosion.clone()],
            ..DataComponentPatchSummary::default()
        };
        let maximal_fireworks = DataComponentPatchSummary {
            fireworks_flight_duration: Some(i32::MIN),
            fireworks_explosions_count: Some(MAX_FIREWORK_EXPLOSIONS),
            fireworks_explosions: vec![explosion.clone(); MAX_FIREWORK_EXPLOSIONS],
            ..DataComponentPatchSummary::default()
        };
        fireworks_descriptor
            .encode(&maximal_fireworks, &mut Encoder::new())
            .unwrap();

        let mut fireworks_cases = Vec::new();
        let mut missing_duration = valid_fireworks.clone();
        missing_duration.fireworks_flight_duration = None;
        fireworks_cases.push((missing_duration, "missing fireworks flight duration"));
        let mut missing_count = valid_fireworks.clone();
        missing_count.fireworks_explosions_count = None;
        fireworks_cases.push((missing_count, "missing fireworks explosion count"));
        let mut mismatched_count = valid_fireworks.clone();
        mismatched_count.fireworks_explosions_count = Some(0);
        fireworks_cases.push((mismatched_count, "does not match 1 explosions"));
        let oversized_explosions = DataComponentPatchSummary {
            fireworks_flight_duration: Some(0),
            fireworks_explosions_count: Some(MAX_FIREWORK_EXPLOSIONS + 1),
            fireworks_explosions: vec![explosion.clone(); MAX_FIREWORK_EXPLOSIONS + 1],
            ..DataComponentPatchSummary::default()
        };
        let mut oversized_nested_colors = valid_fireworks;
        oversized_nested_colors.fireworks_explosions[0].colors =
            vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1];

        for (summary, expected_error) in fireworks_cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = fireworks_descriptor
                .encode(&summary, &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
        for (summary, expected_max) in [
            (oversized_explosions, MAX_FIREWORK_EXPLOSIONS),
            (oversized_nested_colors, MAX_DATA_COMPONENT_LIST_ITEMS),
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(matches!(
                fireworks_descriptor.encode(&summary, &mut encoded),
                Err(ProtocolError::PacketTooLarge(_, max)) if max == expected_max
            ));
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn use_effects_descriptor_round_trips_raw_float_bits_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID).unwrap();
        for bits in [
            0.0_f32.to_bits(),
            (-0.0_f32).to_bits(),
            1.0_f32.to_bits(),
            f32::INFINITY.to_bits(),
            f32::NAN.to_bits(),
        ] {
            let summary = DataComponentPatchSummary {
                use_effects: Some(UseEffectsSummary {
                    can_sprint: true,
                    interact_vibrations: false,
                    speed_multiplier: f32::from_bits(bits),
                }),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let encoded = encoded.into_inner();
            assert_eq!(&encoded[..2], &[1, 0]);
            assert_eq!(&encoded[2..], &bits.to_be_bytes());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&encoded);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.use_effects, summary.use_effects);
        }

        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("missing use_effects component value"));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn minimum_attack_charge_descriptor_round_trips_raw_float_bits() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID)
                .unwrap();
        for bits in [
            0.0_f32.to_bits(),
            (-0.0_f32).to_bits(),
            1.0_f32.to_bits(),
            f32::INFINITY.to_bits(),
            f32::NAN.to_bits(),
        ] {
            let summary = DataComponentPatchSummary {
                minimum_attack_charge_bits: Some(bits),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let encoded = encoded.into_inner();
            assert_eq!(encoded, bits.to_be_bytes());

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&encoded);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.minimum_attack_charge_bits, Some(bits));
        }

        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("missing minimum_attack_charge component value"));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn damage_type_descriptor_round_trips_holder_ids_and_rejects_missing_or_negative_values() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID).unwrap();
        for (holder_id, expected) in [(0, vec![0]), (49, vec![49]), (300, vec![0xac, 2])] {
            let summary = DataComponentPatchSummary {
                damage_type_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let encoded = encoded.into_inner();
            assert_eq!(encoded, expected);

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&encoded);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.damage_type_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                damage_type_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn item_model_descriptor_round_trips_canonical_identifier() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID).unwrap();
        let summary = DataComponentPatchSummary {
            item_model: Some("example:models/variant".to_string()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_string("example:models/variant");
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.item_model, summary.item_model);
    }

    #[test]
    fn item_model_descriptor_rejects_missing_noncanonical_and_oversized_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID).unwrap();
        let oversized = format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1));
        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                item_model: Some("stone".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                item_model: Some("Minecraft:stone".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                item_model: Some(oversized),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn tooltip_style_descriptor_round_trips_canonical_identifier() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID)
                .unwrap();
        let summary = DataComponentPatchSummary {
            tooltip_style: Some("example:rare".to_string()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_string("example:rare");
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.tooltip_style, summary.tooltip_style);
    }

    #[test]
    fn tooltip_style_descriptor_rejects_missing_noncanonical_and_oversized_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID)
                .unwrap();
        let oversized = format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1));
        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                tooltip_style: Some("rare".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tooltip_style: Some("Minecraft:rare".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tooltip_style: Some(oversized),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn note_block_sound_descriptor_round_trips_canonical_identifier() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID)
                .unwrap();
        let summary = DataComponentPatchSummary {
            note_block_sound: Some("bbb:custom/chime".to_string()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_string("bbb:custom/chime");
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.note_block_sound, summary.note_block_sound);

        let mut encoded = Encoder::new();
        encoded.write_string("custom/chime");
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.note_block_sound.as_deref(),
            Some("minecraft:custom/chime")
        );
    }

    #[test]
    fn note_block_sound_descriptor_rejects_missing_noncanonical_and_oversized_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID)
                .unwrap();
        let oversized = format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1));
        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                note_block_sound: Some("custom/chime".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                note_block_sound: Some("BBB:custom/chime".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                note_block_sound: Some(oversized),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn pot_decorations_descriptor_normalizes_missing_faces_to_four_bricks() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID)
                .unwrap();
        let stone = crate::item_types::VANILLA_ITEM_STONE_ID;
        let summary = DataComponentPatchSummary {
            pot_decorations_item_ids: vec![stone],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(4);
        expected.write_var_i32(stone);
        expected.write_var_i32(VANILLA_ITEM_BRICK_ID);
        expected.write_var_i32(VANILLA_ITEM_BRICK_ID);
        expected.write_var_i32(VANILLA_ITEM_BRICK_ID);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.pot_decorations_item_ids,
            vec![
                stone,
                VANILLA_ITEM_BRICK_ID,
                VANILLA_ITEM_BRICK_ID,
                VANILLA_ITEM_BRICK_ID,
            ]
        );
    }

    #[test]
    fn base_color_descriptor_round_trips_canonical_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID).unwrap();
        for color_id in [0, VANILLA_DYE_COLOR_RED_ID, 15] {
            let summary = DataComponentPatchSummary {
                base_color_id: Some(color_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(color_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.base_color_id, Some(color_id));
        }

        for wire_color_id in [-1, 16] {
            let mut encoded = Encoder::new();
            encoded.write_var_i32(wire_color_id);
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.base_color_id, Some(0));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                base_color_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                base_color_id: Some(16),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn villager_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, 2, i32::MAX] {
            let summary = DataComponentPatchSummary {
                villager_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.villager_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                villager_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn wolf_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID).unwrap();
        for holder_id in [0, 8, i32::MAX] {
            let summary = DataComponentPatchSummary {
                wolf_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.wolf_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                wolf_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn wolf_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, 6, i32::MAX] {
            let summary = DataComponentPatchSummary {
                wolf_sound_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.wolf_sound_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                wolf_sound_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn wolf_collar_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID).unwrap();
        for color_id in [0, 11, 15] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                    color_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(color_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, color_id)])
            );
        }

        for raw_color_id in [-1, 16, 99] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_color_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, 0)])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                    16,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn fox_variant_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID).unwrap();
        for variant_id in [0, 1] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, variant_id)])
            );
        }

        for raw_variant_id in [-1, 2, 99] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, 0)])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                    2,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn pig_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID).unwrap();
        for holder_id in [0, 2, i32::MAX] {
            let summary = DataComponentPatchSummary {
                pig_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.pig_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                pig_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn pig_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                pig_sound_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.pig_sound_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                pig_sound_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn cow_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID).unwrap();
        for holder_id in [0, 2, i32::MAX] {
            let summary = DataComponentPatchSummary {
                cow_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.cow_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                cow_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn cow_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                cow_sound_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.cow_sound_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                cow_sound_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn chicken_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, 2, i32::MAX] {
            let summary = DataComponentPatchSummary {
                chicken_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.chicken_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                chicken_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn chicken_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                chicken_sound_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.chicken_sound_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                chicken_sound_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn zombie_nautilus_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
        )
        .unwrap();
        for holder_id in [0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                zombie_nautilus_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.zombie_nautilus_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                zombie_nautilus_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn frog_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID).unwrap();
        for holder_id in [0, 2, i32::MAX] {
            let summary = DataComponentPatchSummary {
                frog_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.frog_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                frog_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn cat_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID).unwrap();
        for holder_id in [0, 10, i32::MAX] {
            let summary = DataComponentPatchSummary {
                cat_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.cat_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                cat_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn cat_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID)
                .unwrap();
        for holder_id in [0, 1, i32::MAX] {
            let summary = DataComponentPatchSummary {
                cat_sound_variant_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(holder_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.cat_sound_variant_id, Some(holder_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                cat_sound_variant_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn cat_collar_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID).unwrap();
        for color_id in [0, 5, 15] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                    color_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(color_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID, color_id)])
            );
        }

        for raw_color_id in [-1, 16, 99] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_color_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID, 0)])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                    16,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn salmon_size_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID).unwrap();
        for variant_id in [0, 1, 2] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, variant_id)])
            );
        }

        for (raw_variant_id, expected_variant_id) in [(-1, 0), (3, 2), (99, 2)] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                    expected_variant_id,
                )])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                    3,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn parrot_variant_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID)
                .unwrap();
        for variant_id in [0, 2, 4] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID, variant_id,)])
            );
        }

        for (raw_variant_id, expected_variant_id) in [(-1, 0), (5, 4), (99, 4)] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                    expected_variant_id,
                )])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                    5,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn tropical_fish_pattern_descriptor_round_trips_sparse_ids_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID)
                .unwrap();
        for pattern_id in [0, 257, 1281] {
            let summary = DataComponentPatchSummary {
                tropical_fish_pattern_id: Some(pattern_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(pattern_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.tropical_fish_pattern_id, Some(pattern_id));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                tropical_fish_pattern_id: Some(2),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tropical_fish_pattern_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn tropical_fish_pattern_color_descriptor_round_trips_canonical_ids_normalizes_decode_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        )
        .unwrap();
        for color_id in [0, 9, 15] {
            let summary = DataComponentPatchSummary {
                tropical_fish_pattern_color_id: Some(color_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(color_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.tropical_fish_pattern_color_id, Some(color_id));
        }

        for raw_color_id in [-1, 16] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_color_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.tropical_fish_pattern_color_id, Some(0));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                tropical_fish_pattern_color_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tropical_fish_pattern_color_id: Some(16),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn tropical_fish_base_color_descriptor_round_trips_canonical_ids_normalizes_decode_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        )
        .unwrap();
        for color_id in [0, 11, 15] {
            let summary = DataComponentPatchSummary {
                tropical_fish_base_color_id: Some(color_id),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(color_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.tropical_fish_base_color_id, Some(color_id));
        }

        for raw_color_id in [-1, 16] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_color_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.tropical_fish_base_color_id, Some(0));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                tropical_fish_base_color_id: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                tropical_fish_base_color_id: Some(16),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn mooshroom_variant_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID)
                .unwrap();
        for variant_id in [0, 1] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID, variant_id,)])
            );
        }

        for (raw_variant_id, expected_variant_id) in [(-1, 0), (2, 1), (99, 1)] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                    expected_variant_id,
                )])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                    2,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn rabbit_variant_descriptor_round_trips_sparse_ids_normalizes_decode_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID)
                .unwrap();
        for variant_id in [0, 5, 99] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, variant_id)])
            );
        }

        for raw_variant_id in [-1, 6, 98, 100] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, 0)])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                    6,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                    98,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn horse_variant_descriptor_round_trips_canonical_ids_normalizes_decode_wrap_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID)
                .unwrap();
        for variant_id in [0, 3, 6] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, variant_id)])
            );
        }

        for (raw_variant_id, expected_variant_id) in [(-1, 6), (7, 0), (8, 1)] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                    expected_variant_id,
                )])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                    7,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn llama_variant_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID)
                .unwrap();
        for variant_id in [0, 2, 3] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID, variant_id)])
            );
        }

        for (raw_variant_id, expected_variant_id) in [(-1, 0), (4, 3), (99, 3)] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                    expected_variant_id,
                )])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                    4,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn axolotl_variant_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID)
                .unwrap();
        for variant_id in [0, 2, 4] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                    variant_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(variant_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, variant_id,)])
            );
        }

        for raw_variant_id in [-1, 5, 99] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_variant_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, 0)])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                    5,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn sheep_color_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID).unwrap();
        for color_id in [0, 9, 15] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                    color_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(color_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID, color_id)])
            );
        }

        for raw_color_id in [-1, 16, 99] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_color_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID, 0)])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                    16,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn shulker_color_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
    ) {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID)
                .unwrap();
        for color_id in [0, 11, 15] {
            let summary = DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                    color_id,
                )]),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(color_id);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID, color_id)])
            );
        }

        for raw_color_id in [-1, 16, 99] {
            let mut wire = Encoder::new();
            wire.write_var_i32(raw_color_id);
            let wire = wire.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.builtin_enum_component_ids,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID, 0)])
            );
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                    -1,
                )]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                    16,
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn ominous_bottle_amplifier_descriptor_round_trips_persistent_range_and_rejects_invalid_values_atomically(
    ) {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        )
        .unwrap();
        for amplifier in [0, 2, 4] {
            let summary = DataComponentPatchSummary {
                ominous_bottle_amplifier: Some(amplifier),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(amplifier);
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.ominous_bottle_amplifier, Some(amplifier));
        }

        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                ominous_bottle_amplifier: Some(-1),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                ominous_bottle_amplifier: Some(5),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn provides_banner_patterns_descriptor_round_trips_direct_and_named_holder_sets() {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        )
        .unwrap();
        for summary in [
            DataComponentPatchSummary {
                provides_banner_pattern_registry_ids: vec![3, 1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                provides_banner_pattern_tag: Some("minecraft:no_item_required".to_string()),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.provides_banner_pattern_registry_ids,
                summary.provides_banner_pattern_registry_ids
            );
            assert_eq!(
                decoded.provides_banner_pattern_tag,
                summary.provides_banner_pattern_tag
            );
        }
    }

    #[test]
    fn provides_banner_patterns_descriptor_rejects_noncanonical_sets_atomically() {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        )
        .unwrap();
        for summary in [
            DataComponentPatchSummary {
                provides_banner_pattern_registry_ids: vec![0],
                provides_banner_pattern_tag: Some("minecraft:no_item_required".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                provides_banner_pattern_registry_ids: vec![-1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                provides_banner_pattern_tag: Some("no_item_required".to_string()),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                provides_banner_pattern_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn block_state_descriptor_round_trips_empty_and_ordered_properties() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID).unwrap();
        for summary in [
            DataComponentPatchSummary::default(),
            DataComponentPatchSummary {
                block_state_properties: BTreeMap::from([
                    ("powered".to_string(), "true".to_string()),
                    ("facing".to_string(), "north".to_string()),
                ]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();

            let mut expected = Encoder::new();
            expected.write_var_i32(summary.block_state_properties.len() as i32);
            for (name, value) in &summary.block_state_properties {
                expected.write_string(name);
                expected.write_string(value);
            }
            assert_eq!(encoded.clone().into_inner(), expected.into_inner());

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(
                decoded.block_state_properties,
                summary.block_state_properties
            );
        }
    }

    #[test]
    fn block_state_descriptor_rejects_collection_and_java_string_overflow_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID).unwrap();
        let maximal = DataComponentPatchSummary {
            block_state_properties: (0..MAX_BLOCK_STATE_PROPERTIES)
                .map(|index| (format!("property_{index}"), "value".to_string()))
                .collect(),
            ..DataComponentPatchSummary::default()
        };
        let mut maximal_wire = Encoder::new();
        descriptor.encode(&maximal, &mut maximal_wire).unwrap();
        let maximal_wire = maximal_wire.into_inner();
        let mut decoder = Decoder::new(&maximal_wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.block_state_properties,
            maximal.block_state_properties
        );

        let oversized = "😀".repeat(MAX_STRING_CHARS / 2 + 1);
        for summary in [
            DataComponentPatchSummary {
                block_state_properties: (0..=MAX_BLOCK_STATE_PROPERTIES)
                    .map(|index| (format!("property_{index}"), "value".to_string()))
                    .collect(),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                block_state_properties: BTreeMap::from([(oversized.clone(), "value".to_string())]),
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                block_state_properties: BTreeMap::from([(
                    "property".to_string(),
                    oversized.clone(),
                )]),
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn pot_decorations_descriptor_rejects_oversized_and_unknown_items_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID)
                .unwrap();
        for summary in [
            DataComponentPatchSummary {
                pot_decorations_item_ids: vec![VANILLA_ITEM_BRICK_ID; MAX_POT_DECORATIONS + 1],
                ..DataComponentPatchSummary::default()
            },
            DataComponentPatchSummary {
                pot_decorations_item_ids: vec![i32::MAX],
                ..DataComponentPatchSummary::default()
            },
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(descriptor.encode(&summary, &mut encoded).is_err());
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut invalid_wire = Encoder::new();
        invalid_wire.write_var_i32(1);
        invalid_wire.write_var_i32(i32::MAX);
        let wire = invalid_wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        let error = descriptor.decode(&mut decoder, &mut decoded).unwrap_err();
        assert!(error
            .to_string()
            .contains("invalid pot decoration item registry id"));
        assert!(decoded.pot_decorations_item_ids.is_empty());
    }

    #[test]
    fn rarity_descriptor_round_trips_exact_vanilla_ids_and_rejects_missing_value_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RARITY_TYPE_ID).unwrap();
        for (rarity, expected_id) in [
            (ItemRaritySummary::Common, 0),
            (ItemRaritySummary::Uncommon, 1),
            (ItemRaritySummary::Rare, 2),
            (ItemRaritySummary::Epic, 3),
        ] {
            let summary = DataComponentPatchSummary {
                rarity: Some(rarity),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            assert_eq!(encoded.clone().into_inner(), [expected_id]);

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.rarity, Some(rarity));
        }

        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn enchantment_glint_override_descriptor_round_trips_booleans_and_rejects_missing_atomically() {
        let descriptor = vanilla_data_component_descriptor(
            VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        )
        .unwrap();
        for override_value in [false, true] {
            let summary = DataComponentPatchSummary {
                enchantment_glint_override: Some(override_value),
                ..DataComponentPatchSummary::default()
            };
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            assert_eq!(encoded.clone().into_inner(), [u8::from(override_value)]);

            let wire = encoded.into_inner();
            let mut decoder = Decoder::new(&wire);
            let mut decoded = DataComponentPatchSummary::default();
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            assert_eq!(decoded.enchantment_glint_override, Some(override_value));
        }

        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn intangible_projectile_descriptor_round_trips_persistent_fallback_unit_nbt() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID)
                .unwrap();
        let summary = DataComponentPatchSummary {
            intangible_projectile: true,
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        assert_eq!(encoded.clone().into_inner(), [10, 0]);

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert!(decoded.intangible_projectile);
    }

    #[test]
    fn intangible_projectile_descriptor_rejects_absent_unit_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("missing intangible_projectile component value"));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn item_enchantment_descriptors_round_trip_maps_and_reject_noncanonical_values() {
        for type_id in [
            VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        ] {
            let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
            let entries = vec![
                ItemEnchantmentSummary {
                    holder_id: 37,
                    level: 4,
                },
                ItemEnchantmentSummary {
                    holder_id: 300,
                    level: 255,
                },
            ];
            let mut summary = DataComponentPatchSummary::default();
            if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
                summary.enchantments.clone_from(&entries);
            } else {
                summary.stored_enchantments.clone_from(&entries);
            }

            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let encoded = encoded.into_inner();
            assert_eq!(encoded, [2, 37, 4, 0xac, 2, 0xff, 1]);

            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&encoded);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
                assert_eq!(decoded.enchantments, entries);
            } else {
                assert_eq!(decoded.stored_enchantments, entries);
            }

            for invalid_entries in [
                vec![ItemEnchantmentSummary {
                    holder_id: -1,
                    level: 1,
                }],
                vec![
                    ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 1,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 1,
                        level: 2,
                    },
                ],
                vec![ItemEnchantmentSummary {
                    holder_id: 1,
                    level: -1,
                }],
                vec![ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 256,
                }],
            ] {
                let mut invalid = DataComponentPatchSummary::default();
                if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
                    invalid.enchantments = invalid_entries;
                } else {
                    invalid.stored_enchantments = invalid_entries;
                }
                let mut encoded = Encoder::new();
                encoded.write_u8(0xaa);
                assert!(descriptor.encode(&invalid, &mut encoded).is_err());
                assert_eq!(encoded.into_inner(), [0xaa]);
            }
        }

        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID).unwrap();
        let oversized = DataComponentPatchSummary {
            enchantments: (0..=MAX_DATA_COMPONENT_LIST_ITEMS)
                .map(|holder_id| ItemEnchantmentSummary {
                    holder_id: holder_id as i32,
                    level: 1,
                })
                .collect(),
            ..DataComponentPatchSummary::default()
        };
        assert!(matches!(
            descriptor.encode(&oversized, &mut Encoder::new()),
            Err(ProtocolError::PacketTooLarge(
                _,
                MAX_DATA_COMPONENT_LIST_ITEMS
            ))
        ));
    }

    #[test]
    fn name_component_descriptors_round_trip_exact_component_trees() {
        let custom_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Custom".to_string()),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("gold".to_string()),
            },
        ]);
        let cases = [
            (
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                DataComponentPatchSummary {
                    custom_name: Some("Custom".to_string()),
                    custom_name_styled: Some(vec![StyledTextRun {
                        text: "Custom".to_string(),
                        style: crate::ComponentStyle {
                            bold: Some(true),
                            color: Some(0xFF_AA_00),
                            ..Default::default()
                        },
                    }]),
                    custom_name_nbt: Some(custom_nbt),
                    ..DataComponentPatchSummary::default()
                },
            ),
            (
                VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
                DataComponentPatchSummary {
                    item_name: Some("Item".to_string()),
                    item_name_styled: Some(vec![StyledTextRun {
                        text: "Item".to_string(),
                        style: crate::ComponentStyle::default(),
                    }]),
                    item_name_nbt: Some(NbtSummaryValue::String("Item".to_string())),
                    ..DataComponentPatchSummary::default()
                },
            ),
        ];

        for (type_id, summary) in cases {
            let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let encoded = encoded.into_inner();
            let mut decoded = DataComponentPatchSummary::default();
            let mut decoder = Decoder::new(&encoded);
            descriptor.decode(&mut decoder, &mut decoded).unwrap();
            assert!(decoder.is_empty());
            match type_id {
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => {
                    assert_eq!(decoded.custom_name, summary.custom_name);
                    assert_eq!(decoded.custom_name_styled, summary.custom_name_styled);
                    assert_eq!(decoded.custom_name_nbt, summary.custom_name_nbt);
                }
                VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
                    assert_eq!(
                        encoded,
                        [8, 0, 4, b'I', b't', b'e', b'm'],
                        "item name must be one unwrapped ComponentSerialization root"
                    );
                    assert_eq!(decoded.item_name, summary.item_name);
                    assert_eq!(decoded.item_name_styled, summary.item_name_styled);
                    assert_eq!(decoded.item_name_nbt, summary.item_name_nbt);
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn name_component_descriptors_reject_noncanonical_projections_atomically() {
        let valid_custom = DataComponentPatchSummary {
            custom_name: Some("Custom".to_string()),
            custom_name_styled: Some(vec![StyledTextRun {
                text: "Custom".to_string(),
                style: crate::ComponentStyle::default(),
            }]),
            custom_name_nbt: Some(NbtSummaryValue::String("Custom".to_string())),
            ..DataComponentPatchSummary::default()
        };
        let valid_item = DataComponentPatchSummary {
            item_name: Some("Item".to_string()),
            item_name_styled: Some(vec![StyledTextRun {
                text: "Item".to_string(),
                style: crate::ComponentStyle::default(),
            }]),
            item_name_nbt: Some(NbtSummaryValue::String("Item".to_string())),
            ..DataComponentPatchSummary::default()
        };
        let cases = [
            (
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                DataComponentPatchSummary::default(),
                "text is missing",
            ),
            (
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                DataComponentPatchSummary {
                    custom_name: Some("Custom".to_string()),
                    ..DataComponentPatchSummary::default()
                },
                "missing component nbt",
            ),
            (
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                DataComponentPatchSummary {
                    custom_name: Some("stale".to_string()),
                    ..valid_custom
                },
                "noncanonical component projections",
            ),
            (
                VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
                DataComponentPatchSummary::default(),
                "text is missing",
            ),
            (
                VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
                DataComponentPatchSummary {
                    item_name_styled: Some(Vec::new()),
                    ..valid_item
                },
                "noncanonical component projections",
            ),
        ];
        for (type_id, summary, expected_error) in cases {
            let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn lore_descriptor_round_trips_canonical_components_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LORE_TYPE_ID).unwrap();
        let styled_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Styled".to_string()),
            },
            NbtSummaryEntry {
                name: "italic".to_string(),
                value: NbtSummaryValue::Byte(0),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("gold".to_string()),
            },
        ]);
        let summary = DataComponentPatchSummary {
            lore: vec!["Plain".to_string(), "Styled".to_string()],
            lore_styled: vec![
                vec![StyledTextRun {
                    text: "Plain".to_string(),
                    style: crate::ComponentStyle::default(),
                }],
                vec![StyledTextRun {
                    text: "Styled".to_string(),
                    style: crate::ComponentStyle {
                        italic: Some(false),
                        color: Some(0xFF_AA_00),
                        ..Default::default()
                    },
                }],
            ],
            lore_nbt: vec![NbtSummaryValue::String("Plain".to_string()), styled_nbt],
            ..DataComponentPatchSummary::default()
        };

        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(encoded[0], 2);
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.lore, summary.lore);
        assert_eq!(decoded.lore_styled, summary.lore_styled);
        assert_eq!(decoded.lore_nbt, summary.lore_nbt);

        let mut empty = Encoder::new();
        descriptor
            .encode(&DataComponentPatchSummary::default(), &mut empty)
            .unwrap();
        assert_eq!(empty.into_inner(), [0]);

        let maximal = DataComponentPatchSummary {
            lore: vec!["x".to_string(); MAX_LORE_LINES],
            lore_styled: vec![
                vec![StyledTextRun {
                    text: "x".to_string(),
                    style: crate::ComponentStyle::default(),
                }];
                MAX_LORE_LINES
            ],
            lore_nbt: vec![NbtSummaryValue::String("x".to_string()); MAX_LORE_LINES],
            ..DataComponentPatchSummary::default()
        };
        descriptor.encode(&maximal, &mut Encoder::new()).unwrap();
    }

    #[test]
    fn lore_descriptor_rejects_limits_and_noncanonical_projections_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LORE_TYPE_ID).unwrap();
        let valid = DataComponentPatchSummary {
            lore: vec!["line".to_string()],
            lore_styled: vec![vec![StyledTextRun {
                text: "line".to_string(),
                style: crate::ComponentStyle::default(),
            }]],
            lore_nbt: vec![NbtSummaryValue::String("line".to_string())],
            ..DataComponentPatchSummary::default()
        };
        let cases = [
            (
                DataComponentPatchSummary {
                    lore: valid.lore.clone(),
                    ..DataComponentPatchSummary::default()
                },
                "projections are not aligned",
            ),
            (
                DataComponentPatchSummary {
                    lore: vec!["stale".to_string()],
                    ..valid.clone()
                },
                "noncanonical component projections",
            ),
            (
                DataComponentPatchSummary {
                    lore_styled: vec![vec![StyledTextRun {
                        text: "stale".to_string(),
                        style: crate::ComponentStyle::default(),
                    }]],
                    ..valid.clone()
                },
                "noncanonical component projections",
            ),
            (
                DataComponentPatchSummary {
                    lore: vec!["component nbt".to_string()],
                    lore_styled: vec![Vec::new()],
                    lore_nbt: vec![NbtSummaryValue::Int(1)],
                    ..DataComponentPatchSummary::default()
                },
                "is not a component",
            ),
        ];
        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized = DataComponentPatchSummary {
            lore: vec!["x".to_string(); MAX_LORE_LINES + 1],
            lore_styled: vec![
                vec![StyledTextRun {
                    text: "x".to_string(),
                    style: crate::ComponentStyle::default(),
                }];
                MAX_LORE_LINES + 1
            ],
            lore_nbt: vec![NbtSummaryValue::String("x".to_string()); MAX_LORE_LINES + 1],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(count, max))
                if count == MAX_LORE_LINES + 1 && max == MAX_LORE_LINES
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn writable_book_descriptor_round_trips_filterable_strings_in_wire_order() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
                .unwrap();
        let summary = DataComponentPatchSummary {
            writable_book_pages: vec!["raw page".to_string(), "😀".repeat(512)],
            writable_book_page_filters: vec![Some("filtered page".to_string()), None],
            ..DataComponentPatchSummary::default()
        };

        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(2);
        expected.write_string("raw page");
        expected.write_bool(true);
        expected.write_string("filtered page");
        expected.write_string(&"😀".repeat(512));
        expected.write_bool(false);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.writable_book_pages, summary.writable_book_pages);
        assert_eq!(
            decoded.writable_book_page_filters,
            summary.writable_book_page_filters
        );

        let mut empty = Encoder::new();
        descriptor
            .encode(&DataComponentPatchSummary::default(), &mut empty)
            .unwrap();
        assert_eq!(empty.into_inner(), [0]);

        let maximal = DataComponentPatchSummary {
            writable_book_pages: vec![String::new(); MAX_BOOK_PAGES],
            writable_book_page_filters: vec![None; MAX_BOOK_PAGES],
            ..DataComponentPatchSummary::default()
        };
        descriptor.encode(&maximal, &mut Encoder::new()).unwrap();
    }

    #[test]
    fn writable_book_descriptor_enforces_page_and_java_string_bounds_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
                .unwrap();
        let cases = [
            (
                DataComponentPatchSummary {
                    writable_book_pages: vec![String::new()],
                    ..DataComponentPatchSummary::default()
                },
                "projections are not aligned",
            ),
            (
                DataComponentPatchSummary {
                    writable_book_pages: vec!["x".repeat(MAX_WRITABLE_BOOK_PAGE_CHARS + 1)],
                    writable_book_page_filters: vec![None],
                    ..DataComponentPatchSummary::default()
                },
                "string has 1025 bytes, max is 1024",
            ),
            (
                DataComponentPatchSummary {
                    writable_book_pages: vec![String::new()],
                    writable_book_page_filters: vec![Some(
                        "😀".repeat(MAX_WRITABLE_BOOK_PAGE_CHARS / 2 + 1),
                    )],
                    ..DataComponentPatchSummary::default()
                },
                "string has 1026 bytes, max is 1024",
            ),
        ];
        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized = DataComponentPatchSummary {
            writable_book_pages: vec![String::new(); MAX_BOOK_PAGES + 1],
            writable_book_page_filters: vec![None; MAX_BOOK_PAGES + 1],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(count, max))
                if count == MAX_BOOK_PAGES + 1 && max == MAX_BOOK_PAGES
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn written_book_descriptor_round_trips_filterable_components_in_wire_order() {
        let raw_page = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Raw page".to_string()),
            },
            NbtSummaryEntry {
                name: "italic".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        let filtered_page = NbtSummaryValue::String("Filtered page".to_string());
        let summary = DataComponentPatchSummary {
            written_book: Some(WrittenBookContentSummary {
                title: "Guide".to_string(),
                title_filter: Some("Safe Guide".to_string()),
                author: "Alex".to_string(),
                generation: 2,
                pages: vec!["Raw page".to_string(), "Second page".to_string()],
                page_filters: vec![Some("Filtered page".to_string()), None],
                resolved: true,
            }),
            written_book_pages_nbt: vec![
                Some(raw_page.clone()),
                Some(NbtSummaryValue::String("Second page".to_string())),
            ],
            written_book_page_filters_nbt: vec![Some(filtered_page.clone()), None],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_string("Guide");
        expected.write_bool(true);
        expected.write_string("Safe Guide");
        expected.write_string("Alex");
        expected.write_var_i32(2);
        expected.write_var_i32(2);
        expected.write_bytes(
            &encode_nbt_summary_bounded(Some(&raw_page), i32::MAX as usize, MAX_NBT_DEPTH).unwrap(),
        );
        expected.write_bool(true);
        expected.write_bytes(
            &encode_nbt_summary_bounded(Some(&filtered_page), i32::MAX as usize, MAX_NBT_DEPTH)
                .unwrap(),
        );
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                Some(&NbtSummaryValue::String("Second page".to_string())),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        expected.write_bool(false);
        expected.write_bool(true);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.written_book, summary.written_book);
        assert_eq!(
            decoded.written_book_pages_nbt,
            summary.written_book_pages_nbt
        );
        assert_eq!(
            decoded.written_book_page_filters_nbt,
            summary.written_book_page_filters_nbt
        );
    }

    #[test]
    fn written_book_descriptor_enforces_java_bounds_and_canonical_alignment_atomically() {
        let valid = DataComponentPatchSummary {
            written_book: Some(WrittenBookContentSummary {
                title: "😀".repeat(16),
                title_filter: None,
                author: "Author".to_string(),
                generation: 0,
                pages: vec!["Page".to_string()],
                page_filters: vec![None],
                resolved: false,
            }),
            written_book_pages_nbt: vec![Some(NbtSummaryValue::String("Page".to_string()))],
            written_book_page_filters_nbt: vec![None],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
                .unwrap();
        let mut boundary = Encoder::new();
        descriptor.encode(&valid, &mut boundary).unwrap();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&boundary.into_inner()), &mut decoded)
            .unwrap();
        assert_eq!(decoded.written_book, valid.written_book);
        let mut author_boundary = valid.clone();
        author_boundary.written_book.as_mut().unwrap().author = format!("{}a", "😀".repeat(16_383));
        descriptor
            .encode(&author_boundary, &mut Encoder::new())
            .unwrap();

        let mut cases = Vec::new();
        cases.push((
            DataComponentPatchSummary::default(),
            "missing written_book_content",
        ));
        let mut invalid_generation = valid.clone();
        invalid_generation.written_book.as_mut().unwrap().generation = 4;
        cases.push((invalid_generation, "outside 0..=3"));
        let mut oversized_title = valid.clone();
        oversized_title.written_book.as_mut().unwrap().title = "😀".repeat(17);
        cases.push((oversized_title, "string has 34 bytes, max is 32"));
        let mut oversized_filtered_title = valid.clone();
        oversized_filtered_title
            .written_book
            .as_mut()
            .unwrap()
            .title_filter = Some("😀".repeat(17));
        cases.push((oversized_filtered_title, "string has 34 bytes, max is 32"));
        let mut oversized_author = valid.clone();
        oversized_author.written_book.as_mut().unwrap().author = "😀".repeat(16_384);
        cases.push((oversized_author, "string has 32768 bytes, max is 32767"));
        let mut misaligned_pages = valid.clone();
        misaligned_pages.written_book_pages_nbt.clear();
        cases.push((misaligned_pages, "not aligned"));
        let mut missing_raw_page = valid.clone();
        missing_raw_page.written_book_pages_nbt[0] = None;
        cases.push((missing_raw_page, "missing raw component nbt"));
        let mut stale_page = valid.clone();
        stale_page.written_book.as_mut().unwrap().pages[0] = "Stale".to_string();
        cases.push((stale_page, "noncanonical component projections"));
        let mut malformed_page = valid.clone();
        malformed_page.written_book_pages_nbt[0] = Some(NbtSummaryValue::Int(1));
        cases.push((malformed_page, "is not a component"));
        let mut mismatched_filter = valid.clone();
        mismatched_filter
            .written_book
            .as_mut()
            .unwrap()
            .page_filters[0] = Some("Filtered".to_string());
        cases.push((mismatched_filter, "filtered projections are not aligned"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized_page_count = MAX_DATA_COMPONENT_LIST_ITEMS + 1;
        let oversized = DataComponentPatchSummary {
            written_book: Some(WrittenBookContentSummary {
                title: "Title".to_string(),
                title_filter: None,
                author: "Author".to_string(),
                generation: 0,
                pages: vec!["Page".to_string(); oversized_page_count],
                page_filters: vec![None; oversized_page_count],
                resolved: false,
            }),
            written_book_pages_nbt: vec![
                Some(NbtSummaryValue::String("Page".to_string()));
                oversized_page_count
            ],
            written_book_page_filters_nbt: vec![None; oversized_page_count],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor.encode(&oversized, &mut encoded).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, max)
                if actual == oversized_page_count && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);

        for (wire, expected_error) in [
            (
                {
                    let mut wire = Encoder::new();
                    wire.write_string(&"😀".repeat(17));
                    wire.write_bool(false);
                    wire.write_string("Author");
                    wire.write_var_i32(0);
                    wire.write_var_i32(0);
                    wire.write_bool(false);
                    wire.into_inner()
                },
                "string has 34 bytes, max is 32",
            ),
            (
                {
                    let mut wire = Encoder::new();
                    wire.write_string("Title");
                    wire.write_bool(false);
                    wire.write_string("Author");
                    wire.write_var_i32(-1);
                    wire.write_var_i32(0);
                    wire.write_bool(false);
                    wire.into_inner()
                },
                "outside 0..=3",
            ),
        ] {
            let error = descriptor
                .decode(
                    &mut Decoder::new(&wire),
                    &mut DataComponentPatchSummary::default(),
                )
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
        }
    }

    #[test]
    fn descriptor_handler_projects_component_value() {
        let mut payload = Encoder::new();
        payload.write_var_i32(99);
        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let mut summary = DataComponentPatchSummary::default();

        vanilla_data_component_descriptor(1)
            .unwrap()
            .decode(&mut decoder, &mut summary)
            .unwrap();

        assert_eq!(summary.max_stack_size, Some(99));
        assert!(decoder.is_empty());
    }

    #[test]
    fn foundational_descriptors_encode_exact_network_values() {
        let summary = DataComponentPatchSummary {
            max_stack_size: Some(i32::MIN),
            max_damage: Some(0),
            damage: Some(-1),
            unbreakable: true,
            ..DataComponentPatchSummary::default()
        };
        for (type_id, expected_value) in [
            (
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                Some(i32::MIN),
            ),
            (VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, Some(0)),
            (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, Some(-1)),
            (VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, None),
        ] {
            let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let encoded = encoded.into_inner();
            if let Some(expected_value) = expected_value {
                let mut decoder = Decoder::new(&encoded);
                assert_eq!(decoder.read_var_i32().unwrap(), expected_value);
                assert!(decoder.is_empty());
            } else {
                assert!(encoded.is_empty());
            }
        }
    }

    #[test]
    fn custom_data_descriptor_round_trips_exact_compound_nbt() {
        let custom_data = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "answer".to_string(),
                value: NbtSummaryValue::Int(42),
            },
            NbtSummaryEntry {
                name: "owner".to_string(),
                value: NbtSummaryValue::String("Alex".to_string()),
            },
        ]);
        let summary = DataComponentPatchSummary {
            custom_data: Some(custom_data.clone()),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(
            encoded,
            [
                10, // unnamed compound root
                3, 0, 6, b'a', b'n', b's', b'w', b'e', b'r', 0, 0, 0, 42, 8, 0, 5, b'o', b'w',
                b'n', b'e', b'r', 0, 4, b'A', b'l', b'e', b'x', 0, // compound end
            ]
        );

        let mut decoder = Decoder::new(&encoded);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.custom_data, Some(custom_data));
    }

    #[test]
    fn custom_data_descriptor_rejects_missing_null_and_scalar_roots() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID).unwrap();
        for custom_data in [None, Some(NbtSummaryValue::Int(42))] {
            let summary = DataComponentPatchSummary {
                custom_data,
                ..DataComponentPatchSummary::default()
            };
            let error = descriptor
                .encode(&summary, &mut Encoder::new())
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(if summary.custom_data.is_some() {
                        "must be a non-null compound tag"
                    } else {
                        "missing custom_data"
                    }),
                "{error}"
            );
        }

        for wire in [&[0][..], &[3, 0, 0, 0, 42][..]] {
            let mut decoder = Decoder::new(wire);
            let error = descriptor
                .decode(&mut decoder, &mut DataComponentPatchSummary::default())
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("must be a non-null compound tag"),
                "{error}"
            );
        }
    }

    #[test]
    fn entity_data_descriptors_encode_exact_holder_and_compound_values() {
        let entity_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Health".to_string(),
            value: NbtSummaryValue::Short(5),
        }]);
        let bucket_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Variant".to_string(),
            value: NbtSummaryValue::Int(2),
        }]);
        let block_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Delay".to_string(),
            value: NbtSummaryValue::Short(20),
        }]);
        let summary = DataComponentPatchSummary {
            entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            entity_data_tag: Some(entity_tag),
            codec_backed_nbt: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                bucket_tag,
            )]),
            block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID),
            block_entity_data_tag: Some(block_tag),
            ..DataComponentPatchSummary::default()
        };

        for (type_id, expected) in [
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                vec![
                    0x96, 0x01, // zombie holder 150
                    10, 2, 0, 6, b'H', b'e', b'a', b'l', b't', b'h', 0, 5, 0,
                ],
            ),
            (
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                vec![
                    10, 3, 0, 7, b'V', b'a', b'r', b'i', b'a', b'n', b't', 0, 0, 0, 2, 0,
                ],
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                vec![
                    9, // mob spawner holder
                    10, 2, 0, 5, b'D', b'e', b'l', b'a', b'y', 0, 20, 0,
                ],
            ),
        ] {
            let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            assert_eq!(encoded.into_inner(), expected);
        }
    }

    #[test]
    fn typed_entity_data_decode_strips_id_and_rejects_invalid_values() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID).unwrap();
        let mut wire = Encoder::new();
        wire.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        wire.write_bytes(&[
            10, // compound
            8, 0, 2, b'i', b'd', 0, 16, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':',
            b'z', b'o', b'm', b'b', b'i', b'e', 2, 0, 6, b'H', b'e', b'a', b'l', b't', b'h', 0, 5,
            0,
        ]);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut summary = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut summary).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            summary.entity_data_tag,
            Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "Health".to_string(),
                value: NbtSummaryValue::Short(5),
            }]))
        );

        for (type_id, wire, expected_error) in [
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_ENTITY_TYPES.len() as i32);
                    wire.write_bytes(&[10, 0]);
                    wire.into_inner()
                },
                "invalid vanilla entity type holder",
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_BLOCK_ENTITY_TYPES.len() as i32);
                    wire.write_bytes(&[10, 0]);
                    wire.into_inner()
                },
                "invalid vanilla block entity type holder",
            ),
            (
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                vec![3, 0, 0, 0, 42],
                "must be a non-null compound tag",
            ),
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
                    wire.write_u8(0);
                    wire.into_inner()
                },
                "must be a non-null compound tag",
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID);
                    wire.write_bytes(&[3, 0, 0, 0, 42]);
                    wire.into_inner()
                },
                "must be a non-null compound tag",
            ),
        ] {
            let error = vanilla_data_component_descriptor(type_id)
                .unwrap()
                .decode(
                    &mut Decoder::new(&wire),
                    &mut DataComponentPatchSummary::default(),
                )
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
        }
    }

    #[test]
    fn entity_data_descriptors_reject_missing_scalar_and_embedded_id_summaries() {
        let compound = NbtSummaryValue::Compound(Vec::new());
        let cases = [
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                DataComponentPatchSummary {
                    entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
                    ..DataComponentPatchSummary::default()
                },
                "missing entity_data component value",
            ),
            (
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                DataComponentPatchSummary {
                    codec_backed_nbt: BTreeMap::from([(
                        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                        NbtSummaryValue::Int(42),
                    )]),
                    ..DataComponentPatchSummary::default()
                },
                "must be a non-null compound tag",
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                DataComponentPatchSummary {
                    block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID),
                    block_entity_data_tag: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "id".to_string(),
                        value: NbtSummaryValue::String("minecraft:mob_spawner".to_string()),
                    }])),
                    ..DataComponentPatchSummary::default()
                },
                "must not contain top-level id",
            ),
        ];
        for (type_id, summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = vanilla_data_component_descriptor(type_id)
                .unwrap()
                .encode(&summary, &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let valid_entity = DataComponentPatchSummary {
            entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPES.len() as i32),
            entity_data_tag: Some(compound),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID)
            .unwrap()
            .encode(&valid_entity, &mut encoded)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid vanilla entity type holder"),
            "{error}"
        );
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn profile_descriptor_encodes_exact_game_and_partial_values() {
        let uuid = Uuid::from_u128(1);
        let game_profile = DataComponentPatchSummary {
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::GameProfile,
                uuid: Some(uuid),
                name: Some("Alex".to_string()),
                properties: vec![GameProfilePropertySummary {
                    name: "p".to_string(),
                    value: "v".to_string(),
                    signature: None,
                }],
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary {
                    model: Some(PlayerModelTypeSummary::Wide),
                    ..PlayerSkinPatchSummary::default()
                },
            }),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&game_profile, &mut encoded).unwrap();
        assert_eq!(
            encoded.into_inner(),
            [
                1, // Either.left(GameProfile)
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
                4, b'A', b'l', b'e', b'x', // name
                1,    // property count
                1, b'p', 1, b'v', 0, // property and absent signature
                0, 0, 0, // absent body, cape, and elytra
                1, 0, // present wide model
            ]
        );

        let partial_profile = DataComponentPatchSummary {
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::Partial,
                uuid: Some(uuid),
                name: None,
                properties: Vec::new(),
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary {
                    body: Some(ResourceTextureSummary {
                        asset_id: "minecraft:entity/player/wide/steve".to_string(),
                        texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
                    }),
                    cape: None,
                    elytra: Some(ResourceTextureSummary {
                        asset_id: "minecraft:entity/player/elytra".to_string(),
                        texture_path: "minecraft:textures/entity/player/elytra.png".to_string(),
                    }),
                    model: Some(PlayerModelTypeSummary::Slim),
                },
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&partial_profile, &mut encoded).unwrap();
        assert_eq!(
            encoded.into_inner(),
            [
                0, // Either.right(Partial)
                0, // absent name
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
                0, // property count
                1, 34, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'e', b'n',
                b't', b'i', b't', b'y', b'/', b'p', b'l', b'a', b'y', b'e', b'r', b'/', b'w', b'i',
                b'd', b'e', b'/', b's', b't', b'e', b'v', b'e', // body
                0,    // absent cape
                1, 30, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'e', b'n',
                b't', b'i', b't', b'y', b'/', b'p', b'l', b'a', b'y', b'e', b'r', b'/', b'e', b'l',
                b'y', b't', b'r', b'a', // elytra
                1, 1, // present slim model
            ]
        );
    }

    #[test]
    fn profile_descriptor_round_trips_java_utf16_name_boundary() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
        let name = "😀".repeat(8);
        let summary = DataComponentPatchSummary {
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::Partial,
                uuid: None,
                name: Some(name.clone()),
                properties: Vec::new(),
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary::default(),
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.profile, summary.profile);

        let mut oversized = encoded;
        oversized[1] = 1;
        oversized[2] = 36;
        oversized.splice(3..35, "😀".repeat(9).into_bytes());
        let error = descriptor
            .decode(
                &mut Decoder::new(&oversized),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::StringTooLong {
                actual: 18,
                max: MAX_PLAYER_NAME_CHARS
            }
        ));

        let mut too_many_properties = Encoder::new();
        too_many_properties.write_bool(false);
        too_many_properties.write_bool(false);
        too_many_properties.write_bool(false);
        too_many_properties.write_var_i32((MAX_PROFILE_PROPERTIES + 1) as i32);
        let too_many_properties = too_many_properties.into_inner();
        let error = descriptor
            .decode(
                &mut Decoder::new(&too_many_properties),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(
                actual,
                MAX_PROFILE_PROPERTIES
            ) if actual == MAX_PROFILE_PROPERTIES + 1
        ));

        let mut oversized_property_name = Encoder::new();
        oversized_property_name.write_bool(false);
        oversized_property_name.write_bool(false);
        oversized_property_name.write_bool(false);
        oversized_property_name.write_var_i32(1);
        oversized_property_name.write_string(&"😀".repeat(33));
        let oversized_property_name = oversized_property_name.into_inner();
        let error = descriptor
            .decode(
                &mut Decoder::new(&oversized_property_name),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::StringTooLong {
                actual: 66,
                max: MAX_PROFILE_PROPERTY_NAME_CHARS
            }
        ));
    }

    #[test]
    fn profile_descriptor_rejects_noncanonical_and_oversized_summaries_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
        let valid = || ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some("Alex".to_string()),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        };
        let mut cases = Vec::new();

        cases.push((
            DataComponentPatchSummary::default(),
            "missing profile component value",
        ));

        let mut missing_game_uuid = valid();
        missing_game_uuid.kind = ResolvableProfileKindSummary::GameProfile;
        cases.push((
            DataComponentPatchSummary {
                profile: Some(missing_game_uuid),
                ..DataComponentPatchSummary::default()
            },
            "missing its UUID",
        ));

        let mut missing_game_name = valid();
        missing_game_name.kind = ResolvableProfileKindSummary::GameProfile;
        missing_game_name.uuid = Some(Uuid::from_u128(1));
        missing_game_name.name = None;
        cases.push((
            DataComponentPatchSummary {
                profile: Some(missing_game_name),
                ..DataComponentPatchSummary::default()
            },
            "missing its name",
        ));

        let mut oversized_name = valid();
        oversized_name.name = Some("😀".repeat(9));
        cases.push((
            DataComponentPatchSummary {
                profile: Some(oversized_name),
                ..DataComponentPatchSummary::default()
            },
            "string has 18 bytes",
        ));

        let mut too_many_properties = valid();
        too_many_properties.properties = (0..=MAX_PROFILE_PROPERTIES)
            .map(|index| GameProfilePropertySummary {
                name: format!("p{index}"),
                value: "value".to_string(),
                signature: None,
            })
            .collect();
        cases.push((
            DataComponentPatchSummary {
                profile: Some(too_many_properties),
                ..DataComponentPatchSummary::default()
            },
            "exceeds configured maximum 16",
        ));

        for (property, expected_error) in [
            (
                GameProfilePropertySummary {
                    name: "p".repeat(MAX_PROFILE_PROPERTY_NAME_CHARS + 1),
                    value: "value".to_string(),
                    signature: None,
                },
                "max is 64",
            ),
            (
                GameProfilePropertySummary {
                    name: "p".to_string(),
                    value: "v".repeat(MAX_STRING_CHARS + 1),
                    signature: None,
                },
                "max is 32767",
            ),
            (
                GameProfilePropertySummary {
                    name: "p".to_string(),
                    value: "value".to_string(),
                    signature: Some("s".repeat(MAX_PROFILE_SIGNATURE_CHARS + 1)),
                },
                "max is 1024",
            ),
        ] {
            let mut oversized_property = valid();
            oversized_property.properties.push(property);
            cases.push((
                DataComponentPatchSummary {
                    profile: Some(oversized_property),
                    ..DataComponentPatchSummary::default()
                },
                expected_error,
            ));
        }

        let mut stale_profile_textures = valid();
        stale_profile_textures.profile_textures = Some(ProfileTexturesSummary {
            skin: None,
            cape: None,
            elytra: None,
        });
        cases.push((
            DataComponentPatchSummary {
                profile: Some(stale_profile_textures),
                ..DataComponentPatchSummary::default()
            },
            "noncanonical derived texture values",
        ));

        let mut stale_path = valid();
        stale_path.skin_patch.body = Some(ResourceTextureSummary {
            asset_id: "minecraft:entity/player/wide/steve".to_string(),
            texture_path: "minecraft:wrong.png".to_string(),
        });
        cases.push((
            DataComponentPatchSummary {
                profile: Some(stale_path),
                ..DataComponentPatchSummary::default()
            },
            "noncanonical derived texture values",
        ));

        let mut invalid_asset = valid();
        invalid_asset.skin_patch.body = Some(ResourceTextureSummary {
            asset_id: "minecraft:Bad Asset".to_string(),
            texture_path: "minecraft:textures/Bad Asset.png".to_string(),
        });
        cases.push((
            DataComponentPatchSummary {
                profile: Some(invalid_asset),
                ..DataComponentPatchSummary::default()
            },
            "invalid resource location",
        ));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(
                error
                    .to_string()
                    .to_ascii_lowercase()
                    .contains(&expected_error.to_ascii_lowercase()),
                "{error}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn component_patch_loop_dispatches_through_catalog_descriptor() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(20);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let summary = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(summary.added_type_ids, vec![1]);
        assert_eq!(summary.max_stack_size, Some(64));
        assert_eq!(summary.removed_type_ids, vec![20]);
        assert!(decoder.is_empty());
    }
}
