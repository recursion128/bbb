use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::{decode_component_summary_from_decoder, skip_nbt_tag_from_decoder},
};

pub(crate) const MAX_DATA_COMPONENT_PATCH_ENTRIES: usize = 1024;
pub(crate) const MAX_DATA_COMPONENT_PREDICATE_ENTRIES: usize = 1024;
const MAX_IDENTIFIER_CHARS: usize = 32767;
const MAX_DATA_COMPONENT_LIST_ITEMS: usize = 4096;
const MAX_BLOCK_STATE_PROPERTIES: usize = 256;
const MAX_BOOK_PAGES: usize = 100;
const MAX_CONTAINER_ITEMS: usize = 256;
const MAX_FIREWORK_EXPLOSIONS: usize = 256;
const MAX_LORE_LINES: usize = 256;
const MAX_MOB_EFFECT_DETAILS_DEPTH: usize = 16;
const MAX_PARTIAL_DATA_COMPONENT_PREDICATES: usize = 64;
const MAX_PLAYER_NAME_CHARS: usize = 16;
const MAX_POT_DECORATIONS: usize = 4;
const MAX_PROFILE_PROPERTIES: usize = 16;
const MAX_PROFILE_PROPERTY_NAME_CHARS: usize = 64;
const MAX_PROFILE_SIGNATURE_CHARS: usize = 1024;
const MAX_STRING_CHARS: usize = 32767;
const MAX_WRITABLE_BOOK_PAGE_CHARS: usize = 1024;
const MAX_WRITTEN_BOOK_TITLE_CHARS: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataComponentPatchSummary {
    pub added: usize,
    #[serde(default)]
    pub added_type_ids: Vec<i32>,
    pub removed_type_ids: Vec<i32>,
}

pub(crate) fn decode_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
) -> Result<DataComponentPatchSummary> {
    let added = decoder.read_len()?;
    let removed = decoder.read_len()?;
    if added + removed > MAX_DATA_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            added + removed,
            MAX_DATA_COMPONENT_PATCH_ENTRIES,
        ));
    }

    let added_type_ids = decode_typed_data_component_list(decoder, added)?;
    let mut removed_type_ids = Vec::with_capacity(removed);
    for _ in 0..removed {
        removed_type_ids.push(decoder.read_var_i32()?);
    }

    Ok(DataComponentPatchSummary {
        added,
        added_type_ids,
        removed_type_ids,
    })
}

pub(crate) fn decode_data_component_exact_predicate_type_ids(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<i32>> {
    let component_count = decoder.read_len()?;
    if component_count > MAX_DATA_COMPONENT_PREDICATE_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            component_count,
            MAX_DATA_COMPONENT_PREDICATE_ENTRIES,
        ));
    }
    decode_typed_data_component_list(decoder, component_count)
}

fn decode_typed_data_component_list(decoder: &mut Decoder<'_>, count: usize) -> Result<Vec<i32>> {
    let mut type_ids = Vec::with_capacity(count);
    for _ in 0..count {
        let type_id = decoder.read_var_i32()?;
        decode_data_component_value(decoder, type_id)?;
        type_ids.push(type_id);
    }
    Ok(type_ids)
}

fn decode_data_component_value(decoder: &mut Decoder<'_>, type_id: i32) -> Result<()> {
    match type_id {
        // These components use DataComponentType's codec-backed stream codec,
        // which serializes one NBT tag through ByteBufCodecs.fromCodec*.
        // custom_data, intangible_projectile, map_decorations, debug_stick_state,
        // bucket_entity_data, recipes, lock, and container_loot.
        0 | 22 | 47 | 57 | 59 | 66 | 78 | 79 => skip_nbt_tag_from_decoder(decoder)?,
        // 26.1 DataComponents: max_stack_size, max_damage, damage, repair_cost,
        // additional_trade_cost, map_id, ominous_bottle_amplifier, enchantable.
        1 | 2 | 3 | 19 | 31 | 41 | 46 | 63 => {
            decoder.read_var_i32()?;
        }
        // use_effects.
        5 => decode_use_effects(decoder)?,
        // unbreakable, creative_slot_lock, glider use Unit.STREAM_CODEC.
        4 | 20 | 34 => {}
        // damage_type and holderRegistry-backed entity variants.
        8 | 81 | 82 | 83 | 93 | 94 | 95 | 96 | 97 | 98 | 99 | 100 | 105 | 106 => {
            decode_holder_registry(decoder)?
        }
        // custom_name and item_name use ComponentSerialization.STREAM_CODEC.
        6 | 9 => {
            decode_component_summary_from_decoder(decoder)?;
        }
        // lore: list(256) of ComponentSerialization.STREAM_CODEC.
        11 => decode_lore(decoder)?,
        // minimum_attack_charge and potion_duration_scale.
        7 | 52 => {
            decoder.read_f32()?;
        }
        // item_model, tooltip_style, note_block_sound.
        10 | 35 | 71 => {
            decoder.read_string(MAX_IDENTIFIER_CHARS)?;
        }
        // rarity, dye, map_post_processing, animal variant enums, collars,
        // tropical fish colors, sheep_color, shulker_color.
        12 | 43 | 48 | 73 | 84 | 85 | 86 | 87 | 88 | 89 | 90 | 91 | 92 | 101 | 103 | 104 | 107
        | 108 | 109 => {
            decoder.read_var_i32()?;
        }
        // enchantments and stored_enchantments: map(enchantment holder id -> level).
        13 | 42 => decode_varint_map(decoder)?,
        // can_place_on and can_break.
        14 | 15 => decode_adventure_mode_predicate(decoder)?,
        // attribute_modifiers.
        16 => decode_attribute_modifiers(decoder)?,
        // custom_model_data: floats, flags, strings, colors.
        17 => decode_custom_model_data(decoder)?,
        // tooltip_display: bool + collection of data component type ids.
        18 => decode_tooltip_display(decoder)?,
        // enchantment_glint_override.
        21 => {
            decoder.read_bool()?;
        }
        // food, consumable, use_remainder.
        23 => decode_food(decoder)?,
        24 => decode_consumable(decoder)?,
        25 => decode_use_remainder(decoder)?,
        // use_cooldown.
        26 => decode_use_cooldown(decoder)?,
        // tool: rules, default mining speed, damage per block, creative flag.
        28 => decode_tool(decoder)?,
        // damage_resistant and repairable are holder sets.
        27 | 33 => decode_holder_set(decoder)?,
        // weapon.
        29 => decode_weapon(decoder)?,
        // attack_range.
        30 => decode_attack_range(decoder)?,
        // equippable.
        32 => decode_equippable(decoder)?,
        // death_protection, blocks_attacks, piercing_weapon, and kinetic_weapon.
        36 => decode_death_protection(decoder)?,
        37 => decode_blocks_attacks(decoder)?,
        38 => decode_piercing_weapon(decoder)?,
        39 => decode_kinetic_weapon(decoder)?,
        // swing_animation.
        40 => decode_swing_animation(decoder)?,
        // dyed_color and map_color.
        44 | 45 => {
            decoder.read_i32()?;
        }
        // charged_projectiles and bundle_contents.
        49 | 50 => decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?,
        // potion_contents.
        51 => decode_potion_contents(decoder)?,
        // suspicious_stew_effects.
        53 => decode_suspicious_stew_effects(decoder)?,
        // writable_book_content and written_book_content.
        54 => decode_writable_book_content(decoder)?,
        55 => decode_written_book_content(decoder)?,
        // trim.
        56 => decode_armor_trim(decoder)?,
        // entity_data and block_entity_data.
        58 | 60 => decode_typed_entity_data(decoder)?,
        // instrument, trim material, jukebox playable, break sound, painting variant.
        61 => decode_instrument_component(decoder)?,
        62 => decode_trim_material_holder(decoder)?,
        64 => decode_jukebox_playable(decoder)?,
        65 => decode_holder_set(decoder)?,
        67 => decode_lodestone_tracker(decoder)?,
        70 => decode_resolvable_profile(decoder)?,
        80 => decode_sound_event_holder(decoder)?,
        102 => decode_painting_variant_holder(decoder)?,
        // firework_explosion and fireworks.
        68 => decode_firework_explosion(decoder)?,
        69 => decode_fireworks(decoder)?,
        // banner_patterns, pot_decorations, and bees.
        72 => decode_banner_pattern_layers(decoder)?,
        74 => decode_pot_decorations(decoder)?,
        77 => decode_bees(decoder)?,
        // block_state.
        76 => decode_string_map(decoder, MAX_BLOCK_STATE_PROPERTIES)?,
        // container.
        75 => decode_item_container_contents(decoder)?,
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "unsupported data component type id {other}"
            )))
        }
    }
    Ok(())
}

fn decode_holder_registry(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    Ok(())
}

fn decode_holder_with_direct(
    decoder: &mut Decoder<'_>,
    decode_direct: fn(&mut Decoder<'_>) -> Result<()>,
) -> Result<()> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        decode_direct(decoder)?;
    }
    Ok(())
}

fn decode_holder_set(decoder: &mut Decoder<'_>) -> Result<()> {
    let encoded_count = decoder.read_var_i32()?;
    if encoded_count < 0 {
        return Err(ProtocolError::NegativeLength(encoded_count));
    }
    if encoded_count == 0 {
        decoder.read_string(MAX_IDENTIFIER_CHARS)?;
        return Ok(());
    }

    let count = (encoded_count - 1) as usize;
    if count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    for _ in 0..count {
        decode_holder_registry(decoder)?;
    }
    Ok(())
}

fn decode_identifier(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_string(MAX_IDENTIFIER_CHARS)?;
    Ok(())
}

fn decode_optional_i32(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_i32()?;
    }
    Ok(())
}

fn decode_optional_f32(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_f32()?;
    }
    Ok(())
}

fn decode_optional_bool(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_bool()?;
    }
    Ok(())
}

fn decode_optional_holder_set(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_holder_set(decoder)?;
    }
    Ok(())
}

fn decode_optional_sound_event_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_sound_event_holder(decoder)?;
    }
    Ok(())
}

fn decode_optional_global_pos(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_global_pos(decoder)?;
    }
    Ok(())
}

fn decode_global_pos(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_identifier(decoder)?;
    decoder.read_i64()?;
    Ok(())
}

fn decode_lore(decoder: &mut Decoder<'_>) -> Result<()> {
    let line_count = read_bounded_len(decoder, MAX_LORE_LINES)?;
    for _ in 0..line_count {
        decode_component_summary_from_decoder(decoder)?;
    }
    Ok(())
}

fn decode_varint_map(decoder: &mut Decoder<'_>) -> Result<()> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..count {
        decoder.read_var_i32()?;
        decoder.read_var_i32()?;
    }
    Ok(())
}

fn decode_adventure_mode_predicate(decoder: &mut Decoder<'_>) -> Result<()> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..count {
        decode_block_predicate(decoder)?;
    }
    Ok(())
}

fn decode_block_predicate(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_holder_set(decoder)?;
    }
    if decoder.read_bool()? {
        decode_state_properties_predicate(decoder)?;
    }
    if decoder.read_bool()? {
        skip_nbt_tag_from_decoder(decoder)?;
    }
    decode_data_component_matchers(decoder)
}

fn decode_state_properties_predicate(decoder: &mut Decoder<'_>) -> Result<()> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..count {
        decoder.read_string(MAX_STRING_CHARS)?;
        decode_state_property_value_matcher(decoder)?;
    }
    Ok(())
}

fn decode_state_property_value_matcher(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_string(MAX_STRING_CHARS)?;
    } else {
        decode_optional_string(decoder, MAX_STRING_CHARS)?;
        decode_optional_string(decoder, MAX_STRING_CHARS)?;
    }
    Ok(())
}

fn decode_data_component_matchers(decoder: &mut Decoder<'_>) -> Result<()> {
    let exact_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_PREDICATE_ENTRIES)?;
    decode_typed_data_component_list(decoder, exact_count)?;

    let partial_count = read_bounded_len(decoder, MAX_PARTIAL_DATA_COMPONENT_PREDICATES)?;
    for _ in 0..partial_count {
        decoder.read_bool()?;
        decoder.read_var_i32()?;
        skip_nbt_tag_from_decoder(decoder)?;
    }
    Ok(())
}

fn decode_attribute_modifiers(decoder: &mut Decoder<'_>) -> Result<()> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..count {
        decode_holder_registry(decoder)?;
        decode_identifier(decoder)?;
        decoder.read_f64()?;
        decoder.read_var_i32()?;
        decoder.read_var_i32()?;
        decode_attribute_modifier_display(decoder)?;
    }
    Ok(())
}

fn decode_attribute_modifier_display(decoder: &mut Decoder<'_>) -> Result<()> {
    match decoder.read_var_i32()? {
        0 | 1 => Ok(()),
        2 => {
            decode_component_summary_from_decoder(decoder)?;
            Ok(())
        }
        other => Err(ProtocolError::InvalidData(format!(
            "invalid attribute modifier display type id {other}"
        ))),
    }
}

fn decode_custom_model_data(decoder: &mut Decoder<'_>) -> Result<()> {
    let floats = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..floats {
        decoder.read_f32()?;
    }

    let flags = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..flags {
        decoder.read_bool()?;
    }

    let strings = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..strings {
        decoder.read_string(MAX_STRING_CHARS)?;
    }

    let colors = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..colors {
        decoder.read_i32()?;
    }

    Ok(())
}

fn decode_use_effects(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_bool()?;
    decoder.read_bool()?;
    decoder.read_f32()?;
    Ok(())
}

fn decode_food(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_f32()?;
    decoder.read_bool()?;
    Ok(())
}

fn decode_consumable(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decoder.read_var_i32()?;
    decode_sound_event_holder(decoder)?;
    decoder.read_bool()?;

    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..effect_count {
        decode_consume_effect(decoder)?;
    }
    Ok(())
}

fn decode_consume_effect(decoder: &mut Decoder<'_>) -> Result<()> {
    match decoder.read_var_i32()? {
        0 => {
            let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
            for _ in 0..effect_count {
                decode_mob_effect_instance(decoder)?;
            }
            decoder.read_f32()?;
        }
        1 => decode_holder_set(decoder)?,
        2 => {}
        3 => {
            decoder.read_f32()?;
        }
        4 => decode_sound_event_holder(decoder)?,
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid consume effect type id {other}"
            )));
        }
    }
    Ok(())
}

fn decode_use_remainder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_item_stack_template(decoder)
}

fn decode_item_stack_template(decoder: &mut Decoder<'_>) -> Result<()> {
    let item_id = decoder.read_var_i32()?;
    if item_id < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid item stack template item id {item_id}"
        )));
    }
    let count = decoder.read_var_i32()?;
    if count <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid item stack template count {count}"
        )));
    }
    decode_data_component_patch_summary(decoder)?;
    Ok(())
}

fn decode_item_stack_template_list(decoder: &mut Decoder<'_>, max: usize) -> Result<()> {
    let count = read_bounded_len(decoder, max)?;
    for _ in 0..count {
        decode_item_stack_template(decoder)?;
    }
    Ok(())
}

fn decode_optional_item_stack_template(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_item_stack_template(decoder)?;
    }
    Ok(())
}

fn decode_item_container_contents(decoder: &mut Decoder<'_>) -> Result<()> {
    let count = read_bounded_len(decoder, MAX_CONTAINER_ITEMS)?;
    for _ in 0..count {
        decode_optional_item_stack_template(decoder)?;
    }
    Ok(())
}

fn decode_tool(decoder: &mut Decoder<'_>) -> Result<()> {
    let rule_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..rule_count {
        decode_holder_set(decoder)?;
        decode_optional_f32(decoder)?;
        decode_optional_bool(decoder)?;
    }
    decoder.read_f32()?;
    decoder.read_var_i32()?;
    decoder.read_bool()?;
    Ok(())
}

fn decode_use_cooldown(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decode_optional_string(decoder, MAX_IDENTIFIER_CHARS)
}

fn decode_weapon(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_f32()?;
    Ok(())
}

fn decode_attack_range(decoder: &mut Decoder<'_>) -> Result<()> {
    for _ in 0..6 {
        decoder.read_f32()?;
    }
    Ok(())
}

fn decode_death_protection(decoder: &mut Decoder<'_>) -> Result<()> {
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..effect_count {
        decode_consume_effect(decoder)?;
    }
    Ok(())
}

fn decode_blocks_attacks(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decoder.read_f32()?;

    let reduction_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..reduction_count {
        decode_damage_reduction(decoder)?;
    }

    decode_item_damage_function(decoder)?;
    decode_optional_holder_set(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    Ok(())
}

fn decode_damage_reduction(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decode_optional_holder_set(decoder)?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    Ok(())
}

fn decode_item_damage_function(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_f32()?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    Ok(())
}

fn decode_piercing_weapon(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_bool()?;
    decoder.read_bool()?;
    decode_optional_sound_event_holder(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    Ok(())
}

fn decode_kinetic_weapon(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_var_i32()?;
    decode_optional_kinetic_weapon_condition(decoder)?;
    decode_optional_kinetic_weapon_condition(decoder)?;
    decode_optional_kinetic_weapon_condition(decoder)?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    decode_optional_sound_event_holder(decoder)?;
    decode_optional_sound_event_holder(decoder)?;
    Ok(())
}

fn decode_optional_kinetic_weapon_condition(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_var_i32()?;
        decoder.read_f32()?;
        decoder.read_f32()?;
    }
    Ok(())
}

fn decode_equippable(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decode_sound_event_holder(decoder)?;
    decode_optional_string(decoder, MAX_IDENTIFIER_CHARS)?;
    decode_optional_string(decoder, MAX_IDENTIFIER_CHARS)?;
    if decoder.read_bool()? {
        decode_holder_set(decoder)?;
    }
    for _ in 0..5 {
        decoder.read_bool()?;
    }
    decode_sound_event_holder(decoder)?;
    Ok(())
}

fn decode_armor_trim(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_trim_material_holder(decoder)?;
    decode_trim_pattern_holder(decoder)
}

fn decode_typed_entity_data(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    skip_nbt_tag_from_decoder(decoder)
}

fn decode_instrument_component(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_instrument)
}

fn decode_direct_instrument(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_sound_event_holder(decoder)?;
    decoder.read_f32()?;
    decoder.read_f32()?;
    decode_component_summary_from_decoder(decoder)?;
    Ok(())
}

fn decode_trim_material_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_trim_material)
}

fn decode_direct_trim_material(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_material_asset_group(decoder)?;
    decode_component_summary_from_decoder(decoder)?;
    Ok(())
}

fn decode_material_asset_group(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_string(MAX_STRING_CHARS)?;
    let overrides = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..overrides {
        decode_identifier(decoder)?;
        decoder.read_string(MAX_STRING_CHARS)?;
    }
    Ok(())
}

fn decode_trim_pattern_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_trim_pattern)
}

fn decode_direct_trim_pattern(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_identifier(decoder)?;
    decode_component_summary_from_decoder(decoder)?;
    decoder.read_bool()?;
    Ok(())
}

fn decode_jukebox_playable(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_jukebox_song)
}

fn decode_lodestone_tracker(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_optional_global_pos(decoder)?;
    decoder.read_bool()?;
    Ok(())
}

fn decode_resolvable_profile(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_game_profile(decoder)?;
    } else {
        decode_partial_profile(decoder)?;
    }
    decode_player_skin_patch(decoder)
}

fn decode_game_profile(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_uuid()?;
    decoder.read_string(MAX_PLAYER_NAME_CHARS)?;
    decode_game_profile_properties(decoder)
}

fn decode_partial_profile(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_optional_string(decoder, MAX_PLAYER_NAME_CHARS)?;
    if decoder.read_bool()? {
        decoder.read_uuid()?;
    }
    decode_game_profile_properties(decoder)
}

fn decode_game_profile_properties(decoder: &mut Decoder<'_>) -> Result<()> {
    let property_count = read_bounded_len(decoder, MAX_PROFILE_PROPERTIES)?;
    for _ in 0..property_count {
        decoder.read_string(MAX_PROFILE_PROPERTY_NAME_CHARS)?;
        decoder.read_string(MAX_STRING_CHARS)?;
        decode_optional_string(decoder, MAX_PROFILE_SIGNATURE_CHARS)?;
    }
    Ok(())
}

fn decode_player_skin_patch(decoder: &mut Decoder<'_>) -> Result<()> {
    for _ in 0..3 {
        decode_optional_resource_texture(decoder)?;
    }
    if decoder.read_bool()? {
        decoder.read_bool()?;
    }
    Ok(())
}

fn decode_optional_resource_texture(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_identifier(decoder)?;
    }
    Ok(())
}

fn decode_direct_jukebox_song(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_sound_event_holder(decoder)?;
    decode_component_summary_from_decoder(decoder)?;
    decoder.read_f32()?;
    decoder.read_var_i32()?;
    Ok(())
}

fn decode_sound_event_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_sound_event)
}

fn decode_direct_sound_event(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_identifier(decoder)?;
    decode_optional_f32(decoder)
}

fn decode_banner_pattern_layers(decoder: &mut Decoder<'_>) -> Result<()> {
    let layer_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..layer_count {
        decode_banner_pattern_holder(decoder)?;
        decoder.read_var_i32()?;
    }
    Ok(())
}

fn decode_banner_pattern_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_banner_pattern)
}

fn decode_direct_banner_pattern(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_identifier(decoder)?;
    decoder.read_string(MAX_STRING_CHARS)?;
    Ok(())
}

fn decode_pot_decorations(decoder: &mut Decoder<'_>) -> Result<()> {
    let item_count = read_bounded_len(decoder, MAX_POT_DECORATIONS)?;
    for _ in 0..item_count {
        decoder.read_var_i32()?;
    }
    Ok(())
}

fn decode_bees(decoder: &mut Decoder<'_>) -> Result<()> {
    let bee_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..bee_count {
        decoder.read_var_i32()?;
        skip_nbt_tag_from_decoder(decoder)?;
        decoder.read_var_i32()?;
        decoder.read_var_i32()?;
    }
    Ok(())
}

fn decode_painting_variant_holder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_with_direct(decoder, decode_direct_painting_variant)
}

fn decode_direct_painting_variant(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_var_i32()?;
    decode_identifier(decoder)?;
    decode_optional_component(decoder)?;
    decode_optional_component(decoder)?;
    Ok(())
}

fn decode_swing_animation(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decoder.read_var_i32()?;
    Ok(())
}

fn decode_potion_contents(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_holder_registry(decoder)?;
    }
    decode_optional_i32(decoder)?;
    let effects = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..effects {
        decode_mob_effect_instance(decoder)?;
    }
    decode_optional_string(decoder, MAX_STRING_CHARS)?;
    Ok(())
}

fn decode_suspicious_stew_effects(decoder: &mut Decoder<'_>) -> Result<()> {
    let effects = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..effects {
        decode_holder_registry(decoder)?;
        decoder.read_var_i32()?;
    }
    Ok(())
}

fn decode_mob_effect_instance(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_holder_registry(decoder)?;
    decode_mob_effect_details(decoder, 0)
}

fn decode_mob_effect_details(decoder: &mut Decoder<'_>, depth: usize) -> Result<()> {
    if depth > MAX_MOB_EFFECT_DETAILS_DEPTH {
        return Err(ProtocolError::InvalidData(
            "mob effect details exceeded max depth".to_string(),
        ));
    }
    decoder.read_var_i32()?;
    decoder.read_var_i32()?;
    decoder.read_bool()?;
    decoder.read_bool()?;
    decoder.read_bool()?;
    if decoder.read_bool()? {
        decode_mob_effect_details(decoder, depth + 1)?;
    }
    Ok(())
}

fn decode_writable_book_content(decoder: &mut Decoder<'_>) -> Result<()> {
    let pages = read_bounded_len(decoder, MAX_BOOK_PAGES)?;
    for _ in 0..pages {
        decode_filterable_string(decoder, MAX_WRITABLE_BOOK_PAGE_CHARS)?;
    }
    Ok(())
}

fn decode_written_book_content(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_filterable_string(decoder, MAX_WRITTEN_BOOK_TITLE_CHARS)?;
    decoder.read_string(MAX_STRING_CHARS)?;
    decoder.read_var_i32()?;
    let pages = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..pages {
        decode_filterable_component(decoder)?;
    }
    decoder.read_bool()?;
    Ok(())
}

fn decode_filterable_string(decoder: &mut Decoder<'_>, max_chars: usize) -> Result<()> {
    decoder.read_string(max_chars)?;
    decode_optional_string(decoder, max_chars)
}

fn decode_filterable_component(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_component_summary_from_decoder(decoder)?;
    if decoder.read_bool()? {
        decode_component_summary_from_decoder(decoder)?;
    }
    Ok(())
}

fn decode_optional_component(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_component_summary_from_decoder(decoder)?;
    }
    Ok(())
}

fn decode_fireworks(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    let explosions = read_bounded_len(decoder, MAX_FIREWORK_EXPLOSIONS)?;
    for _ in 0..explosions {
        decode_firework_explosion(decoder)?;
    }
    Ok(())
}

fn decode_firework_explosion(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_var_i32()?;
    decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    decoder.read_bool()?;
    decoder.read_bool()?;
    Ok(())
}

fn decode_int_list(decoder: &mut Decoder<'_>, max: usize) -> Result<()> {
    let count = read_bounded_len(decoder, max)?;
    for _ in 0..count {
        decoder.read_i32()?;
    }
    Ok(())
}

fn decode_string_map(decoder: &mut Decoder<'_>, max: usize) -> Result<()> {
    let count = read_bounded_len(decoder, max)?;
    for _ in 0..count {
        decoder.read_string(MAX_STRING_CHARS)?;
        decoder.read_string(MAX_STRING_CHARS)?;
    }
    Ok(())
}

fn decode_optional_string(decoder: &mut Decoder<'_>, max_chars: usize) -> Result<()> {
    if decoder.read_bool()? {
        decoder.read_string(max_chars)?;
    }
    Ok(())
}

fn decode_tooltip_display(decoder: &mut Decoder<'_>) -> Result<()> {
    decoder.read_bool()?;
    let hidden_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..hidden_count {
        decoder.read_var_i32()?;
    }
    Ok(())
}

fn read_bounded_len(decoder: &mut Decoder<'_>, max: usize) -> Result<usize> {
    let len = decoder.read_len()?;
    if len > max {
        return Err(ProtocolError::PacketTooLarge(len, max));
    }
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Encoder;
    use uuid::Uuid;

    #[test]
    fn decodes_supported_data_component_patch_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(5);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(4);
        payload.write_var_i32(6);
        payload.write_bytes(&nbt_string_root("Named"));
        payload.write_var_i32(10);
        payload.write_string("minecraft:diamond_sword");
        payload.write_var_i32(21);
        payload.write_bool(true);
        payload.write_var_i32(3);
        payload.write_var_i32(12);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 5,
                added_type_ids: vec![1, 4, 6, 10, 21],
                removed_type_ids: vec![3, 12],
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_common_complex_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(11);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Line one"));
        payload.write_bytes(&nbt_string_root("Line two"));

        payload.write_var_i32(13);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_var_i32(3);
        payload.write_var_i32(9);
        payload.write_var_i32(1);

        payload.write_var_i32(17);
        payload.write_var_i32(2);
        payload.write_f32(1.0);
        payload.write_f32(2.5);
        payload.write_var_i32(2);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_var_i32(1);
        payload.write_string("variant");
        payload.write_var_i32(2);
        payload.write_i32(0x112233);
        payload.write_i32(0x445566);

        payload.write_var_i32(18);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_var_i32(11);
        payload.write_var_i32(13);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![11, 13, 17, 18],
                removed_type_ids: Vec::new(),
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_interaction_and_attribute_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(3);
        payload.write_var_i32(0);

        payload.write_var_i32(14);
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_bool(true);
        payload.write_string("north");
        payload.write_string("age");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("1");
        payload.write_bool(true);
        payload.write_string("3");
        payload.write_bool(true);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(1);
        payload.write_bool(false);
        payload.write_var_i32(6);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(15);
        payload.write_var_i32(1);
        write_empty_block_predicate(&mut payload);

        payload.write_var_i32(16);
        payload.write_var_i32(3);
        write_attribute_modifier_entry(&mut payload, "minecraft:test/default", 0, None);
        write_attribute_modifier_entry(&mut payload, "minecraft:test/hidden", 1, None);
        write_attribute_modifier_entry(
            &mut payload,
            "minecraft:test/override",
            2,
            Some("Override"),
        );

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![14, 15, 16],
                removed_type_ids: Vec::new(),
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_combat_item_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(36);
        payload.write_var_i32(2);
        payload.write_var_i32(2);
        payload.write_var_i32(4);
        write_direct_sound_event(&mut payload, "minecraft:item.totem.use", None);

        payload.write_var_i32(37);
        payload.write_f32(0.25);
        payload.write_f32(1.5);
        payload.write_var_i32(1);
        payload.write_f32(90.0);
        payload.write_bool(true);
        write_holder_set_tag(&mut payload, "minecraft:bypasses_shield");
        payload.write_f32(1.0);
        payload.write_f32(0.5);
        payload.write_f32(1.0);
        payload.write_f32(0.0);
        payload.write_f32(1.0);
        payload.write_bool(true);
        write_holder_set_ids(&mut payload, &[3]);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.shield.block"));
        write_optional_direct_sound_event(&mut payload, None);

        payload.write_var_i32(38);
        payload.write_bool(true);
        payload.write_bool(true);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.mace.smash_air"));
        write_optional_direct_sound_event(&mut payload, None);

        payload.write_var_i32(39);
        payload.write_var_i32(10);
        payload.write_var_i32(2);
        payload.write_bool(true);
        write_kinetic_weapon_condition(&mut payload, 20, 0.25, 0.5);
        payload.write_bool(false);
        payload.write_bool(true);
        write_kinetic_weapon_condition(&mut payload, 30, 1.0, 1.5);
        payload.write_f32(0.2);
        payload.write_f32(2.0);
        write_optional_direct_sound_event(&mut payload, None);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.mace.smash_ground"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![36, 37, 38, 39],
                removed_type_ids: Vec::new(),
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_animal_variant_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [85, 86, 87, 88, 91, 92, 101, 103, 104];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        for (index, component_id) in component_ids.iter().enumerate() {
            payload.write_var_i32(*component_id);
            payload.write_var_i32(index as i32);
        }

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_profile_and_decorative_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [65, 67, 70, 72, 74, 77];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(65);
        write_holder_set_tag(&mut payload, "minecraft:no_item_required");

        payload.write_var_i32(67);
        payload.write_bool(true);
        payload.write_string("minecraft:overworld");
        payload.write_i64(0);
        payload.write_bool(true);

        payload.write_var_i32(70);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("Steve");
        payload.write_bool(true);
        payload.write_uuid(Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678));
        payload.write_var_i32(1);
        payload.write_string("textures");
        payload.write_string("skin-value");
        payload.write_bool(true);
        payload.write_string("skin-signature");
        payload.write_bool(true);
        payload.write_string("minecraft:entity/player/wide/steve");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("minecraft:entity/player/elytra");
        payload.write_bool(true);
        payload.write_bool(true);

        payload.write_var_i32(72);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_var_i32(14);
        payload.write_var_i32(0);
        payload.write_string("minecraft:stripe_bottom");
        payload.write_string("block.minecraft.banner.stripe_bottom");
        payload.write_var_i32(11);

        payload.write_var_i32(74);
        payload.write_var_i32(4);
        for item_id in [1, 2, 3, 4] {
            payload.write_var_i32(item_id);
        }

        payload.write_var_i32(77);
        payload.write_var_i32(1);
        payload.write_var_i32(3);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(40);
        payload.write_var_i32(2400);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_entity_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [58, 59, 60];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(58);
        payload.write_var_i32(1);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(59);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(60);
        payload.write_var_i32(2);
        payload.write_bytes(&empty_nbt_compound_root());

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_additional_item_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [
            0, 5, 23, 24, 25, 26, 28, 29, 30, 32, 40, 44, 45, 49, 50, 51, 53, 54, 55, 56, 61, 64,
            68, 69, 75, 76, 80, 102,
        ];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(0);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_f32(0.5);

        payload.write_var_i32(23);
        payload.write_var_i32(6);
        payload.write_f32(7.2);
        payload.write_bool(true);

        payload.write_var_i32(24);
        payload.write_f32(1.6);
        payload.write_var_i32(2);
        write_direct_sound_event(&mut payload, "minecraft:entity.generic.drink", None);
        payload.write_bool(true);
        payload.write_var_i32(5);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(5);
        write_mob_effect_details(&mut payload, false);
        payload.write_f32(0.75);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(6);
        payload.write_var_i32(2);
        payload.write_var_i32(3);
        payload.write_f32(16.0);
        payload.write_var_i32(4);
        write_direct_sound_event(&mut payload, "minecraft:item.honey_bottle.drink", None);

        payload.write_var_i32(25);
        write_item_stack_template(&mut payload, 42, 1);

        payload.write_var_i32(26);
        payload.write_f32(1.25);
        payload.write_bool(true);
        payload.write_string("minecraft:ender_pearl");

        payload.write_var_i32(28);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_f32(8.0);
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_f32(1.0);
        payload.write_var_i32(1);
        payload.write_bool(true);

        payload.write_var_i32(29);
        payload.write_var_i32(1);
        payload.write_f32(0.5);

        payload.write_var_i32(30);
        for value in [0.0, 3.0, 0.0, 5.0, 0.3, 1.0] {
            payload.write_f32(value);
        }

        payload.write_var_i32(32);
        payload.write_var_i32(5);
        write_direct_sound_event(&mut payload, "minecraft:item.armor.equip_generic", None);
        payload.write_bool(true);
        payload.write_string("minecraft:diamond");
        payload.write_bool(true);
        payload.write_string("minecraft:misc/pumpkinblur");
        payload.write_bool(true);
        payload.write_var_i32(0);
        payload.write_string("minecraft:skeletons");
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_bool(true);
        write_direct_sound_event(&mut payload, "minecraft:item.shears.snip", None);

        payload.write_var_i32(40);
        payload.write_var_i32(0);
        payload.write_var_i32(6);

        payload.write_var_i32(44);
        payload.write_i32(0x112233);
        payload.write_var_i32(45);
        payload.write_i32(0x445566);

        payload.write_var_i32(49);
        payload.write_var_i32(2);
        write_item_stack_template(&mut payload, 50, 1);
        write_item_stack_template(&mut payload, 51, 2);

        payload.write_var_i32(50);
        payload.write_var_i32(1);
        write_item_stack_template(&mut payload, 52, 3);

        payload.write_var_i32(51);
        payload.write_bool(true);
        payload.write_var_i32(3);
        payload.write_bool(true);
        payload.write_i32(0x778899);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        write_mob_effect_details(&mut payload, false);
        payload.write_bool(true);
        payload.write_string("healing");

        payload.write_var_i32(53);
        payload.write_var_i32(1);
        payload.write_var_i32(4);
        payload.write_var_i32(160);

        payload.write_var_i32(54);
        payload.write_var_i32(1);
        write_filterable_string(&mut payload, "raw page", Some("filtered page"));

        payload.write_var_i32(55);
        write_filterable_string(&mut payload, "Title", None);
        payload.write_string("Author");
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_bytes(&nbt_string_root("Page"));
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Filtered"));
        payload.write_bool(true);

        payload.write_var_i32(56);
        payload.write_var_i32(2);
        payload.write_var_i32(3);

        payload.write_var_i32(61);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_f32(1.0);
        payload.write_f32(16.0);
        payload.write_bytes(&nbt_string_root("Instrument"));

        payload.write_var_i32(64);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_bytes(&nbt_string_root("Song"));
        payload.write_f32(120.0);
        payload.write_var_i32(15);

        payload.write_var_i32(68);
        write_firework_explosion(&mut payload, 2);

        payload.write_var_i32(69);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        write_firework_explosion(&mut payload, 0);

        payload.write_var_i32(75);
        payload.write_var_i32(3);
        payload.write_bool(false);
        payload.write_bool(true);
        write_item_stack_template(&mut payload, 53, 4);
        payload.write_bool(false);

        payload.write_var_i32(76);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_string("north");
        payload.write_string("lit");
        payload.write_string("true");

        payload.write_var_i32(80);
        write_direct_sound_event(&mut payload, "minecraft:block.note_block.harp", Some(16.0));

        payload.write_var_i32(102);
        payload.write_var_i32(0);
        payload.write_var_i32(16);
        payload.write_var_i32(16);
        payload.write_string("minecraft:kebab");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Painter"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn rejects_unknown_data_component_type_without_consuming_payload_guesswork() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(110);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported data component type id 110"));
    }

    pub(super) fn nbt_string_root(value: &str) -> Vec<u8> {
        let mut out = vec![8];
        write_mutf8(&mut out, value);
        out
    }

    fn empty_nbt_compound_root() -> Vec<u8> {
        vec![10, 0]
    }

    fn write_filterable_string(payload: &mut Encoder, raw: &str, filtered: Option<&str>) {
        payload.write_string(raw);
        match filtered {
            Some(filtered) => {
                payload.write_bool(true);
                payload.write_string(filtered);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_mob_effect_details(payload: &mut Encoder, hidden: bool) {
        payload.write_var_i32(1);
        payload.write_var_i32(200);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_bool(hidden);
        if hidden {
            write_mob_effect_details(payload, false);
        }
    }

    fn write_firework_explosion(payload: &mut Encoder, shape: i32) {
        payload.write_var_i32(shape);
        payload.write_var_i32(2);
        payload.write_i32(0x010203);
        payload.write_i32(0x040506);
        payload.write_var_i32(1);
        payload.write_i32(0x070809);
        payload.write_bool(true);
        payload.write_bool(false);
    }

    fn write_item_stack_template(payload: &mut Encoder, item_id: i32, count: i32) {
        payload.write_var_i32(item_id);
        payload.write_var_i32(count);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
    }

    fn write_empty_block_predicate(payload: &mut Encoder) {
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
    }

    fn write_attribute_modifier_entry(
        payload: &mut Encoder,
        id: &str,
        display_type: i32,
        display_text: Option<&str>,
    ) {
        payload.write_var_i32(7);
        payload.write_string(id);
        payload.write_f64(1.5);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(display_type);
        if let Some(text) = display_text {
            payload.write_bytes(&nbt_string_root(text));
        }
    }

    fn write_holder_set_tag(payload: &mut Encoder, tag: &str) {
        payload.write_var_i32(0);
        payload.write_string(tag);
    }

    fn write_holder_set_ids(payload: &mut Encoder, ids: &[i32]) {
        payload.write_var_i32(ids.len() as i32 + 1);
        for id in ids {
            payload.write_var_i32(*id);
        }
    }

    fn write_optional_direct_sound_event(payload: &mut Encoder, id: Option<&str>) {
        match id {
            Some(id) => {
                payload.write_bool(true);
                write_direct_sound_event(payload, id, None);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_kinetic_weapon_condition(
        payload: &mut Encoder,
        max_duration_ticks: i32,
        min_speed: f32,
        min_relative_speed: f32,
    ) {
        payload.write_var_i32(max_duration_ticks);
        payload.write_f32(min_speed);
        payload.write_f32(min_relative_speed);
    }

    fn write_direct_sound_event(payload: &mut Encoder, id: &str, fixed_range: Option<f32>) {
        payload.write_var_i32(0);
        payload.write_string(id);
        match fixed_range {
            Some(range) => {
                payload.write_bool(true);
                payload.write_f32(range);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
