use super::EntityChunkLoadStatus;
use crate::chunk_sender::ChunkKey;
use crate::world::ServerWorld;
use crate::world::{LevelId, ServerLevel};
use bbb_server_persistence::world_storage::Java26RuntimeColumn;
use bbb_server_persistence::{Java26LoadedDimension, Java26LoadedEntityChunk};
use bbb_server_types::save::{Java26LevelKey, SaveError, SaveResult};
use std::collections::BTreeSet;

/// The columns a ticket reaches whose saved record has not been read.
fn pending_columns(level: &ServerLevel) -> impl Iterator<Item = ChunkKey> + '_ {
    level
        .chunk_lifecycle
        .loading_levels
        .keys()
        .copied()
        .filter(|&chunk| {
            level.chunk_lifecycle.evicted_columns.contains(&chunk)
                || level.chunks.is_stored_readable(chunk)
        })
}

/// Unloads held back from one unload save, per level.
#[must_use]
pub struct HeldUnloads(Vec<(BTreeSet<ChunkKey>, BTreeSet<ChunkKey>)>);

impl ServerWorld {
    /// Every column a ticket reaches whose saved record has to be read first.
    pub fn pending_runtime_columns(&self) -> Vec<(Java26LevelKey, ChunkKey)> {
        self.levels
            .iter()
            .flat_map(|(_, level)| {
                pending_columns(level).map(move |chunk| (level.dimension.as_ref().clone(), chunk))
            })
            .collect()
    }

    /// The first stored record a level refused to read because it is not
    /// 26.1 data. The server stops on it rather than play on without it.
    pub fn storage_refusal(&self) -> Option<String> {
        self.levels
            .values()
            .find_map(|level| level.chunks.storage_refusal())
    }

    /// Reads the level's pending columns on this thread.
    pub(in crate::world) fn read_pending_columns_now(&mut self, dimension: &LevelId) {
        let level = self.level_by_id(dimension);
        let Some(reader) = level.chunks.stored_column_reader().cloned() else {
            return;
        };
        let key = level.dimension.as_ref().clone();
        let records = pending_columns(level)
            .map(|chunk| (chunk, reader.read_column(&key, chunk.0, chunk.1)))
            .collect::<Vec<_>>();
        if !records.is_empty() {
            self.complete_runtime_columns(&key, records);
        }
    }

    /// Reads the level's pending entity records on this thread.
    pub(in crate::world) fn read_pending_entities_now(&mut self, dimension: &LevelId) {
        let level = self.level_by_id(dimension);
        let Some(reader) = level.chunks.stored_column_reader().cloned() else {
            return;
        };
        let key = level.dimension.as_ref().clone();
        let pending = level
            .chunk_lifecycle
            .entity_loads
            .iter()
            .filter(|(_, status)| **status == EntityChunkLoadStatus::Pending)
            .map(|(&chunk, _)| chunk)
            .collect::<Vec<_>>();
        for chunk in pending {
            match reader.read_entity_chunk(&key, chunk.0, chunk.1) {
                Ok(record) => {
                    if let Err(error) = self.complete_runtime_entity_chunk(&key, chunk, record) {
                        tracing::error!(
                            event = "world.runtime_entity_publication_failed",
                            dimension = %key.resource_id(),
                            x = chunk.0,
                            z = chunk.1,
                            %error
                        );
                    }
                }
                Err(error) => self.fail_runtime_entity_chunk(&key, chunk, &error),
            }
        }
    }

    pub fn pending_runtime_entity_chunks(&mut self) -> Vec<(Java26LevelKey, ChunkKey)> {
        let mut requests = Vec::new();
        for (_, level) in self.levels.iter_mut() {
            if !level.chunk_lifecycle.storage_bound {
                continue;
            }
            // PersistentEntitySectionManager.java:193-198: Fresh roots load the old record before saving.
            for chunk in level.entities.iter_occupied_chunks() {
                level.chunk_lifecycle.request_entities(chunk);
                if !level
                    .entities
                    .section_visibility(crate::entity::SectionKey::new(chunk.0, 0, chunk.1))
                    .is_accessible()
                {
                    level.chunk_lifecycle.entity_unloads.insert(chunk);
                }
            }
            requests.extend(
                level
                    .chunk_lifecycle
                    .entity_loads
                    .iter()
                    .filter(|(_, status)| **status == EntityChunkLoadStatus::Pending)
                    .map(|(&chunk, _)| (level.dimension.as_ref().clone(), chunk)),
            );
        }
        requests
    }

    /// Publishes each read column on its own. A column that cannot be read or
    /// restored is reported and generated afresh instead
    /// (`ChunkMap.java:580-603`), so it never holds back the rest of the level.
    pub fn complete_runtime_columns(
        &mut self,
        dimension: &Java26LevelKey,
        records: Vec<(ChunkKey, SaveResult<Java26RuntimeColumn>)>,
    ) -> Vec<(ChunkKey, SaveError)> {
        let dimension = &self.require_level_id(dimension);
        let resolver = self.item_stack_resolver();
        let mut failures = Vec::new();
        for (chunk, record) in records {
            let level = self
                .levels
                .get_mut(dimension)
                .expect("served storage level");
            if !level.chunk_lifecycle.evicted_columns.remove(&chunk)
                && !level.chunks.is_stored_readable(chunk)
            {
                continue;
            }
            level.chunk_lifecycle.settled = None;
            let record = match record {
                Ok(record) => record,
                Err(error) => {
                    if level.chunks.stored_read_failed(chunk, "chunk", &error) {
                        level.chunks.discard_unrestorable_column(chunk.0, chunk.1);
                    }
                    failures.push((chunk, error));
                    continue;
                }
            };
            level.chunks.forget_stored_column(chunk);
            let mut saved = Java26LoadedDimension::new(level.dimension.as_ref().clone());
            saved.chunks.extend(record.chunk.map(|data| (chunk, data)));
            saved
                .poi_chunks
                .extend(record.poi.map(|data| (chunk, data)));
            let restored =
                level
                    .chunks
                    .restore_saved_columns(Some(&saved), &self.registries, &resolver, true);
            if let Err(error) = restored {
                level.chunks.discard_unrestorable_column(chunk.0, chunk.1);
                level.chunks.stored_read_failed(chunk, "chunk", &error);
                failures.push((chunk, error));
                continue;
            }
            if let Some(record) = saved.chunks.get(&chunk) {
                level.saved_chunks.insert(chunk);
                if !record.data.block_ticks().is_empty() || !record.data.fluid_ticks().is_empty() {
                    level.saved_tick_chunks.insert(chunk);
                }
            }
        }
        self.reconcile_saved_column_accesses(dimension);
        failures
    }

    pub fn complete_runtime_entity_chunk(
        &mut self,
        dimension: &Java26LevelKey,
        chunk: ChunkKey,
        record: Option<Java26LoadedEntityChunk>,
    ) -> SaveResult<()> {
        let dimension = &self.require_level_id(dimension);
        if self
            .level_by_id(dimension)
            .chunk_lifecycle
            .entity_loads
            .get(&chunk)
            != Some(&EntityChunkLoadStatus::Pending)
        {
            return Ok(());
        }
        if let Some(record) = record {
            let stand_teams = crate::entity::save::armor_stand_teams(&record.data);
            let input = self.entity_nbt_read_snapshot();
            let registries = self.registries.clone();
            let game_time = self.game_time;
            let scoreboard = &mut self.scoreboard;
            let level = self
                .levels
                .get_mut(dimension)
                .expect("served storage level");
            level.entities.set_game_time(game_time);
            let ids = crate::entity::save::read_entity_chunk_with_context(
                &mut level.entities,
                &record.data,
                input.context(),
            );
            let ids = ids.map_err(entity_input_error)?;
            let mut roots = Vec::with_capacity(ids.len());
            for id in ids {
                let entity = level.entities.get_mut(id).expect("published entity");
                if !crate::entity::resolve_spawner_registry_holders(entity, |registry, resource| {
                    registries.registry_element_index(registry, resource)
                }) {
                    tracing::warn!(
                        event = "world.entity_variant_unresolved",
                        x = chunk.0,
                        z = chunk.1,
                        entity = %entity.uuid()
                    );
                }
                let uuid = entity.uuid();
                roots.push(uuid);
                if let Some(team) = entity.take_loaded_team() {
                    scoreboard.add_player_to_team(&uuid.to_string(), &team);
                }
                if matches!(entity.data(), crate::entity::EntityData::ArmorStand(_)) {
                    if let Some(&team) = stand_teams.get(&uuid) {
                        scoreboard.add_player_to_team(&uuid.to_string(), team);
                    }
                }
                crate::world::raids::resolve_raider_raids_in_tree_parts(
                    &mut level.entities,
                    &mut level.raids,
                    id,
                );
                if let Some(tree) = level.entities.entity_tree_ids(id) {
                    for child_id in tree {
                        if child_id == id {
                            continue;
                        }
                        let child = level
                            .entities
                            .get_mut(child_id)
                            .expect("published passenger");
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
            crate::entity::resolve_saved_last_hurt_by_mob_references(&mut level.entities);
            level.saved_entity_chunks.insert(chunk);
            // A region copy of a pearl tree an offline player holds stays with
            // the player, as it does for the records read at restore.
            self.ensure_offline_pearl_owners_loaded(dimension);
            let held = self
                .cold
                .restored_players
                .values()
                .flat_map(|player| &player.ender_pearls)
                .map(|pearl| pearl.entity.as_ref().uuid())
                .filter(|uuid| roots.contains(uuid))
                .collect::<Vec<_>>();
            self.unload_restored_ender_pearls(held);
        }
        self.level_mut_by_id(dimension)
            .chunk_lifecycle
            .entity_loads
            .insert(chunk, EntityChunkLoadStatus::Loaded);
        Ok(())
    }

    /// An entity record that did not read stays unloaded and is never written
    /// over.
    pub fn fail_runtime_entity_chunk(
        &mut self,
        dimension: &Java26LevelKey,
        chunk: ChunkKey,
        error: &SaveError,
    ) {
        let dimension = &self.require_level_id(dimension);
        let level = self.level_mut_by_id(dimension);
        if level.chunk_lifecycle.entity_loads.get(&chunk) != Some(&EntityChunkLoadStatus::Pending) {
            return;
        }
        level
            .chunk_lifecycle
            .entity_loads
            .insert(chunk, EntityChunkLoadStatus::Failed);
        level.chunks.stored_read_failed(chunk, "entities", error);
    }

    /// Leaves at most `batch` column and `batch` entity unloads for the next
    /// unload save; the rest wait for later ticks, the way
    /// `ChunkMap.processUnloads` (`ChunkMap.java:485-491`) only drains its
    /// unload queue while the tick has time.
    pub fn hold_back_runtime_unloads(&mut self, batch: usize) -> HeldUnloads {
        let held = self
            .levels
            .iter_mut()
            .map(|(_, level)| {
                let lifecycle = &mut level.chunk_lifecycle;
                let split = |unloads: &mut BTreeSet<ChunkKey>| match unloads.iter().nth(batch) {
                    Some(&first_held) => unloads.split_off(&first_held),
                    None => BTreeSet::new(),
                };
                (
                    split(&mut lifecycle.column_unloads),
                    split(&mut lifecycle.entity_unloads),
                )
            })
            .collect();
        HeldUnloads(held)
    }

    pub fn release_runtime_unloads(&mut self, held: HeldUnloads) {
        for ((_, level), (columns, entities)) in self.levels.iter_mut().zip(held.0) {
            level.chunk_lifecycle.column_unloads.extend(columns);
            level.chunk_lifecycle.entity_unloads.extend(entities);
        }
    }

    /// Columns waiting for an unload save, over every level.
    pub fn runtime_unload_backlog(&self) -> usize {
        self.levels
            .values()
            .map(|level| level.chunk_lifecycle.column_unloads.len())
            .sum()
    }

    pub fn needs_runtime_unload_save(&self) -> bool {
        self.levels.values().any(|level| {
            level.chunk_lifecycle.storage_bound
                && ((self.auto_save && !level.chunk_lifecycle.column_unloads.is_empty())
                    || level
                        .chunk_lifecycle
                        .entity_unloads
                        .iter()
                        .any(|&chunk| level.chunk_lifecycle.entities_loaded(chunk)))
        })
    }

    pub(in crate::world) fn finish_runtime_unloads_after_save(
        &mut self,
        request: &bbb_server_persistence::Java26SaveRequest,
    ) {
        for level in self.levels.values_mut() {
            let written = request
                .entity_writes
                .iter()
                .filter(|write| write.dimension == *level.dimension)
                .map(|write| (write.chunk_x, write.chunk_z))
                .collect::<BTreeSet<_>>();
            // A written record must be overwritten once its entities leave.
            level.saved_entity_chunks.extend(written.iter().copied());
            if !level.chunk_lifecycle.storage_bound {
                continue;
            }
            let unloads = level
                .chunk_lifecycle
                .entity_unloads
                .iter()
                .copied()
                .collect::<Vec<_>>();
            for chunk in unloads {
                let stored = written.contains(&chunk);
                if !level.chunk_lifecycle.entities_loaded(chunk)
                    || (!stored
                        && (level.saved_entity_chunks.contains(&chunk)
                            || level.entities.is_chunk_occupied(chunk)))
                {
                    continue;
                }
                level.entities.unload_chunk_entities_after_save(chunk);
                level.chunk_lifecycle.entity_loads.remove(&chunk);
                if stored {
                    level.chunk_lifecycle.stored_entities.insert(chunk);
                }
                level.chunk_lifecycle.entity_unloads.remove(&chunk);
                level.saved_entity_chunks.remove(&chunk);
            }
            if !self.auto_save {
                continue;
            }
            let unloads = level
                .chunk_lifecycle
                .column_unloads
                .iter()
                .copied()
                .collect::<Vec<_>>();
            for chunk in unloads {
                let has_reload_backing = level.chunks.has_saved_column_input(chunk)
                    || (level.chunks.stored_column_reader().is_some()
                        && level.saved_chunks.contains(&chunk));
                if level.chunk_lifecycle.entity_loads.get(&chunk)
                    == Some(&EntityChunkLoadStatus::Pending)
                    || !has_reload_backing
                {
                    continue;
                }
                level.chunks.unload_saved_column(chunk.0, chunk.1);
                level.saved_tick_chunks.remove(&chunk);
                level.chunk_lifecycle.confirmed.remove(&chunk);
                level.chunk_lifecycle.column_unloads.remove(&chunk);
                level.chunk_lifecycle.evicted_columns.insert(chunk);
                level.chunk_lifecycle.settled = None;
            }
        }
    }
}

fn entity_input_error(error: impl std::fmt::Display) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("runtime entity load: {error}"),
    }
}

#[cfg(test)]
impl ServerWorld {
    /// Reads every stored column, entity, player, stat, advancement, and map
    /// record now, the way a world that opened its whole save at startup would
    /// hold them.
    pub(crate) fn read_every_stored_record_for_test(&mut self) {
        let player_uuids: Vec<_> = self.cold.stored_players.iter().copied().collect();
        for uuid in player_uuids {
            self.ensure_restored_player_loaded(uuid)
                .expect("stored playerdata should load cleanly");
        }
        let stat_uuids: Vec<_> = self.cold.stored_player_stats.iter().copied().collect();
        for uuid in stat_uuids {
            self.ensure_restored_player_stats_loaded(uuid)
                .expect("stored player stats should load cleanly");
        }
        let advancement_uuids: Vec<_> = self
            .cold
            .stored_player_advancements
            .iter()
            .copied()
            .collect();
        for uuid in advancement_uuids {
            self.ensure_restored_player_advancements_loaded(uuid)
                .expect("stored player advancements should load cleanly");
        }
        self.maps.load_all_stored_maps_for_test();
        for dimension in self.level_ids() {
            let level = self.level_mut_by_id(&dimension);
            let Some(reader) = level.chunks.stored_column_reader().cloned() else {
                continue;
            };
            let key = level.dimension.as_ref().clone();
            let records = level
                .chunks
                .stored_unread_columns_for_test()
                .into_iter()
                .map(|chunk| (chunk, reader.read_column(&key, chunk.0, chunk.1)))
                .collect::<Vec<_>>();
            for chunk in std::mem::take(&mut level.chunk_lifecycle.stored_entities) {
                level
                    .chunk_lifecycle
                    .entity_loads
                    .insert(chunk, EntityChunkLoadStatus::Pending);
            }
            self.complete_runtime_columns(&key, records);
            self.read_pending_entities_now(&dimension);
            let level = self.level_mut_by_id(&dimension);
            for &chunk in &level.saved_chunks {
                level
                    .chunk_lifecycle
                    .entity_loads
                    .entry(chunk)
                    .or_insert(EntityChunkLoadStatus::Loaded);
                level.chunk_lifecycle.entity_unloads.insert(chunk);
            }
            for chunk in self.level_by_id(&dimension).saved_chunks.clone() {
                if self
                    .level_by_id(&dimension)
                    .chunks
                    .is_ticking_full(chunk.0, chunk.1)
                {
                    self.register_chunk_tick_containers_by_key(&dimension, chunk);
                }
            }
            let mut events = Vec::new();
            self.apply_level_entity_lifecycle_by_key(&dimension, &mut events);
        }
    }
}
