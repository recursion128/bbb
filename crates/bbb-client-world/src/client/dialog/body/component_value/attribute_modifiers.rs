use bbb_protocol::{
    attribute_modifier_types::{
        attribute_modifier_display_name, attribute_modifier_operation_name,
    },
    attribute_types::vanilla_attribute_type,
    equipment_slots::vanilla_equipment_slot_group,
    packets::AttributeModifierSummary,
};
use serde::{Deserialize, Serialize};

use super::super::super::{decode_component, DialogDecodeContext};
use super::holder_set::DialogRegistryHolderReferenceState;
use super::DialogTextState;

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

pub(super) fn project_attribute_modifiers(
    modifiers: &[AttributeModifierSummary],
    context: DialogDecodeContext<'_>,
) -> Result<Vec<DialogAttributeModifierState>, String> {
    let context = DialogDecodeContext {
        prepare_item_summary: false,
        ..context
    };
    modifiers
        .iter()
        .map(|modifier| {
            let attribute = vanilla_attribute_type(modifier.attribute_id).unwrap();
            let slot = vanilla_equipment_slot_group(modifier.slot_id).unwrap();
            Ok(DialogAttributeModifierState {
                attribute: DialogRegistryHolderReferenceState {
                    holder_id: attribute.id,
                    resource_id: attribute.resource_id.to_string(),
                },
                modifier_id: modifier.modifier_id.clone(),
                amount_bits: modifier.amount_bits,
                operation_id: modifier.operation_id,
                operation: attribute_modifier_operation_name(modifier.operation_id)
                    .unwrap()
                    .to_string(),
                slot_group_id: slot.id,
                slot_group: slot.serialized_name.to_string(),
                display: DialogAttributeModifierDisplayState {
                    type_id: modifier.display_id,
                    serialized_name: attribute_modifier_display_name(modifier.display_id)
                        .unwrap()
                        .to_string(),
                    value: modifier
                        .display_text_nbt
                        .as_ref()
                        .map(|value| decode_component(value, "attribute modifier display", context))
                        .transpose()?,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
