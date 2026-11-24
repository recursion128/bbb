//! `Villager.spawnGolemIfNeeded` (`Villager.java:834-848`): villagers that
//! slept lately and saw no golem call one when enough of them agree.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
};
use bbb_server_types::geometry::Aabb;

use super::block_entity_removal::SpawnStrategy;
use super::*;
use crate::entity::ai::{MemoryModuleType, MemoryValue};

/// `GolemSensor.MEMORY_TIME_TO_LIVE` (`GolemSensor.java:13`).
const GOLEM_DETECTED_TTL: i64 = 599;

impl ServerWorld {
    pub(in crate::world) fn villager_spawn_golem_if_needed(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        needed: usize,
    ) {
        let game_time = self.game_time();
        let entities = &self.level_by_id(dimension).entities;
        let wants = |id| crate::entity::villager::wants_to_spawn_golem(entities, id, game_time);
        if !wants(entity_id) {
            return;
        }
        let Some(villager) = entities.get(entity_id) else {
            return;
        };
        let start = villager.block_position();
        let area: Aabb = villager.bounding_box().inflate(10.0, 10.0, 10.0);
        let nearby: Vec<i32> = entities
            .ids_in_box_sections(area)
            .into_iter()
            .filter(|id| {
                entities.get(*id).is_some_and(|entity| {
                    entity.entity_type_id() == VANILLA_ENTITY_TYPE_VILLAGER_ID
                        && entity.bounding_box().intersects(area)
                })
            })
            .collect();
        if nearby.iter().filter(|id| wants(**id)).take(5).count() < needed {
            return;
        }
        let spawned = self.try_spawn_mob(
            dimension,
            crate::world::block_entity_removal::MobSpawnRequest {
                entity_type_id: VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                start,
                attempts: 10,
                range_xz: 8,
                range_y: 6,
                strategy: SpawnStrategy::LegacyIronGolem,
                check_collisions: false,
                reason: crate::entity::SpawnReason::MobSummoned,
            },
        );
        if spawned.is_none() {
            return;
        }
        // `GolemSensor.golemDetected` (`GolemSensor.java:43-45`).
        let entities = &mut self.level_mut_by_id(dimension).entities;
        for id in nearby {
            if let Some(runtime) = entities.mob_ai_mut(id) {
                runtime.set_brain_memory_with_expiry(
                    MemoryModuleType::GolemDetectedRecently,
                    MemoryValue::Bool(true),
                    GOLEM_DETECTED_TTL,
                );
            }
        }
    }
}
