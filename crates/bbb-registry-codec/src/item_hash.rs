//! Shared Vanilla `HashOps.CRC32C_INSTANCE`, `HashedStack`, and `HashedPatchMap`
//! digest computation and verification.
//!
//! Anchors:
//! - `HashOps` (`CRC32C_INSTANCE` tag framing and canonical map sorting)
//! - `HashedStack` (`HashedStack.ActualItem.matches`)
//! - `HashedPatchMap` (`HashedPatchMap.matches`)

use std::collections::{BTreeMap, BTreeSet};

use bbb_facts::item_types::{vanilla_item_type, VANILLA_ITEM_AIR_ID};
use bbb_facts::static_registry_elements::vanilla_static_registry_elements;
use bbb_facts::PersistentRegistryMembership;
use bbb_protocol::packets::{
    DataComponentPatchSummary, HashedComponentPatch, HashedItemStack, HashedStack, ItemStackSummary,
};

use crate::persistent_item_stack_projection::{
    item_stack_summary_is_empty, PersistentItemStackHolderResolver, UnavailableHolderResolver,
};

pub mod components;
pub mod components_predicates;
pub mod components_rich;
pub mod ops;
#[cfg(test)]
mod tests;

pub use components::{hash_component_value, hash_firework_explosion};
pub use components_predicates::{
    hash_block_predicate, hash_concrete_component_predicate, hash_partial_component_predicates,
    hash_persistent_item_predicate,
};
pub use components_rich::{hash_component, hash_resolvable_profile};
pub use ops::{
    crc32c, hash_boolean, hash_byte, hash_byte_array, hash_double, hash_empty, hash_empty_list,
    hash_empty_map, hash_float, hash_int, hash_int_array, hash_java_string, hash_list, hash_long,
    hash_long_array, hash_map, hash_nbt, hash_ops_crc32c_int, hash_record, hash_short, hash_string,
    normalize_codec_identifier, HASH_TAG_BOOLEAN, HASH_TAG_BYTE, HASH_TAG_BYTE_ARRAY_END,
    HASH_TAG_BYTE_ARRAY_START, HASH_TAG_DOUBLE, HASH_TAG_EMPTY, HASH_TAG_FLOAT, HASH_TAG_INT,
    HASH_TAG_INT_ARRAY_END, HASH_TAG_INT_ARRAY_START, HASH_TAG_LIST_END, HASH_TAG_LIST_START,
    HASH_TAG_LONG, HASH_TAG_LONG_ARRAY_END, HASH_TAG_LONG_ARRAY_START, HASH_TAG_MAP_END,
    HASH_TAG_MAP_START, HASH_TAG_SHORT, HASH_TAG_STRING,
};

/// Crate-neutral registry holder and dialog lookup for `HashOps` encoding and
/// `HashedStack` verification across both client prediction and server validation.
pub trait ItemHashLookup: PersistentItemStackHolderResolver {
    /// Resolves a numeric item id to its canonical resource id, checking dynamic
    /// holder overrides first, then built-in Vanilla 26.1 items (O(1)), then
    /// custom items in [`PersistentItemStackHolderResolver::content_registry`].
    fn item_resource_id(&self, item_id: i32) -> Option<String> {
        self.holder_resource_id("minecraft:item", item_id)
            .or_else(|| vanilla_item_type(item_id).map(|entry| entry.resource_id.to_owned()))
            .or_else(|| {
                self.content_registry()
                    .item_by_id(item_id)
                    .map(|entry| entry.resource_id.clone())
            })
    }

    /// Resolves a registry holder id to its canonical resource id, checking session
    /// registries first, then built-in Vanilla 26.1 static catalogs, then
    /// [`PersistentItemStackHolderResolver::content_registry`] for custom content.
    fn resolve_holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        if let Some(id) = self.holder_resource_id(registry_id, holder_id) {
            return Some(id);
        }
        if let Some(id) = usize::try_from(holder_id).ok().and_then(|idx| {
            vanilla_static_registry_elements(registry_id)
                .and_then(|elements| elements.get(idx).copied())
        }) {
            return Some(id.to_owned());
        }
        let content_registry = self.content_registry();
        match registry_id {
            "minecraft:item" => content_registry
                .item_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:block" => content_registry
                .block_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:entity_type" => content_registry
                .entity_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:sound_event" => content_registry
                .sound_event_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            "minecraft:particle_type" => content_registry
                .particle_by_id(holder_id)
                .map(|entry| entry.resource_id.clone()),
            _ => None,
        }
    }

    /// Whether `minecraft:dialog` contains a synchronized entry named `resource_id`.
    fn contains_dialog(&self, resource_id: &str) -> bool {
        self.holder_id("minecraft:dialog", resource_id).is_some()
            || self.resource_membership("minecraft:dialog", resource_id)
                == PersistentRegistryMembership::Present
    }
}

impl ItemHashLookup for UnavailableHolderResolver {}

/// Built-in Vanilla-only [`ItemHashLookup`] with static catalogs available and
/// dynamic datapack registries unresolved.
#[derive(Debug, Clone, Copy, Default)]
pub struct VanillaHashLookup;

impl bbb_facts::PersistentRegistryResolver for VanillaHashLookup {
    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        UnavailableHolderResolver.resource_membership(registry_id, resource_id)
    }

    fn tag_members(
        &self,
        registry_id: &str,
        tag_id: &str,
    ) -> bbb_facts::PersistentRegistryTagMembers {
        UnavailableHolderResolver.tag_members(registry_id, tag_id)
    }
}

impl PersistentItemStackHolderResolver for VanillaHashLookup {
    fn holder_id(&self, _registry_id: &str, _resource_id: &str) -> Option<i32> {
        None
    }
}

impl ItemHashLookup for VanillaHashLookup {}

/// Encodes an [`ItemStackSummary`] into its wire [`HashedStack`], matching
/// `HashedStack.create`, or `None` if an added
/// component cannot be hashed (transient or unknown dynamic registry holder).
pub fn hashed_stack_from_summary(
    lookup: &dyn ItemHashLookup,
    stack: &ItemStackSummary,
) -> Option<HashedStack> {
    if item_stack_summary_is_empty(stack) {
        return Some(HashedStack::Empty);
    }
    let item_id = stack.item_id?;
    if item_id == VANILLA_ITEM_AIR_ID || lookup.item_resource_id(item_id).is_none() {
        return None;
    }
    let components = hashed_component_patch_from_summary(lookup, &stack.component_patch)?;
    Some(HashedStack::Item(HashedItemStack {
        item_id,
        count: stack.count,
        components,
    }))
}

/// Encodes a [`DataComponentPatchSummary`] into its [`HashedComponentPatch`],
/// returning `None` if any added component has no hashable persistent form.
pub fn hashed_component_patch_from_summary(
    lookup: &dyn ItemHashLookup,
    patch: &DataComponentPatchSummary,
) -> Option<HashedComponentPatch> {
    let mut added_components = BTreeMap::new();
    for &type_id in &patch.added_type_ids {
        let digest = hash_component_value(lookup, patch, type_id)?;
        added_components.insert(type_id, digest as i32);
    }
    let removed_components: BTreeSet<i32> = patch.removed_type_ids.iter().copied().collect();
    Some(HashedComponentPatch {
        added_components,
        removed_components,
    })
}

/// Verifies a client-supplied [`HashedComponentPatch`] against a local patch
/// (`HashedPatchMap.matches`).
pub fn hashed_patch_matches(
    lookup: &dyn ItemHashLookup,
    expected: &HashedComponentPatch,
    local: &DataComponentPatchSummary,
) -> bool {
    let local_removed: BTreeSet<i32> = local.removed_type_ids.iter().copied().collect();
    if local_removed != expected.removed_components {
        return false;
    }
    let local_added: BTreeSet<i32> = local.added_type_ids.iter().copied().collect();
    if local_added.len() != expected.added_components.len() {
        return false;
    }
    for type_id in local_added {
        let Some(expected_hash) = expected.added_components.get(&type_id) else {
            return false;
        };
        let Some(actual_hash) = hash_component_value(lookup, local, type_id) else {
            return false;
        };
        if (actual_hash as i32) != *expected_hash {
            return false;
        }
    }
    true
}

/// Verifies a client-supplied [`HashedStack`] against a local [`ItemStackSummary`]
/// (`HashedStack.ActualItem.matches`).
pub fn hashed_stack_matches(
    lookup: &dyn ItemHashLookup,
    expected: &HashedStack,
    local: &ItemStackSummary,
) -> bool {
    match expected {
        HashedStack::Empty => item_stack_summary_is_empty(local),
        HashedStack::Item(expected_item) => {
            if item_stack_summary_is_empty(local)
                || expected_item.count != local.count
                || local.item_id != Some(expected_item.item_id)
            {
                return false;
            }
            hashed_patch_matches(lookup, &expected_item.components, &local.component_patch)
        }
    }
}
