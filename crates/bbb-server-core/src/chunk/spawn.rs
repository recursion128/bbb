//! Original mob population and LIGHT → SPAWN promotion.

use super::{ChunkSource, ChunkStore};
use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::chunk_data::ChunkGenerationState;
use bbb_server_types::save::{SaveError, SaveResult};

use bbb_server_types::chunk_data::spawn::ChunkSpawnState;

pub type OriginalMobPopulation<'a> =
    dyn FnMut(&ChunkStore, (i32, i32)) -> SaveResult<Vec<Vec<bbb_protocol::NbtSummaryEntry>>> + 'a;

impl ChunkStore {
    pub fn try_ensure_served_full_columns_with_population(
        &mut self,
        centers: &[(i32, i32)],
        population: &mut OriginalMobPopulation<'_>,
    ) -> SaveResult<()> {
        if self.is_ordinary_source() {
            self.ensure_ordinary_full_columns(centers)
        } else {
            self.ensure_noise_full_columns_populated(centers, Some(population))
        }
    }

    pub(super) fn spawn_chunk_with_population(
        &mut self,
        center: (i32, i32),
        population: Option<&mut OriginalMobPopulation<'_>>,
    ) -> SaveResult<()> {
        self.advance_spawn_chunk_impl(center, population)
    }

    /// Advances one correct LIGHT center to SPAWN.
    /// `ChunkStatusTasks.java:166-174`; `NoiseBasedChunkGenerator.java:488-496`.
    pub fn spawn_chunk(&mut self, center: (i32, i32)) -> SaveResult<()> {
        self.advance_spawn_chunk(center)
    }

    pub fn advance_spawn_chunk(&mut self, center: (i32, i32)) -> SaveResult<()> {
        self.advance_spawn_chunk_impl(center, None)
    }

    fn advance_spawn_chunk_impl(
        &mut self,
        center: (i32, i32),
        population: Option<&mut OriginalMobPopulation<'_>>,
    ) -> SaveResult<()> {
        let Some(state) = self.generation(center) else {
            return Err(invalid_spawn(center, "the center has no generation state"));
        };
        let ChunkGenerationState::Light(light) = state else {
            return Err(invalid_spawn(
                center,
                "SPAWN requires a correct LIGHT center",
            ));
        };
        if !light.is_light_correct() {
            return Err(invalid_spawn(center, "SPAWN requires isLightCorrect LIGHT"));
        }
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return Err(invalid_spawn(
                center,
                "SPAWN is a noise ProtoChunk owner, not a flat FULL column",
            ));
        };
        let disable_mob_generation = program.disable_mob_generation();
        let light = light.clone();
        // ChunkStatusTasks.java:166-174; ChunkAccess.java:459-465.
        let upgrading = self
            .pending_legacy_generation_work(center.0, center.1)
            .is_some_and(|work| work.below_zero_retrogen().is_some());
        let (original_mobs_refused, records) = if disable_mob_generation || upgrading {
            (false, Vec::new())
        } else if let Some(population) = population {
            (false, population(self, center)?)
        } else {
            (true, Vec::new())
        };
        let spawn =
            ChunkSpawnState::from_light(light, disable_mob_generation, original_mobs_refused);
        let loaded = self.loaded_mut(center);
        loaded.proto_entities.extend(records);
        loaded.generation = Some(ChunkGenerationState::Spawn(spawn));
        self.advance_pending_retrogen_status(center, Java26ChunkStatus::Spawn);
        Ok(())
    }
}

pub fn invalid_spawn(center: (i32, i32), detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("SPAWN [{},{}]: {}", center.0, center.1, detail.into()),
    }
}
