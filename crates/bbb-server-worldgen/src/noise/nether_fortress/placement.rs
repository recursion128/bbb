//! Nether fortress `placeInChunk`. Every persisted piece kind is owned.

use crate::random::WorldgenRandom;
use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_set_property, catalog_default_state, WorldGenLevel};

use super::{FortressPiece, FortressPieceKind, FortressStructureStart};

/// `Block.UPDATE_CLIENTS` (`Block.java:92`).
const UPDATE_CLIENTS: i32 = 2;
/// `BuiltInLootTables.NETHER_BRIDGE` (`BuiltInLootTables.java:36`).
const BRIDGE_LOOT: &str = "minecraft:chests/nether_bridge";

impl FortressStructureStart {
    /// `StructureStart.placeInChunk` then each owned `postProcess`
    /// (`StructureStart.java:93-96`, `NetherFortressPieces.java:138-1603`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> bool {
        let mut placed = false;
        for piece in &mut self.pieces {
            if !piece.bounding_box.intersects(chunk_bounds) {
                continue;
            }
            placed = true;
            place_piece(piece, level, chunk_bounds, random);
        }
        placed
    }
}

fn place_piece<L: WorldGenLevel>(
    piece: &mut FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    match piece.kind {
        FortressPieceKind::Start | FortressPieceKind::BridgeCrossing => {
            place_bridge_crossing(piece, level, chunk_bounds)
        }
        FortressPieceKind::EndFiller => place_bridge_end_filler(piece, level, chunk_bounds),
        FortressPieceKind::BridgeStraight => place_bridge_straight(piece, level, chunk_bounds),
        FortressPieceKind::CorridorStairs => place_corridor_stairs(piece, level, chunk_bounds),
        FortressPieceKind::TBalcony => place_t_balcony(piece, level, chunk_bounds),
        FortressPieceKind::CastleEntrance => place_castle_entrance(piece, level, chunk_bounds),
        FortressPieceKind::SmallCrossing => place_small_crossing(piece, level, chunk_bounds),
        FortressPieceKind::SmallLeftTurn => {
            place_small_left_turn(piece, level, chunk_bounds, random)
        }
        FortressPieceKind::SmallCorridor => place_small_corridor(piece, level, chunk_bounds),
        FortressPieceKind::SmallRightTurn => {
            place_small_right_turn(piece, level, chunk_bounds, random)
        }
        FortressPieceKind::StalkRoom => place_stalk_room(piece, level, chunk_bounds),
        FortressPieceKind::MonsterThrone => place_monster_throne(piece, level, chunk_bounds),
        FortressPieceKind::RoomCrossing => place_room_crossing(piece, level, chunk_bounds),
        FortressPieceKind::StairsRoom => place_stairs_room(piece, level, chunk_bounds),
    }
}

fn frame(piece: &FortressPiece) -> GeneratedPieceFrame {
    GeneratedPieceFrame {
        bounding_box: piece.bounding_box,
        orientation: piece.orientation,
    }
}

fn state(name: &'static str) -> i32 {
    catalog_default_state(name)
}

fn with_prop(block: i32, name: &str, value: &str) -> i32 {
    block_set_property(block, name, value)
}

fn bricks() -> i32 {
    state("minecraft:nether_bricks")
}

fn air() -> i32 {
    state("minecraft:air")
}

fn fence(north: bool, east: bool, south: bool, west: bool) -> i32 {
    let mut block = state("minecraft:nether_brick_fence");
    if north {
        block = with_prop(block, "north", "true");
    }
    if east {
        block = with_prop(block, "east", "true");
    }
    if south {
        block = with_prop(block, "south", "true");
    }
    if west {
        block = with_prop(block, "west", "true");
    }
    block
}

fn stairs(facing: &'static str) -> i32 {
    with_prop(state("minecraft:nether_brick_stairs"), "facing", facing)
}

#[allow(clippy::too_many_arguments)]
fn fill_box<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
    block: i32,
) {
    frame.generate_box(
        level,
        chunk_bounds,
        x0,
        y0,
        z0,
        x1,
        y1,
        z1,
        block,
        block,
        false,
    )
}

fn put<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    block: i32,
    x: i32,
    y: i32,
    z: i32,
) {
    frame.place_block(level, chunk_bounds, block, x, y, z)
}

fn down<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    bricks: i32,
    x: i32,
    z: i32,
) {
    frame.fill_column_down(level, bricks, x, -1, z, chunk_bounds)
}

#[allow(clippy::too_many_arguments)]
fn maybe_chest<L: WorldGenLevel>(
    piece: &mut FortressPiece,
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
    x: i32,
    y: i32,
    z: i32,
) {
    if piece.chest_pending && chunk_bounds.contains(frame.world_pos(x, y, z)) {
        piece.chest_pending = false;
        frame.create_chest(level, chunk_bounds, random, x, y, z, BRIDGE_LOOT);
    }
}

/// `BridgeCrossing.postProcess` (`NetherFortressPieces.java:138-182`).
/// `StartPiece` inherits this body (`NetherFortressPieces.java:1606`).
fn place_bridge_crossing<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    fill_box(frame, level, chunk_bounds, 7, 3, 0, 11, 4, 18, bricks);
    fill_box(frame, level, chunk_bounds, 0, 3, 7, 18, 4, 11, bricks);
    fill_box(frame, level, chunk_bounds, 8, 5, 0, 10, 7, 18, air);
    fill_box(frame, level, chunk_bounds, 0, 5, 8, 18, 7, 10, air);
    fill_box(frame, level, chunk_bounds, 7, 5, 0, 7, 5, 7, bricks);
    fill_box(frame, level, chunk_bounds, 7, 5, 11, 7, 5, 18, bricks);
    fill_box(frame, level, chunk_bounds, 11, 5, 0, 11, 5, 7, bricks);
    fill_box(frame, level, chunk_bounds, 11, 5, 11, 11, 5, 18, bricks);
    fill_box(frame, level, chunk_bounds, 0, 5, 7, 7, 5, 7, bricks);
    fill_box(frame, level, chunk_bounds, 11, 5, 7, 18, 5, 7, bricks);
    fill_box(frame, level, chunk_bounds, 0, 5, 11, 7, 5, 11, bricks);
    fill_box(frame, level, chunk_bounds, 11, 5, 11, 18, 5, 11, bricks);
    fill_box(frame, level, chunk_bounds, 7, 2, 0, 11, 2, 5, bricks);
    fill_box(frame, level, chunk_bounds, 7, 2, 13, 11, 2, 18, bricks);
    fill_box(frame, level, chunk_bounds, 7, 0, 0, 11, 1, 3, bricks);
    fill_box(frame, level, chunk_bounds, 7, 0, 15, 11, 1, 18, bricks);
    for x in 7..=11 {
        for z in 0..=2 {
            down(frame, level, chunk_bounds, bricks, x, z);
            down(frame, level, chunk_bounds, bricks, x, 18 - z);
        }
    }
    fill_box(frame, level, chunk_bounds, 0, 2, 7, 5, 2, 11, bricks);
    fill_box(frame, level, chunk_bounds, 13, 2, 7, 18, 2, 11, bricks);
    fill_box(frame, level, chunk_bounds, 0, 0, 7, 3, 1, 11, bricks);
    fill_box(frame, level, chunk_bounds, 15, 0, 7, 18, 1, 11, bricks);
    for x in 0..=2 {
        for z in 7..=11 {
            down(frame, level, chunk_bounds, bricks, x, z);
            down(frame, level, chunk_bounds, bricks, 18 - x, z);
        }
    }
}

/// `BridgeEndFiller.postProcess` (`NetherFortressPieces.java:224-258`).
fn place_bridge_end_filler<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let mut self_random = WorldgenRandom::with_seed(i64::from(piece.random_seed));
    for x in 0..=4 {
        for y in 3..=4 {
            let z = self_random.next_int(8);
            fill_box(frame, level, chunk_bounds, x, y, 0, x, y, z, bricks);
        }
    }
    let mut z = self_random.next_int(8);
    fill_box(frame, level, chunk_bounds, 0, 5, 0, 0, 5, z, bricks);
    z = self_random.next_int(8);
    fill_box(frame, level, chunk_bounds, 4, 5, 0, 4, 5, z, bricks);
    for x in 0..=4 {
        let zx = self_random.next_int(5);
        fill_box(frame, level, chunk_bounds, x, 2, 0, x, 2, zx, bricks);
    }
    for x in 0..=4 {
        for y in 0..=1 {
            let zx = self_random.next_int(3);
            fill_box(frame, level, chunk_bounds, x, y, 0, x, y, zx, bricks);
        }
    }
}

/// `BridgeStraight.postProcess` (`NetherFortressPieces.java:296-332`).
fn place_bridge_straight<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    fill_box(frame, level, chunk_bounds, 0, 3, 0, 4, 4, 18, bricks);
    fill_box(frame, level, chunk_bounds, 1, 5, 0, 3, 7, 18, air);
    fill_box(frame, level, chunk_bounds, 0, 5, 0, 0, 5, 18, bricks);
    fill_box(frame, level, chunk_bounds, 4, 5, 0, 4, 5, 18, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 4, 2, 5, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 13, 4, 2, 18, bricks);
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 4, 1, 3, bricks);
    fill_box(frame, level, chunk_bounds, 0, 0, 15, 4, 1, 18, bricks);
    for x in 0..=4 {
        for z in 0..=2 {
            down(frame, level, chunk_bounds, bricks, x, z);
            down(frame, level, chunk_bounds, bricks, x, 18 - z);
        }
    }
    let nse = fence(true, true, true, false);
    let nsw = fence(true, false, true, true);
    fill_box(frame, level, chunk_bounds, 0, 1, 1, 0, 4, 1, nse);
    fill_box(frame, level, chunk_bounds, 0, 3, 4, 0, 4, 4, nse);
    fill_box(frame, level, chunk_bounds, 0, 3, 14, 0, 4, 14, nse);
    fill_box(frame, level, chunk_bounds, 0, 1, 17, 0, 4, 17, nse);
    fill_box(frame, level, chunk_bounds, 4, 1, 1, 4, 4, 1, nsw);
    fill_box(frame, level, chunk_bounds, 4, 3, 4, 4, 4, 4, nsw);
    fill_box(frame, level, chunk_bounds, 4, 3, 14, 4, 4, 14, nsw);
    fill_box(frame, level, chunk_bounds, 4, 1, 17, 4, 4, 17, nsw)
}

/// `CastleCorridorStairsPiece.postProcess` (`NetherFortressPieces.java:364-404`).
fn place_corridor_stairs<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    let stairs = stairs("south");
    let ns = fence(true, false, true, false);
    for step in 0..=9 {
        let floor = (7 - step).max(1);
        let roof = (floor + 5).max(14 - step).min(13);
        let z = step;
        fill_box(frame, level, chunk_bounds, 0, 0, z, 4, floor, z, bricks);
        fill_box(
            frame,
            level,
            chunk_bounds,
            1,
            floor + 1,
            z,
            3,
            roof - 1,
            z,
            air,
        );
        if step <= 6 {
            put(frame, level, chunk_bounds, stairs, 1, floor + 1, z);
            put(frame, level, chunk_bounds, stairs, 2, floor + 1, z);
            put(frame, level, chunk_bounds, stairs, 3, floor + 1, z);
        }
        fill_box(frame, level, chunk_bounds, 0, roof, z, 4, roof, z, bricks);
        fill_box(
            frame,
            level,
            chunk_bounds,
            0,
            floor + 1,
            z,
            0,
            roof - 1,
            z,
            bricks,
        );
        fill_box(
            frame,
            level,
            chunk_bounds,
            4,
            floor + 1,
            z,
            4,
            roof - 1,
            z,
            bricks,
        );
        if step & 1 == 0 {
            fill_box(
                frame,
                level,
                chunk_bounds,
                0,
                floor + 2,
                z,
                0,
                floor + 3,
                z,
                ns,
            );
            fill_box(
                frame,
                level,
                chunk_bounds,
                4,
                floor + 2,
                z,
                4,
                floor + 3,
                z,
                ns,
            );
        }
        for x in 0..=4 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `CastleCorridorTBalconyPiece.postProcess` (`NetherFortressPieces.java:443-485`).
fn place_t_balcony<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    let ns = fence(true, false, true, false);
    let we = fence(false, true, false, true);
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 8, 1, 8, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 8, 5, 8, air);
    fill_box(frame, level, chunk_bounds, 0, 6, 0, 8, 6, 5, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 2, 5, 0, bricks);
    fill_box(frame, level, chunk_bounds, 6, 2, 0, 8, 5, 0, bricks);
    fill_box(frame, level, chunk_bounds, 1, 3, 0, 1, 4, 0, we);
    fill_box(frame, level, chunk_bounds, 7, 3, 0, 7, 4, 0, we);
    fill_box(frame, level, chunk_bounds, 0, 2, 4, 8, 2, 8, bricks);
    fill_box(frame, level, chunk_bounds, 1, 1, 4, 2, 2, 4, air);
    fill_box(frame, level, chunk_bounds, 6, 1, 4, 7, 2, 4, air);
    fill_box(frame, level, chunk_bounds, 1, 3, 8, 7, 3, 8, we);
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, true, true, false),
        0,
        3,
        8,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, false, true, true),
        8,
        3,
        8,
    );
    fill_box(frame, level, chunk_bounds, 0, 3, 6, 0, 3, 7, ns);
    fill_box(frame, level, chunk_bounds, 8, 3, 6, 8, 3, 7, ns);
    fill_box(frame, level, chunk_bounds, 0, 3, 4, 0, 5, 5, bricks);
    fill_box(frame, level, chunk_bounds, 8, 3, 4, 8, 5, 5, bricks);
    fill_box(frame, level, chunk_bounds, 1, 3, 5, 2, 5, 5, bricks);
    fill_box(frame, level, chunk_bounds, 6, 3, 5, 7, 5, 5, bricks);
    fill_box(frame, level, chunk_bounds, 1, 4, 5, 1, 5, 5, we);
    fill_box(frame, level, chunk_bounds, 7, 4, 5, 7, 5, 5, we);
    for z in 0..=5 {
        for x in 0..=8 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `CastleEntrance.postProcess` (`NetherFortressPieces.java:523-611`).
fn place_castle_entrance<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    fill_box(frame, level, chunk_bounds, 0, 3, 0, 12, 4, 12, bricks);
    fill_box(frame, level, chunk_bounds, 0, 5, 0, 12, 13, 12, air);
    fill_box(frame, level, chunk_bounds, 0, 5, 0, 1, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 11, 5, 0, 12, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 11, 4, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 8, 5, 11, 10, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 5, 9, 11, 7, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 0, 4, 12, 1, bricks);
    fill_box(frame, level, chunk_bounds, 8, 5, 0, 10, 12, 1, bricks);
    fill_box(frame, level, chunk_bounds, 5, 9, 0, 7, 12, 1, bricks);
    fill_box(frame, level, chunk_bounds, 2, 11, 2, 10, 12, 10, bricks);
    fill_box(
        frame,
        level,
        chunk_bounds,
        5,
        8,
        0,
        7,
        8,
        0,
        state("minecraft:nether_brick_fence"),
    );
    let we = fence(false, true, false, true);
    let ns = fence(true, false, true, false);
    let mut i = 1;
    while i <= 11 {
        fill_box(frame, level, chunk_bounds, i, 10, 0, i, 11, 0, we);
        fill_box(frame, level, chunk_bounds, i, 10, 12, i, 11, 12, we);
        fill_box(frame, level, chunk_bounds, 0, 10, i, 0, 11, i, ns);
        fill_box(frame, level, chunk_bounds, 12, 10, i, 12, 11, i, ns);
        put(frame, level, chunk_bounds, bricks, i, 13, 0);
        put(frame, level, chunk_bounds, bricks, i, 13, 12);
        put(frame, level, chunk_bounds, bricks, 0, 13, i);
        put(frame, level, chunk_bounds, bricks, 12, 13, i);
        if i != 11 {
            put(frame, level, chunk_bounds, we, i + 1, 13, 0);
            put(frame, level, chunk_bounds, we, i + 1, 13, 12);
            put(frame, level, chunk_bounds, ns, 0, 13, i + 1);
            put(frame, level, chunk_bounds, ns, 12, 13, i + 1);
        }
        i += 2;
    }
    put(
        frame,
        level,
        chunk_bounds,
        fence(true, true, false, false),
        0,
        13,
        0,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, true, true, false),
        0,
        13,
        12,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, false, true, true),
        12,
        13,
        12,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(true, false, false, true),
        12,
        13,
        0,
    );
    let nsw = fence(true, false, true, true);
    let nse = fence(true, true, true, false);
    let mut z = 3;
    while z <= 9 {
        fill_box(frame, level, chunk_bounds, 1, 7, z, 1, 8, z, nsw);
        fill_box(frame, level, chunk_bounds, 11, 7, z, 11, 8, z, nse);
        z += 2;
    }
    fill_box(frame, level, chunk_bounds, 4, 2, 0, 8, 2, 12, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 4, 12, 2, 8, bricks);
    fill_box(frame, level, chunk_bounds, 4, 0, 0, 8, 1, 3, bricks);
    fill_box(frame, level, chunk_bounds, 4, 0, 9, 8, 1, 12, bricks);
    fill_box(frame, level, chunk_bounds, 0, 0, 4, 3, 1, 8, bricks);
    fill_box(frame, level, chunk_bounds, 9, 0, 4, 12, 1, 8, bricks);
    for x in 4..=8 {
        for z in 0..=2 {
            down(frame, level, chunk_bounds, bricks, x, z);
            down(frame, level, chunk_bounds, bricks, x, 12 - z);
        }
    }
    for x in 0..=2 {
        for z in 4..=8 {
            down(frame, level, chunk_bounds, bricks, x, z);
            down(frame, level, chunk_bounds, bricks, 12 - x, z);
        }
    }
    fill_box(frame, level, chunk_bounds, 5, 5, 5, 7, 5, 7, bricks);
    fill_box(frame, level, chunk_bounds, 6, 1, 6, 6, 4, 6, air);
    put(frame, level, chunk_bounds, bricks, 6, 0, 6);
    put(frame, level, chunk_bounds, state("minecraft:lava"), 6, 5, 6);
    let pos = frame.world_pos(6, 5, 6);
    if chunk_bounds.contains(pos) {
        level.schedule_fluid_tick(pos, 0);
    }
}

/// `CastleSmallCorridorCrossingPiece.postProcess` (`NetherFortressPieces.java:645-667`).
fn place_small_crossing<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 4, 1, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 4, 5, 4, air);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 0, 5, 0, bricks);
    fill_box(frame, level, chunk_bounds, 4, 2, 0, 4, 5, 0, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 4, 0, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 4, 2, 4, 4, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 6, 0, 4, 6, 4, bricks);
    for x in 0..=4 {
        for z in 0..=4 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `CastleSmallCorridorLeftTurnPiece.postProcess` (`NetherFortressPieces.java:714-746`).
fn place_small_left_turn<L: WorldGenLevel>(
    piece: &mut FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    let we = fence(false, true, false, true);
    let ns = fence(true, false, true, false);
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 4, 1, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 4, 5, 4, air);
    fill_box(frame, level, chunk_bounds, 4, 2, 0, 4, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 4, 3, 1, 4, 4, 1, ns);
    fill_box(frame, level, chunk_bounds, 4, 3, 3, 4, 4, 3, ns);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 0, 5, 0, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 4, 3, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 1, 3, 4, 1, 4, 4, we);
    fill_box(frame, level, chunk_bounds, 3, 3, 4, 3, 4, 4, we);
    maybe_chest(piece, frame, level, chunk_bounds, random, 3, 2, 3);
    fill_box(frame, level, chunk_bounds, 0, 6, 0, 4, 6, 4, bricks);
    for x in 0..=4 {
        for z in 0..=4 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `CastleSmallCorridorPiece.postProcess` (`NetherFortressPieces.java:778-803`).
fn place_small_corridor<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    let ns = fence(true, false, true, false);
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 4, 1, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 4, 5, 4, air);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 0, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 4, 2, 0, 4, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 3, 1, 0, 4, 1, ns);
    fill_box(frame, level, chunk_bounds, 0, 3, 3, 0, 4, 3, ns);
    fill_box(frame, level, chunk_bounds, 4, 3, 1, 4, 4, 1, ns);
    fill_box(frame, level, chunk_bounds, 4, 3, 3, 4, 4, 3, ns);
    fill_box(frame, level, chunk_bounds, 0, 6, 0, 4, 6, 4, bricks);
    for x in 0..=4 {
        for z in 0..=4 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `CastleSmallCorridorRightTurnPiece.postProcess` (`NetherFortressPieces.java:850-882`).
fn place_small_right_turn<L: WorldGenLevel>(
    piece: &mut FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    let we = fence(false, true, false, true);
    let ns = fence(true, false, true, false);
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 4, 1, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 4, 5, 4, air);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 0, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 3, 1, 0, 4, 1, ns);
    fill_box(frame, level, chunk_bounds, 0, 3, 3, 0, 4, 3, ns);
    fill_box(frame, level, chunk_bounds, 4, 2, 0, 4, 5, 0, bricks);
    fill_box(frame, level, chunk_bounds, 1, 2, 4, 4, 5, 4, bricks);
    fill_box(frame, level, chunk_bounds, 1, 3, 4, 1, 4, 4, we);
    fill_box(frame, level, chunk_bounds, 3, 3, 4, 3, 4, 4, we);
    maybe_chest(piece, frame, level, chunk_bounds, random, 1, 2, 3);
    fill_box(frame, level, chunk_bounds, 0, 6, 0, 4, 6, 4, bricks);
    for x in 0..=4 {
        for z in 0..=4 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `CastleStalkRoom.postProcess` (`NetherFortressPieces.java:915-1042`).
fn place_stalk_room<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    fill_box(frame, level, chunk_bounds, 0, 3, 0, 12, 4, 12, bricks);
    fill_box(frame, level, chunk_bounds, 0, 5, 0, 12, 13, 12, air);
    fill_box(frame, level, chunk_bounds, 0, 5, 0, 1, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 11, 5, 0, 12, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 11, 4, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 8, 5, 11, 10, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 5, 9, 11, 7, 12, 12, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 0, 4, 12, 1, bricks);
    fill_box(frame, level, chunk_bounds, 8, 5, 0, 10, 12, 1, bricks);
    fill_box(frame, level, chunk_bounds, 5, 9, 0, 7, 12, 1, bricks);
    fill_box(frame, level, chunk_bounds, 2, 11, 2, 10, 12, 10, bricks);
    let we = fence(false, true, false, true);
    let ns = fence(true, false, true, false);
    let nsw = fence(true, false, true, true);
    let nse = fence(true, true, true, false);
    let mut i = 1;
    while i <= 11 {
        fill_box(frame, level, chunk_bounds, i, 10, 0, i, 11, 0, we);
        fill_box(frame, level, chunk_bounds, i, 10, 12, i, 11, 12, we);
        fill_box(frame, level, chunk_bounds, 0, 10, i, 0, 11, i, ns);
        fill_box(frame, level, chunk_bounds, 12, 10, i, 12, 11, i, ns);
        put(frame, level, chunk_bounds, bricks, i, 13, 0);
        put(frame, level, chunk_bounds, bricks, i, 13, 12);
        put(frame, level, chunk_bounds, bricks, 0, 13, i);
        put(frame, level, chunk_bounds, bricks, 12, 13, i);
        if i != 11 {
            put(frame, level, chunk_bounds, we, i + 1, 13, 0);
            put(frame, level, chunk_bounds, we, i + 1, 13, 12);
            put(frame, level, chunk_bounds, ns, 0, 13, i + 1);
            put(frame, level, chunk_bounds, ns, 12, 13, i + 1);
        }
        i += 2;
    }
    put(
        frame,
        level,
        chunk_bounds,
        fence(true, true, false, false),
        0,
        13,
        0,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, true, true, false),
        0,
        13,
        12,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, false, true, true),
        12,
        13,
        12,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(true, false, false, true),
        12,
        13,
        0,
    );
    let mut z = 3;
    while z <= 9 {
        fill_box(frame, level, chunk_bounds, 1, 7, z, 1, 8, z, nsw);
        fill_box(frame, level, chunk_bounds, 11, 7, z, 11, 8, z, nse);
        z += 2;
    }
    let north_stairs = stairs("north");
    for i in 0..=6 {
        let z = i + 4;
        for x in 5..=7 {
            put(frame, level, chunk_bounds, north_stairs, x, 5 + i, z);
        }
        if (5..=8).contains(&z) {
            fill_box(frame, level, chunk_bounds, 5, 5, z, 7, i + 4, z, bricks);
        } else if (9..=10).contains(&z) {
            fill_box(frame, level, chunk_bounds, 5, 8, z, 7, i + 4, z, bricks);
        }
        if i >= 1 {
            fill_box(frame, level, chunk_bounds, 5, 6 + i, z, 7, 9 + i, z, air);
        }
    }
    for x in 5..=7 {
        put(frame, level, chunk_bounds, north_stairs, x, 12, 11);
    }
    fill_box(frame, level, chunk_bounds, 5, 6, 7, 5, 7, 7, nse);
    fill_box(frame, level, chunk_bounds, 7, 6, 7, 7, 7, 7, nsw);
    fill_box(frame, level, chunk_bounds, 5, 13, 12, 7, 13, 12, air);
    fill_box(frame, level, chunk_bounds, 2, 5, 2, 3, 5, 3, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 9, 3, 5, 10, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 4, 2, 5, 8, bricks);
    fill_box(frame, level, chunk_bounds, 9, 5, 2, 10, 5, 3, bricks);
    fill_box(frame, level, chunk_bounds, 9, 5, 9, 10, 5, 10, bricks);
    fill_box(frame, level, chunk_bounds, 10, 5, 4, 10, 5, 8, bricks);
    let east_stairs = stairs("east");
    let west_stairs = stairs("west");
    put(frame, level, chunk_bounds, west_stairs, 4, 5, 2);
    put(frame, level, chunk_bounds, west_stairs, 4, 5, 3);
    put(frame, level, chunk_bounds, west_stairs, 4, 5, 9);
    put(frame, level, chunk_bounds, west_stairs, 4, 5, 10);
    put(frame, level, chunk_bounds, east_stairs, 8, 5, 2);
    put(frame, level, chunk_bounds, east_stairs, 8, 5, 3);
    put(frame, level, chunk_bounds, east_stairs, 8, 5, 9);
    put(frame, level, chunk_bounds, east_stairs, 8, 5, 10);
    let soul = state("minecraft:soul_sand");
    let wart = state("minecraft:nether_wart");
    fill_box(frame, level, chunk_bounds, 3, 4, 4, 4, 4, 8, soul);
    fill_box(frame, level, chunk_bounds, 8, 4, 4, 9, 4, 8, soul);
    fill_box(frame, level, chunk_bounds, 3, 5, 4, 4, 5, 8, wart);
    fill_box(frame, level, chunk_bounds, 8, 5, 4, 9, 5, 8, wart);
    fill_box(frame, level, chunk_bounds, 4, 2, 0, 8, 2, 12, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 4, 12, 2, 8, bricks);
    fill_box(frame, level, chunk_bounds, 4, 0, 0, 8, 1, 3, bricks);
    fill_box(frame, level, chunk_bounds, 4, 0, 9, 8, 1, 12, bricks);
    fill_box(frame, level, chunk_bounds, 0, 0, 4, 3, 1, 8, bricks);
    fill_box(frame, level, chunk_bounds, 9, 0, 4, 12, 1, 8, bricks);
    for x in 4..=8 {
        for z in 0..=2 {
            down(frame, level, chunk_bounds, bricks, x, z);
            down(frame, level, chunk_bounds, bricks, x, 12 - z);
        }
    }
    for x in 0..=2 {
        for z in 4..=8 {
            down(frame, level, chunk_bounds, bricks, x, z);
            down(frame, level, chunk_bounds, bricks, 12 - x, z);
        }
    }
}

/// `MonsterThrone.postProcess` (`NetherFortressPieces.java:1077-1139`).
fn place_monster_throne<L: WorldGenLevel>(
    piece: &mut FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 6, 7, 7, air);
    fill_box(frame, level, chunk_bounds, 1, 0, 0, 5, 1, 7, bricks);
    fill_box(frame, level, chunk_bounds, 1, 2, 1, 5, 2, 7, bricks);
    fill_box(frame, level, chunk_bounds, 1, 3, 2, 5, 3, 7, bricks);
    fill_box(frame, level, chunk_bounds, 1, 4, 3, 5, 4, 7, bricks);
    fill_box(frame, level, chunk_bounds, 1, 2, 0, 1, 4, 2, bricks);
    fill_box(frame, level, chunk_bounds, 5, 2, 0, 5, 4, 2, bricks);
    fill_box(frame, level, chunk_bounds, 1, 5, 2, 1, 5, 3, bricks);
    fill_box(frame, level, chunk_bounds, 5, 5, 2, 5, 5, 3, bricks);
    fill_box(frame, level, chunk_bounds, 0, 5, 3, 0, 5, 8, bricks);
    fill_box(frame, level, chunk_bounds, 6, 5, 3, 6, 5, 8, bricks);
    fill_box(frame, level, chunk_bounds, 1, 5, 8, 5, 5, 8, bricks);
    let we = fence(false, true, false, true);
    let ns = fence(true, false, true, false);
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, false, false, true),
        1,
        6,
        3,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, true, false, false),
        5,
        6,
        3,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(true, true, false, false),
        0,
        6,
        3,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(true, false, false, true),
        6,
        6,
        3,
    );
    fill_box(frame, level, chunk_bounds, 0, 6, 4, 0, 6, 7, ns);
    fill_box(frame, level, chunk_bounds, 6, 6, 4, 6, 6, 7, ns);
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, true, true, false),
        0,
        6,
        8,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, false, true, true),
        6,
        6,
        8,
    );
    fill_box(frame, level, chunk_bounds, 1, 6, 8, 5, 6, 8, we);
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, true, false, false),
        1,
        7,
        8,
    );
    fill_box(frame, level, chunk_bounds, 2, 7, 8, 4, 7, 8, we);
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, false, false, true),
        5,
        7,
        8,
    );
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, true, false, false),
        2,
        8,
        8,
    );
    put(frame, level, chunk_bounds, we, 3, 8, 8);
    put(
        frame,
        level,
        chunk_bounds,
        fence(false, false, false, true),
        4,
        8,
        8,
    );
    if !piece.spawner_placed {
        let pos = frame.world_pos(3, 5, 5);
        if chunk_bounds.contains(pos) {
            piece.spawner_placed = true;
            let spawner = state("minecraft:spawner");
            level.set_block(pos, spawner, UPDATE_CLIENTS);
            level.set_spawner_entity_id(pos, "minecraft:blaze");
        }
    }
    for x in 0..=6 {
        for z in 0..=6 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `RoomCrossing.postProcess` (`NetherFortressPieces.java:1502-1537`).
fn place_room_crossing<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    let we = fence(false, true, false, true);
    let ns = fence(true, false, true, false);
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 6, 1, 6, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 6, 7, 6, air);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 1, 6, 0, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 6, 1, 6, 6, bricks);
    fill_box(frame, level, chunk_bounds, 5, 2, 0, 6, 6, 0, bricks);
    fill_box(frame, level, chunk_bounds, 5, 2, 6, 6, 6, 6, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 0, 6, 1, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 5, 0, 6, 6, bricks);
    fill_box(frame, level, chunk_bounds, 6, 2, 0, 6, 6, 1, bricks);
    fill_box(frame, level, chunk_bounds, 6, 2, 5, 6, 6, 6, bricks);
    fill_box(frame, level, chunk_bounds, 2, 6, 0, 4, 6, 0, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 0, 4, 5, 0, we);
    fill_box(frame, level, chunk_bounds, 2, 6, 6, 4, 6, 6, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 6, 4, 5, 6, we);
    fill_box(frame, level, chunk_bounds, 0, 6, 2, 0, 6, 4, bricks);
    fill_box(frame, level, chunk_bounds, 0, 5, 2, 0, 5, 4, ns);
    fill_box(frame, level, chunk_bounds, 6, 6, 2, 6, 6, 4, bricks);
    fill_box(frame, level, chunk_bounds, 6, 5, 2, 6, 5, 4, ns);
    for x in 0..=6 {
        for z in 0..=6 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

/// `StairsRoom.postProcess` (`NetherFortressPieces.java:1567-1603`).
fn place_stairs_room<L: WorldGenLevel>(
    piece: &FortressPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let frame = frame(piece);
    let bricks = bricks();
    let air = air();
    let we = fence(false, true, false, true);
    let ns = fence(true, false, true, false);
    fill_box(frame, level, chunk_bounds, 0, 0, 0, 6, 1, 6, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 6, 10, 6, air);
    fill_box(frame, level, chunk_bounds, 0, 2, 0, 1, 8, 0, bricks);
    fill_box(frame, level, chunk_bounds, 5, 2, 0, 6, 8, 0, bricks);
    fill_box(frame, level, chunk_bounds, 0, 2, 1, 0, 8, 6, bricks);
    fill_box(frame, level, chunk_bounds, 6, 2, 1, 6, 8, 6, bricks);
    fill_box(frame, level, chunk_bounds, 1, 2, 6, 5, 8, 6, bricks);
    fill_box(frame, level, chunk_bounds, 0, 3, 2, 0, 5, 4, ns);
    fill_box(frame, level, chunk_bounds, 6, 3, 2, 6, 5, 2, ns);
    fill_box(frame, level, chunk_bounds, 6, 3, 4, 6, 5, 4, ns);
    put(frame, level, chunk_bounds, bricks, 5, 2, 5);
    fill_box(frame, level, chunk_bounds, 4, 2, 5, 4, 3, 5, bricks);
    fill_box(frame, level, chunk_bounds, 3, 2, 5, 3, 4, 5, bricks);
    fill_box(frame, level, chunk_bounds, 2, 2, 5, 2, 5, 5, bricks);
    fill_box(frame, level, chunk_bounds, 1, 2, 5, 1, 6, 5, bricks);
    fill_box(frame, level, chunk_bounds, 1, 7, 1, 5, 7, 4, bricks);
    fill_box(frame, level, chunk_bounds, 6, 8, 2, 6, 8, 4, air);
    fill_box(frame, level, chunk_bounds, 2, 6, 0, 4, 8, 0, bricks);
    fill_box(frame, level, chunk_bounds, 2, 5, 0, 4, 5, 0, we);
    for x in 0..=6 {
        for z in 0..=6 {
            down(frame, level, chunk_bounds, bricks, x, z);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::random::WorldgenRandom;
    use bbb_protocol::packets::BlockPos;

    use super::*;
    use crate::noise::scattered::HorizontalDirection;
    use crate::noise::structures::StructureTerrainAdjustment;
    use crate::{block_is, WorldGenLevel};

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        loot: Vec<(BlockPos, &'static str, i64)>,
        spawners: Vec<(BlockPos, &'static str)>,
    }

    impl PlacementLevel {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
                air: catalog_default_state("minecraft:air"),
                loot: Vec::new(),
                spawners: Vec::new(),
            }
        }
    }

    impl WorldGenLevel for PlacementLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.cells
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.air)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            self.cells.insert((pos.x, pos.y, pos.z), state);
            true
        }

        fn get_min_y(&self) -> i32 {
            0
        }

        fn get_max_y(&self) -> i32 {
            255
        }

        fn get_sea_level(&self) -> i32 {
            32
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn set_block_entity_loot_table(
            &mut self,
            pos: BlockPos,
            loot_table: &'static str,
            seed: i64,
        ) {
            self.loot.push((pos, loot_table, seed));
        }

        fn set_spawner_entity_id(&mut self, pos: BlockPos, entity_type: &'static str) {
            self.spawners.push((pos, entity_type));
        }
    }

    fn wide_bounds() -> JigsawBoundingBox {
        JigsawBoundingBox {
            min: BlockPos {
                x: -512,
                y: 0,
                z: -512,
            },
            max: BlockPos {
                x: 512,
                y: 255,
                z: 512,
            },
        }
    }

    fn box_at(min: [i32; 3], max: [i32; 3]) -> JigsawBoundingBox {
        JigsawBoundingBox {
            min: BlockPos {
                x: min[0],
                y: min[1],
                z: min[2],
            },
            max: BlockPos {
                x: max[0],
                y: max[1],
                z: max[2],
            },
        }
    }

    fn piece(
        kind: FortressPieceKind,
        bounds: JigsawBoundingBox,
        chest_pending: bool,
    ) -> FortressPiece {
        FortressPiece {
            kind,
            generation_depth: i32::from(kind != FortressPieceKind::Start),
            bounding_box: bounds,
            orientation: HorizontalDirection::South,
            random_seed: 0,
            chest_pending,
            spawner_placed: false,
        }
    }

    fn start_with(
        kind: FortressPieceKind,
        bounds: JigsawBoundingBox,
        chest_pending: bool,
    ) -> FortressStructureStart {
        let start_bounds = if kind == FortressPieceKind::Start {
            bounds
        } else {
            box_at([2000, 64, 2000], [2018, 73, 2018])
        };
        let mut pieces = vec![piece(FortressPieceKind::Start, start_bounds, false)];
        if kind != FortressPieceKind::Start {
            pieces.push(piece(kind, bounds, chest_pending));
        }
        FortressStructureStart::from_parts([0, 0], 0, StructureTerrainAdjustment::None, pieces)
    }

    fn local(piece: &FortressPiece, x: i32, y: i32, z: i32) -> BlockPos {
        frame(piece).world_pos(x, y, z)
    }

    fn placed_piece(start: &FortressStructureStart, kind: FortressPieceKind) -> &FortressPiece {
        start
            .pieces()
            .iter()
            .find(|piece| piece.kind == kind)
            .expect("piece kind present")
    }

    #[test]
    fn bridge_crossing_writes_the_plus_walkway_instead_of_a_solid_shell() {
        let mut start = start_with(
            FortressPieceKind::BridgeCrossing,
            box_at([0, 64, 0], [18, 73, 18]),
            false,
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        assert!(start.place_in_chunk(&mut level, wide_bounds(), &mut random));
        let piece = placed_piece(&start, FortressPieceKind::BridgeCrossing);
        let walkway = local(piece, 7, 3, 0);
        let corner = local(piece, 0, 3, 0);
        let opening = local(piece, 8, 5, 9);
        let center = local(piece, 9, 3, 9);
        assert!(block_is(
            level.get_block(walkway),
            "minecraft:nether_bricks"
        ));
        assert!(block_is(level.get_block(center), "minecraft:nether_bricks"));
        assert!(!level.cells.contains_key(&(corner.x, corner.y, corner.z)));
        assert!(block_is(level.get_block(opening), "minecraft:air"));
        assert!(level.cells.contains_key(&(opening.x, opening.y, opening.z)));
    }

    #[test]
    fn monster_throne_places_the_blaze_spawner_at_local_3_5_5() {
        let mut start = start_with(
            FortressPieceKind::MonsterThrone,
            box_at([0, 64, 0], [6, 71, 8]),
            false,
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        start.place_in_chunk(&mut level, wide_bounds(), &mut random);
        let piece = placed_piece(&start, FortressPieceKind::MonsterThrone);
        let pos = local(piece, 3, 5, 5);
        assert!(piece.spawner_placed);
        assert!(block_is(level.get_block(pos), "minecraft:spawner"));
        assert_eq!(level.spawners, vec![(pos, "minecraft:blaze")]);
        assert!(!block_is(
            level.get_block(local(piece, 2, 5, 3)),
            "minecraft:spawner"
        ));
    }

    #[test]
    fn stalk_room_places_both_soul_sand_and_nether_wart_beds() {
        let mut start = start_with(
            FortressPieceKind::StalkRoom,
            box_at([0, 64, 0], [12, 77, 12]),
            false,
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        start.place_in_chunk(&mut level, wide_bounds(), &mut random);
        let piece = placed_piece(&start, FortressPieceKind::StalkRoom);
        assert!(block_is(
            level.get_block(local(piece, 3, 4, 4)),
            "minecraft:soul_sand"
        ));
        assert!(block_is(
            level.get_block(local(piece, 8, 4, 4)),
            "minecraft:soul_sand"
        ));
        assert!(block_is(
            level.get_block(local(piece, 4, 5, 8)),
            "minecraft:nether_wart"
        ));
        assert!(block_is(
            level.get_block(local(piece, 9, 5, 8)),
            "minecraft:nether_wart"
        ));
        assert!(block_is(
            level.get_block(local(piece, 0, 3, 0)),
            "minecraft:nether_bricks"
        ));
    }

    #[test]
    fn place_in_chunk_skips_pieces_that_miss_the_writable_area() {
        let mut start = start_with(
            FortressPieceKind::Start,
            box_at([0, 64, 0], [18, 73, 18]),
            false,
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        let far = JigsawBoundingBox {
            min: BlockPos {
                x: 256,
                y: 0,
                z: 256,
            },
            max: BlockPos {
                x: 271,
                y: 255,
                z: 271,
            },
        };
        assert!(!start.place_in_chunk(&mut level, far, &mut random));
        assert!(level.cells.is_empty());
        assert!(level.spawners.is_empty());
        assert!(level.loot.is_empty());
    }

    #[test]
    fn left_turn_records_nether_bridge_loot_at_local_3_2_3() {
        let mut start = start_with(
            FortressPieceKind::SmallLeftTurn,
            box_at([0, 64, 0], [4, 70, 4]),
            true,
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        start.place_in_chunk(&mut level, wide_bounds(), &mut random);
        let piece = placed_piece(&start, FortressPieceKind::SmallLeftTurn);
        assert!(!piece.chest_pending);
        let pos = local(piece, 3, 2, 3);
        assert!(block_is(level.get_block(pos), "minecraft:chest"));
        assert!(level
            .loot
            .iter()
            .any(|(loot_pos, table, _)| *loot_pos == pos && *table == BRIDGE_LOOT));
    }
}
