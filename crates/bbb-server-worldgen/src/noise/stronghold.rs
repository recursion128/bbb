//! Typed generation and persistence for the multi-piece stronghold
//! StructureStart (`StrongholdStructure.java`, `StrongholdPieces.java`).

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use crate::noise::scattered::HorizontalDirection;
use crate::noise::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox};
use crate::noise::structures::{
    NoiseStructureDefinition, NoiseStructurePlacement, NoiseStructureSet,
    StructureTerrainAdjustment,
};

pub mod geometry;
pub mod persistence;
pub mod pieces;
pub mod placement;
pub mod start;

pub use placement::StrongholdChunkPlacement;

pub const STRONGHOLD_TYPE_ID: &str = "minecraft:stronghold";
pub const STRONGHOLD_ID: &str = "minecraft:stronghold";
pub const STRONGHOLD_SET_ID: &str = "minecraft:strongholds";

pub fn owns_set<'a>(
    set: &NoiseStructureSet,
    definitions: impl IntoIterator<Item = Option<&'a NoiseStructureDefinition>>,
) -> bool {
    matches!(
        set.placement,
        NoiseStructurePlacement::ConcentricRings { .. }
    ) && !set.structures.is_empty()
        && definitions.into_iter().all(|definition| {
            definition.is_some_and(|definition| definition.type_id == STRONGHOLD_TYPE_ID)
        })
}

/// `StructurePieceType.java:46-58,86-91` lowercases each registered id.
pub const PIECE_ID_CHEST_CORRIDOR: &str = "minecraft:shcc";
pub const PIECE_ID_FILLER_CORRIDOR: &str = "minecraft:shfc";
pub const PIECE_ID_FIVE_CROSSING: &str = "minecraft:sh5c";
pub const PIECE_ID_LEFT_TURN: &str = "minecraft:shlt";
pub const PIECE_ID_LIBRARY: &str = "minecraft:shli";
pub const PIECE_ID_PORTAL_ROOM: &str = "minecraft:shpr";
pub const PIECE_ID_PRISON_HALL: &str = "minecraft:shph";
pub const PIECE_ID_RIGHT_TURN: &str = "minecraft:shrt";
pub const PIECE_ID_ROOM_CROSSING: &str = "minecraft:shrc";
pub const PIECE_ID_STAIRS_DOWN: &str = "minecraft:shsd";
pub const PIECE_ID_START: &str = "minecraft:shstart";
pub const PIECE_ID_STRAIGHT: &str = "minecraft:shs";
pub const PIECE_ID_STRAIGHT_STAIRS_DOWN: &str = "minecraft:shssd";

/// `StrongholdPiece.SmallDoorType` (`StrongholdPieces.java:1739-1743`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmallDoorType {
    Opening,
    WoodDoor,
    Grates,
    IronDoor,
}

impl SmallDoorType {
    pub const fn legacy_name(self) -> &'static str {
        match self {
            Self::Opening => "OPENING",
            Self::WoodDoor => "WOOD_DOOR",
            Self::Grates => "GRATES",
            Self::IronDoor => "IRON_DOOR",
        }
    }

    pub fn from_legacy_name(name: &str) -> Option<Self> {
        match name {
            "OPENING" => Some(Self::Opening),
            "WOOD_DOOR" => Some(Self::WoodDoor),
            "GRATES" => Some(Self::Grates),
            "IRON_DOOR" => Some(Self::IronDoor),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrongholdPieceKind {
    ChestCorridor,
    FillerCorridor,
    FiveCrossing,
    LeftTurn,
    Library,
    PortalRoom,
    PrisonHall,
    RightTurn,
    RoomCrossing,
    StairsDown,
    Start,
    Straight,
    StraightStairsDown,
}

impl StrongholdPieceKind {
    pub const fn piece_id(self) -> &'static str {
        match self {
            Self::ChestCorridor => PIECE_ID_CHEST_CORRIDOR,
            Self::FillerCorridor => PIECE_ID_FILLER_CORRIDOR,
            Self::FiveCrossing => PIECE_ID_FIVE_CROSSING,
            Self::LeftTurn => PIECE_ID_LEFT_TURN,
            Self::Library => PIECE_ID_LIBRARY,
            Self::PortalRoom => PIECE_ID_PORTAL_ROOM,
            Self::PrisonHall => PIECE_ID_PRISON_HALL,
            Self::RightTurn => PIECE_ID_RIGHT_TURN,
            Self::RoomCrossing => PIECE_ID_ROOM_CROSSING,
            Self::StairsDown => PIECE_ID_STAIRS_DOWN,
            Self::Start => PIECE_ID_START,
            Self::Straight => PIECE_ID_STRAIGHT,
            Self::StraightStairsDown => PIECE_ID_STRAIGHT_STAIRS_DOWN,
        }
    }

    pub fn from_piece_id(id: &str) -> Option<Self> {
        match id {
            PIECE_ID_CHEST_CORRIDOR => Some(Self::ChestCorridor),
            PIECE_ID_FILLER_CORRIDOR => Some(Self::FillerCorridor),
            PIECE_ID_FIVE_CROSSING => Some(Self::FiveCrossing),
            PIECE_ID_LEFT_TURN => Some(Self::LeftTurn),
            PIECE_ID_LIBRARY => Some(Self::Library),
            PIECE_ID_PORTAL_ROOM => Some(Self::PortalRoom),
            PIECE_ID_PRISON_HALL => Some(Self::PrisonHall),
            PIECE_ID_RIGHT_TURN => Some(Self::RightTurn),
            PIECE_ID_ROOM_CROSSING => Some(Self::RoomCrossing),
            PIECE_ID_STAIRS_DOWN => Some(Self::StairsDown),
            PIECE_ID_START => Some(Self::Start),
            PIECE_ID_STRAIGHT => Some(Self::Straight),
            PIECE_ID_STRAIGHT_STAIRS_DOWN => Some(Self::StraightStairsDown),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrongholdPieceExtra {
    None,
    ChestCorridor {
        chest: bool,
    },
    FillerCorridor {
        steps: i32,
    },
    FiveCrossing {
        left_low: bool,
        left_high: bool,
        right_low: bool,
        right_high: bool,
    },
    Library {
        tall: bool,
    },
    PortalRoom {
        mob: bool,
    },
    RoomCrossing {
        room_type: i32,
    },
    StairsDown {
        source: bool,
    },
    Straight {
        left: bool,
        right: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrongholdPiece {
    kind: StrongholdPieceKind,
    bounding_box: [i32; 6],
    orientation: HorizontalDirection,
    gen_depth: i32,
    entry_door: SmallDoorType,
    extra: StrongholdPieceExtra,
}

impl StrongholdPiece {
    pub fn new(
        kind: StrongholdPieceKind,
        bounding_box: [i32; 6],
        orientation: HorizontalDirection,
        gen_depth: i32,
        entry_door: SmallDoorType,
        extra: StrongholdPieceExtra,
    ) -> Self {
        Self {
            kind,
            bounding_box,
            orientation,
            gen_depth,
            entry_door,
            extra,
        }
    }

    pub const fn kind(&self) -> StrongholdPieceKind {
        self.kind
    }

    pub const fn piece_id(&self) -> &'static str {
        self.kind.piece_id()
    }

    pub const fn bounding_box(&self) -> [i32; 6] {
        self.bounding_box
    }

    pub const fn orientation(&self) -> HorizontalDirection {
        self.orientation
    }

    pub const fn gen_depth(&self) -> i32 {
        self.gen_depth
    }

    pub const fn entry_door(&self) -> SmallDoorType {
        self.entry_door
    }

    pub const fn extra(&self) -> &StrongholdPieceExtra {
        &self.extra
    }

    pub fn extra_mut(&mut self) -> &mut StrongholdPieceExtra {
        &mut self.extra
    }

    pub fn offset_y(&mut self, dy: i32) {
        self.bounding_box[1] = self.bounding_box[1].wrapping_add(dy);
        self.bounding_box[4] = self.bounding_box[4].wrapping_add(dy);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrongholdStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    pieces: Vec<StrongholdPiece>,
    cached_bounding_box: CachedStartBoundingBox,
}

impl StrongholdStructureStart {
    pub fn from_generation(
        structure_id: impl Into<String>,
        start_chunk: [i32; 2],
        references: i32,
        pieces: Vec<StrongholdPiece>,
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

    pub fn pieces(&self) -> &[StrongholdPiece] {
        &self.pieces
    }

    pub fn pieces_mut(&mut self) -> &mut [StrongholdPiece] {
        self.cached_bounding_box.invalidate();
        &mut self.pieces
    }

    pub fn bounding_box(&self) -> Option<JigsawBoundingBox> {
        self.cached_bounding_box.get_or_init(|| {
            JigsawBoundingBox::adjusted_union(
                self.pieces
                    .iter()
                    .map(|piece| JigsawBoundingBox::from_array(piece.bounding_box())),
                self.terrain_adjustment,
            )
        })
    }
}
