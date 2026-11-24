//! Typed generation and placement components for the one-piece buried treasure structure.

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use bbb_protocol::packets::BlockPos;

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox};
use super::structures::StructureTerrainAdjustment;

pub mod persistence;
pub mod placement;
pub mod start;

pub const BURIED_TREASURE_ID: &str = "minecraft:buried_treasure";
pub const BURIED_TREASURE_SET_ID: &str = "minecraft:buried_treasures";
pub const BURIED_TREASURE_LOOT_TABLE: &str = "minecraft:chests/buried_treasure";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuriedTreasureGenerationStub {
    structure_id: String,
    source_chunk: [i32; 2],
    generation_anchor: BlockPos,
    terrain_adjustment: StructureTerrainAdjustment,
    piece: BuriedTreasurePiece,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuriedTreasureStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    piece: BuriedTreasurePiece,
    cached_bounding_box: CachedStartBoundingBox,
}

impl BuriedTreasureStructureStart {
    pub fn from_generation_stub(stub: BuriedTreasureGenerationStub, references: i32) -> Self {
        Self {
            structure_id: stub.structure_id,
            start_chunk: stub.source_chunk,
            references,
            terrain_adjustment: stub.terrain_adjustment,
            piece: stub.piece,
            cached_bounding_box: CachedStartBoundingBox::new(),
        }
    }

    pub fn with_terrain_adjustment(
        mut self,
        terrain_adjustment: StructureTerrainAdjustment,
    ) -> Self {
        self.terrain_adjustment = terrain_adjustment;
        self.cached_bounding_box.invalidate();
        self
    }

    pub const fn terrain_adjustment(&self) -> StructureTerrainAdjustment {
        self.terrain_adjustment
    }

    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub const fn start_chunk(&self) -> [i32; 2] {
        self.start_chunk
    }

    pub const fn references(&self) -> i32 {
        self.references
    }

    pub(super) fn try_add_reference(&mut self) -> bool {
        if self.references >= 1 {
            return false;
        }
        self.references = self.references.wrapping_add(1);
        true
    }

    pub const fn piece(&self) -> &BuriedTreasurePiece {
        &self.piece
    }

    pub fn piece_mut(&mut self) -> &mut BuriedTreasurePiece {
        self.cached_bounding_box.invalidate();
        &mut self.piece
    }

    /// `BuriedTreasurePieces.BuriedTreasurePiece.postProcess`
    /// (`BuriedTreasurePieces.java:35-78`).
    pub fn place_in_chunk(
        &mut self,
        level: &mut impl crate::WorldGenLevel,
        loot_seed: i64,
    ) -> Option<BuriedTreasurePlacement> {
        self.piece.place(level, loot_seed)
    }

    pub fn place_in_chunk_with_bounds(
        &mut self,
        level: &mut impl crate::WorldGenLevel,
        bounds: super::structure_pieces::JigsawBoundingBox,
        random: &mut dyn bbb_registry_codec::random_source::RandomSource,
    ) -> Option<BuriedTreasurePlacement> {
        self.piece.place_with_bounds(level, bounds, random)
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                Some(
                    JigsawBoundingBox::point(self.piece.position)
                        .adjusted_for_terrain(self.terrain_adjustment),
                )
            })
            .expect("buried treasure start owns one piece")
    }

    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.bounding_box().intersects_chunk(chunk_x, chunk_z)
    }
}

impl BuriedTreasureGenerationStub {
    pub fn new(
        source_chunk: [i32; 2],
        generation_anchor: BlockPos,
        piece_position: BlockPos,
    ) -> Self {
        Self::new_registered(
            BURIED_TREASURE_ID,
            source_chunk,
            generation_anchor,
            piece_position,
        )
    }

    pub fn new_registered(
        structure_id: impl Into<String>,
        source_chunk: [i32; 2],
        generation_anchor: BlockPos,
        piece_position: BlockPos,
    ) -> Self {
        Self {
            structure_id: structure_id.into(),
            source_chunk,
            generation_anchor,
            terrain_adjustment: StructureTerrainAdjustment::None,
            piece: BuriedTreasurePiece {
                position: piece_position,
            },
        }
    }

    pub const fn with_terrain_adjustment(
        mut self,
        terrain_adjustment: StructureTerrainAdjustment,
    ) -> Self {
        self.terrain_adjustment = terrain_adjustment;
        self
    }

    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub const fn source_chunk(&self) -> [i32; 2] {
        self.source_chunk
    }

    pub const fn generation_anchor(&self) -> BlockPos {
        self.generation_anchor
    }

    pub const fn piece(&self) -> &BuriedTreasurePiece {
        &self.piece
    }

    pub const fn piece_mut(&mut self) -> &mut BuriedTreasurePiece {
        &mut self.piece
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuriedTreasurePiece {
    position: BlockPos,
}

impl BuriedTreasurePiece {
    pub const fn position(&self) -> BlockPos {
        self.position
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuriedTreasurePlacement {
    chest_position: BlockPos,
}

impl BuriedTreasurePlacement {
    pub const fn new(chest_position: BlockPos) -> Self {
        Self { chest_position }
    }

    pub const fn chest_position(self) -> BlockPos {
        self.chest_position
    }
}
