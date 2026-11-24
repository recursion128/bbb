//! World activation and retention of saved cold columns.

use crate::chunk::{generation_publication::GenerationItemPublication, SavedColumnResources};
use crate::chunk_sender::ChunkKey;
use crate::world::LevelId;
use crate::world::ServerWorld;
use bbb_server_persistence::world_storage::Java26RuntimeColumn;
use bbb_server_types::save::Java26LevelKey;
use std::sync::Arc;

impl ServerWorld {
    pub(in crate::world) fn refresh_saved_column_resources(&mut self) {
        for dimension in self.level_ids() {
            let resources = SavedColumnResources {
                dimension: self.level_key(dimension).clone(),
                registries: self.registries.clone(),
                items: GenerationItemPublication::new(Arc::new(self.item_stack_resolver())),
            };
            self.level_mut_by_id(&dimension)
                .chunks
                .install_saved_column_resources(resources);
        }
    }

    pub fn runtime_unload_columns(&self) -> Vec<(Java26LevelKey, ChunkKey)> {
        self.levels
            .iter()
            .filter(|_| self.auto_save)
            .flat_map(|(_, level)| {
                level
                    .chunk_lifecycle
                    .column_unloads
                    .iter()
                    .filter(|&&(x, z)| level.chunks.is_loaded(x, z))
                    .map(move |&chunk| (level.dimension.as_ref().clone(), chunk))
            })
            .collect()
    }

    pub fn retain_runtime_unload_columns(
        &mut self,
        dimension: &Java26LevelKey,
        records: Vec<(ChunkKey, Java26RuntimeColumn)>,
    ) {
        let Some(id) = self.level_id(dimension) else {
            return;
        };
        let dimension = &id;
        let resources = SavedColumnResources {
            dimension: self.level_key(*dimension).clone(),
            registries: self.registries.clone(),
            items: GenerationItemPublication::new(Arc::new(self.item_stack_resolver())),
        };
        let level = self.level_mut_by_id(dimension);
        level.chunks.install_saved_column_resources(resources);
        if level.chunks.stored_column_reader().is_some() {
            // Storage-backed worlds reload evicted chunks from region storage on
            // demand instead of retaining full copies in memory.
            return;
        }
        for (chunk, record) in records {
            if level.chunk_lifecycle.column_unloads.contains(&chunk)
                && level.chunk_lifecycle.entity_loads.get(&chunk)
                    != Some(&super::EntityChunkLoadStatus::Pending)
                && record.chunk.is_some()
            {
                level.chunks.retain_saved_column_input(chunk, record);
            }
        }
    }

    pub(in crate::world) fn reconcile_saved_column_accesses(&mut self, dimension: &LevelId) {
        let game_time = self.game_time();
        let level = self.level_mut_by_id(dimension);
        reconcile_saved_column_accesses(
            &mut level.chunks,
            &mut level.chunk_lifecycle,
            &mut level.saved_chunks,
            &mut level.saved_tick_chunks,
            game_time,
        );
    }
}

pub(in crate::world) fn reconcile_saved_column_accesses(
    chunks: &mut crate::chunk::ChunkStore,
    lifecycle: &mut super::ChunkLifecycle,
    saved_chunks: &mut std::collections::BTreeSet<ChunkKey>,
    saved_tick_chunks: &mut std::collections::BTreeSet<ChunkKey>,
    game_time: i64,
) {
    for chunk in chunks.take_cold_column_reads() {
        lifecycle.add_unknown(chunk, 33);
    }
    for chunk in chunks.take_regenerated_stored_columns() {
        lifecycle.evicted_columns.remove(&chunk);
        lifecycle.settled = None;
    }
    for effect in chunks.take_saved_column_activations() {
        let chunk = effect.chunk;
        saved_chunks.insert(chunk);
        if effect.had_saved_ticks {
            saved_tick_chunks.insert(chunk);
        }
        lifecycle.evicted_columns.remove(&chunk);
        lifecycle.column_unloads.remove(&chunk);
        lifecycle.settled = None;
        if chunks.is_ticking_full(chunk.0, chunk.1) {
            chunks.register_ticks(chunk, game_time);
        }
    }
}
