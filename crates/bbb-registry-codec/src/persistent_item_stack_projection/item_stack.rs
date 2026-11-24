//! The stack-level projections: `ItemStack.CODEC` /
//! `ItemStack.OPTIONAL_CODEC` against the network projection, and the patch
//! entry loop that dispatches one component at a time.

use std::collections::{BTreeMap, BTreeSet};

use bbb_facts::item_types::{
    vanilla_item_type, VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE, VANILLA_ITEM_AIR_ID,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::{
    DataComponentPatchSummary, ItemStackSummary, SparseDataComponentPatchSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::persistent_item_nbt::PERSISTENT_ITEM_STACK_MIN_COUNT;
use crate::registry_codec::{
    recursive_values::{CanonicalRegistryCodecValue, OwnedRegistryCodecReference},
    RegistryResourceIdentifier,
};

use super::{
    component_sets::{declared_component, retained_persistent_component_nbt},
    error::{ItemStackPersistentProjectionError, Result},
    holder::{
        item_resource_id_in_registry, PersistentItemStackHolderResolver, UnavailableHolderResolver,
    },
    persistent_to_wire_components::{
        project_persistent_component, validate_persistent_scalar_component,
    },
    wire_to_persistent_components::typed_persistent_component_nbt,
};

/// The item holder id of a non-empty stack, or `None` when the summary is
/// `ItemStack.EMPTY`.
///
/// Reproduces `ItemStack.isEmpty`: a missing item,
/// `minecraft:air`, or a count of zero or less. `ItemStackSummary::empty`
/// spells the same state as `item_id: None, count: 0`.
fn non_empty_item_id(summary: &ItemStackSummary) -> Option<i32> {
    let item_id = summary.item_id?;
    (item_id != VANILLA_ITEM_AIR_ID && summary.count > 0).then_some(item_id)
}

/// `ItemStack.isEmpty` for the network projection.
pub fn item_stack_summary_is_empty(summary: &ItemStackSummary) -> bool {
    non_empty_item_id(summary).is_none()
}

/// Projects a summary through `ItemStack.CODEC` (`ItemStack.java:111-122`).
///
/// The record is `id` / `count` / `components`:
///
/// * `id` is a string, because `Item.CODEC` is
///   `BuiltInRegistries.ITEM.holderByNameCodec()` and `Identifier.toString`
///   always writes `namespace:path` (`Item`);
/// * `count` is an int and is always written -- `orElse(1)` supplies the
///   decode default but does not drop the field while encoding;
/// * `components` is omitted only for an empty patch
///   (`optionalFieldOf("components", DataComponentPatch.EMPTY)`).
///
/// The compound's own key order is not part of the format: a `CompoundTag` is
/// a `HashMap`. This writes `id`, `count` and
/// `components` in the order `MAP_CODEC` declares them.
pub fn item_stack_summary_to_persistent_nbt(summary: &ItemStackSummary) -> Result<NbtSummaryValue> {
    item_stack_summary_to_persistent_nbt_with_resolver(summary, &UnavailableHolderResolver)
}

/// Projects a summary with the exact dynamic-registry generation that issued
/// the holder ids it carries.
///
/// `minecraft:enchantments` needs it: `Enchantment.CODEC` is a
/// `RegistryFixedCodec` that writes each holder's identifier
/// so the projection
/// has to name ids the resolver-free entry point cannot.
pub fn item_stack_summary_to_persistent_nbt_with_resolver<R>(
    summary: &ItemStackSummary,
    resolver: &R,
) -> Result<NbtSummaryValue>
where
    R: PersistentItemStackHolderResolver + ?Sized,
{
    item_stack_summary_to_persistent_nbt_with_resolver_and_content_registry(
        summary,
        resolver,
        resolver.content_registry(),
    )
}

/// Registry-aware form of [`item_stack_summary_to_persistent_nbt_with_resolver`].
pub fn item_stack_summary_to_persistent_nbt_with_resolver_and_content_registry<R>(
    summary: &ItemStackSummary,
    resolver: &R,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue>
where
    R: PersistentItemStackHolderResolver + ?Sized,
{
    project_item_stack_summary(
        summary,
        &|registry_id, holder_id| resolver.holder_resource_id(registry_id, holder_id),
        content_registry,
    )
}

pub fn data_component_patch_to_persistent_nbt_with_resolver<R>(
    patch: &DataComponentPatchSummary,
    resolver: &R,
) -> Result<Option<NbtSummaryValue>>
where
    R: PersistentItemStackHolderResolver + ?Sized,
{
    data_component_patch_to_persistent_nbt_with_resolver_and_content_registry(
        patch,
        resolver,
        resolver.content_registry(),
    )
}

pub fn data_component_patch_to_persistent_nbt_with_resolver_and_content_registry<R>(
    patch: &DataComponentPatchSummary,
    resolver: &R,
    content_registry: &ContentRegistry,
) -> Result<Option<NbtSummaryValue>>
where
    R: PersistentItemStackHolderResolver + ?Sized,
{
    data_component_patch_to_persistent_nbt(
        patch,
        &|registry_id, holder_id| resolver.holder_resource_id(registry_id, holder_id),
        content_registry,
        0,
        0,
    )
}

fn project_item_stack_summary(
    summary: &ItemStackSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let Some(item_id) = non_empty_item_id(summary) else {
        return Err(ItemStackPersistentProjectionError::EmptyStack);
    };
    let item_resource_id = item_resource_id_in_registry(item_id, content_registry)
        .ok_or(ItemStackPersistentProjectionError::UnknownItem { item_id })?;
    if !(PERSISTENT_ITEM_STACK_MIN_COUNT..=VANILLA_ITEM_ABSOLUTE_MAX_STACK_SIZE)
        .contains(&summary.count)
    {
        return Err(ItemStackPersistentProjectionError::CountOutOfRange {
            count: summary.count,
        });
    }

    let mut entries = vec![
        NbtSummaryEntry {
            name: "id".into(),
            value: NbtSummaryValue::String(item_resource_id.into()),
        },
        NbtSummaryEntry {
            name: "count".into(),
            value: NbtSummaryValue::Int(summary.count),
        },
    ];
    if let Some(components) = data_component_patch_to_persistent_nbt(
        &summary.component_patch,
        holder_resource_id,
        content_registry,
        0,
        0,
    )? {
        entries.push(NbtSummaryEntry {
            name: "components".into(),
            value: components,
        });
    }
    Ok(NbtSummaryValue::Compound(entries))
}

pub(super) fn project_persistent_patch_into(
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    project_persistent_patch_into_at_depth(value, holder_id, content_registry, patch, 0, 0)?;
    bbb_protocol::packets::data_components::validate_persistent_data_component_patch_summary_with_registry(
        patch,
        content_registry,
    )
    .map_err(|error| ItemStackPersistentProjectionError::InvalidWireProjection {
        detail: error.to_string(),
    })?;
    Ok(())
}

pub(super) fn project_persistent_patch_into_at_depth(
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
    predicate_depth: usize,
    item_depth: usize,
) -> Result<()> {
    let entries = crate::persistent_item_nbt::component_entries(value).map_err(structure_error)?;
    let mut seen = BTreeSet::new();
    for entry in entries {
        let (removed, component, value) =
            crate::persistent_item_nbt::decode_component_entry(entry).map_err(structure_error)?;
        let type_id = component.type_id;
        let declared = declared_component(type_id, &mut seen)?;
        if removed {
            patch.removed_type_ids.push(type_id);
            continue;
        }
        let value = value.ok_or(ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id: declared.resource_id,
        })?;
        project_persistent_component(
            type_id,
            declared.resource_id,
            value,
            holder_id,
            content_registry,
            patch,
            predicate_depth,
            item_depth,
        )?;
        patch.added_type_ids.push(type_id);
    }
    patch.added_type_ids.sort_unstable();
    patch.removed_type_ids.sort_unstable();
    Ok(())
}

/// Projects a component patch through `DataComponentPatch.CODEC`
/// (`DataComponentPatch.java:23-57`), or `None` when `ItemStack.MAP_CODEC`
/// omits the field.
///
/// Three encode-side rules come straight from that codec:
///
/// * transient components are dropped -- the `xmap` that turns a patch into
///   the dispatched map skips every `type.isTransient()` entry
///   (`DataComponentPatch.java:41-57`);
/// * a present entry is keyed by `id.toString()` and a removed one by
///   `"!" + id` (`DataComponentPatch.java:289-313`), the removed value being
///   `Codec.EMPTY.codec()`, an empty compound
///   (`DataComponentPatch.java:315-317`);
/// * whether `ItemStack.MAP_CODEC` omits the field is decided *before* that
///   filter. `optionalFieldOf` compares the patch with `DataComponentPatch.EMPTY`
///   (`DataComponentPatch.java:22`) through `equals`, which compares the whole
///   map including transient entries (`DataComponentPatch.java:210-212`). A
///   patch holding nothing but transient entries is therefore written as an
///   empty `components` compound rather than dropped.
pub(super) fn data_component_patch_to_persistent_nbt(
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
    content_registry: &ContentRegistry,
    predicate_depth: usize,
    item_depth: usize,
) -> Result<Option<NbtSummaryValue>> {
    if patch.added_type_ids.is_empty() && patch.removed_type_ids.is_empty() {
        return Ok(None);
    }

    let mut seen = BTreeSet::new();
    let mut projected: BTreeMap<i32, NbtSummaryEntry> = BTreeMap::new();
    for &type_id in &patch.added_type_ids {
        let component = declared_component(type_id, &mut seen)?;
        if !component.persistent {
            continue;
        }
        // Typed components validate their persistent invariants directly in
        // `typed_persistent_component_nbt` without re-parsing the emitted tree
        // (avoiding O(depth^2) full-tree re-parses on nested containers/bundles).
        // Untyped retained fallback entries still run the persistent reader.
        let value = match typed_persistent_component_nbt(
            type_id,
            component.resource_id,
            patch,
            holder_resource_id,
            content_registry,
            predicate_depth,
            item_depth,
        ) {
            Some(value) => value?,
            None => {
                let value = retained_persistent_component_nbt(type_id, patch)
                    .cloned()
                    .ok_or(ItemStackPersistentProjectionError::UnsupportedComponent {
                        type_id,
                        resource_id: component.resource_id,
                    })?;
                validate_untyped_persistent_component(
                    type_id,
                    component.resource_id,
                    &value,
                    content_registry,
                )?;
                value
            }
        };
        projected.insert(
            type_id,
            NbtSummaryEntry {
                name: component.resource_id.into(),
                value,
            },
        );
    }
    for &type_id in &patch.removed_type_ids {
        let component = declared_component(type_id, &mut seen)?;
        if !component.persistent {
            continue;
        }
        projected.insert(
            type_id,
            NbtSummaryEntry {
                name: format!("!{}", component.resource_id).into(),
                value: NbtSummaryValue::Compound(Vec::new()),
            },
        );
    }

    Ok(Some(NbtSummaryValue::Compound(
        projected.into_values().collect(),
    )))
}

/// Validates an untyped retained fallback component through the persistent
/// reader, with every dynamic holder name taken as published.
pub(super) fn validate_untyped_persistent_component(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<()> {
    if let Some(result) = validate_persistent_scalar_component(type_id, value) {
        return result;
    }
    let next_holder = std::cell::Cell::new(0);
    read_persistent_component_with_content_registry(
        type_id,
        resource_id,
        value,
        &|_, _| Some(next_holder.replace(next_holder.get() + 1)),
        content_registry,
    )
    .map(drop)
}

/// Reads one component value into a single-component patch: the reader the
/// stack projection uses, for callers that only validate or compare values.
pub(crate) fn read_persistent_component(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
) -> Result<DataComponentPatchSummary> {
    read_persistent_component_with_content_registry(
        type_id,
        resource_id,
        value,
        holder_id,
        ContentRegistry::vanilla(),
    )
}

fn read_persistent_component_with_content_registry(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
) -> Result<DataComponentPatchSummary> {
    let mut patch = Box::<DataComponentPatchSummary>::default();
    project_persistent_component(
        type_id,
        resource_id,
        value,
        holder_id,
        content_registry,
        &mut patch,
        0,
        0,
    )?;
    Ok(*patch)
}

/// Re-encodes one component and retains its unresolved holder references.
pub(crate) fn canonical_persistent_component(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    owner_registry: &str,
    content_registry: &ContentRegistry,
) -> Result<CanonicalRegistryCodecValue> {
    let references = std::cell::RefCell::new(Vec::new());
    let mut patch = read_persistent_component_with_content_registry(
        type_id,
        resource_id,
        value,
        &|registry, name| {
            let name = RegistryResourceIdentifier::parse(name)?;
            let mut references = references.borrow_mut();
            references.push(OwnedRegistryCodecReference::element(registry, name));
            i32::try_from(references.len() - 1).ok()
        },
        content_registry,
    )?;
    if type_id == bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID
        && bbb_facts::registry_data_types::synchronized_registry_data_type(owner_registry).is_some()
    {
        // PotionContents: publication uses the getter's effect copies.
        for effect in &mut patch.potion_contents.custom_effects {
            effect.hidden_effect = None;
        }
    }
    let references = references.into_inner();
    let holder_resource_id = |_: &str, id: i32| {
        references
            .get(usize::try_from(id).ok()?)
            .map(|reference| reference.borrowed().resource_id.as_str().to_owned())
    };
    let value = typed_persistent_component_nbt(
        type_id,
        resource_id,
        &patch,
        &holder_resource_id,
        content_registry,
        0,
        0,
    )
    .unwrap_or_else(|| {
        retained_persistent_component_nbt(type_id, &patch)
            .cloned()
            .ok_or(ItemStackPersistentProjectionError::UnsupportedComponent {
                type_id,
                resource_id,
            })
    })?;
    Ok(CanonicalRegistryCodecValue::from_parts(value, references))
}

fn structure_error(
    error: bbb_protocol::codec::ProtocolError,
) -> ItemStackPersistentProjectionError {
    ItemStackPersistentProjectionError::InvalidWireProjection {
        detail: error.to_string(),
    }
}

/// Reads one saved `ItemStack.OPTIONAL_CODEC` value (`{}` is the empty stack)
/// into the summary every live stack is held as.
pub fn read_item_stack(
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<ItemStackSummary> {
    read_item_stack_with_content_registry(value, resolver, resolver.content_registry())
}

/// Registry-aware form of [`read_item_stack`].
pub fn read_item_stack_with_content_registry(
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
    content_registry: &ContentRegistry,
) -> Result<ItemStackSummary> {
    let Some((item_id, count, components)) =
        crate::persistent_item_nbt::decode_item_stack_fields_with_content_registry(
            value,
            content_registry,
        )
        .map_err(structure_error)?
    else {
        return Ok(ItemStackSummary::empty());
    };
    let component_patch = if let Some(components) = components {
        let mut patch = Box::<DataComponentPatchSummary>::default();
        read_component_patch_into_with_content_registry(
            components,
            resolver,
            content_registry,
            &mut patch,
        )?;
        patch.into()
    } else {
        SparseDataComponentPatchSummary::empty()
    };
    Ok(ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch,
    })
}

/// Reads a `DataComponentPatch.CODEC` / `DataComponentMap.CODEC` compound.
pub fn read_component_patch(
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<DataComponentPatchSummary> {
    read_component_patch_with_content_registry(value, resolver, resolver.content_registry())
}

pub fn read_component_patch_with_content_registry(
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
    content_registry: &ContentRegistry,
) -> Result<DataComponentPatchSummary> {
    let mut patch = DataComponentPatchSummary::default();
    read_component_patch_into_with_content_registry(value, resolver, content_registry, &mut patch)?;
    Ok(patch)
}

/// Reads a patch into a newly initialized empty destination. Discard the
/// destination after an error, which may leave partial output.
pub fn read_component_patch_into(
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    read_component_patch_into_with_content_registry(
        value,
        resolver,
        resolver.content_registry(),
        patch,
    )
}

pub fn read_component_patch_into_with_content_registry(
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    project_persistent_patch_into(
        value,
        &|registry, resource| resolver.holder_id(registry, resource),
        content_registry,
        patch,
    )
}

/// The persistent NBT of one effective component: the patched value, or the
/// item prototype's when the patch does not mention it.
pub fn item_component_nbt(
    summary: &ItemStackSummary,
    type_id: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<Option<NbtSummaryValue>> {
    item_component_nbt_with_content_registry(
        summary,
        type_id,
        resolver,
        resolver.content_registry(),
    )
}

pub fn item_component_nbt_with_content_registry(
    summary: &ItemStackSummary,
    type_id: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
    content_registry: &ContentRegistry,
) -> Result<Option<NbtSummaryValue>> {
    use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID;
    let Some(item_id) = non_empty_item_id(summary) else {
        return Ok(None);
    };
    let patch = &summary.component_patch;
    if patch.removed_type_ids.contains(&type_id) {
        return Ok(None);
    }
    let component = bbb_facts::data_component_types::vanilla_data_component_type(type_id)
        .ok_or(ItemStackPersistentProjectionError::UnknownComponent { type_id })?;
    if patch.added_type_ids.contains(&type_id) {
        if type_id == VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID {
            return Ok(patch.additional_trade_cost.map(NbtSummaryValue::Int));
        }
        let holder_resource_id =
            |registry: &str, holder: i32| resolver.holder_resource_id(registry, holder);
        return match typed_persistent_component_nbt(
            type_id,
            component.resource_id,
            patch,
            &holder_resource_id,
            content_registry,
            0,
            0,
        ) {
            Some(value) => value.map(Some),
            None => retained_persistent_component_nbt(type_id, patch)
                .cloned()
                .map(Some)
                .ok_or(ItemStackPersistentProjectionError::UnsupportedComponent {
                    type_id,
                    resource_id: component.resource_id,
                }),
        };
    }
    let Some(item) = vanilla_item_type(item_id) else {
        if let Some(mod_item) = content_registry.item_by_id(item_id) {
            if type_id
                == bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID
            {
                return Ok(Some(NbtSummaryValue::Int(i32::from(
                    mod_item.max_stack_size,
                ))));
            }
            return Ok(None);
        }
        return Err(ItemStackPersistentProjectionError::UnknownItem { item_id });
    };
    let Some(raw) = crate::persistent_component_nbt::prototype_component_value(
        item_id,
        item.resource_id,
        type_id,
    )
    .map_err(
        |error| ItemStackPersistentProjectionError::InvalidWireProjection {
            detail: error.to_string(),
        },
    )?
    else {
        return Ok(None);
    };
    // The prototype catalog is JSON, so its numbers lose their NBT tag type;
    // re-encode through the component codec when the holders resolve.
    let holder_resource_id =
        |registry: &str, holder: i32| resolver.holder_resource_id(registry, holder);
    let typed =
        read_item_component_with_content_registry(type_id, &raw, resolver, content_registry)
            .ok()
            .and_then(|patch| {
                typed_persistent_component_nbt(
                    type_id,
                    component.resource_id,
                    &patch,
                    &holder_resource_id,
                    content_registry,
                    0,
                    0,
                )
                .and_then(Result::ok)
            });
    Ok(Some(typed.unwrap_or(raw)))
}

/// One component read the way a stack's `components` entry is read, as a
/// single-component patch.
pub fn read_item_component(
    type_id: i32,
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<DataComponentPatchSummary> {
    read_item_component_with_content_registry(type_id, value, resolver, resolver.content_registry())
}

pub fn read_item_component_with_content_registry(
    type_id: i32,
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
    content_registry: &ContentRegistry,
) -> Result<DataComponentPatchSummary> {
    let mut patch = DataComponentPatchSummary::default();
    read_item_component_into_with_content_registry(
        type_id,
        value,
        resolver,
        content_registry,
        &mut patch,
    )?;
    Ok(patch)
}

/// Reads a component whose value and membership are absent from the destination.
/// Other components are retained. Discard the destination after an error.
pub fn read_item_component_into(
    type_id: i32,
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    read_item_component_into_with_content_registry(
        type_id,
        value,
        resolver,
        resolver.content_registry(),
        patch,
    )
}

pub fn read_item_component_into_with_content_registry(
    type_id: i32,
    value: &NbtSummaryValue,
    resolver: &dyn PersistentItemStackHolderResolver,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let component = bbb_facts::data_component_types::vanilla_data_component_type(type_id)
        .ok_or(ItemStackPersistentProjectionError::UnknownComponent { type_id })?;
    project_persistent_component(
        type_id,
        component.resource_id,
        value,
        &|registry, resource| resolver.holder_id(registry, resource),
        content_registry,
        patch,
        0,
        0,
    )?;
    patch.added_type_ids.push(type_id);
    Ok(())
}
