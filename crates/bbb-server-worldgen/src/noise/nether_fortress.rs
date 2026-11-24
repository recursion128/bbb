//! Built-in Nether fortress graph and generated-piece placement.

use crate::random::large_feature_random;
use std::collections::VecDeque;
use std::sync::OnceLock;

use crate::random::WorldgenRandom;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_server_datapack::mob_spawn::{MobSpawnList, MobSpawnerData, WeightedMobSpawnerData};

use super::scattered::HorizontalDirection;
use super::stronghold::geometry::{intersects, make_bounding_box, orient_box, union_boxes};
use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox};
use super::structures::{NoiseStructurePlan, StructureTerrainAdjustment};

pub const FORTRESS_ID: &str = "minecraft:fortress";
pub const FORTRESS_TYPE_ID: &str = "minecraft:fortress";
pub const NETHER_COMPLEXES_SET_ID: &str = "minecraft:nether_complexes";

// `NetherFortressStructure.java:16-22`.
pub fn fortress_natural_enemy_spawns() -> &'static MobSpawnList {
    static SPAWNS: OnceLock<MobSpawnList> = OnceLock::new();
    SPAWNS.get_or_init(|| MobSpawnList {
        entries: vec![
            fortress_spawn("minecraft:blaze", VANILLA_ENTITY_TYPE_BLAZE_ID, 2, 3, 10),
            fortress_spawn(
                "minecraft:zombified_piglin",
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
                4,
                4,
                5,
            ),
            fortress_spawn(
                "minecraft:wither_skeleton",
                VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
                5,
                5,
                8,
            ),
            fortress_spawn(
                "minecraft:skeleton",
                VANILLA_ENTITY_TYPE_SKELETON_ID,
                5,
                5,
                2,
            ),
            fortress_spawn(
                "minecraft:magma_cube",
                VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
                4,
                4,
                3,
            ),
        ],
        total_weight: 28,
    })
}

fn fortress_spawn(
    entity_type: &str,
    entity_type_id: i32,
    min_count: i32,
    max_count: i32,
    weight: i32,
) -> WeightedMobSpawnerData {
    WeightedMobSpawnerData {
        data: MobSpawnerData {
            entity_type_id,
            entity_type: entity_type.to_string(),
            min_count,
            max_count,
        },
        weight,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FortressPieceKind {
    Start,
    BridgeCrossing,
    BridgeStraight,
    RoomCrossing,
    StairsRoom,
    MonsterThrone,
    CastleEntrance,
    SmallCorridor,
    SmallCrossing,
    SmallLeftTurn,
    SmallRightTurn,
    CorridorStairs,
    TBalcony,
    StalkRoom,
    EndFiller,
}

impl FortressPieceKind {
    pub const fn piece_id(self) -> &'static str {
        match self {
            Self::Start => "minecraft:nestart",
            Self::BridgeCrossing => "minecraft:nebcr",
            Self::BridgeStraight => "minecraft:nebs",
            Self::RoomCrossing => "minecraft:nerc",
            Self::StairsRoom => "minecraft:nesr",
            Self::MonsterThrone => "minecraft:nemt",
            Self::CastleEntrance => "minecraft:nece",
            Self::SmallCorridor => "minecraft:nesc",
            Self::SmallCrossing => "minecraft:nescsc",
            Self::SmallLeftTurn => "minecraft:nesclt",
            Self::SmallRightTurn => "minecraft:nescrt",
            Self::CorridorStairs => "minecraft:neccs",
            Self::TBalcony => "minecraft:nectb",
            Self::StalkRoom => "minecraft:necsr",
            Self::EndFiller => "minecraft:nebef",
        }
    }

    fn geometry(self) -> (i32, i32, i32, i32, i32, i32) {
        match self {
            Self::Start | Self::BridgeCrossing => (-8, -3, 0, 19, 10, 19),
            Self::BridgeStraight => (-1, -3, 0, 5, 10, 19),
            Self::RoomCrossing => (-2, 0, 0, 7, 9, 7),
            Self::StairsRoom => (-2, 0, 0, 7, 11, 7),
            Self::MonsterThrone => (-2, 0, 0, 7, 8, 9),
            Self::CastleEntrance | Self::StalkRoom => (-5, -3, 0, 13, 14, 13),
            Self::SmallCorridor
            | Self::SmallCrossing
            | Self::SmallLeftTurn
            | Self::SmallRightTurn => (-1, 0, 0, 5, 7, 5),
            Self::CorridorStairs => (-1, -7, 0, 5, 14, 10),
            Self::TBalcony => (-3, 0, 0, 9, 7, 9),
            Self::EndFiller => (-1, -3, 0, 5, 10, 8),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FortressPiece {
    pub kind: FortressPieceKind,
    pub generation_depth: i32,
    pub bounding_box: JigsawBoundingBox,
    pub orientation: HorizontalDirection,
    pub random_seed: i32,
    pub chest_pending: bool,
    pub spawner_placed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FortressStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    generation_anchor: BlockPos,
    pieces: Vec<FortressPiece>,
    cached_bounding_box: CachedStartBoundingBox,
}

impl FortressStructureStart {
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

    pub const fn terrain_adjustment(&self) -> StructureTerrainAdjustment {
        self.terrain_adjustment
    }

    pub const fn generation_anchor(&self) -> BlockPos {
        self.generation_anchor
    }

    pub fn pieces(&self) -> &[FortressPiece] {
        &self.pieces
    }

    pub fn from_parts(
        start_chunk: [i32; 2],
        references: i32,
        terrain_adjustment: StructureTerrainAdjustment,
        pieces: Vec<FortressPiece>,
    ) -> Self {
        Self::from_parts_for(
            FORTRESS_ID,
            start_chunk,
            references,
            terrain_adjustment,
            pieces,
        )
    }

    pub(crate) fn from_parts_for(
        structure_id: &str,
        start_chunk: [i32; 2],
        references: i32,
        terrain_adjustment: StructureTerrainAdjustment,
        pieces: Vec<FortressPiece>,
    ) -> Self {
        Self {
            structure_id: structure_id.to_string(),
            start_chunk,
            references,
            terrain_adjustment,
            generation_anchor: BlockPos {
                x: start_chunk[0] * 16,
                y: 64,
                z: start_chunk[1] * 16,
            },
            pieces,
            cached_bounding_box: CachedStartBoundingBox::new(),
        }
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                JigsawBoundingBox::adjusted_union(
                    self.pieces.iter().map(|piece| piece.bounding_box),
                    self.terrain_adjustment,
                )
            })
            .unwrap_or_else(|| JigsawBoundingBox::from_array([0; 6]))
    }

    pub fn intersects_chunk(&self, x: i32, z: i32) -> bool {
        !self.pieces.is_empty() && self.bounding_box().intersects_chunk(x, z)
    }
}

impl NoiseStructurePlan {
    /// `NetherFortressStructure.findGenerationPoint` and `StartPiece` graph
    /// (`NetherFortressStructure.java:31-52`, `NetherFortressPieces.java:1142-1242`).
    pub fn fortress_start(
        &self,
        seed: i64,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Option<FortressStructureStart> {
        self.fortress_start_for(FORTRESS_ID, seed, chunk_x, chunk_z)
    }

    /// Also serves `PlaceCommand.placeStructure` (`PlaceCommand.java:277-296`),
    /// whose biome predicate accepts every biome.
    pub fn fortress_start_for(
        &self,
        structure_id: &str,
        seed: i64,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Option<FortressStructureStart> {
        let definition = self.structures().get(structure_id)?;
        let mut random = large_feature_random(seed, chunk_x, chunk_z);
        let direction = random_direction(&mut random);
        let west = chunk_x * 16 + 2;
        let north = chunk_z * 16 + 2;
        let root_box = box_from_array(make_bounding_box(west, 64, north, direction, 19, 10, 19));
        let mut pieces = vec![piece(
            FortressPieceKind::Start,
            0,
            root_box,
            direction,
            &mut random,
        )];
        let mut pending = VecDeque::from([0_usize]);
        let mut weights = Weights::default();
        while let Some(index) = pop_random(&mut pending, &mut random) {
            add_children(index, &mut pieces, &mut pending, &mut weights, &mut random);
        }
        move_inside_heights(&mut pieces, 48, 70, &mut random);
        Some(FortressStructureStart::from_parts_for(
            structure_id,
            [chunk_x, chunk_z],
            0,
            definition.terrain_adjustment,
            pieces,
        ))
    }
}

#[derive(Default)]
struct Weights {
    bridge: [u8; 6],
    castle: [u8; 7],
    previous: Option<FortressPieceKind>,
}

const BRIDGE: [(FortressPieceKind, i32, u8, bool); 6] = [
    (FortressPieceKind::BridgeStraight, 30, 0, true),
    (FortressPieceKind::BridgeCrossing, 10, 4, false),
    (FortressPieceKind::RoomCrossing, 10, 4, false),
    (FortressPieceKind::StairsRoom, 10, 3, false),
    (FortressPieceKind::MonsterThrone, 5, 2, false),
    (FortressPieceKind::CastleEntrance, 5, 1, false),
];
const CASTLE: [(FortressPieceKind, i32, u8, bool); 7] = [
    (FortressPieceKind::SmallCorridor, 25, 0, true),
    (FortressPieceKind::SmallCrossing, 15, 5, false),
    (FortressPieceKind::SmallRightTurn, 5, 10, false),
    (FortressPieceKind::SmallLeftTurn, 5, 10, false),
    (FortressPieceKind::CorridorStairs, 10, 3, true),
    (FortressPieceKind::TBalcony, 7, 2, false),
    (FortressPieceKind::StalkRoom, 5, 2, false),
];

fn add_children(
    index: usize,
    pieces: &mut Vec<FortressPiece>,
    pending: &mut VecDeque<usize>,
    weights: &mut Weights,
    random: &mut WorldgenRandom,
) {
    let parent = pieces[index].clone();
    let mut links = Vec::new();
    match parent.kind {
        FortressPieceKind::Start | FortressPieceKind::BridgeCrossing => {
            links.push(forward(&parent, 8, 3, false));
            links.push(left(&parent, 3, 8, false));
            links.push(right(&parent, 3, 8, false));
        }
        FortressPieceKind::BridgeStraight => links.push(forward(&parent, 1, 3, false)),
        FortressPieceKind::RoomCrossing => {
            links.push(forward(&parent, 2, 0, false));
            links.push(left(&parent, 0, 2, false));
            links.push(right(&parent, 0, 2, false));
        }
        FortressPieceKind::StairsRoom => links.push(right(&parent, 6, 2, false)),
        FortressPieceKind::CastleEntrance => links.push(forward(&parent, 5, 3, true)),
        FortressPieceKind::SmallCorridor => links.push(forward(&parent, 1, 0, true)),
        FortressPieceKind::SmallCrossing => {
            links.push(forward(&parent, 1, 0, true));
            links.push(left(&parent, 0, 1, true));
            links.push(right(&parent, 0, 1, true));
        }
        FortressPieceKind::SmallLeftTurn => links.push(left(&parent, 0, 1, true)),
        FortressPieceKind::SmallRightTurn => links.push(right(&parent, 0, 1, true)),
        FortressPieceKind::CorridorStairs => links.push(forward(&parent, 1, 0, true)),
        FortressPieceKind::TBalcony => {
            links.push(left(&parent, 0, 3, random.next_int(8) > 0));
            links.push(right(&parent, 0, 3, random.next_int(8) > 0));
        }
        FortressPieceKind::StalkRoom => {
            links.push(forward(&parent, 5, 3, true));
            links.push(forward(&parent, 5, 11, true));
        }
        FortressPieceKind::MonsterThrone | FortressPieceKind::EndFiller => {}
    }
    let root = pieces[0].bounding_box;
    for (x, y, z, direction, castle) in links {
        if (i64::from(x) - i64::from(root.min.x)).abs() > 112
            || (i64::from(z) - i64::from(root.min.z)).abs() > 112
        {
            continue;
        }
        let depth = parent.generation_depth + 1;
        let Some(next) = generate_piece(castle, depth, x, y, z, direction, pieces, weights, random)
        else {
            continue;
        };
        let next_index = pieces.len();
        pieces.push(next);
        pending.push_back(next_index);
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_piece(
    castle: bool,
    depth: i32,
    x: i32,
    y: i32,
    z: i32,
    direction: HorizontalDirection,
    existing: &[FortressPiece],
    weights: &mut Weights,
    random: &mut WorldgenRandom,
) -> Option<FortressPiece> {
    let table = if castle { &CASTLE[..] } else { &BRIDGE[..] };
    let has_available = table.iter().enumerate().any(|(index, (_, _, maximum, _))| {
        *maximum > 0
            && (if castle {
                weights.castle[index]
            } else {
                weights.bridge[index]
            }) < *maximum
    });
    let total = if has_available {
        table.iter().map(|entry| entry.1).sum::<i32>()
    } else {
        -1
    };
    if total > 0 && depth <= 30 {
        for _ in 0..5 {
            let mut draw = random.next_int(total);
            for (index, &(kind, weight, maximum, allow_in_row)) in table.iter().enumerate() {
                draw -= weight;
                if draw >= 0 {
                    continue;
                }
                let count = if castle {
                    weights.castle[index]
                } else {
                    weights.bridge[index]
                };
                if (maximum > 0 && count >= maximum)
                    || (!allow_in_row && weights.previous == Some(kind))
                {
                    break;
                }
                if let Some(next) = create_piece(kind, depth, x, y, z, direction, existing, random)
                {
                    if castle {
                        weights.castle[index] = weights.castle[index].saturating_add(1);
                    } else {
                        weights.bridge[index] = weights.bridge[index].saturating_add(1);
                    }
                    weights.previous = Some(kind);
                    return Some(next);
                }
                break;
            }
        }
    }
    create_piece(
        FortressPieceKind::EndFiller,
        depth,
        x,
        y,
        z,
        direction,
        existing,
        random,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_piece(
    kind: FortressPieceKind,
    depth: i32,
    x: i32,
    y: i32,
    z: i32,
    direction: HorizontalDirection,
    existing: &[FortressPiece],
    random: &mut WorldgenRandom,
) -> Option<FortressPiece> {
    let (offset_x, offset_y, offset_z, width, height, piece_depth) = kind.geometry();
    let bounding_box = box_from_array(orient_box(
        x,
        y,
        z,
        offset_x,
        offset_y,
        offset_z,
        width,
        height,
        piece_depth,
        direction,
    ));
    if bounding_box.min.y <= 10
        || existing
            .iter()
            .any(|piece| intersects(box_array(piece.bounding_box), box_array(bounding_box)))
    {
        None
    } else {
        Some(piece(kind, depth, bounding_box, direction, random))
    }
}

fn piece(
    kind: FortressPieceKind,
    generation_depth: i32,
    bounding_box: JigsawBoundingBox,
    orientation: HorizontalDirection,
    random: &mut WorldgenRandom,
) -> FortressPiece {
    FortressPiece {
        kind,
        generation_depth,
        bounding_box,
        orientation,
        random_seed: if kind == FortressPieceKind::EndFiller {
            random.next_bits(32) as i32
        } else {
            0
        },
        chest_pending: matches!(
            kind,
            FortressPieceKind::SmallLeftTurn | FortressPieceKind::SmallRightTurn
        ) && random.next_int(3) == 0,
        spawner_placed: false,
    }
}

type Link = (i32, i32, i32, HorizontalDirection, bool);

fn forward(piece: &FortressPiece, x_offset: i32, y_offset: i32, castle: bool) -> Link {
    let bounds = piece.bounding_box;
    match piece.orientation {
        HorizontalDirection::North => (
            bounds.min.x + x_offset,
            bounds.min.y + y_offset,
            bounds.min.z - 1,
            piece.orientation,
            castle,
        ),
        HorizontalDirection::South => (
            bounds.min.x + x_offset,
            bounds.min.y + y_offset,
            bounds.max.z + 1,
            piece.orientation,
            castle,
        ),
        HorizontalDirection::West => (
            bounds.min.x - 1,
            bounds.min.y + y_offset,
            bounds.min.z + x_offset,
            piece.orientation,
            castle,
        ),
        HorizontalDirection::East => (
            bounds.max.x + 1,
            bounds.min.y + y_offset,
            bounds.min.z + x_offset,
            piece.orientation,
            castle,
        ),
    }
}

fn left(piece: &FortressPiece, y_offset: i32, z_offset: i32, castle: bool) -> Link {
    let bounds = piece.bounding_box;
    match piece.orientation {
        HorizontalDirection::North | HorizontalDirection::South => (
            bounds.min.x - 1,
            bounds.min.y + y_offset,
            bounds.min.z + z_offset,
            HorizontalDirection::West,
            castle,
        ),
        _ => (
            bounds.min.x + z_offset,
            bounds.min.y + y_offset,
            bounds.min.z - 1,
            HorizontalDirection::North,
            castle,
        ),
    }
}

fn right(piece: &FortressPiece, y_offset: i32, z_offset: i32, castle: bool) -> Link {
    let bounds = piece.bounding_box;
    match piece.orientation {
        HorizontalDirection::North | HorizontalDirection::South => (
            bounds.max.x + 1,
            bounds.min.y + y_offset,
            bounds.min.z + z_offset,
            HorizontalDirection::East,
            castle,
        ),
        _ => (
            bounds.min.x + z_offset,
            bounds.min.y + y_offset,
            bounds.max.z + 1,
            HorizontalDirection::South,
            castle,
        ),
    }
}

fn move_inside_heights(
    pieces: &mut [FortressPiece],
    min_y: i32,
    max_y: i32,
    random: &mut WorldgenRandom,
) {
    let boxes = pieces
        .iter()
        .map(|piece| box_array(piece.bounding_box))
        .collect::<Vec<_>>();
    let bounds = union_boxes(&boxes).unwrap_or_default();
    let span = bounds[4].wrapping_sub(bounds[1]).wrapping_add(1);
    let range = max_y.wrapping_sub(min_y).wrapping_add(1).wrapping_sub(span);
    let target_y = if range > 1 {
        min_y.wrapping_add(random.next_int(range))
    } else {
        min_y
    };
    let delta_y = target_y.wrapping_sub(bounds[1]);
    for piece in pieces {
        piece.bounding_box.min.y = piece.bounding_box.min.y.wrapping_add(delta_y);
        piece.bounding_box.max.y = piece.bounding_box.max.y.wrapping_add(delta_y);
    }
}

fn pop_random(queue: &mut VecDeque<usize>, random: &mut WorldgenRandom) -> Option<usize> {
    if queue.is_empty() {
        None
    } else {
        queue.remove(random.next_int(queue.len() as i32) as usize)
    }
}

fn random_direction(random: &mut WorldgenRandom) -> HorizontalDirection {
    [
        HorizontalDirection::North,
        HorizontalDirection::East,
        HorizontalDirection::South,
        HorizontalDirection::West,
    ][random.next_int(4) as usize]
}

fn box_array(bounds: JigsawBoundingBox) -> [i32; 6] {
    [
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
    ]
}

fn box_from_array(bounds: [i32; 6]) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: bounds[0],
            y: bounds[1],
            z: bounds[2],
        },
        max: BlockPos {
            x: bounds[3],
            y: bounds[4],
            z: bounds[5],
        },
    }
}

pub mod persistence;
pub mod placement;

#[cfg(test)]
mod spawn_override_tests {
    use super::fortress_natural_enemy_spawns;

    #[test]
    fn fortress_spawn_override_catalog_preserves_vanilla_order_and_weights() {
        let spawns = fortress_natural_enemy_spawns();
        assert_eq!(spawns.total_weight, 28);
        assert_eq!(
            spawns
                .entries
                .iter()
                .map(|entry| {
                    (
                        entry.data.entity_type.as_str(),
                        entry.data.min_count,
                        entry.data.max_count,
                        entry.weight,
                    )
                })
                .collect::<Vec<_>>(),
            [
                ("minecraft:blaze", 2, 3, 10),
                ("minecraft:zombified_piglin", 4, 4, 5),
                ("minecraft:wither_skeleton", 5, 5, 8),
                ("minecraft:skeleton", 5, 5, 2),
                ("minecraft:magma_cube", 4, 4, 3),
            ]
        );
    }
}
