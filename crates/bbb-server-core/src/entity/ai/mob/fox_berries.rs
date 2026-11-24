//! Fox navigation to berry crops and authoritative harvest.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::VANILLA_ITEM_SWEET_BERRIES_ID;
use bbb_protocol::packets::{BlockPos, ItemStackSummary};
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::block_behavior::{growing_plant, sweet_berry_bush, SET_BLOCK_UPDATE_CLIENTS};
use crate::entity::store::{EntityBodyReadView, PendingEntitySound};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::{LootEntity, StoredLootEntity};
use crate::world::simulation_level::LevelCtx;

pub(super) fn awake(body: &Entity) -> bool {
    matches!(body.data(), EntityData::GeneralLiving(data)
        if data.fox_retained().is_some_and(|fox| !fox.is_sleeping()))
}

pub(super) fn ripe(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let state = level.block_state(pos);
    level.has_ticking_column_at(pos)
        && ((sweet_berry_bush::is_sweet_berry_bush(state)
            && sweet_berry_bush::age(state).is_some_and(|age| age >= 2))
            || growing_plant::has_berries(state))
}

pub(in crate::entity::ai::mob) fn harvest(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_FOX_ID
        || !entity.is_alive_with_health()
        || !awake(entity)
        || !matches!(entity.data(), EntityData::GeneralLiving(data) if !data.is_no_ai())
        || !level.mob_griefing()
        || !ripe(level, pos)
        || entity
            .position()
            .distance_to_sqr(super::move_to_block::MoveToBlockGoal::block_point(pos, 1.5))
            >= 4.0
    {
        return;
    }
    let state = level.block_state(pos);
    if !sweet_berry_bush::is_sweet_berry_bush(state) {
        level.harvest_cave_vines(
            pos,
            entity.id(),
            LootEntity::Stored(StoredLootEntity {
                entity,
                bodies: Some(EntityBodyReadView::new(store, Some(entity), None)),
                raid_at_position: level.raid_at(entity.block_position()),
            }),
        );
        return;
    }
    let Some(age) = sweet_berry_bush::age(state) else {
        return;
    };
    let Some(next) = sweet_berry_bush::with_age(state, 1) else {
        return;
    };
    let mut count = 1 + level.level_random().next_int(2) + i32::from(age == 3);
    let stack = |count| ItemStackSummary {
        item_id: Some(VANILLA_ITEM_SWEET_BERRIES_ID),
        count,
        component_patch: Default::default(),
    };
    if entity.living().is_some_and(|living| {
        living
            .item_by_slot(EquipmentSlot::MainHand)
            .is_none_or(|item| item.is_empty())
    }) {
        if crate::entity::item_slots::set_living_item_in_tick(
            entity,
            store,
            level,
            EquipmentSlot::MainHand,
            Some(RuntimeItemStack::from_summary(stack(1))),
        )
        .is_err()
        {
            return;
        }
        count -= 1;
    }
    if count > 0 {
        level.pop_resource(pos, &stack(count));
    }
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:block.sweet_berry_bush.pick_berries".into(),
            source: crate::entity::living::living_sound_source(entity),
            volume: 1.0,
            pitch: 1.0,
        });
    }
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_CLIENTS,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    level.game_event_from_entity(VanillaGameEvent::BLOCK_CHANGE, pos, Some(entity.id()), None);
}
