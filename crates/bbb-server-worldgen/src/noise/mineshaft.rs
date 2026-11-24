//! Deterministic mineshaft piece-graph generation before block placement.

#![allow(
    dead_code,
    reason = "whole-start FEATURES placement is the later caller"
)]

use crate::random::large_feature_random;
use crate::random::WorldgenRandom;
use bbb_protocol::packets::BlockPos;

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawHeightAccess};
use super::structures::{
    MineshaftType, NoiseStructurePlan, StructureHeightmap, StructureTerrainAdjustment,
};

pub mod persistence;
pub mod placement;
pub mod proto_entity;
pub mod start;
pub mod whole_placement;

pub use placement::{MineshaftPiecePlacement, MineshaftPlacementLevel};
pub use proto_entity::MineshaftChestMinecartRecord;
pub use whole_placement::MineshaftStartChunkPlacement;

const MAX_DEPTH: i32 = 8;
const MAX_HORIZONTAL_DISTANCE: i64 = 80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineshaftDirection {
    North,
    South,
    West,
    East,
}

impl MineshaftDirection {
    /// Horizontal ids used by `O` and crossing `D`
    /// (`Direction.java:32-47`).
    pub const fn nbt_id(self) -> i32 {
        match self {
            Self::South => 0,
            Self::West => 1,
            Self::North => 2,
            Self::East => 3,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::South => "south",
            Self::West => "west",
            Self::East => "east",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MineshaftPieceKind {
    Corridor {
        has_rails: bool,
        spider_corridor: bool,
        has_placed_spider: bool,
        num_sections: i32,
    },
    Crossing {
        direction: MineshaftDirection,
        two_floored: bool,
    },
    Room {
        entrances: Vec<JigsawBoundingBox>,
    },
    Stairs,
}

impl MineshaftPieceKind {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Corridor { .. } => "minecraft:mscorridor",
            Self::Crossing { .. } => "minecraft:mscrossing",
            Self::Room { .. } => "minecraft:msroom",
            Self::Stairs => "minecraft:msstairs",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MineshaftPiece {
    pub kind: MineshaftPieceKind,
    pub generation_depth: i32,
    pub bounding_box: JigsawBoundingBox,
    pub orientation: Option<MineshaftDirection>,
    pub mineshaft_type: MineshaftType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MineshaftPieceGraph {
    pub structure_id: String,
    pub start_chunk: [i32; 2],
    pub generation_anchor: BlockPos,
    pub vertical_offset: i32,
    pub pieces: Vec<MineshaftPiece>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MineshaftStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    pieces: Vec<MineshaftPiece>,
    cached_bounding_box: CachedStartBoundingBox,
}

impl MineshaftStructureStart {
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

    pub fn pieces(&self) -> &[MineshaftPiece] {
        &self.pieces
    }
}

impl MineshaftPieceGraph {
    pub fn bounding_box(&self) -> JigsawBoundingBox {
        combined_bounds(&self.pieces)
    }
}

impl NoiseStructurePlan {
    /// Builds the ordered pieces, then applies the NORMAL or MESA vertical
    /// adjustment before returning the generation anchor
    /// (`MineshaftStructure.java:36-64`).
    #[allow(clippy::too_many_arguments)]
    pub fn mineshaft_piece_graph(
        &self,
        structure_id: &str,
        level_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        sea_level: i32,
        height_access: &dyn JigsawHeightAccess,
    ) -> Option<MineshaftPieceGraph> {
        let settings = self.structures().get(structure_id)?.mineshaft?;
        let mut random = large_feature_random(level_seed, chunk_x, chunk_z);
        random.next_double();
        let west = chunk_x * 16 + 2;
        let north = chunk_z * 16 + 2;
        let mut pieces = vec![MineshaftPiece {
            kind: MineshaftPieceKind::Room {
                entrances: Vec::new(),
            },
            generation_depth: 0,
            bounding_box: box_from_corners(
                west,
                50,
                north,
                west + 7 + random.next_int(6),
                54 + random.next_int(6),
                north + 7 + random.next_int(6),
            ),
            orientation: None,
            mineshaft_type: settings.kind,
        }];
        add_children(0, 0, &mut pieces, &mut random);

        let bounds = combined_bounds(&pieces);
        let dy = match settings.kind {
            MineshaftType::Normal => {
                // `StructurePiecesBuilder.moveBelowSeaLevel`
                // (`StructurePiecesBuilder.java:31-43`).
                let max_y = sea_level - 10;
                let mut y1 = bounds.max.y - bounds.min.y + 1 + min_y + 1;
                if y1 < max_y {
                    y1 += random.next_int(max_y - y1);
                }
                y1 - bounds.max.y
            }
            MineshaftType::Mesa => {
                let center = box_center(bounds);
                let surface = height_access.first_free_height(
                    center.x,
                    center.z,
                    StructureHeightmap::WorldSurfaceWg,
                )?;
                let target = if surface <= sea_level {
                    sea_level
                } else {
                    sea_level + random.next_int(surface - sea_level + 1)
                };
                target - center.y
            }
        };
        offset_pieces(&mut pieces, dy);
        Some(MineshaftPieceGraph {
            structure_id: structure_id.to_string(),
            start_chunk: [chunk_x, chunk_z],
            generation_anchor: BlockPos {
                x: chunk_x * 16 + 8,
                y: 50 + dy,
                z: chunk_z * 16,
            },
            vertical_offset: dy,
            pieces,
        })
    }
}

/// `createRandomShaftPiece` chooses crossing/stairs/corridor in that order;
/// accepted pieces recurse immediately in insertion order
/// (`MineshaftPieces.java:48-105`).
#[allow(clippy::too_many_arguments)]
fn generate_and_add(
    root_index: usize,
    pieces: &mut Vec<MineshaftPiece>,
    random: &mut WorldgenRandom,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: MineshaftDirection,
    depth: i32,
) -> Option<usize> {
    if depth > MAX_DEPTH {
        return None;
    }
    let root = &pieces[root_index];
    if (i64::from(foot_x) - i64::from(root.bounding_box.min.x)).abs() > MAX_HORIZONTAL_DISTANCE
        || (i64::from(foot_z) - i64::from(root.bounding_box.min.z)).abs() > MAX_HORIZONTAL_DISTANCE
    {
        return None;
    }
    let piece = create_random_piece(
        pieces,
        random,
        foot_x,
        foot_y,
        foot_z,
        direction,
        depth + 1,
        root.mineshaft_type,
    )?;
    let index = pieces.len();
    pieces.push(piece);
    add_children(root_index, index, pieces, random);
    Some(index)
}

#[allow(clippy::too_many_arguments)]
fn create_random_piece(
    pieces: &[MineshaftPiece],
    random: &mut WorldgenRandom,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: MineshaftDirection,
    depth: i32,
    mineshaft_type: MineshaftType,
) -> Option<MineshaftPiece> {
    let selection = random.next_int(100);
    if selection >= 80 {
        let y_max = if random.next_int(4) == 0 { 6 } else { 2 };
        let bounds = relative_box(
            foot_x,
            foot_y,
            foot_z,
            match direction {
                MineshaftDirection::North => [-1, 0, -4, 3, y_max, 0],
                MineshaftDirection::South => [-1, 0, 0, 3, y_max, 4],
                MineshaftDirection::West => [-4, 0, -1, 0, y_max, 3],
                MineshaftDirection::East => [0, 0, -1, 4, y_max, 3],
            },
        );
        return (!collides(pieces, bounds)).then_some(MineshaftPiece {
            kind: MineshaftPieceKind::Crossing {
                direction,
                two_floored: bounds.max.y - bounds.min.y + 1 > 3,
            },
            generation_depth: depth,
            bounding_box: bounds,
            orientation: None,
            mineshaft_type,
        });
    }
    if selection >= 70 {
        let bounds = relative_box(
            foot_x,
            foot_y,
            foot_z,
            match direction {
                MineshaftDirection::North => [0, -5, -8, 2, 2, 0],
                MineshaftDirection::South => [0, -5, 0, 2, 2, 8],
                MineshaftDirection::West => [-8, -5, 0, 0, 2, 2],
                MineshaftDirection::East => [0, -5, 0, 8, 2, 2],
            },
        );
        return (!collides(pieces, bounds)).then_some(MineshaftPiece {
            kind: MineshaftPieceKind::Stairs,
            generation_depth: depth,
            bounding_box: bounds,
            orientation: Some(direction),
            mineshaft_type,
        });
    }

    let mut sections = random.next_int(3) + 2;
    while sections > 0 {
        let length = sections * 5;
        let bounds = relative_box(
            foot_x,
            foot_y,
            foot_z,
            match direction {
                MineshaftDirection::North => [0, 0, -(length - 1), 2, 2, 0],
                MineshaftDirection::South => [0, 0, 0, 2, 2, length - 1],
                MineshaftDirection::West => [-(length - 1), 0, 0, 0, 2, 2],
                MineshaftDirection::East => [0, 0, 0, length - 1, 2, 2],
            },
        );
        if !collides(pieces, bounds) {
            let has_rails = random.next_int(3) == 0;
            let spider_corridor = !has_rails && random.next_int(23) == 0;
            let num_sections = match direction {
                MineshaftDirection::North | MineshaftDirection::South => {
                    (bounds.max.z - bounds.min.z + 1) / 5
                }
                MineshaftDirection::West | MineshaftDirection::East => {
                    (bounds.max.x - bounds.min.x + 1) / 5
                }
            };
            return Some(MineshaftPiece {
                kind: MineshaftPieceKind::Corridor {
                    has_rails,
                    spider_corridor,
                    has_placed_spider: false,
                    num_sections,
                },
                generation_depth: depth,
                bounding_box: bounds,
                orientation: Some(direction),
                mineshaft_type,
            });
        }
        sections -= 1;
    }
    None
}

fn add_children(
    root_index: usize,
    index: usize,
    pieces: &mut Vec<MineshaftPiece>,
    random: &mut WorldgenRandom,
) {
    let piece = pieces[index].clone();
    let depth = piece.generation_depth;
    let bounds = piece.bounding_box;
    match (piece.kind, piece.orientation) {
        (MineshaftPieceKind::Room { .. }, _) => {
            add_room_children(root_index, index, bounds, pieces, random)
        }
        (MineshaftPieceKind::Corridor { .. }, Some(direction)) => {
            add_corridor_children(root_index, depth, bounds, direction, pieces, random)
        }
        (
            MineshaftPieceKind::Crossing {
                direction,
                two_floored,
            },
            _,
        ) => add_crossing_children(
            root_index,
            depth,
            bounds,
            direction,
            two_floored,
            pieces,
            random,
        ),
        (MineshaftPieceKind::Stairs, Some(direction)) => {
            add_stairs_child(root_index, depth, bounds, direction, pieces, random)
        }
        (MineshaftPieceKind::Corridor { .. } | MineshaftPieceKind::Stairs, None) => {}
    }
}

fn add_room_children(
    root_index: usize,
    room_index: usize,
    bounds: JigsawBoundingBox,
    pieces: &mut Vec<MineshaftPiece>,
    random: &mut WorldgenRandom,
) {
    let depth = pieces[room_index].generation_depth;
    let height_space = (bounds.max.y - bounds.min.y + 1 - 4).max(1);
    let x_span = bounds.max.x - bounds.min.x + 1;
    let z_span = bounds.max.z - bounds.min.z + 1;
    for direction in [MineshaftDirection::North, MineshaftDirection::South] {
        let mut pos = 0;
        while pos < x_span {
            pos += random.next_int(x_span);
            if pos + 3 > x_span {
                break;
            }
            let foot_x = bounds.min.x + pos;
            let foot_y = bounds.min.y + random.next_int(height_space) + 1;
            let foot_z = if direction == MineshaftDirection::North {
                bounds.min.z - 1
            } else {
                bounds.max.z + 1
            };
            if let Some(child) = generate_and_add(
                root_index, pieces, random, foot_x, foot_y, foot_z, direction, depth,
            ) {
                let child = pieces[child].bounding_box;
                let entrance = if direction == MineshaftDirection::North {
                    box_from_corners(
                        child.min.x,
                        child.min.y,
                        bounds.min.z,
                        child.max.x,
                        child.max.y,
                        bounds.min.z + 1,
                    )
                } else {
                    box_from_corners(
                        child.min.x,
                        child.min.y,
                        bounds.max.z - 1,
                        child.max.x,
                        child.max.y,
                        bounds.max.z,
                    )
                };
                if let MineshaftPieceKind::Room { entrances } = &mut pieces[room_index].kind {
                    entrances.push(entrance);
                }
            }
            pos += 4;
        }
    }
    for direction in [MineshaftDirection::West, MineshaftDirection::East] {
        let mut pos = 0;
        while pos < z_span {
            pos += random.next_int(z_span);
            if pos + 3 > z_span {
                break;
            }
            let foot_x = if direction == MineshaftDirection::West {
                bounds.min.x - 1
            } else {
                bounds.max.x + 1
            };
            let foot_y = bounds.min.y + random.next_int(height_space) + 1;
            let foot_z = bounds.min.z + pos;
            if let Some(child) = generate_and_add(
                root_index, pieces, random, foot_x, foot_y, foot_z, direction, depth,
            ) {
                let child = pieces[child].bounding_box;
                let entrance = if direction == MineshaftDirection::West {
                    box_from_corners(
                        bounds.min.x,
                        child.min.y,
                        child.min.z,
                        bounds.min.x + 1,
                        child.max.y,
                        child.max.z,
                    )
                } else {
                    box_from_corners(
                        bounds.max.x - 1,
                        child.min.y,
                        child.min.z,
                        bounds.max.x,
                        child.max.y,
                        child.max.z,
                    )
                };
                if let MineshaftPieceKind::Room { entrances } = &mut pieces[room_index].kind {
                    entrances.push(entrance);
                }
            }
            pos += 4;
        }
    }
}

fn add_corridor_children(
    root_index: usize,
    depth: i32,
    bounds: JigsawBoundingBox,
    direction: MineshaftDirection,
    pieces: &mut Vec<MineshaftPiece>,
    random: &mut WorldgenRandom,
) {
    let selection = random.next_int(4);
    let y = bounds.min.y - 1 + random.next_int(3);
    let (foot_x, foot_z, child_direction) = match direction {
        MineshaftDirection::North => match selection {
            0 | 1 => (bounds.min.x, bounds.min.z - 1, direction),
            2 => (bounds.min.x - 1, bounds.min.z, MineshaftDirection::West),
            _ => (bounds.max.x + 1, bounds.min.z, MineshaftDirection::East),
        },
        MineshaftDirection::South => match selection {
            0 | 1 => (bounds.min.x, bounds.max.z + 1, direction),
            2 => (bounds.min.x - 1, bounds.max.z - 3, MineshaftDirection::West),
            _ => (bounds.max.x + 1, bounds.max.z - 3, MineshaftDirection::East),
        },
        MineshaftDirection::West => match selection {
            0 | 1 => (bounds.min.x - 1, bounds.min.z, direction),
            2 => (bounds.min.x, bounds.min.z - 1, MineshaftDirection::North),
            _ => (bounds.min.x, bounds.max.z + 1, MineshaftDirection::South),
        },
        MineshaftDirection::East => match selection {
            0 | 1 => (bounds.max.x + 1, bounds.min.z, direction),
            2 => (
                bounds.max.x - 3,
                bounds.min.z - 1,
                MineshaftDirection::North,
            ),
            _ => (
                bounds.max.x - 3,
                bounds.max.z + 1,
                MineshaftDirection::South,
            ),
        },
    };
    generate_and_add(
        root_index,
        pieces,
        random,
        foot_x,
        y,
        foot_z,
        child_direction,
        depth,
    );

    if depth >= MAX_DEPTH {
        return;
    }
    match direction {
        MineshaftDirection::West | MineshaftDirection::East => {
            let mut x = bounds.min.x + 3;
            while x + 3 <= bounds.max.x {
                match random.next_int(5) {
                    0 => {
                        generate_and_add(
                            root_index,
                            pieces,
                            random,
                            x,
                            bounds.min.y,
                            bounds.min.z - 1,
                            MineshaftDirection::North,
                            depth + 1,
                        );
                    }
                    1 => {
                        generate_and_add(
                            root_index,
                            pieces,
                            random,
                            x,
                            bounds.min.y,
                            bounds.max.z + 1,
                            MineshaftDirection::South,
                            depth + 1,
                        );
                    }
                    _ => {}
                }
                x += 5;
            }
        }
        MineshaftDirection::North | MineshaftDirection::South => {
            let mut z = bounds.min.z + 3;
            while z + 3 <= bounds.max.z {
                match random.next_int(5) {
                    0 => {
                        generate_and_add(
                            root_index,
                            pieces,
                            random,
                            bounds.min.x - 1,
                            bounds.min.y,
                            z,
                            MineshaftDirection::West,
                            depth + 1,
                        );
                    }
                    1 => {
                        generate_and_add(
                            root_index,
                            pieces,
                            random,
                            bounds.max.x + 1,
                            bounds.min.y,
                            z,
                            MineshaftDirection::East,
                            depth + 1,
                        );
                    }
                    _ => {}
                }
                z += 5;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_crossing_children(
    root_index: usize,
    depth: i32,
    bounds: JigsawBoundingBox,
    direction: MineshaftDirection,
    two_floored: bool,
    pieces: &mut Vec<MineshaftPiece>,
    random: &mut WorldgenRandom,
) {
    let min_x_plus_one = bounds.min.x + 1;
    let min_z_plus_one = bounds.min.z + 1;
    let west_x = bounds.min.x - 1;
    let east_x = bounds.max.x + 1;
    let north_z = bounds.min.z - 1;
    let south_z = bounds.max.z + 1;
    let exits = match direction {
        MineshaftDirection::North => [
            (
                min_x_plus_one,
                bounds.min.y,
                north_z,
                MineshaftDirection::North,
            ),
            (
                west_x,
                bounds.min.y,
                min_z_plus_one,
                MineshaftDirection::West,
            ),
            (
                east_x,
                bounds.min.y,
                min_z_plus_one,
                MineshaftDirection::East,
            ),
        ],
        MineshaftDirection::South => [
            (
                min_x_plus_one,
                bounds.min.y,
                south_z,
                MineshaftDirection::South,
            ),
            (
                west_x,
                bounds.min.y,
                min_z_plus_one,
                MineshaftDirection::West,
            ),
            (
                east_x,
                bounds.min.y,
                min_z_plus_one,
                MineshaftDirection::East,
            ),
        ],
        MineshaftDirection::West => [
            (
                min_x_plus_one,
                bounds.min.y,
                north_z,
                MineshaftDirection::North,
            ),
            (
                min_x_plus_one,
                bounds.min.y,
                south_z,
                MineshaftDirection::South,
            ),
            (
                west_x,
                bounds.min.y,
                min_z_plus_one,
                MineshaftDirection::West,
            ),
        ],
        MineshaftDirection::East => [
            (
                min_x_plus_one,
                bounds.min.y,
                north_z,
                MineshaftDirection::North,
            ),
            (
                min_x_plus_one,
                bounds.min.y,
                south_z,
                MineshaftDirection::South,
            ),
            (
                east_x,
                bounds.min.y,
                min_z_plus_one,
                MineshaftDirection::East,
            ),
        ],
    };
    for (x, y, z, exit_direction) in exits {
        generate_and_add(root_index, pieces, random, x, y, z, exit_direction, depth);
    }
    if two_floored {
        let upper_y = bounds.min.y + 4;
        for (x, z, upper_direction) in [
            (min_x_plus_one, north_z, MineshaftDirection::North),
            (west_x, min_z_plus_one, MineshaftDirection::West),
            (east_x, min_z_plus_one, MineshaftDirection::East),
            (min_x_plus_one, south_z, MineshaftDirection::South),
        ] {
            if random.next_boolean() {
                generate_and_add(
                    root_index,
                    pieces,
                    random,
                    x,
                    upper_y,
                    z,
                    upper_direction,
                    depth,
                );
            }
        }
    }
}

fn add_stairs_child(
    root_index: usize,
    depth: i32,
    bounds: JigsawBoundingBox,
    direction: MineshaftDirection,
    pieces: &mut Vec<MineshaftPiece>,
    random: &mut WorldgenRandom,
) {
    let (x, z) = match direction {
        MineshaftDirection::North => (bounds.min.x, bounds.min.z - 1),
        MineshaftDirection::South => (bounds.min.x, bounds.max.z + 1),
        MineshaftDirection::West => (bounds.min.x - 1, bounds.min.z),
        MineshaftDirection::East => (bounds.max.x + 1, bounds.min.z),
    };
    generate_and_add(
        root_index,
        pieces,
        random,
        x,
        bounds.min.y,
        z,
        direction,
        depth,
    );
}

fn collides(pieces: &[MineshaftPiece], bounds: JigsawBoundingBox) -> bool {
    pieces
        .iter()
        .any(|piece| piece.bounding_box.intersects(bounds))
}

fn offset_pieces(pieces: &mut [MineshaftPiece], dy: i32) {
    for piece in pieces {
        piece.bounding_box = offset_box_y(piece.bounding_box, dy);
        if let MineshaftPieceKind::Room { entrances } = &mut piece.kind {
            for entrance in entrances {
                *entrance = offset_box_y(*entrance, dy);
            }
        }
    }
}

fn combined_bounds(pieces: &[MineshaftPiece]) -> JigsawBoundingBox {
    let mut bounds = pieces[0].bounding_box;
    for piece in &pieces[1..] {
        bounds.min.x = bounds.min.x.min(piece.bounding_box.min.x);
        bounds.min.y = bounds.min.y.min(piece.bounding_box.min.y);
        bounds.min.z = bounds.min.z.min(piece.bounding_box.min.z);
        bounds.max.x = bounds.max.x.max(piece.bounding_box.max.x);
        bounds.max.y = bounds.max.y.max(piece.bounding_box.max.y);
        bounds.max.z = bounds.max.z.max(piece.bounding_box.max.z);
    }
    bounds
}

fn box_center(bounds: JigsawBoundingBox) -> BlockPos {
    BlockPos {
        x: bounds.min.x + (bounds.max.x - bounds.min.x + 1) / 2,
        y: bounds.min.y + (bounds.max.y - bounds.min.y + 1) / 2,
        z: bounds.min.z + (bounds.max.z - bounds.min.z + 1) / 2,
    }
}

fn relative_box(foot_x: i32, foot_y: i32, foot_z: i32, offsets: [i32; 6]) -> JigsawBoundingBox {
    box_from_corners(
        foot_x + offsets[0],
        foot_y + offsets[1],
        foot_z + offsets[2],
        foot_x + offsets[3],
        foot_y + offsets[4],
        foot_z + offsets[5],
    )
}

fn box_from_corners(
    min_x: i32,
    min_y: i32,
    min_z: i32,
    max_x: i32,
    max_y: i32,
    max_z: i32,
) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: min_x,
            y: min_y,
            z: min_z,
        },
        max: BlockPos {
            x: max_x,
            y: max_y,
            z: max_z,
        },
    }
}

fn offset_box_y(bounds: JigsawBoundingBox, dy: i32) -> JigsawBoundingBox {
    box_from_corners(
        bounds.min.x,
        bounds.min.y + dy,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y + dy,
        bounds.max.z,
    )
}
