use std::cmp::Ordering;
use std::collections::BTreeSet;

use bbb_protocol::{
    attribute_modifier_types::attribute_modifier_operation_id,
    data_component_types::{
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
    },
    equipment_slots::vanilla_equipment_slot_group_for_serialized_name,
    item_component_value_types::FireworkExplosionShapeSummary,
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{registries::normalize_codec_identifier, RegistrySet};

use super::super::super::{decode_component, DialogDecodeContext};
use super::{
    component_bool, component_f64_bits, component_i32,
    decode_supported_component_value_with_context, find_entry,
    holder_set::{decode_registry_holder_set, DialogRegistryHolderSetState},
    typed_data::decode_custom_data,
    DialogDataComponentValueState, DialogTextState,
};

const ATTRIBUTE_REGISTRY: &str = "minecraft:attribute";
const BLOCK_REGISTRY: &str = "minecraft:block";
const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";
const ITEM_REGISTRY: &str = "minecraft:item";
const JUKEBOX_SONG_REGISTRY: &str = "minecraft:jukebox_song";
const POTION_REGISTRY: &str = "minecraft:potion";
const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
const TRIM_PATTERN_REGISTRY: &str = "minecraft:trim_pattern";
const VILLAGER_TYPE_REGISTRY: &str = "minecraft:villager_type";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogAdventureModePredicateState {
    pub predicates: Vec<DialogBlockPredicateState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBlockPredicateState {
    #[serde(default)]
    pub blocks: Option<DialogRegistryHolderSetState>,
    #[serde(default)]
    pub state: Option<Vec<DialogStatePropertyMatcherState>>,
    #[serde(default)]
    pub nbt: Option<NbtSummaryValue>,
    pub components: DialogDataComponentMatchersState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogStatePropertyMatcherState {
    pub name: String,
    pub value: DialogStatePropertyValueMatcherState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogStatePropertyValueMatcherState {
    Exact {
        value: String,
    },
    Range {
        #[serde(default)]
        min: Option<String>,
        #[serde(default)]
        max: Option<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDataComponentMatchersState {
    #[serde(default)]
    pub exact: Vec<DialogExactDataComponentPredicateState>,
    #[serde(default)]
    pub partial: Vec<DialogPartialDataComponentPredicateState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogExactDataComponentPredicateState {
    pub component_type_id: i32,
    pub resource_id: String,
    pub value: DialogDataComponentValueState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogPartialDataComponentPredicateState {
    Concrete {
        predicate_type_id: i32,
        resource_id: String,
        value: DialogDataComponentPredicateState,
    },
    AnyComponent {
        component_type_id: i32,
        resource_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogDataComponentPredicateState {
    Damage {
        durability: DialogIntBoundsState,
        damage: DialogIntBoundsState,
    },
    Enchantments {
        enchantments: Vec<DialogEnchantmentPredicateState>,
    },
    StoredEnchantments {
        enchantments: Vec<DialogEnchantmentPredicateState>,
    },
    PotionContents {
        potions: DialogRegistryHolderSetState,
    },
    CustomData {
        value: NbtSummaryValue,
    },
    Container {
        #[serde(default)]
        items: Option<DialogCollectionPredicateState<DialogItemPredicateState>>,
    },
    BundleContents {
        #[serde(default)]
        items: Option<DialogCollectionPredicateState<DialogItemPredicateState>>,
    },
    FireworkExplosion {
        predicate: DialogFireworkExplosionPredicateState,
    },
    Fireworks {
        #[serde(default)]
        explosions: Option<DialogCollectionPredicateState<DialogFireworkExplosionPredicateState>>,
        flight_duration: DialogIntBoundsState,
    },
    WritableBookContent {
        #[serde(default)]
        pages: Option<DialogCollectionPredicateState<String>>,
    },
    WrittenBookContent {
        #[serde(default)]
        pages: Option<DialogCollectionPredicateState<DialogTextState>>,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        title: Option<String>,
        generation: DialogIntBoundsState,
        #[serde(default)]
        resolved: Option<bool>,
    },
    AttributeModifiers {
        #[serde(default)]
        modifiers:
            Option<DialogCollectionPredicateState<DialogAttributeModifierEntryPredicateState>>,
    },
    Trim {
        #[serde(default)]
        material: Option<DialogRegistryHolderSetState>,
        #[serde(default)]
        pattern: Option<DialogRegistryHolderSetState>,
    },
    JukeboxPlayable {
        #[serde(default)]
        song: Option<DialogRegistryHolderSetState>,
    },
    VillagerVariant {
        villager_types: DialogRegistryHolderSetState,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogIntBoundsState {
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub max: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDoubleBoundsState {
    #[serde(default)]
    pub min_bits: Option<u64>,
    #[serde(default)]
    pub max_bits: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogCollectionPredicateState<T> {
    pub contains: Option<Vec<T>>,
    pub count: Option<Vec<DialogCollectionCountPredicateState<T>>>,
    pub size: Option<DialogIntBoundsState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogCollectionCountPredicateState<T> {
    pub test: T,
    pub count: DialogIntBoundsState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogItemPredicateState {
    #[serde(default)]
    pub items: Option<DialogRegistryHolderSetState>,
    pub count: DialogIntBoundsState,
    pub components: DialogDataComponentMatchersState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogEnchantmentPredicateState {
    #[serde(default)]
    pub enchantments: Option<DialogRegistryHolderSetState>,
    pub levels: DialogIntBoundsState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogFireworkExplosionPredicateState {
    #[serde(default)]
    pub shape: Option<FireworkExplosionShapeSummary>,
    #[serde(default)]
    pub has_twinkle: Option<bool>,
    #[serde(default)]
    pub has_trail: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogAttributeModifierEntryPredicateState {
    #[serde(default)]
    pub attribute: Option<DialogRegistryHolderSetState>,
    #[serde(default)]
    pub modifier_id: Option<String>,
    pub amount: DialogDoubleBoundsState,
    #[serde(default)]
    pub operation_id: Option<i32>,
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub slot_group_id: Option<i32>,
    #[serde(default)]
    pub slot_group: Option<String>,
}

pub(super) fn decode_adventure_mode_predicate(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<DialogAdventureModePredicateState, String> {
    let predicates = match value {
        NbtSummaryValue::Compound(_) => std::slice::from_ref(value),
        NbtSummaryValue::List(values) if !values.is_empty() => values.as_slice(),
        NbtSummaryValue::List(_) => {
            return Err(format!("{what} must contain at least one block predicate"))
        }
        _ => {
            return Err(format!(
                "{what} is neither a block predicate nor a predicate list"
            ))
        }
    };
    Ok(DialogAdventureModePredicateState {
        predicates: predicates
            .iter()
            .enumerate()
            .map(|(index, value)| {
                decode_block_predicate(value, context, &format!("{what}[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub(super) fn decode_item_predicate(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<DialogItemPredicateState, String> {
    let entries = compound(value, what)?;
    let items = find_entry(entries, "items")
        .map(|value| decode_registry_holder_set(value, ITEM_REGISTRY, context.registries, what))
        .transpose()?;
    let count = find_entry(entries, "count")
        .map(|value| decode_int_bounds(value, &format!("{what} field count")))
        .transpose()?
        .unwrap_or_default();
    Ok(DialogItemPredicateState {
        items,
        count,
        components: decode_data_component_matchers(entries, context, what)?,
    })
}

fn decode_block_predicate(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<DialogBlockPredicateState, String> {
    let entries = compound(value, what)?;
    Ok(DialogBlockPredicateState {
        blocks: find_entry(entries, "blocks")
            .map(|value| {
                decode_registry_holder_set(value, BLOCK_REGISTRY, context.registries, what)
            })
            .transpose()?,
        state: find_entry(entries, "state")
            .map(|value| decode_state_properties(value, &format!("{what} field state")))
            .transpose()?,
        nbt: find_entry(entries, "nbt")
            .map(|value| decode_custom_data(value, &format!("{what} field nbt")))
            .transpose()?,
        components: decode_data_component_matchers(entries, context, what)?,
    })
}

fn decode_state_properties(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogStatePropertyMatcherState>, String> {
    compound(value, what)?
        .iter()
        .map(|entry| {
            let value = match &entry.value {
                NbtSummaryValue::String(value) => DialogStatePropertyValueMatcherState::Exact {
                    value: value.clone(),
                },
                NbtSummaryValue::Compound(entries) => DialogStatePropertyValueMatcherState::Range {
                    min: optional_string(entries, "min", what)?,
                    max: optional_string(entries, "max", what)?,
                },
                _ => {
                    return Err(format!(
                        "{what} property {} is neither a string nor a range",
                        entry.name
                    ))
                }
            };
            Ok(DialogStatePropertyMatcherState {
                name: entry.name.clone(),
                value,
            })
        })
        .collect()
}

fn decode_data_component_matchers(
    entries: &[NbtSummaryEntry],
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<DialogDataComponentMatchersState, String> {
    let exact = find_entry(entries, "components")
        .map(|value| decode_exact_component_predicates(value, context, what))
        .transpose()?
        .unwrap_or_default();
    let partial = find_entry(entries, "predicates")
        .map(|value| decode_partial_component_predicates(value, context, what))
        .transpose()?
        .unwrap_or_default();
    Ok(DialogDataComponentMatchersState { exact, partial })
}

fn decode_exact_component_predicates(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<Vec<DialogExactDataComponentPredicateState>, String> {
    let entries = compound(value, &format!("{what} field components"))?;
    let mut seen = BTreeSet::new();
    entries
        .iter()
        .map(|entry| {
            let resource_id = normalize_codec_identifier(&entry.name).ok_or_else(|| {
                format!("invalid {what} exact component resource id {}", entry.name)
            })?;
            let component_type = vanilla_data_component_type_for_resource_id(&resource_id)
                .ok_or_else(|| format!("unknown {what} exact component {resource_id}"))?;
            if !component_type.persistent {
                return Err(format!(
                    "{what} exact component {resource_id} is not persistent"
                ));
            }
            if !seen.insert(component_type.type_id) {
                return Err(format!("duplicate {what} exact component {resource_id}"));
            }
            let value = decode_supported_component_value_with_context(
                component_type.type_id,
                &resource_id,
                &entry.value,
                context,
            )?
            .ok_or_else(|| {
                format!("missing typed decoder for {what} exact component {resource_id}")
            })?;
            Ok(DialogExactDataComponentPredicateState {
                component_type_id: component_type.type_id,
                resource_id,
                value,
            })
        })
        .collect()
}

fn decode_partial_component_predicates(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<Vec<DialogPartialDataComponentPredicateState>, String> {
    let entries = compound(value, &format!("{what} field predicates"))?;
    let mut seen = BTreeSet::new();
    entries
        .iter()
        .map(|entry| {
            let resource_id = normalize_codec_identifier(&entry.name).ok_or_else(|| {
                format!(
                    "invalid {what} partial predicate resource id {}",
                    entry.name
                )
            })?;
            if let Some(predicate_type) = VANILLA_DATA_COMPONENT_PREDICATE_TYPES
                .iter()
                .find(|candidate| candidate.resource_id == resource_id)
            {
                if !seen.insert((0_u8, predicate_type.type_id)) {
                    return Err(format!("duplicate {what} partial predicate {resource_id}"));
                }
                return Ok(DialogPartialDataComponentPredicateState::Concrete {
                    predicate_type_id: predicate_type.type_id,
                    resource_id: resource_id.clone(),
                    value: decode_concrete_component_predicate(
                        predicate_type.resource_id,
                        &entry.value,
                        context,
                        &format!("{what} partial predicate {resource_id}"),
                    )?,
                });
            }
            let component_type = vanilla_data_component_type_for_resource_id(&resource_id)
                .ok_or_else(|| format!("unknown {what} partial predicate {resource_id}"))?;
            compound(
                &entry.value,
                &format!("{what} any-component predicate {resource_id}"),
            )?;
            if !seen.insert((1_u8, component_type.type_id)) {
                return Err(format!("duplicate {what} partial predicate {resource_id}"));
            }
            Ok(DialogPartialDataComponentPredicateState::AnyComponent {
                component_type_id: component_type.type_id,
                resource_id,
            })
        })
        .collect()
}

fn decode_concrete_component_predicate(
    resource_id: &str,
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    what: &str,
) -> Result<DialogDataComponentPredicateState, String> {
    match resource_id {
        "minecraft:damage" => {
            let entries = compound(value, what)?;
            Ok(DialogDataComponentPredicateState::Damage {
                durability: optional_int_bounds(entries, "durability", what)?.unwrap_or_default(),
                damage: optional_int_bounds(entries, "damage", what)?.unwrap_or_default(),
            })
        }
        "minecraft:enchantments" | "minecraft:stored_enchantments" => {
            let enchantments = decode_enchantment_predicates(value, context.registries, what)?;
            if resource_id == "minecraft:enchantments" {
                Ok(DialogDataComponentPredicateState::Enchantments { enchantments })
            } else {
                Ok(DialogDataComponentPredicateState::StoredEnchantments { enchantments })
            }
        }
        "minecraft:potion_contents" => Ok(DialogDataComponentPredicateState::PotionContents {
            potions: decode_registry_holder_set(value, POTION_REGISTRY, context.registries, what)?,
        }),
        "minecraft:custom_data" => Ok(DialogDataComponentPredicateState::CustomData {
            value: decode_custom_data(value, what)?,
        }),
        "minecraft:container" | "minecraft:bundle_contents" => {
            let entries = compound(value, what)?;
            let items = find_entry(entries, "items")
                .map(|value| {
                    decode_collection_predicate(value, what, &|value, nested_what| {
                        decode_item_predicate(value, context, nested_what)
                    })
                })
                .transpose()?;
            if resource_id == "minecraft:container" {
                Ok(DialogDataComponentPredicateState::Container { items })
            } else {
                Ok(DialogDataComponentPredicateState::BundleContents { items })
            }
        }
        "minecraft:firework_explosion" => {
            Ok(DialogDataComponentPredicateState::FireworkExplosion {
                predicate: decode_firework_explosion_predicate(value, what)?,
            })
        }
        "minecraft:fireworks" => {
            let entries = compound(value, what)?;
            Ok(DialogDataComponentPredicateState::Fireworks {
                explosions: find_entry(entries, "explosions")
                    .map(|value| {
                        decode_collection_predicate(value, what, &|value, nested_what| {
                            decode_firework_explosion_predicate(value, nested_what)
                        })
                    })
                    .transpose()?,
                flight_duration: optional_int_bounds(entries, "flight_duration", what)?
                    .unwrap_or_default(),
            })
        }
        "minecraft:writable_book_content" => {
            let entries = compound(value, what)?;
            Ok(DialogDataComponentPredicateState::WritableBookContent {
                pages: find_entry(entries, "pages")
                    .map(|value| decode_collection_predicate(value, what, &decode_predicate_string))
                    .transpose()?,
            })
        }
        "minecraft:written_book_content" => {
            let entries = compound(value, what)?;
            Ok(DialogDataComponentPredicateState::WrittenBookContent {
                pages: find_entry(entries, "pages")
                    .map(|value| {
                        decode_collection_predicate(value, what, &|value, nested_what| {
                            decode_component(value, nested_what, context)
                        })
                    })
                    .transpose()?,
                author: optional_string(entries, "author", what)?,
                title: optional_string(entries, "title", what)?,
                generation: optional_int_bounds(entries, "generation", what)?.unwrap_or_default(),
                resolved: find_entry(entries, "resolved")
                    .map(|value| component_bool(value, &format!("{what} field resolved")))
                    .transpose()?,
            })
        }
        "minecraft:attribute_modifiers" => {
            let entries = compound(value, what)?;
            Ok(DialogDataComponentPredicateState::AttributeModifiers {
                modifiers: find_entry(entries, "modifiers")
                    .map(|value| {
                        decode_collection_predicate(value, what, &|value, nested_what| {
                            decode_attribute_modifier_entry_predicate(
                                value,
                                context.registries,
                                nested_what,
                            )
                        })
                    })
                    .transpose()?,
            })
        }
        "minecraft:trim" => {
            let entries = compound(value, what)?;
            Ok(DialogDataComponentPredicateState::Trim {
                material: find_entry(entries, "material")
                    .map(|value| {
                        decode_registry_holder_set(
                            value,
                            TRIM_MATERIAL_REGISTRY,
                            context.registries,
                            what,
                        )
                    })
                    .transpose()?,
                pattern: find_entry(entries, "pattern")
                    .map(|value| {
                        decode_registry_holder_set(
                            value,
                            TRIM_PATTERN_REGISTRY,
                            context.registries,
                            what,
                        )
                    })
                    .transpose()?,
            })
        }
        "minecraft:jukebox_playable" => {
            let entries = compound(value, what)?;
            Ok(DialogDataComponentPredicateState::JukeboxPlayable {
                song: find_entry(entries, "song")
                    .map(|value| {
                        decode_registry_holder_set(
                            value,
                            JUKEBOX_SONG_REGISTRY,
                            context.registries,
                            what,
                        )
                    })
                    .transpose()?,
            })
        }
        "minecraft:villager/variant" => Ok(DialogDataComponentPredicateState::VillagerVariant {
            villager_types: decode_registry_holder_set(
                value,
                VILLAGER_TYPE_REGISTRY,
                context.registries,
                what,
            )?,
        }),
        _ => Err(format!(
            "missing typed decoder for {what} concrete predicate {resource_id}"
        )),
    }
}

fn decode_enchantment_predicates(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<Vec<DialogEnchantmentPredicateState>, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let nested_what = format!("{what}[{index}]");
            let entries = compound(value, &nested_what)?;
            Ok(DialogEnchantmentPredicateState {
                enchantments: find_entry(entries, "enchantments")
                    .map(|value| {
                        decode_registry_holder_set(
                            value,
                            ENCHANTMENT_REGISTRY,
                            registries,
                            &nested_what,
                        )
                    })
                    .transpose()?,
                levels: optional_int_bounds(entries, "levels", &nested_what)?.unwrap_or_default(),
            })
        })
        .collect()
}

fn decode_firework_explosion_predicate(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogFireworkExplosionPredicateState, String> {
    let entries = compound(value, what)?;
    let shape = match find_entry(entries, "shape") {
        None => None,
        Some(NbtSummaryValue::String(name)) => Some(
            FireworkExplosionShapeSummary::from_serialized_name(name)
                .ok_or_else(|| format!("unknown {what} shape {name}"))?,
        ),
        Some(_) => return Err(format!("{what} field shape is not a shape name")),
    };
    Ok(DialogFireworkExplosionPredicateState {
        shape,
        has_twinkle: find_entry(entries, "has_twinkle")
            .map(|value| component_bool(value, &format!("{what} field has_twinkle")))
            .transpose()?,
        has_trail: find_entry(entries, "has_trail")
            .map(|value| component_bool(value, &format!("{what} field has_trail")))
            .transpose()?,
    })
}

fn decode_attribute_modifier_entry_predicate(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<DialogAttributeModifierEntryPredicateState, String> {
    let entries = compound(value, what)?;
    let modifier_id = optional_identifier(entries, "id", what)?;
    let (operation_id, operation) = match optional_string(entries, "operation", what)? {
        None => (None, None),
        Some(operation) => {
            let operation_id = attribute_modifier_operation_id(&operation)
                .ok_or_else(|| format!("unknown {what} operation {operation}"))?;
            (Some(operation_id), Some(operation))
        }
    };
    let (slot_group_id, slot_group) = match optional_string(entries, "slot", what)? {
        None => (None, None),
        Some(slot) => {
            let entry = vanilla_equipment_slot_group_for_serialized_name(&slot)
                .ok_or_else(|| format!("unknown {what} equipment slot group {slot}"))?;
            (Some(entry.id), Some(slot))
        }
    };
    Ok(DialogAttributeModifierEntryPredicateState {
        attribute: find_entry(entries, "attribute")
            .map(|value| decode_registry_holder_set(value, ATTRIBUTE_REGISTRY, registries, what))
            .transpose()?,
        modifier_id,
        amount: find_entry(entries, "amount")
            .map(|value| decode_double_bounds(value, &format!("{what} field amount")))
            .transpose()?
            .unwrap_or_default(),
        operation_id,
        operation,
        slot_group_id,
        slot_group,
    })
}

fn decode_collection_predicate<T, F>(
    value: &NbtSummaryValue,
    what: &str,
    decode_element: &F,
) -> Result<DialogCollectionPredicateState<T>, String>
where
    F: Fn(&NbtSummaryValue, &str) -> Result<T, String>,
{
    let entries = compound(value, what)?;
    let contains = find_entry(entries, "contains")
        .map(|value| decode_list(value, &format!("{what} field contains"), decode_element))
        .transpose()?;
    let count = find_entry(entries, "count")
        .map(|value| {
            let NbtSummaryValue::List(values) = value else {
                return Err(format!("{what} field count is not a list"));
            };
            values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let nested_what = format!("{what} field count[{index}]");
                    let entries = compound(value, &nested_what)?;
                    let test = find_entry(entries, "test")
                        .ok_or_else(|| format!("{nested_what} is missing field test"))?;
                    let count = find_entry(entries, "count")
                        .ok_or_else(|| format!("{nested_what} is missing field count"))?;
                    Ok(DialogCollectionCountPredicateState {
                        test: decode_element(test, &format!("{nested_what} field test"))?,
                        count: decode_int_bounds(count, &format!("{nested_what} field count"))?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?;
    Ok(DialogCollectionPredicateState {
        contains,
        count,
        size: optional_int_bounds(entries, "size", what)?,
    })
}

fn decode_list<T, F>(
    value: &NbtSummaryValue,
    what: &str,
    decode_element: &F,
) -> Result<Vec<T>, String>
where
    F: Fn(&NbtSummaryValue, &str) -> Result<T, String>,
{
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_element(value, &format!("{what}[{index}]")))
        .collect()
}

fn decode_predicate_string(value: &NbtSummaryValue, what: &str) -> Result<String, String> {
    let NbtSummaryValue::String(value) = value else {
        return Err(format!("{what} is not a string"));
    };
    Ok(value.clone())
}

fn optional_int_bounds(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Option<DialogIntBoundsState>, String> {
    find_entry(entries, key)
        .map(|value| decode_int_bounds(value, &format!("{what} field {key}")))
        .transpose()
}

fn decode_int_bounds(value: &NbtSummaryValue, what: &str) -> Result<DialogIntBoundsState, String> {
    let bounds = match value {
        NbtSummaryValue::Compound(entries) => DialogIntBoundsState {
            min: find_entry(entries, "min")
                .map(|value| component_i32(value, &format!("{what} min")))
                .transpose()?,
            max: find_entry(entries, "max")
                .map(|value| component_i32(value, &format!("{what} max")))
                .transpose()?,
        },
        _ => {
            let exact = component_i32(value, what)?;
            DialogIntBoundsState {
                min: Some(exact),
                max: Some(exact),
            }
        }
    };
    if matches!((bounds.min, bounds.max), (Some(min), Some(max)) if min > max) {
        return Err(format!("{what} has swapped bounds"));
    }
    Ok(bounds)
}

fn decode_double_bounds(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDoubleBoundsState, String> {
    let bounds = match value {
        NbtSummaryValue::Compound(entries) => DialogDoubleBoundsState {
            min_bits: find_entry(entries, "min")
                .map(|value| component_f64_bits(value, &format!("{what} min")))
                .transpose()?,
            max_bits: find_entry(entries, "max")
                .map(|value| component_f64_bits(value, &format!("{what} max")))
                .transpose()?,
        },
        _ => {
            let exact = component_f64_bits(value, what)?;
            DialogDoubleBoundsState {
                min_bits: Some(exact),
                max_bits: Some(exact),
            }
        }
    };
    if matches!(
        (bounds.min_bits, bounds.max_bits),
        (Some(min), Some(max)) if java_double_cmp(min, max) == Ordering::Greater
    ) {
        return Err(format!("{what} has swapped bounds"));
    }
    Ok(bounds)
}

fn java_double_cmp(left_bits: u64, right_bits: u64) -> Ordering {
    let canonical = |bits| {
        let value = f64::from_bits(bits);
        if value.is_nan() {
            f64::NAN
        } else {
            value
        }
    };
    canonical(left_bits).total_cmp(&canonical(right_bits))
}

fn optional_identifier(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Option<String>, String> {
    optional_string(entries, key, what)?
        .map(|raw| {
            normalize_codec_identifier(&raw)
                .ok_or_else(|| format!("invalid {what} field {key} resource id {raw}"))
        })
        .transpose()
}

fn optional_string(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Option<String>, String> {
    match find_entry(entries, key) {
        None => Ok(None),
        Some(NbtSummaryValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("{what} field {key} is not a string")),
    }
}

fn compound<'a>(value: &'a NbtSummaryValue, what: &str) -> Result<&'a [NbtSummaryEntry], String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    Ok(entries)
}

#[cfg(test)]
mod tests;
