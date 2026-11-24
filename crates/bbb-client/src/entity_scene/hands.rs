use super::*;
use bbb_protocol::packets::SwingAnimationTypeSummary;

pub(super) fn item_stack_swing_animation_type(
    world: &WorldStore,
    stack: Option<&ItemStackSummary>,
) -> SwingAnimationTypeSummary {
    stack
        .map(|stack| world.item_stack_swing_animation(stack).animation_type)
        .unwrap_or(SwingAnimationTypeSummary::Whack)
}

/// Whether the item in the given hand has the `BLOCK` use-animation (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.BLOCK`, returned by `Item.getUseAnimation` for a non-consumable item carrying
/// `DataComponents.BLOCKS_ATTACKS`). While the entity raises it, `HumanoidModel.poseRightArm`/`poseLeftArm`
/// `poseBlockingArm` tucks that arm's blocking item forward. Prototype defaults
/// and explicit stack patches are resolved through the same world generation.
pub(super) fn item_stack_blocks_attacks(world: &WorldStore, stack: &ItemStackSummary) -> bool {
    if stack.item_id.is_none() {
        return false;
    }
    if stack
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
        && !stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
    {
        return false;
    }
    world.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
}

pub(super) fn item_stack_non_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_some() && stack.count > 0
}

/// The supported skull block item in the HEAD equipment slot, if any. Vanilla
/// `LivingEntityRenderer.extractRenderState` routes skull `BlockItem`s to `wornHeadType` and clears the
/// generic head item; bbb mirrors the implemented static mob, player-default, dragon, and piglin skull
/// branches.
pub(super) fn entity_custom_head_skull(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
) -> Option<EntityCustomHeadSkull> {
    let item_runtime = item_runtime?;
    let stack = world.equipment_item(entity_id, EquipmentSlot::Head)?;
    item_runtime.custom_head_skull_for_stack(stack)
}

/// Vanilla `ItemTags.PIGLIN_LOVED` tag id — the items a piglin admires.
pub(crate) const PIGLIN_LOVED_ITEM_TAG: &str = "minecraft:piglin_loved";

/// Vanilla `Piglin.isChargingCrossbow()` (the synced `DATA_IS_CHARGING_CROSSBOW` boolean): the
/// piglin is drawing its crossbow, so `getArmPose` returns `CROSSBOW_CHARGE` rather than `CROSSBOW_HOLD`.
/// The generated accessor supplies regular-Piglin membership, wire index and typed default, excluding
/// PiglinBrute and unrelated metadata at the same slot.
pub(super) fn piglin_is_charging_crossbow(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    bbb_client_world::vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
        entity_type_id,
    )
    .map(|(data_id, default)| entity_data_bool(values, data_id, default))
    .unwrap_or_default()
}

/// Vanilla `Pillager.isChargingCrossbow()` (the synced `IS_CHARGING_CROSSBOW` boolean): the
/// pillager is drawing its crossbow, so `getArmPose` returns `CROSSBOW_CHARGE` rather than
/// `CROSSBOW_HOLD`. The generated accessor supplies Pillager membership, wire index and typed default,
/// excluding other illagers with unrelated metadata at the same slot.
pub(super) fn pillager_is_charging_crossbow(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    bbb_client_world::vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
        entity_type_id,
    )
    .map(|(data_id, default)| entity_data_bool(values, data_id, default))
    .unwrap_or_default()
}
