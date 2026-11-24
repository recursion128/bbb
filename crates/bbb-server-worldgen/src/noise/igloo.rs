//! Typed generation and persistence for the multi-piece igloo StructureStart.

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use bbb_protocol::packets::BlockPos;

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawRotation};
use super::structures::StructureTerrainAdjustment;

pub mod persistence;
pub mod start;

pub const IGLOO_TYPE_ID: &str = "minecraft:igloo";
pub const IGLOO_ID: &str = "minecraft:igloo";
pub const IGLOO_SET_ID: &str = "minecraft:igloos";
/// `StructurePieceType.java:61,86-96` lowercases the registered `Iglu` id.
pub const IGLOO_PIECE_ID: &str = "minecraft:iglu";
/// `BuiltInLootTables.IGLOO_CHEST` (`BuiltInLootTables.java:43`).
pub const IGLOO_CHEST_LOOT: &str = "minecraft:chests/igloo_chest";

/// `IglooStructure.java:27-31`.
pub const INITIAL_TEMPLATE_Y: i32 = 90;

/// `IglooPieces.java:35-37`.
pub const TEMPLATE_TOP: &str = "minecraft:igloo/top";
pub const TEMPLATE_MIDDLE: &str = "minecraft:igloo/middle";
pub const TEMPLATE_BOTTOM: &str = "minecraft:igloo/bottom";

/// `IglooPieces.java:38-40`.
pub const PIVOT_TOP: BlockPos = BlockPos { x: 3, y: 5, z: 5 };
pub const PIVOT_MIDDLE: BlockPos = BlockPos { x: 1, y: 3, z: 1 };
pub const PIVOT_BOTTOM: BlockPos = BlockPos { x: 3, y: 6, z: 7 };

/// `IglooPieces.java:41-43`.
pub const OFFSET_TOP: BlockPos = BlockPos { x: 0, y: 0, z: 0 };
pub const OFFSET_MIDDLE: BlockPos = BlockPos { x: 2, y: -3, z: 4 };
pub const OFFSET_BOTTOM: BlockPos = BlockPos { x: 0, y: -3, z: -2 };

pub const IGLOO_TEMPLATES: &[&str] = &[TEMPLATE_TOP, TEMPLATE_MIDDLE, TEMPLATE_BOTTOM];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IglooPiece {
    template: String,
    template_position: BlockPos,
    rotation: JigsawRotation,
    template_size: [i32; 3],
}

impl IglooPiece {
    pub fn new(
        template: impl Into<String>,
        template_position: BlockPos,
        rotation: JigsawRotation,
    ) -> Self {
        Self {
            template: template.into(),
            template_position,
            rotation,
            template_size: [0, 0, 0],
        }
    }

    pub fn with_template_size(mut self, size: [i32; 3]) -> Self {
        self.template_size = size;
        self
    }

    pub const fn template_size(&self) -> [i32; 3] {
        self.template_size
    }

    pub fn set_template_size(&mut self, size: [i32; 3]) {
        self.template_size = size;
    }

    pub fn template(&self) -> &str {
        &self.template
    }

    pub const fn template_position(&self) -> BlockPos {
        self.template_position
    }

    pub fn set_template_position(&mut self, position: BlockPos) {
        self.template_position = position;
    }

    pub const fn rotation(&self) -> JigsawRotation {
        self.rotation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IglooGenerationStub {
    source_chunk: [i32; 2],
    generation_anchor: BlockPos,
    structure_id: String,
    pieces: Vec<IglooPiece>,
}

impl IglooGenerationStub {
    pub fn new(
        source_chunk: [i32; 2],
        generation_anchor: BlockPos,
        structure_id: impl Into<String>,
        pieces: Vec<IglooPiece>,
    ) -> Self {
        Self {
            source_chunk,
            generation_anchor,
            structure_id: structure_id.into(),
            pieces,
        }
    }

    pub const fn source_chunk(&self) -> [i32; 2] {
        self.source_chunk
    }

    pub const fn generation_anchor(&self) -> BlockPos {
        self.generation_anchor
    }

    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub fn pieces(&self) -> &[IglooPiece] {
        &self.pieces
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IglooStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    pieces: Vec<IglooPiece>,
    cached_bounding_box: CachedStartBoundingBox,
}

impl IglooStructureStart {
    pub fn from_generation_stub(stub: IglooGenerationStub, references: i32) -> Self {
        Self {
            structure_id: stub.structure_id,
            start_chunk: stub.source_chunk,
            references,
            terrain_adjustment: StructureTerrainAdjustment::None,
            pieces: stub.pieces,
            cached_bounding_box: CachedStartBoundingBox::new(),
        }
    }

    pub fn from_parts(
        structure_id: impl Into<String>,
        start_chunk: [i32; 2],
        references: i32,
        pieces: Vec<IglooPiece>,
    ) -> Self {
        Self {
            structure_id: structure_id.into(),
            start_chunk,
            references,
            terrain_adjustment: StructureTerrainAdjustment::None,
            pieces,
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

    pub fn pieces(&self) -> &[IglooPiece] {
        &self.pieces
    }

    pub fn pieces_mut(&mut self) -> &mut [IglooPiece] {
        self.cached_bounding_box.invalidate();
        &mut self.pieces
    }

    pub fn bounding_box(&self) -> Option<JigsawBoundingBox> {
        self.cached_bounding_box.get_or_init(|| {
            JigsawBoundingBox::adjusted_union(
                self.pieces.iter().map(|piece| {
                    let size = piece.template_size();
                    let position = piece.template_position();
                    if size == [0, 0, 0] {
                        JigsawBoundingBox::point(position)
                    } else {
                        JigsawBoundingBox::from_array(persistence::piece_bounding_box(
                            piece.template(),
                            position,
                            piece.rotation(),
                            size,
                        ))
                    }
                }),
                self.terrain_adjustment,
            )
        })
    }
}

/// `StructureTemplate.transform` with `Mirror.NONE` (`StructureTemplate.java:528-555`).
pub fn transform_about_pivot(pos: BlockPos, rotation: JigsawRotation, pivot: BlockPos) -> BlockPos {
    match rotation {
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: pivot.x.wrapping_sub(pivot.z).wrapping_add(pos.z),
            y: pos.y,
            z: pivot.x.wrapping_add(pivot.z).wrapping_sub(pos.x),
        },
        JigsawRotation::Clockwise90 => BlockPos {
            x: pivot.x.wrapping_add(pivot.z).wrapping_sub(pos.z),
            y: pos.y,
            z: pivot.z.wrapping_sub(pivot.x).wrapping_add(pos.x),
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: pivot.x.wrapping_add(pivot.x).wrapping_sub(pos.x),
            y: pos.y,
            z: pivot.z.wrapping_add(pivot.z).wrapping_sub(pos.z),
        },
        JigsawRotation::None => pos,
    }
}

pub fn template_pivot(template: &str) -> Option<BlockPos> {
    match template {
        TEMPLATE_TOP => Some(PIVOT_TOP),
        TEMPLATE_MIDDLE => Some(PIVOT_MIDDLE),
        TEMPLATE_BOTTOM => Some(PIVOT_BOTTOM),
        _ => None,
    }
}

pub fn template_offset(template: &str) -> Option<BlockPos> {
    match template {
        TEMPLATE_TOP => Some(OFFSET_TOP),
        TEMPLATE_MIDDLE => Some(OFFSET_MIDDLE),
        TEMPLATE_BOTTOM => Some(OFFSET_BOTTOM),
        _ => None,
    }
}

pub mod placement;
pub use placement::IglooChunkPlacement;
