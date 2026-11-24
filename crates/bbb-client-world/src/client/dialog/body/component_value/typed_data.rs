use bbb_protocol::{
    block_entity_types::vanilla_block_entity_type, entity_types::vanilla_entity_type,
    packets::DataComponentPatchSummary, parse_snbt_compound_utf16, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTypedEntityDataState {
    pub entity_type: DialogRegistryHolderReferenceState,
    pub data: NbtSummaryValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTypedBlockEntityDataState {
    pub block_entity_type: DialogRegistryHolderReferenceState,
    pub data: NbtSummaryValue,
}

pub(super) fn project_entity_data(
    patch: &DataComponentPatchSummary,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<DialogTypedEntityDataState, String> {
    let id = patch
        .entity_data_entity_type_id
        .ok_or_else(|| "missing entity_data entity type id".to_string())?;
    let resource_id = vanilla_entity_type(id)
        .map(|entity| entity.resource_id)
        .or_else(|| {
            (!content_registry.is_vanilla_only())
                .then(|| {
                    content_registry
                        .entity_by_id(id)
                        .map(|spec| spec.resource_id.as_str())
                })
                .flatten()
        })
        .ok_or_else(|| format!("unknown entity_data entity type id {id}"))?;
    let data = patch
        .entity_data_tag
        .as_ref()
        .ok_or_else(|| "missing entity_data payload tag".to_string())?
        .clone();
    Ok(DialogTypedEntityDataState {
        entity_type: DialogRegistryHolderReferenceState {
            holder_id: id,
            resource_id: resource_id.to_string(),
        },
        data,
    })
}

pub(super) fn project_block_entity_data(
    patch: &DataComponentPatchSummary,
) -> Result<DialogTypedBlockEntityDataState, String> {
    let id = patch
        .block_entity_data_type_id
        .ok_or_else(|| "missing block_entity_data type id".to_string())?;
    let block_entity = vanilla_block_entity_type(id)
        .ok_or_else(|| format!("unknown block_entity_data type id {id}"))?;
    let data = patch
        .block_entity_data_tag
        .as_ref()
        .ok_or_else(|| "missing block_entity_data payload tag".to_string())?
        .clone();
    Ok(DialogTypedBlockEntityDataState {
        block_entity_type: DialogRegistryHolderReferenceState {
            holder_id: id,
            resource_id: block_entity.resource_id.to_string(),
        },
        data,
    })
}

pub(super) fn decode_custom_data(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<NbtSummaryValue, String> {
    match value {
        NbtSummaryValue::Compound(_) => Ok(value.clone()),
        NbtSummaryValue::String(raw) => parse_snbt_compound_utf16(raw.units())
            .map_err(|error| format!("invalid {what} flattened SNBT string: {error}")),
        _ => Err(format!("{what} is not a compound")),
    }
}

#[cfg(test)]
mod tests;
