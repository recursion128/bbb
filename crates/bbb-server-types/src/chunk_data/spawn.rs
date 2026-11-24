//! The SPAWN generation-state payload (`ChunkStatus.SPAWN`,
//! `ChunkStatus.java:31`); the promotion that builds it lives with the store.

use crate::chunk_data::ChunkLightState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSpawnState {
    light: ChunkLightState,
    disable_mob_generation: bool,
    original_mobs_refused: bool,
}

impl ChunkSpawnState {
    pub fn into_light(self) -> ChunkLightState {
        self.light
    }

    pub(crate) fn clear_postprocessing_positions(&mut self) {
        self.light
            .initialized
            .features
            .postprocessing_positions
            .clear();
    }

    pub const fn from_light(
        light: ChunkLightState,
        disable_mob_generation: bool,
        original_mobs_refused: bool,
    ) -> Self {
        Self {
            light,
            disable_mob_generation,
            original_mobs_refused,
        }
    }

    #[allow(dead_code, reason = "FULL persist reuses the LIGHT payload")]
    pub const fn light(&self) -> &ChunkLightState {
        &self.light
    }

    pub fn light_mut(&mut self) -> &mut ChunkLightState {
        &mut self.light
    }

    #[allow(dead_code, reason = "SPAWN publication and persist read this gate")]
    pub const fn disable_mob_generation(&self) -> bool {
        self.disable_mob_generation
    }

    #[allow(dead_code, reason = "SPAWN publication exposes the counted refusal")]
    pub const fn original_mobs_refused(&self) -> bool {
        self.original_mobs_refused
    }
}
