//! Typed generation, placement and persistence for clustered ocean-ruin starts.

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use bbb_protocol::packets::BlockPos;

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawRotation};
use super::structures::StructureTerrainAdjustment;

pub mod persistence;
pub mod start;

pub const OCEAN_RUIN_TYPE_ID: &str = "minecraft:ocean_ruin";
pub const OCEAN_RUIN_WARM_ID: &str = "minecraft:ocean_ruin_warm";
pub const OCEAN_RUIN_COLD_ID: &str = "minecraft:ocean_ruin_cold";
pub const OCEAN_RUIN_SET_ID: &str = "minecraft:ocean_ruins";
/// `StructurePieceType.java:60,86-96` lowercases the registered `ORP` id.
pub const OCEAN_RUIN_PIECE_ID: &str = "minecraft:orp";
pub const OCEAN_RUIN_BIG_LOOT: &str = "minecraft:chests/underwater_ruin_big";
pub const OCEAN_RUIN_SMALL_LOOT: &str = "minecraft:chests/underwater_ruin_small";
pub const OCEAN_RUIN_WARM_ARCHAEOLOGY: &str = "minecraft:archaeology/ocean_ruin_warm";
pub const OCEAN_RUIN_COLD_ARCHAEOLOGY: &str = "minecraft:archaeology/ocean_ruin_cold";

/// `OceanRuinStructure.java:45`.
pub const INITIAL_TEMPLATE_Y: i32 = 90;
/// Cluster parent extent used only for overlap (`OceanRuinPieces.java:177`).
pub const CLUSTER_PARENT_EXTENT: BlockPos = BlockPos { x: 15, y: 0, z: 15 };
/// Cluster child extent used only for overlap (`OceanRuinPieces.java:190`).
pub const CLUSTER_CHILD_EXTENT: BlockPos = BlockPos { x: 5, y: 0, z: 6 };

/// `OceanRuinPieces.java:62-71`.
pub const WARM_SMALL_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/warm_1",
    "minecraft:underwater_ruin/warm_2",
    "minecraft:underwater_ruin/warm_3",
    "minecraft:underwater_ruin/warm_4",
    "minecraft:underwater_ruin/warm_5",
    "minecraft:underwater_ruin/warm_6",
    "minecraft:underwater_ruin/warm_7",
    "minecraft:underwater_ruin/warm_8",
];

/// `OceanRuinPieces.java:120-125`.
pub const WARM_LARGE_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/big_warm_4",
    "minecraft:underwater_ruin/big_warm_5",
    "minecraft:underwater_ruin/big_warm_6",
    "minecraft:underwater_ruin/big_warm_7",
];

/// `OceanRuinPieces.java:72-81`.
pub const COLD_SMALL_BRICK_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/brick_1",
    "minecraft:underwater_ruin/brick_2",
    "minecraft:underwater_ruin/brick_3",
    "minecraft:underwater_ruin/brick_4",
    "minecraft:underwater_ruin/brick_5",
    "minecraft:underwater_ruin/brick_6",
    "minecraft:underwater_ruin/brick_7",
    "minecraft:underwater_ruin/brick_8",
];

/// `OceanRuinPieces.java:82-91`.
pub const COLD_SMALL_CRACKED_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/cracked_1",
    "minecraft:underwater_ruin/cracked_2",
    "minecraft:underwater_ruin/cracked_3",
    "minecraft:underwater_ruin/cracked_4",
    "minecraft:underwater_ruin/cracked_5",
    "minecraft:underwater_ruin/cracked_6",
    "minecraft:underwater_ruin/cracked_7",
    "minecraft:underwater_ruin/cracked_8",
];

/// `OceanRuinPieces.java:92-101`.
pub const COLD_SMALL_MOSSY_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/mossy_1",
    "minecraft:underwater_ruin/mossy_2",
    "minecraft:underwater_ruin/mossy_3",
    "minecraft:underwater_ruin/mossy_4",
    "minecraft:underwater_ruin/mossy_5",
    "minecraft:underwater_ruin/mossy_6",
    "minecraft:underwater_ruin/mossy_7",
    "minecraft:underwater_ruin/mossy_8",
];

/// `OceanRuinPieces.java:102-107`.
pub const COLD_LARGE_BRICK_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/big_brick_1",
    "minecraft:underwater_ruin/big_brick_2",
    "minecraft:underwater_ruin/big_brick_3",
    "minecraft:underwater_ruin/big_brick_8",
];

/// `OceanRuinPieces.java:114-119`.
pub const COLD_LARGE_CRACKED_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/big_cracked_1",
    "minecraft:underwater_ruin/big_cracked_2",
    "minecraft:underwater_ruin/big_cracked_3",
    "minecraft:underwater_ruin/big_cracked_8",
];

/// `OceanRuinPieces.java:108-113`.
pub const COLD_LARGE_MOSSY_TEMPLATES: &[&str] = &[
    "minecraft:underwater_ruin/big_mossy_1",
    "minecraft:underwater_ruin/big_mossy_2",
    "minecraft:underwater_ruin/big_mossy_3",
    "minecraft:underwater_ruin/big_mossy_8",
];

/// `OceanRuinStructure.Type` (`OceanRuinStructure.java:55-76`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OceanRuinBiomeTemp {
    Warm,
    Cold,
}

impl OceanRuinBiomeTemp {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::Cold => "cold",
        }
    }

    /// `ExtraCodecs.legacyEnum(Type::valueOf)` stores the enum name.
    pub const fn legacy_name(self) -> &'static str {
        match self {
            Self::Warm => "WARM",
            Self::Cold => "COLD",
        }
    }

    pub fn from_legacy_name(name: &str) -> Option<Self> {
        match name {
            "WARM" => Some(Self::Warm),
            "COLD" => Some(Self::Cold),
            _ => None,
        }
    }
}

/// Bit-exact f32 so settings and piece integrity stay `Eq`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OceanRuinFloat {
    bits: u32,
}

impl OceanRuinFloat {
    pub const fn from_f32(value: f32) -> Self {
        Self {
            bits: value.to_bits(),
        }
    }

    pub const fn get(self) -> f32 {
        f32::from_bits(self.bits)
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OceanRuinStructureSettings {
    pub biome_temp: OceanRuinBiomeTemp,
    pub large_probability: OceanRuinFloat,
    pub cluster_probability: OceanRuinFloat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanRuinPiece {
    template: String,
    template_position: BlockPos,
    rotation: JigsawRotation,
    biome_temp: OceanRuinBiomeTemp,
    integrity: OceanRuinFloat,
    is_large: bool,
    template_size: [i32; 3],
}

impl OceanRuinPiece {
    pub fn new(
        template: impl Into<String>,
        template_position: BlockPos,
        rotation: JigsawRotation,
        biome_temp: OceanRuinBiomeTemp,
        integrity: OceanRuinFloat,
        is_large: bool,
    ) -> Self {
        Self {
            template: template.into(),
            template_position,
            rotation,
            biome_temp,
            integrity,
            is_large,
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

    pub const fn biome_temp(&self) -> OceanRuinBiomeTemp {
        self.biome_temp
    }

    pub const fn integrity(&self) -> OceanRuinFloat {
        self.integrity
    }

    pub const fn is_large(&self) -> bool {
        self.is_large
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanRuinGenerationStub {
    source_chunk: [i32; 2],
    generation_anchor: BlockPos,
    structure_id: String,
    pieces: Vec<OceanRuinPiece>,
}

impl OceanRuinGenerationStub {
    pub fn new(
        source_chunk: [i32; 2],
        generation_anchor: BlockPos,
        structure_id: impl Into<String>,
        pieces: Vec<OceanRuinPiece>,
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

    pub fn pieces(&self) -> &[OceanRuinPiece] {
        &self.pieces
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanRuinStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    pieces: Vec<OceanRuinPiece>,
    cached_bounding_box: CachedStartBoundingBox,
}

impl OceanRuinStructureStart {
    pub fn from_generation_stub(stub: OceanRuinGenerationStub, references: i32) -> Self {
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
        pieces: Vec<OceanRuinPiece>,
    ) -> Self {
        let structure_id = structure_id.into();
        Self {
            structure_id,
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

    pub fn pieces(&self) -> &[OceanRuinPiece] {
        &self.pieces
    }

    pub fn pieces_mut(&mut self) -> &mut [OceanRuinPiece] {
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
                            size,
                        ))
                    }
                }),
                self.terrain_adjustment,
            )
        })
    }
}

/// `StructureTemplate.transform` with `Mirror.NONE` and pivot `ZERO`
/// (`StructureTemplate.java:528-555`).
pub fn transform_about_origin(pos: BlockPos, rotation: JigsawRotation) -> BlockPos {
    match rotation {
        JigsawRotation::None => pos,
        JigsawRotation::Clockwise90 => BlockPos {
            x: -pos.z,
            y: pos.y,
            z: pos.x,
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: -pos.x,
            y: pos.y,
            z: -pos.z,
        },
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: pos.z,
            y: pos.y,
            z: -pos.x,
        },
    }
}

pub mod placement;
pub use placement::OceanRuinChunkPlacement;
