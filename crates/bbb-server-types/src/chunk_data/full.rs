//! The FULL generation-state payload of a noise column (`ChunkStatus.FULL`,
//! `ChunkStatus.java:32`); the promotion that builds it lives with the store.

use crate::chunk_data::spawn::ChunkSpawnState;
use crate::chunk_data::ChunkLightState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkNoiseFullState {
    spawn: ChunkSpawnState,
}

impl ChunkNoiseFullState {
    pub(crate) fn clear_postprocessing_positions(&mut self) {
        self.spawn.clear_postprocessing_positions();
    }

    pub const fn from_spawn(spawn: ChunkSpawnState) -> Self {
        Self { spawn }
    }

    #[allow(dead_code, reason = "FULL persist reads the SPAWN envelope")]
    pub const fn spawn(&self) -> &ChunkSpawnState {
        &self.spawn
    }

    #[allow(dead_code, reason = "FULL persist reuses the LIGHT payload")]
    pub const fn light(&self) -> &ChunkLightState {
        self.spawn.light()
    }

    pub fn light_mut(&mut self) -> &mut ChunkLightState {
        self.spawn.light_mut()
    }
}
