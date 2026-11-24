//! Loading dependencies retain their actual generation owner and persisted stage.

use bbb_server_types::chunk_data::{ChunkColumn, ChunkFeaturesState, ChunkGenerationState};
use std::collections::BTreeSet;

use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{ChunkPreCarversState, ChunkPreCarversStatus};

use super::parallel_generation::Prefetched;
use super::*;

impl ChunkStore {
    pub fn generation_status(&self, cx: i32, cz: i32) -> Option<Java26ChunkStatus> {
        if let Some(state) = self.pre_carvers((cx, cz)) {
            return Some(restored_generation::pre_carvers_java_status(
                *state.status(),
            ));
        }
        self.generation((cx, cz)).map(|state| match state {
            ChunkGenerationState::Full { .. } | ChunkGenerationState::NoiseFull(_) => {
                Java26ChunkStatus::Full
            }
            ChunkGenerationState::Carvers { .. } => Java26ChunkStatus::Carvers,
            ChunkGenerationState::Features(_) => Java26ChunkStatus::Features,
            ChunkGenerationState::InitializeLight(_) => Java26ChunkStatus::InitializeLight,
            ChunkGenerationState::Light(_) => Java26ChunkStatus::Light,
            ChunkGenerationState::Spawn(_) => Java26ChunkStatus::Spawn,
        })
    }

    pub fn ensure_loading_generation(
        &mut self,
        targets: &[((i32, i32), Java26ChunkStatus)],
    ) -> SaveResult<()> {
        if targets.iter().all(|&(chunk, target)| {
            target <= Java26ChunkStatus::InitializeLight
                && self.require_generation_admission(chunk).is_ok()
                && self
                    .generation_status(chunk.0, chunk.1)
                    .is_some_and(|status| status >= target)
        }) {
            return Ok(());
        }
        self.pregenerate(&[], targets)?;
        let mut ordered = BTreeMap::new();
        for &(chunk, target) in targets {
            self.prepare_generation_admission(chunk)?;
            if target > Java26ChunkStatus::InitializeLight {
                return Err(invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "loading generation leaves FULL promotion to the level owner",
                ));
            }
            ordered
                .entry(chunk)
                .and_modify(|prior: &mut Java26ChunkStatus| *prior = (*prior).max(target))
                .or_insert(target);
        }
        let mut initialize = BTreeSet::new();
        // Finish all terrain dependencies before any feature write or source initialization.
        for (&chunk, &target) in &ordered {
            if self
                .generation_status(chunk.0, chunk.1)
                .is_some_and(|status| status >= target)
            {
                continue;
            }
            self.ensure_loading_terrain(chunk, target.min(Java26ChunkStatus::Carvers))?;
            if target >= Java26ChunkStatus::Features {
                initialize.insert(chunk);
            }
        }
        if self.is_ordinary_source() {
            for &chunk in &initialize {
                self.complete_ordinary_features(chunk)?;
                if !matches!(
                    self.generation(chunk),
                    Some(ChunkGenerationState::Features(_))
                ) {
                    let features = ChunkFeaturesState {
                        postprocessing_positions: self.postprocessing_positions(chunk.0, chunk.1),
                        carving_mask: Vec::new(),
                        final_heightmaps: FeatureHeightmaps::prime(&self.loaded(chunk).column),
                    };
                    self.set_generation(chunk, ChunkGenerationState::Features(features));
                }
            }
            let lit = initialize
                .iter()
                .copied()
                .filter(|chunk| ordered[chunk] == Java26ChunkStatus::InitializeLight)
                .collect::<Vec<_>>();
            self.initialize_light_batch(&lit)?;
        } else {
            self.ensure_loading_features_with_relight(&initialize)?;
            let lit = initialize
                .iter()
                .copied()
                .filter(|chunk| {
                    ordered[chunk] == Java26ChunkStatus::InitializeLight
                        && matches!(
                            self.generation(*chunk),
                            Some(ChunkGenerationState::Features(_))
                        )
                })
                .collect::<Vec<_>>();
            self.initialize_light_batch(&lit)?;
        }
        Ok(())
    }

    fn ensure_loading_terrain(
        &mut self,
        chunk: (i32, i32),
        target: Java26ChunkStatus,
    ) -> SaveResult<()> {
        if self
            .generation_status(chunk.0, chunk.1)
            .is_some_and(|status| status >= target)
        {
            return Ok(());
        }
        if target >= Java26ChunkStatus::Carvers {
            self.try_load_column_unlit(chunk.0, chunk.1)?;
            // Sources whose ordinary adapter skips empty tasks still publish the
            // requested ProtoChunk stage rather than creating a LevelChunk.
            if self.is_ordinary_source()
                && matches!(
                    self.generation(chunk),
                    Some(ChunkGenerationState::Full { .. })
                )
            {
                let loaded = self.loaded_mut(chunk);
                loaded.generation = Some(ChunkGenerationState::Carvers {
                    postprocessing_positions: Vec::new(),
                    carving_mask: Vec::new(),
                });
                loaded.pending_initial_persistence = true;
            }
            if matches!(self.source, ChunkSource::Debug(_)) {
                let mut column = ChunkColumn::new_with_biome_registry(
                    self.min_y(),
                    self.section_count(),
                    self.air_state_id(),
                    self.biome_id,
                    self.biome_registry_len,
                );
                let previous = &self.loaded(chunk).column;
                column.set_inhabited_time(previous.inhabited_time());
                *column.block_ticks_mut() = previous.block_ticks().clone();
                *column.fluid_ticks_mut() = previous.fluid_ticks().clone();
                self.put_column(chunk, column);
            }
            return Ok(());
        }
        if target == Java26ChunkStatus::Empty {
            self.hydrate_saved_generation_column(chunk.0, chunk.1, target);
            if !self.chunks.contains_key(&chunk) {
                let column = ChunkColumn::new_with_biome_registry(
                    self.min_y(),
                    self.section_count(),
                    self.air_state_id(),
                    self.biome_id,
                    self.biome_registry_len,
                );
                self.put_column(chunk, column).blending = Some(None);
            }
            let loaded = self.loaded_mut(chunk);
            loaded.pre_carvers = Some(ChunkPreCarversState::new(
                ChunkPreCarversStatus::Empty,
                Vec::new(),
            ));
            loaded.pending_initial_persistence = true;
            return Ok(());
        }
        let program = match &self.source {
            ChunkSource::NoiseCarvers(program) => Some(program.clone()),
            _ => None,
        };
        if let Some(program) = &program {
            self.ensure_structure_starts_column(program, [chunk.0, chunk.1])?;
        } else {
            self.ensure_ordinary_structure_starts(chunk.0, chunk.1)?;
            self.publish_structure_starts_column([chunk.0, chunk.1])?;
        }
        let mut status = ChunkPreCarversStatus::StructureStarts;
        if target >= Java26ChunkStatus::StructureReferences {
            if let Some(program) = &program {
                self.prepare_structure_window_admission(chunk)?;
                let (_, beardifier) = self
                    .structure_status
                    .prepare_structure_references(program, chunk.0, chunk.1)?;
                if target >= Java26ChunkStatus::Biomes {
                    if self
                        .generation_status(chunk.0, chunk.1)
                        .is_none_or(|current| current < Java26ChunkStatus::Biomes)
                    {
                        let blender = self.blending_resolver(chunk)?;
                        let column = self.loaded(chunk).column.clone();
                        let column = match self.structure_status.take_prefetched([chunk.0, chunk.1])
                        {
                            Some(Prefetched::Biomes { input, output })
                                if blender.is_empty() && input == column =>
                            {
                                output
                            }
                            _ => {
                                let mut column = column;
                                program.fill_loading_biomes(
                                    chunk,
                                    &mut column,
                                    &beardifier,
                                    &blender,
                                )?;
                                column
                            }
                        };
                        self.put_column(chunk, column);
                    }
                    status = ChunkPreCarversStatus::Biomes;
                } else {
                    status = ChunkPreCarversStatus::StructureReferences;
                }
            } else {
                self.ensure_ordinary_structure_references(chunk.0, chunk.1)?;
                status = if target >= Java26ChunkStatus::Biomes {
                    ChunkPreCarversStatus::Biomes
                } else {
                    ChunkPreCarversStatus::StructureReferences
                };
            }
        }
        let mut positions = self.postprocessing_positions(chunk.0, chunk.1);
        if target >= Java26ChunkStatus::Noise {
            if let Some(program) = &program {
                self.prepare_structure_window_admission(chunk)?;
                let (_, beardifier) = self
                    .structure_status
                    .prepare_structure_references(program, chunk.0, chunk.1)?;
                let blender = self.blending_resolver(chunk)?;
                let generated = program.resume_saved_chunk_to_status(
                    crate::chunk::noise_generator::SavedChunkResume {
                        chunk: [chunk.0, chunk.1],
                        saved_status: self.generation_status(chunk.0, chunk.1).unwrap(),
                        saved_column: &self.loaded(chunk).column,
                        saved_postprocessing: positions,
                    },
                    target,
                    &beardifier,
                    &blender,
                )?;
                positions = generated.postprocessing_positions().to_vec();
                let column = generated.into_column();
                self.put_column(chunk, column);
            } else if matches!(self.source, ChunkSource::Flat(_))
                && self
                    .generation_status(chunk.0, chunk.1)
                    .is_some_and(|current| current < Java26ChunkStatus::Noise)
            {
                let previous = &self.loaded(chunk).column;
                let mut column = self.generated.clone();
                column.set_inhabited_time(previous.inhabited_time());
                *column.block_ticks_mut() = previous.block_ticks().clone();
                *column.fluid_ticks_mut() = previous.fluid_ticks().clone();
                if self
                    .generation_status(chunk.0, chunk.1)
                    .is_some_and(|current| current >= Java26ChunkStatus::Biomes)
                {
                    for y in previous.min_y().div_euclid(4)
                        ..previous.min_y().div_euclid(4) + previous.section_count() as i32 * 4
                    {
                        for x in 0..4 {
                            for z in 0..4 {
                                column
                                    .set_noise_biome(
                                        x,
                                        y,
                                        z,
                                        previous
                                            .get_noise_biome(x, y, z)
                                            .expect("saved biome cell"),
                                    )
                                    .map_err(|error| {
                                        invalid_carvers_region(
                                            [chunk.0, chunk.1],
                                            error.to_string(),
                                        )
                                    })?;
                            }
                        }
                    }
                }
                self.put_column(chunk, column);
            }
            status = if target == Java26ChunkStatus::Noise {
                ChunkPreCarversStatus::Noise
            } else {
                ChunkPreCarversStatus::Surface
            };
        }
        self.loaded_mut(chunk).pre_carvers = Some(ChunkPreCarversState::new(status, positions));
        if self.is_storage_backed_generation(chunk) {
            self.mark_whole_rewrite_pending(chunk)?;
        }
        Ok(())
    }

    pub fn postprocessing_positions(&self, cx: i32, cz: i32) -> Vec<(i32, i32, i32)> {
        if let Some(state) = self.pre_carvers((cx, cz)) {
            return state.postprocessing_positions().to_vec();
        }
        self.generation((cx, cz))
            .map(ordinary_generation::ordinary_postprocessing)
            .unwrap_or_default()
    }

    pub fn clear_postprocessing_positions(&mut self, cx: i32, cz: i32) {
        let chunk = (cx, cz);
        let changed = !self.postprocessing_positions(cx, cz).is_empty();
        self.update_chunk(chunk, |loaded| {
            if let Some(state) = &mut loaded.generation {
                state.clear_postprocessing_positions();
            }
            if let Some(state) = &mut loaded.pre_carvers {
                state.clear_postprocessing_positions();
            }
        });
        if changed
            && (self.is_durable(chunk)
                || (self.is_ordinary_source()
                    && matches!(
                        self.generation(chunk),
                        Some(ChunkGenerationState::Full { .. })
                    )))
        {
            self.mark_whole_rewrite_pending(chunk)
                .expect("postprocessing clear rewrite");
        }
    }
}

#[cfg(test)]
mod tests;
