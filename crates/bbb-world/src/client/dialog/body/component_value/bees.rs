use bbb_protocol::{
    entity_types::vanilla_entity_type_for_resource_id, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBeeOccupantState {
    pub entity_type: DialogRegistryHolderReferenceState,
    pub entity_data: NbtSummaryValue,
    pub ticks_in_hive: i32,
    pub min_ticks_in_hive: i32,
}

pub(super) fn decode_bees(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogBeeOccupantState>, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_bee_occupant(value, &format!("{what}[{index}]")))
        .collect()
}

fn decode_bee_occupant(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogBeeOccupantState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let entity_data = required_field(entries, "entity_data", what)?;
    let NbtSummaryValue::Compound(entity_entries) = entity_data else {
        return Err(format!("{what} field entity_data is not a compound"));
    };
    let id = required_field(entity_entries, "id", &format!("{what} field entity_data"))?;
    let NbtSummaryValue::String(raw_id) = id else {
        return Err(format!("{what} field entity_data id is not a resource id"));
    };
    let resource_id = normalize_codec_identifier(raw_id)
        .ok_or_else(|| format!("invalid {what} field entity_data id {raw_id}"))?;
    let entity_type = vanilla_entity_type_for_resource_id(&resource_id).ok_or_else(|| {
        format!("missing minecraft:entity_type resource {resource_id} for {what}")
    })?;
    Ok(DialogBeeOccupantState {
        entity_type: DialogRegistryHolderReferenceState {
            holder_id: entity_type.id,
            resource_id: entity_type.resource_id.to_string(),
        },
        entity_data: NbtSummaryValue::Compound(
            entity_entries
                .iter()
                .filter(|entry| entry.name != "id")
                .cloned()
                .collect(),
        ),
        ticks_in_hive: super::component_i32(
            required_field(entries, "ticks_in_hive", what)?,
            &format!("{what} field ticks_in_hive"),
        )?,
        min_ticks_in_hive: super::component_i32(
            required_field(entries, "min_ticks_in_hive", what)?,
            &format!("{what} field min_ticks_in_hive"),
        )?,
    })
}

fn required_field<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a NbtSummaryValue, String> {
    entries
        .iter()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} is missing field {field}"))
}

#[cfg(test)]
mod tests;
