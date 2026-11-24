//! Ordered placement for one typed mineshaft `StructureStart`.

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use crate::noise::structure_pieces::JigsawBoundingBox;

use super::placement::writable_chunk_box;
use super::{
    MineshaftPieceKind, MineshaftPiecePlacement, MineshaftPlacementLevel, MineshaftStructureStart,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MineshaftStartChunkPlacement {
    reference_position: BlockPos,
    attempted_piece_indices: Vec<usize>,
    placed_piece_indices: Vec<usize>,
    invalid_piece_indices: Vec<usize>,
}

impl MineshaftStartChunkPlacement {
    pub const fn reference_position(&self) -> BlockPos {
        self.reference_position
    }

    pub fn attempted_piece_indices(&self) -> &[usize] {
        &self.attempted_piece_indices
    }

    pub fn placed_piece_indices(&self) -> &[usize] {
        &self.placed_piece_indices
    }

    pub fn invalid_piece_indices(&self) -> &[usize] {
        &self.invalid_piece_indices
    }
}

impl MineshaftStructureStart {
    /// `StructureStart.placeInChunk` order for the mineshaft piece family;
    /// its inherited `Structure.afterPlace` hook is empty
    /// (`StructureStart.java:79-99`, `Structure.java:143-152`).
    pub fn place_in_chunk<L: MineshaftPlacementLevel>(
        &mut self,
        level: &mut L,
        random: &mut dyn RandomSource,
        chunk: [i32; 2],
    ) -> MineshaftStartChunkPlacement {
        let bounds = writable_chunk_box(level, chunk);
        self.place_in_chunk_with_bounds(level, random, bounds)
    }

    /// `PlaceCommand.java:300-313`, `StructureStart.java:79-99`.
    pub fn place_in_chunk_with_bounds<L: MineshaftPlacementLevel>(
        &mut self,
        level: &mut L,
        random: &mut dyn RandomSource,
        chunk_box: JigsawBoundingBox,
    ) -> MineshaftStartChunkPlacement {
        let first = self.pieces[0].bounding_box;
        let reference_position = BlockPos {
            x: first.min.x + (first.max.x - first.min.x + 1) / 2,
            y: first.min.y,
            z: first.min.z + (first.max.z - first.min.z + 1) / 2,
        };
        let mut report = MineshaftStartChunkPlacement {
            reference_position,
            attempted_piece_indices: Vec::new(),
            placed_piece_indices: Vec::new(),
            invalid_piece_indices: Vec::new(),
        };
        for (index, piece) in self.pieces.iter_mut().enumerate() {
            if !piece.bounding_box.intersects(chunk_box) {
                continue;
            }
            report.attempted_piece_indices.push(index);
            let placement = if matches!(&piece.kind, MineshaftPieceKind::Corridor { .. }) {
                piece.place_corridor_in_chunk(level, random, chunk_box)
            } else {
                piece.place_simple_in_chunk(level, chunk_box)
            };
            match placement {
                MineshaftPiecePlacement::Placed => report.placed_piece_indices.push(index),
                MineshaftPiecePlacement::InvalidLocation => {
                    report.invalid_piece_indices.push(index)
                }
            }
        }
        report
    }
}
