use std::collections::HashSet;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use crate::registries::{normalize_codec_identifier, BlockStateRegistry};

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDebugStickPropertyState {
    pub block: DialogRegistryHolderReferenceState,
    pub property: String,
}

pub(super) fn decode_debug_stick_state(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogDebugStickPropertyState>, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let blocks = BlockStateRegistry::vanilla_26_1();
    let mut compound_entries = Vec::<&NbtSummaryEntry>::with_capacity(entries.len());
    for entry in entries {
        if let Some(existing) = compound_entries
            .iter_mut()
            .find(|existing| existing.name == entry.name)
        {
            *existing = entry;
        } else {
            compound_entries.push(entry);
        }
    }
    let mut decoded_blocks = HashSet::new();
    compound_entries
        .iter()
        .map(|entry| {
            let resource_id = normalize_codec_identifier(&entry.name)
                .ok_or_else(|| format!("invalid {what} block resource id {}", entry.name))?;
            if !decoded_blocks.insert(resource_id.clone()) {
                return Err(format!(
                    "duplicate {what} block holder after resource-id normalization: {resource_id}"
                ));
            }
            let holder_id = blocks
                .block_registry_id_by_name(&resource_id)
                .ok_or_else(|| {
                    format!("missing minecraft:block resource {resource_id} for {what}")
                })?;
            let NbtSummaryValue::String(property) = &entry.value else {
                return Err(format!(
                    "{what} property for block {resource_id} is not a string"
                ));
            };
            if !blocks.block_has_property(&resource_id, property) {
                return Err(format!(
                    "block {resource_id} has no property {property} for {what}"
                ));
            }
            Ok(DialogDebugStickPropertyState {
                block: DialogRegistryHolderReferenceState {
                    holder_id,
                    resource_id,
                },
                property: property.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
