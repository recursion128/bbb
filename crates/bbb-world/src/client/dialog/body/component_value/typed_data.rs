use bbb_protocol::{
    block_entity_types::vanilla_block_entity_type_for_resource_id,
    entity_types::vanilla_entity_type_for_resource_id, parse_snbt_compound, NbtSummaryEntry,
    NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

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

pub(super) fn decode_builtin_entity_data(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogTypedEntityDataState, String> {
    let (resource_id, data) = decode_typed_data(value, what)?;
    let entity_type = vanilla_entity_type_for_resource_id(&resource_id).ok_or_else(|| {
        format!("missing minecraft:entity_type resource {resource_id} for {what}")
    })?;
    Ok(DialogTypedEntityDataState {
        entity_type: DialogRegistryHolderReferenceState {
            holder_id: entity_type.id,
            resource_id: entity_type.resource_id.to_string(),
        },
        data,
    })
}

pub(super) fn decode_builtin_block_entity_data(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogTypedBlockEntityDataState, String> {
    let (resource_id, data) = decode_typed_data(value, what)?;
    let block_entity_type =
        vanilla_block_entity_type_for_resource_id(&resource_id).ok_or_else(|| {
            format!("missing minecraft:block_entity_type resource {resource_id} for {what}")
        })?;
    Ok(DialogTypedBlockEntityDataState {
        block_entity_type: DialogRegistryHolderReferenceState {
            holder_id: block_entity_type.id,
            resource_id: block_entity_type.resource_id.to_string(),
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
        NbtSummaryValue::String(raw) => parse_snbt_compound(raw)
            .map_err(|error| format!("invalid {what} flattened SNBT string: {error}")),
        _ => Err(format!("{what} is not a compound")),
    }
}

fn decode_typed_data(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<(String, NbtSummaryValue), String> {
    let parsed;
    let entries = match value {
        NbtSummaryValue::Compound(entries) => entries,
        NbtSummaryValue::String(_) => {
            parsed = decode_custom_data(value, what)?;
            let NbtSummaryValue::Compound(entries) = &parsed else {
                unreachable!("parse_snbt_compound returned a compound")
            };
            entries
        }
        _ => return Err(format!("{what} is not a compound")),
    };
    let id = required_field(entries, "id", what)?;
    let NbtSummaryValue::String(raw_id) = id else {
        return Err(format!("{what} field id is not a resource id"));
    };
    let resource_id = normalize_codec_identifier(raw_id)
        .ok_or_else(|| format!("invalid {what} field id {raw_id}"))?;
    let data = NbtSummaryValue::Compound(
        entries
            .iter()
            .filter(|entry| entry.name != "id")
            .cloned()
            .collect(),
    );
    Ok((resource_id, data))
}

fn required_field<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a NbtSummaryValue, String> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} is missing field {field}"))
}

#[cfg(test)]
mod tests;
