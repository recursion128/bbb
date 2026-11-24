//! Persistent codecs for `minecraft:can_break` / `minecraft:can_place_on`
//! (`AdventureModePredicate.CODEC`, `AdventureModePredicate.java:26-32`) and
//! nested `DataComponentMatchers`. Depth is bounded by
//! `MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH`.

use bbb_facts::data_component_types::{
    vanilla_data_component_predicate_resource_id, vanilla_data_component_type,
    vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::entry as field;
use bbb_protocol::packets::data_components::MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH;
use bbb_protocol::packets::{
    AdventureModePredicateDataSummary, BlockPredicateSummary, DataComponentPatchSummary,
    DataComponentPredicateTypeSummary, HolderSetSummary, PartialDataComponentPredicateSummary,
    StatePropertyMatcherSummary, StatePropertyValueMatcherSummary,
};
use bbb_protocol::{
    canonical_snbt_java_string, parse_snbt_compound_utf16, JavaString, NbtSummaryEntry,
    NbtSummaryValue,
};

use super::component_sets::declared_component;
use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::holder::{block_resource_id_in_registry, project_block_holder_set, BLOCK_REGISTRY};
use super::item_stack::validate_untyped_persistent_component;
use super::nbt_support::{
    compound_fields, list_entries, normalize_resource_id_units, persistent_compact_holder_set_nbt,
    MAX_EXACT_PREDICATE_ENTRIES, MAX_PARTIAL_PREDICATES, MAX_PERSISTENT_LIST_ITEMS,
    MAX_PREALLOC_CAPACITY,
};
use super::persistent_to_wire_components::project_persistent_component;
use super::wire_to_persistent_components::typed_persistent_component_nbt;

/// `minecraft:can_break` / `minecraft:can_place_on` -- one shared codec
/// (`DataComponents.java:144-149`), so one shared transcoder.
pub(super) fn persistent_adventure_mode_predicate_nbt(
    type_id: i32,
    resource_id: &'static str,
    predicate: &AdventureModePredicateDataSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    depth: usize,
    item_depth: usize,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return Err(invalid(format!(
            "adventure mode predicate nesting exceeds \
             {MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH}"
        )));
    }
    // `ExtraCodecs.nonEmptyList` wraps the list branch
    // while the
    // network codec is a plain `ByteBufCodecs.list()`
    // so an empty wire list has no save
    // form at all.
    if predicate.predicates.is_empty() {
        return Err(invalid(
            "an empty adventure mode predicate list has no persistent form".to_string(),
        ));
    }
    let mut values = Vec::with_capacity(predicate.predicates.len());
    for block in &predicate.predicates {
        values.push(persistent_block_predicate_nbt(
            type_id,
            resource_id,
            block,
            holder_resource_id,
            content_registry,
            depth,
            item_depth,
        )?);
    }
    // `compactListCodec` compacts exactly one element onto the bare element
    if values.len() == 1 {
        return Ok(values.remove(0));
    }
    Ok(NbtSummaryValue::List(values))
}

#[allow(clippy::too_many_arguments)]
fn persistent_block_predicate_nbt(
    type_id: i32,
    resource_id: &'static str,
    predicate: &BlockPredicateSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    depth: usize,
    item_depth: usize,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let mut fields = Vec::new();
    if let Some(blocks) = &predicate.blocks {
        fields.push(field(
            "blocks",
            persistent_block_holder_set(type_id, resource_id, blocks, content_registry)?,
        ));
    }
    if let Some(state) = &predicate.state {
        fields.push(field(
            "state",
            persistent_state_properties_nbt(type_id, resource_id, state)?,
        ));
    }
    if let Some(nbt) = &predicate.nbt {
        // `NbtPredicate.CODEC` is `TagParser.LENIENT_CODEC`
        // a `Codec.withAlternative` whose writer is
        // the *primary* `FLATTENED_CODEC`, so the save
        // form is the flattened SNBT string and never the nested compound the
        // network carries.
        if !matches!(nbt, NbtSummaryValue::Compound(_)) {
            return Err(invalid(
                "block predicate nbt is not a compound tag".to_string(),
            ));
        }
        fields.push(field("nbt", canonical_snbt_java_string(nbt).to_nbt()));
    }
    // `BlockPredicate.java:29-37`, `DataComponentMatchers.java:18-24`:
    // `optionalFieldOf("components", DataComponentExactPredicate.EMPTY)` omits empty maps.
    if let Some(exact) = predicate.exact_components.as_deref() {
        if !exact.added_type_ids.is_empty() || !exact.removed_type_ids.is_empty() {
            fields.push(field(
                "components",
                persistent_exact_component_predicate_nbt(
                    type_id,
                    resource_id,
                    exact,
                    holder_resource_id,
                    content_registry,
                    depth + 1,
                    item_depth,
                )?,
            ));
        }
    }
    if !predicate.partial_components.is_empty() {
        fields.push(field(
            "predicates",
            persistent_partial_component_predicates_nbt(
                type_id,
                resource_id,
                &predicate.partial_components,
            )?,
        ));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// `RegistryCodecs.homogeneousList(Registries.BLOCK)`
/// written as the compact holder-set form.
fn persistent_block_holder_set(
    type_id: i32,
    resource_id: &'static str,
    blocks: &HolderSetSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let names = blocks.registry_ids.iter().map(|&block_id| {
        block_resource_id_in_registry(block_id, content_registry).ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("unknown {BLOCK_REGISTRY} holder id {block_id}"),
            )
        })
    });
    persistent_compact_holder_set_nbt(type_id, resource_id, blocks.tag.as_deref(), names)
}

/// `StatePropertiesPredicate.CODEC` is
/// an unbounded map from a property name to `ValueMatcher.CODEC`, which is
/// `Codec.either(ExactMatcher.CODEC, RangedMatcher.CODEC)`
/// a plain string for the exact arm
/// and a `{min?, max?}` record for the ranged one.
///
/// The encode direction of that xmap is `Collectors.toMap`, so a repeated
/// property name is an exception in Vanilla rather than a value; this refuses it
/// by name instead.
fn persistent_state_properties_nbt(
    type_id: i32,
    resource_id: &'static str,
    properties: &[StatePropertyMatcherSummary],
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let mut seen = std::collections::BTreeSet::new();
    let mut fields = Vec::with_capacity(properties.len());
    for property in properties {
        if !seen.insert(&property.name) {
            return Err(invalid(format!(
                "state properties repeat property {}",
                property.name.to_display_string()
            )));
        }
        let value = match &property.value {
            StatePropertyValueMatcherSummary::Exact(value) => value.to_nbt(),
            StatePropertyValueMatcherSummary::Range { min, max } => {
                let mut range = Vec::new();
                if let Some(min) = min {
                    range.push(field("min", min.to_nbt()));
                }
                if let Some(max) = max {
                    range.push(field("max", max.to_nbt()));
                }
                NbtSummaryValue::Compound(range)
            }
        };
        fields.push(NbtSummaryEntry {
            name: property.name.clone(),
            value,
        });
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// The `components` term of `DataComponentMatchers.CODEC`
/// which is
/// `DataComponentExactPredicate.CODEC` = `DataComponentType.VALUE_MAP_CODEC`
/// `DataComponentType`): a map from a *persistent* component's
/// identifier to that component's own codec value.
///
/// This is the same per-component encoder the enclosing stack patch uses, so a
/// component this projection cannot carry is the same named refusal one level
/// down rather than a dropped entry.
#[allow(clippy::too_many_arguments)]
pub(super) fn persistent_exact_component_predicate_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    depth: usize,
    item_depth: usize,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    // `Codec.dispatchedMap` has no removal spelling and the network codec
    // rejects one too (`catalog/encode.rs`'s `encode_data_component_matchers`).
    if !patch.removed_type_ids.is_empty() {
        return Err(invalid(
            "an exact component predicate cannot remove components".to_string(),
        ));
    }
    if patch.added_type_ids.is_empty() {
        return Err(invalid(
            "an empty exact component predicate has no persistent form".to_string(),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut fields = Vec::with_capacity(patch.added_type_ids.len());
    for &nested_id in &patch.added_type_ids {
        let component = declared_component(nested_id, &mut seen)?;
        // `PERSISTENT_CODEC` rejects a transient component as a map key
        // the xmap's own filter would drop it
        // silently, which this projection never does.
        if !component.persistent {
            return Err(invalid(format!(
                "exact component predicate names transient component {}",
                component.resource_id
            )));
        }
        let value = match typed_persistent_component_nbt(
            nested_id,
            component.resource_id,
            patch,
            holder_resource_id,
            content_registry,
            depth,
            item_depth,
        ) {
            Some(value) => value?,
            None => {
                let value =
                    super::component_sets::retained_persistent_component_nbt(nested_id, patch)
                        .cloned()
                        .ok_or(ItemStackPersistentProjectionError::UnsupportedComponent {
                            type_id: nested_id,
                            resource_id: component.resource_id,
                        })?;
                validate_untyped_persistent_component(
                    nested_id,
                    component.resource_id,
                    &value,
                    content_registry,
                )?;
                value
            }
        };
        fields.push(field(component.resource_id, value));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

/// The `predicates` term of `DataComponentMatchers.CODEC`
/// `DataComponentPredicate.CODEC` =
/// `Codec.dispatchedMap(Type.CODEC, Type::codec)`
///
/// The key is a concrete predicate's identifier, or -- for the `AnyValue` case
/// -- a component type's identifier, and
/// the value is that codec's own NBT, which is what the wire already carried.
fn persistent_partial_component_predicates_nbt(
    type_id: i32,
    resource_id: &'static str,
    predicates: &[PartialDataComponentPredicateSummary],
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let mut seen = std::collections::BTreeSet::new();
    let mut fields = Vec::with_capacity(predicates.len());
    for predicate in predicates {
        let value = predicate
            .wire_value
            .as_ref()
            .ok_or_else(|| invalid("a partial component predicate carries no value".to_string()))?;
        let name = match predicate.predicate_type {
            DataComponentPredicateTypeSummary::Concrete(predicate_id) => {
                vanilla_data_component_predicate_resource_id(predicate_id).ok_or_else(|| {
                    invalid(format!(
                        "invalid data component predicate type id {predicate_id}"
                    ))
                })?
            }
            DataComponentPredicateTypeSummary::AnyComponent(component_id) => {
                let component = vanilla_data_component_type(component_id).ok_or_else(|| {
                    invalid(format!("unknown data component type id {component_id}"))
                })?;
                // `AnyValueType`'s codec is `MapCodec.unitCodec`
                // i.e. an empty map.
                if !matches!(value, NbtSummaryValue::Compound(entries) if entries.is_empty()) {
                    return Err(invalid(format!(
                        "any-component predicate {} is not an empty compound",
                        component.resource_id
                    )));
                }
                // `Type.CODEC` writes either registry's name and decodes the
                // predicate registry first,
                // so a component sharing a name with a concrete predicate would
                // come back as that predicate.
                if VANILLA_DATA_COMPONENT_PREDICATE_TYPES
                    .iter()
                    .any(|predicate| predicate.resource_id == component.resource_id)
                {
                    return Err(invalid(format!(
                        "any-component predicate {} collides with a concrete predicate",
                        component.resource_id
                    )));
                }
                component.resource_id
            }
        };
        if !seen.insert(name) {
            return Err(invalid(format!(
                "a partial component predicate repeats {name}"
            )));
        }
        fields.push(field(name, value.clone()));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn project_adventure_mode_predicate(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
    depth: usize,
    item_depth: usize,
) -> Result<()> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return Err(invalid(format!(
            "adventure mode predicate nesting exceeds \
             {MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH}"
        )));
    }
    let predicates = if matches!(value, NbtSummaryValue::Compound(_)) {
        vec![project_block_predicate(
            type_id,
            resource_id,
            value,
            holder_id,
            content_registry,
            depth,
            item_depth,
        )?]
    } else {
        let entries = list_entries(value)
            .ok_or_else(|| invalid("adventure mode predicate is not a list".to_string()))?;
        // `ExtraCodecs.nonEmptyList`.
        if entries.is_empty() {
            return Err(invalid(
                "adventure mode predicate list must not be empty".to_string(),
            ));
        }
        if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
            return Err(invalid(format!(
                "adventure mode predicate list has {} entries, maximum is \
                 {MAX_PERSISTENT_LIST_ITEMS}",
                entries.len()
            )));
        }
        let mut predicates = Vec::with_capacity(entries.len().min(MAX_PREALLOC_CAPACITY));
        for entry in entries.iter() {
            predicates.push(project_block_predicate(
                type_id,
                resource_id,
                &entry,
                holder_id,
                content_registry,
                depth,
                item_depth,
            )?);
        }
        predicates
    };
    let predicate = AdventureModePredicateDataSummary { predicates };
    if type_id == VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID {
        patch.can_break_persistent = Some(predicate);
    } else {
        patch.can_place_on_persistent = Some(predicate);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn project_block_predicate(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    depth: usize,
    item_depth: usize,
) -> Result<BlockPredicateSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("block predicate is not a compound".to_string()))?;
    let blocks = match super::nbt_support::latest_field(fields, "blocks") {
        Some(blocks) => Some(project_block_holder_set(
            type_id,
            resource_id,
            blocks,
            content_registry,
        )?),
        None => None,
    };
    let state = match super::nbt_support::latest_field(fields, "state") {
        Some(state) => Some(project_state_properties(type_id, resource_id, state)?),
        None => None,
    };
    let nbt = match super::nbt_support::latest_field(fields, "nbt") {
        Some(NbtSummaryValue::Compound(_)) => Some(
            super::nbt_support::latest_field(fields, "nbt")
                .unwrap()
                .clone(),
        ),
        Some(NbtSummaryValue::String(_)) => {
            let raw = super::nbt_support::latest_field(fields, "nbt").unwrap();
            let snbt = JavaString::from_nbt(raw)
                .ok_or_else(|| invalid("block predicate nbt is not a SNBT string".to_string()))?;
            let parsed = parse_snbt_compound_utf16(snbt.units()).map_err(|error| {
                invalid(format!(
                    "block predicate nbt is not a SNBT compound: {error}"
                ))
            })?;
            Some(parsed)
        }
        Some(_) => {
            return Err(invalid(
                "block predicate nbt is neither a compound nor a SNBT string".to_string(),
            ))
        }
        None => None,
    };
    // `BlockPredicate.java:29-37`, `DataComponentMatchers.java:18-24`:
    // an explicit empty `"components": {}` decodes to `DataComponentExactPredicate.EMPTY`.
    let exact_components = match super::nbt_support::latest_field(fields, "components") {
        Some(exact) => {
            let patch = project_exact_component_predicate(
                type_id,
                resource_id,
                exact,
                holder_id,
                content_registry,
                depth + 1,
                item_depth,
            )?;
            (!patch.added_type_ids.is_empty()).then_some(patch)
        }
        None => None,
    };
    let partial_components = match super::nbt_support::latest_field(fields, "predicates") {
        Some(predicates) => project_partial_component_predicates(type_id, resource_id, predicates)?,
        None => Vec::new(),
    };
    Ok(BlockPredicateSummary {
        blocks,
        state,
        nbt,
        exact_components,
        partial_components,
    })
}

/// The persistent `state` map back into the network list, in the order the
/// `unboundedMap` decode walks it. A
/// duplicate key is last-wins at the position its first occurrence had.
fn project_state_properties(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<Vec<StatePropertyMatcherSummary>> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("state properties predicate is not a compound".to_string()))?;
    if fields.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(invalid(format!(
            "state properties predicate has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            fields.len()
        )));
    }
    let mut order: Vec<JavaString> = Vec::new();
    let mut matchers: std::collections::BTreeMap<JavaString, StatePropertyValueMatcherSummary> =
        std::collections::BTreeMap::new();
    for field in fields {
        let matcher = match &field.value {
            NbtSummaryValue::String(_) => {
                StatePropertyValueMatcherSummary::Exact(JavaString::from_nbt(&field.value).unwrap())
            }
            NbtSummaryValue::Compound(range) => {
                let bound = |name: &str| -> Result<Option<JavaString>> {
                    super::nbt_support::latest_field(range, name)
                        .map(|value| {
                            JavaString::from_nbt(value).ok_or_else(|| {
                                invalid(format!("state property {name} is not a string"))
                            })
                        })
                        .transpose()
                };
                StatePropertyValueMatcherSummary::Range {
                    min: bound("min")?,
                    max: bound("max")?,
                }
            }
            _ => {
                return Err(invalid(format!(
                    "state property {} is neither a string nor a range record",
                    field.name.to_display_string()
                )))
            }
        };
        if matchers.insert(field.name.clone(), matcher).is_none() {
            order.push(field.name.clone());
        }
    }
    Ok(order
        .into_iter()
        .map(|name| StatePropertyMatcherSummary {
            value: matchers.remove(&name).expect("recorded matcher is present"),
            name,
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
fn project_exact_component_predicate(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    depth: usize,
    item_depth: usize,
) -> Result<Box<DataComponentPatchSummary>> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("exact component predicate is not a compound".to_string()))?;
    if fields.len() > MAX_EXACT_PREDICATE_ENTRIES {
        return Err(invalid(format!(
            "exact component predicate has {} entries, maximum is {MAX_EXACT_PREDICATE_ENTRIES}",
            fields.len()
        )));
    }
    let mut patch = Box::<DataComponentPatchSummary>::default();
    let mut seen = std::collections::BTreeSet::new();
    for field in fields {
        let name = normalize_resource_id_units(field.name.units())?;
        let component = vanilla_data_component_type_for_resource_id(&name).ok_or_else(|| {
            invalid(format!(
                "exact component predicate names unknown component {name}"
            ))
        })?;
        if !component.persistent {
            return Err(invalid(format!(
                "exact component predicate names transient component {name}"
            )));
        }
        if !seen.insert(component.type_id) {
            return Err(invalid(format!(
                "exact component predicate repeats component {name}"
            )));
        }
        // The nested value goes through the same per-component entry point the
        // enclosing patch uses, so `can_break` inside `can_break` is the same
        // transcoder one level deeper.
        project_persistent_component(
            component.type_id,
            component.resource_id,
            &field.value,
            holder_id,
            content_registry,
            &mut patch,
            depth,
            item_depth,
        )?;
        patch.added_type_ids.push(component.type_id);
    }
    patch.added_type_ids.sort_unstable();
    Ok(patch)
}

fn project_partial_component_predicates(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
) -> Result<Vec<PartialDataComponentPredicateSummary>> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("partial component predicates are not a compound".to_string()))?;
    if fields.len() > MAX_PARTIAL_PREDICATES {
        return Err(invalid(format!(
            "partial component predicates have {} entries, maximum is {MAX_PARTIAL_PREDICATES}",
            fields.len()
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut predicates = Vec::with_capacity(fields.len().min(MAX_PREALLOC_CAPACITY));
    for field in fields {
        let name = normalize_resource_id_units(field.name.units())?;
        if !seen.insert(name.clone()) {
            return Err(invalid(format!(
                "a partial component predicate repeats {name}"
            )));
        }
        let predicate_type = if let Some(predicate) = VANILLA_DATA_COMPONENT_PREDICATE_TYPES
            .iter()
            .find(|predicate| predicate.resource_id == name)
        {
            DataComponentPredicateTypeSummary::Concrete(predicate.type_id)
        } else {
            let component =
                vanilla_data_component_type_for_resource_id(&name).ok_or_else(|| {
                    invalid(format!(
                        "partial component predicate names unknown predicate or component {name}"
                    ))
                })?;
            DataComponentPredicateTypeSummary::AnyComponent(component.type_id)
        };
        predicates.push(PartialDataComponentPredicateSummary {
            predicate_type,
            wire_value: Some(field.value.clone()),
        });
    }
    Ok(predicates)
}
