use std::collections::BTreeMap;

use bbb_protocol::codec::ProtocolError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod block_events;
mod chunks;
mod client_hud;
mod command_suggestions;
mod counters;
mod entities;
mod entity_status;
mod inventory;
mod level;
mod player_info;
mod position;
mod registries;
mod scoreboard;
mod server_presentation;
mod terrain;
mod world_border;

pub use block_events::{BlockDestructionProgress, BlockEventRecord, LevelEventRecord};
pub use chunks::{
    decode_level_chunk_with_light, BlockEntityRecord, ChunkColumn, ChunkSection, ChunkState,
    HeightmapData, LightData, NbtPayloadSummary, PaletteDomain, PaletteKind, PaletteValue,
    PalettedContainerData,
};
pub use client_hud::{BossBarState, ClientHudState, DifficultyState, TabListState};
pub use command_suggestions::{
    CommandSuggestionState, CommandSuggestionsResultState, CommandSuggestionsState,
};
pub use counters::WorldCounters;
pub use entities::{EntityState, EntityVec3, VehicleMoveReport};
pub use entity_status::{EntityDamageEventState, ItemCooldownState, MobEffectState};
pub use inventory::{
    ContainerDataValue, ContainerSlot, ContainerState, InventorySlot, InventoryState,
};
pub use level::{WorldDimension, WorldLevelInfo};
pub use player_info::{PlayerInfoEntryState, PlayerInfoProfileState, PlayerInfoState};
pub use position::{BlockPos, ChunkPos};
pub use registries::{BlockStateInfo, BlockStateRegistry, RegistryPacket, RegistrySet};
pub use scoreboard::{
    ScoreboardObjective, ScoreboardScore, ScoreboardState, ScoreboardTeam, ScoreboardTeamParameters,
};
pub use server_presentation::{ResourcePackState, ServerDataState, ServerPresentationState};
pub use terrain::{
    BlockProbe, TerrainBlockCell, TerrainChunkSnapshot, TerrainChunkSummary, TerrainLight,
    TerrainMaterialClass,
};
pub use world_border::WorldBorderState;

pub(crate) use position::{protocol_block_pos, section_biome_index, section_block_index};

#[derive(Debug, Error)]
pub enum WorldDecodeError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("invalid paletted container bits_per_entry {0}")]
    InvalidPalettedBits(u8),
    #[error("chunk section buffer has {actual} bytes, max is {max}")]
    ChunkSectionBufferTooLarge { actual: usize, max: usize },
    #[error("byte array has {actual} bytes, max is {max}")]
    ByteArrayTooLarge { actual: usize, max: usize },
    #[error("biome update has {remaining} trailing bytes")]
    TrailingBiomeData { remaining: usize },
    #[error("block entity data has {remaining} trailing bytes")]
    TrailingBlockEntityData { remaining: usize },
    #[error("negative NBT array length {0}")]
    NegativeNbtArrayLength(i32),
    #[error("invalid NBT tag id {0}")]
    InvalidNbtTag(u8),
}

pub type Result<T> = std::result::Result<T, WorldDecodeError>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStore {
    dimension: WorldDimension,
    level: Option<WorldLevelInfo>,
    #[serde(default)]
    world_border: WorldBorderState,
    registries: RegistrySet,
    chunks: Vec<ChunkColumn>,
    #[serde(default)]
    block_destructions: Vec<BlockDestructionProgress>,
    #[serde(default)]
    block_events: Vec<BlockEventRecord>,
    #[serde(default)]
    level_events: Vec<LevelEventRecord>,
    entities: Vec<EntityState>,
    #[serde(default)]
    scoreboard: ScoreboardState,
    #[serde(default)]
    client_hud: ClientHudState,
    #[serde(default)]
    player_info: PlayerInfoState,
    #[serde(default)]
    presentation: ServerPresentationState,
    #[serde(default)]
    cooldowns: BTreeMap<String, ItemCooldownState>,
    #[serde(default)]
    command_suggestions: CommandSuggestionsState,
    #[serde(default)]
    local_player_id: Option<i32>,
    #[serde(default)]
    local_player_vehicle_id: Option<i32>,
    inventory: InventoryState,
    counters: WorldCounters,
}

impl WorldStore {
    pub fn new() -> Self {
        Self {
            registries: RegistrySet::vanilla_26_1(),
            ..Self::default()
        }
    }
}
