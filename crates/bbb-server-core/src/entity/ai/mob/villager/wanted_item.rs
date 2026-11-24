//! Villagers sense wanted ground stacks and approach them through the brain's navigation sink.

use crate::entity::ai::{
    Brain, MemoryModuleType, MemoryPositionTracker, MemoryValue, MemoryWalkTarget,
};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;

pub(in crate::entity::ai::mob) fn tick(
    brain: &mut Brain,
    body: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    let game_time = level.game_time();
    // `NearestItemSensor.doTick`: scan a 32 x 16 x 32 inflation, with a strict 32-block radius.
    if game_time.rem_euclid(20) == i64::from(body.id().rem_euclid(20)) {
        let area = body.bounding_box().inflate(32.0, 16.0, 32.0);
        let from = Vec3d {
            y: body.position().y + f64::from(body.eye_height()),
            ..body.position()
        };
        let mut nearest = None;
        let mut nearest_distance = 32.0 * 32.0;
        for id in store.ids_in_box_sections(area) {
            let Some(item) = store.get(id).filter(|item| item.is_alive()) else {
                continue;
            };
            let EntityData::Item(data) = item.data() else {
                continue;
            };
            let distance = distance_squared(body.position(), item.position());
            if distance >= nearest_distance
                || !item.bounding_box().intersects(area)
                || !crate::entity::villager_life::wants_to_pick_up(body, level, data.item())
            {
                continue;
            }
            let to = Vec3d {
                y: item.position().y + f64::from(item.eye_height()),
                ..item.position()
            };
            if crate::entity::projectile_hit::collision_clip_misses(level, from, to) {
                nearest = Some(MemoryValue::EntityRef(id));
                nearest_distance = distance;
            }
        }
        brain.set_memory(MemoryModuleType::NearestVisibleWantedItem, nearest);
    }
    // `GoToWantedItem.create` and `VillagerGoalPackages.getCorePackage`: preserve an ongoing walk.
    if brain.has_memory_value(MemoryModuleType::WalkTarget)
        || brain.has_memory_value(MemoryModuleType::ItemPickupCooldownTicks)
        || !matches!(body.data(), EntityData::GeneralLiving(data)
            if data.mob.as_ref().is_some_and(|mob| mob.can_pick_up_loot()))
    {
        return;
    }
    let Some(MemoryValue::EntityRef(id)) = brain.memory(MemoryModuleType::NearestVisibleWantedItem)
    else {
        return;
    };
    let Some(item) = store.get(*id).filter(|item| item.is_alive()) else {
        return;
    };
    let EntityData::Item(data) = item.data() else {
        return;
    };
    if distance_squared(body.position(), item.position()) >= 4.0 * 4.0
        || !crate::entity::villager_life::wants_to_pick_up(body, level, data.item())
        || !level
            .world_border()
            .is_some_and(|border| border.contains_block_at(item.block_position(), game_time))
    {
        return;
    }
    brain.set_memory(
        MemoryModuleType::LookTarget,
        Some(MemoryValue::EntityTracker(item.id())),
    );
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::Entity {
                entity_id: item.id(),
                last_seen: item.block_position(),
            },
            speed_modifier: super::SPEED_MODIFIER,
            close_enough_dist: 0,
        })),
    );
}

fn distance_squared(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)
}
