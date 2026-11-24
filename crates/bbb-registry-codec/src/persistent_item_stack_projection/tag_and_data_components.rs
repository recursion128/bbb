//! Persistent projections for `minecraft:banner_patterns`,
//! `minecraft:provides_banner_patterns`, `minecraft:debug_stick_state`,
//! `minecraft:entity_data`, `minecraft:block_entity_data`, and `minecraft:bees`.

use bbb_facts::block_entity_types::{
    vanilla_block_entity_type, vanilla_block_entity_type_for_resource_id,
};
use bbb_facts::builtin_enum_component_values::builtin_enum_component_value_name;
use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::entry as field;
use bbb_protocol::packets::{BeeOccupantSummary, DataComponentPatchSummary};
use bbb_protocol::NbtSummaryValue;

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::holder::{
    entity_id_in_registry, entity_resource_id_in_registry, registered_holder_name,
    resolve_holder_id, BANNER_PATTERN_REGISTRY,
};
use super::nbt_support::{
    compound_fields, latest_field, list_entries, nbt_string, normalize_resource_id,
    persistent_compact_holder_set_nbt, project_compact_holder_set, ComponentReader,
    MAX_PERSISTENT_LIST_ITEMS, MAX_PREALLOC_CAPACITY,
};

const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";
const BLOCK_ENTITY_TYPE_REGISTRY: &str = "minecraft:block_entity_type";

/// `minecraft:banner_patterns` --
/// `BannerPatternLayers.Layer.CODEC`.
pub(super) fn persistent_banner_patterns_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    let layers = patch
        .banner_pattern_layers
        .iter()
        .map(|layer| {
            let color = builtin_enum_component_value_name(
                VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                layer.color_id,
            )
            .ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("unknown dye colour id {}", layer.color_id),
                )
            })?;
            let pattern = match (&layer.registry_id, &layer.asset_id) {
                (Some(holder_id), None) => NbtSummaryValue::String(
                    registered_holder_name(
                        type_id,
                        resource_id,
                        BANNER_PATTERN_REGISTRY,
                        *holder_id,
                        holder_resource_id,
                    )?
                    .into(),
                ),
                (None, Some(asset_id)) => {
                    // `BannerPattern.DIRECT_CODEC`
                    // writes both fields and no registry name.
                    let translation_key = layer.translation_key.as_ref().ok_or_else(|| {
                        invalid_persistent_component(
                            type_id,
                            resource_id,
                            "a direct banner pattern carries no translation key".to_string(),
                        )
                    })?;
                    NbtSummaryValue::Compound(vec![
                        field(
                            "asset_id",
                            NbtSummaryValue::String(normalize_resource_id(asset_id)?.into()),
                        ),
                        field("translation_key", translation_key.to_nbt()),
                    ])
                }
                _ => {
                    return Err(invalid_persistent_component(
                        type_id,
                        resource_id,
                        "a banner pattern layer names neither a holder nor a direct pattern"
                            .to_string(),
                    ))
                }
            };
            Ok(NbtSummaryValue::Compound(vec![
                field("pattern", pattern),
                field("color", NbtSummaryValue::String(color.into())),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NbtSummaryValue::List(layers))
}

/// `minecraft:provides_banner_patterns` --
/// `RegistryCodecs.homogeneousList(Registries.BANNER_PATTERN)`
/// (`DataComponents.java:285`). The registry is a datapack one, so the names
/// come from the publication.
pub(super) fn persistent_provides_banner_patterns_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    let names = patch
        .provides_banner_pattern_registry_ids
        .iter()
        .map(|&holder_id| {
            registered_holder_name(
                type_id,
                resource_id,
                BANNER_PATTERN_REGISTRY,
                holder_id,
                holder_resource_id,
            )
        });
    persistent_compact_holder_set_nbt(
        type_id,
        resource_id,
        patch.provides_banner_pattern_tag.as_deref(),
        names,
    )
}

pub(super) fn project_provides_banner_patterns(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (registry_ids, tag) =
        project_compact_holder_set(ComponentReader::new(type_id, resource_id), value, |named| {
            resolve_holder_id(
                BANNER_PATTERN_REGISTRY,
                &super::nbt_support::normalize_resource_id_units(named.units())?,
                holder_id,
            )
        })?;
    patch.provides_banner_pattern_registry_ids = registry_ids;
    patch.provides_banner_pattern_tag = tag;
    Ok(())
}

/// `minecraft:debug_stick_state` -- `Codec.dispatchedMap` over
/// `BuiltInRegistries.BLOCK.holderByNameCodec()`,
/// keyed by the block's identifier with the property's name as the value.
pub(super) fn persistent_debug_stick_state_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    bbb_protocol::packets::data_components::catalog::canonical_debug_stick_properties_nbt_with_registry(
        &patch.debug_stick_properties,
        content_registry,
    )
    .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))
}

pub(super) fn project_debug_stick_state(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    patch.debug_stick_properties =
        bbb_protocol::packets::data_components::catalog::decode_debug_stick_properties_with_registry(
            value,
            content_registry,
        )
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))?;
    patch
        .debug_stick_properties
        .sort_by_key(|property| property.block_registry_id);
    Ok(())
}

/// `minecraft:entity_data` / `minecraft:block_entity_data` --
/// `TypedEntityData.codec` (`TypedEntityData.java:39-67`), the retained
/// compound with an `id` naming the type.
fn persistent_typed_entity_data_nbt(
    type_id: i32,
    resource_id: &'static str,
    named_type: &str,
    tag: Option<&NbtSummaryValue>,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let NbtSummaryValue::Compound(entries) =
        tag.ok_or(ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        })?
    else {
        return Err(invalid("typed entity data is not a compound".to_string()));
    };
    // The network tag must not carry a top-level `id`
    // (`catalog/encode.rs`'s `encode_typed_entity_data`), so a summary holding
    // one has no save form that would not shadow it.
    if entries.iter().any(|entry| entry.name == "id") {
        return Err(invalid(
            "typed entity data already carries a top-level id".to_string(),
        ));
    }
    let mut fields = Vec::with_capacity(entries.len() + 1);
    fields.push(field("id", NbtSummaryValue::String(named_type.into())));
    fields.extend(entries.iter().cloned());
    Ok(NbtSummaryValue::Compound(fields))
}

/// The `id` field split back out of a typed entity data compound.
fn project_typed_entity_data(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<(String, NbtSummaryValue)> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let value = super::component_sets::custom_data_compound(value)
        .ok_or_else(|| invalid("typed entity data is not a compound".to_string()))?;
    let fields = compound_fields(&value).expect("custom data is a compound");
    let id = latest_field(fields, "id")
        .ok_or_else(|| invalid("typed entity data carries no id".to_string()))?;
    let id = nbt_string(id)
        .ok_or_else(|| invalid("typed entity data id is not a name".to_string()))?
        .to_string();
    let rest = fields
        .iter()
        .filter(|entry| entry.name != "id")
        .cloned()
        .collect();
    Ok((id, NbtSummaryValue::Compound(rest)))
}

fn project_entity_data(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (id, tag) = project_typed_entity_data(type_id, resource_id, value)?;
    let id = normalize_resource_id(&id)?;
    let entity_id = entity_id_in_registry(&id, content_registry).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            format!("unknown {ENTITY_TYPE_REGISTRY} holder {id}"),
        )
    })?;
    patch.entity_data_entity_type_id = Some(entity_id);
    patch.entity_data_tag = Some(tag);
    Ok(())
}

fn project_block_entity_data(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (id, tag) = project_typed_entity_data(type_id, resource_id, value)?;
    let id = normalize_resource_id(&id)?;
    let block_entity = vanilla_block_entity_type_for_resource_id(&id).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            format!("unknown {BLOCK_ENTITY_TYPE_REGISTRY} holder {id}"),
        )
    })?;
    patch.block_entity_data_type_id = Some(block_entity.id);
    patch.block_entity_spawn_entity_type =
        bbb_protocol::packets::data_components::typed_entity_data_spawn_entity_type(Some(&tag));
    patch.block_entity_data_tag = Some(tag);
    Ok(())
}

/// `minecraft:bees` -- `BeehiveBlockEntity.Occupant.LIST_CODEC`
pub(super) fn persistent_bees_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    if !patch.bees_present {
        return Err(ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        });
    }
    let occupants = patch
        .bees
        .iter()
        .map(|occupant| {
            let entity = entity_resource_id_in_registry(occupant.entity_type_id, content_registry)
                .ok_or_else(|| {
                    invalid_persistent_component(
                        type_id,
                        resource_id,
                        format!(
                            "unknown {ENTITY_TYPE_REGISTRY} holder id {}",
                            occupant.entity_type_id
                        ),
                    )
                })?;
            Ok(NbtSummaryValue::Compound(vec![
                field(
                    "entity_data",
                    persistent_typed_entity_data_nbt(
                        type_id,
                        resource_id,
                        entity,
                        occupant.entity_data_tag.as_ref(),
                    )?,
                ),
                field(
                    "ticks_in_hive",
                    NbtSummaryValue::Int(occupant.ticks_in_hive),
                ),
                field(
                    "min_ticks_in_hive",
                    NbtSummaryValue::Int(occupant.min_ticks_in_hive),
                ),
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NbtSummaryValue::List(occupants))
}

pub(super) fn project_bees(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let entries = list_entries(value).ok_or_else(|| invalid("bees is not a list".to_string()))?;
    if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(invalid(format!(
            "bees list has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            entries.len()
        )));
    }
    let mut bees = Vec::with_capacity(entries.len().min(MAX_PREALLOC_CAPACITY));
    for entry in entries.iter() {
        let fields = compound_fields(&entry)
            .ok_or_else(|| invalid("a bee occupant is not a compound".to_string()))?;
        let (id, tag) = project_typed_entity_data(
            type_id,
            resource_id,
            latest_field(fields, "entity_data")
                .ok_or_else(|| invalid("a bee occupant carries no entity_data".to_string()))?,
        )?;
        let id = normalize_resource_id(&id)?;
        let entity_type_id = entity_id_in_registry(&id, content_registry)
            .ok_or_else(|| invalid(format!("unknown {ENTITY_TYPE_REGISTRY} holder {id}")))?;
        let int = |name: &str| -> Result<i32> {
            latest_field(fields, name)
                .and_then(bbb_protocol::nbt_summary::codec_int_value)
                .ok_or_else(|| invalid(format!("a bee occupant has no {name}")))
        };
        bees.push(BeeOccupantSummary {
            entity_type_id,
            entity_data_tag: Some(tag),
            ticks_in_hive: int("ticks_in_hive")?,
            min_ticks_in_hive: int("min_ticks_in_hive")?,
        });
    }
    patch.bees_present = true;
    patch.bees = bees;
    Ok(())
}

/// The four components in this module that write one `TypedEntityData`.
pub(super) fn persistent_typed_data_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    match type_id {
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => {
            let entity_type_id = patch.entity_data_entity_type_id.ok_or(
                ItemStackPersistentProjectionError::UnsupportedComponent {
                    type_id,
                    resource_id,
                },
            )?;
            let entity = entity_resource_id_in_registry(entity_type_id, content_registry)
                .ok_or_else(|| {
                    invalid_persistent_component(
                        type_id,
                        resource_id,
                        format!("unknown {ENTITY_TYPE_REGISTRY} holder id {entity_type_id}"),
                    )
                })?;
            persistent_typed_entity_data_nbt(
                type_id,
                resource_id,
                entity,
                patch.entity_data_tag.as_ref(),
            )
        }
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
            let block_entity_type_id = patch.block_entity_data_type_id.ok_or(
                ItemStackPersistentProjectionError::UnsupportedComponent {
                    type_id,
                    resource_id,
                },
            )?;
            let block_entity =
                vanilla_block_entity_type(block_entity_type_id).ok_or_else(|| {
                    invalid_persistent_component(
                        type_id,
                        resource_id,
                        format!(
                            "unknown {BLOCK_ENTITY_TYPE_REGISTRY} holder id {block_entity_type_id}"
                        ),
                    )
                })?;
            persistent_typed_entity_data_nbt(
                type_id,
                resource_id,
                block_entity.resource_id,
                patch.block_entity_data_tag.as_ref(),
            )
        }
        _ => unreachable!("typed data dispatch is guarded by the caller"),
    }
}

pub(super) fn project_typed_data(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    match type_id {
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => {
            project_entity_data(type_id, resource_id, value, content_registry, patch)
        }
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
            project_block_entity_data(type_id, resource_id, value, patch)
        }
        _ => unreachable!("typed data dispatch is guarded by the caller"),
    }
}
