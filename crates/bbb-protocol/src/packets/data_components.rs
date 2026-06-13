use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary_from_decoder,
};

pub(crate) const MAX_DATA_COMPONENT_PATCH_ENTRIES: usize = 1024;
pub(crate) const MAX_DATA_COMPONENT_PREDICATE_ENTRIES: usize = 1024;
const MAX_IDENTIFIER_CHARS: usize = 32767;
const MAX_DATA_COMPONENT_LIST_ITEMS: usize = 4096;
const MAX_LORE_LINES: usize = 256;
const MAX_STRING_CHARS: usize = 32767;

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
        // 26.1 DataComponents: max_stack_size, max_damage, damage, repair_cost,
        // additional_trade_cost, map_id, ominous_bottle_amplifier.
        1 | 2 | 3 | 19 | 41 | 46 | 63 => {
            decoder.read_var_i32()?;
        }
        // unbreakable, creative_slot_lock, glider use Unit.STREAM_CODEC.
        4 | 20 | 34 => {}
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
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "unsupported data component type id {other}"
            )))
        }
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

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
