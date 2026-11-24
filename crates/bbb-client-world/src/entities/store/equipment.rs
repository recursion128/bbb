use super::*;

/// Vanilla 26.1 `EntityType.PIG` registry id, used to gate `PigRenderState.saddle`.
/// Vanilla 26.1 `EntityType.WOLF` registry id, used to gate `WolfArmorLayer`.
/// Vanilla 26.1 skeleton-family ids whose renderers extend `HumanoidMobRenderer`
/// and therefore feed `ItemInHandLayer` through `getItemHeldByArm`.
/// Vanilla 26.1 `EvokerRenderer` installs an `ItemInHandLayer` over the illager model.
/// Vanilla 26.1 `ZombifiedPiglinRenderer` extends `HumanoidMobRenderer`.
pub(super) fn wolf_armor_crackiness(
    item: &ItemStackSummary,
    profiles: &crate::ItemProfiles,
) -> WolfArmorCrackiness {
    let Some((damage, max_damage)) =
        crate::ItemProfiles::damage_state(Some(profiles), item.item_id, &item.component_patch)
    else {
        return WolfArmorCrackiness::None;
    };
    let fraction = (max_damage - damage) as f32 / max_damage as f32;
    if fraction < 0.32 {
        WolfArmorCrackiness::High
    } else if fraction < 0.69 {
        WolfArmorCrackiness::Medium
    } else if fraction < 0.95 {
        WolfArmorCrackiness::Low
    } else {
        WolfArmorCrackiness::None
    }
}

pub(super) fn vanilla_camel_saddle_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_camel(entity_type_id)
}

pub(super) fn equipment_saddle_item<'a>(
    equipment: Option<&'a EntityEquipment>,
    profiles: &crate::ItemProfiles,
) -> Option<&'a ItemStackSummary> {
    let item = equipment?
        .equipment
        .iter()
        .find(|update| update.slot == ProtocolEquipmentSlot::Saddle)
        .map(|update| &update.item)?;
    if item.count <= 0 {
        return None;
    }
    (profiles.item_stack_equipment(item)?.0 == ProtocolEquipmentSlot::Saddle).then_some(item)
}

pub(super) fn equipment_contains_saddle_item(
    equipment: Option<&EntityEquipment>,
    profiles: &crate::ItemProfiles,
) -> bool {
    equipment_saddle_item(equipment, profiles).is_some()
}

pub(super) fn equipment_slot_item(
    equipment: Option<&EntityEquipment>,
    slot: ProtocolEquipmentSlot,
) -> Option<&ItemStackSummary> {
    equipment?
        .equipment
        .iter()
        .find(|update| update.slot == slot)
        .map(|update| &update.item)
        .filter(|item| item.count > 0)
}

pub(super) fn equipment_asset_name(
    generation: u64,
    asset_names: &[String],
    asset_id: EntityEquipmentAssetId,
) -> Option<&str> {
    if asset_id.generation != generation {
        return None;
    }
    usize::try_from(asset_id.index)
        .ok()
        .and_then(|asset_id| asset_names.get(asset_id))
        .map(String::as_str)
}

pub(super) fn effective_equipment_asset(
    item: &ItemStackSummary,
    required_slot: Option<ProtocolEquipmentSlot>,
    profiles: &crate::ItemProfiles,
) -> Option<EntityEquipmentAssetId> {
    let (slot, asset) = profiles.item_stack_equipment(item)?;
    if required_slot.is_some_and(|required| required != slot) {
        return None;
    }
    asset
}
