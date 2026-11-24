//! Typed generation and persistence for ocean-monument StructureStarts.

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use bbb_server_datapack::startup::DatapackStartup;
use std::collections::{BTreeMap, BTreeSet};

use super::scattered::HorizontalDirection;
use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox};
use super::structures::StructureTerrainAdjustment;

pub mod graph;
pub mod persistence;
pub mod placement;
pub mod start;

pub use placement::OceanMonumentChunkPlacement;

pub const OCEAN_MONUMENT_TYPE_ID: &str = "minecraft:ocean_monument";
pub const OCEAN_MONUMENT_ID: &str = "minecraft:monument";
pub const OCEAN_MONUMENT_SET_ID: &str = "minecraft:ocean_monuments";
/// `StructurePieceType.java:65,86-91` lowercases the registered `OMB` id.
pub const OCEAN_MONUMENT_PIECE_ID: &str = "minecraft:omb";
/// `StructurePieceType.java:66-76,86-91`.
pub const OCEAN_MONUMENT_ENTRY_PIECE_ID: &str = "minecraft:omentry";
pub const OCEAN_MONUMENT_CORE_PIECE_ID: &str = "minecraft:omcr";
pub const OCEAN_MONUMENT_DOUBLE_X_PIECE_ID: &str = "minecraft:omdxr";
pub const OCEAN_MONUMENT_DOUBLE_XY_PIECE_ID: &str = "minecraft:omdxyr";
pub const OCEAN_MONUMENT_DOUBLE_Y_PIECE_ID: &str = "minecraft:omdyr";
pub const OCEAN_MONUMENT_DOUBLE_YZ_PIECE_ID: &str = "minecraft:omdyzr";
pub const OCEAN_MONUMENT_DOUBLE_Z_PIECE_ID: &str = "minecraft:omdzr";
pub const OCEAN_MONUMENT_SIMPLE_PIECE_ID: &str = "minecraft:omsimple";
pub const OCEAN_MONUMENT_SIMPLE_TOP_PIECE_ID: &str = "minecraft:omsimplet";
pub const OCEAN_MONUMENT_WING_PIECE_ID: &str = "minecraft:omwr";
pub const OCEAN_MONUMENT_PENTHOUSE_PIECE_ID: &str = "minecraft:ompenthouse";

/// `OceanMonumentStructure.java:31-32,44-46`.
pub const SURROUND_RADIUS: i32 = 29;
pub const SURROUND_OFFSET: i32 = 9;
pub const BUILDING_WEST_NORTH_OFFSET: i32 = 29;
/// `OceanMonumentPieces.MonumentBuilding` (`OceanMonumentPieces.java:181-182`).
pub const BUILDING_MIN_Y: i32 = 39;
pub const BUILDING_WIDTH: i32 = 58;
pub const BUILDING_HEIGHT: i32 = 23;
pub const BUILDING_DEPTH: i32 = 58;

/// `BiomeTags.REQUIRED_OCEAN_MONUMENT_SURROUNDING`.
pub const REQUIRED_SURROUNDING_TAG: &str = "minecraft:required_ocean_monument_surrounding";

/// `Direction.Plane.HORIZONTAL` (`Direction.java:577`).
pub const HORIZONTAL: [HorizontalDirection; 4] = [
    HorizontalDirection::North,
    HorizontalDirection::East,
    HorizontalDirection::South,
    HorizontalDirection::West,
];

/// `Direction.get3DDataValue` (`Direction.java:33-38`).
pub const DIR_DOWN: usize = 0;
pub const DIR_UP: usize = 1;
pub const DIR_NORTH: usize = 2;
pub const DIR_SOUTH: usize = 3;
pub const DIR_WEST: usize = 4;
pub const DIR_EAST: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OceanMonumentRoomKind {
    Entry,
    Core,
    DoubleX,
    DoubleXY,
    DoubleY,
    DoubleYZ,
    DoubleZ,
    Simple,
    SimpleTop,
    Wing,
    Penthouse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanMonumentPiece {
    bounding_box: [i32; 6],
    orientation: HorizontalDirection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanMonumentRoomPiece {
    kind: OceanMonumentRoomKind,
    bounding_box: [i32; 6],
    orientation: HorizontalDirection,
    room_index: i32,
    has_opening: [bool; 6],
    connection_present: [bool; 6],
    main_design: i32,
}

impl OceanMonumentRoomPiece {
    pub const fn kind(&self) -> OceanMonumentRoomKind {
        self.kind
    }

    pub const fn bounding_box(&self) -> [i32; 6] {
        self.bounding_box
    }

    pub const fn orientation(&self) -> HorizontalDirection {
        self.orientation
    }

    pub const fn room_index(&self) -> i32 {
        self.room_index
    }

    pub const fn has_opening(&self, direction: usize) -> bool {
        self.has_opening[direction]
    }

    pub const fn connection_present(&self, direction: usize) -> bool {
        self.connection_present[direction]
    }

    pub const fn main_design(&self) -> i32 {
        self.main_design
    }

    pub fn opening_count(&self) -> i32 {
        self.has_opening.iter().filter(|open| **open).count() as i32
    }
}

/// Grid-index snapshot of `RoomDefinition` openings and connections after the
/// close-opening pass (`OceanMonumentPieces.java:205-218`). Double rooms walk
/// neighbor cells; `hasOpening` and `connections[face] == null` are distinct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanMonumentRoomCells {
    openings: BTreeMap<i32, [bool; 6]>,
    connections: BTreeMap<i32, [Option<i32>; 6]>,
}

impl OceanMonumentRoomCells {
    pub fn from_pairs(pairs: impl IntoIterator<Item = (i32, [bool; 6], [Option<i32>; 6])>) -> Self {
        let mut openings = BTreeMap::new();
        let mut connections = BTreeMap::new();
        for (index, opening, connection) in pairs {
            openings.insert(index, opening);
            connections.insert(index, connection);
        }
        Self {
            openings,
            connections,
        }
    }

    pub fn contains(&self, index: i32) -> bool {
        self.openings.contains_key(&index)
    }

    pub fn opening(&self, index: i32, face: usize) -> bool {
        self.openings
            .get(&index)
            .is_some_and(|openings| openings[face])
    }

    pub fn neighbor(&self, index: i32, face: usize) -> Option<i32> {
        self.connections
            .get(&index)
            .and_then(|connections| connections[face])
    }

    pub fn neighbor_present(&self, index: i32, face: usize) -> bool {
        self.neighbor(index, face).is_some()
    }

    pub fn walk(&self, mut index: i32, faces: &[usize]) -> Option<i32> {
        for &face in faces {
            index = self.neighbor(index, face)?;
        }
        Some(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegeneratedMonumentChildren {
    pub pieces: Vec<OceanMonumentRoomPiece>,
    pub cells: OceanMonumentRoomCells,
}

impl OceanMonumentPiece {
    pub const fn new(bounding_box: [i32; 6], orientation: HorizontalDirection) -> Self {
        Self {
            bounding_box,
            orientation,
        }
    }

    pub const fn bounding_box(&self) -> [i32; 6] {
        self.bounding_box
    }

    pub const fn orientation(&self) -> HorizontalDirection {
        self.orientation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanMonumentGenerationStub {
    source_chunk: [i32; 2],
    generation_anchor: bbb_protocol::packets::BlockPos,
    structure_id: String,
    piece: OceanMonumentPiece,
}

impl OceanMonumentGenerationStub {
    pub fn new(
        source_chunk: [i32; 2],
        generation_anchor: bbb_protocol::packets::BlockPos,
        structure_id: impl Into<String>,
        piece: OceanMonumentPiece,
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

    pub const fn generation_anchor(&self) -> bbb_protocol::packets::BlockPos {
        self.generation_anchor
    }

    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub fn piece(&self) -> &OceanMonumentPiece {
        &self.piece
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OceanMonumentStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    piece: OceanMonumentPiece,
    cached_bounding_box: CachedStartBoundingBox,
}

impl OceanMonumentStructureStart {
    pub fn from_generation_stub(stub: OceanMonumentGenerationStub, references: i32) -> Self {
        Self::from_parts(stub.structure_id, stub.source_chunk, references, stub.piece)
    }

    pub fn from_parts(
        structure_id: impl Into<String>,
        start_chunk: [i32; 2],
        references: i32,
        piece: OceanMonumentPiece,
    ) -> Self {
        let structure_id = structure_id.into();
        Self {
            structure_id,
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

    pub fn piece(&self) -> &OceanMonumentPiece {
        &self.piece
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                Some(
                    JigsawBoundingBox::from_array(self.piece.bounding_box())
                        .adjusted_for_terrain(self.terrain_adjustment),
                )
            })
            .expect("ocean monument start owns one piece")
    }
}

/// Expands `#minecraft:required_ocean_monument_surrounding` (`BiomeTags.java`).
pub fn required_surrounding_biomes(startup: &DatapackStartup) -> Option<BTreeSet<String>> {
    startup
        .tags
        .registry("minecraft:worldgen/biome")
        .and_then(|tags| tags.tag(REQUIRED_SURROUNDING_TAG))
        .map(|members| members.iter().cloned().collect())
}

/// `QuartPos.fromBlock` (`QuartPos.java:12-14`).
pub const fn quart_from_block(coord: i32) -> i32 {
    coord >> 2
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::structures::NoiseStructurePlan;
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
        let source = NoiseBiomeSource::from_registry(
            &startup.registries,
            "minecraft:overworld",
            "minecraft:overworld",
            0,
        )
        .unwrap();
        NoiseStructurePlan::from_startup(startup, source.overworld_biome_order().unwrap()).unwrap()
    }

    #[test]
    fn surrounding_tag_expands_ocean_and_river() {
        let startup = startup();
        let surrounding = required_surrounding_biomes(&startup).unwrap();
        for biome in [
            "minecraft:ocean",
            "minecraft:deep_ocean",
            "minecraft:frozen_ocean",
            "minecraft:river",
            "minecraft:frozen_river",
        ] {
            assert!(surrounding.contains(biome), "{biome}");
        }
        assert!(!surrounding.contains("minecraft:plains"));
        assert!(!surrounding.contains("minecraft:deep_dark"));
    }

    #[test]
    fn biomes_within_visits_the_vanilla_quart_box() {
        let mut samples = Vec::new();
        let biomes = start::biomes_within(9, 63, 9, SURROUND_RADIUS, |position| {
            samples.push((
                position.x.div_euclid(4),
                position.y.div_euclid(4),
                position.z.div_euclid(4),
            ));
            Some("minecraft:ocean".to_string())
        })
        .unwrap();
        assert_eq!(biomes.len(), 1);
        assert_eq!(samples.len(), 15 * 15 * 16);
        assert_eq!(samples[0], (-5, 8, -5));
        assert_eq!(*samples.last().unwrap(), (9, 23, 9));
    }
}
