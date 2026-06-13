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
const MAX_FIREWORK_EXPLOSIONS: usize = 256;
const MAX_LORE_LINES: usize = 256;
const MAX_MOB_EFFECT_DETAILS_DEPTH: usize = 16;
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
        // recipes, lock, and container_loot.
        0 | 22 | 47 | 57 | 66 | 78 | 79 => skip_nbt_tag_from_decoder(decoder)?,
        // 26.1 DataComponents: max_stack_size, max_damage, damage, repair_cost,
        // additional_trade_cost, map_id, ominous_bottle_amplifier, enchantable.
        1 | 2 | 3 | 19 | 31 | 41 | 46 | 63 => {
            decoder.read_var_i32()?;
        }
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
        // rarity, dye, map_post_processing, base_color, wolf/cat collars,
        // tropical fish colors, sheep_color, shulker_color.
        12 | 43 | 48 | 73 | 84 | 89 | 90 | 107 | 108 | 109 => {
            decoder.read_var_i32()?;
        }
        // enchantments and stored_enchantments: map(enchantment holder id -> level).
        13 | 42 => decode_varint_map(decoder)?,
        // custom_model_data: floats, flags, strings, colors.
        17 => decode_custom_model_data(decoder)?,
        // tooltip_display: bool + collection of data component type ids.
        18 => decode_tooltip_display(decoder)?,
        // enchantment_glint_override.
        21 => {
            decoder.read_bool()?;
        }
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
        // swing_animation.
        40 => decode_swing_animation(decoder)?,
        // dyed_color and map_color.
        44 | 45 => {
            decoder.read_i32()?;
        }
        // potion_contents.
        51 => decode_potion_contents(decoder)?,
        // suspicious_stew_effects.
        53 => decode_suspicious_stew_effects(decoder)?,
        // writable_book_content and written_book_content.
        54 => decode_writable_book_content(decoder)?,
        55 => decode_written_book_content(decoder)?,
        // trim.
        56 => decode_armor_trim(decoder)?,
        // instrument, trim material, jukebox playable, break sound, painting variant.
        61 => decode_instrument_component(decoder)?,
        62 => decode_trim_material_holder(decoder)?,
        64 => decode_jukebox_playable(decoder)?,
        80 => decode_sound_event_holder(decoder)?,
        102 => decode_painting_variant_holder(decoder)?,
        // firework_explosion and fireworks.
        68 => decode_firework_explosion(decoder)?,
        69 => decode_fireworks(decoder)?,
        // block_state.
        76 => decode_string_map(decoder, MAX_BLOCK_STATE_PROPERTIES)?,
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

fn decode_armor_trim(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_trim_material_holder(decoder)?;
    decode_trim_pattern_holder(decoder)
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
    fn decodes_additional_item_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [
            0, 26, 28, 29, 30, 40, 44, 45, 51, 53, 54, 55, 56, 61, 64, 68, 69, 76, 80, 102,
        ];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(0);
        payload.write_bytes(&empty_nbt_compound_root());

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

        payload.write_var_i32(40);
        payload.write_var_i32(0);
        payload.write_var_i32(6);

        payload.write_var_i32(44);
        payload.write_i32(0x112233);
        payload.write_var_i32(45);
        payload.write_i32(0x445566);

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
        payload.write_var_i32(75);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported data component type id 75"));
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
