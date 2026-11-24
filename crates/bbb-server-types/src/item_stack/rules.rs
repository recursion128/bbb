use crate::{Result, ServerWorldError};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
    },
    equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS},
    packets::ItemStackSummary,
    vanilla_item_prototype_component_catalog,
};
const ABSENT_MAX_STACK_SIZE: i32 = 1;

pub fn add_patched_component(item: &mut ItemStackSummary, type_id: i32) {
    item.component_patch
        .removed_type_ids
        .retain(|removed| *removed != type_id);
    if !item.component_patch.added_type_ids.contains(&type_id) {
        item.component_patch.added_type_ids.push(type_id);
    }
}

pub fn remove_patched_component(item: &mut ItemStackSummary, type_id: i32) {
    item.component_patch
        .added_type_ids
        .retain(|added| *added != type_id);
}

pub fn has_component(item: &ItemStackSummary, type_id: i32) -> Result<bool> {
    let Some(item_id) = item.item_id else {
        return Ok(false);
    };
    if item.component_patch.removed_type_ids.contains(&type_id) {
        return Ok(false);
    }
    if item.component_patch.added_type_ids.contains(&type_id) {
        return Ok(true);
    }
    let catalog = vanilla_item_prototype_component_catalog()
        .map_err(|_| ServerWorldError::MissingCatalogFact("item prototype component catalog"))?;
    Ok(catalog.has_component(item_id, type_id))
}

/// `UseEffects.interactVibrations` (`UseEffects.java:9-10`), `false` for a
/// stack without the component.
pub fn interact_vibrations(item: &ItemStackSummary) -> bool {
    let Some(item_id) = item.item_id.filter(|_| item.count > 0) else {
        return false;
    };
    let type_id = VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID;
    if !has_component(item, type_id).unwrap_or(false) {
        return false;
    }
    if item.component_patch.added_type_ids.contains(&type_id) {
        return item
            .component_patch
            .use_effects
            .is_some_and(|effects| effects.interact_vibrations);
    }
    prototype_components(item_id)
        .ok()
        .flatten()
        .and_then(|components| components.get(&type_id))
        .map(|value| {
            value
                .get("interact_vibrations")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
        })
        .unwrap_or(false)
}

pub fn component_i32(
    item: &ItemStackSummary,
    type_id: i32,
    patched: Option<i32>,
    absent: i32,
) -> Result<i32> {
    let Some(item_id) = item.item_id else {
        return Ok(absent);
    };
    if item.component_patch.removed_type_ids.contains(&type_id) {
        return Ok(absent);
    }
    if let Some(value) = patched {
        return Ok(value);
    }
    let catalog = vanilla_item_prototype_component_catalog()
        .map_err(|_| ServerWorldError::MissingCatalogFact("item prototype component catalog"))?;
    Ok(catalog
        .int_component(item_id, type_id)
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(absent))
}

pub fn prototype_components(
    item_id: i32,
) -> Result<Option<&'static std::collections::BTreeMap<i32, serde_json::Value>>> {
    let catalog = vanilla_item_prototype_component_catalog()
        .map_err(|_| ServerWorldError::MissingCatalogFact("item prototype component catalog"))?;
    Ok(catalog.component_codec_values(item_id))
}

pub fn item_prototype_int_component(item_id: i32, type_id: i32) -> Option<i32> {
    let catalog = vanilla_item_prototype_component_catalog().ok()?;
    catalog
        .int_component(item_id, type_id)
        .and_then(|value| i32::try_from(value).ok())
}

pub fn max_stack_size(item: &ItemStackSummary) -> Result<i32> {
    max_stack_size_with_registry(item, ContentRegistry::vanilla())
}

pub fn max_stack_size_with_registry(
    item: &ItemStackSummary,
    registry: &ContentRegistry,
) -> Result<i32> {
    let Some(item_id) = item.item_id else {
        return Ok(ABSENT_MAX_STACK_SIZE);
    };
    if item
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID)
    {
        return Ok(ABSENT_MAX_STACK_SIZE);
    }
    if let Some(value) = item.component_patch.max_stack_size {
        return Ok(value);
    }
    let catalog = vanilla_item_prototype_component_catalog()
        .map_err(|_| ServerWorldError::MissingCatalogFact("item prototype component catalog"))?;
    if let Some(value) = catalog
        .int_component(item_id, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID)
        .and_then(|value| i32::try_from(value).ok())
    {
        return Ok(value);
    }
    Ok(registry
        .item_by_id(item_id)
        .map(|spec| i32::from(spec.max_stack_size))
        .unwrap_or(ABSENT_MAX_STACK_SIZE))
}

pub fn is_same_item_same_components(a: &ItemStackSummary, b: &ItemStackSummary) -> bool {
    a.item_id == b.item_id && (a.item_id.is_none() || a.component_patch == b.component_patch)
}

pub fn is_equippable_in_slot(item: &ItemStackSummary, slot: EquipmentSlot) -> Result<bool> {
    Ok(
        super::static_item_component(item, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)?
            .and_then(|patch| patch.equippable.as_ref())
            .and_then(|equippable| {
                bbb_protocol::equipment_slots::vanilla_equipment_slot_by_id(equippable.slot_id)
            })
            .is_some_and(|declared| declared.slot == slot),
    )
}

pub fn equipment_slot_serialized_name(slot: EquipmentSlot) -> &'static str {
    VANILLA_EQUIPMENT_SLOTS
        .iter()
        .find(|entry| entry.slot == slot)
        .expect("the generated catalog carries every equipment slot")
        .serialized_name
}

pub fn equipment_slot_for_item(item: &ItemStackSummary) -> Result<EquipmentSlot> {
    Ok(
        super::static_item_component(item, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)?
            .and_then(|patch| patch.equippable.as_ref())
            .and_then(|equippable| {
                bbb_protocol::equipment_slots::vanilla_equipment_slot_by_id(equippable.slot_id)
            })
            .map_or(EquipmentSlot::MainHand, |entry| entry.slot),
    )
}

pub fn damageable_max_damage(stack: &ItemStackSummary) -> Option<i32> {
    if stack.item_id.is_none() || stack.count <= 0 {
        return None;
    }
    if has_component(stack, VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID).unwrap_or(false) {
        return None;
    }
    if !has_component(stack, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID).unwrap_or(false) {
        return None;
    }
    effective_int_component(
        stack,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        stack.component_patch.max_damage,
    )
}

pub fn damage_and_max(stack: &ItemStackSummary) -> Option<(i32, i32)> {
    let max_damage = damageable_max_damage(stack)?;
    Some((current_damage(stack, max_damage), max_damage))
}

pub fn current_damage(stack: &ItemStackSummary, max_damage: i32) -> i32 {
    effective_int_component(
        stack,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        stack.component_patch.damage,
    )
    .unwrap_or(0)
    .clamp(0, max_damage.max(0))
}

fn effective_int_component(
    stack: &ItemStackSummary,
    type_id: i32,
    patch_value: Option<i32>,
) -> Option<i32> {
    let patch = &stack.component_patch;
    if patch.removed_type_ids.contains(&type_id) {
        return None;
    }
    if patch.added_type_ids.contains(&type_id) {
        return patch_value;
    }
    item_prototype_int_component(stack.item_id?, type_id)
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
