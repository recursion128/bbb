//! HashOps encoding for `can_place_on`, `can_break`, `lock`, and component predicates.

use std::collections::{BTreeMap, BTreeSet};

use bbb_facts::attribute_modifier_types::attribute_modifier_operation_id;
use bbb_facts::data_component_types::{
    vanilla_data_component_predicate_resource_id, vanilla_data_component_type,
    vanilla_data_component_type_for_resource_id,
    VANILLA_DATA_COMPONENT_PREDICATE_ARMOR_TRIM_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_CONTAINER_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_CUSTOM_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_FIREWORKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_FIREWORK_EXPLOSION_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_JUKEBOX_PLAYABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_POTIONS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_STORED_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
    VANILLA_DATA_COMPONENT_PREDICATE_VILLAGER_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_WRITABLE_BOOK_TYPE_ID,
    VANILLA_DATA_COMPONENT_PREDICATE_WRITTEN_BOOK_TYPE_ID,
};
use bbb_facts::equipment_slots::vanilla_equipment_slot_group_for_serialized_name;
use bbb_facts::static_registry_elements::vanilla_static_registry_elements;
use bbb_facts::{PersistentRegistryMembership, PersistentRegistryTagMembers};
use bbb_protocol::canonical_snbt_java_string;
use bbb_protocol::packets::data_components::MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH;
use bbb_protocol::packets::{
    AdventureModePredicateDataSummary, BlockPredicateSummary, DataComponentPatchSummary,
    DataComponentPredicateTypeSummary, FireworkExplosionShapeSummary,
    PartialDataComponentPredicateSummary, PersistentDataComponentValueSummary,
    PersistentItemPredicateSummary, StatePropertyValueMatcherSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::read_persistent_component_into_with_resolver;

use super::components::hash_component_value_at_depth;
use super::components_rich::hash_component_at_depth;
use super::ops::{
    hash_boolean, hash_double, hash_empty_map, hash_holder_set_from_ids, hash_int,
    hash_java_string, hash_list, hash_map, hash_record, hash_string, normalize_codec_identifier,
};
use super::ItemHashLookup;

pub(super) fn hash_adventure_mode_predicate(
    lookup: &dyn ItemHashLookup,
    predicate: &AdventureModePredicateDataSummary,
    depth: usize,
) -> Option<u32> {
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH || predicate.predicates.is_empty() {
        return None;
    }
    let mut encoded = Vec::with_capacity(predicate.predicates.len());
    for block_predicate in &predicate.predicates {
        encoded.push(hash_block_predicate_at_depth(
            lookup,
            block_predicate,
            depth,
        )?);
    }
    if encoded.len() == 1 {
        Some(encoded[0])
    } else {
        Some(hash_list(encoded))
    }
}

pub fn hash_block_predicate(
    lookup: &dyn ItemHashLookup,
    predicate: &BlockPredicateSummary,
) -> Option<u32> {
    hash_block_predicate_at_depth(lookup, predicate, 0)
}

fn hash_block_predicate_at_depth(
    lookup: &dyn ItemHashLookup,
    predicate: &BlockPredicateSummary,
    depth: usize,
) -> Option<u32> {
    let mut pairs = Vec::new();
    if let Some(blocks) = &predicate.blocks {
        pairs.push((
            hash_string("blocks"),
            hash_holder_set_from_ids(
                lookup,
                "minecraft:block",
                &blocks.registry_ids,
                blocks.tag.as_deref(),
            )?,
        ));
    }
    if let Some(state) = &predicate.state {
        let mut seen = BTreeSet::new();
        let mut state_pairs = Vec::with_capacity(state.len());
        for matcher in state {
            if !seen.insert(&matcher.name) {
                return None;
            }
            let val_hash = match &matcher.value {
                StatePropertyValueMatcherSummary::Exact(value) => hash_java_string(value),
                StatePropertyValueMatcherSummary::Range { min, max } => {
                    let mut range = Vec::new();
                    if let Some(min) = min {
                        range.push(("min", hash_java_string(min)));
                    }
                    if let Some(max) = max {
                        range.push(("max", hash_java_string(max)));
                    }
                    hash_record(range)
                }
            };
            state_pairs.push((hash_java_string(&matcher.name), val_hash));
        }
        pairs.push((hash_string("state"), hash_map(state_pairs)));
    }
    if let Some(nbt) = &predicate.nbt {
        let NbtSummaryValue::Compound(_) = nbt else {
            return None;
        };
        pairs.push((
            hash_string("nbt"),
            hash_java_string(&canonical_snbt_java_string(nbt)),
        ));
    }
    // `BlockPredicate.java:29-37`, `DataComponentMatchers.java:18-24`:
    // `optionalFieldOf("components", DataComponentExactPredicate.EMPTY)` omits empty maps.
    if let Some(exact) = predicate.exact_components.as_deref() {
        if !exact.removed_type_ids.is_empty() {
            return None;
        }
        if !exact.added_type_ids.is_empty() {
            pairs.push((
                hash_string("components"),
                hash_exact_component_patch(lookup, exact, depth + 1)?,
            ));
        }
    }
    if !predicate.partial_components.is_empty() {
        pairs.push((
            hash_string("predicates"),
            hash_partial_component_predicates_at_depth(
                lookup,
                &predicate.partial_components,
                depth + 1,
            )?,
        ));
    }
    Some(hash_map(pairs))
}

pub fn hash_persistent_item_predicate(
    lookup: &dyn ItemHashLookup,
    predicate: &PersistentItemPredicateSummary,
) -> Option<u32> {
    hash_persistent_item_predicate_at_depth(lookup, predicate, 0)
}

pub(super) fn hash_persistent_item_predicate_at_depth(
    lookup: &dyn ItemHashLookup,
    predicate: &PersistentItemPredicateSummary,
    depth: usize,
) -> Option<u32> {
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return None;
    }
    let mut pairs = Vec::new();
    if let Some(items) = &predicate.items {
        pairs.push((
            hash_string("items"),
            hash_holder_set_from_ids(
                lookup,
                "minecraft:item",
                &items.registry_ids,
                items.tag.as_deref(),
            )?,
        ));
    }
    match (predicate.count_min, predicate.count_max) {
        (Some(min), Some(max)) if min == max => {
            pairs.push((hash_string("count"), hash_int(min)));
        }
        (None, None) => {}
        (min, max) => {
            let mut count_fields = Vec::new();
            if let Some(min) = min {
                count_fields.push(("min", hash_int(min)));
            }
            if let Some(max) = max {
                count_fields.push(("max", hash_int(max)));
            }
            pairs.push((hash_string("count"), hash_record(count_fields)));
        }
    }
    if !predicate.exact_components.is_empty() {
        pairs.push((
            hash_string("components"),
            hash_exact_component_values(lookup, &predicate.exact_components, depth + 1)?,
        ));
    }
    if !predicate.partial_components.is_empty() {
        pairs.push((
            hash_string("predicates"),
            hash_partial_component_predicates_at_depth(
                lookup,
                &predicate.partial_components,
                depth + 1,
            )?,
        ));
    }
    Some(hash_map(pairs))
}

fn hash_exact_component_values(
    lookup: &dyn ItemHashLookup,
    values: &[PersistentDataComponentValueSummary],
    depth: usize,
) -> Option<u32> {
    let mut seen = BTreeSet::new();
    let mut patch = DataComponentPatchSummary::default();
    for entry in values {
        if !seen.insert(entry.component_type_id) {
            return None;
        }
        read_persistent_component_into_with_resolver(
            entry.component_type_id,
            &entry.value,
            Some(lookup),
            &mut patch,
        )
        .ok()?;
    }
    hash_exact_component_patch(lookup, &patch, depth)
}

fn hash_exact_component_patch(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
    depth: usize,
) -> Option<u32> {
    if !patch.removed_type_ids.is_empty() {
        return None;
    }
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::with_capacity(patch.added_type_ids.len());
    for &type_id in &patch.added_type_ids {
        if !seen.insert(type_id) {
            return None;
        }
        let component = vanilla_data_component_type(type_id)?;
        if !component.persistent {
            return None;
        }
        let value_hash = hash_component_value_at_depth(lookup, patch, type_id, depth)?;
        pairs.push((hash_string(component.resource_id), value_hash));
    }
    Some(hash_map(pairs))
}

pub fn hash_partial_component_predicates(
    lookup: &dyn ItemHashLookup,
    predicates: &[PartialDataComponentPredicateSummary],
) -> Option<u32> {
    hash_partial_component_predicates_at_depth(lookup, predicates, 0)
}

fn hash_partial_component_predicates_at_depth(
    lookup: &dyn ItemHashLookup,
    predicates: &[PartialDataComponentPredicateSummary],
    depth: usize,
) -> Option<u32> {
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::with_capacity(predicates.len());
    for entry in predicates {
        let (key, val_hash) = match entry.predicate_type {
            DataComponentPredicateTypeSummary::AnyComponent(type_id) => {
                if !seen.insert((1u8, type_id)) {
                    return None;
                }
                let comp = vanilla_data_component_type(type_id)?;
                // Wire and persistent decoders represent `AnyValueType`'s unit codec
                // (`DataComponentPredicate.java:42`) as an empty compound `{}`.
                if let Some(wire_value) = &entry.wire_value {
                    if !matches!(wire_value, NbtSummaryValue::Compound(entries) if entries.is_empty())
                    {
                        return None;
                    }
                }
                (comp.resource_id, hash_empty_map())
            }
            DataComponentPredicateTypeSummary::Concrete(predicate_type_id) => {
                if !seen.insert((0u8, predicate_type_id)) {
                    return None;
                }
                let resource_id = vanilla_data_component_predicate_resource_id(predicate_type_id)?;
                let wire_value = entry.wire_value.as_ref()?;
                let val_hash = hash_concrete_component_predicate_at_depth(
                    lookup,
                    predicate_type_id,
                    wire_value,
                    depth,
                )?;
                (resource_id, val_hash)
            }
        };
        pairs.push((hash_string(key), val_hash));
    }
    Some(hash_map(pairs))
}

pub fn hash_concrete_component_predicate(
    lookup: &dyn ItemHashLookup,
    predicate_type_id: i32,
    wire_value: &NbtSummaryValue,
) -> Option<u32> {
    hash_concrete_component_predicate_at_depth(lookup, predicate_type_id, wire_value, 0)
}

fn hash_concrete_component_predicate_at_depth(
    lookup: &dyn ItemHashLookup,
    predicate_type_id: i32,
    wire_value: &NbtSummaryValue,
    depth: usize,
) -> Option<u32> {
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return None;
    }
    match predicate_type_id {
        // `DamagePredicate.CODEC` (`net/minecraft/core/component/predicates/DamagePredicate.java:10-16`).
        VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID => {
            hash_compound_checked(wire_value, &["durability", "damage"], |_, val| {
                let bounds = IntBounds::from_nbt(val)?;
                Some((!bounds.is_any()).then(|| bounds.hash(hash_int)))
            })
        }
        // `EnchantmentPredicate.CODEC` (`EnchantmentPredicate.java:15-21`).
        VANILLA_DATA_COMPONENT_PREDICATE_ENCHANTMENTS_TYPE_ID
        | VANILLA_DATA_COMPONENT_PREDICATE_STORED_ENCHANTMENTS_TYPE_ID => {
            let NbtSummaryValue::List(items) = wire_value else {
                return None;
            };
            let mut hashes = Vec::with_capacity(items.len());
            for item in items {
                hashes.push(hash_compound_checked(
                    item,
                    &["enchantments", "levels"],
                    |key, val| match key {
                        "enchantments" => {
                            hash_nbt_holder_set(lookup, "minecraft:enchantment", val).map(Some)
                        }
                        "levels" => {
                            let bounds = IntBounds::from_nbt(val)?;
                            Some((!bounds.is_any()).then(|| bounds.hash(hash_int)))
                        }
                        _ => None,
                    },
                )?);
            }
            Some(hash_list(hashes))
        }
        VANILLA_DATA_COMPONENT_PREDICATE_POTIONS_TYPE_ID => {
            hash_nbt_holder_set(lookup, "minecraft:potion", wire_value)
        }
        VANILLA_DATA_COMPONENT_PREDICATE_CUSTOM_DATA_TYPE_ID => match wire_value {
            NbtSummaryValue::String(snbt) => Some(hash_java_string(snbt)),
            NbtSummaryValue::Compound(_) => {
                Some(hash_java_string(&canonical_snbt_java_string(wire_value)))
            }
            _ => None,
        },
        VANILLA_DATA_COMPONENT_PREDICATE_CONTAINER_TYPE_ID
        | VANILLA_DATA_COMPONENT_PREDICATE_BUNDLE_CONTENTS_TYPE_ID => {
            hash_compound_checked(wire_value, &["items"], |_, val| {
                hash_collection_predicate(val, |elem| {
                    hash_nbt_item_predicate(lookup, elem, depth + 1)
                })
                .map(Some)
            })
        }
        VANILLA_DATA_COMPONENT_PREDICATE_FIREWORK_EXPLOSION_TYPE_ID => {
            hash_firework_explosion_predicate(wire_value)
        }
        // `FireworksPredicate.CODEC` (`FireworksPredicate.java:17-25`).
        VANILLA_DATA_COMPONENT_PREDICATE_FIREWORKS_TYPE_ID => hash_compound_checked(
            wire_value,
            &["explosions", "flight_duration"],
            |key, val| match key {
                "explosions" => {
                    hash_collection_predicate(val, hash_firework_explosion_predicate).map(Some)
                }
                "flight_duration" => {
                    let bounds = IntBounds::from_nbt(val)?;
                    Some((!bounds.is_any()).then(|| bounds.hash(hash_int)))
                }
                _ => None,
            },
        ),
        VANILLA_DATA_COMPONENT_PREDICATE_WRITABLE_BOOK_TYPE_ID => {
            hash_compound_checked(wire_value, &["pages"], |_, val| {
                hash_collection_predicate(val, |elem| match elem {
                    NbtSummaryValue::String(s) => Some(hash_java_string(s)),
                    _ => None,
                })
                .map(Some)
            })
        }
        // `WrittenBookPredicate.CODEC` (`WrittenBookPredicate.java:24-35`).
        VANILLA_DATA_COMPONENT_PREDICATE_WRITTEN_BOOK_TYPE_ID => hash_compound_checked(
            wire_value,
            &["pages", "author", "title", "generation", "resolved"],
            |key, val| match key {
                "pages" => hash_collection_predicate(val, |elem| {
                    hash_component_at_depth(lookup, elem, depth + 1)
                })
                .map(Some),
                "author" | "title" => match val {
                    NbtSummaryValue::String(s) => Some(Some(hash_java_string(s))),
                    _ => None,
                },
                "generation" => {
                    let bounds = IntBounds::from_nbt(val)?;
                    Some((!bounds.is_any()).then(|| bounds.hash(hash_int)))
                }
                "resolved" => nbt_boolean(val).map(hash_boolean).map(Some),
                _ => None,
            },
        ),
        // `AttributeModifiersPredicate.CODEC` (`AttributeModifiersPredicate.java:23-61`).
        VANILLA_DATA_COMPONENT_PREDICATE_ATTRIBUTE_MODIFIERS_TYPE_ID => {
            hash_compound_checked(wire_value, &["modifiers"], |_, val| {
                hash_collection_predicate(val, |elem| {
                    hash_attribute_modifier_entry_predicate(lookup, elem)
                })
                .map(Some)
            })
        }
        VANILLA_DATA_COMPONENT_PREDICATE_ARMOR_TRIM_TYPE_ID => {
            hash_compound_checked(wire_value, &["material", "pattern"], |key, val| match key {
                "material" => hash_nbt_holder_set(lookup, "minecraft:trim_material", val).map(Some),
                "pattern" => hash_nbt_holder_set(lookup, "minecraft:trim_pattern", val).map(Some),
                _ => None,
            })
        }
        VANILLA_DATA_COMPONENT_PREDICATE_JUKEBOX_PLAYABLE_TYPE_ID => {
            hash_compound_checked(wire_value, &["song"], |_, val| {
                hash_nbt_holder_set(lookup, "minecraft:jukebox_song", val).map(Some)
            })
        }
        VANILLA_DATA_COMPONENT_PREDICATE_VILLAGER_VARIANT_TYPE_ID => {
            hash_nbt_holder_set(lookup, "minecraft:villager_type", wire_value)
        }
        _ => None,
    }
}

/// `FireworkExplosionPredicate.FireworkPredicate.CODEC` (`FireworkExplosionPredicate.java:27-34`).
fn hash_firework_explosion_predicate(value: &NbtSummaryValue) -> Option<u32> {
    hash_compound_checked(
        value,
        &["shape", "has_twinkle", "has_trail"],
        |key, val| match key {
            "shape" => {
                let NbtSummaryValue::String(shape) = val else {
                    return None;
                };
                let shape = shape
                    .with_utf8(FireworkExplosionShapeSummary::from_serialized_name)
                    .flatten()?;
                Some(Some(hash_string(shape.as_str())))
            }
            "has_twinkle" | "has_trail" => nbt_boolean(val).map(hash_boolean).map(Some),
            _ => None,
        },
    )
}

fn hash_attribute_modifier_entry_predicate(
    lookup: &dyn ItemHashLookup,
    value: &NbtSummaryValue,
) -> Option<u32> {
    hash_compound_checked(
        value,
        &["attribute", "id", "amount", "operation", "slot"],
        |key, val| match key {
            "attribute" => hash_nbt_holder_set(lookup, "minecraft:attribute", val).map(Some),
            "id" => {
                let NbtSummaryValue::String(id) = val else {
                    return None;
                };
                let normalized = id.with_utf8(normalize_codec_identifier).flatten()?;
                Some(Some(hash_string(&normalized)))
            }
            "amount" => {
                let bounds = DoubleBounds::from_nbt(val)?;
                Some((!bounds.is_any()).then(|| bounds.hash(hash_double)))
            }
            "operation" => {
                let NbtSummaryValue::String(op) = val else {
                    return None;
                };
                op.with_utf8(attribute_modifier_operation_id).flatten()?;
                Some(Some(hash_java_string(op)))
            }
            "slot" => {
                let NbtSummaryValue::String(slot) = val else {
                    return None;
                };
                slot.with_utf8(vanilla_equipment_slot_group_for_serialized_name)
                    .flatten()?;
                Some(Some(hash_java_string(slot)))
            }
            _ => None,
        },
    )
}

fn nbt_record_fields<'a>(
    value: &'a NbtSummaryValue,
    allowed_keys: &'a [&'a str],
) -> Option<BTreeMap<&'a str, &'a NbtSummaryValue>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let mut fields = BTreeMap::new();
    for entry in entries {
        let name = allowed_keys
            .iter()
            .copied()
            .find(|name| entry.name == *name)?;
        if fields.insert(name, &entry.value).is_some() {
            return None;
        }
    }
    Some(fields)
}

fn hash_compound_checked(
    value: &NbtSummaryValue,
    allowed_keys: &[&str],
    mut hash_field: impl FnMut(&str, &NbtSummaryValue) -> Option<Option<u32>>,
) -> Option<u32> {
    let fields = nbt_record_fields(value, allowed_keys)?;
    let mut pairs = Vec::with_capacity(fields.len());
    for (key, field_val) in fields {
        if let Some(val_hash) = hash_field(key, field_val)? {
            pairs.push((hash_string(key), val_hash));
        }
    }
    Some(hash_map(pairs))
}

fn holder_is_valid_in_registry(
    lookup: &dyn ItemHashLookup,
    registry_id: &str,
    resource_id: &str,
) -> bool {
    match lookup.resource_membership(registry_id, resource_id) {
        PersistentRegistryMembership::Present => true,
        PersistentRegistryMembership::Absent => false,
        PersistentRegistryMembership::Unavailable => {
            lookup.holder_id(registry_id, resource_id).is_some()
                || vanilla_static_registry_elements(registry_id)
                    .is_some_and(|elements| elements.contains(&resource_id))
        }
    }
}

fn hash_nbt_holder_set(
    lookup: &dyn ItemHashLookup,
    registry_id: &str,
    value: &NbtSummaryValue,
) -> Option<u32> {
    match value {
        NbtSummaryValue::String(raw) => {
            let s = raw.try_to_string().ok()?;
            if let Some(tag) = s.strip_prefix('#') {
                let normalized = normalize_codec_identifier(tag)?;
                if matches!(
                    lookup.tag_members(registry_id, &normalized),
                    PersistentRegistryTagMembers::Absent
                ) {
                    return None;
                }
                Some(hash_string(&format!("#{normalized}")))
            } else {
                let normalized = normalize_codec_identifier(&s)?;
                if !holder_is_valid_in_registry(lookup, registry_id, &normalized) {
                    return None;
                }
                Some(hash_string(&normalized))
            }
        }
        NbtSummaryValue::List(items) => {
            let mut hashes = Vec::with_capacity(items.len());
            for item in items {
                let NbtSummaryValue::String(raw) = item else {
                    return None;
                };
                let normalized = normalize_codec_identifier(&raw.try_to_string().ok()?)?;
                if !holder_is_valid_in_registry(lookup, registry_id, &normalized) {
                    return None;
                }
                hashes.push(hash_string(&normalized));
            }
            match hashes.as_slice() {
                [single] => Some(*single),
                _ => Some(hash_list(hashes)),
            }
        }
        _ => None,
    }
}

/// `CollectionPredicate.codec` (`CollectionPredicate.java:12-21`) and
/// `CollectionCountsPredicate.Entry.codec` (`CollectionCountsPredicate.java:29-37`).
fn hash_collection_predicate(
    value: &NbtSummaryValue,
    mut hash_elem: impl FnMut(&NbtSummaryValue) -> Option<u32>,
) -> Option<u32> {
    hash_compound_checked(
        value,
        &["contains", "count", "size"],
        |key, field_val| match key {
            "contains" => {
                let NbtSummaryValue::List(items) = field_val else {
                    return None;
                };
                let mut hashes = Vec::with_capacity(items.len());
                for item in items {
                    hashes.push(hash_elem(item)?);
                }
                Some(Some(hash_list(hashes)))
            }
            "count" => {
                let NbtSummaryValue::List(items) = field_val else {
                    return None;
                };
                let mut hashes = Vec::with_capacity(items.len());
                for item in items {
                    let fields = nbt_record_fields(item, &["test", "count"])?;
                    let test_hash = hash_elem(fields.get("test")?)?;
                    let count_hash = IntBounds::from_nbt(fields.get("count")?)?.hash(hash_int);
                    hashes.push(hash_record([("test", test_hash), ("count", count_hash)]));
                }
                Some(Some(hash_list(hashes)))
            }
            "size" => Some(Some(IntBounds::from_nbt(field_val)?.hash(hash_int))),
            _ => None,
        },
    )
}

/// `ItemPredicate.CODEC` (`ItemPredicate.java:17-24`).
fn hash_nbt_item_predicate(
    lookup: &dyn ItemHashLookup,
    value: &NbtSummaryValue,
    depth: usize,
) -> Option<u32> {
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return None;
    }
    hash_compound_checked(
        value,
        &["items", "count", "components", "predicates"],
        |key, field_val| match key {
            "items" => hash_nbt_holder_set(lookup, "minecraft:item", field_val).map(Some),
            "count" => {
                let bounds = IntBounds::from_nbt(field_val)?;
                Some((!bounds.is_any()).then(|| bounds.hash(hash_int)))
            }
            // `DataComponentMatchers.CODEC` (`DataComponentMatchers.java:18-24`) omits empty
            // `components` and `predicates` maps via `optionalFieldOf`.
            "components" => {
                let NbtSummaryValue::Compound(comp_entries) = field_val else {
                    return None;
                };
                if comp_entries.is_empty() {
                    return Some(None);
                }
                let mut values = Vec::with_capacity(comp_entries.len());
                for NbtSummaryEntry { name, value } in comp_entries {
                    let comp_name = normalize_codec_identifier(&name.try_to_string().ok()?)?;
                    let comp_id = vanilla_data_component_type_for_resource_id(&comp_name)?.type_id;
                    values.push(PersistentDataComponentValueSummary {
                        component_type_id: comp_id,
                        value: value.clone(),
                    });
                }
                hash_exact_component_values(lookup, &values, depth + 1).map(Some)
            }
            "predicates" => {
                let NbtSummaryValue::Compound(pred_entries) = field_val else {
                    return None;
                };
                if pred_entries.is_empty() {
                    return Some(None);
                }
                let mut predicates = Vec::with_capacity(pred_entries.len());
                for NbtSummaryEntry { name, value } in pred_entries {
                    let resource_id = normalize_codec_identifier(&name.try_to_string().ok()?)?;
                    let predicate_type = if let Some(concrete) =
                        VANILLA_DATA_COMPONENT_PREDICATE_TYPES
                            .iter()
                            .find(|predicate| predicate.resource_id == resource_id)
                    {
                        DataComponentPredicateTypeSummary::Concrete(concrete.type_id)
                    } else {
                        let component = vanilla_data_component_type_for_resource_id(&resource_id)?;
                        DataComponentPredicateTypeSummary::AnyComponent(component.type_id)
                    };
                    predicates.push(PartialDataComponentPredicateSummary {
                        predicate_type,
                        wire_value: Some(value.clone()),
                    });
                }
                hash_partial_component_predicates_at_depth(lookup, &predicates, depth + 1).map(Some)
            }
            _ => None,
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bounds<T> {
    min: Option<T>,
    max: Option<T>,
}

type IntBounds = Bounds<i32>;
type DoubleBounds = Bounds<u64>;

impl<T: Copy + PartialEq> Bounds<T> {
    fn exact(value: T) -> Self {
        Self {
            min: Some(value),
            max: Some(value),
        }
    }

    fn is_any(self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    fn hash(self, hasher: impl Fn(T) -> u32) -> u32 {
        if let (Some(min), Some(max)) = (self.min, self.max) {
            if min == max {
                return hasher(min);
            }
        }
        let mut fields = Vec::new();
        if let Some(min) = self.min {
            fields.push(("min", hasher(min)));
        }
        if let Some(max) = self.max {
            fields.push(("max", hasher(max)));
        }
        hash_record(fields)
    }
}

impl IntBounds {
    fn from_nbt(value: &NbtSummaryValue) -> Option<Self> {
        let bounds = match value {
            NbtSummaryValue::Compound(_) => {
                let fields = nbt_record_fields(value, &["min", "max"])?;
                Self {
                    min: match fields.get("min") {
                        Some(v) => Some(nbt_i32(v)?),
                        None => None,
                    },
                    max: match fields.get("max") {
                        Some(v) => Some(nbt_i32(v)?),
                        None => None,
                    },
                }
            }
            other => Self::exact(nbt_i32(other)?),
        };
        if matches!((bounds.min, bounds.max), (Some(min), Some(max)) if min > max) {
            return None;
        }
        Some(bounds)
    }
}

impl DoubleBounds {
    fn from_nbt(value: &NbtSummaryValue) -> Option<Self> {
        let bounds = match value {
            NbtSummaryValue::Compound(_) => {
                let fields = nbt_record_fields(value, &["min", "max"])?;
                Self {
                    min: match fields.get("min") {
                        Some(v) => Some(nbt_f64_bits(v)?),
                        None => None,
                    },
                    max: match fields.get("max") {
                        Some(v) => Some(nbt_f64_bits(v)?),
                        None => None,
                    },
                }
            }
            other => Self::exact(nbt_f64_bits(other)?),
        };
        if let Some(min) = bounds.min {
            if !f64::from_bits(min).is_finite() {
                return None;
            }
        }
        if let Some(max) = bounds.max {
            if !f64::from_bits(max).is_finite() {
                return None;
            }
        }
        if let (Some(min), Some(max)) = (bounds.min, bounds.max) {
            if f64::from_bits(min) > f64::from_bits(max) {
                return None;
            }
        }
        Some(bounds)
    }
}

fn nbt_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(v) => Some(i32::from(*v)),
        NbtSummaryValue::Short(v) => Some(i32::from(*v)),
        NbtSummaryValue::Int(v) => Some(*v),
        _ => None,
    }
}

/// `NbtOps.getNumberValue` (`NbtOps.java:65-67`) widens every numeric tag via `Number.doubleValue()`.
fn nbt_f64_bits(value: &NbtSummaryValue) -> Option<u64> {
    match value {
        NbtSummaryValue::Byte(v) => Some(f64::from(*v).to_bits()),
        NbtSummaryValue::Short(v) => Some(f64::from(*v).to_bits()),
        NbtSummaryValue::Int(v) => Some(f64::from(*v).to_bits()),
        NbtSummaryValue::Long(v) => Some((*v as f64).to_bits()),
        NbtSummaryValue::Float(bits) => Some(f64::from(f32::from_bits(*bits)).to_bits()),
        NbtSummaryValue::Double(bits) => Some(*bits),
        _ => None,
    }
}

fn nbt_boolean(value: &NbtSummaryValue) -> Option<bool> {
    match value {
        NbtSummaryValue::Byte(0) => Some(false),
        NbtSummaryValue::Byte(1) => Some(true),
        _ => None,
    }
}
