use serde::{Deserialize, Serialize};

use crate::ChunkPos;

use super::{light::LightData, palette::PalettedContainerData};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEntityRecord {
    pub local_x: u8,
    pub y: i16,
    pub local_z: u8,
    pub type_id: i32,
    pub nbt: Option<NbtPayloadSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NbtPayloadSummary {
    pub root_type: u8,
    pub byte_len: usize,
}
