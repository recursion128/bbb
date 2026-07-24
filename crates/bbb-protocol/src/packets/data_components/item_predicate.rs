use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    DataComponentPredicateTypeSummary, PartialDataComponentPredicateSummary,
    MAX_DATA_COMPONENT_LIST_ITEMS, MAX_DATA_COMPONENT_PREDICATE_ENTRIES,
    MAX_PARTIAL_DATA_COMPONENT_PREDICATES,
};
use crate::{
    codec::{ProtocolError, Result},
    data_component_types::{
        vanilla_data_component_predicate_resource_id, vanilla_data_component_type,
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
    },
    item_types::{vanilla_item_type, vanilla_item_type_for_resource_id},
    normalize_resource_location, NbtSummaryEntry, NbtSummaryValue,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemHolderSetSummary {
    #[serde(default)]
    pub registry_ids: Vec<i32>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentDataComponentValueSummary {
    pub component_type_id: i32,
    pub value: NbtSummaryValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentItemPredicateSummary {
    #[serde(default)]
    pub items: Option<ItemHolderSetSummary>,
    #[serde(default)]
    pub count_min: Option<i32>,
    #[serde(default)]
    pub count_max: Option<i32>,
    #[serde(default)]
    pub exact_components: Vec<PersistentDataComponentValueSummary>,
    #[serde(default)]
    pub partial_components: Vec<PartialDataComponentPredicateSummary>,
}

pub fn decode_persistent_item_predicate_summary(
    value: &NbtSummaryValue,
) -> Result<PersistentItemPredicateSummary> {
    let entries = compound_entries(value, "item predicate")?;
    let items = last_field(entries, "items")
        .map(decode_item_holder_set)
        .transpose()?;
    let (count_min, count_max) = last_field(entries, "count")
        .map(decode_int_bounds)
        .transpose()?
        .unwrap_or((None, None));
    let exact_components = last_field(entries, "components")
        .map(decode_exact_components)
        .transpose()?
        .unwrap_or_default();
    let partial_components = last_field(entries, "predicates")
        .map(decode_partial_components)
        .transpose()?
        .unwrap_or_default();
    Ok(PersistentItemPredicateSummary {
        items,
        count_min,
        count_max,
        exact_components,
        partial_components,
    })
}

pub fn encode_persistent_item_predicate_summary(
    predicate: &PersistentItemPredicateSummary,
) -> Result<NbtSummaryValue> {
    let mut entries = Vec::new();
    if let Some(items) = &predicate.items {
        entries.push(NbtSummaryEntry {
            name: "items".to_string(),
            value: encode_item_holder_set(items)?,
        });
    }
    if predicate.count_min.is_some() || predicate.count_max.is_some() {
        entries.push(NbtSummaryEntry {
            name: "count".to_string(),
            value: encode_int_bounds(predicate.count_min, predicate.count_max)?,
        });
    }
    if !predicate.exact_components.is_empty() {
        entries.push(NbtSummaryEntry {
            name: "components".to_string(),
            value: encode_exact_components(&predicate.exact_components)?,
        });
    }
    if !predicate.partial_components.is_empty() {
        entries.push(NbtSummaryEntry {
            name: "predicates".to_string(),
            value: encode_partial_components(&predicate.partial_components)?,
        });
    }
    Ok(NbtSummaryValue::Compound(entries))
}

fn decode_item_holder_set(value: &NbtSummaryValue) -> Result<ItemHolderSetSummary> {
    match value {
        NbtSummaryValue::String(raw) if raw.starts_with('#') => {
            let tag = normalize_resource_location(raw[1..].to_string())?;
            Ok(ItemHolderSetSummary {
                tag: Some(tag),
                ..ItemHolderSetSummary::default()
            })
        }
        NbtSummaryValue::String(raw) => Ok(ItemHolderSetSummary {
            registry_ids: vec![item_registry_id(raw)?],
            tag: None,
        }),
        NbtSummaryValue::List(values) => {
            check_len(values.len(), MAX_DATA_COMPONENT_LIST_ITEMS)?;
            let registry_ids = values
                .iter()
                .map(|value| {
                    let NbtSummaryValue::String(raw) = value else {
                        return Err(ProtocolError::InvalidData(
                            "item predicate holder-set entry is not a resource id".to_string(),
                        ));
                    };
                    item_registry_id(raw)
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(ItemHolderSetSummary {
                registry_ids,
                tag: None,
            })
        }
        _ => Err(ProtocolError::InvalidData(
            "item predicate holder set is neither a tag, resource id, nor list".to_string(),
        )),
    }
}

fn encode_item_holder_set(items: &ItemHolderSetSummary) -> Result<NbtSummaryValue> {
    if let Some(tag) = &items.tag {
        if !items.registry_ids.is_empty() {
            return Err(ProtocolError::InvalidData(
                "item predicate holder set mixes a tag and direct items".to_string(),
            ));
        }
        let canonical = normalize_resource_location(tag.clone())?;
        if canonical != *tag {
            return Err(ProtocolError::InvalidData(format!(
                "item predicate tag is not canonical: {tag}"
            )));
        }
        return Ok(NbtSummaryValue::String(format!("#{tag}")));
    }
    check_len(items.registry_ids.len(), MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let resource_ids = items
        .registry_ids
        .iter()
        .map(|registry_id| {
            vanilla_item_type(*registry_id)
                .map(|item| item.resource_id)
                .ok_or_else(|| {
                    ProtocolError::InvalidData(format!(
                        "item predicate has invalid item holder id {registry_id}"
                    ))
                })
        })
        .collect::<Result<Vec<_>>>()?;
    if let [resource_id] = resource_ids.as_slice() {
        return Ok(NbtSummaryValue::String((*resource_id).to_string()));
    }
    Ok(NbtSummaryValue::List(
        resource_ids
            .into_iter()
            .map(|resource_id| NbtSummaryValue::String(resource_id.to_string()))
            .collect(),
    ))
}

fn item_registry_id(raw: &str) -> Result<i32> {
    let resource_id = normalize_resource_location(raw.to_string())?;
    vanilla_item_type_for_resource_id(&resource_id)
        .map(|item| item.id)
        .ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "item predicate references unknown item {resource_id}"
            ))
        })
}

fn decode_int_bounds(value: &NbtSummaryValue) -> Result<(Option<i32>, Option<i32>)> {
    let bounds = match value {
        NbtSummaryValue::Compound(entries) => (
            last_field(entries, "min").map(nbt_i32).transpose()?,
            last_field(entries, "max").map(nbt_i32).transpose()?,
        ),
        _ => {
            let exact = nbt_i32(value)?;
            (Some(exact), Some(exact))
        }
    };
    validate_int_bounds(bounds.0, bounds.1)?;
    Ok(bounds)
}

fn encode_int_bounds(min: Option<i32>, max: Option<i32>) -> Result<NbtSummaryValue> {
    validate_int_bounds(min, max)?;
    if let (Some(min), Some(max)) = (min, max) {
        if min == max {
            return Ok(NbtSummaryValue::Int(min));
        }
    }
    let mut entries = Vec::new();
    if let Some(min) = min {
        entries.push(NbtSummaryEntry {
            name: "min".to_string(),
            value: NbtSummaryValue::Int(min),
        });
    }
    if let Some(max) = max {
        entries.push(NbtSummaryEntry {
            name: "max".to_string(),
            value: NbtSummaryValue::Int(max),
        });
    }
    Ok(NbtSummaryValue::Compound(entries))
}

fn validate_int_bounds(min: Option<i32>, max: Option<i32>) -> Result<()> {
    if matches!((min, max), (Some(min), Some(max)) if min > max) {
        return Err(ProtocolError::InvalidData(
            "item predicate count has swapped bounds".to_string(),
        ));
    }
    Ok(())
}

fn nbt_i32(value: &NbtSummaryValue) -> Result<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Short(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Int(value) => Ok(*value),
        NbtSummaryValue::Long(value) => Ok(*value as i32),
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits) as i32),
        NbtSummaryValue::Double(bits) => Ok(f64::from_bits(*bits) as i32),
        _ => Err(ProtocolError::InvalidData(
            "item predicate count bound is not numeric".to_string(),
        )),
    }
}

fn decode_exact_components(
    value: &NbtSummaryValue,
) -> Result<Vec<PersistentDataComponentValueSummary>> {
    let entries =
        canonical_map_entries(compound_entries(value, "item predicate exact components")?);
    check_len(entries.len(), MAX_DATA_COMPONENT_PREDICATE_ENTRIES)?;
    let mut seen = BTreeSet::new();
    entries
        .into_iter()
        .map(|entry| {
            let resource_id = normalize_resource_location(entry.name.clone())?;
            let component =
                vanilla_data_component_type_for_resource_id(&resource_id).ok_or_else(|| {
                    ProtocolError::InvalidData(format!(
                        "item predicate references unknown exact component {resource_id}"
                    ))
                })?;
            if !component.persistent {
                return Err(ProtocolError::InvalidData(format!(
                    "item predicate exact component {resource_id} is not persistent"
                )));
            }
            if !seen.insert(component.type_id) {
                return Err(ProtocolError::InvalidData(format!(
                    "item predicate contains duplicate exact component {resource_id}"
                )));
            }
            Ok(PersistentDataComponentValueSummary {
                component_type_id: component.type_id,
                value: entry.value.clone(),
            })
        })
        .collect()
}

fn encode_exact_components(
    components: &[PersistentDataComponentValueSummary],
) -> Result<NbtSummaryValue> {
    check_len(components.len(), MAX_DATA_COMPONENT_PREDICATE_ENTRIES)?;
    let mut seen = BTreeSet::new();
    let entries = components
        .iter()
        .map(|component| {
            let component_type = vanilla_data_component_type(component.component_type_id)
                .ok_or_else(|| {
                    ProtocolError::InvalidData(format!(
                        "item predicate has invalid exact component type id {}",
                        component.component_type_id
                    ))
                })?;
            if !component_type.persistent {
                return Err(ProtocolError::InvalidData(format!(
                    "item predicate exact component {} is not persistent",
                    component_type.resource_id
                )));
            }
            if !seen.insert(component.component_type_id) {
                return Err(ProtocolError::InvalidData(format!(
                    "item predicate contains duplicate exact component {}",
                    component_type.resource_id
                )));
            }
            Ok(NbtSummaryEntry {
                name: component_type.resource_id.to_string(),
                value: component.value.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NbtSummaryValue::Compound(entries))
}

fn decode_partial_components(
    value: &NbtSummaryValue,
) -> Result<Vec<PartialDataComponentPredicateSummary>> {
    let entries = canonical_map_entries(compound_entries(
        value,
        "item predicate partial components",
    )?);
    check_len(entries.len(), MAX_PARTIAL_DATA_COMPONENT_PREDICATES)?;
    let mut seen = BTreeSet::new();
    entries
        .into_iter()
        .map(|entry| {
            let resource_id = normalize_resource_location(entry.name.clone())?;
            let (predicate_type, wire_value) = if let Some(predicate) =
                VANILLA_DATA_COMPONENT_PREDICATE_TYPES
                    .iter()
                    .find(|predicate| predicate.resource_id == resource_id)
            {
                (
                    DataComponentPredicateTypeSummary::Concrete(predicate.type_id),
                    entry.value.clone(),
                )
            } else {
                let component = vanilla_data_component_type_for_resource_id(&resource_id)
                    .ok_or_else(|| {
                        ProtocolError::InvalidData(format!(
                            "item predicate references unknown partial component {resource_id}"
                        ))
                    })?;
                (
                    DataComponentPredicateTypeSummary::AnyComponent(component.type_id),
                    canonical_any_component_value(component.type_id, &entry.value)?,
                )
            };
            if !seen.insert(resource_id.clone()) {
                return Err(ProtocolError::InvalidData(format!(
                    "item predicate contains duplicate partial component {resource_id}"
                )));
            }
            Ok(PartialDataComponentPredicateSummary {
                predicate_type,
                wire_value: Some(wire_value),
            })
        })
        .collect()
}

fn encode_partial_components(
    predicates: &[PartialDataComponentPredicateSummary],
) -> Result<NbtSummaryValue> {
    check_len(predicates.len(), MAX_PARTIAL_DATA_COMPONENT_PREDICATES)?;
    let mut seen = BTreeSet::new();
    let entries = predicates
        .iter()
        .map(|predicate| {
            let value = predicate.wire_value.as_ref().ok_or_else(|| {
                ProtocolError::InvalidData(
                    "item predicate partial component is missing a value".to_string(),
                )
            })?;
            let resource_id = match predicate.predicate_type {
                DataComponentPredicateTypeSummary::Concrete(type_id) => {
                    vanilla_data_component_predicate_resource_id(type_id).ok_or_else(|| {
                        ProtocolError::InvalidData(format!(
                            "item predicate has invalid concrete predicate type id {type_id}"
                        ))
                    })?
                }
                DataComponentPredicateTypeSummary::AnyComponent(type_id) => {
                    let component = vanilla_data_component_type(type_id).ok_or_else(|| {
                        ProtocolError::InvalidData(format!(
                            "item predicate has invalid any-component type id {type_id}"
                        ))
                    })?;
                    if VANILLA_DATA_COMPONENT_PREDICATE_TYPES
                        .iter()
                        .any(|predicate| predicate.resource_id == component.resource_id)
                    {
                        return Err(ProtocolError::InvalidData(format!(
                            "item predicate any-component {} collides with a concrete predicate",
                            component.resource_id
                        )));
                    }
                    require_empty_any_component_value(type_id, value)?;
                    component.resource_id
                }
            };
            if !seen.insert(resource_id) {
                return Err(ProtocolError::InvalidData(format!(
                    "item predicate contains duplicate partial component {resource_id}"
                )));
            }
            Ok(NbtSummaryEntry {
                name: resource_id.to_string(),
                value: value.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NbtSummaryValue::Compound(entries))
}

fn require_empty_any_component_value(type_id: i32, value: &NbtSummaryValue) -> Result<()> {
    if !matches!(value, NbtSummaryValue::Compound(entries) if entries.is_empty()) {
        return Err(ProtocolError::InvalidData(format!(
            "item predicate any-component type id {type_id} must use an empty compound"
        )));
    }
    Ok(())
}

fn canonical_any_component_value(type_id: i32, value: &NbtSummaryValue) -> Result<NbtSummaryValue> {
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(format!(
            "item predicate any-component type id {type_id} is not a compound"
        )));
    }
    Ok(NbtSummaryValue::Compound(Vec::new()))
}

fn canonical_map_entries(entries: &[NbtSummaryEntry]) -> Vec<&NbtSummaryEntry> {
    let mut canonical = Vec::<&NbtSummaryEntry>::with_capacity(entries.len());
    for entry in entries {
        if let Some(existing) = canonical
            .iter_mut()
            .find(|existing| existing.name == entry.name)
        {
            *existing = entry;
        } else {
            canonical.push(entry);
        }
    }
    canonical
}

fn compound_entries<'a>(value: &'a NbtSummaryValue, what: &str) -> Result<&'a [NbtSummaryEntry]> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(format!(
            "{what} is not a compound"
        )));
    };
    Ok(entries)
}

fn last_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn check_len(actual: usize, max: usize) -> Result<()> {
    if actual > max {
        return Err(ProtocolError::PacketTooLarge(actual, max));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_component_types::{
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    };
    use crate::item_types::{VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_STONE_ID};

    #[test]
    fn persistent_item_predicate_round_trips_canonical_codec_fields() {
        let value = compound(vec![
            entry(
                "items",
                NbtSummaryValue::List(vec![string("stone"), string("minecraft:diamond")]),
            ),
            entry(
                "count",
                compound(vec![
                    entry("min", NbtSummaryValue::Byte(1)),
                    entry("max", NbtSummaryValue::Long(3)),
                ]),
            ),
            entry(
                "components",
                compound(vec![
                    entry("max_stack_size", NbtSummaryValue::Int(16)),
                    entry("minecraft:custom_name", string("Locked")),
                ]),
            ),
            entry(
                "predicates",
                compound(vec![
                    entry("damage", compound(Vec::new())),
                    entry("minecraft:lock", compound(Vec::new())),
                ]),
            ),
            entry("ignored", NbtSummaryValue::Int(1)),
        ]);

        let decoded = decode_persistent_item_predicate_summary(&value).unwrap();
        assert_eq!(
            decoded,
            PersistentItemPredicateSummary {
                items: Some(ItemHolderSetSummary {
                    registry_ids: vec![VANILLA_ITEM_STONE_ID, VANILLA_ITEM_DIAMOND_ID],
                    tag: None,
                }),
                count_min: Some(1),
                count_max: Some(3),
                exact_components: vec![
                    PersistentDataComponentValueSummary {
                        component_type_id: VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                        value: NbtSummaryValue::Int(16),
                    },
                    PersistentDataComponentValueSummary {
                        component_type_id: VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                        value: string("Locked"),
                    },
                ],
                partial_components: vec![
                    PartialDataComponentPredicateSummary {
                        predicate_type: DataComponentPredicateTypeSummary::Concrete(0),
                        wire_value: Some(compound(Vec::new())),
                    },
                    PartialDataComponentPredicateSummary {
                        predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
                            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                        ),
                        wire_value: Some(compound(Vec::new())),
                    },
                ],
            }
        );

        assert_eq!(
            encode_persistent_item_predicate_summary(&decoded).unwrap(),
            compound(vec![
                entry(
                    "items",
                    NbtSummaryValue::List(vec![
                        string("minecraft:stone"),
                        string("minecraft:diamond"),
                    ]),
                ),
                entry(
                    "count",
                    compound(vec![
                        entry("min", NbtSummaryValue::Int(1)),
                        entry("max", NbtSummaryValue::Int(3)),
                    ]),
                ),
                entry(
                    "components",
                    compound(vec![
                        entry("minecraft:max_stack_size", NbtSummaryValue::Int(16)),
                        entry("minecraft:custom_name", string("Locked")),
                    ]),
                ),
                entry(
                    "predicates",
                    compound(vec![
                        entry("minecraft:damage", compound(Vec::new())),
                        entry("minecraft:lock", compound(Vec::new())),
                    ]),
                ),
            ])
        );
    }

    #[test]
    fn persistent_item_predicate_compacts_singletons_and_defaults() {
        let decoded = decode_persistent_item_predicate_summary(&compound(vec![
            entry("items", string("#tools")),
            entry("count", NbtSummaryValue::Short(2)),
            entry("components", compound(Vec::new())),
            entry("predicates", compound(Vec::new())),
        ]))
        .unwrap();
        assert_eq!(
            encode_persistent_item_predicate_summary(&decoded).unwrap(),
            compound(vec![
                entry("items", string("#minecraft:tools")),
                entry("count", NbtSummaryValue::Int(2)),
            ])
        );

        let singleton = PersistentItemPredicateSummary {
            items: Some(ItemHolderSetSummary {
                registry_ids: vec![VANILLA_ITEM_STONE_ID],
                tag: None,
            }),
            ..PersistentItemPredicateSummary::default()
        };
        assert_eq!(
            encode_persistent_item_predicate_summary(&singleton).unwrap(),
            compound(vec![entry("items", string("minecraft:stone"))])
        );
    }

    #[test]
    fn persistent_item_predicate_rejects_invalid_codec_shapes() {
        for value in [
            NbtSummaryValue::List(Vec::new()),
            compound(vec![entry("items", string("minecraft:missing"))]),
            compound(vec![entry(
                "items",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
            )]),
            compound(vec![entry(
                "count",
                compound(vec![
                    entry("min", NbtSummaryValue::Int(2)),
                    entry("max", NbtSummaryValue::Int(1)),
                ]),
            )]),
            compound(vec![entry(
                "components",
                compound(vec![entry(
                    "minecraft:creative_slot_lock",
                    compound(Vec::new()),
                )]),
            )]),
            compound(vec![entry(
                "predicates",
                compound(vec![entry("minecraft:lock", NbtSummaryValue::Int(1))]),
            )]),
            compound(vec![entry(
                "predicates",
                compound(vec![entry("minecraft:missing", compound(Vec::new()))]),
            )]),
            compound(vec![entry(
                "components",
                compound(vec![
                    entry("max_stack_size", NbtSummaryValue::Int(16)),
                    entry("minecraft:max_stack_size", NbtSummaryValue::Int(32)),
                ]),
            )]),
        ] {
            assert!(
                decode_persistent_item_predicate_summary(&value).is_err(),
                "accepted invalid item predicate {value:?}"
            );
        }
    }

    #[test]
    fn persistent_item_predicate_encoder_rejects_noncanonical_summaries() {
        for predicate in [
            PersistentItemPredicateSummary {
                items: Some(ItemHolderSetSummary {
                    registry_ids: vec![VANILLA_ITEM_STONE_ID],
                    tag: Some("minecraft:tools".to_string()),
                }),
                ..PersistentItemPredicateSummary::default()
            },
            PersistentItemPredicateSummary {
                count_min: Some(2),
                count_max: Some(1),
                ..PersistentItemPredicateSummary::default()
            },
            PersistentItemPredicateSummary {
                exact_components: vec![PersistentDataComponentValueSummary {
                    component_type_id: 20,
                    value: compound(Vec::new()),
                }],
                ..PersistentItemPredicateSummary::default()
            },
            PersistentItemPredicateSummary {
                partial_components: vec![PartialDataComponentPredicateSummary {
                    predicate_type: DataComponentPredicateTypeSummary::AnyComponent(3),
                    wire_value: Some(compound(Vec::new())),
                }],
                ..PersistentItemPredicateSummary::default()
            },
            PersistentItemPredicateSummary {
                partial_components: vec![PartialDataComponentPredicateSummary {
                    predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
                        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                    ),
                    wire_value: Some(NbtSummaryValue::Int(1)),
                }],
                ..PersistentItemPredicateSummary::default()
            },
        ] {
            assert!(
                encode_persistent_item_predicate_summary(&predicate).is_err(),
                "encoded invalid item predicate {predicate:?}"
            );
        }
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    fn string(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::String(value.to_string())
    }
}
