use bbb_protocol::item_component_value_types::{ItemRaritySummary, SwingAnimationTypeSummary};
use bbb_protocol::{
    builtin_enum_component_values::{
        builtin_enum_component_value_for_name, VANILLA_BUILTIN_ENUM_COMPONENTS,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
    },
    tropical_fish_patterns::vanilla_tropical_fish_pattern_for_name,
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::super::{compound_entries, find_entry};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogStringMapEntryState {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "codec", rename_all = "snake_case")]
pub enum DialogDataComponentValueState {
    Integer {
        value: i32,
    },
    Float {
        bits: u32,
    },
    Boolean {
        value: bool,
    },
    Unit,
    ResourceId {
        resource_id: String,
    },
    Rarity {
        value: ItemRaritySummary,
    },
    StringEnum {
        value_id: i32,
        serialized_name: String,
    },
    UseEffects {
        can_sprint: bool,
        interact_vibrations: bool,
        speed_multiplier_bits: u32,
    },
    SwingAnimation {
        animation_type: SwingAnimationTypeSummary,
        duration: i32,
    },
    BlockState {
        properties: Vec<DialogStringMapEntryState>,
    },
}

impl DialogDataComponentValueState {
    pub fn float_value(&self) -> Option<f32> {
        match self {
            Self::Float { bits } => Some(f32::from_bits(*bits)),
            _ => None,
        }
    }
}

pub(super) fn decode_supported_component_value(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
) -> Result<Option<DialogDataComponentValueState>, String> {
    let what = format!("dialog item component {resource_id}");
    let decoded = match type_id {
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 1, 99, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 1, i32::MAX, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID | VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 0, i32::MAX, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID | VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => {
            DialogDataComponentValueState::Integer {
                value: component_i32(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 0, 4, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => {
            let bits = component_f32_bits(value, &what)?;
            validate_component_float(bits, 0.0, 1.0, &what)?;
            DialogDataComponentValueState::Float { bits }
        }
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => {
            let bits = component_f32_bits(value, &what)?;
            validate_component_float(bits, 0.0, f32::MAX, &what)?;
            DialogDataComponentValueState::Float { bits }
        }
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            DialogDataComponentValueState::Boolean {
                value: component_bool(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID
        | VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID
        | VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => {
            compound_entries(value, &what)?;
            DialogDataComponentValueState::Unit
        }
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID
        | VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID
        | VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => {
            let NbtSummaryValue::String(raw) = value else {
                return Err(format!("{what} is not a resource id"));
            };
            let resource_id = normalize_codec_identifier(raw)
                .ok_or_else(|| format!("invalid {what} resource id {raw}"))?;
            DialogDataComponentValueState::ResourceId { resource_id }
        }
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => {
            let NbtSummaryValue::String(raw) = value else {
                return Err(format!("{what} is not a rarity name"));
            };
            let value = ItemRaritySummary::from_serialized_name(raw)
                .ok_or_else(|| format!("unknown {what} rarity {raw}"))?;
            DialogDataComponentValueState::Rarity { value }
        }
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => decode_use_effects(value, &what)?,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => decode_swing_animation(value, &what)?,
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => decode_block_state(value, &what)?,
        _ => {
            return decode_string_enum_component_value(type_id, &what, value);
        }
    };
    Ok(Some(decoded))
}

fn decode_string_enum_component_value(
    type_id: i32,
    what: &str,
    value: &NbtSummaryValue,
) -> Result<Option<DialogDataComponentValueState>, String> {
    let is_builtin_enum = VANILLA_BUILTIN_ENUM_COMPONENTS
        .iter()
        .any(|entry| entry.type_id == type_id);
    if !is_builtin_enum && type_id != VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID {
        return Ok(None);
    }
    let NbtSummaryValue::String(serialized_name) = value else {
        return Err(format!("{what} is not an enum name"));
    };
    let value_id = if type_id == VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID {
        vanilla_tropical_fish_pattern_for_name(serialized_name)
            .map(|entry| entry.packed_id)
            .ok_or_else(|| format!("unknown {what} enum value {serialized_name}"))?
    } else {
        builtin_enum_component_value_for_name(type_id, serialized_name)
            .map(|entry| entry.id)
            .ok_or_else(|| format!("unknown {what} enum value {serialized_name}"))?
    };
    Ok(Some(DialogDataComponentValueState::StringEnum {
        value_id,
        serialized_name: serialized_name.clone(),
    }))
}

fn decode_use_effects(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let can_sprint = optional_component_bool(entries, "can_sprint", false, what)?;
    let interact_vibrations = optional_component_bool(entries, "interact_vibrations", true, what)?;
    let speed_multiplier_bits = match find_entry(entries, "speed_multiplier") {
        Some(value) => component_f32_bits(value, &format!("{what} field speed_multiplier"))?,
        None => 0.2f32.to_bits(),
    };
    validate_component_float(
        speed_multiplier_bits,
        0.0,
        1.0,
        &format!("{what} field speed_multiplier"),
    )?;
    Ok(DialogDataComponentValueState::UseEffects {
        can_sprint,
        interact_vibrations,
        speed_multiplier_bits,
    })
}

fn decode_swing_animation(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let animation_type = match find_entry(entries, "type") {
        None => SwingAnimationTypeSummary::Whack,
        Some(NbtSummaryValue::String(serialized_name)) => {
            SwingAnimationTypeSummary::from_serialized_name(serialized_name)
                .ok_or_else(|| format!("unknown {what} animation type {serialized_name}"))?
        }
        Some(_) => return Err(format!("{what} field type is not an enum name")),
    };
    let duration = match find_entry(entries, "duration") {
        Some(value) => component_i32(value, &format!("{what} field duration"))?,
        None => 6,
    };
    validate_i32_range(duration, 1, i32::MAX, &format!("{what} field duration"))?;
    Ok(DialogDataComponentValueState::SwingAnimation {
        animation_type,
        duration,
    })
}

fn decode_block_state(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let properties = entries
        .iter()
        .map(|entry| {
            let NbtSummaryValue::String(value) = &entry.value else {
                return Err(format!("{what} property {} is not a string", entry.name));
            };
            Ok(DialogStringMapEntryState {
                key: entry.name.clone(),
                value: value.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogDataComponentValueState::BlockState { properties })
}

fn optional_component_bool(
    entries: &[NbtSummaryEntry],
    key: &str,
    default: bool,
    what: &str,
) -> Result<bool, String> {
    find_entry(entries, key).map_or(Ok(default), |value| {
        component_bool(value, &format!("{what} field {key}"))
    })
}

fn component_i32(value: &NbtSummaryValue, what: &str) -> Result<i32, String> {
    match value {
        NbtSummaryValue::Byte(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Short(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Int(value) => Ok(*value),
        NbtSummaryValue::Long(value) => Ok(*value as i32),
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits) as i32),
        NbtSummaryValue::Double(bits) => Ok(f64::from_bits(*bits) as i32),
        _ => Err(format!("{what} is not numeric")),
    }
}

fn component_f32_bits(value: &NbtSummaryValue, what: &str) -> Result<u32, String> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => return Ok(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return Err(format!("{what} is not numeric")),
    };
    Ok(value.to_bits())
}

fn component_bool(value: &NbtSummaryValue, what: &str) -> Result<bool, String> {
    let value = match value {
        NbtSummaryValue::Byte(value) => *value,
        NbtSummaryValue::Short(value) => *value as i8,
        NbtSummaryValue::Int(value) => *value as i8,
        NbtSummaryValue::Long(value) => *value as i8,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) as i32 as i8,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as i32 as i8,
        _ => return Err(format!("{what} is not a boolean")),
    };
    Ok(value != 0)
}

fn validate_i32_range(value: i32, min: i32, max: i32, what: &str) -> Result<(), String> {
    if !(min..=max).contains(&value) {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

fn validate_component_float(bits: u32, min: f32, max: f32, what: &str) -> Result<(), String> {
    let value = f32::from_bits(bits);
    let negative_zero_below_min = min == 0.0 && bits == (-0.0f32).to_bits();
    if value.is_nan() || negative_zero_below_min || value < min || value > max {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
    };

    use super::*;

    #[test]
    fn decodes_primitive_component_value_codecs() {
        for (type_id, resource_id, input, expected) in [
            (
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                "minecraft:max_stack_size",
                NbtSummaryValue::Byte(99),
                DialogDataComponentValueState::Integer { value: 99 },
            ),
            (
                VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                "minecraft:max_damage",
                NbtSummaryValue::Long(10),
                DialogDataComponentValueState::Integer { value: 10 },
            ),
            (
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                "minecraft:damage",
                NbtSummaryValue::Double(3.75f64.to_bits()),
                DialogDataComponentValueState::Integer { value: 3 },
            ),
            (
                VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
                "minecraft:repair_cost",
                NbtSummaryValue::Short(7),
                DialogDataComponentValueState::Integer { value: 7 },
            ),
            (
                VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
                "minecraft:enchantment_glint_override",
                NbtSummaryValue::Int(2),
                DialogDataComponentValueState::Boolean { value: true },
            ),
        ] {
            assert_eq!(
                decoded(type_id, resource_id, &input).unwrap(),
                Some(expected)
            );
        }

        for (type_id, resource_id) in [
            (
                VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                "minecraft:unbreakable",
            ),
            (
                VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
                "minecraft:intangible_projectile",
            ),
            (VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, "minecraft:glider"),
        ] {
            assert_eq!(
                decoded(type_id, resource_id, &compound(Vec::new())).unwrap(),
                Some(DialogDataComponentValueState::Unit)
            );
        }

        let minimum = decoded(
            VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
            "minecraft:minimum_attack_charge",
            &NbtSummaryValue::Double(0.5f64.to_bits()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(minimum.float_value(), Some(0.5));
        let scale = decoded(
            VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            "minecraft:potion_duration_scale",
            &NbtSummaryValue::Double(1.25f64.to_bits()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(scale.float_value(), Some(1.25));

        for (type_id, resource_id, raw, normalized) in [
            (
                VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
                "minecraft:item_model",
                "stone",
                "minecraft:stone",
            ),
            (
                VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
                "minecraft:tooltip_style",
                ":compact",
                "minecraft:compact",
            ),
            (
                VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
                "minecraft:note_block_sound",
                "block.note_block.harp",
                "minecraft:block.note_block.harp",
            ),
        ] {
            assert_eq!(
                decoded(
                    type_id,
                    resource_id,
                    &NbtSummaryValue::String(raw.to_string())
                )
                .unwrap(),
                Some(DialogDataComponentValueState::ResourceId {
                    resource_id: normalized.to_string(),
                })
            );
        }
        assert_eq!(
            decoded(
                VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                "minecraft:rarity",
                &NbtSummaryValue::String("rare".to_string()),
            )
            .unwrap(),
            Some(DialogDataComponentValueState::Rarity {
                value: ItemRaritySummary::Rare,
            })
        );
        for (type_id, resource_id) in [
            (
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                "minecraft:custom_name",
            ),
            (
                VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
                "minecraft:item_name",
            ),
            (VANILLA_DATA_COMPONENT_LORE_TYPE_ID, "minecraft:lore"),
        ] {
            assert_eq!(
                decoded(type_id, resource_id, &NbtSummaryValue::List(Vec::new())).unwrap(),
                None
            );
        }
    }

    #[test]
    fn rejects_invalid_primitive_component_value_codecs() {
        for (type_id, resource_id, value, expected) in [
            (
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                "minecraft:max_stack_size",
                NbtSummaryValue::Int(0),
                "outside 1..=99",
            ),
            (
                VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                "minecraft:max_damage",
                NbtSummaryValue::Int(0),
                "outside 1..=",
            ),
            (
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                "minecraft:damage",
                NbtSummaryValue::Int(-1),
                "outside 0..=",
            ),
            (
                VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
                "minecraft:minimum_attack_charge",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
                "outside 0..=1",
            ),
            (
                VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                "minecraft:potion_duration_scale",
                NbtSummaryValue::Float(f32::INFINITY.to_bits()),
                "outside 0..=",
            ),
            (
                VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
                "minecraft:enchantment_glint_override",
                NbtSummaryValue::String("true".to_string()),
                "is not a boolean",
            ),
            (
                VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                "minecraft:unbreakable",
                NbtSummaryValue::Byte(1),
                "is not a compound",
            ),
            (
                VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
                "minecraft:item_model",
                NbtSummaryValue::String("Invalid".to_string()),
                "invalid dialog item component",
            ),
            (
                VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                "minecraft:rarity",
                NbtSummaryValue::String("legendary".to_string()),
                "unknown dialog item component",
            ),
        ] {
            let error = decoded(type_id, resource_id, &value).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    #[test]
    fn decodes_all_string_enum_component_value_codecs() {
        let expected = [
            ("dye", "red", 14),
            ("base_color", "blue", 11),
            ("wolf/collar", "lime", 5),
            ("fox/variant", "snow", 1),
            ("salmon/size", "large", 2),
            ("parrot/variant", "gray", 4),
            ("tropical_fish/pattern", "spotty", 1280),
            ("tropical_fish/base_color", "cyan", 9),
            ("tropical_fish/pattern_color", "purple", 10),
            ("mooshroom/variant", "brown", 1),
            ("rabbit/variant", "evil", 99),
            ("horse/variant", "dark_brown", 6),
            ("llama/variant", "gray", 3),
            ("axolotl/variant", "blue", 4),
            ("cat/collar", "orange", 1),
            ("sheep/color", "white", 0),
            ("shulker/color", "black", 15),
        ];

        for (component, serialized_name, value_id) in expected {
            let resource_id = format!("minecraft:{component}");
            let type_id =
                bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id(
                    &resource_id,
                )
                .unwrap()
                .type_id;
            assert_eq!(
                decoded(
                    type_id,
                    &resource_id,
                    &NbtSummaryValue::String(serialized_name.to_string()),
                )
                .unwrap(),
                Some(DialogDataComponentValueState::StringEnum {
                    value_id,
                    serialized_name: serialized_name.to_string(),
                })
            );
        }
    }

    #[test]
    fn rejects_invalid_string_enum_component_value_codecs() {
        for (type_id, resource_id, value, expected) in [
            (
                VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                "minecraft:dye",
                NbtSummaryValue::String("scarlet".to_string()),
                "unknown dialog item component minecraft:dye enum value scarlet",
            ),
            (
                VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                "minecraft:fox/variant",
                NbtSummaryValue::Int(1),
                "is not an enum name",
            ),
            (
                VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
                "minecraft:tropical_fish/pattern",
                NbtSummaryValue::String("missing".to_string()),
                "unknown dialog item component minecraft:tropical_fish/pattern enum value missing",
            ),
        ] {
            let error = decoded(type_id, resource_id, &value).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    #[test]
    fn decodes_scalar_wrappers_flat_records_and_string_map() {
        for (type_id, resource_id, input, expected) in [
            (
                VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
                "minecraft:map_color",
                NbtSummaryValue::Long(0x1_0012_3456),
                DialogDataComponentValueState::Integer { value: 0x0012_3456 },
            ),
            (
                VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                "minecraft:map_id",
                NbtSummaryValue::Double(12.75f64.to_bits()),
                DialogDataComponentValueState::Integer { value: 12 },
            ),
            (
                VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
                "minecraft:ominous_bottle_amplifier",
                NbtSummaryValue::Byte(4),
                DialogDataComponentValueState::Integer { value: 4 },
            ),
        ] {
            assert_eq!(
                decoded(type_id, resource_id, &input).unwrap(),
                Some(expected)
            );
        }

        assert_eq!(
            decoded(
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                "minecraft:use_effects",
                &compound(Vec::new()),
            )
            .unwrap(),
            Some(DialogDataComponentValueState::UseEffects {
                can_sprint: false,
                interact_vibrations: true,
                speed_multiplier_bits: 0.2f32.to_bits(),
            })
        );
        assert_eq!(
            decoded(
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                "minecraft:use_effects",
                &compound(vec![
                    value_entry("can_sprint", NbtSummaryValue::Int(2)),
                    value_entry("interact_vibrations", NbtSummaryValue::Short(256)),
                    value_entry(
                        "speed_multiplier",
                        NbtSummaryValue::Double(0.75f64.to_bits()),
                    ),
                ]),
            )
            .unwrap(),
            Some(DialogDataComponentValueState::UseEffects {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier_bits: 0.75f32.to_bits(),
            })
        );

        assert_eq!(
            decoded(
                VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                "minecraft:swing_animation",
                &compound(Vec::new()),
            )
            .unwrap(),
            Some(DialogDataComponentValueState::SwingAnimation {
                animation_type: SwingAnimationTypeSummary::Whack,
                duration: 6,
            })
        );
        assert_eq!(
            decoded(
                VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                "minecraft:swing_animation",
                &compound(vec![
                    string_entry("type", "stab"),
                    value_entry("duration", NbtSummaryValue::Long(9)),
                ]),
            )
            .unwrap(),
            Some(DialogDataComponentValueState::SwingAnimation {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 9,
            })
        );

        assert_eq!(
            decoded(
                VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
                "minecraft:block_state",
                &compound(vec![
                    string_entry("axis", "x"),
                    string_entry("waterlogged", "false"),
                ]),
            )
            .unwrap(),
            Some(DialogDataComponentValueState::BlockState {
                properties: vec![
                    DialogStringMapEntryState {
                        key: "axis".to_string(),
                        value: "x".to_string(),
                    },
                    DialogStringMapEntryState {
                        key: "waterlogged".to_string(),
                        value: "false".to_string(),
                    },
                ],
            })
        );
    }

    #[test]
    fn rejects_invalid_scalar_wrapper_and_flat_record_codecs() {
        for (type_id, resource_id, value, expected) in [
            (
                VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
                "minecraft:ominous_bottle_amplifier",
                NbtSummaryValue::Int(5),
                "outside 0..=4",
            ),
            (
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                "minecraft:use_effects",
                NbtSummaryValue::Byte(1),
                "is not a compound",
            ),
            (
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                "minecraft:use_effects",
                compound(vec![value_entry(
                    "speed_multiplier",
                    NbtSummaryValue::Float((-0.0f32).to_bits()),
                )]),
                "outside 0..=1",
            ),
            (
                VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                "minecraft:swing_animation",
                compound(vec![string_entry("type", "slash")]),
                "unknown dialog item component minecraft:swing_animation animation type slash",
            ),
            (
                VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                "minecraft:swing_animation",
                compound(vec![value_entry("duration", NbtSummaryValue::Int(0))]),
                "outside 1..=",
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
                "minecraft:block_state",
                compound(vec![value_entry("axis", NbtSummaryValue::Int(1))]),
                "property axis is not a string",
            ),
        ] {
            let error = decoded(type_id, resource_id, &value).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    fn decoded(
        type_id: i32,
        resource_id: &str,
        value: &NbtSummaryValue,
    ) -> Result<Option<DialogDataComponentValueState>, String> {
        decode_supported_component_value(type_id, resource_id, value)
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
        value_entry(name, NbtSummaryValue::String(value.to_string()))
    }
}
