//! `EquipmentDispenseItemBehavior` and the chest behaviour: equipping the
//! first living entity standing in front of the dispenser.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::equipment_slots::{vanilla_equipment_slot_by_id, EquipmentSlot};
use bbb_protocol::game_types::GameType;
use bbb_server_types::geometry::Aabb;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed};
use crate::entity::general_living::is_abstract_horse_entity_type;
use crate::entity::horse_phenotype::{set_command_chest, HorsePhenotypeState};
use crate::entity::living::valid_equipment::{
    can_be_equipped_by, equippable, is_equippable_in_slot,
};
use crate::entity::name_tag::living_entity_is_alive;
use crate::entity::{Entity, EntityData, EntityRegistryTagView};
use crate::world::{LevelId, ServerWorld};

pub(super) fn has_equippable(stack: &RuntimeItemStack) -> bool {
    equippable(stack).is_some_and(|equippable| equippable.is_some())
}

pub(super) fn dispense(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    if dispense_equipment(world, source, &mut stack) {
        return Ok(Dispensed::new(stack, super::DISPENSE_SOUND));
    }
    world.default_dispense(source, stack)
}

/// `EquipmentDispenseItemBehavior.dispenseEquipment`.
pub(super) fn dispense_equipment(
    world: &mut ServerWorld,
    source: &BlockSource,
    stack: &mut RuntimeItemStack,
) -> bool {
    let tags = tag_view(world, &source.level);
    let Some(Some(equippable)) = equippable(stack) else {
        return false;
    };
    let Some(slot) = vanilla_equipment_slot_by_id(equippable.slot_id).map(|entry| entry.slot)
    else {
        return false;
    };
    if !equippable.dispensable {
        return false;
    }
    let area = block_area(source);
    let store = &world.level_by_id(&source.level).entities;
    let target = store.ids_in_box_sections(area).into_iter().find(|&id| {
        store.get(id).is_some_and(|entity| {
            entity.bounding_box().intersects(area)
                && can_equip_with_dispenser(entity, stack, slot, &tags)
        })
    });
    if let Some(id) = target {
        equip_entity(world, &source.level, id, slot, stack.split(1));
        return true;
    }
    if can_be_equipped_by(VANILLA_ENTITY_TYPE_PLAYER_ID, equippable, &tags) != Some(true) {
        return false;
    }
    equip_player(world, source, area, slot, stack)
}

fn equip_entity(
    world: &mut ServerWorld,
    level: &LevelId,
    id: i32,
    slot: EquipmentSlot,
    one: RuntimeItemStack,
) {
    let level = world.level_mut_by_id(level);
    let Some(entity) = level.entities.get_mut(id) else {
        return;
    };
    let Ok(effects) = crate::entity::item_slots::equip_dropped_when_killed(entity, slot, one)
    else {
        return;
    };
    let position = entity.position();
    if let Some(sound) = effects.sound {
        let seed = level.entities.random().next_long();
        level.entities.play_entity_sound_with_seed(sound, seed);
    }
    if let Some(event) = effects.equip_event {
        level
            .event_sink
            .game_event_with_source_at(event, position, Some(id), None);
    }
}

/// The player arm of `getEntitiesOfClass(LivingEntity.class, ...)`; a player
/// wears what lands in its inventory's armour slots.
fn equip_player(
    world: &mut ServerWorld,
    source: &BlockSource,
    area: Aabb,
    slot: EquipmentSlot,
    stack: &mut RuntimeItemStack,
) -> bool {
    let target = world
        .levels
        .player_entries()
        .find_map(|(&connection_id, player)| {
            if player.dimension != source.level
                || player.game_type == GameType::Spectator
                || player.damage.health <= 0.0
                || !player
                    .bounding_box()
                    .is_ok_and(|bounds| bounds.intersects(area))
            {
                return None;
            }
            let (_, index) = player
                .inventory
                .equipment_slots()
                .into_iter()
                .find(|&(candidate, _)| candidate == slot)?;
            player
                .inventory
                .slot(index)?
                .is_empty()
                .then_some((connection_id, index))
        });
    let Some((connection_id, index)) = target else {
        return false;
    };
    let one = stack.split(1);
    world.levels[connection_id]
        .inventory
        .set_inventory_slot(index, one.clone());
    world.player_equip_item_queued(connection_id, slot, &RuntimeItemStack::empty(), &one);
    true
}

/// `LivingEntity.canEquipWithDispenser`.
fn can_equip_with_dispenser(
    entity: &Entity,
    stack: &RuntimeItemStack,
    slot: EquipmentSlot,
    tags: &EntityRegistryTagView,
) -> bool {
    living_entity_is_alive(entity)
        && is_equippable_in_slot(entity, stack, slot, tags) == Some(true)
        && entity
            .living()
            .is_some_and(|living| !living.has_item_in_slot(slot))
        && can_dispenser_equip_into_slot(entity, slot)
}

/// `LivingEntity.canDispenserEquipIntoSlot` and its overrides.
fn can_dispenser_equip_into_slot(entity: &Entity, slot: EquipmentSlot) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return true;
    };
    let Some(mob) = data.mob.as_ref() else {
        return true;
    };
    let loot = mob.can_pick_up_loot();
    use bbb_protocol::entity_types::*;
    match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_ALLAY_ID => false,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => slot == EquipmentSlot::Body,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID
        | VANILLA_ENTITY_TYPE_PANDA_ID
        | VANILLA_ENTITY_TYPE_FOX_ID => slot == EquipmentSlot::MainHand && loot,
        VANILLA_ENTITY_TYPE_PIG_ID | VANILLA_ENTITY_TYPE_STRIDER_ID => {
            slot == EquipmentSlot::Saddle || loot
        }
        VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            matches!(slot, EquipmentSlot::Body | EquipmentSlot::Saddle) || loot
        }
        _ if is_abstract_horse_entity_type(entity.entity_type_id()) => {
            matches!(slot, EquipmentSlot::Body | EquipmentSlot::Saddle)
                && data
                    .abstract_horse_retained()
                    .is_some_and(|horse| horse.is_tame())
                || loot
        }
        _ => loot,
    }
}

/// The chest behaviour: a tamed chestless donkey, mule or llama takes it.
pub(super) fn chest(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let area = block_area(source);
    let store = &mut world.level_mut_by_id(&source.level).entities;
    let candidates = store.ids_in_box_sections(area);
    for id in candidates {
        let Some(entity) = store.get_mut(id) else {
            continue;
        };
        if !entity.bounding_box().intersects(area)
            || !living_entity_is_alive(entity)
            || !chestless_and_tamed(entity)
        {
            continue;
        }
        if set_command_chest(entity, &stack.with_count(1)) == Ok(true) {
            stack.shrink(1);
            return Ok(Dispensed::optional(stack, true));
        }
    }
    world.default_dispense(source, stack)
}

fn chestless_and_tamed(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    matches!(
        data.horse_phenotype(),
        Some(
            HorsePhenotypeState::Chested { has_chest: false }
                | HorsePhenotypeState::Llama {
                    has_chest: false,
                    ..
                }
        )
    ) && data
        .abstract_horse_retained()
        .is_some_and(|horse| horse.is_tame())
}

/// `new AABB(pos)` in front of the dispenser.
pub(super) fn block_area(source: &BlockSource) -> Aabb {
    let pos = source.target();
    Aabb {
        min_x: f64::from(pos.x),
        min_y: f64::from(pos.y),
        min_z: f64::from(pos.z),
        max_x: f64::from(pos.x) + 1.0,
        max_y: f64::from(pos.y) + 1.0,
        max_z: f64::from(pos.z) + 1.0,
    }
}

pub(super) fn tag_view(world: &ServerWorld, level: &LevelId) -> EntityRegistryTagView {
    world
        .level_by_id(level)
        .chunks
        .registry_tags_shared()
        .map_or(
            EntityRegistryTagView::Builtin26_1,
            EntityRegistryTagView::Frozen,
        )
}
