use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::TagLoadReport;
use bbb_server_types::chunk_data::ChunkColumn;

use super::super::biome::{NoiseBiomeSearch, NoiseBiomeSource};
use super::super::density::{NoiseDensityError, NoiseDensitySource};
use super::super::surface::NoiseSurfaceProgram;
use super::{error, OverworldCarverProgram};

pub const NETHER_CARVER_ID: &str = "minecraft:nether_cave";
pub const NETHER_REPLACEABLE_TAG: &str = "minecraft:nether_carver_replaceables";
pub const NETHER_BIOMES: [&str; 5] = [
    "minecraft:basalt_deltas",
    "minecraft:crimson_forest",
    "minecraft:nether_wastes",
    "minecraft:soul_sand_valley",
    "minecraft:warped_forest",
];

pub struct NetherCarverProgram {
    world_seed: i64,
    program: OverworldCarverProgram,
}

impl NetherCarverProgram {
    pub fn from_registry(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        world_seed: i64,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
    ) -> Result<Self, NoiseDensityError> {
        validate_source(
            source,
            biome_source,
            world_seed,
            "noise_settings.minecraft:nether.carvers",
        )?;
        Ok(Self {
            world_seed,
            program: OverworldCarverProgram::from_registry(
                generation,
                tags,
                world_seed,
                &NETHER_BIOMES,
            )?,
        })
    }

    pub fn validate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Result<(), NoiseDensityError> {
        self.program.validate_chunk(chunk_x, chunk_z)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        surface: &NoiseSurfaceProgram,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
    ) -> Result<Vec<i64>, NoiseDensityError> {
        validate_source(
            source,
            biome_source,
            self.world_seed,
            "noise_settings.minecraft:nether.carvers.apply",
        )?;
        self.program.apply(
            source,
            biome_source,
            surface,
            chunk_x,
            chunk_z,
            column,
            ocean_floor_wg,
            world_surface_wg,
            postprocessing,
        )
    }
}

fn validate_source(
    source: &NoiseDensitySource,
    biome_source: &NoiseBiomeSource,
    world_seed: i64,
    path: &str,
) -> Result<(), NoiseDensityError> {
    if source.min_y() != 0
        || source.height() != 128
        || source.aquifers_enabled()
        || source.ore_veins_enabled()
        || source.world_seed() != world_seed
    {
        return Err(error(
            path,
            "carver runtime owns only the compatible Nether generation shape and seed",
        ));
    }
    if !matches!(biome_source.create_search(), NoiseBiomeSearch::Nether(_)) {
        return Err(error(
            path,
            "carver runtime requires the built-in Nether biome source",
        ));
    }
    Ok(())
}
