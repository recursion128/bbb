//! Ordinary terrain uses the shared structure placement and state publication owners.

use super::*;
use crate::chunk::noise_generator::features::FeatureRegionData;
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{ChunkFeaturesState, ChunkGenerationState};

impl ChunkStore {
    pub(crate) fn complete_ordinary_features(&mut self, target: (i32, i32)) -> SaveResult<()> {
        // A stored column is read first: its saved status may already be past FEATURES.
        self.prepare_generation_admission(target)?;
        if (self.ordinary_structures.is_none() && !matches!(self.source, ChunkSource::Debug(_)))
            || self.features_completed(target)
        {
            return Ok(());
        }
        self.try_load_column_unlit(target.0, target.1)?;
        self.ensure_ordinary_structure_references(target.0, target.1)?;
        if let ChunkSource::Debug(generator) = &self.source {
            // DebugLevelSource.java:60-75: decoration replaces the generic structure pass.
            let mut column = generator.generate_chunk_with_biome_registry(
                target.0,
                target.1,
                self.biome_id,
                self.biome_registry_len,
            );
            let saved = &self.loaded(target).column;
            column.set_inhabited_time(saved.inhabited_time());
            *column.block_ticks_mut() = saved.block_ticks().clone();
            *column.fluid_ticks_mut() = saved.fluid_ticks().clone();
            let state = ChunkGenerationState::Features(ChunkFeaturesState {
                postprocessing_positions: ordinary_postprocessing(self.state(target)),
                carving_mask: Vec::new(),
                final_heightmaps: FeatureHeightmaps::prime(&column),
            });
            self.put_column(target, column).generation = Some(state);
            if self.is_storage_backed_generation(target) {
                self.mark_whole_rewrite_pending(target)?;
            }
            return Ok(());
        }

        let coordinates = carvers_region_coordinates([target.0, target.1])?;
        let chunks = coordinates
            .into_iter()
            .map(|[x, z]| (x, z))
            .collect::<BTreeSet<_>>();
        for &(x, z) in &chunks {
            self.try_load_column_unlit(x, z)?;
        }
        let sources = self
            .structure_status
            .component(target.0, target.1)
            .expect("ordinary References are ready")
            .referenced_source_chunks();
        let program = self
            .ordinary_structures
            .as_ref()
            .expect("configured ordinary program")
            .clone();
        for &[x, z] in &sources {
            self.try_load_column_unlit(x, z)?;
        }
        let context = program.context(&self.source);
        let mut steps =
            crate::chunk::features_orchestration::registered_sequence::prepare_flat_structure_steps(
                &context,
                &self.structure_status,
                [target.0, target.1],
            )?;
        let source = context.flat_feature_placement_source(
            program.features().expect("configured flat feature plan"),
        );
        let mut data = FeatureRegionData::new(
            &mut self.chunks,
            &mut self.block_entities,
            &mut self.poi,
            &mut self.pending_light_changes,
            &chunks,
        );
        let placement = (|| {
            let mut region = data.region(&source, [target.0, target.1])?;
            for step in 0..source.step_count() {
                steps.place_step(&context, &mut self.structure_status, step, &mut region)?;
                region.place_feature_step(&source, step)?;
            }
            let _ = region;
            data.complete_target(target);
            Ok::<_, bbb_server_types::save::SaveError>(())
        })();
        let dirty = std::mem::take(&mut data.dirty);
        drop(data);
        for key in dirty {
            if self.is_storage_backed_generation(key) {
                self.mark_whole_rewrite_pending(key)?;
            }
        }
        for [x, z] in sources {
            if self.is_storage_backed_generation((x, z)) {
                self.mark_whole_rewrite_pending((x, z))?;
            }
        }
        placement
    }
}

pub(crate) fn ordinary_postprocessing(state: &ChunkGenerationState) -> Vec<(i32, i32, i32)> {
    match state {
        ChunkGenerationState::Carvers {
            postprocessing_positions,
            ..
        } => postprocessing_positions.clone(),
        ChunkGenerationState::Features(features) => features.postprocessing_positions.clone(),
        ChunkGenerationState::InitializeLight(light) => {
            light.features().postprocessing_positions().to_vec()
        }
        ChunkGenerationState::Light(light) => light
            .initialized()
            .features()
            .postprocessing_positions()
            .to_vec(),
        ChunkGenerationState::Spawn(spawn) => spawn
            .light()
            .initialized()
            .features()
            .postprocessing_positions()
            .to_vec(),
        ChunkGenerationState::NoiseFull(full) => full
            .light()
            .initialized()
            .features()
            .postprocessing_positions()
            .to_vec(),
        ChunkGenerationState::Full {
            postprocessing_positions,
            ..
        } => postprocessing_positions.clone(),
    }
}
