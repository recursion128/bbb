use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_render_types::vanilla_map_decoration_type_for_resource_id;
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogMapDecorationState {
    pub id: String,
    pub decoration_type: DialogRegistryHolderReferenceState,
    pub x_bits: u64,
    pub z_bits: u64,
    pub rotation_bits: u32,
}

pub(super) fn decode_map_decorations(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogMapDecorationState>, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound map"));
    };
    let mut normalized = Vec::<&NbtSummaryEntry>::with_capacity(entries.len());
    for entry in entries {
        if let Some(index) = normalized
            .iter()
            .position(|existing| existing.name == entry.name)
        {
            normalized[index] = entry;
        } else {
            normalized.push(entry);
        }
    }
    normalized
        .into_iter()
        .map(|entry| decode_map_decoration(&entry.name, &entry.value, what))
        .collect()
}

fn decode_map_decoration(
    id: &str,
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogMapDecorationState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} entry {id:?} is not a compound"));
    };
    let entry_what = format!("{what} entry {id:?}");
    let raw_type = match required_field(entries, "type", &entry_what)? {
        NbtSummaryValue::String(raw_type) => raw_type,
        _ => return Err(format!("{entry_what} field type is not a resource id")),
    };
    let resource_id = normalize_codec_identifier(raw_type)
        .ok_or_else(|| format!("invalid {entry_what} type resource id {raw_type}"))?;
    let decoration_type =
        vanilla_map_decoration_type_for_resource_id(&resource_id).ok_or_else(|| {
            format!("missing minecraft:map_decoration_type resource {resource_id} for {entry_what}")
        })?;
    Ok(DialogMapDecorationState {
        id: id.to_string(),
        decoration_type: DialogRegistryHolderReferenceState {
            holder_id: decoration_type.type_id,
            resource_id: decoration_type.resource_id.to_string(),
        },
        x_bits: super::component_f64_bits(
            required_field(entries, "x", &entry_what)?,
            &format!("{entry_what} field x"),
        )?,
        z_bits: super::component_f64_bits(
            required_field(entries, "z", &entry_what)?,
            &format!("{entry_what} field z"),
        )?,
        rotation_bits: super::component_f32_bits(
            required_field(entries, "rotation", &entry_what)?,
            &format!("{entry_what} field rotation"),
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
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} is missing field {field}"))
}

#[cfg(test)]
mod tests;
