use std::collections::BTreeSet;

use bbb_server_types::save::{SaveError, SaveResult};

use super::{ChunkSource, ChunkStore};
use bbb_server_types::chunk_data::ChunkGenerationState;

#[cfg(test)]
mod tests;

impl ChunkStore {
    pub(super) fn is_ordinary_source(&self) -> bool {
        matches!(self.source, ChunkSource::Flat(_) | ChunkSource::Debug(_))
    }

    pub fn ordinary_derived_dirty_chunks(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.chunks
            .iter()
            .filter(|(_, chunk)| chunk.derived_dirty)
            .map(|(&key, _)| key)
    }

    pub fn is_ordinary_derived_dirty(&self, chunk: (i32, i32)) -> bool {
        self.chunk_flag(chunk, |loaded| loaded.derived_dirty)
    }

    pub fn begin_ordinary_light_restore(&mut self) {
        if self.is_ordinary_source() {
            self.ordinary_light_restoring = true;
        }
    }

    pub fn finish_ordinary_light_restore(&mut self) -> SaveResult<()> {
        if !self.is_ordinary_source() || !self.ordinary_light_restoring {
            return Ok(());
        }
        self.ordinary_light_restoring = false;
        self.flush_ordinary_lights(true)?;
        self.live_light_updates.clear();
        for chunk in self.chunks.values_mut() {
            chunk.derived_dirty = false;
        }
        Ok(())
    }

    pub(super) fn ensure_ordinary_full_columns(
        &mut self,
        columns: &[(i32, i32)],
    ) -> SaveResult<()> {
        if self.ordinary_light_restoring {
            return Err(invalid_ordinary_light(
                "cannot serve a restore batch before light initialization",
            ));
        }
        for &column in columns {
            self.require_legacy_generation_work_completed(column)?;
        }
        if columns.iter().all(|&chunk| {
            matches!(
                self.generation(chunk),
                Some(ChunkGenerationState::Full { .. })
            )
        }) && !self.ordinary_light_pending()
        {
            return Ok(());
        }
        if self.ordinary_structures.is_some() {
            let mut targets = BTreeSet::new();
            for &(x, z) in columns {
                for [x, z] in super::carvers_region_coordinates([x, z])? {
                    targets.insert((x, z));
                }
            }
            // ChunkGenerationTask.java:115-128: a dependency layer is scheduled x-major then z-minor.
            for target in targets {
                self.complete_ordinary_features(target)?;
            }
        }
        if matches!(self.source, super::ChunkSource::Debug(_)) && self.ordinary_structures.is_none()
        {
            for &chunk in columns {
                self.complete_ordinary_features(chunk)?;
            }
        }
        for &(x, z) in columns {
            self.try_load_column_unlit(x, z)?;
            self.ensure_ordinary_structure_references(x, z)?;
            if !matches!(self.state((x, z)), ChunkGenerationState::Full { .. }) {
                self.publish_proto_entities_at_full((x, z))?;
                self.set_generation((x, z), self.ordinary_full_state((x, z)));
                if self.is_storage_backed_generation((x, z)) {
                    self.mark_whole_rewrite_pending((x, z))?;
                }
            }
        }
        self.flush_ordinary_lights(false)?;
        Ok(())
    }

    /// A loaded FULL column whose light has not been propagated yet.
    fn is_unlit_ordinary(&self, chunk: (i32, i32), loaded: &super::Chunk) -> bool {
        matches!(loaded.generation, Some(ChunkGenerationState::Full { .. }))
            && !self.light_storage.is_enabled(chunk)
    }

    #[cfg(test)]
    fn unlit_ordinary_columns(&self) -> BTreeSet<(i32, i32)> {
        self.chunks
            .iter()
            .filter(|(&chunk, loaded)| self.is_unlit_ordinary(chunk, loaded))
            .map(|(&chunk, _)| chunk)
            .collect()
    }

    pub(super) fn ordinary_light_pending(&self) -> bool {
        !self.pending_light_changes.is_empty()
            || (self.ordinary_lights_dirty
                && self.chunks.iter().any(|(&chunk, loaded)| {
                    loaded.saved_light || self.is_unlit_ordinary(chunk, loaded)
                }))
    }

    pub(super) fn validate_ordinary_light(&self, column: (i32, i32)) -> SaveResult<()> {
        if self.ordinary_light_restoring
            || !self.chunks.contains_key(&column)
            || !self.light_storage.is_enabled(column)
            || !self.pending_light_changes.is_empty()
            || !matches!(
                self.generation(column),
                Some(ChunkGenerationState::Full { .. })
            )
        {
            return Err(invalid_ordinary_light(format!(
                "column {column:?} has no current committed FULL light"
            )));
        }
        Ok(())
    }

    /// Lights new FULL columns and re-checks changed blocks; `quiet` publishes
    /// neither live light updates nor save dirt.
    pub(super) fn flush_ordinary_lights(&mut self, quiet: bool) -> SaveResult<()> {
        if self.ordinary_light_restoring
            || (!self.ordinary_lights_dirty && self.pending_light_changes.is_empty())
        {
            return Ok(());
        }
        let mut new = BTreeSet::new();
        let mut restored = BTreeSet::new();
        if self.ordinary_lights_dirty {
            for (&key, chunk) in &mut self.chunks {
                if chunk.saved_light {
                    chunk.saved_light = false;
                    restored.insert(key);
                }
                if matches!(chunk.generation, Some(ChunkGenerationState::Full { .. }))
                    && !self.light_storage.is_enabled(key)
                {
                    new.insert(key);
                }
            }
            if new.is_empty() && restored.is_empty() {
                self.ordinary_lights_dirty = false;
            }
        }
        if new.is_empty() && self.pending_light_changes.is_empty() && restored.is_empty() {
            return Ok(());
        }
        let changes = std::mem::take(&mut self.pending_light_changes);
        self.light_storage.take_dirty();
        self.light_new_columns(&new, &restored)?;
        self.ordinary_lights_dirty = false;
        self.relight_changed_blocks(&changes)?;
        let mut dirty = self.light_storage.take_dirty();
        for &chunk in dirty.keys() {
            if let Some(loaded) = self.chunks.get_mut(&chunk) {
                loaded.cached_packet.take();
            }
        }
        if quiet {
            return Ok(());
        }
        dirty.retain(|&chunk, _| {
            !new.contains(&chunk)
                && matches!(
                    self.generation(chunk),
                    Some(ChunkGenerationState::Full { .. })
                )
        });
        for chunk in self.record_light_dirt(dirty) {
            let loaded = self.loaded_mut(chunk);
            loaded.derived_dirty = true;
            loaded.mark_save_dirty();
        }
        Ok(())
    }
}

fn invalid_ordinary_light(detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("ordinary FULL light: {}", detail.into()),
    }
}
