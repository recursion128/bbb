//!
//! A decision whose facts this server cannot answer leaves the ground stack
//! where it is.

use crate::world::simulation_level::LevelCtx;
mod item_facts;

use crate::connection::ClientboundPlayPacket;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ARMOR_ID, VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID,
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_ITEM_ENTITY_ITEM_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_HUSK_ID, VANILLA_ENTITY_TYPE_ILLUSIONER_ID, VANILLA_ENTITY_TYPE_PANDA_ID,
    VANILLA_ENTITY_TYPE_PARCHED_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID,
    VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_STRAY_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WITCH_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::equipment_slots::{vanilla_equipment_slot_by_id, EquipmentSlot};
use bbb_protocol::item_types::{
    VANILLA_ITEM_GLOW_INK_SAC_ID, VANILLA_ITEM_GOLDEN_AXE_ID, VANILLA_ITEM_NAUTILUS_SHELL_ID,
};
use bbb_protocol::packets::data_components::EquippableSummary;
use bbb_protocol::packets::{TakeItemEntity, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::raider::RaiderRaid;

use crate::entity::store::{PendingEntitySound, PendingThrownItemPickup};
use crate::entity::{Entity, EntityData, EntityStore, RemovalReason};
use item_facts::Read;

/// `Mob.ITEM_PICKUP_REACH` (`Mob.java:99`).
const ITEM_PICKUP_REACH: [f64; 3] = [1.0, 0.0, 1.0];

/// The `Mob` subclasses whose pickup overrides differ, by the class that
/// declares them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Mob,
    Allay,
    /// `Zombie` and the subclasses that inherit its two overrides.
    Zombie,
    Drowned,
    ZombifiedPiglin,
    /// `AbstractSkeleton` with the preferred weapon tag.
    Skeleton,
    WitherSkeleton,
    PiglinBrute,
    Piglin,
    Fox,
    Dolphin,
    Panda,
    Pillager,
    Raider,
    /// `Villager.pickUpItem` into its `InventoryCarrier` inventory.
    Villager,
}

impl Kind {
    fn of(entity: &Entity) -> Option<Self> {
        let EntityData::GeneralLiving(data) = entity.data() else {
            return None;
        };
        data.mob.as_ref()?;
        Some(match entity.entity_type_id() {
            VANILLA_ENTITY_TYPE_ALLAY_ID => Self::Allay,
            VANILLA_ENTITY_TYPE_PIGLIN_ID => Self::Piglin,
            VANILLA_ENTITY_TYPE_VILLAGER_ID => Self::Villager,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID
            | VANILLA_ENTITY_TYPE_HUSK_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => Self::Zombie,
            VANILLA_ENTITY_TYPE_DROWNED_ID => Self::Drowned,
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => Self::ZombifiedPiglin,
            VANILLA_ENTITY_TYPE_SKELETON_ID
            | VANILLA_ENTITY_TYPE_STRAY_ID
            | VANILLA_ENTITY_TYPE_BOGGED_ID
            | VANILLA_ENTITY_TYPE_PARCHED_ID => Self::Skeleton,
            VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => Self::WitherSkeleton,
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => Self::PiglinBrute,
            VANILLA_ENTITY_TYPE_FOX_ID => Self::Fox,
            VANILLA_ENTITY_TYPE_DOLPHIN_ID => Self::Dolphin,
            VANILLA_ENTITY_TYPE_PANDA_ID => Self::Panda,
            VANILLA_ENTITY_TYPE_PILLAGER_ID => Self::Pillager,
            VANILLA_ENTITY_TYPE_VINDICATOR_ID
            | VANILLA_ENTITY_TYPE_EVOKER_ID
            | VANILLA_ENTITY_TYPE_ILLUSIONER_ID
            | VANILLA_ENTITY_TYPE_WITCH_ID
            | VANILLA_ENTITY_TYPE_RAVAGER_ID => Self::Raider,
            _ => Self::Mob,
        })
    }

    /// `getPreferredWeaponType` (`AbstractSkeleton.java:187-189`,
    /// `WitherSkeleton.java:64-66`, `Drowned.java:297-299`,
    /// `Pillager.java:118-120`).
    const fn preferred_weapon_tag(self) -> Option<&'static str> {
        match self {
            Self::Skeleton => Some("minecraft:skeleton_preferred_weapons"),
            Self::Drowned => Some("minecraft:drowned_preferred_weapons"),
            Self::Pillager => Some("minecraft:pillager_preferred_weapons"),
            _ => None,
        }
    }
}

/// The looting section of `Mob.aiStep`. `false` refuses the whole tick: a
/// mutation that had already begun could not finish.
pub(crate) fn ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let Some(kind) = Kind::of(entity) else {
        return true;
    };
    let can_pick_up_loot = if kind == Kind::Allay {
        main_hand(entity).is_some() && !crate::entity::allay::pickup_on_cooldown(entity)
    } else {
        matches!(entity.data(), EntityData::GeneralLiving(data)
            if data.mob.as_ref().is_some_and(|mob| mob.can_pick_up_loot()))
    };
    let alive = entity.is_alive()
        && entity
            .living()
            .is_some_and(|living| living.health() > 0.0 && !living.dead);
    if !can_pick_up_loot || !alive || !level.mob_griefing() {
        return true;
    }
    // Allay.java:71,334-336.
    let [x, y, z] = if kind == Kind::Allay {
        [1.0, 1.0, 1.0]
    } else {
        ITEM_PICKUP_REACH
    };
    let area = entity.bounding_box().inflate(x, y, z);
    let candidates: Vec<i32> = store
        .ids_in_box_sections(area)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|other| {
                matches!(other.data(), EntityData::Item(_)) && other.bounding_box().intersects(area)
            })
        })
        .collect();
    for id in candidates {
        // `:466`: an item an earlier pickup of this walk removed is skipped.
        let Some(EntityData::Item(item)) = store.get(id).map(Entity::data) else {
            continue;
        };
        if item.item().is_empty() || item.has_pickup_delay() {
            continue;
        }
        let stack = item.item().clone();
        let thrower = item.thrower();
        match pick_up(kind, entity, store, level, id, &stack, thrower) {
            Ok(_) => {}
            Err(()) => return false,
        }
    }
    true
}

/// `wantsToPickUp` then `pickUpItem` for one ground stack. The outer error is
/// a mutation that could not finish; the inner one a decision refused before
/// any mutation.
fn pick_up(
    kind: Kind,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    item_id: i32,
    stack: &RuntimeItemStack,
    thrower: Option<Uuid>,
) -> Result<Read<()>, ()> {
    let target = Ground {
        id: item_id,
        stack,
        thrower,
    };
    match wants_to_pick_up(kind, entity, store, level, stack) {
        Ok(true) => {}
        Ok(false) => return Ok(Ok(())),
        Err(detail) => return Ok(Err(detail)),
    }
    match kind {
        Kind::Piglin => {
            let amount = if stack.item_resource_id() == Some("minecraft:gold_nugget") {
                stack.count()
            } else {
                1
            };
            let taken = stack.with_count(amount);
            crate::entity::piglin::pick_up_ground_item(entity, store, level, taken.clone())?;
            on_item_pickup(
                store,
                entity,
                Ground {
                    stack: &taken,
                    ..target
                },
            );
            take(store, entity, item_id, amount);
            shrink(store, item_id, stack, amount)?;
            Ok(Ok(()))
        }
        Kind::Fox => fox_pick_up(entity, store, level, target),
        Kind::Dolphin | Kind::Panda => hold_whole_stack(kind, entity, store, level, target),
        Kind::Pillager if !is_banner_item(stack) => Ok(Ok(())),
        Kind::Pillager | Kind::Raider => raider_pick_up(kind, entity, store, level, target),
        Kind::Villager | Kind::Allay => {
            inventory_pick_up(entity, store, level, target)?;
            Ok(Ok(()))
        }
        _ => mob_pick_up(kind, entity, store, level, target),
    }
}

#[derive(Clone, Copy)]
struct Ground<'a> {
    id: i32,
    stack: &'a RuntimeItemStack,
    thrower: Option<Uuid>,
}

fn wants_to_pick_up(
    kind: Kind,
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    stack: &RuntimeItemStack,
) -> Read<bool> {
    match kind {
        Kind::Allay => Ok(crate::entity::allay::wants_to_pick_up(
            entity, store, level, stack,
        )),
        Kind::Piglin => Ok(crate::entity::piglin::wants_ground_item(
            entity, stack, store, level,
        )),
        Kind::Zombie if item_facts::is_item(stack, VANILLA_ITEM_GLOW_INK_SAC_ID) => Ok(false),
        Kind::Drowned
            if item_facts::is_in_tag(level, stack, "minecraft:spears")
                || item_facts::is_item(stack, VANILLA_ITEM_GLOW_INK_SAC_ID) =>
        {
            Ok(false)
        }
        Kind::Skeleton | Kind::WitherSkeleton
            if item_facts::is_in_tag(level, stack, "minecraft:spears") =>
        {
            Ok(false)
        }
        Kind::PiglinBrute if !item_facts::is_item(stack, VANILLA_ITEM_GOLDEN_AXE_ID) => Ok(false),
        Kind::Villager => Ok(crate::entity::villager_life::wants_to_pick_up(
            entity, level, stack,
        )),
        _ => can_hold_item(kind, entity, store, level, stack),
    }
}

fn can_hold_item(
    kind: Kind,
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    stack: &RuntimeItemStack,
) -> Read<bool> {
    match kind {
        Kind::Zombie | Kind::Drowned | Kind::ZombifiedPiglin => {
            let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
            Ok(!(item_facts::is_in_tag(level, stack, "minecraft:eggs")
                && baby
                && store.vehicle(entity.id()).is_some()))
        }
        Kind::WitherSkeleton => Ok(!item_facts::is_in_tag(
            level,
            stack,
            "minecraft:wither_skeleton_disliked_weapons",
        )),
        Kind::Fox => {
            let Some(held) = main_hand(entity) else {
                return Ok(true);
            };
            let ticks_since_eaten =
                crate::entity::fox::ticks_since_eaten(entity).ok_or("fox state is missing")?;
            Ok(ticks_since_eaten > 0
                && crate::entity::item_components::is_consumable_food(stack)?
                && !crate::entity::item_components::is_consumable_food(held)?)
        }
        _ => Ok(true),
    }
}

fn mob_pick_up(
    kind: Kind,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    ground: Ground<'_>,
) -> Result<Read<()>, ()> {
    let plan = match plan_equip(kind, entity, store, level, ground.stack) {
        Ok(Some(plan)) => plan,
        Ok(None) => return Ok(Ok(())),
        Err(detail) => return Ok(Err(detail)),
    };
    let equipped = equip(entity, store, level, plan, ground.stack)?;
    on_item_pickup(store, entity, ground);
    take(store, entity, ground.id, equipped);
    shrink(store, ground.id, ground.stack, equipped)?;
    Ok(Ok(()))
}

struct EquipPlan {
    slot: EquipmentSlot,
    current: Option<bbb_server_types::item_stack::RuntimeItemStack>,
}

fn plan_equip(
    kind: Kind,
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    stack: &RuntimeItemStack,
) -> Read<Option<EquipPlan>> {
    let can_use_slot = |slot| {
        crate::entity::living::valid_equipment::can_use_slot(entity, slot)
            .ok_or_else(|| "canUseSlot is unreadable".to_string())
    };
    let equippable = item_facts::equippable(stack)?;
    let mut slot = equipment_slot_for_item(entity, equippable)?;
    let equippable_in_slot = match equippable {
        None => slot == EquipmentSlot::MainHand && can_use_slot(EquipmentSlot::MainHand)?,
        Some(equippable) => {
            slot == vanilla_equipment_slot_by_id(equippable.slot_id)
                .ok_or("equippable slot is unreadable")?
                .slot
                && can_use_slot(slot)?
                && crate::entity::living::valid_equipment::can_be_equipped_by(
                    entity.entity_type_id(),
                    equippable,
                    &level.capture_registry_tags(),
                )
                .ok_or("allowed entities are unreadable")?
        }
    };
    if !equippable_in_slot {
        return Ok(None);
    }
    let mut current = item_in(entity, slot).cloned();
    let mut can_replace =
        can_replace_current_item(kind, entity, store, level, stack, current.as_ref(), slot)?;
    if is_armor(slot) && !can_replace {
        slot = EquipmentSlot::MainHand;
        current = main_hand(entity).cloned();
        can_replace = current.is_none();
    }
    if can_replace && can_hold_item(kind, entity, store, level, stack)? {
        Ok(Some(EquipPlan {
            slot,
            current: entity
                .living()
                .and_then(|living| living.equipment().get(slot).cloned()),
        }))
    } else {
        Ok(None)
    }
}

/// The mutation half of `Mob.equipItemIfPossible`, answering the equipped
/// count.
fn equip(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    plan: EquipPlan,
    stack: &RuntimeItemStack,
) -> Result<i32, ()> {
    let EquipPlan { slot, current } = plan;
    let drop_chance = mob_state(entity).ok_or(())?.drop_chance(slot);
    if let Some(current) = current {
        let draw = store.random().next_float();
        if (draw - 0.1).max(0.0) < drop_chance {
            crate::entity::item::spawn_dropped_item_entity(store, entity.position(), current)
                .ok_or(())?;
        }
    }
    let count = crate::entity::item::stack_count(stack);
    let equipped = if count_limit(slot) > 0 {
        count.min(count_limit(slot))
    } else {
        count
    };
    let to_equip = stack.with_count(equipped);
    set_item_slot_and_drop_when_killed(entity, store, level, slot, to_equip)?;
    Ok(equipped)
}

fn set_item_slot_and_drop_when_killed(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    slot: EquipmentSlot,
    stack: RuntimeItemStack,
) -> Result<(), ()> {
    crate::entity::item_slots::set_living_item_in_tick(entity, store, level, slot, Some(stack))
        .map_err(|_| ())?;
    let mob = mob_state_mut(entity).ok_or(())?;
    mob.set_guaranteed_drop(slot);
    mob.set_persistence_required(true);
    Ok(())
}

// LivingEntity.java:3816-3818. Slot selection does not require the item to be equippable.
fn equipment_slot_for_item(
    entity: &Entity,
    equippable: Option<&EquippableSummary>,
) -> Read<EquipmentSlot> {
    if let Some(equippable) = equippable {
        let slot = vanilla_equipment_slot_by_id(equippable.slot_id)
            .ok_or("equippable slot is unreadable")?
            .slot;
        if crate::entity::living::valid_equipment::can_use_slot(entity, slot)
            .ok_or("canUseSlot is unreadable")?
        {
            return Ok(slot);
        }
    }
    Ok(EquipmentSlot::MainHand)
}

pub(crate) fn can_replace_piglin_item(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    stack: &RuntimeItemStack,
) -> bool {
    // Piglin.java:375-378.
    let result = item_facts::equippable(stack).and_then(|equippable| {
        let slot = equipment_slot_for_item(entity, equippable)?;
        can_replace_current_item(
            Kind::Piglin,
            entity,
            store,
            level,
            stack,
            item_in(entity, slot),
            slot,
        )
    });
    result.unwrap_or(false)
}

pub(crate) fn equip_piglin_item(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    stack: &RuntimeItemStack,
) -> bool {
    let Ok(Some(plan)) = plan_equip(Kind::Piglin, entity, store, level, stack) else {
        return false;
    };
    equip(entity, store, level, plan, stack).is_ok()
}

fn can_replace_current_item(
    kind: Kind,
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    new: &RuntimeItemStack,
    current: Option<&RuntimeItemStack>,
    slot: EquipmentSlot,
) -> Read<bool> {
    if kind == Kind::Drowned
        && current
            .is_some_and(|current| item_facts::is_item(current, VANILLA_ITEM_NAUTILUS_SHELL_ID))
    {
        return Ok(false);
    }
    let Some(current) = current else {
        return Ok(true);
    };
    // Piglin.java:382-393.
    if kind == Kind::Piglin {
        if item_facts::enchantment_ids(current, store)
            .iter()
            .any(|id| store.prevents_armor_change(id))
        {
            return Ok(false);
        }
        let adult = !matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
        let wanted = |stack: &RuntimeItemStack| {
            item_facts::is_in_tag(level, stack, "minecraft:piglin_loved")
                || (adult
                    && item_facts::is_in_tag(level, stack, "minecraft:piglin_preferred_weapons"))
        };
        if wanted(new) != wanted(current) {
            return Ok(wanted(new));
        }
    }
    if is_armor(slot) {
        compare_armor(entity, store, new, current, slot)
    } else if slot == EquipmentSlot::MainHand {
        compare_weapons(kind, entity, level, new, current, slot)
    } else {
        Ok(false)
    }
}

fn compare_armor(
    entity: &Entity,
    store: &EntityStore,
    new: &RuntimeItemStack,
    current: &RuntimeItemStack,
    slot: EquipmentSlot,
) -> Read<bool> {
    if item_facts::enchantment_ids(current, store)
        .iter()
        .any(|id| store.prevents_armor_change(id))
    {
        return Ok(false);
    }
    let new_defense = approximate_attribute(entity, new, VANILLA_ATTRIBUTE_ARMOR_ID, slot)?;
    let old_defense = approximate_attribute(entity, current, VANILLA_ATTRIBUTE_ARMOR_ID, slot)?;
    let new_toughness =
        approximate_attribute(entity, new, VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID, slot)?;
    let old_toughness =
        approximate_attribute(entity, current, VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID, slot)?;
    if new_defense != old_defense {
        Ok(new_defense > old_defense)
    } else if new_toughness != old_toughness {
        Ok(new_toughness > old_toughness)
    } else {
        can_replace_equal_item(new, current)
    }
}

fn compare_weapons(
    kind: Kind,
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    new: &RuntimeItemStack,
    current: &RuntimeItemStack,
    slot: EquipmentSlot,
) -> Read<bool> {
    if let Some(tag) = kind.preferred_weapon_tag() {
        let current_preferred = item_facts::is_in_tag(level, current, tag);
        let new_preferred = item_facts::is_in_tag(level, new, tag);
        if current_preferred && !new_preferred {
            return Ok(false);
        }
        if !current_preferred && new_preferred {
            return Ok(true);
        }
    }
    let new_damage = approximate_attribute(entity, new, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, slot)?;
    let old_damage =
        approximate_attribute(entity, current, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, slot)?;
    if new_damage != old_damage {
        Ok(new_damage > old_damage)
    } else {
        can_replace_equal_item(new, current)
    }
}

fn approximate_attribute(
    entity: &Entity,
    stack: &RuntimeItemStack,
    attribute_id: i32,
    slot: EquipmentSlot,
) -> Read<f64> {
    let attributes = entity.living().ok_or("body is not living")?.attributes();
    let base = if attributes.has_attribute(attribute_id) {
        attributes
            .base_value(attribute_id)
            .ok_or("attribute base is unreadable")?
    } else {
        0.0
    };
    let resource_id = bbb_protocol::attribute_types::vanilla_attribute_type(attribute_id)
        .ok_or("attribute is outside the registry")?
        .resource_id;
    item_facts::compute_attribute(stack, resource_id, base, slot)
}

fn can_replace_equal_item(new: &RuntimeItemStack, current: &RuntimeItemStack) -> Read<bool> {
    let current_enchantments = crate::entity::item_components::enchantment_count(current);
    let new_enchantments = crate::entity::item_components::enchantment_count(new);
    if new_enchantments != current_enchantments {
        return Ok(new_enchantments > current_enchantments);
    }
    let new_damage = item_facts::damage_value(new)?;
    let current_damage = item_facts::damage_value(current)?;
    if new_damage != current_damage {
        return Ok(new_damage < current_damage);
    }
    Ok(item_facts::has_custom_name(new)? && !item_facts::has_custom_name(current)?)
}

// Raider.java:428-431. This selected-item pickup also runs without area looting.
pub(crate) fn pick_up_raider_banner(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target: crate::entity::ai::mob::target::TargetRef,
) -> Option<()> {
    let Some(item) = store
        .get(target.entity_id)
        .filter(|item| item.uuid() == target.uuid && item.is_alive())
    else {
        return Some(());
    };
    let EntityData::Item(data) = item.data() else {
        return Some(());
    };
    if data.has_pickup_delay()
        || !crate::entity::general_living::spawn_finalize_equipment::is_ominous_banner(
            data.item(),
            level.item_stack_resolver(),
        )
    {
        return Some(());
    }
    let stack = data.item().clone();
    let ground = Ground {
        id: target.entity_id,
        stack: &stack,
        thrower: data.thrower(),
    };
    let Some(kind @ (Kind::Pillager | Kind::Raider)) = Kind::of(entity) else {
        return Some(());
    };
    raider_pick_up(kind, entity, store, level, ground)
        .ok()?
        .ok()?;
    Some(())
}

fn raider_pick_up(
    kind: Kind,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    ground: Ground<'_>,
) -> Result<Read<()>, ()> {
    let binding = match entity.data() {
        EntityData::GeneralLiving(data) => {
            data.raider_retained().and_then(|state| match state.raid() {
                RaiderRaid::Current(id) => Some((id, state.wave())),
                _ => None,
            })
        }
        _ => None,
    };
    // Raider.java:209-230. Pickup requires an absent leader, even if a recorded one died.
    if let Some((raid_id, wave)) = binding.filter(|(id, wave)| {
        level
            .raider_wave(*id, *wave)
            .is_some_and(|facts| facts.active && facts.leader.is_none())
            && crate::entity::general_living::spawn_finalize_equipment::is_ominous_banner(
                ground.stack,
                level.item_stack_resolver(),
            )
    }) {
        let slot = EquipmentSlot::Head;
        let drop_chance = mob_state(entity).ok_or(())?.drop_chance(slot);
        if let Some(current) = item_in(entity, slot).cloned() {
            if (store.random().next_float() - 0.1).max(0.0) < drop_chance {
                crate::entity::item::spawn_dropped_item_entity(store, entity.position(), current)
                    .ok_or(())?;
            }
        }
        on_item_pickup(store, entity, ground);
        crate::entity::item_slots::set_living_item_in_tick(
            entity,
            store,
            level,
            slot,
            Some(ground.stack.clone()),
        )
        .map_err(|_| ())?;
        take(store, entity, ground.id, ground.stack.count());
        store.remove(ground.id, RemovalReason::Discarded);
        // Raid.java:724-728. A captain drops its banner without becoming persistent.
        mob_state_mut(entity).ok_or(())?.set_guaranteed_drop(slot);
        crate::entity::raider::set_patrol_leader(entity);
        store.queue_command(crate::entity::EntityCommand::RaiderLeader(
            crate::entity::raider::PendingRaiderLeader {
                raider: crate::entity::ai::mob::target::TargetRef {
                    entity_id: entity.id(),
                    uuid: entity.uuid(),
                },
                raid_id,
                wave,
            },
        ));
        return Ok(Ok(()));
    }
    mob_pick_up(kind, entity, store, level, ground)
}

fn fox_pick_up(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    ground: Ground<'_>,
) -> Result<Read<()>, ()> {
    let count = crate::entity::item::stack_count(ground.stack);
    let position = entity.position();
    let held = entity
        .living()
        .and_then(|living| living.equipment().get(EquipmentSlot::MainHand).cloned());
    if count > 1 {
        let rest = ground.stack.with_count(count - 1);
        crate::entity::item::spawn_item_entity(store, position, rest).ok_or(())?;
    }
    if let Some(held) = held {
        let look = crate::entity::ai::mob::stroll::view_vector(entity.x_rot(), entity.y_rot());
        if !entity.is_silent() {
            store.play_entity_sound(PendingEntitySound {
                position,
                resource_id: "minecraft:entity.fox.spit".into(),
                source: crate::entity::living::living_sound_source(entity),
                volume: 1.0,
                pitch: 1.0,
            });
        }
        let at = Vec3d {
            x: position.x + look.x,
            y: position.y + 1.0,
            z: position.z + look.z,
        };
        crate::entity::item::spawn_thrown_item_entity(store, at, held, 40, entity.uuid())
            .ok_or(())?;
    }
    let one = ground.stack.with_count(1);
    on_item_pickup(
        store,
        entity,
        Ground {
            stack: &one,
            ..ground
        },
    );
    crate::entity::item_slots::set_living_item_in_tick(
        entity,
        store,
        level,
        EquipmentSlot::MainHand,
        Some(one),
    )
    .map_err(|_| ())?;
    mob_state_mut(entity)
        .ok_or(())?
        .set_guaranteed_drop(EquipmentSlot::MainHand);
    // Both splits have emptied the ground stack by now.
    take(store, entity, ground.id, 0);
    store.remove(ground.id, RemovalReason::Discarded);
    crate::entity::fox::reset_ticks_since_eaten(entity);
    Ok(Ok(()))
}

fn hold_whole_stack(
    kind: Kind,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    ground: Ground<'_>,
) -> Result<Read<()>, ()> {
    if main_hand(entity).is_some() {
        return Ok(Ok(()));
    }
    if kind == Kind::Panda
        && !item_facts::is_in_tag(level, ground.stack, "minecraft:panda_eats_from_ground")
    {
        return Ok(Ok(()));
    }
    on_item_pickup(store, entity, ground);
    let original = store
        .get(ground.id)
        .and_then(|body| match body.data() {
            EntityData::Item(item) => Some(item.item().clone()),
            _ => None,
        })
        .ok_or(())?;
    crate::entity::item_slots::set_living_item_in_tick(
        entity,
        store,
        level,
        EquipmentSlot::MainHand,
        Some(original),
    )
    .map_err(|_| ())?;
    mob_state_mut(entity)
        .ok_or(())?
        .set_guaranteed_drop(EquipmentSlot::MainHand);
    take(
        store,
        entity,
        ground.id,
        crate::entity::item::stack_count(ground.stack),
    );
    store.remove(ground.id, RemovalReason::Discarded);
    Ok(Ok(()))
}

/// `InventoryCarrier.pickUpItem` (`InventoryCarrier.java:16-35`) after
/// `wantsToPickUp`, which already asked for room.
fn inventory_pick_up(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    ground: Ground<'_>,
) -> Result<(), ()> {
    on_item_pickup(store, entity, ground);
    let count = crate::entity::item::stack_count(ground.stack);
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return Err(());
    };
    let rest = data
        .internal_inventory_mut()
        .ok_or(())?
        .add_item(level.item_stack_resolver(), ground.stack.clone());
    take(store, entity, ground.id, count - rest.count());
    shrink(store, ground.id, ground.stack, count - rest.count())
}

fn on_item_pickup(store: &mut EntityStore, entity: &Entity, ground: Ground<'_>) {
    if let Some(thrower) = ground.thrower {
        let thrower = store
            .get(ground.id)
            .and_then(|body| match body.data() {
                crate::entity::EntityData::Item(data) => data.thrower_reference(),
                _ => None,
            })
            .unwrap_or(crate::entity::entity_reference::EntityReference::Uuid(
                thrower,
            ));
        store.queue_command(crate::entity::EntityCommand::ThrownItemPickup(
            PendingThrownItemPickup {
                thrower,
                entity_id: entity.id(),
                item: ground.stack.clone(),
            },
        ));
    }
}

fn take(store: &mut EntityStore, entity: &Entity, item_id: i32, amount: i32) {
    store.broadcast_to_tracking(
        item_id,
        ClientboundPlayPacket::TakeItemEntity(TakeItemEntity {
            item_id,
            player_id: entity.id(),
            amount,
        }),
    );
}

fn shrink(
    store: &mut EntityStore,
    item_id: i32,
    stack: &RuntimeItemStack,
    amount: i32,
) -> Result<(), ()> {
    let remaining = crate::entity::item::stack_count(stack) - amount;
    let rest = stack.with_count(remaining);
    let item = store.get_mut(item_id).ok_or(())?;
    let EntityData::Item(data) = &mut item.data else {
        return Err(());
    };
    data.update_item_count(rest);
    item.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
    if remaining <= 0 {
        store.remove(item_id, RemovalReason::Discarded);
    }
    Ok(())
}

fn item_in(
    entity: &Entity,
    slot: EquipmentSlot,
) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
    entity
        .living()?
        .item_by_slot(slot)
        .filter(|stack| !stack.is_empty())
}

fn main_hand(entity: &Entity) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
    item_in(entity, EquipmentSlot::MainHand)
}

/// `EquipmentSlot.isArmor` (`EquipmentSlot.java:73-75`).
const fn is_armor(slot: EquipmentSlot) -> bool {
    matches!(
        slot,
        EquipmentSlot::Feet
            | EquipmentSlot::Legs
            | EquipmentSlot::Chest
            | EquipmentSlot::Head
            | EquipmentSlot::Body
    )
}

/// `EquipmentSlot.countLimit` (`EquipmentSlot.java:13-20`).
const fn count_limit(slot: EquipmentSlot) -> i32 {
    match slot {
        EquipmentSlot::MainHand | EquipmentSlot::OffHand => 0,
        _ => 1,
    }
}

/// `instanceof BannerItem`: the sixteen dyed banners (`Items.java`).
fn is_banner_item(stack: &RuntimeItemStack) -> bool {
    const COLORS: [&str; 16] = [
        "white",
        "orange",
        "magenta",
        "light_blue",
        "yellow",
        "lime",
        "pink",
        "gray",
        "light_gray",
        "cyan",
        "purple",
        "blue",
        "brown",
        "green",
        "red",
        "black",
    ];
    stack.item_resource_id().is_some_and(|item| {
        item.strip_prefix("minecraft:")
            .and_then(|path| path.strip_suffix("_banner"))
            .is_some_and(|color| COLORS.contains(&color))
    })
}

fn mob_state(entity: &Entity) -> Option<&crate::entity::general_living::MobRetainedState> {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.mob.as_ref(),
        _ => None,
    }
}

fn mob_state_mut(
    entity: &mut Entity,
) -> Option<&mut crate::entity::general_living::MobRetainedState> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => data.mob.as_mut(),
        _ => None,
    }
}
