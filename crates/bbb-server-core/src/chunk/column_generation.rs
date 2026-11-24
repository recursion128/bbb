//! Column terrain publication through the source's generation owner.
use super::*;
use bbb_server_types::chunk_data::{ChunkColumn, ChunkGenerationState};
impl ChunkStore {
    pub(crate) fn advance_ordinary_pre_carvers_column(
        &mut self,
        chunk: (i32, i32),
    ) -> SaveResult<()> {
        let (mut column, state, _) = self
            .generated_ordinary_column(chunk.0, chunk.1)
            .expect("ordinary terrain");
        if let Some(saved) = self.column_at(chunk) {
            if self
                .pre_carvers(chunk)
                .expect("the chunk has a pre-CARVERS state")
                .status()
                .is_at_least(&bbb_server_types::chunk_data::ChunkPreCarversStatus::Noise)
            {
                column = saved.clone();
            }
            column.set_inhabited_time(saved.inhabited_time());
            *column.block_ticks_mut() = saved.block_ticks().clone();
            *column.fluid_ticks_mut() = saved.fluid_ticks().clone();
        }
        self.put_column(chunk, column);
        let mut state = state;
        if let ChunkGenerationState::Carvers {
            postprocessing_positions,
            ..
        } = &mut state
        {
            *postprocessing_positions = self
                .pre_carvers(chunk)
                .expect("the chunk has a pre-CARVERS state")
                .postprocessing_positions()
                .to_vec();
        }
        let loaded = self.loaded_mut(chunk);
        loaded.generation = Some(state);
        loaded.pre_carvers = None;
        if self.is_storage_backed_generation(chunk) {
            self.mark_whole_rewrite_pending(chunk)?;
        }
        Ok(())
    }

    pub(crate) fn generated_ordinary_column(
        &self,
        cx: i32,
        cz: i32,
    ) -> Option<(ChunkColumn, ChunkGenerationState, bool)> {
        let result = match &self.source {
            ChunkSource::Flat(generator) => {
                let surface_y = generator
                    .base_height(
                        bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
                    )
                    .map_or(self.generated.min_y(), |first_free| first_free - 1);
                (
                    self.generated.clone(),
                    ChunkGenerationState::Full {
                        surface_y,
                        postprocessing_positions: Vec::new(),
                    },
                    self.ordinary_structures.is_some(),
                )
            }
            ChunkSource::Debug(generator) => {
                let surface_y = [70, 60]
                    .into_iter()
                    .find(|&y| {
                        (generator.min_y()..generator.min_y() + generator.height()).contains(&y)
                    })
                    .unwrap_or(generator.min_y());
                (
                    generator.generate_chunk_with_biome_registry(
                        cx,
                        cz,
                        self.biome_id,
                        self.biome_registry_len,
                    ),
                    ChunkGenerationState::Full {
                        surface_y,
                        postprocessing_positions: Vec::new(),
                    },
                    true,
                )
            }
            ChunkSource::NoiseCarvers(_) => return None,
        };
        if self.ordinary_structures.is_some() || matches!(self.source, ChunkSource::Debug(_)) {
            let column = if self.ordinary_structures.is_some()
                && matches!(self.source, ChunkSource::Debug(_))
            {
                ChunkColumn::new_with_biome_registry(
                    self.min_y(),
                    self.section_count(),
                    self.air_state_id(),
                    self.biome_id,
                    self.biome_registry_len,
                )
            } else {
                result.0
            };
            Some((
                column,
                ChunkGenerationState::Carvers {
                    postprocessing_positions: Vec::new(),
                    carving_mask: Vec::new(),
                },
                true,
            ))
        } else {
            Some(result)
        }
    }

    pub(crate) fn ordinary_full_state(&self, chunk: (i32, i32)) -> ChunkGenerationState {
        let surface_y = match &self.source {
            ChunkSource::Flat(generator) => generator
                .base_height(bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID)
                .map_or(self.min_y(), |height| height - 1),
            ChunkSource::Debug(_) => [70, 60]
                .into_iter()
                .find(|&y| {
                    (self.min_y()..self.min_y() + self.section_count() as i32 * 16).contains(&y)
                })
                .unwrap_or(self.min_y()),
            ChunkSource::NoiseCarvers(_) => unreachable!("ordinary FULL for {chunk:?}"),
        };
        ChunkGenerationState::Full {
            surface_y,
            postprocessing_positions: self
                .generation(chunk)
                .map(crate::chunk::ordinary_generation::ordinary_postprocessing)
                .unwrap_or_default(),
        }
    }
    pub(crate) fn try_load_column_unlit(&mut self, cx: i32, cz: i32) -> SaveResult<&ChunkColumn> {
        self.prepare_generation_admission((cx, cz))?;
        self.advance_pre_carvers_column((cx, cz))?;
        self.ensure_ordinary_structure_starts(cx, cz)?;
        if !self.chunks.contains_key(&(cx, cz)) {
            let (column, state, needs_initial_persistence) = if let Some(ordinary) =
                self.generated_ordinary_column(cx, cz)
            {
                ordinary
            } else {
                if matches!(self.source, ChunkSource::NoiseCarvers(_)) {
                    self.prepare_structure_window_admission((cx, cz))?;
                }
                match &self.source {
                    ChunkSource::NoiseCarvers(program) => {
                        let biome_blender = self.blending_resolver((cx, cz))?;
                        program.preflight_structure_start_owners(
                            self.structure_status.generate_structures(),
                        )?;
                        let (generated, _) = self
                            .structure_status
                            .generate_carvers_with_biome_blender(program, cx, cz, &biome_blender)?;
                        let (_, _, column, state) = generated.into_persistence_parts()?;
                        (column, state, true)
                    }
                    _ => unreachable!("ordinary sources use their terrain adapter"),
                }
            };
            let loaded = self.put_column((cx, cz), column);
            loaded.generation = Some(state);
            loaded.blending = Some(None);
            loaded.pending_initial_persistence = needs_initial_persistence;
        }
        Ok(&self.loaded((cx, cz)).column)
    }
}
