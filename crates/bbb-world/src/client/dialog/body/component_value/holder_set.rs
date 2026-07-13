use bbb_protocol::NbtSummaryValue;
use serde::{Deserialize, Serialize};

use crate::{registries::normalize_codec_identifier, RegistrySet};

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
