//! The world half of the entity game-event listeners: the
//! `ServerLevel.EntityCallbacks` edges that add, move and remove them
//! (`ServerLevel.java:1942`, `:1970`, `:1975`), their position source during a
//! walk, and the delivery to their listener bodies.

use crate::block_entity::game_event::{
    containing_block_pos, EntityGameEventListenerKind, GameEventContext, GameEventListenerEntry,
};
use crate::block_entity::vibration::listener_handle_game_event;
use crate::entity::game_event_listener::{
    allay_jukebox_handle_game_event, can_receive_vibration, entity_game_event_listeners,
    entity_listener_position, put_back_vibration_listener, take_vibration_listener,
    VibrationReceiveFacts,
};
use crate::entity::{Entity, EntityLifecycleEvent};
use crate::level_event::VanillaGameEvent;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use super::vibration::WorldVibrationReads;
use super::{ServerWorld, WorldEvent};
use bbb_protocol::game_types::GameType;

impl ServerWorld {
    /// Applies the add/move/remove edges the level's entity store journaled
    /// since the last sync to the listener registry, in journal order:
    /// `DynamicGameEventListener::add` on tracking start, `::move` on a section
    /// change and `::remove` on tracking end.
    ///
    /// Vanilla runs each callback inside the store mutation; here the
    /// registry lives with the block entities, so the edges are applied before
    /// the next post and before the tracker drains the journal. A move whose
    /// body is lifted out for its own tick stops the sync there, so the move
    /// is applied once the body is back.
    pub(in crate::world) fn sync_entity_game_event_listeners(&mut self, dimension: &LevelId) {
        let Some(level) = self.levels.get_mut(dimension) else {
            return;
        };
        let entities = &mut level.entities;
        let chunks = &mut level.chunks;
        let mut applied = 0;
        for edge in entities.lifecycle_events_since_listener_sync() {
            // `None` position: `DynamicGameEventListener.remove`.
            let (entity, position) = match edge {
                EntityLifecycleEvent::Added {
                    entity,
                    tracked: true,
                }
                | EntityLifecycleEvent::TrackingStarted { entity } => {
                    (entity, Some(entity_listener_position(entity)))
                }
                EntityLifecycleEvent::Moved { entity_id, .. } => match entities.get(*entity_id) {
                    Some(entity) => (entity, Some(entity_listener_position(entity))),
                    None if entities.is_detached(*entity_id) => break,
                    None => {
                        applied += 1;
                        continue;
                    }
                },
                EntityLifecycleEvent::TrackingStopped { entity }
                | EntityLifecycleEvent::Removed {
                    entity,
                    tracked: true,
                } => (entity, None),
                _ => {
                    applied += 1;
                    continue;
                }
            };
            for spec in entity_game_event_listeners(entity.entity_type_id()) {
                apply_listener_edge(chunks, entity.id(), spec.kind, spec.radius, position);
            }
            applied += 1;
        }
        entities.advance_listener_sync(applied);
    }

    /// `EntityPositionSource.getPosition` (`EntityPositionSource.java:47-53`)
    /// for a registered entity listener: empty when the entity is not in the
    /// store.
    pub(super) fn entity_listener_position(
        &self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> Option<Vec3d> {
        self.level_by_id(dimension)
            .entities
            .get(entity_id)
            .map(entity_listener_position)
    }

    /// `GameEventListener.handleGameEvent` of an entity listener
    /// (`GameEventDispatcher.java:36`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn deliver_entity_game_event(
        &mut self,
        dimension: &LevelId,
        listener: GameEventListenerEntry,
        entity_id: i32,
        kind: EntityGameEventListenerKind,
        game_event: VanillaGameEvent,
        context: GameEventContext,
        position: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        match kind {
            EntityGameEventListenerKind::AllayJukebox => {
                allay_jukebox_handle_game_event(
                    &mut self.level_mut_by_id(dimension).entities,
                    entity_id,
                    game_event,
                    position,
                );
            }
            EntityGameEventListenerKind::Vibration => {
                let Some(can_receive) = self.entity_can_receive_vibration(
                    dimension,
                    entity_id,
                    containing_block_pos(position),
                    context,
                ) else {
                    return;
                };
                let Some(mut vibration) = self
                    .level_mut_by_id(dimension)
                    .entities
                    .get_mut(entity_id)
                    .and_then(|entity| take_vibration_listener(entity, can_receive))
                else {
                    return;
                };
                let mut level = WorldVibrationReads {
                    world: self,
                    dimension,
                    listener_pos: listener.pos,
                    events,
                };
                listener_handle_game_event(
                    &mut vibration,
                    &mut level,
                    game_event,
                    context,
                    position,
                );
                if let Some(entity) = self.level_mut_by_id(dimension).entities.get_mut(entity_id) {
                    put_back_vibration_listener(entity, vibration);
                }
            }
        }
    }

    /// `VibrationSystem.User.canReceiveVibration` (`VibrationSystem.java:382`)
    /// of an entity user, with the level half answered here.
    fn entity_can_receive_vibration(
        &self,
        dimension: &LevelId,
        entity_id: i32,
        pos: BlockPos,
        context: GameEventContext,
    ) -> Option<bool> {
        let level = self.level_by_id(dimension);
        let entity = level.entities.get(entity_id)?;
        let living_source_targetable = context
            .source_entity_id
            .and_then(|source| self.warden_can_target(dimension, entity, source));
        Some(can_receive_vibration(
            entity,
            pos,
            VibrationReceiveFacts {
                dimension: self.level_key(*dimension).as_str(),
                pos_within_border: level.world_border.contains_block_at(pos, self.game_time()),
                living_source_targetable,
            },
        ))
    }

    /// `Warden.canTargetEntity` (`Warden.java:388-399`) for a source entity
    /// that is a `LivingEntity`; `None` for any other source. The alliance is
    /// `Entity.isAlliedTo`'s team half (`Entity.java:2696-2706`): no
    /// `considersEntityAsAlly` override in 26.1 names a warden.
    pub(in crate::world) fn warden_can_target(
        &self,
        dimension: &LevelId,
        warden: &Entity,
        source_entity_id: i32,
    ) -> Option<bool> {
        let teams = &self.scoreboard.board().teams;
        let warden_team = teams.entity_team(warden.uuid());
        let border = self.level_by_id(dimension).world_border;
        let game_time = self.game_time();
        let player = self
            .levels
            .players()
            .find(|player| player.entity_id() == source_entity_id);
        if let Some(player) = player {
            let facts = Self::nearby_player_facts(player);
            // `EntitySelector.NO_CREATIVE_OR_SPECTATOR` (`EntitySelector.java:14-16`).
            let creative_or_spectator =
                matches!(player.game_type, GameType::Creative | GameType::Spectator);
            return Some(crate::entity::warden::can_target(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                &player.dimension == dimension,
                creative_or_spectator,
                crate::entity::ScoreboardTeams::allied(
                    warden_team,
                    teams.team_of(&player.profile.name),
                ),
                facts.invulnerable,
                facts.dead_or_dying,
                border,
                game_time,
                facts.bounding_box(),
            ));
        }
        let source = self.level_by_id(dimension).entities.get(source_entity_id)?;
        let living = source.living()?;
        let source_team = teams.entity_team(source.uuid());
        Some(crate::entity::warden::can_target(
            source.entity_type_id(),
            true,
            false,
            crate::entity::ScoreboardTeams::allied(warden_team, source_team),
            source.invulnerable(),
            living.is_dead_or_dying(),
            border,
            game_time,
            source.bounding_box(),
        ))
    }
}

/// One `DynamicGameEventListener` call, with `ifChunkExists`
/// (`DynamicGameEventListener.java:41-48`) answered for the two columns it can
/// touch before the registry is borrowed.
fn apply_listener_edge(
    chunks: &mut crate::chunk::ChunkStore,
    entity_id: i32,
    kind: EntityGameEventListenerKind,
    radius: i32,
    position: Option<Vec3d>,
) {
    let last = chunks
        .block_entities()
        .game_event_listeners()
        .entity_listener_column(entity_id, kind);
    let current = position.map(|at| {
        let pos = containing_block_pos(at);
        (pos.x >> 4, pos.z >> 4)
    });
    let loaded =
        [last, current].map(|column| column.map(|(x, z)| ((x, z), chunks.is_loaded(x, z))));
    let chunk_exists = |column: (i32, i32)| {
        loaded
            .iter()
            .flatten()
            .any(|(known, is_loaded)| *known == column && *is_loaded)
    };
    let registry = chunks.block_entities_mut().game_event_listeners_mut();
    match position {
        Some(position) => {
            registry.move_entity_listener(entity_id, kind, radius, position, chunk_exists);
        }
        None => registry.remove_entity_listener(entity_id, kind, chunk_exists),
    }
}
