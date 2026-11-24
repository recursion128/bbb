use bbb_protocol::{
    content_registry::ContentRegistry,
    data_component_types::vanilla_data_component_type,
    packets::{
        data_components::validate_data_component_patch_summary_with_registry, ConsumeEffectSummary,
        DataComponentPatchSummary,
    },
    NbtSummaryValue,
};

use crate::{
    read_item_component_into, registry_codec::RegistryElementSource,
    PersistentItemStackHolderResolver, UnavailableHolderResolver,
};

pub fn read_persistent_component_into_with_resolver(
    type_id: i32,
    value: &NbtSummaryValue,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
    patch: &mut DataComponentPatchSummary,
) -> Result<(), String> {
    let component_type = vanilla_data_component_type(type_id)
        .ok_or_else(|| format!("unknown persistent data component type id {type_id}"))?;
    if !component_type.persistent {
        return Err(format!(
            "data component {} is not persistent",
            component_type.resource_id
        ));
    }
    let resolver: &dyn PersistentItemStackHolderResolver = match resolver {
        Some(resolver) => resolver,
        None => &UnavailableHolderResolver,
    };
    read_item_component_into(type_id, value, resolver, patch).map_err(|error| error.to_string())
}

fn materialize_named_holder_set(
    ids: &mut Vec<i32>,
    tag: &Option<String>,
    registry: &'static str,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> Result<(), String> {
    let Some(tag) = tag else {
        return Ok(());
    };
    let resolver = resolver.ok_or_else(|| {
        format!("{registry} holder set cannot be decoded without registry context")
    })?;
    *ids = resolver.tag_holder_ids(registry, tag)?;
    Ok(())
}

pub fn materialize_patch_named_holder_sets_with_resolver(
    patch: &mut DataComponentPatchSummary,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> Result<(), String> {
    materialize_named_holder_set(
        &mut patch.damage_resistant_type_registry_ids,
        &patch.damage_resistant_type_tag,
        "minecraft:damage_type",
        resolver,
    )?;
    for rule in patch.tool.iter_mut().flat_map(|tool| tool.rules.iter_mut()) {
        materialize_named_holder_set(
            &mut rule.block_registry_ids,
            &rule.block_tag,
            "minecraft:block",
            resolver,
        )?;
    }
    if let Some(entities) = patch
        .equippable
        .as_mut()
        .and_then(|value| value.allowed_entities.as_mut())
    {
        materialize_named_holder_set(
            &mut entities.registry_ids,
            &entities.tag,
            "minecraft:entity_type",
            resolver,
        )?;
    }
    materialize_named_holder_set(
        &mut patch.repairable_item_registry_ids,
        &patch.repairable_item_tag,
        "minecraft:item",
        resolver,
    )?;
    for effect in patch
        .consumable_effects
        .iter_mut()
        .chain(&mut patch.death_protection_effects)
    {
        if let ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids,
            effect_tag,
        } = effect
        {
            materialize_named_holder_set(
                effect_registry_ids,
                effect_tag,
                "minecraft:mob_effect",
                resolver,
            )?;
        }
    }
    if let Some(attacks) = patch.blocks_attacks.as_mut() {
        for types in attacks
            .damage_reductions
            .iter_mut()
            .filter_map(|reduction| reduction.damage_types.as_mut())
            .chain(attacks.bypassed_by.as_mut())
        {
            materialize_named_holder_set(
                &mut types.registry_ids,
                &types.tag,
                "minecraft:damage_type",
                resolver,
            )?;
        }
    }
    materialize_named_holder_set(
        &mut patch.provides_banner_pattern_registry_ids,
        &patch.provides_banner_pattern_tag,
        "minecraft:banner_pattern",
        resolver,
    )?;
    for predicate in [
        patch.can_place_on_persistent.as_mut(),
        patch.can_break_persistent.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        for block in &mut predicate.predicates {
            if let Some(blocks) = block.blocks.as_mut() {
                materialize_named_holder_set(
                    &mut blocks.registry_ids,
                    &blocks.tag,
                    "minecraft:block",
                    resolver,
                )?;
            }
            if let Some(exact) = block.exact_components.as_deref_mut() {
                materialize_patch_named_holder_sets_with_resolver(exact, resolver)?;
            }
        }
    }
    if let Some(items) = patch
        .lock_predicate
        .as_mut()
        .and_then(|predicate| predicate.items.as_mut())
    {
        materialize_named_holder_set(
            &mut items.registry_ids,
            &items.tag,
            "minecraft:item",
            resolver,
        )?;
    }
    if let Some(remainder) = patch.use_remainder.as_deref_mut() {
        materialize_patch_named_holder_sets_with_resolver(
            &mut remainder.component_patch,
            resolver,
        )?;
    }
    for item in patch
        .charged_projectiles_items
        .iter_mut()
        .chain(&mut patch.bundle_contents_items)
        .chain(patch.container.slots.iter_mut().map(|slot| &mut slot.item))
    {
        materialize_patch_named_holder_sets_with_resolver(&mut item.component_patch, resolver)?;
    }
    Ok(())
}

pub fn decode_persistent_component_select_value_with_resolver(
    type_id: i32,
    value: &serde_json::Value,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> Result<Option<DataComponentPatchSummary>, String> {
    let value = bbb_protocol::json_value_to_nbt_summary(value)
        .map_err(|error| format!("component select value is not representable as NBT: {error}"))?;
    decode_persistent_component_select_value_from_source_with_resolver(
        type_id,
        &value,
        RegistryElementSource::ClientKnownPackJson,
        resolver,
    )
}

pub fn decode_persistent_component_select_value_from_nbt_with_resolver(
    type_id: i32,
    value: &NbtSummaryValue,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> Result<Option<DataComponentPatchSummary>, String> {
    decode_persistent_component_select_value_from_source_with_resolver(
        type_id,
        value,
        RegistryElementSource::NetworkNbt,
        resolver,
    )
}

pub fn decode_persistent_component_select_value_from_source_with_resolver(
    type_id: i32,
    value: &NbtSummaryValue,
    _source: RegistryElementSource,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> Result<Option<DataComponentPatchSummary>, String> {
    let mut patch = DataComponentPatchSummary::default();
    read_persistent_component_into_with_resolver(type_id, value, resolver, &mut patch)?;
    materialize_patch_named_holder_sets_with_resolver(&mut patch, resolver)?;
    let content_registry = resolver.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        PersistentItemStackHolderResolver::content_registry,
    );
    validate_data_component_patch_summary_with_registry(&patch, content_registry)
        .map_err(|error| error.to_string())?;
    Ok(Some(patch))
}
