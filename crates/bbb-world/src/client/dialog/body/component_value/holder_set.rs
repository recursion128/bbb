use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
    },
    NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{registries::normalize_codec_identifier, RegistrySet};

use super::DialogDataComponentValueState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogRegistryHolderReferenceState {
    pub holder_id: i32,
    pub resource_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogRegistryHolderSetState {
    pub holder_set_kind: String,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub entries: Vec<DialogRegistryHolderReferenceState>,
}

pub(super) fn decode_holder_backed_component_value(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Option<Result<DialogDataComponentValueState, String>> {
    match type_id {
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => Some(
            decode_registry_holder_set_field(value, "items", "minecraft:item", registries, what)
                .map(|items| DialogDataComponentValueState::Repairable { items }),
        ),
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => Some(
            decode_registry_holder_set(value, "minecraft:banner_pattern", registries, what)
                .map(|patterns| DialogDataComponentValueState::ProvidesBannerPatterns { patterns }),
        ),
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => Some(
            decode_registry_holder_reference(value, "minecraft:jukebox_song", registries, what)
                .map(|song| DialogDataComponentValueState::JukeboxPlayable { song }),
        ),
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => Some(
            decode_registry_holder_set_field(
                value,
                "types",
                "minecraft:damage_type",
                registries,
                what,
            )
            .map(|types| DialogDataComponentValueState::DamageResistant { types }),
        ),
        _ => None,
    }
}

pub(super) fn decode_registry_holder_reference(
    value: &NbtSummaryValue,
    registry: &str,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let registries = registries
        .ok_or_else(|| format!("{what} holder cannot be decoded without registry context"))?;
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} holder is not a resource id"));
    };
    decode_holder_reference(raw, registry, registries, what)
}

pub(super) fn decode_registry_holder_set_field(
    value: &NbtSummaryValue,
    field: &str,
    registry: &str,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<DialogRegistryHolderSetState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let field_value = entries
        .iter()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} is missing field {field}"))?;
    decode_registry_holder_set(field_value, registry, registries, what)
}

pub(super) fn decode_registry_holder_set(
    value: &NbtSummaryValue,
    registry: &str,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<DialogRegistryHolderSetState, String> {
    let Some(registries) = registries else {
        if matches!(value, NbtSummaryValue::List(values) if values.is_empty()) {
            return Ok(DialogRegistryHolderSetState {
                holder_set_kind: "direct".to_string(),
                tag: None,
                entries: Vec::new(),
            });
        }
        return Err(format!(
            "{what} holder set cannot be decoded without registry context"
        ));
    };
    if let NbtSummaryValue::String(raw) = value {
        if let Some(raw_tag) = raw.strip_prefix('#') {
            return decode_named_holder_set(raw_tag, registry, registries, what);
        }
        return Ok(DialogRegistryHolderSetState {
            holder_set_kind: "direct".to_string(),
            tag: None,
            entries: vec![decode_holder_reference(raw, registry, registries, what)?],
        });
    }

    let NbtSummaryValue::List(values) = value else {
        return Err(format!(
            "{what} holder set is neither a tag, resource id, nor list"
        ));
    };
    let entries = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let NbtSummaryValue::String(raw) = value else {
                return Err(format!(
                    "{what} holder set entry {index} is not a resource id"
                ));
            };
            decode_holder_reference(
                raw,
                registry,
                registries,
                &format!("{what} holder set entry {index}"),
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogRegistryHolderSetState {
        holder_set_kind: "direct".to_string(),
        tag: None,
        entries,
    })
}

fn decode_named_holder_set(
    raw_tag: &str,
    registry: &str,
    registries: &RegistrySet,
    what: &str,
) -> Result<DialogRegistryHolderSetState, String> {
    let tag = normalize_codec_identifier(raw_tag)
        .ok_or_else(|| format!("invalid {what} holder-set tag #{raw_tag}"))?;
    let holder_ids = registries
        .holder_ids_for_tag(registry, &tag)
        .ok_or_else(|| format!("missing {registry} tag #{tag} for {what}"))?;
    let entries = holder_ids
        .iter()
        .map(|holder_id| {
            let entry = registries
                .resolved_entry_by_holder_id(registry, *holder_id)
                .ok_or_else(|| {
                    format!("{what} tag #{tag} contains missing {registry} holder {holder_id}")
                })?;
            Ok(DialogRegistryHolderReferenceState {
                holder_id: entry.holder_id,
                resource_id: entry.resource_id.to_string(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogRegistryHolderSetState {
        holder_set_kind: "named".to_string(),
        tag: Some(tag),
        entries,
    })
}

fn decode_holder_reference(
    raw: &str,
    registry: &str,
    registries: &RegistrySet,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let entry = registries
        .resolved_entry_by_resource_id(registry, &resource_id)
        .ok_or_else(|| format!("missing {registry} resource {resource_id} for {what}"))?;
    Ok(DialogRegistryHolderReferenceState {
        holder_id: entry.holder_id,
        resource_id: entry.resource_id.to_string(),
    })
}

#[cfg(test)]
mod tests;
