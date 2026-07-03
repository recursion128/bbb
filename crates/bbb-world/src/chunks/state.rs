use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ChunkPos;

use super::{light::LightData, palette::PalettedContainerData};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkViewState {
    pub center: Option<ChunkPos>,
    pub radius: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkState {
    Missing,
    Received,
    Decoded,
    NeighborsReady,
    MeshPending,
    MeshBuilding,
    MeshReady,
    GpuUploading,
    GpuResidentHidden,
    Visible,
    Retiring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkColumn {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: Vec<HeightmapData>,
    pub sections: Vec<ChunkSection>,
    pub block_entities: Vec<BlockEntityRecord>,
    pub light: LightData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkProbeSummaryState {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: usize,
    pub sections: usize,
    pub block_entities: usize,
    pub sky_light_arrays: usize,
    pub block_light_arrays: usize,
}

impl ChunkProbeSummaryState {
    pub(crate) fn from_chunk(chunk: &ChunkColumn) -> Self {
        Self {
            pos: chunk.pos,
            state: chunk.state,
            heightmaps: chunk.heightmaps.len(),
            sections: chunk.sections.len(),
            block_entities: chunk.block_entities.len(),
            sky_light_arrays: chunk.light.sky_updates.len(),
            block_light_arrays: chunk.light.block_updates.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeightmapData {
    pub kind_id: i32,
    pub data: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkSection {
    pub non_empty_block_count: i16,
    pub fluid_count: i16,
    pub block_states: PalettedContainerData,
    pub biomes: PalettedContainerData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockEntityRecord {
    pub local_x: u8,
    pub y: i16,
    pub local_z: u8,
    pub type_id: i32,
    pub nbt: Option<NbtPayloadSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sign_text: Option<SignBlockEntityTextState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_shared_data: Option<VaultSharedDataState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignBlockEntityTextState {
    pub front: [String; 4],
    pub back: [String; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultSharedDataState {
    pub connected_players: Vec<Uuid>,
    pub connected_particles_range: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultConnectionParticleState {
    pub origin: [f64; 3],
    pub targets: Vec<VaultConnectionParticleTargetState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VaultConnectionParticleTargetState {
    pub entity_id: i32,
    pub uuid: Uuid,
    pub target_position: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NbtPayloadSummary {
    pub root_type: u8,
    pub byte_len: usize,
}
