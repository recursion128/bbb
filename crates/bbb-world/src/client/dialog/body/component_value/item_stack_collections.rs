use bbb_protocol::NbtSummaryValue;

use crate::RegistrySet;

use super::super::{decode_item_stack_template, DialogItemStackTemplateState};
use super::{component_i32, component_list, find_entry, validate_i32_range};

const MAX_CONTAINER_SLOT_RECORDS: usize = 256;

pub(super) fn decode_item_stack_template_list(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<Vec<DialogItemStackTemplateState>, String> {
    component_list(value)
        .map_err(|_| format!("{what} is not a list"))?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            decode_item_stack_template(value, registries).map_err(|error| {
                format!("{what}[{index}] failed item stack template decoding: {error}")
            })
        })
        .collect()
}

pub(super) fn decode_item_stack_template_slots(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<Vec<Option<DialogItemStackTemplateState>>, String> {
    let values = component_list(value).map_err(|_| format!("{what} is not a list"))?;
    if values.len() > MAX_CONTAINER_SLOT_RECORDS {
        return Err(format!(
            "{what} has {} slot records, maximum is {MAX_CONTAINER_SLOT_RECORDS}",
            values.len()
        ));
    }
    let mut slots = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let entry_what = format!("{what}[{index}]");
        let NbtSummaryValue::Compound(entries) = value else {
            return Err(format!("{entry_what} is not a compound"));
        };
        let slot_value = find_entry(entries, "slot")
            .ok_or_else(|| format!("{entry_what} is missing field slot"))?;
        let slot = component_i32(slot_value, &format!("{entry_what} field slot"))?;
        validate_i32_range(slot, 0, 255, &format!("{entry_what} field slot"))?;
        let item_value = find_entry(entries, "item")
            .ok_or_else(|| format!("{entry_what} is missing field item"))?;
        let item = decode_item_stack_template(item_value, registries).map_err(|error| {
            format!("{entry_what} field item failed item stack template decoding: {error}")
        })?;
        let slot = slot as usize;
        if slots.len() <= slot {
            slots.resize_with(slot + 1, || None);
        }
        slots[slot] = Some(item);
    }
    Ok(slots)
}

#[cfg(test)]
mod tests;
