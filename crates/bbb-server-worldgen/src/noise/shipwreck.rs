//! Typed generation, placement and persistence for the one-piece shipwreck structure.

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use bbb_protocol::packets::BlockPos;

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawRotation};
use super::structures::StructureTerrainAdjustment;

pub mod persistence;
pub mod start;

pub const SHIPWRECK_TYPE_ID: &str = "minecraft:shipwreck";
pub const SHIPWRECK_ID: &str = "minecraft:shipwreck";
pub const SHIPWRECK_BEACHED_ID: &str = "minecraft:shipwreck_beached";
pub const SHIPWRECK_SET_ID: &str = "minecraft:shipwrecks";
/// `StructurePieceType.java:80-91` lowercases the registered `Shipwreck` id.
pub const SHIPWRECK_PIECE_ID: &str = "minecraft:shipwreck";
pub const SHIPWRECK_MAP_LOOT: &str = "minecraft:chests/shipwreck_map";
pub const SHIPWRECK_TREASURE_LOOT: &str = "minecraft:chests/shipwreck_treasure";
pub const SHIPWRECK_SUPPLY_LOOT: &str = "minecraft:chests/shipwreck_supply";

/// `ShipwreckPieces.java:33`.
pub const SHIPWRECK_PIVOT: BlockPos = BlockPos { x: 4, y: 0, z: 15 };
/// `ShipwreckPieces.java:32`.
pub const WORLD_GEN_REGION_MAX: i32 = 32;
/// `ShipwreckStructure.java:34`.
pub const INITIAL_TEMPLATE_Y: i32 = 90;

/// `ShipwreckPieces.java:34-46`.
pub const SHIPWRECK_BEACHED_TEMPLATES: &[&str] = &[
    "minecraft:shipwreck/with_mast",
    "minecraft:shipwreck/sideways_full",
    "minecraft:shipwreck/sideways_fronthalf",
    "minecraft:shipwreck/sideways_backhalf",
    "minecraft:shipwreck/rightsideup_full",
    "minecraft:shipwreck/rightsideup_fronthalf",
    "minecraft:shipwreck/rightsideup_backhalf",
    "minecraft:shipwreck/with_mast_degraded",
    "minecraft:shipwreck/rightsideup_full_degraded",
    "minecraft:shipwreck/rightsideup_fronthalf_degraded",
    "minecraft:shipwreck/rightsideup_backhalf_degraded",
];

/// `ShipwreckPieces.java:47-68`.
pub const SHIPWRECK_OCEAN_TEMPLATES: &[&str] = &[
    "minecraft:shipwreck/with_mast",
    "minecraft:shipwreck/upsidedown_full",
    "minecraft:shipwreck/upsidedown_fronthalf",
    "minecraft:shipwreck/upsidedown_backhalf",
    "minecraft:shipwreck/sideways_full",
    "minecraft:shipwreck/sideways_fronthalf",
    "minecraft:shipwreck/sideways_backhalf",
    "minecraft:shipwreck/rightsideup_full",
    "minecraft:shipwreck/rightsideup_fronthalf",
    "minecraft:shipwreck/rightsideup_backhalf",
    "minecraft:shipwreck/with_mast_degraded",
    "minecraft:shipwreck/upsidedown_full_degraded",
    "minecraft:shipwreck/upsidedown_fronthalf_degraded",
    "minecraft:shipwreck/upsidedown_backhalf_degraded",
    "minecraft:shipwreck/sideways_full_degraded",
    "minecraft:shipwreck/sideways_fronthalf_degraded",
    "minecraft:shipwreck/sideways_backhalf_degraded",
    "minecraft:shipwreck/rightsideup_full_degraded",
    "minecraft:shipwreck/rightsideup_fronthalf_degraded",
    "minecraft:shipwreck/rightsideup_backhalf_degraded",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipwreckStructureSettings {
    pub is_beached: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipwreckPiece {
    template: String,
    template_position: BlockPos,
    rotation: JigsawRotation,
    is_beached: bool,
    height_adjusted: bool,
    template_size: [i32; 3],
}

impl ShipwreckPiece {
    pub fn new(
        template: impl Into<String>,
        template_position: BlockPos,
        rotation: JigsawRotation,
        is_beached: bool,
    ) -> Self {
        Self {
            template: template.into(),
            template_position,
            rotation,
            is_beached,
            height_adjusted: false,
            template_size: [0, 0, 0],
        }
    }

    pub fn from_parts(
        template: impl Into<String>,
        template_position: BlockPos,
        rotation: JigsawRotation,
        is_beached: bool,
        height_adjusted: bool,
    ) -> Self {
        Self {
            template: template.into(),
            template_position,
            rotation,
            is_beached,
            height_adjusted,
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

    pub const fn rotation(&self) -> JigsawRotation {
        self.rotation
    }

    pub const fn is_beached(&self) -> bool {
        self.is_beached
    }

    pub const fn height_adjusted(&self) -> bool {
        self.height_adjusted
    }

    /// `ShipwreckPieces.java:173-176`.
    pub const fn is_too_big_to_fit_in_world_gen_region(size: [i32; 3]) -> bool {
        size[0] > WORLD_GEN_REGION_MAX || size[1] > WORLD_GEN_REGION_MAX
    }

    /// `ShipwreckPieces.java:178-180`.
    pub fn calculate_beached_position(min_y: i32, size_y: i32, extra: i32) -> i32 {
        min_y - size_y / 2 - extra
    }

    /// `ShipwreckPieces.java:182-185`.
    pub fn adjust_position_height(&mut self, new_height: i32) {
        self.height_adjusted = true;
        self.template_position.y = new_height;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipwreckGenerationStub {
    source_chunk: [i32; 2],
    generation_anchor: BlockPos,
    structure_id: String,
    piece: ShipwreckPiece,
}

impl ShipwreckGenerationStub {
    pub fn new(
        source_chunk: [i32; 2],
        generation_anchor: BlockPos,
        structure_id: impl Into<String>,
        piece: ShipwreckPiece,
    ) -> Self {
        Self {
            source_chunk,
            generation_anchor,
            structure_id: structure_id.into(),
            piece,
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

    pub fn piece(&self) -> &ShipwreckPiece {
        &self.piece
    }

    pub fn piece_mut(&mut self) -> &mut ShipwreckPiece {
        &mut self.piece
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipwreckStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    piece: ShipwreckPiece,
    cached_bounding_box: CachedStartBoundingBox,
}

impl ShipwreckStructureStart {
    pub fn from_generation_stub(stub: ShipwreckGenerationStub, references: i32) -> Self {
        Self {
            structure_id: stub.structure_id,
            start_chunk: stub.source_chunk,
            references,
            terrain_adjustment: StructureTerrainAdjustment::None,
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

    pub fn piece(&self) -> &ShipwreckPiece {
        &self.piece
    }

    pub fn piece_mut(&mut self) -> &mut ShipwreckPiece {
        self.cached_bounding_box.invalidate();
        &mut self.piece
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                let size = self.piece.template_size();
                let position = self.piece.template_position();
                let raw = if size == [0, 0, 0] {
                    JigsawBoundingBox::point(position)
                } else {
                    JigsawBoundingBox::from_array(persistence::piece_bounding_box(
                        position,
                        self.piece.rotation(),
                        size,
                    ))
                };
                Some(raw.adjusted_for_terrain(self.terrain_adjustment))
            })
            .expect("shipwreck start owns one piece")
    }
}

pub mod placement;
pub use placement::ShipwreckChunkPlacement;
