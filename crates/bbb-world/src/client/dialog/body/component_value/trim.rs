use std::collections::BTreeMap;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use crate::{registries::normalize_codec_identifier, RegistrySet};

use super::super::super::{decode_component, DialogTextState};
use super::{component_bool, find_entry, holder_set::decode_registry_holder_reference};

const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
const TRIM_PATTERN_REGISTRY: &str = "minecraft:trim_pattern";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "holder_kind", rename_all = "snake_case")]
pub enum DialogTrimMaterialHolderState {
    Named {
        registry_id: i32,
        resource_id: String,
    },
    Direct {
        material: DialogTrimMaterialState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTrimMaterialState {
    pub asset_name: String,
    #[serde(default)]
    pub override_armor_assets: BTreeMap<String, String>,
    pub description: DialogTextState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "holder_kind", rename_all = "snake_case")]
pub enum DialogTrimPatternHolderState {
    Named {
        registry_id: i32,
        resource_id: String,
    },
    Direct {
        pattern: DialogTrimPatternState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTrimPatternState {
    pub asset_id: String,
    pub description: DialogTextState,
    pub decal: bool,
}

pub(super) fn decode_trim_material_holder(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<DialogTrimMaterialHolderState, String> {
    match value {
        NbtSummaryValue::String(_) => {
            let holder =
                decode_registry_holder_reference(value, TRIM_MATERIAL_REGISTRY, registries, what)?;
            Ok(DialogTrimMaterialHolderState::Named {
                registry_id: holder.holder_id,
                resource_id: holder.resource_id,
            })
        }
        NbtSummaryValue::Compound(entries) => Ok(DialogTrimMaterialHolderState::Direct {
            material: decode_direct_trim_material(entries, what)?,
        }),
        _ => Err(format!(
            "{what} holder is neither a resource id nor a direct trim material"
        )),
    }
}

pub(super) fn decode_trim_pattern_holder(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<DialogTrimPatternHolderState, String> {
    match value {
        NbtSummaryValue::String(_) => {
            let holder =
                decode_registry_holder_reference(value, TRIM_PATTERN_REGISTRY, registries, what)?;
            Ok(DialogTrimPatternHolderState::Named {
                registry_id: holder.holder_id,
                resource_id: holder.resource_id,
            })
        }
        NbtSummaryValue::Compound(entries) => Ok(DialogTrimPatternHolderState::Direct {
            pattern: decode_direct_trim_pattern(entries, what)?,
        }),
        _ => Err(format!(
            "{what} holder is neither a resource id nor a direct trim pattern"
        )),
    }
}

pub(super) fn decode_armor_trim(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<(DialogTrimMaterialHolderState, DialogTrimPatternHolderState), String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let material = decode_trim_material_holder(
        required_entry(entries, "material", what)?,
        registries,
        &format!("{what} field material"),
    )?;
    let pattern = decode_trim_pattern_holder(
        required_entry(entries, "pattern", what)?,
        registries,
        &format!("{what} field pattern"),
    )?;
    Ok((material, pattern))
}

fn decode_direct_trim_material(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogTrimMaterialState, String> {
    let direct_what = format!("{what} direct holder");
    Ok(DialogTrimMaterialState {
        asset_name: decode_resource_path(
            required_entry(entries, "asset_name", &direct_what)?,
            &format!("{direct_what} field asset_name"),
        )?,
        override_armor_assets: decode_override_armor_assets(entries, &direct_what)?,
        description: decode_component(
            required_entry(entries, "description", &direct_what)?,
            &format!("{direct_what} field description"),
        )?,
    })
}

fn decode_direct_trim_pattern(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogTrimPatternState, String> {
    let direct_what = format!("{what} direct holder");
    let asset_id = required_entry(entries, "asset_id", &direct_what)?;
    let NbtSummaryValue::String(raw_asset_id) = asset_id else {
        return Err(format!("{direct_what} field asset_id is not a resource id"));
    };
    Ok(DialogTrimPatternState {
        asset_id: normalize_codec_identifier(raw_asset_id).ok_or_else(|| {
            format!("invalid {direct_what} field asset_id resource id {raw_asset_id}")
        })?,
        description: decode_component(
            required_entry(entries, "description", &direct_what)?,
            &format!("{direct_what} field description"),
        )?,
        decal: find_entry(entries, "decal").map_or(Ok(false), |value| {
            component_bool(value, &format!("{direct_what} field decal"))
        })?,
    })
}

fn decode_override_armor_assets(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<BTreeMap<String, String>, String> {
    let Some(value) = find_entry(entries, "override_armor_assets") else {
        return Ok(BTreeMap::new());
    };
    let NbtSummaryValue::Compound(overrides) = value else {
        return Err(format!("{what} field override_armor_assets is not a map"));
    };
    let mut decoded = BTreeMap::new();
    for entry in overrides {
        let equipment_asset = normalize_codec_identifier(&entry.name).ok_or_else(|| {
            format!(
                "invalid {what} field override_armor_assets equipment asset {}",
                entry.name
            )
        })?;
        let suffix = decode_resource_path(
            &entry.value,
            &format!("{what} field override_armor_assets equipment asset {equipment_asset}"),
        )?;
        if decoded.insert(equipment_asset.clone(), suffix).is_some() {
            return Err(format!(
                "duplicate {what} field override_armor_assets equipment asset {equipment_asset}"
            ));
        }
    }
    Ok(decoded)
}

fn decode_resource_path(value: &NbtSummaryValue, what: &str) -> Result<String, String> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} is not a resource path"));
    };
    if !raw.chars().all(|ch| {
        ch == '_'
            || ch == '-'
            || ch == '/'
            || ch == '.'
            || ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
    }) {
        return Err(format!("invalid {what} resource path {raw}"));
    }
    Ok(raw.clone())
}

fn required_entry<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<&'a NbtSummaryValue, String> {
    find_entry(entries, field).ok_or_else(|| format!("{what} is missing field {field}"))
}

#[cfg(test)]
mod tests;
