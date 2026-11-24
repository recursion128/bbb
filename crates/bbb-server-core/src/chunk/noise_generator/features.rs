//! FEATURES over borrowed canonical chunk data.

use std::collections::BTreeSet;

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_support_facts,
    VanillaBlockSupportDirection,
};
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::biome_climate::height_adjusted_temperature;

use crate::block_entity::{
    create_block_entity_on_block_change, remove_block_entity_on_block_change, BlockEntity,
};
use crate::chunk::block_entities::{BlockEntitiesMut, LevelBlockEntities};
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::structure_template::StructureTemplate;
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::{ChunkColumn, ChunkGenerationState};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, FluidFamily};

use crate::chunk::generation_publication::GenerationItemPublication;
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
use bbb_server_worldgen::noise::features::NoiseFeaturesPlan;
use bbb_server_worldgen::noise::mineshaft::{
    MineshaftChestMinecartRecord, MineshaftPlacementLevel,
};
use bbb_server_worldgen::noise::proto_chunk_entity::EndCrystalRecord;
use bbb_server_worldgen::noise::{worldgen_region_random, WorldgenRandom};
use bbb_server_worldgen::noise::{
    EndCityItemFrameRecord, ProtoChunkEntityRecord, StructureMobKind, StructureMobRecord,
    TemplateMobRecord,
};
use bbb_server_worldgen::WorldGenLevel;

use super::{invalid_schema, NoiseFinalDensityProgram, SaveResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructurePlacementFamily {
    Mineshaft,
    BuriedTreasure,
    Shipwreck,
    OceanRuin,
    Igloo,
    EndCity,
    WoodlandMansion,
    RuinedPortal,
    SwampHut,
    DesertPyramid,
    JungleTemple,
    OceanMonument,
    Stronghold,
    Jigsaw,
    Fortress,
    NetherFossil,
}

mod biomes;
mod blending;
mod borrowed;
pub mod jigsaw_invocation;
pub mod ocean_monument_invocation;
mod program;
mod proto_entity_uuid;
mod region_level;
mod resources;
pub mod scattered_invocation;
mod structure_placement;

pub(crate) use resources::GenericFeatureResources;

use bbb_server_datapack::worldgen::BiomeClimate;
use biomes::neighborhood_biomes;
pub(crate) use borrowed::FeatureRegionData;
pub(crate) use program::FeaturePlacementSource;
use proto_entity_uuid::ProtoEntityUuidSequence;
pub(crate) use region_level::FeatureWorldGenRegion;

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

fn region_index(center_x: i32, center_z: i32, chunk_x: i32, chunk_z: i32) -> Option<usize> {
    let offset_x = i64::from(chunk_x) - i64::from(center_x) + 1;
    let offset_z = i64::from(chunk_z) - i64::from(center_z) + 1;
    ((0..3).contains(&offset_x) && (0..3).contains(&offset_z))
        .then_some((offset_z * 3 + offset_x) as usize)
}

#[cfg(test)]
pub mod tests;
