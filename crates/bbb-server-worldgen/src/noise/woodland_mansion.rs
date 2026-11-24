//! Typed generation and persistence for the multi-piece woodland-mansion
//! StructureStart (`WoodlandMansionStructure.java`, `WoodlandMansionPieces.java`).

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use super::ruined_portal::RuinedPortalMirror;
use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawRotation};
use super::structures::StructureTerrainAdjustment;

pub mod direction;
pub mod grid;
pub mod persistence;
pub mod placer;
pub mod start;

pub const WOODLAND_MANSION_TYPE_ID: &str = "minecraft:woodland_mansion";
pub const WOODLAND_MANSION_ID: &str = "minecraft:mansion";
pub const WOODLAND_MANSION_SET_ID: &str = "minecraft:woodland_mansions";
/// `StructurePieceType.java:78` lowercases the registered `WMP` id.
pub const WOODLAND_MANSION_PIECE_ID: &str = "minecraft:wmp";
/// `BuiltInLootTables.WOODLAND_MANSION` (`BuiltInLootTables.java:44`).
pub const WOODLAND_MANSION_LOOT: &str = "minecraft:chests/woodland_mansion";

/// `WoodlandMansionPiece.makeLocation` (`WoodlandMansionPieces.java:1238-1239`).
pub const TEMPLATE_NAMESPACE: &str = "minecraft:woodland_mansion/";

pub fn qualify_template(short_name: &str) -> String {
    format!("{TEMPLATE_NAMESPACE}{short_name}")
}

pub fn short_template_name(template: &str) -> Option<&str> {
    template.strip_prefix(TEMPLATE_NAMESPACE)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WoodlandMansionPiece {
    template: String,
    template_position: BlockPos,
    rotation: JigsawRotation,
    mirror: RuinedPortalMirror,
    template_size: [i32; 3],
}

impl WoodlandMansionPiece {
    pub fn new(
        template: impl Into<String>,
        template_position: BlockPos,
        rotation: JigsawRotation,
        mirror: RuinedPortalMirror,
    ) -> Self {
        Self {
            template: template.into(),
            template_position,
            rotation,
            mirror,
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

    pub const fn mirror(&self) -> RuinedPortalMirror {
        self.mirror
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WoodlandMansionGenerationStub {
    source_chunk: [i32; 2],
    generation_anchor: BlockPos,
    structure_id: String,
    pieces: Vec<WoodlandMansionPiece>,
}

impl WoodlandMansionGenerationStub {
    pub fn new(
        source_chunk: [i32; 2],
        generation_anchor: BlockPos,
        structure_id: impl Into<String>,
        pieces: Vec<WoodlandMansionPiece>,
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

    pub fn pieces(&self) -> &[WoodlandMansionPiece] {
        &self.pieces
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WoodlandMansionStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    pieces: Vec<WoodlandMansionPiece>,
    cached_bounding_box: CachedStartBoundingBox,
}

impl WoodlandMansionStructureStart {
    pub fn from_generation_stub(stub: WoodlandMansionGenerationStub, references: i32) -> Self {
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
        pieces: Vec<WoodlandMansionPiece>,
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

    pub fn pieces(&self) -> &[WoodlandMansionPiece] {
        &self.pieces
    }

    pub fn pieces_mut(&mut self) -> &mut [WoodlandMansionPiece] {
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
                            position,
                            piece.rotation(),
                            piece.mirror(),
                            size,
                        ))
                    }
                }),
                self.terrain_adjustment,
            )
        })
    }
}

pub mod placement;
pub use placement::WoodlandMansionChunkPlacement;

/// `Util.shuffle` (`Util.java:1026-1032`).
pub fn shuffle<T>(values: &mut [T], random: &mut dyn RandomSource) {
    let size = values.len();
    for index in (1..size).rev() {
        let swap_to = random.next_int((index + 1) as i32) as usize;
        values.swap(index, swap_to);
    }
}
