//! Typed generation and persistence for leftover scattered-feature starts.

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox};
use super::structures::StructureTerrainAdjustment;

pub mod desert;
pub mod jungle;
pub mod persistence;
pub mod placement;
pub mod start;

pub use placement::ScatteredFeatureChunkPlacement;

pub const DESERT_PYRAMID_TYPE_ID: &str = "minecraft:desert_pyramid";
pub const DESERT_PYRAMID_ID: &str = "minecraft:desert_pyramid";
pub const DESERT_PYRAMID_SET_ID: &str = "minecraft:desert_pyramids";
/// `StructurePieceType.java:64,86-91` lowercases the registered `TeDP` id.
pub const DESERT_PYRAMID_PIECE_ID: &str = "minecraft:tedp";
pub const JUNGLE_TEMPLE_TYPE_ID: &str = "minecraft:jungle_temple";
pub const JUNGLE_TEMPLE_ID: &str = "minecraft:jungle_pyramid";
pub const JUNGLE_TEMPLE_SET_ID: &str = "minecraft:jungle_temples";
/// `StructurePieceType.java:59,86-91` lowercases the registered `TeJP` id.
pub const JUNGLE_TEMPLE_PIECE_ID: &str = "minecraft:tejp";
pub const SWAMP_HUT_TYPE_ID: &str = "minecraft:swamp_hut";
pub const SWAMP_HUT_ID: &str = "minecraft:swamp_hut";
pub const SWAMP_HUT_SET_ID: &str = "minecraft:swamp_huts";
/// `StructurePieceType.java:63,86-91` lowercases the registered `TeSH` id.
pub const SWAMP_HUT_PIECE_ID: &str = "minecraft:tesh";
/// `ScatteredFeaturePiece` ctor floor (`ScatteredFeaturePiece.java:17-32`).
pub const INITIAL_FLOOR_Y: i32 = 64;

/// 2D `O` values (`Direction.java:32-38`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalDirection {
    North,
    East,
    South,
    West,
}

impl HorizontalDirection {
    pub const fn from_2d(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::South),
            1 => Some(Self::West),
            2 => Some(Self::North),
            3 => Some(Self::East),
            _ => None,
        }
    }

    pub const fn to_2d(self) -> i32 {
        match self {
            Self::South => 0,
            Self::West => 1,
            Self::North => 2,
            Self::East => 3,
        }
    }

    /// `Direction.getAxis() == Axis.Z` for NORTH/SOUTH (`StructurePiece.java:75`).
    pub const fn is_z_axis(self) -> bool {
        matches!(self, Self::North | Self::South)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScatteredFeatureKind {
    DesertPyramid,
    JungleTemple,
    SwampHut,
}

impl ScatteredFeatureKind {
    /// Codec owners (`DesertPyramidStructure.java:25-30,74-77`,
    /// `JungleTempleStructure.java:8-18`).
    pub fn from_type_id(type_id: &str) -> Option<Self> {
        match type_id {
            DESERT_PYRAMID_TYPE_ID => Some(Self::DesertPyramid),
            JUNGLE_TEMPLE_TYPE_ID => Some(Self::JungleTemple),
            SWAMP_HUT_TYPE_ID => Some(Self::SwampHut),
            _ => None,
        }
    }

    pub fn from_set_id(set_id: &str) -> Option<Self> {
        match set_id {
            DESERT_PYRAMID_SET_ID => Some(Self::DesertPyramid),
            JUNGLE_TEMPLE_SET_ID => Some(Self::JungleTemple),
            SWAMP_HUT_SET_ID => Some(Self::SwampHut),
            _ => None,
        }
    }

    pub fn from_structure_id(structure_id: &str) -> Option<Self> {
        match structure_id {
            DESERT_PYRAMID_ID => Some(Self::DesertPyramid),
            JUNGLE_TEMPLE_ID => Some(Self::JungleTemple),
            SWAMP_HUT_ID => Some(Self::SwampHut),
            _ => None,
        }
    }

    pub const fn type_id(self) -> &'static str {
        match self {
            Self::DesertPyramid => DESERT_PYRAMID_TYPE_ID,
            Self::JungleTemple => JUNGLE_TEMPLE_TYPE_ID,
            Self::SwampHut => SWAMP_HUT_TYPE_ID,
        }
    }

    pub const fn structure_id(self) -> &'static str {
        match self {
            Self::DesertPyramid => DESERT_PYRAMID_ID,
            Self::JungleTemple => JUNGLE_TEMPLE_ID,
            Self::SwampHut => SWAMP_HUT_ID,
        }
    }

    pub const fn set_id(self) -> &'static str {
        match self {
            Self::DesertPyramid => DESERT_PYRAMID_SET_ID,
            Self::JungleTemple => JUNGLE_TEMPLE_SET_ID,
            Self::SwampHut => SWAMP_HUT_SET_ID,
        }
    }

    pub const fn leftover_features_label(self) -> Option<&'static str> {
        match self {
            Self::DesertPyramid => Some("desert-pyramid"),
            Self::JungleTemple => Some("jungle-temple"),
            Self::SwampHut => None,
        }
    }

    pub(crate) const fn owns_structure_id(self, _structure_id: &str) -> bool {
        true
    }

    pub const fn piece_id(self) -> &'static str {
        match self {
            Self::DesertPyramid => DESERT_PYRAMID_PIECE_ID,
            Self::JungleTemple => JUNGLE_TEMPLE_PIECE_ID,
            Self::SwampHut => SWAMP_HUT_PIECE_ID,
        }
    }

    /// Piece `width, height, depth` (`DesertPyramidPiece.java:30-32`,
    /// `JungleTemplePiece.java:39-40`, `SwampHutPiece.java:29-30`).
    pub const fn dimensions(self) -> (i32, i32, i32) {
        match self {
            Self::DesertPyramid => (21, 15, 21),
            Self::JungleTemple => (12, 10, 15),
            Self::SwampHut => (7, 7, 9),
        }
    }

    /// `SinglePieceStructure` `getLowestY` width/depth; swamp hut has none
    /// (`DesertPyramidStructure.java:28-29`, `JungleTempleStructure.java:11-12`,
    /// `SwampHutStructure.java:18-19`).
    pub const fn lowest_y_size(self) -> Option<(i32, i32)> {
        match self {
            Self::DesertPyramid => Some((21, 21)),
            Self::JungleTemple => Some((12, 15)),
            Self::SwampHut => None,
        }
    }
}

/// Extra piece flags use the Vanilla NBT field names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScatteredFeatureExtra {
    DesertPyramid {
        has_placed_chest0: bool,
        has_placed_chest1: bool,
        has_placed_chest2: bool,
        has_placed_chest3: bool,
    },
    JungleTemple {
        placed_main_chest: bool,
        placed_hidden_chest: bool,
        placed_trap1: bool,
        placed_trap2: bool,
    },
    SwampHut {
        witch: bool,
        cat: bool,
    },
}

impl ScatteredFeatureExtra {
    pub const fn default_for(kind: ScatteredFeatureKind) -> Self {
        match kind {
            ScatteredFeatureKind::DesertPyramid => Self::DesertPyramid {
                has_placed_chest0: false,
                has_placed_chest1: false,
                has_placed_chest2: false,
                has_placed_chest3: false,
            },
            ScatteredFeatureKind::JungleTemple => Self::JungleTemple {
                placed_main_chest: false,
                placed_hidden_chest: false,
                placed_trap1: false,
                placed_trap2: false,
            },
            ScatteredFeatureKind::SwampHut => Self::SwampHut {
                witch: false,
                cat: false,
            },
        }
    }

    pub const fn all_false(self) -> bool {
        match self {
            Self::DesertPyramid {
                has_placed_chest0,
                has_placed_chest1,
                has_placed_chest2,
                has_placed_chest3,
            } => {
                !has_placed_chest0 && !has_placed_chest1 && !has_placed_chest2 && !has_placed_chest3
            }
            Self::JungleTemple {
                placed_main_chest,
                placed_hidden_chest,
                placed_trap1,
                placed_trap2,
            } => !placed_main_chest && !placed_hidden_chest && !placed_trap1 && !placed_trap2,
            Self::SwampHut { witch, cat } => !witch && !cat,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScatteredFeaturePiece {
    kind: ScatteredFeatureKind,
    width: i32,
    height: i32,
    depth: i32,
    height_position: i32,
    orientation: HorizontalDirection,
    bounding_box: [i32; 6],
    extra: ScatteredFeatureExtra,
}

impl ScatteredFeaturePiece {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ScatteredFeatureKind,
        width: i32,
        height: i32,
        depth: i32,
        height_position: i32,
        orientation: HorizontalDirection,
        bounding_box: [i32; 6],
        extra: ScatteredFeatureExtra,
    ) -> Self {
        Self {
            kind,
            width,
            height,
            depth,
            height_position,
            orientation,
            bounding_box,
            extra,
        }
    }

    pub const fn kind(&self) -> ScatteredFeatureKind {
        self.kind
    }

    pub const fn width(&self) -> i32 {
        self.width
    }

    pub const fn height(&self) -> i32 {
        self.height
    }

    pub const fn depth(&self) -> i32 {
        self.depth
    }

    pub const fn height_position(&self) -> i32 {
        self.height_position
    }

    pub const fn orientation(&self) -> HorizontalDirection {
        self.orientation
    }

    pub const fn bounding_box(&self) -> [i32; 6] {
        self.bounding_box
    }

    pub const fn extra(&self) -> ScatteredFeatureExtra {
        self.extra
    }

    pub fn bounding_box_mut(&mut self) -> &mut [i32; 6] {
        &mut self.bounding_box
    }

    pub fn height_position_mut(&mut self) -> &mut i32 {
        &mut self.height_position
    }

    pub fn extra_mut(&mut self) -> &mut ScatteredFeatureExtra {
        &mut self.extra
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScatteredFeatureStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    piece: ScatteredFeaturePiece,
    cached_bounding_box: CachedStartBoundingBox,
}

impl ScatteredFeatureStructureStart {
    pub fn from_parts(
        structure_id: impl Into<String>,
        start_chunk: [i32; 2],
        references: i32,
        piece: ScatteredFeaturePiece,
    ) -> Self {
        Self {
            structure_id: structure_id.into(),
            start_chunk,
            references,
            terrain_adjustment: StructureTerrainAdjustment::None,
            piece,
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

    pub fn piece(&self) -> &ScatteredFeaturePiece {
        &self.piece
    }

    pub fn piece_mut(&mut self) -> &mut ScatteredFeaturePiece {
        self.cached_bounding_box.invalidate();
        &mut self.piece
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                Some(
                    JigsawBoundingBox::from_array(self.piece.bounding_box())
                        .adjusted_for_terrain(self.terrain_adjustment),
                )
            })
            .expect("scattered feature start owns one piece")
    }
}

/// `StructurePiece.makeBoundingBox` (`StructurePiece.java:72-78`).
pub fn make_bounding_box(
    x: i32,
    y: i32,
    z: i32,
    direction: HorizontalDirection,
    width: i32,
    height: i32,
    depth: i32,
) -> [i32; 6] {
    let (span_x, span_z) = if direction.is_z_axis() {
        (width, depth)
    } else {
        (depth, width)
    };
    [
        x,
        y,
        z,
        x.wrapping_add(span_x - 1),
        y.wrapping_add(height - 1),
        z.wrapping_add(span_z - 1),
    ]
}
