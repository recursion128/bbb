use serde::{Deserialize, Serialize};

use super::read_resource_location;
use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::{decode_component_summary_from_decoder, skip_nbt_tag_from_decoder},
};

pub(crate) const MAX_DATA_COMPONENT_PATCH_ENTRIES: usize = 1024;
pub(crate) const MAX_DATA_COMPONENT_PREDICATE_ENTRIES: usize = 1024;
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
    #[serde(default)]
    pub max_stack_size: Option<i32>,
    #[serde(default)]
    pub max_damage: Option<i32>,
    #[serde(default)]
    pub damage: Option<i32>,
    #[serde(default)]
    pub unbreakable: bool,
    #[serde(default)]
    pub use_cooldown_ticks: Option<i32>,
    #[serde(default)]
    pub use_cooldown_group: Option<String>,
    #[serde(default)]
    pub custom_model_data_colors: Vec<i32>,
    #[serde(default)]
    pub dyed_color: Option<i32>,
    #[serde(default)]
    pub map_color: Option<i32>,
    #[serde(default)]
    pub potion_custom_color: Option<i32>,
    #[serde(default)]
    pub firework_explosion_colors: Vec<i32>,
    #[serde(default)]
    pub bundle_contents_items: Vec<ItemStackTemplateSummary>,
    #[serde(default)]
    pub bundle_contents_item_count: Option<usize>,
    #[serde(default)]
    pub enchantments: Vec<ItemEnchantmentSummary>,
    #[serde(default)]
    pub map_id: Option<i32>,
    #[serde(default)]
    pub map_post_processing: Option<MapPostProcessingSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemEnchantmentSummary {
    pub holder_id: i32,
    pub level: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapPostProcessingSummary {
    Lock,
    Scale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStackTemplateSummary {
    pub item_id: i32,
    pub count: i32,
    pub component_patch: DataComponentPatchSummary,
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

    let mut summary = decode_typed_data_component_patch_summary(decoder, added)?;
    let mut removed_type_ids = Vec::with_capacity(removed);
    for _ in 0..removed {
        removed_type_ids.push(decoder.read_var_i32()?);
    }

    summary.added = added;
    summary.removed_type_ids = removed_type_ids;
    Ok(summary)
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

fn decode_typed_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
    count: usize,
) -> Result<DataComponentPatchSummary> {
    let mut summary = DataComponentPatchSummary {
        added_type_ids: Vec::with_capacity(count),
        ..DataComponentPatchSummary::default()
    };
    for _ in 0..count {
        let type_id = decoder.read_var_i32()?;
        match type_id {
            1 => {
                summary.max_stack_size = Some(decoder.read_var_i32()?);
            }
            2 => {
                summary.max_damage = Some(decoder.read_var_i32()?);
            }
            3 => {
                summary.damage = Some(decoder.read_var_i32()?);
            }
            4 => {
                summary.unbreakable = true;
            }
            26 => {
                let cooldown = decode_use_cooldown_summary(decoder)?;
                summary.use_cooldown_ticks = Some(cooldown.ticks);
                summary.use_cooldown_group = cooldown.cooldown_group;
            }
            17 => {
                summary.custom_model_data_colors = decode_custom_model_data(decoder)?;
            }
            44 => {
                summary.dyed_color = Some(decoder.read_i32()?);
            }
            45 => {
                summary.map_color = Some(decoder.read_i32()?);
            }
            50 => {
                summary.bundle_contents_items =
                    decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
                summary.bundle_contents_item_count = Some(summary.bundle_contents_items.len());
            }
            51 => {
                summary.potion_custom_color = decode_potion_contents(decoder)?;
            }
            68 => {
                summary.firework_explosion_colors = decode_firework_explosion(decoder)?;
            }
            13 => {
                summary.enchantments = decode_varint_map(decoder)?;
            }
            41 => {
                summary.map_id = Some(decoder.read_var_i32()?);
            }
            48 => {
                summary.map_post_processing = Some(decode_map_post_processing(decoder)?);
            }
            _ => decode_data_component_value(decoder, type_id)?,
        }
        summary.added_type_ids.push(type_id);
    }
    Ok(summary)
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
            decode_identifier(decoder)?;
        }
        // rarity, dye, animal variant enums, collars,
        // tropical fish colors, sheep_color, shulker_color.
        12 | 43 | 73 | 84 | 85 | 86 | 87 | 88 | 89 | 90 | 91 | 92 | 101 | 103 | 104 | 107 | 108
        | 109 => {
            decoder.read_var_i32()?;
        }
        // map_post_processing uses ByIdMap.OutOfBoundsStrategy.ZERO.
        48 => {
            let _ = decode_map_post_processing(decoder)?;
        }
        // enchantments and stored_enchantments: map(enchantment holder id -> level).
        13 | 42 => {
            let _ = decode_varint_map(decoder)?;
        }
        // can_place_on and can_break.
        14 | 15 => decode_adventure_mode_predicate(decoder)?,
        // attribute_modifiers.
        16 => decode_attribute_modifiers(decoder)?,
        // custom_model_data: floats, flags, strings, colors.
        17 => {
            decode_custom_model_data(decoder)?;
        }
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
        49 | 50 => {
            let _ = decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
        }
        // potion_contents.
        51 => {
            decode_potion_contents(decoder)?;
        }
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
        68 => {
            decode_firework_explosion(decoder)?;
        }
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
        decode_identifier(decoder)?;
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
    read_resource_location(decoder)?;
    Ok(())
}

fn decode_optional_identifier(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        decode_identifier(decoder)?;
    }
    Ok(())
}

fn decode_optional_identifier_value(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return read_resource_location(decoder).map(Some);
    }
    Ok(None)
}

fn decode_optional_i32_value(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_i32()?));
    }
    Ok(None)
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

fn decode_varint_map(decoder: &mut Decoder<'_>) -> Result<Vec<ItemEnchantmentSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(ItemEnchantmentSummary {
            holder_id: decoder.read_var_i32()?,
            level: decoder.read_var_i32()?,
        });
    }
    Ok(entries)
}

fn decode_map_post_processing(decoder: &mut Decoder<'_>) -> Result<MapPostProcessingSummary> {
    Ok(match decoder.read_var_i32()? {
        1 => MapPostProcessingSummary::Scale,
        _ => MapPostProcessingSummary::Lock,
    })
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

fn decode_custom_model_data(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
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
    let mut color_values = Vec::with_capacity(colors);
    for _ in 0..colors {
        color_values.push(decoder.read_i32()?);
    }

    Ok(color_values)
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
    let _ = decode_item_stack_template(decoder)?;
    Ok(())
}

fn decode_item_stack_template(decoder: &mut Decoder<'_>) -> Result<ItemStackTemplateSummary> {
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
    let component_patch = decode_data_component_patch_summary(decoder)?;
    Ok(ItemStackTemplateSummary {
        item_id,
        count,
        component_patch,
    })
}

fn decode_item_stack_template_list(
    decoder: &mut Decoder<'_>,
    max: usize,
) -> Result<Vec<ItemStackTemplateSummary>> {
    let count = read_bounded_len(decoder, max)?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(decode_item_stack_template(decoder)?);
    }
    Ok(items)
}

fn decode_optional_item_stack_template(decoder: &mut Decoder<'_>) -> Result<()> {
    if decoder.read_bool()? {
        let _ = decode_item_stack_template(decoder)?;
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
    decode_optional_identifier(decoder)
}

struct UseCooldownSummary {
    ticks: i32,
    cooldown_group: Option<String>,
}

fn decode_use_cooldown_summary(decoder: &mut Decoder<'_>) -> Result<UseCooldownSummary> {
    let seconds = decoder.read_f32()?;
    Ok(UseCooldownSummary {
        ticks: (seconds * 20.0) as i32,
        cooldown_group: decode_optional_identifier_value(decoder)?,
    })
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
    decode_optional_identifier(decoder)?;
    decode_optional_identifier(decoder)?;
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

fn decode_potion_contents(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    if decoder.read_bool()? {
        decode_holder_registry(decoder)?;
    }
    let custom_color = decode_optional_i32_value(decoder)?;
    let effects = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    for _ in 0..effects {
        decode_mob_effect_instance(decoder)?;
    }
    decode_optional_string(decoder, MAX_STRING_CHARS)?;
    Ok(custom_color)
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

fn decode_firework_explosion(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
    decoder.read_var_i32()?;
    let colors = decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    decoder.read_bool()?;
    decoder.read_bool()?;
    Ok(colors)
}

fn decode_int_list(decoder: &mut Decoder<'_>, max: usize) -> Result<Vec<i32>> {
    let count = read_bounded_len(decoder, max)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decoder.read_i32()?);
    }
    Ok(values)
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
        payload.write_var_i32(8);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(2);
        payload.write_var_i32(432);
        payload.write_var_i32(3);
        payload.write_var_i32(431);
        payload.write_var_i32(4);
        payload.write_var_i32(6);
        payload.write_bytes(&nbt_string_root("Named"));
        payload.write_var_i32(10);
        payload.write_string("minecraft:diamond_sword");
        payload.write_var_i32(21);
        payload.write_bool(true);
        payload.write_var_i32(26);
        payload.write_f32(1.5);
        payload.write_bool(true);
        payload.write_string("minecraft:ender_pearl");
        payload.write_var_i32(3);
        payload.write_var_i32(12);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 8,
                added_type_ids: vec![1, 2, 3, 4, 6, 10, 21, 26],
                removed_type_ids: vec![3, 12],
                max_stack_size: Some(64),
                max_damage: Some(432),
                damage: Some(431),
                unbreakable: true,
                use_cooldown_ticks: Some(30),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_enchantments_component_summary_in_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(13);
        payload.write_var_i32(3);
        payload.write_var_i32(37);
        payload.write_var_i32(4);
        payload.write_var_i32(12);
        payload.write_var_i32(1);
        payload.write_var_i32(300);
        payload.write_var_i32(5);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![13],
                removed_type_ids: Vec::new(),
                enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 37,
                        level: 4,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 300,
                        level: 5,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_map_component_summary_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);

        payload.write_var_i32(41);
        payload.write_var_i32(123);
        payload.write_var_i32(48);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![41, 48],
                removed_type_ids: Vec::new(),
                map_id: Some(123),
                map_post_processing: Some(MapPostProcessingSummary::Scale),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_map_post_processing_out_of_bounds_as_lock() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(48);
        payload.write_var_i32(99);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch.map_post_processing,
            Some(MapPostProcessingSummary::Lock)
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
                enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 5,
                        level: 3,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 9,
                        level: 1,
                    },
                ],
                custom_model_data_colors: vec![0x112233, 0x445566],
                ..DataComponentPatchSummary::default()
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
                ..DataComponentPatchSummary::default()
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
                ..DataComponentPatchSummary::default()
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
                ..DataComponentPatchSummary::default()
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
                ..DataComponentPatchSummary::default()
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
                ..DataComponentPatchSummary::default()
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
                dyed_color: Some(0x112233),
                map_color: Some(0x445566),
                use_cooldown_ticks: Some(25),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                potion_custom_color: Some(0x778899),
                firework_explosion_colors: vec![0x010203, 0x040506],
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 52,
                    count: 3,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                bundle_contents_item_count: Some(1),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_bundle_contents_item_count_from_component_patch() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(50);
        payload.write_var_i32(2);
        write_item_stack_template_with_patch(&mut payload, 12, 1, |payload| {
            payload.write_var_i32(1);
            payload.write_var_i32(0);
            payload.write_var_i32(44);
            payload.write_i32(0x224466);
        });
        write_item_stack_template_with_patch(&mut payload, 34, 3, |payload| {
            payload.write_var_i32(2);
            payload.write_var_i32(1);
            payload.write_var_i32(2);
            payload.write_var_i32(512);
            payload.write_var_i32(3);
            payload.write_var_i32(17);
            payload.write_var_i32(45);
        });

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![50],
                removed_type_ids: Vec::new(),
                bundle_contents_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 12,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added: 1,
                            added_type_ids: vec![44],
                            removed_type_ids: Vec::new(),
                            dyed_color: Some(0x224466),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 34,
                        count: 3,
                        component_patch: DataComponentPatchSummary {
                            added: 2,
                            added_type_ids: vec![2, 3],
                            removed_type_ids: vec![45],
                            max_damage: Some(512),
                            damage: Some(17),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                bundle_contents_item_count: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn rejects_invalid_identifier_data_component_values() {
        assert_invalid_data_component_identifier(10, |payload| {
            payload.write_string("minecraft:DiamondSword");
        });
        assert_invalid_data_component_identifier(35, |payload| {
            payload.write_string("minecraft:Tooltip");
        });
        assert_invalid_data_component_identifier(71, |payload| {
            payload.write_string("minecraft:NoteBlock");
        });
        assert_invalid_data_component_identifier(26, |payload| {
            payload.write_f32(1.0);
            payload.write_bool(true);
            payload.write_string("minecraft:EnderPearl");
        });
        assert_invalid_data_component_identifier(65, |payload| {
            write_holder_set_tag(payload, "minecraft:NoItemRequired");
        });
        assert_invalid_data_component_identifier(67, |payload| {
            payload.write_bool(true);
            payload.write_string("minecraft:Overworld");
            payload.write_i64(0);
        });
        assert_invalid_data_component_identifier(32, |payload| {
            payload.write_var_i32(5);
            write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
            payload.write_bool(true);
            payload.write_string("minecraft:Diamond");
        });
        assert_invalid_data_component_identifier(32, |payload| {
            payload.write_var_i32(5);
            write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
            payload.write_bool(false);
            payload.write_bool(true);
            payload.write_string("minecraft:Misc/Pumpkinblur");
        });
        assert_invalid_data_component_identifier(80, |payload| {
            payload.write_var_i32(0);
            payload.write_string("minecraft:Block.NoteBlock.Harp");
        });
        assert_invalid_data_component_identifier(102, |payload| {
            payload.write_var_i32(0);
            payload.write_var_i32(16);
            payload.write_var_i32(16);
            payload.write_string("minecraft:Kebab");
        });
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

    fn assert_invalid_data_component_identifier(
        type_id: i32,
        write_value: impl FnOnce(&mut Encoder),
    ) {
        let payload = single_data_component_payload(type_id, write_value);
        let mut decoder = Decoder::new(&payload);
        let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
        assert!(
            err.to_string().contains("invalid resource location"),
            "component {type_id} produced unexpected error: {err}"
        );
    }

    fn single_data_component_payload(
        type_id: i32,
        write_value: impl FnOnce(&mut Encoder),
    ) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(type_id);
        write_value(&mut payload);
        payload.into_inner()
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
        write_item_stack_template_with_patch(payload, item_id, count, |payload| {
            payload.write_var_i32(0);
            payload.write_var_i32(0);
        });
    }

    fn write_item_stack_template_with_patch(
        payload: &mut Encoder,
        item_id: i32,
        count: i32,
        write_patch: impl FnOnce(&mut Encoder),
    ) {
        payload.write_var_i32(item_id);
        payload.write_var_i32(count);
        write_patch(payload);
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
