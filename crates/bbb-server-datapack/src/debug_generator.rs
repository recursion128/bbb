//! The `minecraft:debug` chunk generator backed by the Vanilla 26.1 block-state registry.

use bbb_protocol::block_state_types::{vanilla_block_default_state_id, VANILLA_BLOCK_STATE_COUNT};
use bbb_protocol::block_types::{VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_BARRIER_ID};
use bbb_registry_codec::registry_codec::DimensionTypeScalars;
use bbb_server_types::chunk_data::ChunkColumn;

use bbb_server_types::save::{SaveError, SaveResult};
use bbb_server_types::schema_support::invalid_schema;

const SCHEMA: &str = "debug_generator";
const SECTION_HEIGHT: i32 = 16;
const GRID_WIDTH: i32 = 173;
const GRID_HEIGHT: i32 = 173;
const BARRIER_HEIGHT: i32 = 60;
const DISPLAY_HEIGHT: i32 = 70;
const DEBUG_SEA_LEVEL: i32 = 63;
const DEBUG_SPAWN_HEIGHT: i32 = 64;

pub const DEBUG_GENERATOR_ID: &str = "minecraft:debug";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugLevelSource {
    min_y: i32,
    height: i32,
    air_block_state_id: i32,
    barrier_block_state_id: i32,
}

impl DebugLevelSource {
    pub fn new(min_y: i32, height: i32) -> SaveResult<Self> {
        DimensionTypeScalars::check_shape(min_y, height, None)
            .map_err(|message| invalid_schema(SCHEMA, message))?;
        Ok(Self {
            min_y,
            height,
            air_block_state_id: default_state(VANILLA_BLOCK_AIR_ID)?,
            barrier_block_state_id: default_state(VANILLA_BLOCK_BARRIER_ID)?,
        })
    }

    pub const fn min_y(&self) -> i32 {
        self.min_y
    }

    pub const fn height(&self) -> i32 {
        self.height
    }

    /// `DebugLevelSource.getSeaLevel` (`DebugLevelSource.java:140-142`).
    pub const fn sea_level(&self) -> i32 {
        DEBUG_SEA_LEVEL
    }

    pub const fn spawn_height(&self) -> i32 {
        DEBUG_SPAWN_HEIGHT
    }

    /// The global debug-level view bypasses stored sections
    /// (`LevelChunk.java:213-226`).
    pub fn block_state_at(&self, world_x: i32, y: i32, world_z: i32) -> i32 {
        match y {
            BARRIER_HEIGHT => self.barrier_block_state_id,
            DISPLAY_HEIGHT => self.grid_state_at(world_x, world_z),
            _ => self.air_block_state_id,
        }
    }

    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32, plains_id: i32) -> ChunkColumn {
        self.generate_chunk_with_biome_registry(
            chunk_x,
            chunk_z,
            plains_id,
            bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
        )
    }

    /// `DebugLevelSource.applyBiomeDecoration` (`DebugLevelSource.java:56-75`).
    pub fn generate_chunk_with_biome_registry(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        plains_id: i32,
        biome_registry_len: usize,
    ) -> ChunkColumn {
        let section_count = usize::try_from(self.height / SECTION_HEIGHT).unwrap_or(0);
        let mut column = ChunkColumn::new_with_biome_registry(
            self.min_y,
            section_count,
            self.air_block_state_id,
            plains_id,
            biome_registry_len,
        );
        for x in 0..16 {
            for z in 0..16 {
                let world_x = chunk_x.wrapping_mul(16).wrapping_add(x);
                let world_z = chunk_z.wrapping_mul(16).wrapping_add(z);
                let barrier = self.block_state_at(world_x, BARRIER_HEIGHT, world_z);
                let display = self.block_state_at(world_x, DISPLAY_HEIGHT, world_z);
                let _ = column.set_block_state(x, BARRIER_HEIGHT, z, barrier);
                let _ = column.set_block_state(x, DISPLAY_HEIGHT, z, display);
            }
        }
        column
    }

    /// The list and its ids share the nested block/state registry order
    /// (`DebugLevelSource.java:31-44`; `DebugLevelSource.java:98-112`;
    /// `Blocks.java:7054-7059`).
    fn grid_state_at(&self, world_x: i32, world_z: i32) -> i32 {
        if world_x <= 0 || world_z <= 0 || world_x % 2 == 0 || world_z % 2 == 0 {
            return self.air_block_state_id;
        }
        let grid_x = world_x / 2;
        let grid_z = world_z / 2;
        if grid_x > GRID_WIDTH || grid_z > GRID_HEIGHT {
            return self.air_block_state_id;
        }
        let index = grid_x * GRID_WIDTH + grid_z;
        if index < VANILLA_BLOCK_STATE_COUNT {
            index
        } else {
            self.air_block_state_id
        }
    }
}

fn default_state(block_type_id: i32) -> SaveResult<i32> {
    vanilla_block_default_state_id(block_type_id).ok_or_else(missing_default_state)
}

fn missing_default_state() -> SaveError {
    invalid_schema(
        SCHEMA,
        "the generated Vanilla 26.1 catalog is missing a block default state",
    )
}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_BARRIER_ID;

    use super::*;

    #[test]
    fn grid_uses_global_coordinates_and_registry_state_ids() {
        let source = DebugLevelSource::new(0, 384).unwrap();
        let origin = source.generate_chunk_with_biome_registry(0, 0, 0, 1);
        assert_eq!(origin.get_block_state(1, 70, 1), Some(0));
        assert_eq!(origin.get_block_state(3, 70, 1), Some(173));
        assert_eq!(origin.get_block_state(2, 70, 1), Some(0));

        let edge = source.generate_chunk_with_biome_registry(21, 14, 0, 1);
        assert_eq!(edge.get_block_state(9, 70, 9), Some(29_872));
        assert_eq!(edge.get_block_state(8, 70, 9), Some(0));
    }

    #[test]
    fn global_view_uses_only_the_two_debug_heights_outside_dimension_bounds() {
        let source = DebugLevelSource::new(-64, 16).unwrap();
        let barrier = vanilla_block_default_state_id(VANILLA_BLOCK_BARRIER_ID).unwrap();

        assert_eq!(source.block_state_at(3, 59, 1), 0);
        assert_eq!(source.block_state_at(3, 60, 1), barrier);
        assert_eq!(source.block_state_at(3, 61, 1), 0);
        assert_eq!(source.block_state_at(3, 70, 1), 173);
        assert_eq!(source.block_state_at(3, 71, 1), 0);
    }

    #[test]
    fn every_chunk_has_barrier_floor_and_negative_chunks_have_no_display_states() {
        let source = DebugLevelSource::new(0, 384).unwrap();
        let barrier = vanilla_block_default_state_id(VANILLA_BLOCK_BARRIER_ID).unwrap();
        let negative = source.generate_chunk_with_biome_registry(-1, -1, 0, 1);
        let positive = source.generate_chunk_with_biome_registry(1, 1, 0, 1);

        for column in [&negative, &positive] {
            assert_eq!(column.get_block_state(0, 60, 0), Some(barrier));
            assert_eq!(column.get_block_state(15, 60, 15), Some(barrier));
        }
        assert_eq!(negative.get_block_state(15, 70, 15), Some(0));
        assert_ne!(positive.get_block_state(1, 70, 1), Some(0));
    }
}
