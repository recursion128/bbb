//! FEATURES-stage start placement for jigsaw.

use bbb_registry_codec::random_source::RandomSource;

use crate::noise::structure_piece_placement::{place_jigsaw_pieces_in_chunk, JigsawChunkPlacement};
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;
use bbb_server_datapack::startup::DatapackStartup;

use super::JigsawStructureStart;

impl JigsawStructureStart {
    /// Piece-union bounds, including the twelve-block terrain-adapting inflate
    /// (`Structure.java:75-81`).
    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.bounding_box().intersects_chunk(chunk_x, chunk_z)
    }

    /// `StructureStart.placeInChunk` (`StructureStart.java:79-99`). Jigsaw
    /// `afterPlace` is empty (`Structure.java:143-152`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &self,
        startup: &DatapackStartup,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> JigsawChunkPlacement {
        place_jigsaw_pieces_in_chunk(
            &self.pieces,
            self.liquid_settings,
            startup,
            level,
            chunk_bounds,
            random,
        )
    }
}
