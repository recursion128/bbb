use bbb_world::{BlockProbe, ChunkColumn, ChunkPos, ChunkState, WorldCounters, WorldStore};
use serde::{Deserialize, Serialize};

use super::ConnectionState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub reached_state: ConnectionState,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    pub registries_seen: usize,
    #[serde(default)]
    pub registry_entries_seen: usize,
    #[serde(default)]
    pub registry_entries_with_data: usize,
    #[serde(default)]
    pub registry_entry_stubs: usize,
    #[serde(default)]
    pub registry_entry_payload_bytes: usize,
    #[serde(default)]
    pub registry_content_registries_tracked: usize,
    #[serde(default)]
    pub registry_duplicate_entries: usize,
    pub first_chunk: Option<ChunkPos>,
    pub first_chunk_summary: Option<ChunkProbeSummary>,
    pub first_chunk_center_block: Option<BlockProbe>,
    pub world_counters: WorldCounters,
    #[serde(skip)]
    pub world: WorldStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkProbeSummary {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: usize,
    pub sections: usize,
    pub block_entities: usize,
    pub sky_light_arrays: usize,
    pub block_light_arrays: usize,
}

impl ChunkProbeSummary {
    pub(crate) fn from_column(column: &ChunkColumn) -> Self {
        Self {
            pos: column.pos,
            state: column.state,
            heightmaps: column.heightmaps.len(),
            sections: column.sections.len(),
            block_entities: column.block_entities.len(),
            sky_light_arrays: column.light.sky_updates.len(),
            block_light_arrays: column.light.block_updates.len(),
        }
    }
}
