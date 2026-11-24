#[cfg(test)]
use bbb_protocol::{data_component_types::*, NbtSummaryEntry};
use bbb_protocol::{
    data_component_types::{
        vanilla_data_component_animal_variant_registry, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
    },
    item_types::VANILLA_ITEM_BRICK_ID,
    NbtSummaryValue,
};
use bbb_render_types::VillagerModelType;
use serde::{Deserialize, Serialize};

use crate::{
    registries::normalize_codec_identifier, FrozenRegistryAccess, FrozenRegistryHolderSetKind,
};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogSuspiciousStewEffectState {
    pub effect: DialogRegistryHolderReferenceState,
    pub duration: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogPotDecorationsState {
    #[serde(default)]
    pub back: Option<DialogRegistryHolderReferenceState>,
    #[serde(default)]
    pub left: Option<DialogRegistryHolderReferenceState>,
    #[serde(default)]
    pub right: Option<DialogRegistryHolderReferenceState>,
    #[serde(default)]
    pub front: Option<DialogRegistryHolderReferenceState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBannerPatternLayerState {
    pub pattern: DialogBannerPatternHolderState,
    pub color_id: i32,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "holder_kind", rename_all = "snake_case")]
pub enum DialogBannerPatternHolderState {
    Named {
        registry_id: i32,
        resource_id: String,
    },
    Direct {
        pattern: DialogDirectBannerPatternState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDirectBannerPatternState {
    pub asset_id: String,
    pub translation_key: bbb_protocol::JavaString,
}

pub(super) fn decode_holder_backed_component_value(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Option<Result<DialogDataComponentValueState, String>> {
    if type_id == VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID {
        return Some(
            decode_builtin_villager_type_reference(value, what).map(|holder| {
                DialogDataComponentValueState::RegistryVariant {
                    registry: "minecraft:villager_type".to_string(),
                    holder,
                }
            }),
        );
    }
    if let Some(registry) = vanilla_data_component_animal_variant_registry(type_id) {
        return Some(
            decode_registry_holder_reference(value, registry, registries, what).map(|holder| {
                DialogDataComponentValueState::RegistryVariant {
                    registry: registry.to_string(),
                    holder,
                }
            }),
        );
    }
    None
}

fn decode_builtin_villager_type_reference(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} holder is not a resource id"));
    };
    let resource_id = raw
        .with_utf8(normalize_codec_identifier)
        .flatten()
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let villager_type = VillagerModelType::for_resource_id(&resource_id).ok_or_else(|| {
        format!("missing minecraft:villager_type resource {resource_id} for {what}")
    })?;
    Ok(DialogRegistryHolderReferenceState {
        holder_id: villager_type.vanilla_registry_index() as i32,
        resource_id: villager_type.resource_id().to_string(),
    })
}

pub(super) fn project_banner_pattern_layers(
    layers: &[bbb_protocol::packets::BannerPatternLayerSummary],
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<Vec<DialogBannerPatternLayerState>, String> {
    layers
        .iter()
        .map(|layer| {
            let color =
                bbb_protocol::builtin_enum_component_values::builtin_enum_component_value_name(
                    VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
                    layer.color_id,
                )
                .unwrap();
            let pattern = if let Some(id) = layer.registry_id {
                let holder =
                    project_registry_holder(id, "minecraft:banner_pattern", registries.unwrap())?;
                DialogBannerPatternHolderState::Named {
                    registry_id: holder.holder_id,
                    resource_id: holder.resource_id,
                }
            } else {
                DialogBannerPatternHolderState::Direct {
                    pattern: DialogDirectBannerPatternState {
                        asset_id: layer.asset_id.clone().unwrap(),
                        translation_key: layer.translation_key.clone().unwrap(),
                    },
                }
            };
            Ok(DialogBannerPatternLayerState {
                pattern,
                color_id: layer.color_id,
                color: color.to_string(),
            })
        })
        .collect()
}

pub(super) fn project_pot_decorations(
    sides: &[i32],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<DialogPotDecorationsState, String> {
    let mut resolved = [None, None, None, None];
    for (index, slot) in resolved.iter_mut().enumerate() {
        let id = sides[index];
        if id != VANILLA_ITEM_BRICK_ID {
            let resource_id = bbb_protocol::item_types::vanilla_item_type(id)
                .map(|item| item.resource_id)
                .or_else(|| {
                    (!content_registry.is_vanilla_only())
                        .then(|| {
                            content_registry
                                .item_by_id(id)
                                .map(|spec| spec.resource_id.as_str())
                        })
                        .flatten()
                })
                .ok_or_else(|| format!("unknown pot decoration item id {id}"))?;
            *slot = Some(DialogRegistryHolderReferenceState {
                holder_id: id,
                resource_id: resource_id.to_string(),
            });
        }
    }
    let [back, left, right, front] = resolved;
    Ok(DialogPotDecorationsState {
        back,
        left,
        right,
        front,
    })
}

pub(super) fn project_suspicious_stew_effects(
    effects: &[bbb_protocol::packets::SuspiciousStewEffectSummary],
) -> Result<Vec<DialogSuspiciousStewEffectState>, String> {
    effects
        .iter()
        .map(|effect| {
            let entry = bbb_protocol::mob_effect_types::vanilla_mob_effect_type(effect.effect_id)
                .ok_or_else(|| format!("unknown mob effect id {}", effect.effect_id))?;
            Ok(DialogSuspiciousStewEffectState {
                effect: DialogRegistryHolderReferenceState {
                    holder_id: entry.id,
                    resource_id: entry.resource_id.to_string(),
                },
                duration: effect.duration,
            })
        })
        .collect()
}

pub(super) fn project_registry_holder(
    id: i32,
    registry: &str,
    registries: FrozenRegistryAccess<'_>,
) -> Result<DialogRegistryHolderReferenceState, String> {
    registries
        .require_entry_by_holder_id(registry, id)
        .map(dialog_registry_holder_reference)
        .map_err(|error| error.to_string())
}

pub(super) fn decode_registry_holder_reference(
    value: &NbtSummaryValue,
    registry: &str,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let registries = registries
        .ok_or_else(|| format!("{what} holder cannot be decoded without registry context"))?;
    let entry = registries
        .persistent_holder_from_nbt(registry, value)
        .map_err(|error| format!("{what}: {error}"))?;
    Ok(dialog_registry_holder_reference(entry))
}

pub(super) fn decode_registry_holder_set(
    value: &NbtSummaryValue,
    registry: &str,
    registries: Option<FrozenRegistryAccess<'_>>,
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
    let holder_set = registries
        .persistent_holder_set_from_nbt(registry, value)
        .map_err(|error| format!("{error} for {what}"))?;
    let (holder_set_kind, tag) = match holder_set.kind {
        FrozenRegistryHolderSetKind::Direct => ("direct", None),
        FrozenRegistryHolderSetKind::Named { tag, .. } => ("named", Some(tag.to_string())),
    };
    Ok(DialogRegistryHolderSetState {
        holder_set_kind: holder_set_kind.to_string(),
        tag,
        entries: holder_set
            .entries
            .into_iter()
            .map(dialog_registry_holder_reference)
            .collect(),
    })
}

fn dialog_registry_holder_reference(
    entry: crate::FrozenRegistryEntry<'_>,
) -> DialogRegistryHolderReferenceState {
    DialogRegistryHolderReferenceState {
        holder_id: entry.holder_id,
        resource_id: entry.resource_id.to_string(),
    }
}

pub(super) fn project_registry_holder_set(
    ids: &[i32],
    tag: &Option<String>,
    registry: &str,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogRegistryHolderSetState, String> {
    let Some(registries) = registries else {
        if ids.is_empty() && tag.is_none() {
            return Ok(DialogRegistryHolderSetState {
                holder_set_kind: "direct".to_string(),
                tag: None,
                entries: Vec::new(),
            });
        }
        return Err(format!(
            "{registry} holder set cannot be decoded without registry context"
        ));
    };
    let entries = match tag {
        Some(tag) => {
            registries
                .holder_set_by_tag(registry, tag)
                .map_err(|error| error.to_string())?
                .entries
        }
        None => ids
            .iter()
            .map(|&id| {
                if matches!(
                    registry,
                    "minecraft:damage_type" | "minecraft:banner_pattern"
                ) {
                    registries
                        .require_entry_by_holder_id(registry, id)
                        .map_err(|error| error.to_string())
                } else {
                    resolve_builtin_holder(registry, id, registries)
                }
            })
            .collect::<Result<_, _>>()?,
    };
    Ok(DialogRegistryHolderSetState {
        holder_set_kind: if tag.is_some() { "named" } else { "direct" }.to_string(),
        tag: tag.clone(),
        entries: entries
            .into_iter()
            .map(dialog_registry_holder_reference)
            .collect(),
    })
}

pub(super) fn resolve_builtin_holder<'a>(
    registry: &str,
    id: i32,
    registries: FrozenRegistryAccess<'a>,
) -> Result<crate::FrozenRegistryEntry<'a>, String> {
    let content_registry = registries.content_registry();
    let resource_id = match registry {
        "minecraft:item" => bbb_protocol::item_types::vanilla_item_type(id)
            .map(|entry| entry.resource_id)
            .or_else(|| {
                (!content_registry.is_vanilla_only())
                    .then(|| {
                        content_registry
                            .item_by_id(id)
                            .map(|spec| spec.resource_id.as_str())
                    })
                    .flatten()
            }),
        "minecraft:block" => bbb_protocol::block_types::vanilla_block_type(id)
            .map(|entry| entry.resource_id)
            .or_else(|| {
                (!content_registry.is_vanilla_only())
                    .then(|| {
                        content_registry
                            .block_by_id(id)
                            .map(|spec| spec.resource_id.as_str())
                    })
                    .flatten()
            }),
        "minecraft:entity_type" => bbb_protocol::entity_types::vanilla_entity_type(id)
            .map(|entry| entry.resource_id)
            .or_else(|| {
                (!content_registry.is_vanilla_only())
                    .then(|| {
                        content_registry
                            .entity_by_id(id)
                            .map(|spec| spec.resource_id.as_str())
                    })
                    .flatten()
            }),
        "minecraft:mob_effect" => bbb_protocol::mob_effect_types::vanilla_mob_effect_type(id)
            .map(|entry| entry.resource_id),
        _ => None,
    }
    .ok_or_else(|| format!("unknown {registry} holder {id}"))?;
    registries
        .require_entry_by_resource_id(registry, resource_id)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
