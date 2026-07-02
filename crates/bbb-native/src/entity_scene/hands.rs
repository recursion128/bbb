use super::*;

/// Whether the entity's main-hand item is a bow (vanilla `SkeletonRenderState.isHoldingBow =
/// getMainHandItem().is(Items.BOW)`), driving the skeleton's `BOW_AND_ARROW` aim pose. Resolved through
/// the item registry, so it needs the runtime; `false` without it or for any non-bow / empty hand.
pub(super) fn entity_main_hand_holds_bow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
) -> bool {
    entity_main_hand_is_item(world, item_runtime, entity_id, "minecraft:bow")
}

/// Whether the entity's main-hand item resolves to a specific item resource id. Used for renderer
/// states whose vanilla extraction calls `getMainHandItem().is(Items.X)`.
pub(super) fn entity_main_hand_is_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    resource_id: &str,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, false) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some(resource_id)
}

/// Whether the item in the given hand is a trident (vanilla `Items.TRIDENT`). Drives the drowned's
/// `THROW_TRIDENT` raised-arm pose (`DrownedRenderer.getArmPose`'s `item.is(Items.TRIDENT)`, main hand) and
/// the player's use-item `THROW_TRIDENT` charge pose (`TridentItem.getUseAnimation() == TRIDENT`, either
/// hand). Resolved through the item registry, so it needs the runtime; `false` without it or for any
/// non-trident / empty hand.
pub(super) fn entity_hand_holds_trident(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:trident")
}

/// Whether the item in the given hand is a bow (vanilla `BowItem.getUseAnimation() == BOW`, which only
/// `minecraft:bow` returns). While the entity draws it, `HumanoidModel.poseRightArm`/`poseLeftArm`
/// `BOW_AND_ARROW` raises BOTH arms along the head look (the pose is two-handed + affectsOffhandPose, so the
/// opposite arm's pose is skipped). Resolved through the item registry; `false` without it or for any
/// non-bow / empty hand.
pub(super) fn entity_hand_holds_bow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:bow")
}

/// Whether the item in the given hand is one of the tool-material spears. This is the item/tag side of
/// vanilla spear presentation: held spear poses and kinetic use transforms come from the resolved item
/// prototype. Per-stack `DataComponents.SWING_ANIMATION` overrides are handled by
/// [`entity_hand_swing_is_stab`].
pub(super) fn entity_hand_holds_spear(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    entity_hand_spear_kinetic_weapon(world, item_runtime, entity_id, off_hand).is_some()
}

pub(super) fn entity_hand_spear_kinetic_weapon(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> Option<SpearKineticWeapon> {
    let Some(item_runtime) = item_runtime else {
        return None;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return None;
    };
    let Some(item_id) = stack.item_id else {
        return None;
    };
    spear_kinetic_weapon_for_resource_id(item_runtime.item_resource_id(item_id)?)
}

pub(super) fn spear_kinetic_weapon_for_resource_id(
    resource_id: &str,
) -> Option<SpearKineticWeapon> {
    let (delay, dismount, knockback, damage) = match resource_id {
        // Vanilla `Items.*_SPEAR` -> `Item.Properties.spear(... delay, dismountTime,
        // knockbackTime, damageTime ...)`, each seconds value multiplied by 20 for
        // `KineticWeapon` ticks.
        "minecraft:wooden_spear" => (15.0, 100.0, 200.0, 300.0),
        "minecraft:stone_spear" => (14.0, 90.0, 180.0, 275.0),
        "minecraft:copper_spear" => (13.0, 80.0, 165.0, 250.0),
        "minecraft:iron_spear" => (12.0, 50.0, 135.0, 225.0),
        "minecraft:golden_spear" => (14.0, 70.0, 170.0, 275.0),
        "minecraft:diamond_spear" => (10.0, 60.0, 130.0, 200.0),
        "minecraft:netherite_spear" => (8.0, 50.0, 110.0, 175.0),
        _ => return None,
    };
    Some(SpearKineticWeapon {
        delay_ticks: delay,
        dismount_duration_ticks: dismount,
        knockback_duration_ticks: knockback,
        damage_duration_ticks: damage,
        forward_movement: 0.38,
    })
}

/// Whether the item in the given hand is a spyglass (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.SPYGLASS`, which only `minecraft:spyglass` returns). While the entity is using it,
/// `HumanoidModel.poseRightArm`/`poseLeftArm` raise that arm to hold the spyglass to the eye. Resolved
/// through the item registry; `false` without it or for any other / empty hand.
pub(super) fn entity_hand_holds_spyglass(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:spyglass")
}

/// Whether the item in the given hand is a goat horn (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.TOOT_HORN`, which only `InstrumentItem` / `minecraft:goat_horn` returns). While the
/// entity is tooting it, `HumanoidModel.poseRightArm`/`poseLeftArm` raise that arm to the mouth. Resolved
/// through the item registry; `false` without it or for any other / empty hand.
pub(super) fn entity_hand_holds_goat_horn(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:goat_horn")
}

/// Whether the item in the given hand is a brush (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.BRUSH`, which only `BrushItem` / `minecraft:brush` returns). While the entity is
/// brushing, `HumanoidModel.poseRightArm`/`poseLeftArm` lower that arm to the brushed block. Resolved
/// through the item registry; `false` without it or for any other / empty hand.
pub(super) fn entity_hand_holds_brush(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:brush")
}

/// Vanilla `DataComponents.CONSUMABLE` network type id (24, the registry order of `minecraft:consumable` in
/// `DataComponents`). `Item.getUseAnimation` checks `CONSUMABLE` before `BLOCKS_ATTACKS`, so a stack patch
/// that adds both should stay on EAT/DRINK rather than the block pose.
pub(super) const DATA_COMPONENT_CONSUMABLE_TYPE_ID: i32 = 24;

/// Vanilla `DataComponents.SWING_ANIMATION` network type id (40, the registry order of
/// `minecraft:swing_animation` in `DataComponents`).
pub(super) const DATA_COMPONENT_SWING_ANIMATION_TYPE_ID: i32 = 40;

/// Vanilla `DataComponents.BLOCKS_ATTACKS` network type id (37, the registry order of
/// `minecraft:blocks_attacks` in `DataComponents`). `Item.getUseAnimation` returns `BLOCK` for a non-consumable
/// item carrying this component.
pub(super) const DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID: i32 = 37;

pub(super) fn component_patch_has_added_component(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    type_id: i32,
) -> bool {
    patch.added_type_ids.contains(&type_id) && !patch.removed_type_ids.contains(&type_id)
}

pub(super) fn entity_hand_swing_is_stab(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    if stack
        .component_patch
        .removed_type_ids
        .contains(&DATA_COMPONENT_SWING_ANIMATION_TYPE_ID)
    {
        return false;
    }
    if let Some(swing_animation) = stack.component_patch.swing_animation {
        return swing_animation.animation_type == SwingAnimationTypeSummary::Stab;
    }
    entity_hand_holds_spear(world, item_runtime, entity_id, off_hand)
}

/// Whether the item in the given hand has the `BLOCK` use-animation (vanilla `ItemStack.getUseAnimation() ==
/// ItemUseAnimation.BLOCK`, returned by `Item.getUseAnimation` for a non-consumable item carrying
/// `DataComponents.BLOCKS_ATTACKS`). While the entity raises it, `HumanoidModel.poseRightArm`/`poseLeftArm`
/// `poseBlockingArm` tucks that arm's blocking item forward. The vanilla shield is detected by resolved item
/// id because its component is a prototype default; datapack/patch-granted `blocks_attacks` is detected from
/// `added_type_ids` and does not need the item registry.
pub(super) fn entity_hand_blocks_attacks(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    if component_patch_has_added_component(
        &stack.component_patch,
        DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    ) {
        return false;
    }
    if component_patch_has_added_component(
        &stack.component_patch,
        DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    ) {
        return true;
    }
    if stack
        .component_patch
        .removed_type_ids
        .contains(&DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
    {
        return false;
    }
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:shield")
}

/// Whether the item in the given hand is a crossbow (vanilla `Pillager.isHolding(Items.CROSSBOW)` for the
/// pillager's `CROSSBOW_HOLD`/`CROSSBOW_CHARGE`; also the player's crossbow use poses). Resolved through the
/// item registry, so it needs the runtime; `false` without it or for any non-crossbow / empty hand.
pub(super) fn entity_hand_holds_crossbow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:crossbow")
}

pub(super) fn entity_holds_crossbow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
) -> bool {
    entity_hand_holds_crossbow(world, item_runtime, entity_id, false)
        || entity_hand_holds_crossbow(world, item_runtime, entity_id, true)
}

/// Whether the item in the given hand routes to a SPECIAL `getArmPose` (not the `ITEM` fallback) when used:
/// the use-animation poses `BOW_AND_ARROW` (bow), `CROSSBOW_CHARGE` (crossbow), `THROW_TRIDENT` (trident),
/// `BLOCK` (non-consumable `BLOCKS_ATTACKS` item, normally the shield), `SPYGLASS`, `TOOT_HORN` (goat horn),
/// `BRUSH`, and `SPEAR`. While the entity uses one of these in this hand, that hand gets its dedicated pose
/// instead of `ITEM`; any OTHER used item (food/potion -> `EAT`/`DRINK`, or any plain item) falls through to
/// `ITEM`.
pub(super) fn entity_hand_holds_special_use_item(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    entity_hand_holds_bow(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_crossbow(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_trident(world, item_runtime, entity_id, off_hand)
        || entity_hand_blocks_attacks(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_spyglass(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_goat_horn(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_brush(world, item_runtime, entity_id, off_hand)
        || entity_hand_holds_spear(world, item_runtime, entity_id, off_hand)
}

/// Whether the item in the given hand is a *charged* crossbow (vanilla
/// `isHolding(Items.CROSSBOW) && CrossbowItem.isCharged(getWeaponItem())`), driving the piglin's
/// `CROSSBOW_HOLD` arm pose. `CrossbowItem.isCharged` is the held crossbow's `minecraft:charged_projectiles`
/// component being non-empty (decoded into the held stack's component patch). Resolved through the item
/// registry; `false` without it or for any non-crossbow / empty / un-charged hand.
pub(super) fn entity_hand_holds_charged_crossbow(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    entity_id: i32,
    off_hand: bool,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let Some(stack) = world.held_item(entity_id, off_hand) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime.item_resource_id(item_id) == Some("minecraft:crossbow")
        && !stack.component_patch.charged_projectiles_items.is_empty()
}

/// Vanilla `DataComponents.TOOL` network type id (28, the registry order of `minecraft:tool` in
/// `DataComponents`). `AbstractPiglin.isHoldingMeleeWeapon()` is `getMainHandItem().has(DataComponents.TOOL)`,
/// so a main-hand stack counts as a melee weapon when its decoded component patch added this type.
pub(super) const DATA_COMPONENT_TOOL_TYPE_ID: i32 = 28;

/// Whether the entity's main-hand item is a melee weapon (vanilla
/// `AbstractPiglin.isHoldingMeleeWeapon()` = `getMainHandItem().has(DataComponents.TOOL)`), driving the
/// piglin/brute `ATTACKING_WITH_MELEE_WEAPON` arm pose when aggressive. The decoded component patch records
/// every added component's type id, so the `minecraft:tool` component shows up as
/// [`DATA_COMPONENT_TOOL_TYPE_ID`] in `added_type_ids` — no item-registry lookup is needed (unlike the
/// crossbow/bow checks, which resolve the item id). `false` for an empty hand or a non-tool main-hand item.
pub(super) fn entity_main_hand_holds_melee_weapon(world: &WorldStore, entity_id: i32) -> bool {
    let Some(stack) = world.held_item(entity_id, false) else {
        return false;
    };
    stack
        .component_patch
        .added_type_ids
        .contains(&DATA_COMPONENT_TOOL_TYPE_ID)
}

/// Whether the entity's main hand holds any item at all. Vanilla `AvatarRenderer.getArmPose` falls back to
/// the `ITEM` arm pose for a non-empty main hand that is not a spear / charged crossbow / item-in-use; this is
/// the "is the main hand non-empty" half of that fallback. Resolved from the held-item summary only (no item
/// registry needed), so it works without the runtime; `false` for an empty hand.
pub(super) fn entity_main_hand_non_empty(world: &WorldStore, entity_id: i32) -> bool {
    world
        .held_item(entity_id, false)
        .is_some_and(|stack| item_stack_non_empty(&stack))
}

/// Whether the entity's OFF hand holds any item at all. Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)`
/// likewise falls back to the `ITEM` arm pose for a non-empty off hand that is not a charged crossbow /
/// item-in-use; this is the "is the off hand non-empty" half of that fallback. Resolved from the held-item
/// summary only (no item registry needed); `false` for an empty off hand.
pub(super) fn entity_offhand_non_empty(world: &WorldStore, entity_id: i32) -> bool {
    world
        .held_item(entity_id, true)
        .is_some_and(|stack| item_stack_non_empty(&stack))
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
    item_runtime.custom_head_skull_for_stack(&stack)
}

/// Vanilla `ItemTags.PIGLIN_LOVED` tag id — the items a piglin admires.
pub(super) const PIGLIN_LOVED_ITEM_TAG: &str = "minecraft:piglin_loved";

/// Whether the entity's OFFHAND item is a piglin-loved item (vanilla
/// `PiglinAi.isLovedItem(getOffhandItem())` = `getOffhandItem().is(ItemTags.PIGLIN_LOVED)`), driving the
/// regular piglin's `ADMIRING_ITEM` arm pose. Item tags arrive over the network (`UpdateTags`) into the
/// `minecraft:item` registry tag set, so membership is the offhand item's protocol id appearing in the
/// `minecraft:piglin_loved` tag — no item-registry lookup needed. `false` for an empty offhand, an unknown
/// id, or when the tag set hasn't been received.
pub(super) fn entity_offhand_holds_loved_item(world: &WorldStore, entity_id: i32) -> bool {
    let Some(stack) = world.held_item(entity_id, true) else {
        return false;
    };
    let Some(item_id) = stack.item_id else {
        return false;
    };
    world
        .registry_tags("minecraft:item")
        .and_then(|registry| registry.tags.get(PIGLIN_LOVED_ITEM_TAG))
        .is_some_and(|entries| entries.contains(&item_id))
}

/// Vanilla `Piglin.isChargingCrossbow()` (the synced `DATA_IS_CHARGING_CROSSBOW` boolean, id 18): the
/// piglin is drawing its crossbow, so `getArmPose` returns `CROSSBOW_CHARGE` rather than `CROSSBOW_HOLD`.
/// Only the regular piglin defines that accessor, so the projection is gated to its type.
pub(super) fn piglin_is_charging_crossbow(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PIGLIN_ID
        && entity_data_bool(values, PIGLIN_IS_CHARGING_CROSSBOW_DATA_ID, false)
}

/// Vanilla `Pillager.isChargingCrossbow()` (the synced `IS_CHARGING_CROSSBOW` boolean, id 17): the
/// pillager is drawing its crossbow, so `getArmPose` returns `CROSSBOW_CHARGE` rather than
/// `CROSSBOW_HOLD`. Only the pillager defines that accessor, so the projection is gated to its type.
pub(super) fn pillager_is_charging_crossbow(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    entity_type_id == VANILLA_ENTITY_TYPE_PILLAGER_ID
        && entity_data_bool(values, PILLAGER_IS_CHARGING_CROSSBOW_DATA_ID, false)
}
