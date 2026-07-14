use bbb_protocol::{
    attribute_modifier_types::{
        attribute_modifier_display_id, attribute_modifier_display_name,
        attribute_modifier_operation_id, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
    },
    attribute_types::vanilla_attribute_type_for_resource_id,
    equipment_slots::{
        vanilla_equipment_slot_group, vanilla_equipment_slot_group_for_serialized_name,
        VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
    },
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::super::super::{decode_component, DialogDecodeContext};
use super::holder_set::DialogRegistryHolderReferenceState;
use super::{component_f64_bits, find_entry, DialogTextState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogAttributeModifierDisplayState {
    pub type_id: i32,
    pub serialized_name: String,
    #[serde(default)]
    pub value: Option<DialogTextState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogAttributeModifierState {
    pub attribute: DialogRegistryHolderReferenceState,
    pub modifier_id: String,
    pub amount_bits: u64,
    pub operation_id: i32,
    pub operation: String,
    pub slot_group_id: i32,
    pub slot_group: String,
    pub display: DialogAttributeModifierDisplayState,
}

#[cfg(test)]
pub(super) fn decode_attribute_modifiers(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogAttributeModifierState>, String> {
    decode_attribute_modifiers_with_context(value, what, DialogDecodeContext::new(None, 0))
}

pub(super) fn decode_attribute_modifiers_with_context(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<Vec<DialogAttributeModifierState>, String> {
    let NbtSummaryValue::List(entries) = value else {
        return Err(format!("{what} is not a list"));
    };
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            decode_attribute_modifier(entry, &format!("{what}[{index}]"), context)
        })
        .collect()
}

fn decode_attribute_modifier(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogAttributeModifierState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let raw_attribute = required_string(entries, "type", what, "an attribute resource id")?;
    let attribute = normalize_codec_identifier(raw_attribute)
        .ok_or_else(|| format!("invalid {what} attribute resource id {raw_attribute}"))?;
    let attribute_entry = vanilla_attribute_type_for_resource_id(&attribute)
        .ok_or_else(|| format!("missing minecraft:attribute resource {attribute} for {what}"))?;

    let raw_modifier_id = required_string(entries, "id", what, "a resource id")?;
    let modifier_id = normalize_codec_identifier(raw_modifier_id)
        .ok_or_else(|| format!("invalid {what} modifier resource id {raw_modifier_id}"))?;
    let amount_bits = component_f64_bits(
        required_entry(entries, "amount", what)?,
        &format!("{what} field amount"),
    )?;

    let raw_operation = required_string(entries, "operation", what, "an operation name")?;
    let operation_id = attribute_modifier_operation_id(raw_operation)
        .ok_or_else(|| format!("unknown {what} operation {raw_operation}"))?;

    let (slot_group_id, slot_group) = match find_entry(entries, "slot") {
        None => {
            let slot = vanilla_equipment_slot_group(VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID)
                .expect("generated any equipment-slot group exists");
            (slot.id, slot.serialized_name.to_string())
        }
        Some(NbtSummaryValue::String(raw_slot)) => {
            let slot = vanilla_equipment_slot_group_for_serialized_name(raw_slot)
                .ok_or_else(|| format!("unknown {what} equipment slot group {raw_slot}"))?;
            (slot.id, slot.serialized_name.to_string())
        }
        Some(_) => {
            return Err(format!(
                "{what} field slot is not an equipment slot group name"
            ))
        }
    };

    Ok(DialogAttributeModifierState {
        attribute: DialogRegistryHolderReferenceState {
            holder_id: attribute_entry.id,
            resource_id: attribute,
        },
        modifier_id,
        amount_bits,
        operation_id,
        operation: raw_operation.to_string(),
        slot_group_id,
        slot_group,
        display: find_entry(entries, "display")
            .map(|display| decode_display(display, &format!("{what} field display"), context))
            .transpose()?
            .unwrap_or_else(default_display),
    })
}

fn decode_display(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogAttributeModifierDisplayState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let raw_type = required_string(entries, "type", what, "a display type name")?;
    let type_id = attribute_modifier_display_id(raw_type)
        .ok_or_else(|| format!("unknown {what} type {raw_type}"))?;
    let component = if type_id == VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID {
        let value = required_entry(entries, "value", what)?;
        Some(decode_component(
            value,
            &format!("{what} field value"),
            context,
        )?)
    } else {
        None
    };
    Ok(DialogAttributeModifierDisplayState {
        type_id,
        serialized_name: raw_type.to_string(),
        value: component,
    })
}

fn default_display() -> DialogAttributeModifierDisplayState {
    DialogAttributeModifierDisplayState {
        type_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        serialized_name: attribute_modifier_display_name(
            VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        )
        .expect("generated default attribute-modifier display exists")
        .to_string(),
        value: None,
    }
}

fn required_entry<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a NbtSummaryValue, String> {
    find_entry(entries, field).ok_or_else(|| format!("{what} is missing field {field}"))
}

fn required_string<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
    expected: &str,
) -> Result<&'a str, String> {
    let value = required_entry(entries, field, what)?;
    let NbtSummaryValue::String(value) = value else {
        return Err(format!("{what} field {field} is not {expected}"));
    };
    Ok(value)
}

#[cfg(test)]
mod tests;
