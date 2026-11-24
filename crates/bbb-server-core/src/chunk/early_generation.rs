//! Pre-CARVERS column publication and persistence provenance.

use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::chunk_data::ChunkPreCarversStatus;

use crate::chunk::noise_generator::NoiseFinalDensityProgram;
use crate::chunk::parallel_generation::{Prefetched, ResumeInput};
use crate::chunk::{ChunkPreCarversState, ChunkSource, ChunkStore, SaveResult};
use bbb_server_types::chunk_data::ChunkColumn;

impl ChunkStore {
    pub fn pre_carvers_generation_state(&self, cx: i32, cz: i32) -> Option<&ChunkPreCarversState> {
        self.pre_carvers((cx, cz))
    }

    pub(super) fn empty_proto_column(&self) -> ChunkColumn {
        ChunkColumn::new_with_biome_registry(
            self.min_y(),
            self.section_count(),
            self.air_state_id(),
            self.biome_id,
            self.biome_registry_len,
        )
    }

    fn publish_empty_proto_column(&mut self, chunk: (i32, i32)) {
        let column = self.empty_proto_column();
        self.put_column(chunk, column).blending = Some(None);
    }

    pub(crate) fn ensure_structure_starts_column(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk: [i32; 2],
    ) -> SaveResult<()> {
        self.prepare_generation_admission((chunk[0], chunk[1]))?;
        self.structure_status
            .ensure_starts_chunk(program, chunk[0], chunk[1])?;
        self.publish_structure_starts_column(chunk)?;
        Ok(())
    }

    pub(crate) fn publish_structure_starts_column(&mut self, chunk: [i32; 2]) -> SaveResult<()> {
        let chunk = (chunk[0], chunk[1]);
        self.prepare_generation_admission(chunk)?;
        if !self.chunks.contains_key(&chunk) {
            let status = match self.structure_status.status(chunk.0, chunk.1) {
                crate::chunk::structure_status::NoiseStructureTaskStatus::StructureReferences => {
                    ChunkPreCarversStatus::StructureReferences
                }
                _ => ChunkPreCarversStatus::StructureStarts,
            };
            self.publish_empty_proto_column(chunk);
            let loaded = self.loaded_mut(chunk);
            loaded.pre_carvers = Some(ChunkPreCarversState::new(status, Vec::new()));
            loaded.pending_initial_persistence = true;
        }
        Ok(())
    }

    pub fn hydrate_saved_generation_column(
        &mut self,
        cx: i32,
        cz: i32,
        status: Java26ChunkStatus,
    ) -> bool {
        self.authorize_saved_column_hydration((cx, cz));
        if self.is_ordinary_source()
            && self.ordinary_structures.is_some()
            && status == Java26ChunkStatus::Full
        {
            if !self.chunks.contains_key(&(cx, cz)) {
                let (column, _, _) = self
                    .generated_ordinary_column(cx, cz)
                    .expect("ordinary terrain");
                let state = self.ordinary_full_state((cx, cz));
                let loaded = self.put_column((cx, cz), column);
                loaded.generation = Some(state);
                loaded.blending = Some(None);
            }
            return true;
        }
        if (status >= Java26ChunkStatus::Carvers && matches!(self.source, ChunkSource::Debug(_)))
            || (self.is_ordinary_source()
                && self.ordinary_structures.is_none()
                && status == Java26ChunkStatus::Full)
        {
            return false;
        }
        // The saved sections are installed before publishing their typed status.
        if !self.chunks.contains_key(&(cx, cz)) {
            self.publish_empty_proto_column((cx, cz));
        }
        true
    }

    pub(crate) fn advance_pre_carvers_column(&mut self, chunk: (i32, i32)) -> SaveResult<()> {
        if self.pre_carvers(chunk).is_none()
            || self
                .chunk(chunk)
                .and_then(|chunk| chunk.legacy_work.as_ref())
                .and_then(
                    crate::chunk::restored_generation::PendingLegacyGenerationWork::below_zero_retrogen,
                )
                .is_some()
        {
            return Ok(());
        }
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return self.advance_ordinary_pre_carvers_column(chunk);
        };
        let program = std::sync::Arc::clone(program);
        let blender = self.blending_resolver(chunk)?;
        self.prepare_structure_window_admission(chunk)?;
        let (expected_references, beardifier) = self
            .structure_status
            .prepare_structure_references(&program, chunk.0, chunk.1)?;
        let pre_state = self
            .pre_carvers(chunk)
            .expect("the chunk has a pre-CARVERS state");
        let input = ResumeInput {
            status: crate::chunk::restored_generation::pre_carvers_java_status(*pre_state.status()),
            column: self.loaded(chunk).column.clone(),
            postprocessing: pre_state.postprocessing_positions().to_vec(),
        };
        let generated = match self.structure_status.take_prefetched([chunk.0, chunk.1]) {
            Some(Prefetched::Resumed {
                input: prefetched,
                generated,
            }) if blender.is_empty() && prefetched == input => generated,
            Some(Prefetched::Fresh(generated, references))
                if blender.is_empty()
                    && input.status <= Java26ChunkStatus::StructureReferences
                    && input.postprocessing.is_empty()
                    && input.column == self.empty_proto_column()
                    && references == expected_references =>
            {
                generated
            }
            _ => program.resume_saved_chunk_to_carvers(
                crate::chunk::noise_generator::SavedChunkResume {
                    chunk: [chunk.0, chunk.1],
                    saved_status: input.status,
                    saved_column: &input.column,
                    saved_postprocessing: input.postprocessing,
                },
                &beardifier,
                &blender,
            )?,
        };
        let (_, _, mut column, state) = generated.into_persistence_parts()?;
        if let Some(saved) = self.column_at(chunk) {
            column.set_inhabited_time(saved.inhabited_time());
            *column.block_ticks_mut() = saved.block_ticks().clone();
            *column.fluid_ticks_mut() = saved.fluid_ticks().clone();
        }
        let loaded = self.put_column(chunk, column);
        loaded.generation = Some(state);
        loaded.pre_carvers = None;
        if self.is_storage_backed_generation(chunk) {
            self.mark_whole_rewrite_pending(chunk)?;
        }
        Ok(())
    }
}

impl ChunkStore {
    pub fn mark_persisted(&mut self, cx: i32, cz: i32) {
        let chunk = (cx, cz);
        if (matches!(self.source, ChunkSource::NoiseCarvers(_))
            || self.ordinary_structures.is_some()
            || self.pre_carvers(chunk).is_some())
            && (self.generation(chunk).is_some() || self.pre_carvers(chunk).is_some())
        {
            self.loaded_mut(chunk).storage_backed_generation = true;
        }
        if self.validate_fresh_noise_full_center(chunk).is_ok() {
            self.loaded_mut(chunk).storage_backed_full = true;
        }
        self.update_chunk((cx, cz), |chunk| chunk.pending_initial_persistence = false);
    }
}
