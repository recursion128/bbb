//! `SpawnerBlock.serverTick` (`SpawnerBlock.java:35-37`) over the shared
//! `BaseSpawner` state (`BaseSpawner.java:82-198`).

use crate::block_events::BlockEventData;
use crate::entity::SpawnerTickDecision;
use crate::world::LevelId;
use bbb_protocol::block_types::VANILLA_BLOCK_SPAWNER_ID;

use super::*;

impl ServerWorld {
    pub(super) fn tick_spawner(&mut self, dimension: &LevelId, pos: BlockPos) {
        if !self.game_rules.spawner_blocks_work {
            return;
        }
        let Some(required_player_range) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::spawner)
            .map(|spawner| spawner.spawner_data().required_player_range())
        else {
            return;
        };
        let near_alive_player =
            self.has_nearby_alive_player_by_key(dimension, pos, f64::from(required_player_range));
        if !near_alive_player {
            return;
        }
        // `BaseSpawner.java:88-90`: decrement positive delay in place without cloning or `setChanged`.
        if self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::spawner_mut)
            .is_some_and(|spawner| spawner.spawner_data_mut().tick_countdown_in_place())
        {
            return;
        }
        let Some(mut data) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::spawner)
            .map(|spawner| spawner.spawner_data().clone())
        else {
            return;
        };
        let had_spawn_data = data.next_spawn_data().is_some();
        let random = self.random.clone();
        let mut decision = data.server_tick_with_registry(
            pos,
            near_alive_player,
            self.game_rules.spawner_blocks_work,
            &random,
            self.cold.config.plugins.content_registry(),
        );
        if matches!(decision, SpawnerTickDecision::Dormant) {
            return;
        }
        // `SpawnerBlockEntity.setNextSpawnData` (`SpawnerBlockEntity.java:28-33`) sends
        // the block entity whenever `delay` redraws or `getOrCreateNextSpawnData` fills it.
        let mut sync = false;
        loop {
            match decision {
                SpawnerTickDecision::Attempt(attempt) => {
                    let result = self.run_spawn_attempt(dimension, &attempt.request, &random);
                    decision = data.resolve_attempt_with_registry(
                        pos,
                        result,
                        &random,
                        self.cold.config.plugins.content_registry(),
                    );
                }
                SpawnerTickDecision::Countdown {
                    broadcast_event, ..
                }
                | SpawnerTickDecision::Finished {
                    broadcast_event, ..
                } => {
                    if let Some(event_id) = broadcast_event {
                        sync = true;
                        self.level_mut_by_id(dimension).block_events.block_event(
                            BlockEventData::new(
                                pos,
                                VANILLA_BLOCK_SPAWNER_ID,
                                i32::from(event_id),
                                0,
                            ),
                        );
                    }
                    break;
                }
                SpawnerTickDecision::Dormant => break,
            }
        }
        let data_present = data.next_spawn_data().is_some();
        let level = self.level_mut_by_id(dimension);
        if let Some(spawner) = level
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::spawner_mut)
        {
            *spawner.spawner_data_mut() = data;
        }
        level.chunks.block_entities_mut().set_changed(pos);
        if sync || (!had_spawn_data && data_present) {
            level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
        }
    }

    /// `EntityGetter.hasNearbyAlivePlayer` (`EntityGetter.java:100-111`) at the spawner's
    /// centre, which is what `BaseSpawner.isNearPlayer` asks
    /// (`BaseSpawner.java:59-61`).
    fn has_nearby_alive_player_by_key(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        radius: f64,
    ) -> bool {
        let target = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        };
        let radius_squared = radius * radius;
        self.levels.players().any(|player| {
            &player.dimension == dimension
                && !player.avatar_removed
                && player.game_type != GameType::Spectator
                && !player.damage.is_dead_or_dying()
                && {
                    let p = player.position();
                    (p.x - target.x).mul_add(
                        p.x - target.x,
                        (p.y - target.y)
                            .mul_add(p.y - target.y, (p.z - target.z) * (p.z - target.z)),
                    ) < radius_squared
                }
        })
    }
}
