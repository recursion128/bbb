use bbb_protocol::{
    builtin_enum_component_values::builtin_enum_component_value_for_name,
    data_component_types::{
        vanilla_data_component_animal_variant_registry,
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
    },
    item_types::vanilla_item_type_for_resource_id,
    mob_effect_types::vanilla_mob_effect_type_for_resource_id,
    NbtSummaryEntry, NbtSummaryValue,
};
use bbb_render_types::VillagerModelType;
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
    pub translation_key: String,
}

pub(super) fn decode_holder_backed_component_value(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
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
    match type_id {
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => Some(
            decode_registry_holder_set_field(value, "items", "minecraft:item", registries, what)
                .map(|items| DialogDataComponentValueState::Repairable { items }),
        ),
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => Some(
            decode_registry_holder_set(value, "minecraft:banner_pattern", registries, what)
                .map(|patterns| DialogDataComponentValueState::ProvidesBannerPatterns { patterns }),
        ),
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => Some(
            decode_banner_pattern_layers(value, registries, what)
                .map(|layers| DialogDataComponentValueState::BannerPatterns { layers }),
        ),
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => Some(
            decode_registry_holder_reference(value, "minecraft:jukebox_song", registries, what)
                .map(|song| DialogDataComponentValueState::JukeboxPlayable { song }),
        ),
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => Some(
            decode_registry_holder_reference(value, "minecraft:painting_variant", registries, what)
                .map(|variant| DialogDataComponentValueState::PaintingVariant { variant }),
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
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => Some(
            decode_registry_holder_reference(value, "minecraft:damage_type", registries, what)
                .map(|damage_type| DialogDataComponentValueState::DamageType { damage_type }),
        ),
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => Some(
            decode_suspicious_stew_effects(value, what)
                .map(|effects| DialogDataComponentValueState::SuspiciousStewEffects { effects }),
        ),
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => Some(
            decode_pot_decorations(value, what)
                .map(|decorations| DialogDataComponentValueState::PotDecorations { decorations }),
        ),
        _ => None,
    }
}

fn decode_builtin_villager_type_reference(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} holder is not a resource id"));
    };
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let villager_type = VillagerModelType::for_resource_id(&resource_id).ok_or_else(|| {
        format!("missing minecraft:villager_type resource {resource_id} for {what}")
    })?;
    Ok(DialogRegistryHolderReferenceState {
        holder_id: villager_type.vanilla_registry_index() as i32,
        resource_id: villager_type.resource_id().to_string(),
    })
}

fn decode_banner_pattern_layers(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<Vec<DialogBannerPatternLayerState>, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let entry_what = format!("{what}[{index}]");
            let NbtSummaryValue::Compound(entries) = value else {
                return Err(format!("{entry_what} is not a compound"));
            };
            let pattern_value = entries
                .iter()
                .rev()
                .find(|entry| entry.name == "pattern")
                .map(|entry| &entry.value)
                .ok_or_else(|| format!("{entry_what} is missing field pattern"))?;
            let pattern = decode_banner_pattern_holder(
                pattern_value,
                registries,
                &format!("{entry_what} field pattern"),
            )?;
            let color_value = entries
                .iter()
                .rev()
                .find(|entry| entry.name == "color")
                .map(|entry| &entry.value)
                .ok_or_else(|| format!("{entry_what} is missing field color"))?;
            let NbtSummaryValue::String(color) = color_value else {
                return Err(format!("{entry_what} field color is not a string"));
            };
            let color_entry = builtin_enum_component_value_for_name(
                VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
                color,
            )
            .ok_or_else(|| format!("invalid {entry_what} field color {color}"))?;
            Ok(DialogBannerPatternLayerState {
                pattern,
                color_id: color_entry.id,
                color: color_entry.serialized_name.to_string(),
            })
        })
        .collect()
}

fn decode_banner_pattern_holder(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<DialogBannerPatternHolderState, String> {
    match value {
        NbtSummaryValue::String(_) => {
            let holder = decode_registry_holder_reference(
                value,
                "minecraft:banner_pattern",
                registries,
                what,
            )?;
            Ok(DialogBannerPatternHolderState::Named {
                registry_id: holder.holder_id,
                resource_id: holder.resource_id,
            })
        }
        NbtSummaryValue::Compound(entries) => Ok(DialogBannerPatternHolderState::Direct {
            pattern: decode_direct_banner_pattern(entries, what)?,
        }),
        _ => Err(format!(
            "{what} holder is neither a resource id nor a direct banner pattern"
        )),
    }
}

fn decode_direct_banner_pattern(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogDirectBannerPatternState, String> {
    let asset_id = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "asset_id")
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} direct holder is missing field asset_id"))?;
    let NbtSummaryValue::String(raw_asset_id) = asset_id else {
        return Err(format!(
            "{what} direct holder field asset_id is not a resource id"
        ));
    };
    let asset_id = normalize_codec_identifier(raw_asset_id).ok_or_else(|| {
        format!("invalid {what} direct holder field asset_id resource id {raw_asset_id}")
    })?;
    let translation_key = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "translation_key")
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} direct holder is missing field translation_key"))?;
    let NbtSummaryValue::String(translation_key) = translation_key else {
        return Err(format!(
            "{what} direct holder field translation_key is not a string"
        ));
    };
    Ok(DialogDirectBannerPatternState {
        asset_id,
        translation_key: translation_key.clone(),
    })
}

fn decode_pot_decorations(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogPotDecorationsState, String> {
    const SIDE_COUNT: usize = 4;

    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    if values.len() > SIDE_COUNT {
        return Err(format!(
            "{what} has {} decorations, maximum is {SIDE_COUNT}",
            values.len()
        ));
    }
    let mut decorations = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            decode_builtin_item_holder_reference(value, &format!("{what}[{index}]"))
                .map(|item| (item.resource_id != "minecraft:brick").then_some(item))
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter();
    Ok(DialogPotDecorationsState {
        back: decorations.next().flatten(),
        left: decorations.next().flatten(),
        right: decorations.next().flatten(),
        front: decorations.next().flatten(),
    })
}

fn decode_builtin_item_holder_reference(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} holder is not a resource id"));
    };
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let item = vanilla_item_type_for_resource_id(&resource_id)
        .ok_or_else(|| format!("missing minecraft:item resource {resource_id} for {what}"))?;
    Ok(DialogRegistryHolderReferenceState {
        holder_id: item.id,
        resource_id: item.resource_id.to_string(),
    })
}

fn decode_suspicious_stew_effects(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogSuspiciousStewEffectState>, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let entry_what = format!("{what}[{index}]");
            let NbtSummaryValue::Compound(entries) = value else {
                return Err(format!("{entry_what} is not a compound"));
            };
            let id = entries
                .iter()
                .rev()
                .find(|entry| entry.name == "id")
                .map(|entry| &entry.value)
                .ok_or_else(|| format!("{entry_what} is missing field id"))?;
            let duration = entries
                .iter()
                .rev()
                .find(|entry| entry.name == "duration")
                .map(|entry| {
                    super::component_i32(&entry.value, &format!("{entry_what} field duration"))
                        .unwrap_or(160)
                })
                .unwrap_or(160);
            Ok(DialogSuspiciousStewEffectState {
                effect: decode_builtin_mob_effect_holder_reference(
                    id,
                    &format!("{entry_what} field id"),
                )?,
                duration,
            })
        })
        .collect()
}

pub(super) fn decode_builtin_mob_effect_holder_reference(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} holder is not a resource id"));
    };
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let effect = vanilla_mob_effect_type_for_resource_id(&resource_id)
        .ok_or_else(|| format!("missing minecraft:mob_effect resource {resource_id} for {what}"))?;
    Ok(DialogRegistryHolderReferenceState {
        holder_id: effect.id,
        resource_id: effect.resource_id.to_string(),
    })
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
        .rev()
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
