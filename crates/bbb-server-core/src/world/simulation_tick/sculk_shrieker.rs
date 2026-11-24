//! Shrieker vibration delivery and scheduled response.

use crate::block_entity::sculk_shrieker::{state_with_shrieking, SculkShriekerBlockEntity};
use crate::block_entity::BlockEntity;
#[cfg(test)]
use crate::entity::warden_spawn_tracker::WardenSpawnTracker;
use crate::world::LevelId;
use bbb_protocol::packets::BlockPos;
use uuid::Uuid;

use super::vibration::WorldVibrationReads;
use super::*;

/// `Block.UPDATE_ALL` (`Block.java:104`).
const SET_BLOCK_UPDATE_ALL: i32 = 3;

impl ServerWorld {
    pub(crate) fn tick_warden_warning_trackers(&mut self) {
        for player in self.levels.players_mut() {
            player.cold.warden_tracker.tick();
        }
    }

    pub(super) fn tick_sculk_shrieker(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let game_time = self.game_time();
        if !self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::sculk_shrieker)
            .is_some_and(|shrieker| shrieker.vibration_data().has_pending_tick(game_time))
        {
            return;
        }
        let Some(mut block_entity) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .take(pos)
        else {
            return;
        };
        let pending = if let Some(shrieker) = block_entity.sculk_shrieker_mut() {
            shrieker.tick_vibration(
                pos,
                &mut WorldVibrationReads {
                    world: self,
                    dimension,
                    listener_pos: pos,
                    events,
                },
            )
        } else {
            None
        };
        self.put_back_block_entity(dimension, block_entity);
        if let Some(player) = pending {
            self.shriek_with_context(dimension, pos, player);
        }
    }

    pub(in crate::world) fn step_on_sculk_shrieker(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        player: Uuid,
        _events: &mut Vec<WorldEvent>,
    ) {
        if self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::sculk_shrieker)
            .is_none()
        {
            return;
        }
        self.shriek_with_context(dimension, pos, player);
    }

    fn shriek_with_context(&mut self, dimension: &LevelId, pos: BlockPos, player: Uuid) {
        let source_entity_id = self
            .levels
            .players()
            .find(|session| session.profile.uuid == player)
            .map(|session| session.entity_id());
        self.with_level(dimension, |level| {
            let warden_nearby = level.sculk_shrieker_warden_nearby(pos);
            SculkShriekerBlockEntity::try_shriek_in_place(
                pos,
                level,
                player,
                source_entity_id,
                warden_nearby,
            );
        });
    }

    pub(in crate::world) fn put_back_block_entity(
        &mut self,
        dimension: &LevelId,
        block_entity: BlockEntity,
    ) {
        self.level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .put_back(block_entity);
    }

    pub(crate) fn tick_sculk_shrieker_scheduled(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        state: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(next) = state_with_shrieking(state, false) else {
            return;
        };
        if next == state {
            return;
        }
        self.set_block(dimension, pos, next, SET_BLOCK_UPDATE_ALL);
        let Some((warning_level, can_summon)) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::sculk_shrieker)
            .map(|shrieker| (shrieker.warning_level(), sculk_shrieker_can_summon(state)))
        else {
            return;
        };
        self.sculk_shrieker_try_respond(dimension, pos, warning_level, can_summon, events);
    }
}

fn sculk_shrieker_can_summon(state: i32) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
        .and_then(|identity| identity.properties.get("can_summon").cloned())
        .as_deref()
        == Some("true")
}

#[cfg(test)]
impl ServerWorld {
    /// The tracker of the online player `uuid`.
    pub(in crate::world) fn warden_spawn_tracker(&self, uuid: Uuid) -> Option<WardenSpawnTracker> {
        self.levels
            .players()
            .find(|player| player.profile.uuid == uuid)
            .map(|player| player.cold.warden_tracker)
    }

    fn set_warden_spawn_tracker(&mut self, uuid: Uuid, tracker: WardenSpawnTracker) {
        if let Some(player) = self
            .levels
            .players_mut()
            .find(|player| player.profile.uuid == uuid)
        {
            player.cold.warden_tracker = tracker;
        }
    }

    pub(crate) fn step_on_sculk_shrieker_for_test(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        player: Uuid,
        events: &mut Vec<WorldEvent>,
    ) {
        self.step_on_sculk_shrieker(dimension, pos, player, events);
    }

    pub(crate) fn warden_spawn_tracker_for_test(&self, player: Uuid) -> Option<WardenSpawnTracker> {
        self.warden_spawn_tracker(player)
            .or_else(|| self.cold.restored_warden_trackers.get(&player).copied())
    }

    /// Replaces one profile's tracker; `None` is a fresh one.
    pub(crate) fn set_warden_spawn_tracker_for_test(
        &mut self,
        player: Uuid,
        tracker: Option<WardenSpawnTracker>,
    ) {
        if self.warden_spawn_tracker(player).is_some() {
            self.set_warden_spawn_tracker(player, tracker.unwrap_or_default());
        } else if let Some(tracker) = tracker {
            self.cold.restored_warden_trackers.insert(player, tracker);
        } else {
            self.cold.restored_warden_trackers.remove(&player);
        }
    }
}
