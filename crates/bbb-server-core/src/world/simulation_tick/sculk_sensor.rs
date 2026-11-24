//! Live sculk sensor contacts, vibration ticks and activation effects.

use crate::world::simulation_level::LevelCtx;
use crate::world::LevelId;
use uuid::Uuid;

use super::vibration::WorldVibrationReads;
use super::*;

impl ServerWorld {
    pub(in crate::world) fn step_on_sculk_sensor(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        on_state: i32,
        connection_id: ConnectionId,
        _events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let source = crate::entity::level::SculkSensorStepSource {
            entity_id: player.entity_id(),
            uuid: player.profile.uuid,
            position: player.position(),
            is_warden: false,
            is_projectile: false,
            projectile_owner: None,
        };
        self.with_level(dimension, |level| {
            level.sculk_sensor_step(on_state, pos, source);
        });
    }

    pub(super) fn tick_sculk_sensor(
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
            .and_then(BlockEntity::sculk_sensor)
            .is_some_and(|sensor| sensor.vibration_data().has_pending_tick(game_time))
        {
            return;
        }
        // The listener body writes blocks and reads the level, so its own
        // object is moved out of the store for the length of the tick and put
        // back at the state the write left it in.
        let Some(mut block_entity) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .take(pos)
        else {
            return;
        };
        if block_entity.sculk_sensor().is_none() {
            self.put_back_block_entity(dimension, block_entity);
            return;
        }
        let activation = {
            let mut level = WorldVibrationReads {
                world: self,
                dimension,
                listener_pos: pos,
                events,
            };
            block_entity
                .sculk_sensor_mut()
                .and_then(|sensor| sensor.tick_vibration(pos, &mut level))
        };
        // Return the BE before `activate` notifies neighbor comparators (`SculkSensorBlockEntity.java:119-127`).
        self.put_back_block_entity(dimension, block_entity);
        if let Some((activation, frequency)) = activation {
            let mut level = WorldVibrationReads {
                world: self,
                dimension,
                listener_pos: pos,
                events,
            };
            crate::block_entity::sculk_sensor::SculkSensorBlockEntity::activate(
                pos, &mut level, activation, frequency,
            );
        }
    }
}

impl WorldVibrationReads<'_> {
    pub(crate) fn update_neighbours_at(&mut self, pos: BlockPos, block_type_id: i32) {
        self.world.with_level(self.dimension, |level| {
            LevelCtx::update_neighbors_at(level, pos, block_type_id);
        });
    }

    pub(crate) fn play_block_sound(
        &mut self,
        pos: BlockPos,
        sound_resource_id: &str,
        volume: f32,
        pitch: f32,
    ) {
        self.world
            .level_mut_by_id(self.dimension)
            .event_sink
            .play_block_sound_with_volume_pitch(pos, sound_resource_id, volume, pitch);
    }

    pub(crate) fn raise_game_event(
        &mut self,
        event: crate::level_event::VanillaGameEvent,
        pos: BlockPos,
        source_entity: Option<Uuid>,
        affected_state: Option<i32>,
    ) {
        let source_entity_id =
            source_entity.and_then(|uuid| entity_id_for_uuid(self.world, self.dimension, uuid));
        self.world
            .level_mut_by_id(self.dimension)
            .event_sink
            .game_event_with_source(event, pos, source_entity_id, affected_state);
    }
}

/// `Level.getEntity(uuid)`, as the id `GameEvent.Context.sourceEntity()`
/// carries: a session first, then the level's entity store.
fn entity_id_for_uuid(world: &ServerWorld, dimension: &LevelId, uuid: Uuid) -> Option<i32> {
    if let Some(player) = world
        .levels
        .players()
        .find(|state| state.profile.uuid == uuid)
    {
        return Some(player.entity_id());
    }
    world
        .level_by_id(dimension)
        .entities
        .entity_id_for_uuid(uuid)
}
