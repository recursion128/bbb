//! The block palette this server's placeholder interactions use, and the world
//! the created save describes.
//!
//! Terrain is no longer here: chunks come from the `minecraft:flat` generator
//! the save declares (`persistence::flat_generator`), and
//! `persistence::world_creation::default_level_sources` is what a save-less
//! server generates from. What is left is the pair of block states `world.rs`
//! still writes by hand for break/place, plus the two facts the created
//! `world_gen_settings.dat` needs to describe that generator: which block the
//! floor is made of and how far up it goes. How *deep* the floor is is not a
//! fixture fact: it is the declared `dimension_type`'s `min_y`, which differs
//! per dimension.

use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{vanilla_block_type, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_STONE_ID},
};

use crate::{Result, ServerWorldError};

/// The highest solid y of the world this server describes: in every dimension
/// the created save declares, the flat generator's layers stack from that
/// dimension's own `min_y` up to and including this block.
const FLOOR_Y: i32 = 63;
// The fixture configuration generation owns a singleton biome palette.
const FIXTURE_PLAINS_BIOME_ID: i32 = 0;

#[derive(Debug, Clone)]
pub struct FlatFixture {
    air_state_id: i32,
    stone_state_id: i32,
}

impl FlatFixture {
    pub fn new() -> Result<Self> {
        let air_state_id = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).ok_or(
            ServerWorldError::MissingCatalogFact("minecraft:air default block state"),
        )?;
        let stone_state_id = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).ok_or(
            ServerWorldError::MissingCatalogFact("minecraft:stone default block state"),
        )?;
        Ok(Self {
            air_state_id,
            stone_state_id,
        })
    }

    pub const fn air_state_id(&self) -> i32 {
        self.air_state_id
    }

    /// The fixture configuration generation owns a singleton biome palette.
    pub const fn biome_id(&self) -> i32 {
        FIXTURE_PLAINS_BIOME_ID
    }

    /// The resource id of the block this world's solid column is made of.
    ///
    /// The created save's flat generator names it, and the running server
    /// generates from that same document, so the two are one fact: a floor
    /// built out of a different block would publish a save describing a world
    /// the server does not serve.
    pub fn floor_block_name(&self) -> &'static str {
        vanilla_block_type(VANILLA_BLOCK_STONE_ID)
            .expect("the stone block type is in the catalog")
            .resource_id
    }

    pub const fn stone_state_id(&self) -> i32 {
        self.stone_state_id
    }

    pub const fn floor_y(&self) -> i32 {
        FLOOR_Y
    }
}
