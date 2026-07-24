use super::*;
use std::sync::OnceLock;

use crate::data_component_types::{
    DataComponentTypeCatalogEntry, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, VANILLA_DATA_COMPONENT_TYPES,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
};
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
    skip_var_i32,
    decode_stored_enchantments,
    decode_dye,
    decode_dyed_color,
    decode_map_color,
    decode_map_id,
    decode_codec_backed_nbt,
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
    decode_codec_backed_nbt,
    decode_block_entity_data,
    decode_instrument,
    decode_provides_trim_material,
    decode_ominous_bottle_amplifier,
    decode_jukebox_playable_component,
    decode_provides_banner_patterns,
    decode_codec_backed_nbt,
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
        _ => None,
    }
}

fn decode_custom_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let Some(value @ NbtSummaryValue::Compound(_)) = decode_nbt_summary_from_decoder(decoder)?
    else {
        return Err(ProtocolError::InvalidData(
            "custom_data network value must be a non-null compound tag".to_string(),
        ));
    };
    summary.custom_data = Some(value);
    Ok(())
}

fn encode_custom_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary.custom_data.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing custom_data component value".to_string())
    })?;
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(
            "custom_data network value must be a non-null compound tag".to_string(),
        ));
    }
    let encoded = encode_nbt_summary_bounded(Some(value), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
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
decode_var_i32_field!(decode_repair_cost, repair_cost);
decode_var_i32_field!(decode_enchantable, enchantable_value);
decode_var_i32_field!(decode_map_id, map_id);
decode_var_i32_field!(decode_ominous_bottle_amplifier, ominous_bottle_amplifier);

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

fn decode_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.weapon_item_damage_per_attack = Some(decoder.read_var_i32()?);
    summary.weapon_disable_blocking_seconds_bits = Some(decoder.read_f32()?.to_bits());
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
    let entity_data = decode_typed_entity_data_summary(decoder)?;
    summary.entity_data_entity_type_id = Some(entity_data.type_id);
    summary.entity_data_tag = entity_data.tag;
    Ok(())
}

fn decode_block_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let block_entity_data = decode_typed_entity_data_summary(decoder)?;
    summary.block_entity_data_type_id = Some(block_entity_data.type_id);
    summary.block_entity_spawn_entity_type =
        typed_entity_data_spawn_entity_type(block_entity_data.tag.as_ref());
    summary.block_entity_data_tag = block_entity_data.tag;
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

fn decode_pot_decorations_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pot_decorations_item_ids = decode_pot_decorations_item_ids(decoder)?;
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
    summary.container_loot = true;
    decode_codec_backed_nbt(type_id, decoder, summary)
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

fn decode_zombie_nautilus_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.zombie_nautilus_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_pig_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pig_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_pig_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pig_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cow_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cow_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cow_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cow_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_chicken_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.chicken_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_chicken_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.chicken_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cat_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cat_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cat_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cat_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
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

fn decode_tropical_fish_pattern(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_pattern_id =
        Some(vanilla_tropical_fish_pattern(decoder.read_var_i32()?).packed_id);
    Ok(())
}

fn decode_tropical_fish_base_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_base_color_id = Some(decoder.read_var_i32()?);
    Ok(())
}

fn decode_tropical_fish_pattern_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_pattern_color_id = Some(decoder.read_var_i32()?);
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

fn skip_var_i32(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    decoder.read_var_i32()?;
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
    use std::collections::HashSet;

    use crate::codec::Encoder;

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
            vec![0, 1, 2, 3, 4]
        );
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
