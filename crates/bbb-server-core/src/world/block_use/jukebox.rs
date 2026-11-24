use super::BlockUseContext;
use crate::block_behavior::jukebox as jukebox_block;
use crate::block_behavior::{BlockUseResult, SET_BLOCK_UPDATE_CLIENTS};
use crate::block_entity::jukebox::{JukeboxBlockEntity, JukeboxSongEvent};
use crate::block_entity::BlockEntity;
use crate::entity::spawn_dropped_item_entity;
use crate::jukebox_song_facts::JUKEBOX_SONG_REGISTRY;
use crate::level_event::VanillaGameEvent;
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{LevelId, ServerWorld, WorldEvent};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::custom_stat;

/// `Stats.PLAY_RECORD` (`Stats.java`, `JukeboxPlayable.java:52`).
const PLAY_RECORD: &str = "minecraft:play_record";
/// `LevelEvent.SOUND_PLAY_JUKEBOX_SONG` (`LevelEvent.java:9`), whose `data` is
/// the song's holder id in the published registry (`JukeboxSongPlayer.java:48`).
const LEVEL_EVENT_SOUND_PLAY_JUKEBOX_SONG: i32 = 1010;
use crate::level_event::LEVEL_EVENT_SOUND_STOP_JUKEBOX_SONG;

/// The `item_on:` column: [`ServerWorld::use_jukebox_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_jukebox_item_on(context, events)
}

/// The `without_item:` column: [`ServerWorld::use_jukebox_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_jukebox_without_item(context)
}

impl ServerWorld {
    pub(super) fn use_jukebox_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        _events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        if jukebox_block::has_record(context.block_state_id) {
            return BlockUseResult::TryWithEmptyHand;
        }
        if self.insert_record_into_jukebox(context) {
            BlockUseResult::Consumed
        } else {
            BlockUseResult::TryWithEmptyHand
        }
    }

    fn insert_record_into_jukebox(&mut self, context: &BlockUseContext<'_>) -> bool {
        let old_state = context.block_state_id;
        let Some(new_state) = jukebox_block::with_has_record(old_state, true) else {
            return false;
        };
        let prepared = (|| {
            let player = self
                .levels
                .player(context.connection_id)
                .expect("connection presence was checked");
            if !runtime_item_in_hand(player, context.update.hand)
                .has_component(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID)?
            {
                return Ok(None);
            }
            let record = runtime_item_in_hand(player, context.update.hand).with_count(1);
            let Some(mut candidate) = self
                .level_by_id(&context.dimension)
                .chunks
                .block_entities()
                .get(context.update.hit.pos)
                .and_then(BlockEntity::jukebox)
                .cloned()
            else {
                return Ok(None);
            };
            let event = candidate.set_the_item(record, &self.item_stack_resolver())?;
            Ok::<_, bbb_server_types::ServerWorldError>(Some((
                player.entity_id(),
                candidate,
                event,
            )))
        })();
        let (source_entity_id, candidate, event) = match prepared {
            Ok(Some(prepared)) => prepared,
            Ok(None) => return false,
            Err(error) => {
                tracing::warn!("jukebox record insertion: {error}");
                return false;
            }
        };
        let player = self
            .levels
            .player_mut(context.connection_id)
            .expect("connection presence was checked");
        player
            .inventory
            .consume_in_hand(context.update.hand, 1, player.abilities.instabuild);
        self.commit_jukebox_item(
            context.dimension,
            context.update.hit.pos,
            new_state,
            candidate,
            event,
        );
        self.level_mut_by_id(&context.dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                context.update.hit.pos,
                Some(source_entity_id),
                Some(old_state),
            );
        if let Some(player) = self.levels.player_mut(context.connection_id) {
            player.award_stat_once(&mut self.scoreboard, custom_stat(PLAY_RECORD));
        }
        true
    }

    pub(super) fn use_jukebox_without_item(&mut self, context: &BlockUseContext<'_>) -> bool {
        if !jukebox_block::has_record(context.block_state_id) {
            return false;
        }
        if self
            .level_by_id(&context.dimension)
            .chunks
            .block_entities()
            .get(context.update.hit.pos)
            .and_then(BlockEntity::jukebox)
            .is_none()
        {
            return false;
        }
        self.pop_record_from_jukebox(context);
        true
    }

    fn pop_record_from_jukebox(&mut self, context: &BlockUseContext<'_>) {
        let pos = context.update.hit.pos;
        let Some(record) = self
            .level_by_id(&context.dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::jukebox)
            .map(|jukebox| jukebox.item().clone())
        else {
            return;
        };
        if record.is_empty() {
            return;
        }
        let Some(new_state) = jukebox_block::with_has_record(context.block_state_id, false) else {
            return;
        };
        let item = record.clone();
        if !self.set_the_jukebox_item(context.dimension, pos, new_state, RuntimeItemStack::empty())
        {
            return;
        }
        let dx = (self.random.next_float() - 0.5) * 0.7;
        let dz = (self.random.next_float() - 0.5) * 0.7;
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5 + f64::from(dx),
            y: f64::from(pos.y) + 1.01,
            z: f64::from(pos.z) + 0.5 + f64::from(dz),
        };
        self.spawn_popped_jukebox_record(context.dimension, position, item);
        // `JukeboxBlockEntity.onSongChanged` (`JukeboxBlockEntity.java:36-39`).
        self.with_level(context.dimension, |level| {
            if let Some(block) = vanilla_block_state_block_type_id(new_state) {
                level.update_neighbors_at(pos, block);
            }
            level.block_entity_set_changed(pos, new_state);
        });
    }

    pub(crate) fn set_the_jukebox_item(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: bbb_protocol::packets::BlockPos,
        new_state: i32,
        record: RuntimeItemStack,
    ) -> bool {
        let dimension = dimension.into();
        let Some(mut candidate) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::jukebox)
            .cloned()
        else {
            return false;
        };
        let event = match candidate.set_the_item(record, &self.item_stack_resolver()) {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!("jukebox record update: {error}");
                return false;
            }
        };
        self.commit_jukebox_item(dimension, pos, new_state, candidate, event);
        true
    }

    fn commit_jukebox_item(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: bbb_protocol::packets::BlockPos,
        new_state: i32,
        candidate: JukeboxBlockEntity,
        event: JukeboxSongEvent,
    ) {
        let dimension = dimension.into();
        let song_id = candidate.song_player().song_id().map(str::to_owned);
        *self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::jukebox_mut)
            .expect("candidate jukebox is still present") = candidate;
        self.with_level(dimension, |level| {
            level.block_entity_set_changed(pos, new_state);
        });
        self.set_block(dimension, pos, new_state, SET_BLOCK_UPDATE_CLIENTS);
        self.level_mut_by_id(&dimension).event_sink.game_event(
            VanillaGameEvent::BLOCK_CHANGE,
            pos,
            Some(new_state),
        );
        self.broadcast_jukebox_song_event(&dimension, pos, new_state, event, song_id.as_deref());
    }

    pub(crate) fn broadcast_jukebox_song_event(
        &mut self,
        dimension: &crate::world::LevelId,
        pos: bbb_protocol::packets::BlockPos,
        block_state_id: i32,
        event: JukeboxSongEvent,
        song_id: Option<&str>,
    ) {
        match event {
            JukeboxSongEvent::None => return,
            JukeboxSongEvent::Playing => {
                let level = self.level_mut_by_id(dimension);
                level.event_sink.game_event(
                    VanillaGameEvent::JUKEBOX_PLAY,
                    pos,
                    Some(block_state_id),
                );
                // `JukeboxSongPlayer.spawnMusicParticles` (`:82-88`).
                let colour = level.random.next_int(4) as f32 / 24.0;
                level
                    .entities
                    .send_particles(bbb_protocol::packets::LevelParticles {
                        override_limiter: false,
                        always_show: false,
                        position: Vec3d {
                            x: f64::from(pos.x) + 0.5,
                            y: f64::from(pos.y) + f64::from(1.2_f32),
                            z: f64::from(pos.z) + 0.5,
                        },
                        offset: Vec3d {
                            x: f64::from(colour),
                            y: 0.0,
                            z: 0.0,
                        },
                        max_speed: 1.0,
                        count: 0,
                        particle: bbb_protocol::packets::ParticlePayload {
                            particle_type_id:
                                bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_NOTE_ID,
                            raw_options: Vec::new(),
                        },
                    });
                return;
            }
            JukeboxSongEvent::Started => {
                let Some(song) = song_id.and_then(|id| {
                    self.registries
                        .registry_element_index(JUKEBOX_SONG_REGISTRY, id)
                }) else {
                    return;
                };
                self.level_mut_by_id(dimension).event_sink.level_event(
                    LEVEL_EVENT_SOUND_PLAY_JUKEBOX_SONG,
                    pos,
                    song,
                );
            }
            JukeboxSongEvent::Stopped => {
                self.level_mut_by_id(dimension).event_sink.game_event(
                    VanillaGameEvent::JUKEBOX_STOP_PLAY,
                    pos,
                    Some(block_state_id),
                );
                self.level_mut_by_id(dimension).event_sink.level_event(
                    LEVEL_EVENT_SOUND_STOP_JUKEBOX_SONG,
                    pos,
                    0,
                );
            }
        }
        // `JukeboxBlockEntity.onSongChanged` (`JukeboxBlockEntity.java:36-39`).
        self.with_level(dimension, |level| {
            if let Some(block) = vanilla_block_state_block_type_id(block_state_id) {
                level.update_neighbors_at(pos, block);
            }
            level.block_entity_set_changed(pos, block_state_id);
        });
    }

    fn spawn_popped_jukebox_record(
        &mut self,
        dimension: impl Into<LevelId>,
        position: Vec3d,
        item: RuntimeItemStack,
    ) {
        let dimension = dimension.into();
        if spawn_dropped_item_entity(
            &mut self.level_mut_by_id(&dimension).entities,
            position,
            item,
        )
        .is_none()
        {
            tracing::warn!("the entity store refused the item entity's drawn UUID");
        }
    }
}
