use bbb_protocol::{
    nbt_summary::codec_int_value,
    packets::{data_components::ContainerContentsSummary, ItemStackTemplateSummary},
    NbtSummaryValue,
};

use super::super::super::DialogDecodeContext;
use super::super::{project_item_stack_template, DialogItemStackTemplateState};
use super::{component_list, find_entry};

pub(super) fn project_item_stack_template_list(
    items: &[ItemStackTemplateSummary],
    source: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<Vec<DialogItemStackTemplateState>, String> {
    items
        .iter()
        .zip(component_list(source).unwrap().iter())
        .map(|(item, source)| project_item_stack_template(item, source, context))
        .collect()
}

pub(super) fn project_item_stack_template_slots(
    container: &ContainerContentsSummary,
    source: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<Vec<Option<DialogItemStackTemplateState>>, String> {
    let mut slots = vec![None; container.slot_count.unwrap()];
    for record in component_list(source).unwrap().iter().rev() {
        let NbtSummaryValue::Compound(fields) = record else {
            unreachable!()
        };
        let slot = codec_int_value(find_entry(fields, "slot").unwrap()).unwrap() as usize;
        if slots[slot].is_none() {
            let index = container
                .slots
                .binary_search_by_key(&slot, |entry| entry.slot)
                .unwrap();
            slots[slot] = Some(project_item_stack_template(
                &container.slots[index].item,
                find_entry(fields, "item").unwrap(),
                context,
            )?);
        }
    }
    Ok(slots)
}

#[cfg(test)]
mod tests;
