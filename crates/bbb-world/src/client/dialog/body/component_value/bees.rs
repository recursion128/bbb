use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use super::holder_set::DialogRegistryHolderReferenceState;
use super::typed_data::decode_builtin_entity_data;

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
    let entity_data = decode_builtin_entity_data(
        required_field(entries, "entity_data", what)?,
        &format!("{what} field entity_data"),
    )?;
    Ok(DialogBeeOccupantState {
        entity_type: entity_data.entity_type,
        entity_data: entity_data.data,
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
