use bbb_protocol::NbtSummaryValue;
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
    entries
        .iter()
        .map(|entry| {
            let resource_id = normalize_codec_identifier(&entry.name)
                .ok_or_else(|| format!("invalid {what} block resource id {}", entry.name))?;
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
