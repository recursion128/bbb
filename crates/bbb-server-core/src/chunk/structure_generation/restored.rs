use bbb_server_persistence::{Java26ChunkData, Java26ChunkStatus};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{ChunkFeaturesState, ChunkGenerationState};
use bbb_server_types::chunk_data::{ChunkPreCarversState, ChunkPreCarversStatus};

use super::*;

impl crate::chunk::ChunkStore {
    /// A saved record's structure starts and references, read without
    /// restoring the column.
    pub(crate) fn saved_structure_component(
        &self,
        data: &Java26ChunkData,
    ) -> Option<NoiseStructureChunkData> {
        let (x, z) = (data.x(), data.z());
        if let crate::chunk::ChunkSource::NoiseCarvers(program) = &self.source {
            return program
                .restored_structure_chunk_data(data.structures(), x, z)
                .ok();
        }
        let program = self.ordinary_structures.as_ref()?;
        let mut structures = NoiseStructureChunkData::from_nbt(
            data.structures(),
            x,
            z,
            &program.plan,
            &program.startup,
        );
        if structures.has_bbb_starts() {
            let context = program.context(&self.source);
            if let Ok(natural) =
                context.generate_registered_structure_starts_status_task(x, z, &program.rings)
            {
                structures.hydrate_bbb_starts(natural);
            }
        }
        Some(structures)
    }

    pub(crate) fn restore_ordinary_generation_metadata(
        &mut self,
        data: &Java26ChunkData,
    ) -> SaveResult<()> {
        let chunk = (data.x(), data.z());
        let path = format!(
            "ordinary_generation.restored_structures[{},{}]",
            chunk.0, chunk.1
        );
        if self.ordinary_structures.is_none() && !is_empty_structure_document(data.structures()) {
            return Err(invalid_schema(
                &path,
                "structure restore requires the frozen datapack generation",
            ));
        }
        let status = match data.status() {
            Java26ChunkStatus::Empty => Some(ChunkPreCarversStatus::Empty),
            Java26ChunkStatus::StructureStarts => Some(ChunkPreCarversStatus::StructureStarts),
            Java26ChunkStatus::StructureReferences => {
                Some(ChunkPreCarversStatus::StructureReferences)
            }
            Java26ChunkStatus::Biomes => Some(ChunkPreCarversStatus::Biomes),
            Java26ChunkStatus::Noise => Some(ChunkPreCarversStatus::Noise),
            Java26ChunkStatus::Surface => Some(ChunkPreCarversStatus::Surface),
            _ => None,
        };
        if let Some(program) = &self.ordinary_structures {
            let mut structures = NoiseStructureChunkData::from_nbt(
                data.structures(),
                chunk.0,
                chunk.1,
                &program.plan,
                &program.startup,
            );
            if structures.has_bbb_starts() {
                let context = program.context(&self.source);
                if let Ok(natural) = context.generate_registered_structure_starts_status_task(
                    chunk.0,
                    chunk.1,
                    &program.rings,
                ) {
                    structures.hydrate_bbb_starts(natural);
                }
            }
            self.structure_status.restore_chunk_component_at_status(
                structures,
                data.status() >= Java26ChunkStatus::StructureStarts,
                data.status() >= Java26ChunkStatus::StructureReferences,
            )?;
        }
        if let Some(status) = status {
            let column = &self.loaded(chunk).column;
            let positions =
                crate::chunk::restored_generation::restore_postprocessing_positions(column, data)?;
            let loaded = self.loaded_mut(chunk);
            loaded.pre_carvers = Some(ChunkPreCarversState::new(status, positions));
            loaded.generation = None;
        } else {
            let column = &self.loaded(chunk).column;
            let positions =
                crate::chunk::restored_generation::restore_postprocessing_positions(column, data)?;
            let state = if data.status() == Java26ChunkStatus::Full {
                let mut state = self.ordinary_full_state(chunk);
                if let ChunkGenerationState::Full {
                    postprocessing_positions,
                    ..
                } = &mut state
                {
                    *postprocessing_positions = positions;
                }
                state
            } else if data.status() == Java26ChunkStatus::Carvers {
                ChunkGenerationState::Carvers {
                    postprocessing_positions: positions,
                    carving_mask: data.carving_mask().unwrap_or(&[]).to_vec(),
                }
            } else {
                let features = ChunkFeaturesState {
                    postprocessing_positions: positions,
                    carving_mask: data.carving_mask().unwrap_or(&[]).to_vec(),
                    final_heightmaps: FeatureHeightmaps::from_saved(column, data.heightmaps())?,
                };
                match data.status() {
                    Java26ChunkStatus::InitializeLight => ChunkGenerationState::InitializeLight(
                        self.stage_initialize_light_from_features(chunk, features)?,
                    ),
                    Java26ChunkStatus::Light | Java26ChunkStatus::Spawn => {
                        let light = bbb_server_types::chunk_data::ChunkLightState {
                            initialized: self
                                .stage_initialize_light_from_features(chunk, features)?,

                            is_light_correct: data.is_light_on(),
                        };
                        if data.status() == Java26ChunkStatus::Spawn {
                            ChunkGenerationState::Spawn(
                                bbb_server_types::chunk_data::spawn::ChunkSpawnState::from_light(
                                    light, false, false,
                                ),
                            )
                        } else {
                            ChunkGenerationState::Light(light)
                        }
                    }
                    _ => ChunkGenerationState::Features(features),
                }
            };
            let loaded = self.loaded_mut(chunk);
            loaded.generation = Some(state);
            loaded.pre_carvers = None;
        }
        let loaded = self.loaded_mut(chunk);
        if data.status() < Java26ChunkStatus::Full {
            if let Some(records) = data.entities().filter(|records| !records.is_empty()) {
                loaded.proto_entities = records.to_vec();
            }
        }
        loaded.storage_backed_generation = true;
        Ok(())
    }
}
