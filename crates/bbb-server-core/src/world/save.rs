//! The save request a between-ticks barrier freezes for the storage layer,
//! and the acknowledgement that retires what reached disk.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use bbb_protocol::chunk_light::SavedLightSection;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::{
    Java26BelowZeroRetrogen, Java26BlendingData, Java26ChunkStatus, Java26ChunkWriteEpochs,
    Java26CreatedChunk, Java26EntityChunkWrite, Java26LegacyChunkFields, Java26LevelSaveState,
    Java26LevelSpawn, Java26SaveRequest, Java26SavedChunkTicket, Java26UpgradeData,
    Java26WorldBorderSettings, RegionCompression,
};
use bbb_server_types::chunk_data::{ChunkColumn, ChunkGenerationState, ChunkPreCarversState};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, Java26SavedTick};
use bbb_server_types::save::{Java26LevelKey, SaveError, SaveResult};
use rayon::prelude::*;

use super::chunk_lifecycle::EntityChunkLoadStatus;
use super::{LevelId, ServerLevel, ServerWorld};
use crate::block_entity::saved_record_with_resolver;
use crate::chunk::WholeRewriteIntent;
use crate::chunk_sender::ChunkKey;
use crate::registry_sync::SynchronizedRegistries;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachedUnloadColumn {
    pub dimension: Java26LevelKey,
    pub chunk: ChunkKey,
    pub generation_status: Option<Java26ChunkStatus>,
    pub had_derived_dirty: bool,
    pub saved_light_sections: Vec<SavedLightSection>,
    pub block_ticks: Vec<Java26SavedTick>,
    pub fluid_ticks: Vec<Java26SavedTick>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachedUnloadEntity {
    pub dimension: Java26LevelKey,
    pub chunk: ChunkKey,
    frozen_was_empty: bool,
    frozen_dirty_epoch: u64,
    frozen_entity_ids: Vec<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct DetachedUnloadBatch {
    pub columns: Vec<DetachedUnloadColumn>,
    pub entities: Vec<DetachedUnloadEntity>,
}

/// Lightweight chunk snapshot captured on the world tick thread so full
/// `NamedCompound` / `NbtSummaryValue` serialization runs off-thread on
/// `bbb-save-write`.
#[derive(Debug, Clone)]
pub struct DeferredChunkWrite {
    dimension: Java26LevelKey,
    chunk_x: i32,
    chunk_z: i32,
    game_time: i64,
    column: ChunkColumn,
    biome_names: Arc<[String]>,
    content_registry: Arc<ContentRegistry>,
    pre_carvers: Option<ChunkPreCarversState>,
    generation_state: Option<ChunkGenerationState>,
    saved_light_sections: Vec<SavedLightSection>,
    structures: Option<NbtSummaryValue>,
    proto_entities: Vec<Vec<NbtSummaryEntry>>,
    block_ticks: Vec<Java26SavedTick>,
    fluid_ticks: Vec<Java26SavedTick>,
    block_entities: Vec<Vec<NbtSummaryEntry>>,
    status: Java26ChunkStatus,
    blending_data: Option<Java26BlendingData>,
    upgrade_data: Option<Java26UpgradeData>,
    below_zero_retrogen: Option<Java26BelowZeroRetrogen>,
}

impl DeferredChunkWrite {
    /// Serializes this chunk snapshot into a complete whole-column NBT document.
    pub fn materialize(self) -> SaveResult<Java26CreatedChunk> {
        let biome_name = |id: i32| {
            usize::try_from(id)
                .ok()
                .and_then(|idx| self.biome_names.get(idx))
                .filter(|name| !name.is_empty())
                .cloned()
        };
        let mut document = match self.pre_carvers.as_ref() {
            Some(state) => bbb_server_persistence::pre_carvers_chunk_document_with_content_registry(
                &self.column,
                self.chunk_x,
                self.chunk_z,
                &biome_name,
                state,
                self.structures.as_ref(),
                &self.proto_entities,
                &self.content_registry,
            )?,
            None => bbb_server_persistence::generation_state_chunk_document_with_light_and_content_registry(
                &self.column,
                self.chunk_x,
                self.chunk_z,
                &biome_name,
                self.generation_state
                    .as_ref()
                    .expect("a loaded column retains its generation state"),
                self.structures.as_ref(),
                &self.proto_entities,
                &self.saved_light_sections,
                &self.content_registry,
            )?,
        };
        bbb_server_persistence::write_chunk_ticks(
            &mut document,
            self.game_time,
            &self.block_ticks,
            &self.fluid_ticks,
        );
        bbb_server_persistence::replace_block_entities(&mut document, self.block_entities);
        bbb_server_persistence::write_legacy_chunk_fields(
            &mut document,
            self.status,
            Java26LegacyChunkFields {
                blending_data: self.blending_data.as_ref(),
                upgrade_data: self.upgrade_data.as_ref(),
                below_zero_retrogen: self.below_zero_retrogen.as_ref(),
            },
        );
        Ok(Java26CreatedChunk {
            dimension: self.dimension,
            chunk_x: self.chunk_x,
            chunk_z: self.chunk_z,
            document,
        })
    }
}

/// Materializes deferred chunk snapshots into whole-column NBT documents,
/// fanning out across rayon when a batch contains multiple columns.
pub fn materialize_deferred_chunks(
    chunks: Vec<DeferredChunkWrite>,
) -> SaveResult<Vec<Java26CreatedChunk>> {
    if chunks.len() <= 1 {
        return chunks
            .into_iter()
            .map(DeferredChunkWrite::materialize)
            .collect();
    }
    chunks
        .into_par_iter()
        .map(DeferredChunkWrite::materialize)
        .collect()
}

fn biome_name_table(registries: &SynchronizedRegistries) -> Arc<[String]> {
    let len = registries
        .registry_len("minecraft:worldgen/biome")
        .unwrap_or(0);
    let mut names = Vec::with_capacity(len);
    for id in 0..len {
        names.push(
            registries
                .registry_element_id("minecraft:worldgen/biome", id as i32)
                .unwrap_or_default()
                .to_owned(),
        );
    }
    Arc::from(names)
}

#[derive(Default)]
pub(super) struct AutosaveStaging {
    pending_columns: VecDeque<(LevelId, ChunkKey)>,
    biome_names: Option<Arc<[String]>>,
    deferred_chunks: Vec<DeferredChunkWrite>,
    chunk_write_epochs: BTreeMap<(Java26LevelKey, i32, i32), Java26ChunkWriteEpochs>,
}

fn invalid(detail: String) -> SaveError {
    SaveError::InvalidNbt { path: None, detail }
}

fn packed_ticks(level: &ServerLevel, chunk: ChunkKey, game_time: i64) -> [Vec<Java26SavedTick>; 2] {
    [
        level.chunks.saved_ticks::<BlockTickType>(chunk, game_time),
        level.chunks.saved_ticks::<FluidTickType>(chunk, game_time),
    ]
}

fn acknowledge_saved_ticks(level: &mut ServerLevel, chunk: ChunkKey, has_ticks: bool) {
    if has_ticks {
        level.saved_tick_chunks.insert(chunk);
    } else {
        level.saved_tick_chunks.remove(&chunk);
    }
}

fn level_dirty_chunk_positions(level: &ServerLevel) -> BTreeSet<ChunkKey> {
    let mut positions: BTreeSet<ChunkKey> = level
        .chunks
        .save_dirty_chunks()
        .map(|(chunk, _)| chunk)
        .collect();
    positions.extend(level.chunks.chunks_with_scheduled_ticks());
    positions.extend(
        level
            .saved_tick_chunks
            .iter()
            .copied()
            .filter(|&chunk| level.chunks.ticks_registered(chunk)),
    );
    positions.extend(level.chunks.pending_initial_persistence());
    positions.extend(level.chunks.ordinary_derived_dirty_chunks());
    positions.extend(
        level
            .chunks
            .pending_whole_rewrites()
            .map(|intent| intent.chunk),
    );
    positions
}

#[allow(clippy::too_many_arguments)]
fn snapshot_level_chunk_deferred(
    level: &ServerLevel,
    biome_names: &Arc<[String]>,
    content_registry: &Arc<ContentRegistry>,
    resolver: &super::registry_runtime::PlayerItemStackResolver,
    game_time: i64,
    chunk_x: i32,
    chunk_z: i32,
    deferred_chunks: &mut Vec<DeferredChunkWrite>,
    chunk_write_epochs: &mut BTreeMap<(Java26LevelKey, i32, i32), Java26ChunkWriteEpochs>,
) -> SaveResult<()> {
    let chunk = (chunk_x, chunk_z);
    chunk_write_epochs.insert(
        (level.dimension.as_ref().clone(), chunk_x, chunk_z),
        Java26ChunkWriteEpochs {
            dirty_epoch: level.chunks.save_dirty_epoch(chunk),
            whole_rewrite_epoch: level.chunks.whole_rewrite_epoch(chunk),
            generation_status: level.chunks.generation_status(chunk_x, chunk_z),
        },
    );
    deferred_chunks.push(capture_deferred_chunk(
        level,
        biome_names,
        content_registry,
        resolver,
        game_time,
        chunk_x,
        chunk_z,
    )?);
    Ok(())
}

/// Captures a lightweight chunk snapshot without constructing the full
/// `NamedCompound` block/section NBT AST on the world tick thread.
fn capture_deferred_chunk(
    level: &ServerLevel,
    biome_names: &Arc<[String]>,
    content_registry: &Arc<ContentRegistry>,
    resolver: &super::registry_runtime::PlayerItemStackResolver,
    game_time: i64,
    chunk_x: i32,
    chunk_z: i32,
) -> SaveResult<DeferredChunkWrite> {
    let chunk = (chunk_x, chunk_z);
    level.chunks.validate_light_persistence(chunk)?;
    let column = level
        .chunks
        .column(chunk_x, chunk_z)
        .expect("a changed chunk is a loaded chunk")
        .clone();
    let structures = level
        .chunks
        .structure_component(chunk_x, chunk_z)
        .map(|s| s.to_nbt());
    let proto_entities = level.chunks.proto_entity_records(chunk_x, chunk_z).to_vec();
    let pre_carvers = level
        .chunks
        .pre_carvers_generation_state(chunk_x, chunk_z)
        .cloned();
    let (generation_state, saved_light_sections) = if pre_carvers.is_some() {
        (None, Vec::new())
    } else {
        (
            Some(
                level
                    .chunks
                    .generation_state(chunk_x, chunk_z)
                    .expect("a loaded column retains its generation state")
                    .clone(),
            ),
            level.chunks.saved_light_sections(chunk),
        )
    };
    let [block_ticks, fluid_ticks] = packed_ticks(level, chunk, game_time);
    let block_entities = chunk_block_entity_records(level, chunk, resolver)?;
    let status = level
        .chunks
        .generation_status(chunk_x, chunk_z)
        .expect("a loaded column retains its generation status");
    let legacy_work = level
        .chunks
        .pending_legacy_generation_work(chunk_x, chunk_z);
    Ok(DeferredChunkWrite {
        dimension: level.dimension.as_ref().clone(),
        chunk_x,
        chunk_z,
        game_time,
        column,
        biome_names: Arc::clone(biome_names),
        content_registry: Arc::clone(content_registry),
        pre_carvers,
        generation_state,
        saved_light_sections,
        structures,
        proto_entities,
        block_ticks,
        fluid_ticks,
        block_entities,
        status,
        blending_data: level.chunks.saved_blending_data(chunk_x, chunk_z).cloned(),
        upgrade_data: legacy_work.and_then(|work| work.upgrade_data()).cloned(),
        below_zero_retrogen: legacy_work
            .and_then(|work| work.below_zero_retrogen())
            .cloned(),
    })
}

/// Every block entity of a column being saved (`SerializableChunkData.java:357-363`),
/// merging preserved unmodeled compounds with modeled runtime block entities.
fn chunk_block_entity_records(
    level: &ServerLevel,
    chunk: ChunkKey,
    resolver: &super::registry_runtime::PlayerItemStackResolver,
) -> SaveResult<Vec<Vec<NbtSummaryEntry>>> {
    let mut records = level
        .chunks
        .packed_block_entities(chunk)
        .cloned()
        .unwrap_or_default();
    for block_entity in level.chunks.block_entities().chunk(chunk) {
        let pos = block_entity.block_pos();
        let record = saved_record_with_resolver(block_entity, resolver)
            .map_err(|error| invalid(error.to_string()))?;
        records.insert((pos.x, pos.y, pos.z), record);
    }
    Ok(records.into_values().collect())
}

impl ServerWorld {
    /// Projects the current tick-frozen state into a save request for
    /// `Java26WorldStorage::save_snapshot`. Callers may only call this between
    /// ticks; pending light checks are committed before the projection.
    /// `last_played` is epoch millis, `region_timestamp` epoch seconds.
    /// Fails closed when a live block-state id has no Vanilla 26.1 identity.
    pub fn save_request(
        &mut self,
        last_played: i64,
        region_timestamp: u32,
        region_compression: RegionCompression,
    ) -> SaveResult<Java26SaveRequest> {
        let (mut request, deferred) = self.save_request_scoped_deferred(
            last_played,
            region_timestamp,
            region_compression,
            false,
        )?;
        request.chunk_writes = materialize_deferred_chunks(deferred)?;
        Ok(request)
    }

    /// Deferred variant of [`Self::save_request`] that returns lightweight
    /// chunk snapshots so `NamedCompound` serialization can run on
    /// `bbb-save-write`.
    pub fn save_request_deferred(
        &mut self,
        last_played: i64,
        region_timestamp: u32,
        region_compression: RegionCompression,
    ) -> SaveResult<(Java26SaveRequest, Vec<DeferredChunkWrite>)> {
        self.save_request_scoped_deferred(last_played, region_timestamp, region_compression, false)
    }

    /// The save of only the columns and entity sections the runtime is
    /// unloading.
    pub fn runtime_unload_save_request(
        &mut self,
        region_timestamp: u32,
        region_compression: RegionCompression,
    ) -> SaveResult<Java26SaveRequest> {
        let (mut request, deferred) =
            self.save_request_scoped_deferred(0, region_timestamp, region_compression, true)?;
        request.chunk_writes = materialize_deferred_chunks(deferred)?;
        Ok(request)
    }

    /// Starts an amortized autosave cycle across all served levels.
    /// Call [`Self::step_autosave_snapshot`] each tick until it yields the
    /// completed [`Java26SaveRequest`].
    pub fn begin_autosave_snapshot(&mut self) -> SaveResult<()> {
        for level in self.levels.values_mut() {
            level.chunks.flush_live_relights()?;
        }
        let mut pending_columns = VecDeque::new();
        for (level_id, level) in self.levels.iter() {
            for chunk in level_dirty_chunk_positions(level) {
                if level.chunks.is_loaded(chunk.0, chunk.1) {
                    pending_columns.push_back((level_id, chunk));
                }
            }
        }
        self.cold.autosave_staging = Some(AutosaveStaging {
            pending_columns,
            biome_names: Some(biome_name_table(&self.registries)),
            deferred_chunks: Vec::new(),
            chunk_write_epochs: BTreeMap::new(),
        });
        Ok(())
    }

    pub fn has_pending_autosave_snapshot(&self) -> bool {
        self.cold.autosave_staging.is_some()
    }

    /// Snapshots at most `column_budget` dirty columns from the active
    /// amortized autosave cycle. When no queued columns remain and `finalize`
    /// is true, assembles and returns the complete [`Java26SaveRequest`].
    pub fn step_autosave_snapshot(
        &mut self,
        last_played: i64,
        region_timestamp: u32,
        region_compression: RegionCompression,
        column_budget: usize,
        finalize: bool,
    ) -> SaveResult<Option<Java26SaveRequest>> {
        let Some((mut request, deferred)) = self.step_autosave_snapshot_deferred(
            last_played,
            region_timestamp,
            region_compression,
            column_budget,
            finalize,
        )?
        else {
            return Ok(None);
        };
        request.chunk_writes = materialize_deferred_chunks(deferred)?;
        Ok(Some(request))
    }

    /// Deferred variant of [`Self::step_autosave_snapshot`] that hands off
    /// lightweight chunk snapshots for off-thread NBT serialization.
    pub fn step_autosave_snapshot_deferred(
        &mut self,
        last_played: i64,
        region_timestamp: u32,
        region_compression: RegionCompression,
        column_budget: usize,
        finalize: bool,
    ) -> SaveResult<Option<(Java26SaveRequest, Vec<DeferredChunkWrite>)>> {
        let Some(mut staging) = self.cold.autosave_staging.take() else {
            return Ok(None);
        };
        let column_budget = column_budget.max(1);
        if !staging.pending_columns.is_empty() {
            for level in self.levels.values_mut() {
                level.chunks.flush_live_relights()?;
            }
            self.finalize_structure_mobs();
            let resolver = self.item_stack_resolver();
            let content_registry = Arc::clone(self.plugins().content_registry());
            let biome_names = staging
                .biome_names
                .get_or_insert_with(|| biome_name_table(&self.registries))
                .clone();
            let game_time = self.game_time;
            let mut processed = 0usize;
            while processed < column_budget {
                let Some((level_id, chunk)) = staging.pending_columns.pop_front() else {
                    break;
                };
                let level = self.level_by_id(&level_id);
                if !level.chunks.is_loaded(chunk.0, chunk.1) {
                    continue;
                }
                snapshot_level_chunk_deferred(
                    level,
                    &biome_names,
                    &content_registry,
                    &resolver,
                    game_time,
                    chunk.0,
                    chunk.1,
                    &mut staging.deferred_chunks,
                    &mut staging.chunk_write_epochs,
                )?;
                processed += 1;
            }
        }
        if staging.pending_columns.is_empty() && finalize {
            let request = self.build_save_request_from_chunks(
                last_played,
                region_timestamp,
                region_compression,
                false,
                Vec::new(),
                staging.chunk_write_epochs,
            )?;
            return Ok(Some((request, staging.deferred_chunks)));
        }
        self.cold.autosave_staging = Some(staging);
        Ok(None)
    }

    /// Freezes up to `batch_limit` unloading columns and entity chunks per level
    /// that do not currently have an in-flight read, leaving all remaining
    /// pending unloads intact while capturing epoch state for a detached ack.
    pub fn runtime_unload_save_request_batch(
        &mut self,
        region_timestamp: u32,
        region_compression: RegionCompression,
        batch_limit: usize,
        in_flight_reads: &BTreeSet<(Java26LevelKey, ChunkKey)>,
    ) -> SaveResult<Option<(Java26SaveRequest, DetachedUnloadBatch)>> {
        let Some((mut request, deferred, batch)) = self
            .runtime_unload_save_request_batch_deferred(
                region_timestamp,
                region_compression,
                batch_limit,
                in_flight_reads,
            )?
        else {
            return Ok(None);
        };
        request.chunk_writes = materialize_deferred_chunks(deferred)?;
        Ok(Some((request, batch)))
    }

    /// Deferred variant of [`Self::runtime_unload_save_request_batch`] that
    /// hands off lightweight chunk snapshots for off-thread NBT serialization.
    pub fn runtime_unload_save_request_batch_deferred(
        &mut self,
        region_timestamp: u32,
        region_compression: RegionCompression,
        batch_limit: usize,
        in_flight_reads: &BTreeSet<(Java26LevelKey, ChunkKey)>,
    ) -> SaveResult<
        Option<(
            Java26SaveRequest,
            Vec<DeferredChunkWrite>,
            DetachedUnloadBatch,
        )>,
    > {
        if batch_limit == 0 {
            return Ok(None);
        }
        let mut held_per_level: Vec<(LevelId, BTreeSet<ChunkKey>, BTreeSet<ChunkKey>)> = Vec::new();
        let mut selected_columns: Vec<(LevelId, Java26LevelKey, ChunkKey)> = Vec::new();
        let mut selected_entities: Vec<(LevelId, Java26LevelKey, ChunkKey)> = Vec::new();

        for (level_id, level) in self.levels.iter_mut() {
            if !level.chunk_lifecycle.storage_bound {
                continue;
            }
            let dim_key = level.dimension.as_ref().clone();
            let all_columns = std::mem::take(&mut level.chunk_lifecycle.column_unloads);
            let mut active_columns = BTreeSet::new();
            let mut held_columns = BTreeSet::new();
            for chunk in all_columns {
                if self.auto_save
                    && active_columns.len() < batch_limit
                    && !in_flight_reads.contains(&(dim_key.clone(), chunk))
                {
                    active_columns.insert(chunk);
                    selected_columns.push((level_id, dim_key.clone(), chunk));
                } else {
                    held_columns.insert(chunk);
                }
            }
            level.chunk_lifecycle.column_unloads = active_columns;

            let all_entities = std::mem::take(&mut level.chunk_lifecycle.entity_unloads);
            let mut active_entities = BTreeSet::new();
            let mut held_entities = BTreeSet::new();
            for chunk in all_entities {
                if active_entities.len() < batch_limit
                    && level.chunk_lifecycle.entities_loaded(chunk)
                    && !in_flight_reads.contains(&(dim_key.clone(), chunk))
                {
                    active_entities.insert(chunk);
                    selected_entities.push((level_id, dim_key.clone(), chunk));
                } else {
                    held_entities.insert(chunk);
                }
            }
            level.chunk_lifecycle.entity_unloads = active_entities;
            held_per_level.push((level_id, held_columns, held_entities));
        }

        if selected_columns.is_empty() && selected_entities.is_empty() {
            for (level_id, held_columns, held_entities) in held_per_level {
                let level = self.level_mut_by_id(&level_id);
                level.chunk_lifecycle.column_unloads.extend(held_columns);
                level.chunk_lifecycle.entity_unloads.extend(held_entities);
            }
            return Ok(None);
        }

        let request_result =
            self.save_request_scoped_deferred(0, region_timestamp, region_compression, true);
        let batch = if request_result.is_ok() {
            let columns = selected_columns
                .into_iter()
                .map(|(level_id, dimension, chunk)| {
                    let level = self.level_by_id(&level_id);
                    let had_derived_dirty = level.chunks.is_ordinary_derived_dirty(chunk);
                    let saved_light_sections = if had_derived_dirty {
                        level.chunks.saved_light_sections(chunk)
                    } else {
                        Vec::new()
                    };
                    DetachedUnloadColumn {
                        dimension,
                        chunk,
                        generation_status: level.chunks.generation_status(chunk.0, chunk.1),
                        had_derived_dirty,
                        saved_light_sections,
                        block_ticks: level.chunks.saved_ticks::<BlockTickType>(chunk, 0),
                        fluid_ticks: level.chunks.saved_ticks::<FluidTickType>(chunk, 0),
                    }
                })
                .collect();
            let excluded =
                (!selected_entities.is_empty()).then(|| self.player_ender_pearl_save_exclusions());
            let entities = selected_entities
                .into_iter()
                .map(|(level_id, dimension, chunk)| {
                    let level = self.level_by_id(&level_id);
                    let frozen_was_empty = !level.entities.is_chunk_occupied(chunk);
                    let frozen_dirty_epoch = level.entities.chunk_dirty_epoch(chunk);
                    let mut frozen_entity_ids = Vec::new();
                    if !frozen_was_empty {
                        let excluded_set = excluded.as_ref();
                        for entity_id in level.entities.iter_chunk_section_ids(chunk) {
                            if let Some(entity) = level.entities.get(entity_id) {
                                if excluded_set.is_some_and(|set| set.contains(&entity.uuid())) {
                                    continue;
                                }
                            }
                            frozen_entity_ids.push(entity_id);
                        }
                        frozen_entity_ids.sort_unstable();
                    }
                    DetachedUnloadEntity {
                        dimension,
                        chunk,
                        frozen_was_empty,
                        frozen_dirty_epoch,
                        frozen_entity_ids,
                    }
                })
                .collect();
            Some(DetachedUnloadBatch { columns, entities })
        } else {
            None
        };

        for (level_id, held_columns, held_entities) in held_per_level {
            let level = self.level_mut_by_id(&level_id);
            level.chunk_lifecycle.column_unloads.extend(held_columns);
            level.chunk_lifecycle.entity_unloads.extend(held_entities);
        }

        let (request, deferred) = request_result?;
        Ok(Some((request, deferred, batch.expect("batch built on Ok"))))
    }

    fn save_request_scoped_deferred(
        &mut self,
        last_played: i64,
        region_timestamp: u32,
        region_compression: RegionCompression,
        unload_only: bool,
    ) -> SaveResult<(Java26SaveRequest, Vec<DeferredChunkWrite>)> {
        if !unload_only {
            self.cold.autosave_staging = None;
        }
        for level in self.levels.values_mut() {
            level.chunks.flush_live_relights()?;
        }
        self.finalize_structure_mobs();
        let resolver = self.item_stack_resolver();
        let content_registry = Arc::clone(self.plugins().content_registry());
        let biome_names = biome_name_table(&self.registries);
        let mut deferred_chunks = Vec::new();
        let mut chunk_write_epochs = BTreeMap::new();
        if !unload_only || self.auto_save {
            for level in self.levels.values() {
                self.level_chunk_saves_deferred(
                    level,
                    unload_only,
                    &biome_names,
                    &content_registry,
                    &resolver,
                    &mut deferred_chunks,
                    &mut chunk_write_epochs,
                )?;
            }
        }
        let request = self.build_save_request_from_chunks(
            last_played,
            region_timestamp,
            region_compression,
            unload_only,
            Vec::new(),
            chunk_write_epochs,
        )?;
        Ok((request, deferred_chunks))
    }

    fn build_save_request_from_chunks(
        &mut self,
        last_played: i64,
        region_timestamp: u32,
        region_compression: RegionCompression,
        unload_only: bool,
        chunk_writes: Vec<Java26CreatedChunk>,
        chunk_write_epochs: BTreeMap<(Java26LevelKey, i32, i32), Java26ChunkWriteEpochs>,
    ) -> SaveResult<Java26SaveRequest> {
        if !unload_only {
            self.apply_all_framed_map_removals();
        }
        for level in self.levels.values_mut() {
            level.chunks.flush_live_relights()?;
        }
        self.finalize_structure_mobs();
        if !unload_only {
            self.materialize_saved_merchant_offers()?;
        }
        let resolver = self.item_stack_resolver();
        let game_time = self.game_time;
        // PlayerList.java:619-623 saves every online player; a profile that
        // departed in this run is written from its frozen snapshot.
        let mut player_patches = Vec::new();
        let mut player_advancements = BTreeMap::new();
        if !unload_only {
            for player in self.levels.players() {
                player_patches.push(self.player_save_patch(player)?);
            }
            player_patches.extend(
                self.cold
                    .departed_player_snapshots
                    .values()
                    .map(|snapshot| snapshot.patch.clone()),
            );
            // PlayerList.java:286-297.
            if self.has_advancement_manager() {
                player_advancements = self
                    .levels
                    .players()
                    .map(|player| {
                        (
                            player.profile.uuid,
                            player
                                .cold
                                .advancements
                                .to_data(&self.cold.advancement_tree),
                        )
                    })
                    .chain(
                        self.cold
                            .departed_player_snapshots
                            .iter()
                            .map(|(uuid, snapshot)| (*uuid, snapshot.advancements.clone())),
                    )
                    .collect();
            }
        }
        let map_writes = self
            .maps
            .dirty_persistent_snapshots()
            .into_iter()
            .map(|snapshot| (snapshot.map_id, snapshot.data.to_saved()))
            .collect();
        let world_borders = self
            .levels
            .values()
            .map(|level| {
                let border = level.world_border;
                (
                    level.dimension.as_ref().clone(),
                    Java26WorldBorderSettings {
                        center_x: border.center_x(),
                        center_z: border.center_z(),
                        damage_per_block: border.damage_per_block(),
                        safe_zone: border.safe_zone(),
                        warning_blocks: border.warning_blocks(),
                        warning_time: border.warning_time(),
                        size: border.size_at(game_time),
                        lerp_time: border.remaining_lerp_time(game_time),
                        lerp_target: border.lerp_target(),
                    },
                )
            })
            .collect();
        let chunk_tickets = self
            .levels
            .values()
            .map(|level| {
                (
                    level.dimension.as_ref().clone(),
                    level
                        .saved_chunk_tickets
                        .iter()
                        .map(|ticket| Java26SavedChunkTicket {
                            chunk_x: ticket.chunk.0,
                            chunk_z: ticket.chunk.1,
                            level: ticket.level,
                            ticks_left: ticket.ticks_left,
                        })
                        .collect(),
                )
            })
            .collect();
        let mut request = Java26SaveRequest {
            data_packs: self
                .cold
                .config
                .datapacks
                .as_ref()
                .map(|startup| startup.selection.data_packs().clone()),
            poi_edits: self
                .levels
                .values()
                .flat_map(|level| level.chunks.poi().save_edits(&level.dimension))
                .collect(),
            level_state: Java26LevelSaveState {
                game_time,
                last_played,
                game_type: self.cold.default_game_type.id(),
                initialized: true,
                difficulty: self.admin.settings().difficulty,
                difficulty_locked: self.cold.difficulty_locked,
                // PrimaryLevelData.java:132.
                spawn: Java26LevelSpawn::new(
                    self.level_key(self.cold.default_spawn.dimension)
                        .resource_id()
                        .to_string(),
                    [
                        (self.cold.default_spawn.position.x - 0.5).round() as i32,
                        self.cold.default_spawn.position.y.round() as i32,
                        (self.cold.default_spawn.position.z - 0.5).round() as i32,
                    ],
                    self.cold.default_spawn.rotation.y_rot,
                    self.cold.default_spawn.rotation.x_rot,
                ),
            },
            region_timestamp,
            region_compression,
            player_patches,
            // A departed full patch already carries its stats counter.
            departed_player_stats: BTreeMap::new(),
            player_advancements,
            chunk_writes,
            chunk_write_epochs,
            entity_writes: self.entity_writes(unload_only)?,
            // GameRules.java:99-114: every enabled rule, the whole file.
            game_rules: self
                .admin
                .game_rule_values()
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
            world_clocks: self.world_clock_snapshot()?,
            map_writes,
            map_last_id: self.maps.dirty_index(),
            world_borders,
            raids: self
                .levels
                .values()
                .map(|level| (level.dimension.as_ref().clone(), level.raids.to_saved()))
                .collect(),
            ender_dragon_fights: self
                .levels
                .values()
                .filter_map(|level| {
                    let fight = level.dragon_fight.as_ref()?;
                    Some((level.dimension.as_ref().clone(), fight.saved().clone()))
                })
                .collect(),
            chunk_tickets,
            // Untouched sequences are also legal saved data.
            random_sequences: self.cold.random_sequences.borrow().saved(),
            scheduled_events: self.cold.scheduled_events.pack(),
            stopwatches: self.cold.stopwatches.pack(),
            wandering_trader: self.cold.wandering_trader.data,
            weather: self.weather,
            scoreboard: self.scoreboard.pack(),
            custom_boss_events: self.cold.boss_events.persistent_snapshot(),
            command_storage_writes: self.cold.command_storage.persistent_snapshot(),
            item_stack_resolver: bbb_server_persistence::SaveHolderResolver::new(Arc::new(
                resolver,
            )),
        };
        if unload_only {
            request.poi_edits.retain(|edit| {
                self.auto_save
                    && self
                        .level_by_id(&self.require_level_id(&edit.dimension))
                        .chunk_lifecycle
                        .column_unloads
                        .contains(&(edit.chunk_x, edit.chunk_z))
            });
            request.map_writes.clear();
            request.map_last_id = None;
            request.command_storage_writes.clear();
        }
        Ok(request)
    }

    /// One level's dirty and newly created lightweight chunk snapshots.
    #[allow(clippy::too_many_arguments)]
    fn level_chunk_saves_deferred(
        &self,
        level: &ServerLevel,
        unload_only: bool,
        biome_names: &Arc<[String]>,
        content_registry: &Arc<ContentRegistry>,
        resolver: &super::registry_runtime::PlayerItemStackResolver,
        deferred_chunks: &mut Vec<DeferredChunkWrite>,
        chunk_write_epochs: &mut BTreeMap<(Java26LevelKey, i32, i32), Java26ChunkWriteEpochs>,
    ) -> SaveResult<()> {
        let game_time = self.game_time;
        let in_scope =
            |chunk: ChunkKey| !unload_only || level.chunk_lifecycle.column_unloads.contains(&chunk);
        for chunk in level_dirty_chunk_positions(level) {
            let (chunk_x, chunk_z) = chunk;
            if !in_scope(chunk) || !level.chunks.is_loaded(chunk_x, chunk_z) {
                continue;
            }
            snapshot_level_chunk_deferred(
                level,
                biome_names,
                content_registry,
                resolver,
                game_time,
                chunk_x,
                chunk_z,
                deferred_chunks,
                chunk_write_epochs,
            )?;
        }
        Ok(())
    }

    /// Retires first-write intent only after the exact frozen request reached
    /// storage. The runtime calls this at the same between-ticks barrier that
    /// produced `request`, with no intervening world mutation.
    pub fn acknowledge_save_success(&mut self, request: &Java26SaveRequest) {
        self.acknowledge_checked_save_intent(request);
        for write in &request.chunk_writes {
            let level = self.level_mut_by_id(&self.require_level_id(&write.dimension));
            let chunk = (write.chunk_x, write.chunk_z);
            level.saved_chunks.insert(chunk);
            level.chunks.mark_persisted(write.chunk_x, write.chunk_z);
            level.chunks.acknowledge_full_chunk_save(chunk);
        }
        self.finish_runtime_unloads_after_save(request);
    }

    /// Acknowledges a save that was written while ticks kept running. Any
    /// intent whose live state still matches the epochs and generation status
    /// captured when `request` was frozen is retired, including first-write
    /// `pending_initial_persistence` for newly generated chunks whose status
    /// has not advanced since freezing. Unloading still waits for the next
    /// barrier save, and the runtime starts no unload save while a detached
    /// save is in flight.
    pub fn acknowledge_detached_save(&mut self, request: &Java26SaveRequest) {
        self.acknowledge_checked_save_intent(request);
        for write in &request.chunk_writes {
            let chunk = (write.chunk_x, write.chunk_z);
            let epochs = request
                .chunk_write_epochs
                .get(&(write.dimension.clone(), write.chunk_x, write.chunk_z))
                .copied();
            let level = self.level_mut_by_id(&self.require_level_id(&write.dimension));
            level.saved_chunks.insert(chunk);
            if level.chunks.is_loaded(write.chunk_x, write.chunk_z)
                && epochs.is_some_and(|epochs| {
                    level.chunks.generation_status(write.chunk_x, write.chunk_z)
                        == epochs.generation_status
                })
            {
                level.chunks.mark_persisted(write.chunk_x, write.chunk_z);
            }
        }
        for write in &request.entity_writes {
            let level = self.level_mut_by_id(&self.require_level_id(&write.dimension));
            level
                .saved_entity_chunks
                .insert((write.chunk_x, write.chunk_z));
        }
    }

    /// Acknowledges a detached `ChunkUnload` save that ran off-thread while ticks
    /// kept advancing. Only columns and entity sections whose live epochs,
    /// scheduled ticks, derived lighting, and entity state still match what was
    /// frozen (and that still lack an in-flight read) are retired from memory.
    pub fn acknowledge_detached_unload(
        &mut self,
        request: &Java26SaveRequest,
        batch: &DetachedUnloadBatch,
        in_flight_reads: &BTreeSet<(Java26LevelKey, ChunkKey)>,
    ) {
        self.acknowledge_checked_save_intent(request);
        let derived_snapshots: rustc_hash::FxHashMap<
            (&Java26LevelKey, i32, i32),
            &DetachedUnloadColumn,
        > = batch
            .columns
            .iter()
            .filter(|col| col.had_derived_dirty)
            .map(|col| ((&col.dimension, col.chunk.0, col.chunk.1), col))
            .collect();
        for write in &request.chunk_writes {
            let chunk = (write.chunk_x, write.chunk_z);
            let epochs = request
                .chunk_write_epochs
                .get(&(write.dimension.clone(), write.chunk_x, write.chunk_z))
                .copied();
            let level = self.level_mut_by_id(&self.require_level_id(&write.dimension));
            level.saved_chunks.insert(chunk);
            let status_matches = level.chunks.is_loaded(write.chunk_x, write.chunk_z)
                && epochs.is_some_and(|epochs| {
                    level.chunks.generation_status(write.chunk_x, write.chunk_z)
                        == epochs.generation_status
                });
            if status_matches {
                level.chunks.mark_persisted(write.chunk_x, write.chunk_z);
                if let Some(col) =
                    derived_snapshots.get(&(&write.dimension, write.chunk_x, write.chunk_z))
                {
                    if level.chunks.save_dirty_epoch(chunk).is_none()
                        && level.chunks.whole_rewrite_epoch(chunk).is_none()
                        && level.chunks.saved_light_sections(chunk) == col.saved_light_sections
                    {
                        level.chunks.acknowledge_full_chunk_save(chunk);
                    }
                }
            }
        }

        if !batch.entities.is_empty() {
            let written_entity_chunks: rustc_hash::FxHashSet<(&Java26LevelKey, ChunkKey)> = request
                .entity_writes
                .iter()
                .map(|write| (&write.dimension, (write.chunk_x, write.chunk_z)))
                .collect();
            let excluded = self.player_ender_pearl_save_exclusions();
            let mut live_ids = Vec::new();
            for entity_chunk in &batch.entities {
                let chunk = entity_chunk.chunk;
                let stored = written_entity_chunks.contains(&(&entity_chunk.dimension, chunk));
                let level_id = self.require_level_id(&entity_chunk.dimension);
                let level = self.level_mut_by_id(&level_id);
                if stored {
                    level.saved_entity_chunks.insert(chunk);
                }
                if (!in_flight_reads.is_empty()
                    && in_flight_reads.contains(&(entity_chunk.dimension.clone(), chunk)))
                    || !level.chunk_lifecycle.storage_bound
                    || !level.chunk_lifecycle.entity_unloads.contains(&chunk)
                    || !level.chunk_lifecycle.entities_loaded(chunk)
                {
                    continue;
                }
                let entities_unchanged = if stored {
                    if entity_chunk.frozen_was_empty && !level.entities.is_chunk_occupied(chunk) {
                        true
                    } else if level.entities.chunk_dirty_epoch(chunk)
                        != entity_chunk.frozen_dirty_epoch
                    {
                        false
                    } else {
                        // The O(1) dirty epoch already guarantees no entity in this
                        // chunk was added, removed, moved, or mutated; check only
                        // that online-player ender-pearl exclusions still match.
                        live_ids.clear();
                        if level.entities.is_chunk_occupied(chunk) {
                            for entity_id in level.entities.iter_chunk_section_ids(chunk) {
                                if let Some(entity) = level.entities.get(entity_id) {
                                    if excluded.contains(&entity.uuid()) {
                                        continue;
                                    }
                                }
                                live_ids.push(entity_id);
                            }
                            live_ids.sort_unstable();
                        }
                        live_ids == entity_chunk.frozen_entity_ids
                    }
                } else {
                    !level.saved_entity_chunks.contains(&chunk)
                        && !level.entities.is_chunk_occupied(chunk)
                };
                if !entities_unchanged {
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
        }

        if !self.auto_save {
            return;
        }
        for col in &batch.columns {
            let chunk = col.chunk;
            if !in_flight_reads.is_empty()
                && in_flight_reads.contains(&(col.dimension.clone(), chunk))
            {
                continue;
            }
            let level_id = self.require_level_id(&col.dimension);
            let level = self.level_mut_by_id(&level_id);
            if !level.chunk_lifecycle.storage_bound
                || !level.chunk_lifecycle.column_unloads.contains(&chunk)
                || !level.chunks.is_loaded(chunk.0, chunk.1)
                || level.chunk_lifecycle.entity_loads.get(&chunk)
                    == Some(&EntityChunkLoadStatus::Pending)
            {
                continue;
            }
            let has_reload_backing = level.chunks.has_saved_column_input(chunk)
                || (level.chunks.stored_column_reader().is_some()
                    && level.saved_chunks.contains(&chunk));
            if !has_reload_backing {
                continue;
            }
            if level.chunks.save_dirty_epoch(chunk).is_some()
                || level.chunks.whole_rewrite_epoch(chunk).is_some()
                || level.chunks.is_pending_initial_persistence(chunk)
                || level.chunks.generation_status(chunk.0, chunk.1) != col.generation_status
                || level.chunks.is_ordinary_derived_dirty(chunk)
                || level.chunks.saved_ticks::<BlockTickType>(chunk, 0) != col.block_ticks
                || level.chunks.saved_ticks::<FluidTickType>(chunk, 0) != col.fluid_ticks
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

    fn acknowledge_checked_save_intent(&mut self, request: &Java26SaveRequest) {
        for edit in &request.poi_edits {
            self.level_mut_by_id(&self.require_level_id(&edit.dimension))
                .chunks
                .poi_mut()
                .acknowledge_save((edit.chunk_x, edit.chunk_z), edit.epoch);
        }
        for write in &request.chunk_writes {
            let has_ticks = write.document.entries().iter().any(|entry| {
                (entry.name == "block_ticks" || entry.name == "fluid_ticks")
                    && matches!(&entry.value, NbtSummaryValue::List(ticks) if !ticks.is_empty())
            });
            let chunk = (write.chunk_x, write.chunk_z);
            let epochs = request
                .chunk_write_epochs
                .get(&(write.dimension.clone(), write.chunk_x, write.chunk_z))
                .copied();
            let level = self.level_mut_by_id(&self.require_level_id(&write.dimension));
            acknowledge_saved_ticks(level, chunk, has_ticks);
            if let Some(epoch) = epochs.and_then(|epochs| epochs.whole_rewrite_epoch) {
                let _ = level
                    .chunks
                    .acknowledge_whole_rewrite(WholeRewriteIntent { chunk, epoch });
            }
            if let Some(epoch) = epochs.and_then(|epochs| epochs.dirty_epoch) {
                level.chunks.acknowledge_save_dirty(chunk, epoch);
            }
        }
        for snapshot in self.maps.dirty_persistent_snapshots() {
            if request.map_writes.get(&snapshot.map_id) == Some(&snapshot.data.to_saved()) {
                let _ = self
                    .maps
                    .acknowledge_persistent_snapshot(snapshot.map_id, snapshot.epoch);
            }
        }
        self.maps.acknowledge_index(request.map_last_id);
        self.cold
            .command_storage
            .mark_persisted(&request.command_storage_writes);
    }

    /// `EntityStorage.storeEntities` for every chunk whose entity set this
    /// world owns, plus every chunk the save held entities for, so a chunk
    /// whose entities all left is written empty.
    fn entity_writes(&self, unload_only: bool) -> SaveResult<Vec<Java26EntityChunkWrite>> {
        let mut writes = Vec::new();
        let excluded = self.player_ender_pearl_save_exclusions();
        for level in self.levels.values() {
            let mut chunks: BTreeSet<ChunkKey> = level.entities.iter_occupied_chunks().collect();
            chunks.extend(level.saved_entity_chunks.iter().copied());
            for chunk in chunks {
                if unload_only && !level.chunk_lifecycle.entity_unloads.contains(&chunk) {
                    continue;
                }
                if level.chunk_lifecycle.entities_pending(chunk) {
                    continue;
                }
                let data = crate::entity::save::write_entity_chunk_excluding(
                    &level.entities,
                    chunk,
                    &excluded,
                )
                .map_err(|error| {
                    invalid(format!("entity chunk ({},{}): {error}", chunk.0, chunk.1))
                })?;
                writes.push(Java26EntityChunkWrite {
                    dimension: level.dimension.as_ref().clone(),
                    chunk_x: chunk.0,
                    chunk_z: chunk.1,
                    document: data,
                });
            }
        }
        Ok(writes)
    }
}

#[cfg(test)]
mod tests;
