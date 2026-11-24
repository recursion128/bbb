use std::collections::BTreeMap;

use bbb_protocol::packets::{TrimMaterialSummary, TrimPatternSummary};
use serde::{Deserialize, Serialize};

use super::super::super::{decode_component, DialogDecodeContext, DialogTextState};

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

pub(super) fn project_trim_material_holder(
    id: Option<i32>,
    direct: Option<&TrimMaterialSummary>,
    context: DialogDecodeContext<'_>,
) -> Result<DialogTrimMaterialHolderState, String> {
    if let Some(id) = id {
        let entry = super::holder_set::project_registry_holder(
            id,
            TRIM_MATERIAL_REGISTRY,
            context.registry_access.unwrap(),
        )?;
        return Ok(DialogTrimMaterialHolderState::Named {
            registry_id: entry.holder_id,
            resource_id: entry.resource_id,
        });
    }
    let material = direct.unwrap();
    Ok(DialogTrimMaterialHolderState::Direct {
        material: DialogTrimMaterialState {
            asset_name: material.asset_name.clone(),
            override_armor_assets: material.override_armor_assets.clone(),
            description: decode_component(
                material.description_nbt.as_deref().unwrap(),
                "trim material description",
                DialogDecodeContext {
                    prepare_item_summary: false,
                    ..context
                },
            )?,
        },
    })
}

pub(super) fn project_trim_pattern_holder(
    id: Option<i32>,
    direct: Option<&TrimPatternSummary>,
    context: DialogDecodeContext<'_>,
) -> Result<DialogTrimPatternHolderState, String> {
    if let Some(id) = id {
        let entry = super::holder_set::project_registry_holder(
            id,
            TRIM_PATTERN_REGISTRY,
            context.registry_access.unwrap(),
        )?;
        return Ok(DialogTrimPatternHolderState::Named {
            registry_id: entry.holder_id,
            resource_id: entry.resource_id,
        });
    }
    let pattern = direct.unwrap();
    Ok(DialogTrimPatternHolderState::Direct {
        pattern: DialogTrimPatternState {
            asset_id: pattern.asset_id.clone(),
            description: decode_component(
                pattern.description_nbt.as_deref().unwrap(),
                "trim pattern description",
                DialogDecodeContext {
                    prepare_item_summary: false,
                    ..context
                },
            )?,
            decal: pattern.decal,
        },
    })
}

#[cfg(test)]
mod tests;
