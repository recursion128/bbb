use bbb_protocol::{block_types::vanilla_block_type, packets::DebugStickPropertySummary};
use serde::{Deserialize, Serialize};

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDebugStickPropertyState {
    pub block: DialogRegistryHolderReferenceState,
    pub property: String,
}

pub(super) fn project_debug_stick_state(
    properties: &[DebugStickPropertySummary],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<Vec<DialogDebugStickPropertyState>, String> {
    properties
        .iter()
        .map(|property| {
            let resource_id = vanilla_block_type(property.block_registry_id)
                .map(|block| block.resource_id)
                .or_else(|| {
                    (!content_registry.is_vanilla_only())
                        .then(|| {
                            content_registry
                                .block_by_id(property.block_registry_id)
                                .map(|spec| spec.resource_id.as_str())
                        })
                        .flatten()
                })
                .ok_or_else(|| {
                    format!(
                        "unknown debug stick block id {}",
                        property.block_registry_id
                    )
                })?;
            Ok(DialogDebugStickPropertyState {
                block: DialogRegistryHolderReferenceState {
                    holder_id: property.block_registry_id,
                    resource_id: resource_id.to_string(),
                },
                property: property.property.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
