//! Saved ProtoChunk generation metadata installed beside a restored column.

use std::collections::BTreeSet;

use super::{invalid_carvers_region, ChunkSource, ChunkStore};
use bbb_protocol::chunk_light::SavedLightSection;
use bbb_server_persistence::chunk_schema::{Java26BelowZeroRetrogen, Java26UpgradeData};
use bbb_server_persistence::{Java26ChunkData, Java26ChunkStatus};
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::full::ChunkNoiseFullState;
use bbb_server_types::chunk_data::spawn::ChunkSpawnState;
use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkFeaturesState, ChunkGenerationState, ChunkInitializeLightState,
    ChunkLightState, ChunkPreCarversState, ChunkPreCarversStatus,
};
use bbb_server_types::save::SaveResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingLegacyGenerationWork {
    upgrade_data: Option<Java26UpgradeData>,
    below_zero_retrogen: Option<Java26BelowZeroRetrogen>,
    retrogen_pre_carvers_completed: bool,
}

impl PendingLegacyGenerationWork {
    pub fn upgrade_data(&self) -> Option<&Java26UpgradeData> {
        self.upgrade_data.as_ref()
    }

    pub fn below_zero_retrogen(&self) -> Option<&Java26BelowZeroRetrogen> {
        self.below_zero_retrogen.as_ref()
    }

    fn from_chunk(data: &Java26ChunkData) -> Option<Self> {
        let work = Self {
            upgrade_data: data.upgrade_data().cloned(),
            below_zero_retrogen: data.below_zero_retrogen().cloned(),
            retrogen_pre_carvers_completed: data.status() >= Java26ChunkStatus::Carvers,
        };
        (work.upgrade_data.is_some() || work.below_zero_retrogen.is_some()).then_some(work)
    }
}

pub fn features_state_from_saved(
    column: &ChunkColumn,
    data: &Java26ChunkData,
) -> SaveResult<ChunkFeaturesState> {
    let stage = match data.status() {
        Java26ChunkStatus::Features => "FEATURES",
        Java26ChunkStatus::InitializeLight => "INITIALIZE_LIGHT",
        Java26ChunkStatus::Light => "LIGHT",
        Java26ChunkStatus::Spawn => "SPAWN",
        Java26ChunkStatus::Full => "FULL",
        status => {
            return Err(invalid_carvers_region(
                [data.x(), data.z()],
                format!("cannot restore FEATURES payload from {}", status.as_str()),
            ))
        }
    };
    // LEVELCHUNK omits the proto carving mask (`SerializableChunkData.java:366-375,461-467`).
    let carving_mask = match (data.status(), data.carving_mask()) {
        (Java26ChunkStatus::Full, None) => &[][..],
        (_, Some(mask)) => mask,
        (_, None) => {
            return Err(invalid_carvers_region(
                [data.x(), data.z()],
                format!("saved {stage} state has no carving mask"),
            ))
        }
    };
    Ok(ChunkFeaturesState {
        postprocessing_positions: restore_postprocessing_positions(column, data)?,
        carving_mask: carving_mask.to_vec(),
        final_heightmaps: FeatureHeightmaps::from_saved(column, data.heightmaps())?,
    })
}

impl ChunkStore {
    pub fn restore_saved_generation_metadata(&mut self, data: &Java26ChunkData) -> SaveResult<()> {
        let chunk = (data.x(), data.z());
        let legacy_work = PendingLegacyGenerationWork::from_chunk(data);
        let Some(column) = self.column_at(chunk) else {
            return Err(invalid_carvers_region(
                [chunk.0, chunk.1],
                "saved generation metadata requires a loaded column",
            ));
        };
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            if data.blending_data().is_some() {
                return Err(invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "saved blending data requires a noise generation store",
                ));
            }
            self.restore_ordinary_generation_metadata(data)?;
            if data.is_light_on()
                && matches!(
                    self.generation(chunk),
                    Some(ChunkGenerationState::Full { .. })
                )
            {
                // `ChunkStatusTasks.java:28-32`: a lit saved FULL chunk keeps its light.
                let sources = self.stage_column_light_sources(chunk)?;
                self.install_saved_light(
                    chunk,
                    data,
                    sources.non_empty_sections(),
                    sources.lowest_sky_source_y(),
                    true,
                )?;
                self.loaded_mut(chunk).saved_light = true;
            }
            let loaded = self.loaded_mut(chunk);
            loaded.blending = Some(None);
            loaded.legacy_work = legacy_work;
            loaded.pending_initial_persistence = false;
            return Ok(());
        };
        let pre_carvers_status = match data.status() {
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
        if let Some(pre_carvers_status) = pre_carvers_status {
            if !self.proto_entity_records(chunk.0, chunk.1).is_empty() {
                return Err(invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "saved ProtoChunk entity destination is not pristine",
                ));
            }
            let proto_entities = data.entities().ok_or_else(|| {
                invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "saved pre-CARVERS generation state has no entities list",
                )
            })?;
            let structures =
                program.restored_structure_chunk_data(data.structures(), chunk.0, chunk.1)?;
            let starts_ready =
                pre_carvers_status.is_at_least(&ChunkPreCarversStatus::StructureStarts);
            let references_ready =
                pre_carvers_status.is_at_least(&ChunkPreCarversStatus::StructureReferences);
            let pre_state = ChunkPreCarversState::new(
                pre_carvers_status,
                restore_postprocessing_positions(column, data)?,
            );
            self.structure_status.restore_chunk_component_at_status(
                structures,
                starts_ready,
                references_ready,
            )?;
            if !self.generation_blending_catalog_complete {
                self.generation_blending_restore_in_progress = true;
            }
            let loaded = self.loaded_mut(chunk);
            loaded.proto_entities = proto_entities.to_vec();
            loaded.generation = None;
            loaded.pre_carvers = Some(pre_state);
            loaded.blending = Some(data.blending_data().cloned());
            loaded.legacy_work = legacy_work;
            loaded.pending_initial_persistence = false;
            loaded.storage_backed_generation = true;
            loaded.storage_backed_full = false;
            return Ok(());
        }
        let state = match data.status() {
            Java26ChunkStatus::Carvers => ChunkGenerationState::Carvers {
                postprocessing_positions: restore_postprocessing_positions(column, data)?,
                carving_mask: data
                    .carving_mask()
                    .ok_or_else(|| {
                        invalid_carvers_region(
                            [chunk.0, chunk.1],
                            "saved CARVERS state has no carving mask",
                        )
                    })?
                    .to_vec(),
            },
            Java26ChunkStatus::Features => {
                ChunkGenerationState::Features(features_state_from_saved(column, data)?)
            }
            Java26ChunkStatus::InitializeLight => {
                let features = features_state_from_saved(column, data)?;
                // `ChunkStatusTasks.java:149-157`; `ChunkAccess.java:471-477`;
                // `ThreadedLevelLightEngine.java:144-162`; `ChunkStatus.java:21-32`.
                ChunkGenerationState::InitializeLight(
                    self.stage_initialize_light_from_features(chunk, features)?,
                )
            }
            Java26ChunkStatus::Light => {
                ChunkGenerationState::Light(restored_light_state(self, chunk, column, data)?)
            }
            // Load SPAWN is a pass-through (`ChunkPyramid.java:52`); do not re-run generateSpawn.
            Java26ChunkStatus::Spawn => ChunkGenerationState::Spawn(ChunkSpawnState::from_light(
                restored_light_state(self, chunk, column, data)?,
                false,
                false,
            )),
            Java26ChunkStatus::Full => {
                if data.entities().is_some_and(|records| !records.is_empty()) {
                    return Err(invalid_carvers_region(
                        [chunk.0, chunk.1],
                        "FULL generation state cannot carry ProtoChunk entities",
                    ));
                }
                ChunkGenerationState::NoiseFull(ChunkNoiseFullState::from_spawn(
                    ChunkSpawnState::from_light(
                        restored_light_state(self, chunk, column, data)?,
                        false,
                        false,
                    ),
                ))
            }
            status => {
                return Err(invalid_carvers_region(
                    [chunk.0, chunk.1],
                    format!(
                        "the storage-only noise store cannot restore {}",
                        status.as_str()
                    ),
                ))
            }
        };
        if !self.proto_entity_records(chunk.0, chunk.1).is_empty() {
            return Err(invalid_carvers_region(
                [chunk.0, chunk.1],
                "saved ProtoChunk entity destination is not pristine",
            ));
        }
        let proto_entities = match data.status() {
            Java26ChunkStatus::Full => &[][..],
            _ => data.entities().ok_or_else(|| {
                invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "saved ProtoChunk generation state has no entities list",
                )
            })?,
        };
        let structures =
            program.restored_structure_chunk_data(data.structures(), chunk.0, chunk.1)?;
        if let Some((initialized, enable_column)) = restored_light_column(&state) {
            let lowest = *initialized.lowest_sky_source_y();
            let non_empty = initialized.non_empty_sections().to_vec();
            self.install_saved_light(chunk, data, &non_empty, &lowest, enable_column)?;
        }
        self.structure_status.restore_chunk_component(structures)?;
        let restored_full = matches!(&state, ChunkGenerationState::NoiseFull(_));
        if !self.generation_blending_catalog_complete {
            self.generation_blending_restore_in_progress = true;
        }
        let loaded = self.loaded_mut(chunk);
        loaded.proto_entities = proto_entities.to_vec();
        loaded.generation = Some(state);
        loaded.pre_carvers = None;
        loaded.blending = Some(data.blending_data().cloned());
        loaded.legacy_work = legacy_work;
        loaded.pending_initial_persistence = false;
        loaded.storage_backed_generation = true;
        // Disk FULL is loaded as the wrapped LevelChunk and does not rerun the status pyramid.
        loaded.storage_backed_full = restored_full;
        Ok(())
    }

    pub fn pending_legacy_generation_work(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Option<&PendingLegacyGenerationWork> {
        self.chunk((chunk_x, chunk_z))
            .and_then(|chunk| chunk.legacy_work.as_ref())
    }

    pub(super) fn require_legacy_generation_work_completed(
        &mut self,
        chunk: (i32, i32),
    ) -> SaveResult<()> {
        self.prepare_generation_admission(chunk)?;
        self.execute_pending_retrogen_pre_carvers(chunk)?;
        self.execute_pending_upgrade_data(chunk)?;
        if let Some(work) = self.pending_legacy_generation_work(chunk.0, chunk.1) {
            // UpgradeData runs inside before sides, queued neighbor ticks and
            // chunky fixers (`UpgradeData.java:90-107,109-190`).
            let owner = if work.upgrade_data().is_some() {
                Some("saved UpgradeData requires the block-owned migration executor")
            } else if !work.retrogen_pre_carvers_completed {
                // `BelowZeroRetrogen.java:63-83,98-107`; `ChunkStatusTasks.java:85-104`.
                Some("saved below_zero_retrogen requires the pre-CARVERS retrogen executor")
            } else {
                None
            };
            if let Some(owner) = owner {
                return Err(invalid_carvers_region(
                    [chunk.0, chunk.1],
                    format!("{owner} before FULL serving"),
                ));
            }
        }
        Ok(())
    }

    fn execute_pending_retrogen_pre_carvers(&mut self, chunk: (i32, i32)) -> SaveResult<()> {
        let needs_retrogen = self.pre_carvers(chunk).is_some()
            && self
                .pending_legacy_generation_work(chunk.0, chunk.1)
                .is_some_and(|work| {
                    work.below_zero_retrogen().is_some() && !work.retrogen_pre_carvers_completed
                });
        if !needs_retrogen {
            return Ok(());
        }
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return Ok(());
        };
        let program = program.clone();
        self.resume_pre_carvers_retrogen_chunk(&program, chunk)
    }

    pub(super) fn complete_pending_upgrade_data(&mut self, chunk: (i32, i32)) {
        self.update_chunk(chunk, |loaded| {
            if let Some(work) = &mut loaded.legacy_work {
                work.upgrade_data = None;
                if work.below_zero_retrogen.is_none() {
                    loaded.legacy_work = None;
                }
            }
        });
    }

    pub(super) fn complete_pending_retrogen_pre_carvers(&mut self, chunk: (i32, i32)) {
        self.update_chunk(chunk, |loaded| {
            if let Some(work) = &mut loaded.legacy_work {
                work.retrogen_pre_carvers_completed = true;
            }
        });
        self.advance_pending_retrogen_status(chunk, Java26ChunkStatus::Carvers);
    }

    pub(super) fn advance_pending_retrogen_status(
        &mut self,
        chunk: (i32, i32),
        completed: Java26ChunkStatus,
    ) {
        self.update_chunk(chunk, |loaded| {
            if let Some(work) = &mut loaded.legacy_work {
                if work.retrogen_pre_carvers_completed
                    && work
                        .below_zero_retrogen
                        .as_ref()
                        .is_some_and(|retrogen| completed >= retrogen.target_status())
                {
                    work.below_zero_retrogen = None;
                }
                if work.upgrade_data.is_none() && work.below_zero_retrogen.is_none() {
                    loaded.legacy_work = None;
                }
            }
        });
    }

    pub fn finish_restored_generation_batch(
        &mut self,
        restored_chunks: &BTreeSet<(i32, i32)>,
    ) -> SaveResult<()> {
        if self.is_ordinary_source() {
            return Ok(());
        }
        // Startup restores every saved chunk and old-generation chunks never
        // unload, so the resident witnesses stay the whole old-chunk catalog.
        self.generation_blending_catalog_complete = true;
        let program = match &self.source {
            ChunkSource::NoiseCarvers(program) => program.clone(),
            ChunkSource::Flat(_) | ChunkSource::Debug(_) => unreachable!(),
        };
        let pending = self
            .chunks
            .iter()
            .filter(|(_, chunk)| {
                chunk.pre_carvers.is_some()
                    && chunk
                        .legacy_work
                        .as_ref()
                        .and_then(PendingLegacyGenerationWork::below_zero_retrogen)
                        .is_some()
            })
            .map(|(&key, _)| key)
            .collect::<BTreeSet<_>>();
        for chunk in pending {
            self.resume_pre_carvers_retrogen_chunk(&program, chunk)?;
        }
        let pending_upgrades = restored_chunks
            .iter()
            .copied()
            .filter(|&(chunk_x, chunk_z)| {
                self.pending_legacy_generation_work(chunk_x, chunk_z)
                    .and_then(PendingLegacyGenerationWork::upgrade_data)
                    .is_some()
            })
            .collect::<Vec<_>>();
        for chunk in pending_upgrades {
            self.execute_pending_upgrade_data(chunk)?;
        }
        self.generation_blending_restore_in_progress = false;
        Ok(())
    }

    fn resume_pre_carvers_retrogen_chunk(
        &mut self,
        program: &super::NoiseFinalDensityProgram,
        chunk: (i32, i32),
    ) -> SaveResult<()> {
        let pre_state = self.pre_carvers(chunk).cloned().ok_or_else(|| {
            invalid_carvers_region([chunk.0, chunk.1], "retrogen status disappeared")
        })?;
        let retrogen = self
            .pending_legacy_generation_work(chunk.0, chunk.1)
            .and_then(PendingLegacyGenerationWork::below_zero_retrogen)
            .cloned()
            .ok_or_else(|| {
                invalid_carvers_region(
                    [chunk.0, chunk.1],
                    "pre-CARVERS state has no below-zero retrogen witness",
                )
            })?;
        let saved_column = self.column_at(chunk).cloned().ok_or_else(|| {
            invalid_carvers_region([chunk.0, chunk.1], "retrogen column disappeared")
        })?;
        self.prepare_structure_window_admission(chunk)?;
        let (references, beardifier) = self
            .structure_status
            .prepare_structure_references(program, chunk.0, chunk.1)?;
        let biome_blender = self.blending_resolver(chunk)?;
        let saved_status = pre_carvers_java_status(*pre_state.status());
        let generated = program.resume_below_zero_retrogen_to_carvers(
            crate::chunk::noise_generator::SavedChunkResume {
                chunk: [chunk.0, chunk.1],
                saved_status,
                saved_column: &saved_column,
                saved_postprocessing: pre_state.postprocessing_positions().to_vec(),
            },
            &retrogen,
            &beardifier,
            Some(&biome_blender),
        )?;
        let (chunk_x, chunk_z, column, state) = generated.into_persistence_parts()?;
        if (chunk_x, chunk_z) != chunk {
            return Err(invalid_carvers_region(
                [chunk.0, chunk.1],
                "retrogen generator returned a different chunk",
            ));
        }
        if references.chunk() != [chunk.0, chunk.1] {
            return Err(invalid_carvers_region(
                [chunk.0, chunk.1],
                "retrogen structure references target a different chunk",
            ));
        }
        let loaded = self.put_column(chunk, column);
        loaded.generation = Some(state);
        loaded.pre_carvers = None;
        self.complete_pending_retrogen_pre_carvers(chunk);
        self.mark_whole_rewrite_pending(chunk)?;
        Ok(())
    }
}

pub(crate) fn pre_carvers_java_status(status: ChunkPreCarversStatus) -> Java26ChunkStatus {
    match status {
        ChunkPreCarversStatus::Empty => Java26ChunkStatus::Empty,
        ChunkPreCarversStatus::StructureStarts => Java26ChunkStatus::StructureStarts,
        ChunkPreCarversStatus::StructureReferences => Java26ChunkStatus::StructureReferences,
        ChunkPreCarversStatus::Biomes => Java26ChunkStatus::Biomes,
        ChunkPreCarversStatus::Noise => Java26ChunkStatus::Noise,
        ChunkPreCarversStatus::Surface => Java26ChunkStatus::Surface,
    }
}

impl ChunkStore {
    /// Installs a saved chunk's light layers and section topology
    /// (`SerializableChunkData.java:221-249`; `ThreadedLevelLightEngine.java:144-162`).
    fn install_saved_light(
        &mut self,
        chunk: (i32, i32),
        data: &Java26ChunkData,
        non_empty_sections: &[i32],
        lowest_sky_source_y: &[i32; 256],
        enable_column: bool,
    ) -> SaveResult<()> {
        let min_light_section_y = (self.min_y() >> 4).checked_sub(1).ok_or_else(|| {
            invalid_carvers_region(
                [chunk.0, chunk.1],
                "minimum restored light section underflows",
            )
        })?;
        let sections = data
            .sections()
            .iter()
            .filter_map(|section| {
                // `SerializableChunkData.java:221-249` refuses saved sky arrays
                // when the live dimension type has no sky light engine.
                let sky_light = if self.has_skylight {
                    section.sky_light.clone()
                } else {
                    None
                };
                (section.block_light.is_some() || sky_light.is_some()).then_some(
                    SavedLightSection {
                        y: i32::from(section.y),
                        block_light: section.block_light.clone(),
                        sky_light,
                    },
                )
            })
            .collect::<Vec<_>>();
        let storage = &mut self.light_storage;
        // Saved arrays are queued before section registration consumes them
        // (`SerializableChunkData.java:227-249`; `SkyLightSectionStorage.java:93-111`).
        storage.queue_saved_sections(chunk, &sections);
        for &section_y in non_empty_sections {
            storage.register_non_empty_section((chunk.0, section_y, chunk.1));
        }
        // Vanilla omits empty paired layers on save (`SerializableChunkData.java:344-353`);
        // retain each saved section as light-only for source-exact native snapshots.
        for section in &sections {
            storage.ensure_empty_section_for_restore((chunk.0, section.y, chunk.1));
        }
        storage.ensure_empty_section_for_restore((chunk.0, min_light_section_y, chunk.1));
        storage.set_column_sources(chunk, lowest_sky_source_y);
        if enable_column {
            if self.has_skylight {
                storage.enable_restored_column(chunk, lowest_sky_source_y);
            } else {
                storage.enable_column(chunk);
            }
        }
        Ok(())
    }
}

pub fn restored_light_state(
    store: &ChunkStore,
    chunk: (i32, i32),
    column: &ChunkColumn,
    data: &Java26ChunkData,
) -> SaveResult<ChunkLightState> {
    let features = features_state_from_saved(column, data)?;
    Ok(ChunkLightState {
        initialized: store.stage_initialize_light_from_features(chunk, features)?,

        is_light_correct: data.is_light_on(),
    })
}

pub fn restored_light_column(
    state: &ChunkGenerationState,
) -> Option<(&ChunkInitializeLightState, bool)> {
    match state {
        ChunkGenerationState::InitializeLight(initialized) => Some((initialized, false)),
        ChunkGenerationState::Light(light) => Some((light.initialized(), light.is_light_correct())),
        ChunkGenerationState::Spawn(spawn) => Some((
            spawn.light().initialized(),
            spawn.light().is_light_correct(),
        )),
        ChunkGenerationState::NoiseFull(full) => {
            Some((full.light().initialized(), full.light().is_light_correct()))
        }
        _ => None,
    }
}

pub fn restore_postprocessing_positions(
    column: &ChunkColumn,
    data: &Java26ChunkData,
) -> SaveResult<Vec<(i32, i32, i32)>> {
    const SECTION_WIDTH: i32 = 16;
    let chunk = [data.x(), data.z()];
    let expected_min_section_y = column.min_y().div_euclid(SECTION_WIDTH);
    if data.min_section_y() != expected_min_section_y
        || data.post_processing().len() != column.section_count()
    {
        return Err(invalid_carvers_region(
            chunk,
            "saved PostProcessing geometry does not match the loaded column",
        ));
    }
    let base_x = data.x().checked_mul(SECTION_WIDTH).ok_or_else(|| {
        invalid_carvers_region(chunk, "saved chunk x overflows block coordinates")
    })?;
    let base_z = data.z().checked_mul(SECTION_WIDTH).ok_or_else(|| {
        invalid_carvers_region(chunk, "saved chunk z overflows block coordinates")
    })?;
    let mut positions = Vec::new();
    for (section, offsets) in data.post_processing().iter().enumerate() {
        let section_y = i32::try_from(section)
            .ok()
            .and_then(|section| section.checked_mul(SECTION_WIDTH))
            .and_then(|offset| column.min_y().checked_add(offset))
            .ok_or_else(|| {
                invalid_carvers_region(chunk, "saved PostProcessing section y overflows")
            })?;
        for &packed in offsets {
            let packed = i32::from(packed);
            positions.push((
                base_x + (packed & 15),
                section_y + ((packed >> 4) & 15),
                base_z + ((packed >> 8) & 15),
            ));
        }
    }
    Ok(positions)
}
