//! Building a [`ServerWorld`] out of a validated save candidate.

use std::collections::BTreeSet;

use bbb_protocol::game_rules::vanilla_game_rule;
use bbb_protocol::packets::{PlayerRotation, Vec3d};
use bbb_server_persistence::Java26WorldCandidate;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey, SaveError, SaveResult};
use bbb_server_types::schema_support::invalid_schema;

use super::level_sources::level_sources_from_world_gen_settings;
use super::maps::{ServerMapPersistentData, ServerMapStore};
use super::{
    LevelId, SavedChunkTicket, ServerWorld, ServerWorldConfig, WorldSpawn, CHUNK_FULL_LEVEL,
    GAME_RULE_RESTORE_PRINCIPAL,
};
use crate::admin::{AdminEnvelope, AdminRequest, AdminScope};
use crate::chunk_sender::ChunkKey;

fn invalid(detail: String) -> SaveError {
    SaveError::InvalidNbt { path: None, detail }
}

impl ServerWorld {
    /// Restores a world from a fully validated save candidate, before the
    /// first tick.
    pub fn restore(
        config: ServerWorldConfig,
        candidate: &Java26WorldCandidate,
    ) -> SaveResult<Self> {
        let settings = &candidate.world_gen_settings;
        let declared =
            level_sources_from_world_gen_settings(settings, config.datapacks.as_ref(), false)?;
        let mut world = Self::with_level_sources(
            config,
            &declared.served,
            &declared.dimension_types,
            settings.seed,
            settings.generate_structures,
        )
        .map_err(|error| invalid(error.to_string()))?;
        world.record_unserved_levels(declared.unserved);
        if let Some(saved_features) = candidate.level.enabled_features() {
            world.feature_flags.merge(
                super::simulation_level::SimulationFeatureFlags::from_feature_ids(
                    saved_features.iter().map(String::as_str),
                ),
            );
        }
        // `MinecraftServer.tickCount` restarts every boot; only the level's
        // game time comes out of the save.
        if candidate.level.game_time() < 0 {
            return Err(invalid(format!(
                "level.dat game time {} is negative and cannot date a scheduled tick",
                candidate.level.game_time()
            )));
        }
        world.game_time = candidate.level.game_time();
        world.cold.seed = settings.seed;
        world.restore_world_clocks(&candidate.world_clocks)?;
        for dimension in world.level_ids() {
            let key = world.level_key(dimension).clone();
            // ServerLevel.java:275: each level restores its own Raids SavedData.
            let raids = candidate.raids.get(&key).ok_or_else(|| {
                invalid(format!(
                    "{} has no loaded raid saved data",
                    key.resource_id()
                ))
            })?;
            let border = candidate.world_borders.get(&key).ok_or_else(|| {
                invalid(format!(
                    "{} has no loaded world-border settings",
                    key.resource_id()
                ))
            })?;
            let game_time = world.game_time;
            let seed = world.cold.seed;
            let level = world.level_mut_by_id(&dimension);
            level.raids = super::raids::LevelRaids::from_saved(raids.clone());
            level.dragon_fight = candidate
                .ender_dragon_fights
                .get(&key)
                .map(|saved| super::dragon_fight::DragonFight::new(saved.clone(), seed));
            level.world_border = bbb_server_types::interaction::WorldBorder::from_settings(
                border.center_x,
                border.center_z,
                border.damage_per_block,
                border.safe_zone,
                border.warning_blocks,
                border.warning_time,
                border.size,
                border.lerp_time,
                border.lerp_target,
                game_time,
            );
        }
        let spawn = candidate.level.spawn();
        let [spawn_x, spawn_y, spawn_z] = spawn.position();
        world.cold.default_spawn = WorldSpawn {
            dimension: Java26LevelKey::parse(spawn.dimension())
                .ok()
                .and_then(|key| world.level_id(&key))
                .unwrap_or_else(|| Java26BuiltinDimension::Overworld.into()),
            position: Vec3d {
                x: f64::from(spawn_x) + 0.5,
                y: f64::from(spawn_y),
                z: f64::from(spawn_z) + 0.5,
            },
            rotation: PlayerRotation {
                y_rot: spawn.yaw(),
                x_rot: spawn.pitch(),
            },
        };
        // MinecraftServer.java:343-344: saved data, or a fresh value for a
        // save that never wrote the file.
        *world.cold.random_sequences.get_mut() =
            crate::random_sequences::RandomSequences::from_saved(&candidate.random_sequences);
        world.cold.scheduled_events =
            super::scheduled_events::ScheduledEvents::from_packed(&candidate.scheduled_events);
        world.cold.stopwatches = super::stopwatches::Stopwatches::unpack(&candidate.stopwatches);
        world.cold.wandering_trader.data = candidate.wandering_trader;
        world.weather = candidate.weather;
        world.scoreboard = super::server_scoreboard::ServerScoreboard::restore_with_registry(
            &candidate.scoreboard,
            std::sync::Arc::clone(world.registries.content_registry_arc()),
        );
        world.cold.boss_events = super::boss_events::ServerBossEvents::restore_with_registry(
            &candidate.custom_boss_events,
            world.plugins().content_registry(),
        );
        world.cold.command_storage = super::command_storage::ServerCommandStorage::from_persisted(
            candidate.command_storage.clone(),
        );
        // ServerLevel.java:271-273: a save restored mid-storm is already raining.
        let weather = world.weather;
        for dimension in world.level_ids() {
            world.level_mut_by_id(&dimension).weather.prepare(weather);
        }
        let maps = candidate
            .maps
            .iter()
            .map(|(&map_id, map)| Ok((map_id, ServerMapPersistentData::from_saved(map)?)))
            .collect::<SaveResult<Vec<_>>>()?;
        world.maps = ServerMapStore::from_persistent_parts_with_stored(
            maps,
            candidate.stored_maps.clone(),
            candidate.map_last_id,
            candidate.column_reader.clone(),
        )
        .map_err(|error| invalid(format!("invalid map saved data: {error:?}")))?;
        world.cold.stored_data_reader = candidate.column_reader.clone();
        // DedicatedServer.java:237 overrides the saved game type with the
        // startup property, which `with_level_sources` already applied.
        // PrimaryLevelData.java:255-257; DedicatedServer.java:395-397: the lock
        // survives the forced difficulty.
        world.cold.difficulty_locked = candidate.level.difficulty_settings().locked();
        // MinecraftServer.java:1290: a hardcore save runs HARD whatever the
        // startup difficulty asked for.
        world.cold.hardcore = candidate.level.difficulty_settings().hardcore();
        let mut restore_entity_events = Vec::new();
        // MinecraftServer.java:343-346,399-402: rules and forced difficulty precede ticket loading.
        world.restore_game_rules(candidate);
        world.set_difficulty_ignoring_lock(
            world.admin.settings().difficulty,
            &mut restore_entity_events,
        );
        let registries = world.registries.clone();
        let item_stack_resolver = world.item_stack_resolver();
        world.refresh_saved_column_resources();
        // LevelStorageSource.java:508-510: each level reads its own dimension directory.
        for dimension in world.level_ids() {
            let saved = candidate.dimensions.get(world.level_key(dimension));
            let level = world.level_mut_by_id(&dimension);
            level.chunk_lifecycle.storage_bound = true;
            level
                .chunks
                .restore_saved_columns(saved, &registries, &item_stack_resolver, true)?;
            if let Some(saved) = saved {
                // Stored records are read when a ticket first reaches them.
                level.chunks.open_stored_columns(
                    candidate.column_reader.clone(),
                    &saved.dimension,
                    saved.stored_chunks.iter().copied(),
                    saved.stored_poi_chunks.iter().copied(),
                );
                level.chunk_lifecycle.stored_entities = saved.stored_entity_chunks.clone();
                // Records already in memory publish now, entities included.
                for chunk in saved.chunks.keys().chain(saved.entity_chunks.keys()) {
                    level.chunk_lifecycle.entity_loads.insert(
                        *chunk,
                        super::chunk_lifecycle::EntityChunkLoadStatus::Loaded,
                    );
                    level.chunk_lifecycle.entity_unloads.insert(*chunk);
                }
                level.saved_chunks = saved.chunks.keys().copied().collect();
                level.saved_tick_chunks = saved
                    .chunks
                    .iter()
                    .filter(|(_, chunk)| {
                        !chunk.data.block_ticks().is_empty() || !chunk.data.fluid_ticks().is_empty()
                    })
                    .map(|(&position, _)| position)
                    .collect();
                level.saved_entity_chunks = saved.entity_chunks.keys().copied().collect();
            }
            world.restore_entities(&dimension, candidate)?;
            world.restore_chunk_tickets(&dimension, candidate)?;
            // LevelChunk.java:622-630: every saved FULL column registers its
            // merged tick containers against restored time.
            for chunk in world.level_by_id(&dimension).saved_chunks.clone() {
                if world
                    .level_by_id(&dimension)
                    .chunks
                    .is_ticking_full(chunk.0, chunk.1)
                {
                    world.register_chunk_tick_containers_by_key(&dimension, chunk);
                }
            }
            // PersistentEntitySectionManager.java:70-96: restored entities start
            // tracking before any viewer exists.
            world.apply_level_entity_lifecycle_by_key(&dimension, &mut restore_entity_events);
            assert!(
                restore_entity_events.is_empty(),
                "restoring entities before sessions exist cannot publish packets"
            );
        }
        // MinecraftServer.java:437-455: an uninitialized level picks its spawn once.
        if !candidate.level.initialized() && !world.is_debug() {
            world.set_initial_spawn()?;
        }
        world
            .finish_debug_dimension_initialization(candidate.level.initialized())
            .map_err(|error| invalid(error.to_string()))?;
        world.refresh_all_sky_darkening();
        world.restore_saved_players(candidate)?;
        #[cfg(test)]
        world.publish();
        Ok(world)
    }

    /// `MinecraftServer.java:345` reads the saved `GameRuleMap`; rules equal
    /// to their default are skipped, the rest go through the one writer the
    /// management surface also uses.
    fn restore_game_rules(&mut self, candidate: &Java26WorldCandidate) {
        let updates = candidate
            .game_rules
            .iter()
            .filter(|(key, value)| {
                !vanilla_game_rule(key).is_some_and(|rule| rule.default_value == **value)
            })
            .map(|(key, value)| (key.clone(), *value))
            .collect::<Vec<_>>();
        for (key, value) in updates {
            let online = self.online_players();
            let view = self.admin_world_view(&online);
            let envelope = AdminEnvelope {
                operation_id: 0,
                request_id: format!("{GAME_RULE_RESTORE_PRINCIPAL}:{key}"),
                principal: GAME_RULE_RESTORE_PRINCIPAL.to_string(),
                scope: AdminScope::Manage,
                deadline_tick: u64::MAX,
                now: 0,
            };
            let mut effects = Vec::new();
            let outcome = self.admin.apply(
                &envelope,
                &AdminRequest::UpdateGameRule { key, value },
                view,
                &mut effects,
            );
            debug_assert!(
                effects.is_empty(),
                "a game-rule write produces no administration effect"
            );
            self.absorb_game_rule(&outcome);
        }
    }

    /// `MinecraftServer.prepareLevels` (`MinecraftServer.java:543-561`): the
    /// level's saved `chunk_tickets.dat` tickets activate, and their columns
    /// load to `FULL_CHUNK_LEVEL` 33 because both persisted types carry
    /// `FLAG_LOADING` (`TicketType.java:22-23`). Startup waits until they
    /// reach that status.
    pub(super) fn restore_chunk_tickets(
        &mut self,
        dimension: &LevelId,
        candidate: &Java26WorldCandidate,
    ) -> SaveResult<()> {
        let tickets = candidate
            .chunk_tickets
            .get(self.level_key(*dimension))
            .map(|tickets| {
                tickets
                    .iter()
                    .map(|ticket| SavedChunkTicket {
                        chunk: (ticket.chunk_x, ticket.chunk_z),
                        level: ticket.level,
                        ticks_left: ticket.ticks_left,
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.level_mut_by_id(dimension).saved_chunk_tickets = tickets;
        self.refresh_simulation_chunk_levels_by_key(dimension);
        let chunks = self.full_loading_reach(dimension)?;
        self.install_full_chunks_by_key(dimension, &chunks)?;
        for chunk in chunks {
            self.register_chunk_tick_containers_by_key(dimension, chunk);
        }
        // Startup finishes its tickets' generation before the first tick.
        self.refresh_loading_chunks_waiting(dimension, true)
    }

    fn full_loading_reach(&self, dimension: &LevelId) -> SaveResult<Vec<ChunkKey>> {
        let mut chunks = BTreeSet::new();
        for ticket in &self.level_by_id(dimension).saved_chunk_tickets {
            let Some(radius) = CHUNK_FULL_LEVEL.checked_sub(ticket.level) else {
                continue;
            };
            if radius < 0 {
                continue;
            }
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    let chunk_x = ticket.chunk.0.checked_add(dx).ok_or_else(|| {
                        invalid_schema(
                            "chunk_tickets",
                            "FULL loading reach overflows the chunk x coordinate",
                        )
                    })?;
                    let chunk_z = ticket.chunk.1.checked_add(dz).ok_or_else(|| {
                        invalid_schema(
                            "chunk_tickets",
                            "FULL loading reach overflows the chunk z coordinate",
                        )
                    })?;
                    chunks.insert((chunk_x, chunk_z));
                }
            }
        }
        Ok(chunks.into_iter().collect())
    }

    /// `EntityStorage.loadEntities` for every entity chunk the save holds
    /// (`ServerLevel.java:255`).
    fn restore_entities(
        &mut self,
        dimension: &LevelId,
        candidate: &Java26WorldCandidate,
    ) -> SaveResult<()> {
        let Some(saved) = candidate.dimensions.get(self.level_key(*dimension)) else {
            return Ok(());
        };
        let input = self.entity_nbt_read_snapshot();
        let registries = &self.registries;
        let scoreboard = &mut self.scoreboard;
        let level = self
            .levels
            .get_mut(dimension)
            .expect("every saved dimension has a live level");
        let raids = &mut level.raids;
        let entities = &mut level.entities;
        entities.set_game_time(self.game_time);
        for (&(chunk_x, chunk_z), chunk) in &saved.entity_chunks {
            let stand_teams = crate::entity::save::armor_stand_teams(&chunk.data);
            let ids = crate::entity::save::read_entity_chunk_with_context(
                entities,
                &chunk.data,
                input.context(),
            )
            .map_err(|error| invalid(format!("entity chunk ({chunk_x},{chunk_z}): {error}")))?;
            for id in ids {
                let Some(entity) = entities.get_mut(id) else {
                    continue;
                };
                if !crate::entity::resolve_spawner_registry_holders(entity, |registry, resource| {
                    registries.registry_element_index(registry, resource)
                }) {
                    tracing::warn!(
                        event = "world.entity_variant_unresolved",
                        x = chunk_x,
                        z = chunk_z,
                        entity = %entity.uuid()
                    );
                }
                let uuid = entity.uuid();
                if let Some(team) = entity.take_loaded_team() {
                    scoreboard.add_player_to_team(&uuid.to_string(), &team);
                }
                if matches!(entity.data(), crate::entity::EntityData::ArmorStand(_)) {
                    if let Some(&team) = stand_teams.get(&uuid) {
                        scoreboard.add_player_to_team(&uuid.to_string(), team);
                    }
                }
                // Raider.java:191-206: resolve saved RaidId across root and all passengers.
                crate::world::raids::resolve_raider_raids_in_tree_parts(entities, raids, id);
                if let Some(tree) = entities.entity_tree_ids(id) {
                    for child_id in tree {
                        if child_id == id {
                            continue;
                        }
                        let Some(child) = entities.get_mut(child_id) else {
                            continue;
                        };
                        if let Some(team) = child.take_loaded_team() {
                            scoreboard.add_player_to_team(&child.uuid().to_string(), &team);
                        }
                        if matches!(child.data(), crate::entity::EntityData::ArmorStand(_)) {
                            if let Some(&team) = stand_teams.get(&child.uuid()) {
                                scoreboard.add_player_to_team(&child.uuid().to_string(), team);
                            }
                        }
                    }
                }
            }
        }
        // EntityReference.java:55-76: saved lastHurtByMob UUIDs resolve once
        // every record of the level is live.
        crate::entity::resolve_saved_last_hurt_by_mob_references(entities);
        Ok(())
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn chunk_tick_containers_registered_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        chunk: ChunkKey,
    ) -> bool {
        self.level(dimension).chunks.ticks_registered(chunk)
    }

    pub(crate) fn restored_chunk_ticket_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        chunk: ChunkKey,
    ) -> Option<(i32, Option<i64>)> {
        self.level(dimension)
            .saved_chunk_tickets
            .iter()
            .find(|ticket| ticket.chunk == chunk)
            .map(|ticket| (ticket.level, ticket.ticks_left))
    }
}
