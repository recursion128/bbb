//! Synchronous FULL preparation and publication of generated entities.

use crate::chunk::ChunkStore;
use crate::chunk_sender::ChunkKey;
use crate::entity::save::read_entity_chunk_with_context;
use crate::entity::EntityStore;
use crate::world::natural_spawn::chunk_generation::{OriginalMobInputs, OriginalMobWorldInputs};
use crate::world::registry_runtime::EntityNbtReadSnapshot;
use crate::world::{LevelId, ServerWorld};
use bbb_protocol::NbtSummaryEntry;
use bbb_server_persistence::entity_schema::entity_chunk_document;
use bbb_server_persistence::{Java26ChunkStatus, Java26EntityChunkData};
use bbb_server_types::save::SaveResult;

pub(in crate::world) struct PreparedWorldgenEntityBatch {
    documents: Vec<Java26EntityChunkData>,
    input: EntityNbtReadSnapshot,
}

#[cfg(test)]
pub(in crate::world) fn worldgen_entity_document(
    chunk: ChunkKey,
    records: Vec<Vec<NbtSummaryEntry>>,
) -> SaveResult<Java26EntityChunkData> {
    worldgen_entity_document_with_registry(
        chunk,
        records,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(in crate::world) fn worldgen_entity_document_with_registry(
    chunk: ChunkKey,
    records: Vec<Vec<NbtSummaryEntry>>,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> SaveResult<Java26EntityChunkData> {
    let document = entity_chunk_document(chunk.0, chunk.1, records);
    Java26EntityChunkData::parse_with_content_registry(document, chunk.0, chunk.1, content_registry)
}

pub(in crate::world) fn prepare_worldgen_entity_batch(
    chunks: &mut ChunkStore,
    batch: &[ChunkKey],
    input: EntityNbtReadSnapshot,
) -> SaveResult<PreparedWorldgenEntityBatch> {
    let mut documents = Vec::new();
    for &chunk in batch {
        let records = chunks.take_published_worldgen_entity_records(chunk.0, chunk.1);
        if !records.is_empty() {
            documents.push(worldgen_entity_document_with_registry(
                chunk,
                records,
                input.content_registry(),
            )?);
        }
    }
    Ok(PreparedWorldgenEntityBatch { documents, input })
}

// ChunkStatusTasks.java:176-210: every ProtoChunk entity loads before the LevelChunk publishes.
pub(in crate::world) fn install_worldgen_entities(
    entities: &mut EntityStore,
    prepared: PreparedWorldgenEntityBatch,
) -> bool {
    if prepared.documents.is_empty() {
        return false;
    }
    let context = prepared.input.context();
    for document in &prepared.documents {
        let _ = read_entity_chunk_with_context(entities, document, context);
    }
    true
}

fn full_chunks_served(chunks: &ChunkStore, targets: &[ChunkKey]) -> bool {
    targets.iter().all(|&(x, z)| {
        chunks.validate_served_full_column(x, z).is_ok()
            && chunks.published_worldgen_entity_records(x, z).is_empty()
    })
}

pub(in crate::world) fn prepare_full_chunk_columns(
    chunks: &mut ChunkStore,
    originals: &OriginalMobInputs<'_>,
    targets: &[ChunkKey],
    input: &EntityNbtReadSnapshot,
) -> SaveResult<Option<PreparedWorldgenEntityBatch>> {
    if full_chunks_served(chunks, targets) {
        return Ok(None);
    }
    chunks.flush_live_relights()?;
    if full_chunks_served(chunks, targets) {
        return Ok(None);
    }
    originals.prepare(chunks, targets)?;
    originals.finalize_structure_mobs(chunks, input);
    prepare_worldgen_entity_batch(chunks, targets, input.clone()).map(Some)
}

pub(in crate::world) struct FullChunkPreparationInputs<'a, 'w> {
    pub(in crate::world) originals: OriginalMobWorldInputs<'w>,
    pub(in crate::world) entity_nbt: &'a EntityNbtReadSnapshot,
}

pub(in crate::world) struct FullChunkPreparation<'a> {
    pub(in crate::world) originals: OriginalMobInputs<'a>,
    pub(in crate::world) entity_nbt: &'a EntityNbtReadSnapshot,
    pub(in crate::world) lifecycle: &'a mut super::ChunkLifecycle,
    pub(in crate::world) saved_chunks: &'a mut std::collections::BTreeSet<ChunkKey>,
    pub(in crate::world) saved_tick_chunks: &'a mut std::collections::BTreeSet<ChunkKey>,
}

impl FullChunkPreparation<'_> {
    pub(in crate::world) fn reborrow(&mut self) -> FullChunkPreparation<'_> {
        FullChunkPreparation {
            originals: self.originals.clone(),
            entity_nbt: self.entity_nbt,
            lifecycle: self.lifecycle,
            saved_chunks: self.saved_chunks,
            saved_tick_chunks: self.saved_tick_chunks,
        }
    }

    /// Whether a live write may land in `chunk`, loading it the way vanilla's
    /// `getChunk` does where that costs no generation stage: a retained saved
    /// column is read back, a saved FULL one is published, and an ordinary
    /// (flat, void or debug) column, which reads no neighbour, generates in
    /// place. A noise column that still
    /// needs generation is not loaded: the write is refused the way vanilla's
    /// guarded writers skip an unloaded position, and an UNKNOWN ticket queues
    /// the generation instead of running it on the world thread.
    pub(in crate::world) fn live_write_loaded(
        &mut self,
        chunks: &mut ChunkStore,
        chunk: ChunkKey,
        game_time: i64,
        pending: &mut crate::world::simulation_level::pending_spawns::PendingEntitySpawns,
    ) -> bool {
        let full = chunks.is_ticking_full(chunk.0, chunk.1);
        if !full || self.lifecycle.status(chunk) < super::FullChunkStatus::Full {
            self.lifecycle.add_unknown(chunk, 33);
        }
        if full {
            return true;
        }
        let loaded = if chunks.has_saved_column_input(chunk) {
            chunks.restore_saved_column_input(chunk).map(|_| ())
        } else if chunks.is_stored_readable(chunk) {
            chunks.read_stored_column(chunk)
        } else if !chunks.has_noise_source()
            || chunks.is_storage_backed_noise_full(chunk.0, chunk.1)
        {
            prepare_full_chunk_columns(chunks, &self.originals, &[chunk], self.entity_nbt)
                .map(|batch| pending.worldgen_entities.extend(batch))
        } else {
            Ok(())
        };
        super::cold_columns::reconcile_saved_column_accesses(
            chunks,
            self.lifecycle,
            self.saved_chunks,
            self.saved_tick_chunks,
            game_time,
        );
        if let Err(error) = loaded {
            tracing::warn!("column {chunk:?} for a live write: {error}");
        }
        chunks.is_ticking_full(chunk.0, chunk.1)
    }

    /// Synchronously prepares every target column to `FULL` (including
    /// ungenerated noise columns) for live structure-block Save/Load where
    /// vanilla's `ServerLevel.getChunk` blocks until the column is available.
    pub(in crate::world) fn prepare_full_chunks(
        &mut self,
        chunks: &mut ChunkStore,
        targets: &[ChunkKey],
        game_time: i64,
        pending: &mut crate::world::simulation_level::pending_spawns::PendingEntitySpawns,
    ) {
        if targets.is_empty() {
            return;
        }
        let mut needed = Vec::new();
        for &chunk in targets {
            let full = chunks.is_ticking_full(chunk.0, chunk.1);
            if !full || self.lifecycle.status(chunk) < super::FullChunkStatus::Full {
                self.lifecycle.add_unknown(chunk, 33);
            }
            if !full {
                needed.push(chunk);
            }
        }
        if needed.is_empty() {
            return;
        }
        let mut to_prepare = Vec::new();
        for chunk in needed {
            let loaded = if chunks.has_saved_column_input(chunk) {
                chunks.restore_saved_column_input(chunk).map(|_| ())
            } else if chunks.is_stored_readable(chunk) {
                chunks.read_stored_column(chunk)
            } else {
                to_prepare.push(chunk);
                Ok(())
            };
            if let Err(error) = loaded {
                tracing::warn!("column {chunk:?} for structure block: {error}");
            }
        }
        if !to_prepare.is_empty() {
            match prepare_full_chunk_columns(chunks, &self.originals, &to_prepare, self.entity_nbt)
            {
                Ok(batch) => pending.worldgen_entities.extend(batch),
                Err(error) => tracing::warn!("columns {to_prepare:?} for structure block: {error}"),
            }
        }
        super::cold_columns::reconcile_saved_column_accesses(
            chunks,
            self.lifecycle,
            self.saved_chunks,
            self.saved_tick_chunks,
            game_time,
        );
    }
}

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn parse_worldgen_entity_records_for_test(
        chunk: ChunkKey,
        records: Vec<Vec<NbtSummaryEntry>>,
    ) -> SaveResult<Java26EntityChunkData> {
        worldgen_entity_document(chunk, records)
    }

    /// Brings columns to FULL in place, for a scene built before any ticket
    /// reaches them.
    #[cfg(test)]
    pub(crate) fn prepare_full_for_test(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        chunks: &[ChunkKey],
    ) {
        self.install_full_chunks_by_key(&dimension.into(), chunks)
            .unwrap();
    }

    pub(in crate::world) fn install_worldgen_entities(
        &mut self,
        dimension: &LevelId,
        prepared: PreparedWorldgenEntityBatch,
    ) -> bool {
        install_worldgen_entities(&mut self.level_mut_by_id(dimension).entities, prepared)
    }

    /// Makes one column's blocks readable. `getChunkNow`: a FULL chunk's
    /// blocks are final, so generation queued nearby is no reason to commit
    /// that work before reading it.
    pub(in crate::world) fn install_full_chunk_for_read(
        &mut self,
        dimension: &LevelId,
        (x, z): ChunkKey,
    ) -> SaveResult<()> {
        let level = self.level_mut_by_id(dimension);
        if level.chunks.is_loaded(x, z)
            && level.chunks.generation_status(x, z) == Some(Java26ChunkStatus::Full)
        {
            return level.chunks.flush_live_relights();
        }
        self.install_full_chunks_by_key(dimension, &[(x, z)])
    }

    pub(in crate::world) fn install_full_chunks_by_key(
        &mut self,
        dimension: &LevelId,
        chunks: &[ChunkKey],
    ) -> SaveResult<()> {
        for &(x, z) in chunks {
            if self
                .level_by_id(dimension)
                .chunks
                .validate_served_full_column(x, z)
                .is_err()
                || self.level_by_id(dimension).chunk_lifecycle.status((x, z))
                    < super::FullChunkStatus::Full
            {
                self.level_mut_by_id(dimension)
                    .chunk_lifecycle
                    .add_unknown((x, z), 33);
            }
        }
        if !self.full_chunks_served(dimension, chunks) {
            self.commit_generation_near(dimension, chunks)?;
        }
        let prepared = self.prepare_full_chunk_columns(dimension, chunks);
        let level = self.level_mut_by_id(dimension);
        super::generation_queue::clear_unclaimed_prefetch(
            &mut level.chunks,
            &level.chunk_lifecycle.generation,
        );
        prepared
    }

    /// Generates the column a first-time player spawns in before the server
    /// serves, the way vanilla's level creation leaves its spawn chunk built,
    /// so the first joins find it instead of generating it on the world thread.
    pub fn prepare_default_spawn(&mut self) -> SaveResult<()> {
        let spawn = self.find_default_spawn()?;
        // The search's UNKNOWN tickets would load their whole radius to
        // partial statuses for one tick, columns the first players then have
        // to carry on serially.
        for level in self.levels.values_mut() {
            level.chunk_lifecycle.unknown.clear();
        }
        self.level_mut_by_id(&spawn.dimension)
            .chunk_lifecycle
            .resident_spawn_column = Some((
            (spawn.position.x.floor() as i32).div_euclid(16),
            (spawn.position.z.floor() as i32).div_euclid(16),
        ));
        Ok(())
    }

    pub(in crate::world) fn full_chunks_served(
        &self,
        dimension: &LevelId,
        chunks: &[ChunkKey],
    ) -> bool {
        full_chunks_served(&self.level_by_id(dimension).chunks, chunks)
    }

    pub(in crate::world) fn prepare_full_chunk_columns(
        &mut self,
        dimension: &LevelId,
        chunks: &[ChunkKey],
    ) -> SaveResult<()> {
        if self.full_chunks_served(dimension, chunks) {
            return Ok(());
        }
        let input = self.entity_nbt_read_snapshot();
        let (store, originals) = self.original_mob_inputs(dimension);
        if let Some(prepared) = prepare_full_chunk_columns(store, &originals, chunks, &input)? {
            self.install_worldgen_entities(dimension, prepared);
        }
        self.reconcile_saved_column_accesses(dimension);
        Ok(())
    }
}
