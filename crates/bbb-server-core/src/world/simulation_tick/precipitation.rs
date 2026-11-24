//! `ServerLevel.tickPrecipitation` (`ServerLevel.java:564-594`): water
//! freezing, snow accumulation and cauldron filling at one column's surface.

use bbb_protocol::block_types::{VANILLA_BLOCK_ICE_ID, VANILLA_BLOCK_SNOW_ID};
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::block_queries::{block_type_of, int_property};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, FluidFamily};

use crate::block_behavior::{can_survive_at, cauldron, default_block_state_id, LightLayer};
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::simulation_level::LevelCtx;
use crate::world::weather::{cold_enough_to_snow, precipitation_at, Precipitation};
use bbb_server_datapack::worldgen::BiomeClimate;

const LAYERS: &str = "layers";

pub(super) fn tick_precipitation(level: &mut LevelCtx<'_, '_>, x: i32, z: i32) {
    let Some(height) = level.chunks.final_heightmap_first_available(
        x >> 4,
        z >> 4,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        x & 15,
        z & 15,
    ) else {
        return;
    };
    let top = BlockPos { x, y: height, z };
    let below = BlockPos {
        x,
        y: height - 1,
        z,
    };
    let weather = level.weather;
    let Some(climate) = weather.climate_at(level.chunks, top) else {
        return;
    };
    let sea_level = level.chunks.sea_level();
    if should_freeze(level, climate, below, sea_level) {
        level.set_block(
            below,
            default_block_state_id(VANILLA_BLOCK_ICE_ID),
            SET_BLOCK_UPDATE_ALL,
            UPDATE_LIMIT,
        );
    }
    if !weather.is_raining() {
        return;
    }
    let max_height = level.game_rules.max_snow_accumulation_height;
    if max_height > 0 && should_snow(level, climate, top, sea_level) {
        let state = level.block_state(top);
        let layers = int_property(state, LAYERS).filter(|_| is_snow(state));
        let new_state = match layers {
            Some(layers) if layers < max_height.min(8) => {
                with_property(state, LAYERS, &(layers + 1).to_string())
            }
            Some(_) => None,
            None => Some(default_block_state_id(VANILLA_BLOCK_SNOW_ID)),
        };
        if let Some(new_state) = new_state {
            if layers.is_some() {
                level
                    .item_spawns
                    .entity_push_ups
                    .push((state, new_state, top));
            }
            level.set_block(top, new_state, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
        }
    }
    let precipitation = precipitation_at(climate, below, sea_level);
    if precipitation != Precipitation::None {
        let below_state = level.block_state(below);
        if cauldron::is_cauldron(below_state) {
            cauldron::handle_precipitation(
                level,
                below,
                below_state,
                precipitation == Precipitation::Snow,
            );
        }
    }
}

fn is_snow(state: i32) -> bool {
    block_type_of(state) == Some(VANILLA_BLOCK_SNOW_ID)
}

/// The shared gate of `Biome.shouldFreeze` and `shouldSnow`: inside the build
/// height and block light under 10.
fn dark_enough(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.chunks.is_inside_build_height(pos.y) && level.brightness(LightLayer::Block, pos) < 10
}

/// `Biome.shouldFreeze(level, pos, true)` (`Biome.java:146-170`).
fn should_freeze(
    level: &LevelCtx<'_, '_>,
    climate: BiomeClimate,
    pos: BlockPos,
    sea_level: i32,
) -> bool {
    if !cold_enough_to_snow(climate, pos, sea_level) || !dark_enough(level, pos) {
        return false;
    }
    let state = level.block_state(pos);
    let fluid = block_state_fluid_state(state);
    if !fluid.is_source()
        || fluid.family() != Some(FluidFamily::Water)
        || block_type_of(state) != Some(FluidFamily::Water.liquid_block_type_id())
    {
        return false;
    }
    let is_water_at = |dx: i32, dz: i32| {
        let neighbour = BlockPos {
            x: pos.x + dx,
            y: pos.y,
            z: pos.z + dz,
        };
        level.fluid_state(neighbour).family() == Some(FluidFamily::Water)
    };
    !(is_water_at(-1, 0) && is_water_at(1, 0) && is_water_at(0, -1) && is_water_at(0, 1))
}

/// `Biome.shouldSnow` (`Biome.java:184-197`).
fn should_snow(
    level: &LevelCtx<'_, '_>,
    climate: BiomeClimate,
    pos: BlockPos,
    sea_level: i32,
) -> bool {
    if precipitation_at(climate, pos, sea_level) != Precipitation::Snow || !dark_enough(level, pos)
    {
        return false;
    }
    let state = level.block_state(pos);
    (block_state_is_air(state) || is_snow(state))
        && can_survive_at(default_block_state_id(VANILLA_BLOCK_SNOW_ID), pos, level)
}

#[cfg(test)]
impl crate::world::ServerWorld {
    pub(crate) fn tick_precipitation_for_test(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        x: i32,
        z: i32,
    ) {
        self.with_level(dimension, |level| tick_precipitation(level, x, z));
    }
}
