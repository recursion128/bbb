//! `Level.tickBlockEntities` and the `BlockEntityTicker` dispatch over the
//! owned block-entity types.

use super::*;
use crate::world::LevelId;

impl ServerWorld {
    /// `Level.tickBlockEntities` (`Level.java:529-549`).
    ///
    /// The `tickRateManager().runsNormally()` gate (`:537`) is read once and
    /// applied to every ticker, exactly as vanilla reads it; the walk itself
    /// still runs, because a removed ticker leaves the list whether the world
    /// is frozen or not.
    ///
    /// `BoundTickingBlockEntity.tick` (`LevelChunk.java:741-767`) adds two
    /// gates of its own on top of `shouldTickBlocksAt` — `LevelChunk.isTicking`
    /// (`:411-419`), which is the world border plus the column's full status
    /// plus `areEntitiesLoaded`, and the `getType().isValid(blockState)` check
    /// at `:749`. Loading confirmation and entity storage readiness are separate
    /// from the simulation ticket range; a replaced block entity must not tick.
    pub(super) fn tick_block_entities(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let runs_normally = self.tick_rate.runs_normally();
        let level = self.level_mut_by_id(dimension);
        if !runs_normally {
            level.chunks.block_entities_mut().prune_tickers();
            return;
        }
        let tickers = level.chunks.block_entities_mut().take_tickers();
        let game_time = self.game_time();
        for &(x, y, z) in &tickers {
            let pos = BlockPos { x, y, z };
            let level = self.level_by_id(dimension);
            let chunk = (x >> 4, z >> 4);
            if level.chunk_lifecycle.status(chunk)
                < super::super::chunk_lifecycle::FullChunkStatus::BlockTicking
                || !level.chunk_lifecycle.entities_loaded(chunk)
                || !level.world_border.contains_block_at(pos, game_time)
                || !level.simulation_levels.is_block_ticking(chunk)
            {
                continue;
            }
            let Some(block_state_id) = level.chunks.get_block_state(x, y, z) else {
                continue;
            };
            let Some(block_entity) = level.chunks.block_entities().get(pos) else {
                continue;
            };
            if !block_entity.is_valid_block_state(block_state_id) {
                continue;
            }
            self.tick_one_block_entity(dimension, pos, events);
        }
        self.level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .return_tickers_scratch(tickers);
    }

    /// One `BlockEntityTicker.tick(level, pos, state, entity)`
    /// (`BlockEntityTicker.java:9`) dispatched over the owned types.
    pub(super) fn tick_one_block_entity(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        use crate::block_entity::BlockEntityPayload;

        let Some(payload) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .map(BlockEntity::payload)
        else {
            return;
        };
        match payload {
            BlockEntityPayload::Beehive(_) => self.tick_beehive(dimension, pos),
            BlockEntityPayload::Beacon(_) => self.tick_beacon(dimension, pos, events),
            BlockEntityPayload::Conduit(_) => self.tick_conduit(dimension, pos, events),
            BlockEntityPayload::Vault(_) => self.tick_vault(dimension, pos, events),
            BlockEntityPayload::Crafter(_) => {
                if let Some(finished) = self
                    .level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(BlockEntity::crafter_mut)
                    .map(|crafter| crafter.server_tick())
                {
                    if finished {
                        let state = self
                            .block_state_by_key(dimension, pos)
                            .expect("crafter state");
                        let state = crate::fluid::with_property(state, "crafting", "false")
                            .expect("crafter crafting");
                        self.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL);
                    }
                }
            }
            BlockEntityPayload::EndGateway(_) => {
                if let Some(effects) = self
                    .level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(BlockEntity::end_gateway_mut)
                    .map(|gateway| gateway.tick())
                {
                    let level = self.level_mut_by_id(dimension);
                    if effects.block_event {
                        level
                            .block_events
                            .block_event(crate::block_events::BlockEventData::new(
                                pos,
                                bbb_protocol::block_types::VANILLA_BLOCK_END_GATEWAY_ID,
                                1,
                                0,
                            ));
                    }
                    if effects.changed {
                        level.chunks.block_entities_mut().set_changed(pos);
                    }
                }
            }
            BlockEntityPayload::Piston(_) => self.tick_moving_piston(dimension, pos, events),
            BlockEntityPayload::Hopper(_) => self.tick_hopper(dimension, pos),
            BlockEntityPayload::ShulkerBox(_) => self.tick_shulker_box(dimension, pos, events),
            BlockEntityPayload::Jukebox(_) => self.tick_jukebox(dimension, pos),
            BlockEntityPayload::Campfire(_) => self.tick_campfire(dimension, pos),
            BlockEntityPayload::Furnace(_) => self.tick_furnace(dimension, pos),
            BlockEntityPayload::BrewingStand(_) => self.tick_brewing_stand(dimension, pos),
            BlockEntityPayload::CreakingHeart(_) => self.tick_creaking_heart(dimension, pos),
            BlockEntityPayload::Spawner(_) => self.tick_spawner(dimension, pos),
            BlockEntityPayload::TrialSpawner(_) => self.tick_trial_spawner(dimension, pos, events),
            BlockEntityPayload::SculkShrieker(_) => {
                self.tick_sculk_shrieker(dimension, pos, events)
            }
            BlockEntityPayload::SculkCatalyst(_) => {
                self.tick_sculk_catalyst(dimension, pos, events)
            }
            BlockEntityPayload::SculkSensor(_) => self.tick_sculk_sensor(dimension, pos, events),
            BlockEntityPayload::DaylightDetector(_) => {
                // `DaylightDetectorBlock.getTicker` / `tickEntity` (`:106-116`):
                // only a sky-lit dimension ticks, once every 20 game ticks.
                let level = self.level_by_id(dimension);
                if !level.chunks.has_skylight() || self.game_time % 20 != 0 {
                    return;
                }
                if let Some(state) = self.block_state_by_key(dimension, pos) {
                    self.with_level(dimension, |level| {
                        crate::block_behavior::daylight_detector::update_signal_strength(
                            level, pos, state,
                        );
                    });
                }
            }
            BlockEntityPayload::Bell(_) => self.tick_bell(dimension, pos, events),
            BlockEntityPayload::Sign(_) => {
                self.resolve_loaded_sign_lines(*dimension, pos);
                // `SignBlockEntity.tick` (`SignBlockEntity.java:265-270`) needs the
                // level's player list, which is why the predicate is built here and
                // the object only decides what to do with the answer.
                let editor = self
                    .level_by_id(dimension)
                    .chunks
                    .block_entities()
                    .get(pos)
                    .and_then(BlockEntity::sign)
                    .and_then(sign::SignBlockEntity::player_who_may_edit);
                let Some(editor) = editor else {
                    return;
                };
                let too_far = self.sign_editor_is_too_far_away_by_key(dimension, editor, pos);
                if let Some(sign) = self
                    .level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(BlockEntity::sign_mut)
                {
                    sign.tick(|_| too_far);
                }
            }
            BlockEntityPayload::Chest(_)
            | BlockEntityPayload::Comparator(_)
            | BlockEntityPayload::Barrel(_)
            | BlockEntityPayload::Dispenser(_)
            | BlockEntityPayload::Lectern(_)
            | BlockEntityPayload::ChiseledBookshelf(_)
            | BlockEntityPayload::DecoratedPot(_)
            | BlockEntityPayload::Shelf(_)
            | BlockEntityPayload::CopperGolemStatue(_)
            | BlockEntityPayload::Skull(_)
            | BlockEntityPayload::Brushable(_)
            | BlockEntityPayload::Banner(_)
            | BlockEntityPayload::Bed(_)
            | BlockEntityPayload::EnchantingTable(_)
            | BlockEntityPayload::EndPortal(_)
            | BlockEntityPayload::CommandBlock(_)
            | BlockEntityPayload::StructureBlock(_)
            | BlockEntityPayload::Jigsaw(_)
            | BlockEntityPayload::TestBlock(_)
            | BlockEntityPayload::TestInstanceBlock(_)
            | BlockEntityPayload::EnderChest(_) => {}
        }
    }

    /// `JukeboxBlockEntity.tick` (`JukeboxBlockEntity.java:64-66`) reaching
    /// `JukeboxSongPlayer.tick` (`JukeboxSongPlayer.java:63-76`).
    fn tick_jukebox(&mut self, dimension: &LevelId, pos: BlockPos) {
        let level = self.level_by_id(dimension);
        let Some(block_state_id) = level.chunks.get_block_state(pos.x, pos.y, pos.z) else {
            return;
        };
        if !level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::jukebox)
            .is_some_and(|jukebox| jukebox.song_player().is_playing())
        {
            return;
        }
        let resolver = self.item_stack_resolver();
        let Some(event) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::jukebox_mut)
            .map(|jukebox| jukebox.tick(&resolver))
        else {
            return;
        };
        match event {
            Ok(event) => {
                self.broadcast_jukebox_song_event(dimension, pos, block_state_id, event, None)
            }
            Err(error) => tracing::warn!("jukebox tick: {error}"),
        }
    }

    /// `ShulkerBoxBlockEntity.tick` (`ShulkerBoxBlockEntity.java:62-101`).
    pub(super) fn tick_shulker_box(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(block_state_id) = self.block_state_by_key(dimension, pos) else {
            return;
        };
        let Some(shulker) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::shulker_box_mut)
        else {
            return;
        };
        let opening = shulker.animation_status()
            == crate::block_entity::shulker_box::AnimationStatus::Opening;
        let needs_neighbors = shulker.tick_animation();
        let push = opening.then(|| {
            shulker.lid_push(
                crate::block_behavior::shulker_box::facing(block_state_id),
                pos,
            )
        });
        if needs_neighbors {
            if let Some(block_type_id) = vanilla_block_state_block_type_id(block_state_id) {
                self.with_level(dimension, |level| {
                    // `doNeighborUpdates` (`ShulkerBoxBlockEntity.java:157-160`).
                    level.update_neighbor_shapes(
                        pos,
                        block_state_id,
                        SET_BLOCK_UPDATE_ALL,
                        UPDATE_LIMIT,
                    );
                    level.update_neighbors_at(pos, block_type_id);
                });
            }
        }
        if let Some((band, delta)) = push {
            self.push_by_shulker_lid(dimension, band, delta, events);
        }
    }

    // ShulkerBoxBlockEntity.java:112-131: everything but spectators and
    // `PushReaction.IGNORE` bodies moves with `MoverType.SHULKER_BOX`.
    fn push_by_shulker_lid(
        &mut self,
        dimension: &LevelId,
        band: bbb_server_types::geometry::Aabb,
        delta: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        use crate::entity::{piston_push_reaction, MoverType};
        use bbb_protocol::block_state_types::VanillaPistonPushReaction;
        self.with_level_and_entities(*dimension, |level, store| {
            for id in store.ids_in_box_sections(band) {
                let _ = store.with_entity_mut(id, |entity, store| {
                    if entity.bounding_box().intersects(band)
                        && piston_push_reaction(entity) != VanillaPistonPushReaction::Ignore
                    {
                        let _ = crate::entity::movement::move_living_entity(
                            entity,
                            store,
                            level,
                            MoverType::ShulkerBox,
                            delta,
                        );
                    }
                });
            }
        });
        let players: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                &player.dimension == dimension
                    && !player.avatar_removed
                    && player.game_type != GameType::Spectator
                    && player
                        .bounding_box()
                        .is_ok_and(|bounds| bounds.intersects(band))
            })
            .map(|(&id, _)| id)
            .collect();
        for id in players {
            let _ = self.move_player_by_external_force(id, delta, MoverType::ShulkerBox, events);
        }
    }
}

#[cfg(test)]
mod shulker_lid_tests;
