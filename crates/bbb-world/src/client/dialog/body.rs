use bbb_protocol::item_component_value_types::ItemRaritySummary;
use bbb_protocol::{
    builtin_enum_component_values::{
        builtin_enum_component_value_for_name, VANILLA_BUILTIN_ENUM_COMPONENTS,
    },
    data_component_types::{
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
    },
    item_types::vanilla_item_type_for_resource_id,
    tropical_fish_patterns::vanilla_tropical_fish_pattern_for_name,
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::{
    compound_entries, decode_component, find_entry, optional_bool, optional_i32, required_entry,
    required_string, DialogTextState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogBodyState {
    PlainMessage {
        message: DialogPlainMessageState,
    },
    Item {
        item: DialogItemStackTemplateState,
        #[serde(default)]
        description: Option<DialogPlainMessageState>,
        show_decorations: bool,
        show_tooltip: bool,
        width: i32,
        height: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogPlainMessageState {
    pub contents: DialogTextState,
    pub width: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogItemStackTemplateState {
    pub resource_id: String,
    pub count: i32,
    #[serde(default)]
    pub components: Option<NbtSummaryValue>,
    #[serde(default)]
    pub component_data: Option<DialogDataComponentPatchState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDataComponentPatchState {
    pub entries: Vec<DialogDataComponentPatchEntryState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDataComponentPatchEntryState {
    pub type_id: i32,
    pub resource_id: String,
    pub removed: bool,
    #[serde(default)]
    pub value: Option<NbtSummaryValue>,
    #[serde(default)]
    pub value_data: Option<DialogDataComponentValueState>,
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
}

impl DialogDataComponentValueState {
    pub fn float_value(&self) -> Option<f32> {
        match self {
            Self::Float { bits } => Some(f32::from_bits(*bits)),
            _ => None,
        }
    }
}

pub(super) fn decode_dialog_bodies(
    values: &[NbtSummaryValue],
) -> Result<Vec<DialogBodyState>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_dialog_body(value, index))
        .collect()
}

fn decode_dialog_body(value: &NbtSummaryValue, index: usize) -> Result<DialogBodyState, String> {
    let what = format!("dialog body[{index}]");
    let entries = compound_entries(value, &what)?;
    match required_string(entries, "type", &what)? {
        "plain_message" | "minecraft:plain_message" => Ok(DialogBodyState::PlainMessage {
            message: decode_plain_message_map(entries, &what)?,
        }),
        "item" | "minecraft:item" => {
            let width = optional_i32(entries, "width", 16, "item dialog body")?;
            validate_range(width, 1, 256, "item dialog body width")?;
            let height = optional_i32(entries, "height", 16, "item dialog body")?;
            validate_range(height, 1, 256, "item dialog body height")?;
            Ok(DialogBodyState::Item {
                item: decode_item_stack_template(required_entry(entries, "item", &what)?)?,
                description: find_entry(entries, "description")
                    .map(decode_plain_message)
                    .transpose()?,
                show_decorations: optional_bool(entries, "show_decorations", true)?,
                show_tooltip: optional_bool(entries, "show_tooltip", true)?,
                width,
                height,
            })
        }
        body_type => Err(format!("unsupported dialog body type {body_type}")),
    }
}

fn decode_plain_message(value: &NbtSummaryValue) -> Result<DialogPlainMessageState, String> {
    if let NbtSummaryValue::Compound(entries) = value {
        if find_entry(entries, "contents").is_some() {
            return decode_plain_message_map(entries, "dialog plain message");
        }
    }
    Ok(DialogPlainMessageState {
        contents: decode_component(value, "dialog plain message contents")?,
        width: 200,
    })
}

fn decode_plain_message_map(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogPlainMessageState, String> {
    let width = optional_i32(entries, "width", 200, what)?;
    validate_range(width, 1, 1024, &format!("{what} width"))?;
    Ok(DialogPlainMessageState {
        contents: decode_component(
            required_entry(entries, "contents", what)?,
            "dialog plain message contents",
        )?,
        width,
    })
}

fn decode_item_stack_template(
    value: &NbtSummaryValue,
) -> Result<DialogItemStackTemplateState, String> {
    match value {
        NbtSummaryValue::String(resource_id) => Ok(DialogItemStackTemplateState {
            resource_id: decode_item_resource_id(resource_id)?,
            count: 1,
            components: None,
            component_data: None,
        }),
        NbtSummaryValue::Compound(entries) => {
            let resource_id = decode_item_resource_id(required_string(
                entries,
                "id",
                "dialog item stack template",
            )?)?;
            let count = optional_i32(entries, "count", 1, "dialog item stack template")?;
            validate_range(count, 1, 99, "dialog item stack template count")?;
            let (components, component_data) = find_entry(entries, "components")
                .map(|components| {
                    Ok::<_, String>((components.clone(), decode_data_component_patch(components)?))
                })
                .transpose()?
                .map_or((None, None), |(components, component_data)| {
                    (Some(components), Some(component_data))
                });
            Ok(DialogItemStackTemplateState {
                resource_id,
                count,
                components,
                component_data,
            })
        }
        _ => Err("dialog item stack template is neither an item id nor a compound".to_string()),
    }
}

fn decode_data_component_patch(
    value: &NbtSummaryValue,
) -> Result<DialogDataComponentPatchState, String> {
    let entries = compound_entries(value, "dialog item stack template components")?;
    let mut decoded_entries: Vec<DialogDataComponentPatchEntryState> = Vec::new();
    for entry in entries {
        let (removed, raw_resource_id) = entry
            .name
            .strip_prefix('!')
            .map_or((false, entry.name.as_str()), |resource_id| {
                (true, resource_id)
            });
        let resource_id = normalize_codec_identifier(raw_resource_id).ok_or_else(|| {
            format!("invalid dialog item component resource id {raw_resource_id}")
        })?;
        let component_type = vanilla_data_component_type_for_resource_id(&resource_id)
            .ok_or_else(|| format!("unknown dialog item component {resource_id}"))?;
        if !component_type.persistent {
            return Err(format!(
                "dialog item component {resource_id} is not persistent"
            ));
        }
        if removed && !matches!(entry.value, NbtSummaryValue::Compound(_)) {
            return Err(format!(
                "removed dialog item component {resource_id} payload is not a compound"
            ));
        }
        let value_data = if removed {
            None
        } else {
            decode_supported_component_value(component_type.type_id, &resource_id, &entry.value)?
        };
        let decoded = DialogDataComponentPatchEntryState {
            type_id: component_type.type_id,
            resource_id,
            removed,
            value: (!removed).then(|| entry.value.clone()),
            value_data,
        };
        if let Some(existing) = decoded_entries
            .iter_mut()
            .find(|existing| existing.type_id == decoded.type_id)
        {
            if existing.removed == decoded.removed {
                return Err(format!(
                    "duplicate dialog item component patch key {}",
                    decoded.resource_id
                ));
            }
            *existing = decoded;
        } else {
            decoded_entries.push(decoded);
        }
    }
    Ok(DialogDataComponentPatchState {
        entries: decoded_entries,
    })
}

fn decode_supported_component_value(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
) -> Result<Option<DialogDataComponentValueState>, String> {
    let what = format!("dialog item component {resource_id}");
    let decoded = match type_id {
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_range(value, 1, 99, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_range(value, 1, i32::MAX, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID | VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_range(value, 0, i32::MAX, &what)?;
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

fn validate_component_float(bits: u32, min: f32, max: f32, what: &str) -> Result<(), String> {
    let value = f32::from_bits(bits);
    let negative_zero_below_min = min == 0.0 && bits == (-0.0f32).to_bits();
    if value.is_nan() || negative_zero_below_min || value < min || value > max {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

fn decode_item_resource_id(raw: &str) -> Result<String, String> {
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid dialog item resource id {raw}"))?;
    if resource_id == "minecraft:air" {
        return Err("dialog item must not be minecraft:air".to_string());
    }
    vanilla_item_type_for_resource_id(&resource_id)
        .ok_or_else(|| format!("unknown dialog item {resource_id}"))?;
    Ok(resource_id)
}

fn validate_range(value: i32, min: i32, max: i32, what: &str) -> Result<(), String> {
    if !(min..=max).contains(&value) {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plain_message_contents_and_width() {
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "minecraft:plain_message"),
            value_entry(
                "contents",
                compound(vec![string_entry("translate", "dialog.example.body")]),
            ),
            value_entry("width", NbtSummaryValue::Int(320)),
        ])])
        .unwrap();

        let DialogBodyState::PlainMessage { message } = &bodies[0] else {
            panic!("expected plain message body");
        };
        assert_eq!(message.contents.text, "dialog.example.body");
        assert_eq!(message.width, 320);

        for width in [0, 1025] {
            let error = decode_dialog_bodies(&[compound(vec![
                string_entry("type", "plain_message"),
                value_entry("contents", NbtSummaryValue::String("Message".to_string())),
                value_entry("width", NbtSummaryValue::Int(width)),
            ])])
            .unwrap_err();
            assert!(error.contains("outside 1..=1024"));
        }
    }

    #[test]
    fn decodes_item_body_template_description_defaults_and_flags() {
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "minecraft:item"),
            value_entry("item", NbtSummaryValue::String("diamond".to_string())),
            value_entry(
                "description",
                NbtSummaryValue::String("A diamond".to_string()),
            ),
            value_entry("show_decorations", NbtSummaryValue::Byte(0)),
            value_entry("show_tooltip", NbtSummaryValue::Byte(0)),
            value_entry("width", NbtSummaryValue::Int(32)),
            value_entry("height", NbtSummaryValue::Int(48)),
        ])])
        .unwrap();

        let DialogBodyState::Item {
            item,
            description,
            show_decorations,
            show_tooltip,
            width,
            height,
        } = &bodies[0]
        else {
            panic!("expected item body");
        };
        assert_eq!(item.resource_id, "minecraft:diamond");
        assert_eq!(item.count, 1);
        assert_eq!(item.components, None);
        assert_eq!(item.component_data, None);
        assert_eq!(description.as_ref().unwrap().contents.text, "A diamond");
        assert_eq!(description.as_ref().unwrap().width, 200);
        assert!(!show_decorations);
        assert!(!show_tooltip);
        assert_eq!(*width, 32);
        assert_eq!(*height, 48);
    }

    #[test]
    fn decodes_compound_item_template_and_rejects_codec_ranges() {
        let components = compound(vec![value_entry(
            "minecraft:custom_name",
            NbtSummaryValue::String("Named".to_string()),
        )]);
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "item"),
            value_entry(
                "item",
                compound(vec![
                    string_entry("id", "minecraft:stone"),
                    value_entry("count", NbtSummaryValue::Int(2)),
                    value_entry("components", components.clone()),
                ]),
            ),
            value_entry(
                "description",
                compound(vec![
                    value_entry("contents", NbtSummaryValue::String("Stone".to_string())),
                    value_entry("width", NbtSummaryValue::Int(250)),
                ]),
            ),
        ])])
        .unwrap();
        let DialogBodyState::Item {
            item,
            description,
            show_decorations,
            show_tooltip,
            width,
            height,
        } = &bodies[0]
        else {
            panic!("expected item body");
        };
        assert_eq!(item.resource_id, "minecraft:stone");
        assert_eq!(item.count, 2);
        assert_eq!(item.components, Some(components));
        assert_eq!(
            item.component_data,
            Some(DialogDataComponentPatchState {
                entries: vec![DialogDataComponentPatchEntryState {
                    type_id: 6,
                    resource_id: "minecraft:custom_name".to_string(),
                    removed: false,
                    value: Some(NbtSummaryValue::String("Named".to_string())),
                    value_data: None,
                }],
            })
        );
        assert_eq!(description.as_ref().unwrap().width, 250);
        assert!(show_decorations);
        assert!(show_tooltip);
        assert_eq!(*width, 16);
        assert_eq!(*height, 16);

        for (field, value) in [("width", 0), ("height", 257)] {
            let error = decode_dialog_bodies(&[compound(vec![
                string_entry("type", "item"),
                value_entry("item", NbtSummaryValue::String("stone".to_string())),
                value_entry(field, NbtSummaryValue::Int(value)),
            ])])
            .unwrap_err();
            assert!(error.contains("outside 1..=256"));
        }
        for item in [
            NbtSummaryValue::String("minecraft:air".to_string()),
            NbtSummaryValue::String("example:missing".to_string()),
            compound(vec![
                string_entry("id", "minecraft:stone"),
                value_entry("count", NbtSummaryValue::Int(100)),
            ]),
        ] {
            assert!(decode_dialog_bodies(&[compound(vec![
                string_entry("type", "item"),
                value_entry("item", item),
            ])])
            .is_err());
        }
    }

    #[test]
    fn decodes_component_patch_removals_and_applies_last_entry() {
        let components = compound(vec![
            value_entry("custom_name", NbtSummaryValue::String("First".to_string())),
            value_entry("!minecraft:custom_name", compound(Vec::new())),
            value_entry("minecraft:repair_cost", NbtSummaryValue::Int(3)),
        ]);
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "item"),
            value_entry(
                "item",
                compound(vec![
                    string_entry("id", "stone"),
                    value_entry("components", components.clone()),
                ]),
            ),
        ])])
        .unwrap();
        let DialogBodyState::Item { item, .. } = &bodies[0] else {
            panic!("expected item body");
        };

        assert_eq!(item.components, Some(components));
        assert_eq!(item.component_data.as_ref().unwrap().entries.len(), 2);
        assert_eq!(
            item.component_data.as_ref().unwrap().entries[0],
            DialogDataComponentPatchEntryState {
                type_id: 6,
                resource_id: "minecraft:custom_name".to_string(),
                removed: true,
                value: None,
                value_data: None,
            }
        );
        assert_eq!(
            item.component_data.as_ref().unwrap().entries[1],
            DialogDataComponentPatchEntryState {
                type_id: 19,
                resource_id: "minecraft:repair_cost".to_string(),
                removed: false,
                value: Some(NbtSummaryValue::Int(3)),
                value_data: Some(DialogDataComponentValueState::Integer { value: 3 }),
            }
        );
    }

    #[test]
    fn rejects_unknown_transient_and_malformed_component_patch_entries() {
        for (name, value, expected) in [
            (
                "minecraft:missing",
                compound(Vec::new()),
                "unknown dialog item component",
            ),
            (
                "minecraft:creative_slot_lock",
                compound(Vec::new()),
                "is not persistent",
            ),
            (
                "!minecraft:custom_name",
                NbtSummaryValue::String("not a map".to_string()),
                "payload is not a compound",
            ),
        ] {
            let error = decode_dialog_bodies(&[compound(vec![
                string_entry("type", "item"),
                value_entry(
                    "item",
                    compound(vec![
                        string_entry("id", "stone"),
                        value_entry("components", compound(vec![value_entry(name, value)])),
                    ]),
                ),
            ])])
            .unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        let error = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "item"),
            value_entry(
                "item",
                compound(vec![
                    string_entry("id", "stone"),
                    value_entry(
                        "components",
                        compound(vec![
                            value_entry(
                                "custom_name",
                                NbtSummaryValue::String("First".to_string()),
                            ),
                            value_entry(
                                "minecraft:custom_name",
                                NbtSummaryValue::String("Second".to_string()),
                            ),
                        ]),
                    ),
                ]),
            ),
        ])])
        .unwrap_err();
        assert!(error.contains("duplicate dialog item component patch key"));
    }

    #[test]
    fn decodes_primitive_component_value_codecs() {
        let patch = decode_data_component_patch(&compound(vec![
            value_entry("max_stack_size", NbtSummaryValue::Byte(99)),
            value_entry("max_damage", NbtSummaryValue::Long(10)),
            value_entry("damage", NbtSummaryValue::Double(3.75f64.to_bits())),
            value_entry("unbreakable", compound(Vec::new())),
            value_entry(
                "custom_name",
                compound(vec![string_entry("translate", "item.example")]),
            ),
            value_entry(
                "minimum_attack_charge",
                NbtSummaryValue::Double(0.5f64.to_bits()),
            ),
            value_entry("item_name", NbtSummaryValue::String("Item".to_string())),
            value_entry("item_model", NbtSummaryValue::String("stone".to_string())),
            value_entry("rarity", NbtSummaryValue::String("rare".to_string())),
            value_entry("repair_cost", NbtSummaryValue::Short(7)),
            value_entry("enchantment_glint_override", NbtSummaryValue::Int(2)),
            value_entry("intangible_projectile", compound(Vec::new())),
            value_entry("glider", compound(Vec::new())),
            value_entry(
                "tooltip_style",
                NbtSummaryValue::String(":compact".to_string()),
            ),
            value_entry(
                "potion_duration_scale",
                NbtSummaryValue::Double(1.25f64.to_bits()),
            ),
            value_entry(
                "note_block_sound",
                NbtSummaryValue::String("block.note_block.harp".to_string()),
            ),
            value_entry("lore", NbtSummaryValue::List(Vec::new())),
        ]))
        .unwrap();
        let value = |resource_id: &str| {
            patch
                .entries
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .unwrap()
                .value_data
                .as_ref()
        };

        assert_eq!(
            value("minecraft:max_stack_size"),
            Some(&DialogDataComponentValueState::Integer { value: 99 })
        );
        assert_eq!(
            value("minecraft:max_damage"),
            Some(&DialogDataComponentValueState::Integer { value: 10 })
        );
        assert_eq!(
            value("minecraft:damage"),
            Some(&DialogDataComponentValueState::Integer { value: 3 })
        );
        assert_eq!(
            value("minecraft:repair_cost"),
            Some(&DialogDataComponentValueState::Integer { value: 7 })
        );
        assert_eq!(
            value("minecraft:unbreakable"),
            Some(&DialogDataComponentValueState::Unit)
        );
        assert_eq!(
            value("minecraft:intangible_projectile"),
            Some(&DialogDataComponentValueState::Unit)
        );
        assert_eq!(
            value("minecraft:glider"),
            Some(&DialogDataComponentValueState::Unit)
        );
        assert_eq!(
            value("minecraft:enchantment_glint_override"),
            Some(&DialogDataComponentValueState::Boolean { value: true })
        );
        assert_eq!(
            value("minecraft:minimum_attack_charge")
                .unwrap()
                .float_value(),
            Some(0.5)
        );
        assert_eq!(
            value("minecraft:potion_duration_scale")
                .unwrap()
                .float_value(),
            Some(1.25)
        );
        assert_eq!(
            value("minecraft:item_model"),
            Some(&DialogDataComponentValueState::ResourceId {
                resource_id: "minecraft:stone".to_string(),
            })
        );
        assert_eq!(
            value("minecraft:tooltip_style"),
            Some(&DialogDataComponentValueState::ResourceId {
                resource_id: "minecraft:compact".to_string(),
            })
        );
        assert_eq!(
            value("minecraft:note_block_sound"),
            Some(&DialogDataComponentValueState::ResourceId {
                resource_id: "minecraft:block.note_block.harp".to_string(),
            })
        );
        assert_eq!(
            value("minecraft:rarity"),
            Some(&DialogDataComponentValueState::Rarity {
                value: ItemRaritySummary::Rare,
            })
        );
        for resource_id in [
            "minecraft:custom_name",
            "minecraft:item_name",
            "minecraft:lore",
        ] {
            assert_eq!(value(resource_id), None);
        }
    }

    #[test]
    fn rejects_invalid_primitive_component_value_codecs() {
        for (name, value, expected) in [
            ("max_stack_size", NbtSummaryValue::Int(0), "outside 1..=99"),
            ("max_damage", NbtSummaryValue::Int(0), "outside 1..="),
            ("damage", NbtSummaryValue::Int(-1), "outside 0..="),
            (
                "minimum_attack_charge",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
                "outside 0..=1",
            ),
            (
                "potion_duration_scale",
                NbtSummaryValue::Float(f32::INFINITY.to_bits()),
                "outside 0..=",
            ),
            (
                "enchantment_glint_override",
                NbtSummaryValue::String("true".to_string()),
                "is not a boolean",
            ),
            ("unbreakable", NbtSummaryValue::Byte(1), "is not a compound"),
            (
                "item_model",
                NbtSummaryValue::String("Invalid".to_string()),
                "invalid dialog item component",
            ),
            (
                "rarity",
                NbtSummaryValue::String("legendary".to_string()),
                "unknown dialog item component",
            ),
        ] {
            let error =
                decode_data_component_patch(&compound(vec![value_entry(name, value)])).unwrap_err();
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
        let patch = decode_data_component_patch(&compound(
            expected
                .iter()
                .map(|(component, value, _)| {
                    value_entry(component, NbtSummaryValue::String((*value).to_string()))
                })
                .collect(),
        ))
        .unwrap();

        assert_eq!(patch.entries.len(), expected.len());
        for ((_, serialized_name, value_id), entry) in expected.iter().zip(&patch.entries) {
            assert_eq!(
                entry.value_data,
                Some(DialogDataComponentValueState::StringEnum {
                    value_id: *value_id,
                    serialized_name: (*serialized_name).to_string(),
                })
            );
        }
    }

    #[test]
    fn rejects_invalid_string_enum_component_value_codecs() {
        for (name, value, expected) in [
            (
                "dye",
                NbtSummaryValue::String("scarlet".to_string()),
                "unknown dialog item component minecraft:dye enum value scarlet",
            ),
            (
                "fox/variant",
                NbtSummaryValue::Int(1),
                "is not an enum name",
            ),
            (
                "tropical_fish/pattern",
                NbtSummaryValue::String("missing".to_string()),
                "unknown dialog item component minecraft:tropical_fish/pattern enum value missing",
            ),
        ] {
            let error =
                decode_data_component_patch(&compound(vec![value_entry(name, value)])).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
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
