//! Stronghold `placeInChunk`. Every persisted piece kind is owned.

use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::{bounding_box_from_array, GeneratedPieceFrame};
use crate::noise::scattered::HorizontalDirection;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_set_property, catalog_default_state, WorldGenLevel};

use super::{
    SmallDoorType, StrongholdPiece, StrongholdPieceExtra, StrongholdPieceKind,
    StrongholdStructureStart,
};

const UPDATE_CLIENTS: i32 = 2;
const CORRIDOR_LOOT: &str = "minecraft:chests/stronghold_corridor";
const LIBRARY_LOOT: &str = "minecraft:chests/stronghold_library";
const CROSSING_LOOT: &str = "minecraft:chests/stronghold_crossing";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StrongholdChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
}

impl StrongholdStructureStart {
    /// `StructureStart.placeInChunk` then each owned `postProcess`
    /// (`StructureStart.java:93-96`, `StrongholdPieces.java:265-1442`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> StrongholdChunkPlacement {
        let mut report = StrongholdChunkPlacement::default();
        for piece in self.pieces_mut() {
            let piece_bounds = bounding_box_from_array(piece.bounding_box());
            if !piece_bounds.intersects(chunk_bounds) {
                continue;
            }
            report.attempted = true;
            place_piece(piece, level, chunk_bounds, random);
            report.placed = true;
        }
        report
    }
}

fn place_piece<L: WorldGenLevel>(
    piece: &mut StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    match piece.kind() {
        StrongholdPieceKind::ChestCorridor => {
            place_chest_corridor(piece, level, chunk_bounds, random)
        }
        StrongholdPieceKind::FillerCorridor => place_filler_corridor(piece, level, chunk_bounds),
        StrongholdPieceKind::FiveCrossing => {
            place_five_crossing(piece, level, chunk_bounds, random)
        }
        StrongholdPieceKind::LeftTurn => place_left_turn(piece, level, chunk_bounds, random),
        StrongholdPieceKind::Library => place_library(piece, level, chunk_bounds, random),
        StrongholdPieceKind::PortalRoom => place_portal_room(piece, level, chunk_bounds, random),
        StrongholdPieceKind::PrisonHall => place_prison_hall(piece, level, chunk_bounds, random),
        StrongholdPieceKind::RightTurn => place_right_turn(piece, level, chunk_bounds, random),
        StrongholdPieceKind::RoomCrossing => {
            place_room_crossing(piece, level, chunk_bounds, random)
        }
        StrongholdPieceKind::StairsDown | StrongholdPieceKind::Start => {
            place_stairs_down(piece, level, chunk_bounds, random)
        }
        StrongholdPieceKind::Straight => place_straight(piece, level, chunk_bounds, random),
        StrongholdPieceKind::StraightStairsDown => {
            place_straight_stairs_down(piece, level, chunk_bounds, random)
        }
    }
}

fn frame(piece: &StrongholdPiece) -> GeneratedPieceFrame {
    GeneratedPieceFrame::from_array(piece.bounding_box(), piece.orientation())
}

fn state(name: &'static str) -> i32 {
    catalog_default_state(name)
}

fn with_prop(block: i32, name: &str, value: &str) -> i32 {
    block_set_property(block, name, value)
}

fn cave_air() -> i32 {
    state("minecraft:cave_air")
}

fn stone_bricks() -> i32 {
    state("minecraft:stone_bricks")
}

fn smooth_selector(random: &mut dyn RandomSource, is_edge: bool) -> i32 {
    if is_edge {
        let selection = random.next_float();
        catalog_default_state(if selection < 0.2 {
            "minecraft:cracked_stone_bricks"
        } else if selection < 0.5 {
            "minecraft:mossy_stone_bricks"
        } else if selection < 0.55 {
            "minecraft:infested_stone_bricks"
        } else {
            "minecraft:stone_bricks"
        })
    } else {
        catalog_default_state("minecraft:cave_air")
    }
}

#[allow(clippy::too_many_arguments)]
fn selected_box<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
    skip_air: bool,
    random: &mut dyn RandomSource,
) {
    frame.generate_box_selected(
        level,
        chunk_bounds,
        x0,
        y0,
        z0,
        x1,
        y1,
        z1,
        skip_air,
        random,
        smooth_selector,
    )
}

#[allow(clippy::too_many_arguments)]
fn air_box<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
) {
    let air = cave_air();
    frame.generate_box(level, chunk_bounds, x0, y0, z0, x1, y1, z1, air, air, false)
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

fn bars(north: bool, south: bool, west: bool, east: bool) -> i32 {
    let mut block = state("minecraft:iron_bars");
    if north {
        block = with_prop(block, "north", "true");
    }
    if south {
        block = with_prop(block, "south", "true");
    }
    if west {
        block = with_prop(block, "west", "true");
    }
    if east {
        block = with_prop(block, "east", "true");
    }
    block
}

fn fence(north: bool, south: bool, west: bool, east: bool) -> i32 {
    let mut block = state("minecraft:oak_fence");
    if north {
        block = with_prop(block, "north", "true");
    }
    if south {
        block = with_prop(block, "south", "true");
    }
    if west {
        block = with_prop(block, "west", "true");
    }
    if east {
        block = with_prop(block, "east", "true");
    }
    block
}

fn wall_torch(facing: &str) -> i32 {
    with_prop(state("minecraft:wall_torch"), "facing", facing)
}

fn generate_small_door<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    door: SmallDoorType,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
) {
    match door {
        SmallDoorType::Opening => air_box(
            frame,
            level,
            chunk_bounds,
            foot_x,
            foot_y,
            foot_z,
            foot_x + 2,
            foot_y + 2,
            foot_z,
        ),
        SmallDoorType::WoodDoor => {
            let bricks = stone_bricks();
            for (x, y) in [
                (foot_x, foot_y),
                (foot_x, foot_y + 1),
                (foot_x, foot_y + 2),
                (foot_x + 1, foot_y + 2),
                (foot_x + 2, foot_y + 2),
                (foot_x + 2, foot_y + 1),
                (foot_x + 2, foot_y),
            ] {
                put(frame, level, chunk_bounds, bricks, x, y, foot_z);
            }
            let door = state("minecraft:oak_door");
            put(frame, level, chunk_bounds, door, foot_x + 1, foot_y, foot_z);
            put(
                frame,
                level,
                chunk_bounds,
                with_prop(door, "half", "upper"),
                foot_x + 1,
                foot_y + 1,
                foot_z,
            )
        }
        SmallDoorType::Grates => {
            let air = cave_air();
            put(frame, level, chunk_bounds, air, foot_x + 1, foot_y, foot_z);
            put(
                frame,
                level,
                chunk_bounds,
                air,
                foot_x + 1,
                foot_y + 1,
                foot_z,
            );
            put(
                frame,
                level,
                chunk_bounds,
                bars(false, false, true, false),
                foot_x,
                foot_y,
                foot_z,
            );
            put(
                frame,
                level,
                chunk_bounds,
                bars(false, false, true, false),
                foot_x,
                foot_y + 1,
                foot_z,
            );
            let top = bars(false, false, true, true);
            put(frame, level, chunk_bounds, top, foot_x, foot_y + 2, foot_z);
            put(
                frame,
                level,
                chunk_bounds,
                top,
                foot_x + 1,
                foot_y + 2,
                foot_z,
            );
            put(
                frame,
                level,
                chunk_bounds,
                top,
                foot_x + 2,
                foot_y + 2,
                foot_z,
            );
            put(
                frame,
                level,
                chunk_bounds,
                bars(false, false, false, true),
                foot_x + 2,
                foot_y + 1,
                foot_z,
            );
            put(
                frame,
                level,
                chunk_bounds,
                bars(false, false, false, true),
                foot_x + 2,
                foot_y,
                foot_z,
            )
        }
        SmallDoorType::IronDoor => {
            let bricks = stone_bricks();
            for (x, y) in [
                (foot_x, foot_y),
                (foot_x, foot_y + 1),
                (foot_x, foot_y + 2),
                (foot_x + 1, foot_y + 2),
                (foot_x + 2, foot_y + 2),
                (foot_x + 2, foot_y + 1),
                (foot_x + 2, foot_y),
            ] {
                put(frame, level, chunk_bounds, bricks, x, y, foot_z);
            }
            let door = state("minecraft:iron_door");
            put(frame, level, chunk_bounds, door, foot_x + 1, foot_y, foot_z);
            put(
                frame,
                level,
                chunk_bounds,
                with_prop(door, "half", "upper"),
                foot_x + 1,
                foot_y + 1,
                foot_z,
            );
            let button = state("minecraft:stone_button");
            put(
                frame,
                level,
                chunk_bounds,
                with_prop(button, "facing", "north"),
                foot_x + 2,
                foot_y + 1,
                foot_z + 1,
            );
            put(
                frame,
                level,
                chunk_bounds,
                with_prop(button, "facing", "south"),
                foot_x + 2,
                foot_y + 1,
                foot_z - 1,
            )
        }
    }
}

fn place_chest_corridor<L: WorldGenLevel>(
    piece: &mut StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 4, 4, 6, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 1, 1, 0);
    generate_small_door(frame, level, chunk_bounds, SmallDoorType::Opening, 1, 1, 6);
    fill_box(frame, level, chunk_bounds, 3, 1, 2, 3, 1, 4, stone_bricks());
    let slab = state("minecraft:stone_brick_slab");
    put(frame, level, chunk_bounds, slab, 3, 1, 1);
    put(frame, level, chunk_bounds, slab, 3, 1, 5);
    put(frame, level, chunk_bounds, slab, 3, 2, 2);
    put(frame, level, chunk_bounds, slab, 3, 2, 4);
    for z in 2..=4 {
        put(frame, level, chunk_bounds, slab, 2, 1, z);
    }
    let StrongholdPieceExtra::ChestCorridor { chest } = piece.extra() else {
        unreachable!("piece extra is not a chest corridor")
    };
    if !*chest && chunk_bounds.contains(frame.world_pos(3, 2, 3)) {
        if let StrongholdPieceExtra::ChestCorridor { chest } = piece.extra_mut() {
            *chest = true;
        }
        frame.create_chest(level, chunk_bounds, random, 3, 2, 3, CORRIDOR_LOOT);
    }
}

fn place_filler_corridor<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let StrongholdPieceExtra::FillerCorridor { steps } = *piece.extra() else {
        unreachable!("piece extra is not a filler corridor")
    };
    let frame = frame(piece);
    let bricks = stone_bricks();
    let air = cave_air();
    for i in 0..steps {
        for x in 0..=4 {
            put(frame, level, chunk_bounds, bricks, x, 0, i);
            put(frame, level, chunk_bounds, bricks, x, 4, i);
        }
        for y in 1..=3 {
            put(frame, level, chunk_bounds, bricks, 0, y, i);
            put(frame, level, chunk_bounds, air, 1, y, i);
            put(frame, level, chunk_bounds, air, 2, y, i);
            put(frame, level, chunk_bounds, air, 3, y, i);
            put(frame, level, chunk_bounds, bricks, 4, y, i);
        }
    }
}

fn place_five_crossing<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let StrongholdPieceExtra::FiveCrossing {
        left_low,
        left_high,
        right_low,
        right_high,
    } = *piece.extra()
    else {
        unreachable!("piece extra is not a five crossing")
    };
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 9, 8, 10, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 4, 3, 0);
    if left_low {
        air_box(frame, level, chunk_bounds, 0, 3, 1, 0, 5, 3);
    }
    if right_low {
        air_box(frame, level, chunk_bounds, 9, 3, 1, 9, 5, 3);
    }
    if left_high {
        air_box(frame, level, chunk_bounds, 0, 5, 7, 0, 7, 9);
    }
    if right_high {
        air_box(frame, level, chunk_bounds, 9, 5, 7, 9, 7, 9);
    }
    air_box(frame, level, chunk_bounds, 5, 1, 10, 7, 3, 10);
    selected_box(frame, level, chunk_bounds, 1, 2, 1, 8, 2, 6, false, random);
    selected_box(frame, level, chunk_bounds, 4, 1, 5, 4, 4, 9, false, random);
    selected_box(frame, level, chunk_bounds, 8, 1, 5, 8, 4, 9, false, random);
    selected_box(frame, level, chunk_bounds, 1, 4, 7, 3, 4, 9, false, random);
    selected_box(frame, level, chunk_bounds, 1, 3, 5, 3, 3, 6, false, random);
    let slab = state("minecraft:smooth_stone_slab");
    fill_box(frame, level, chunk_bounds, 1, 3, 4, 3, 3, 4, slab);
    fill_box(frame, level, chunk_bounds, 1, 4, 6, 3, 4, 6, slab);
    selected_box(frame, level, chunk_bounds, 5, 1, 7, 7, 1, 8, false, random);
    fill_box(frame, level, chunk_bounds, 5, 1, 9, 7, 1, 9, slab);
    fill_box(frame, level, chunk_bounds, 5, 2, 7, 7, 2, 7, slab);
    fill_box(frame, level, chunk_bounds, 4, 5, 7, 4, 5, 9, slab);
    fill_box(frame, level, chunk_bounds, 8, 5, 7, 8, 5, 9, slab);
    fill_box(
        frame,
        level,
        chunk_bounds,
        5,
        5,
        7,
        7,
        5,
        9,
        with_prop(slab, "type", "double"),
    );
    put(frame, level, chunk_bounds, wall_torch("south"), 6, 5, 6)
}

fn place_left_turn<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 4, 4, 4, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 1, 1, 0);
    if !matches!(
        piece.orientation(),
        HorizontalDirection::North | HorizontalDirection::East
    ) {
        air_box(frame, level, chunk_bounds, 4, 1, 1, 4, 3, 3)
    } else {
        air_box(frame, level, chunk_bounds, 0, 1, 1, 0, 3, 3)
    }
}

fn place_right_turn<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 4, 4, 4, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 1, 1, 0);
    if !matches!(
        piece.orientation(),
        HorizontalDirection::North | HorizontalDirection::East
    ) {
        air_box(frame, level, chunk_bounds, 0, 1, 1, 0, 3, 3)
    } else {
        air_box(frame, level, chunk_bounds, 4, 1, 1, 4, 3, 3)
    }
}

fn place_library<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let StrongholdPieceExtra::Library { tall } = *piece.extra() else {
        unreachable!("piece extra is not a library")
    };
    let frame = frame(piece);
    let height = if tall { 11 } else { 6 };
    selected_box(
        frame,
        level,
        chunk_bounds,
        0,
        0,
        0,
        13,
        height - 1,
        14,
        true,
        random,
    );
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 4, 1, 0);
    let web = state("minecraft:cobweb");
    frame.generate_maybe_box(
        level,
        chunk_bounds,
        random,
        0.07,
        2,
        1,
        1,
        11,
        4,
        13,
        web,
        web,
        false,
        false,
    );
    let planks = state("minecraft:oak_planks");
    let shelf = state("minecraft:bookshelf");
    for d in 1..=13 {
        if (d - 1) % 4 == 0 {
            fill_box(frame, level, chunk_bounds, 1, 1, d, 1, 4, d, planks);
            fill_box(frame, level, chunk_bounds, 12, 1, d, 12, 4, d, planks);
            put(frame, level, chunk_bounds, wall_torch("east"), 2, 3, d);
            put(frame, level, chunk_bounds, wall_torch("west"), 11, 3, d);
            if tall {
                fill_box(frame, level, chunk_bounds, 1, 6, d, 1, 9, d, planks);
                fill_box(frame, level, chunk_bounds, 12, 6, d, 12, 9, d, planks);
            }
        } else {
            fill_box(frame, level, chunk_bounds, 1, 1, d, 1, 4, d, shelf);
            fill_box(frame, level, chunk_bounds, 12, 1, d, 12, 4, d, shelf);
            if tall {
                fill_box(frame, level, chunk_bounds, 1, 6, d, 1, 9, d, shelf);
                fill_box(frame, level, chunk_bounds, 12, 6, d, 12, 9, d, shelf);
            }
        }
    }
    let mut d = 3;
    while d < 12 {
        fill_box(frame, level, chunk_bounds, 3, 1, d, 4, 3, d, shelf);
        fill_box(frame, level, chunk_bounds, 6, 1, d, 7, 3, d, shelf);
        fill_box(frame, level, chunk_bounds, 9, 1, d, 10, 3, d, shelf);
        d += 2;
    }
    if tall {
        fill_box(frame, level, chunk_bounds, 1, 5, 1, 3, 5, 13, planks);
        fill_box(frame, level, chunk_bounds, 10, 5, 1, 12, 5, 13, planks);
        fill_box(frame, level, chunk_bounds, 4, 5, 1, 9, 5, 2, planks);
        fill_box(frame, level, chunk_bounds, 4, 5, 12, 9, 5, 13, planks);
        put(frame, level, chunk_bounds, planks, 9, 5, 11);
        put(frame, level, chunk_bounds, planks, 8, 5, 11);
        put(frame, level, chunk_bounds, planks, 9, 5, 10);
        let ns = fence(true, true, false, false);
        let we = fence(false, false, true, true);
        fill_box(frame, level, chunk_bounds, 3, 6, 3, 3, 6, 11, ns);
        fill_box(frame, level, chunk_bounds, 10, 6, 3, 10, 6, 9, ns);
        fill_box(frame, level, chunk_bounds, 4, 6, 2, 9, 6, 2, we);
        fill_box(frame, level, chunk_bounds, 4, 6, 12, 7, 6, 12, we);
        put(
            frame,
            level,
            chunk_bounds,
            fence(true, false, false, true),
            3,
            6,
            2,
        );
        put(
            frame,
            level,
            chunk_bounds,
            fence(false, true, false, true),
            3,
            6,
            12,
        );
        put(
            frame,
            level,
            chunk_bounds,
            fence(true, false, true, false),
            10,
            6,
            2,
        );
        for i in 0..=2 {
            put(
                frame,
                level,
                chunk_bounds,
                fence(false, true, true, false),
                8 + i,
                6,
                12 - i,
            );
            if i != 2 {
                put(
                    frame,
                    level,
                    chunk_bounds,
                    fence(true, false, false, true),
                    8 + i,
                    6,
                    11 - i,
                );
            }
        }
        let ladder = with_prop(state("minecraft:ladder"), "facing", "south");
        for y in 1..=7 {
            put(frame, level, chunk_bounds, ladder, 10, y, 13);
        }
        let e_fence = fence(false, false, false, true);
        let w_fence = fence(false, false, true, false);
        put(frame, level, chunk_bounds, e_fence, 6, 9, 7);
        put(frame, level, chunk_bounds, w_fence, 7, 9, 7);
        put(frame, level, chunk_bounds, e_fence, 6, 8, 7);
        put(frame, level, chunk_bounds, w_fence, 7, 8, 7);
        let nswe = fence(true, true, true, true);
        put(frame, level, chunk_bounds, nswe, 6, 7, 7);
        put(frame, level, chunk_bounds, nswe, 7, 7, 7);
        put(frame, level, chunk_bounds, e_fence, 5, 7, 7);
        put(frame, level, chunk_bounds, w_fence, 8, 7, 7);
        put(
            frame,
            level,
            chunk_bounds,
            fence(true, false, false, true),
            6,
            7,
            6,
        );
        put(
            frame,
            level,
            chunk_bounds,
            fence(false, true, false, true),
            6,
            7,
            8,
        );
        put(
            frame,
            level,
            chunk_bounds,
            fence(true, false, true, false),
            7,
            7,
            6,
        );
        put(
            frame,
            level,
            chunk_bounds,
            fence(false, true, true, false),
            7,
            7,
            8,
        );
        let torch = state("minecraft:torch");
        put(frame, level, chunk_bounds, torch, 5, 8, 7);
        put(frame, level, chunk_bounds, torch, 8, 8, 7);
        put(frame, level, chunk_bounds, torch, 6, 8, 6);
        put(frame, level, chunk_bounds, torch, 6, 8, 8);
        put(frame, level, chunk_bounds, torch, 7, 8, 6);
        put(frame, level, chunk_bounds, torch, 7, 8, 8);
    }
    frame.create_chest(level, chunk_bounds, random, 3, 3, 5, LIBRARY_LOOT);
    if tall {
        put(frame, level, chunk_bounds, cave_air(), 12, 9, 1);
        frame.create_chest(level, chunk_bounds, random, 12, 8, 1, LIBRARY_LOOT);
    }
}

fn place_portal_room<L: WorldGenLevel>(
    piece: &mut StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    selected_box(
        frame,
        level,
        chunk_bounds,
        0,
        0,
        0,
        10,
        7,
        15,
        false,
        random,
    );
    generate_small_door(frame, level, chunk_bounds, SmallDoorType::Grates, 4, 1, 0);
    selected_box(frame, level, chunk_bounds, 1, 6, 1, 1, 6, 14, false, random);
    selected_box(frame, level, chunk_bounds, 9, 6, 1, 9, 6, 14, false, random);
    selected_box(frame, level, chunk_bounds, 2, 6, 1, 8, 6, 2, false, random);
    selected_box(
        frame,
        level,
        chunk_bounds,
        2,
        6,
        14,
        8,
        6,
        14,
        false,
        random,
    );
    selected_box(frame, level, chunk_bounds, 1, 1, 1, 2, 1, 4, false, random);
    selected_box(frame, level, chunk_bounds, 8, 1, 1, 9, 1, 4, false, random);
    let lava = state("minecraft:lava");
    fill_box(frame, level, chunk_bounds, 1, 1, 1, 1, 1, 3, lava);
    fill_box(frame, level, chunk_bounds, 9, 1, 1, 9, 1, 3, lava);
    selected_box(frame, level, chunk_bounds, 3, 1, 8, 7, 1, 12, false, random);
    fill_box(frame, level, chunk_bounds, 4, 1, 9, 6, 1, 11, lava);
    let ns = bars(true, true, false, false);
    let we = bars(false, false, true, true);
    let mut z = 3;
    while z < 14 {
        fill_box(frame, level, chunk_bounds, 0, 3, z, 0, 4, z, ns);
        fill_box(frame, level, chunk_bounds, 10, 3, z, 10, 4, z, ns);
        z += 2;
    }
    let mut x = 2;
    while x < 9 {
        fill_box(frame, level, chunk_bounds, x, 3, 15, x, 4, 15, we);
        x += 2;
    }
    selected_box(frame, level, chunk_bounds, 4, 1, 5, 6, 1, 7, false, random);
    selected_box(frame, level, chunk_bounds, 4, 2, 6, 6, 2, 7, false, random);
    selected_box(frame, level, chunk_bounds, 4, 3, 7, 6, 3, 7, false, random);
    let stairs = with_prop(state("minecraft:stone_brick_stairs"), "facing", "north");
    for x in 4..=6 {
        put(frame, level, chunk_bounds, stairs, x, 1, 4);
        put(frame, level, chunk_bounds, stairs, x, 2, 5);
        put(frame, level, chunk_bounds, stairs, x, 3, 6);
    }
    let frame_block =
        |facing: &str| with_prop(state("minecraft:end_portal_frame"), "facing", facing);
    let north = frame_block("north");
    let south = frame_block("south");
    let east = frame_block("east");
    let west = frame_block("west");
    let mut all_eyes = true;
    let mut eyes = [false; 12];
    for eye in &mut eyes {
        *eye = random.next_float() > 0.9;
        all_eyes &= *eye;
    }
    let eyed = |base: i32, eye: bool| {
        if eye {
            with_prop(base, "eye", "true")
        } else {
            base
        }
    };
    put(frame, level, chunk_bounds, eyed(north, eyes[0]), 4, 3, 8);
    put(frame, level, chunk_bounds, eyed(north, eyes[1]), 5, 3, 8);
    put(frame, level, chunk_bounds, eyed(north, eyes[2]), 6, 3, 8);
    put(frame, level, chunk_bounds, eyed(south, eyes[3]), 4, 3, 12);
    put(frame, level, chunk_bounds, eyed(south, eyes[4]), 5, 3, 12);
    put(frame, level, chunk_bounds, eyed(south, eyes[5]), 6, 3, 12);
    put(frame, level, chunk_bounds, eyed(east, eyes[6]), 3, 3, 9);
    put(frame, level, chunk_bounds, eyed(east, eyes[7]), 3, 3, 10);
    put(frame, level, chunk_bounds, eyed(east, eyes[8]), 3, 3, 11);
    put(frame, level, chunk_bounds, eyed(west, eyes[9]), 7, 3, 9);
    put(frame, level, chunk_bounds, eyed(west, eyes[10]), 7, 3, 10);
    put(frame, level, chunk_bounds, eyed(west, eyes[11]), 7, 3, 11);
    if all_eyes {
        let portal = state("minecraft:end_portal");
        for z in 9..=11 {
            for x in 4..=6 {
                put(frame, level, chunk_bounds, portal, x, 3, z);
            }
        }
    }
    let StrongholdPieceExtra::PortalRoom { mob } = piece.extra() else {
        unreachable!("piece extra is not a portal room")
    };
    if !*mob {
        let pos = frame.world_pos(5, 3, 6);
        if chunk_bounds.contains(pos) {
            if let StrongholdPieceExtra::PortalRoom { mob } = piece.extra_mut() {
                *mob = true;
            }
            level.set_block(pos, state("minecraft:spawner"), UPDATE_CLIENTS);
            level.set_spawner_entity_id(pos, "minecraft:silverfish");
        }
    }
}

fn place_prison_hall<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 8, 4, 10, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 1, 1, 0);
    air_box(frame, level, chunk_bounds, 1, 1, 10, 3, 3, 10);
    selected_box(frame, level, chunk_bounds, 4, 1, 1, 4, 3, 1, false, random);
    selected_box(frame, level, chunk_bounds, 4, 1, 3, 4, 3, 3, false, random);
    selected_box(frame, level, chunk_bounds, 4, 1, 7, 4, 3, 7, false, random);
    selected_box(frame, level, chunk_bounds, 4, 1, 9, 4, 3, 9, false, random);
    let ns = bars(true, true, false, false);
    let nse = bars(true, true, false, true);
    let we = bars(false, false, true, true);
    for y in 1..=3 {
        put(frame, level, chunk_bounds, ns, 4, y, 4);
        put(frame, level, chunk_bounds, nse, 4, y, 5);
        put(frame, level, chunk_bounds, ns, 4, y, 6);
        put(frame, level, chunk_bounds, we, 5, y, 5);
        put(frame, level, chunk_bounds, we, 6, y, 5);
        put(frame, level, chunk_bounds, we, 7, y, 5);
    }
    put(frame, level, chunk_bounds, ns, 4, 3, 2);
    put(frame, level, chunk_bounds, ns, 4, 3, 8);
    let door = with_prop(state("minecraft:iron_door"), "facing", "west");
    let door_top = with_prop(door, "half", "upper");
    put(frame, level, chunk_bounds, door, 4, 1, 2);
    put(frame, level, chunk_bounds, door_top, 4, 2, 2);
    put(frame, level, chunk_bounds, door, 4, 1, 8);
    put(frame, level, chunk_bounds, door_top, 4, 2, 8)
}

fn place_room_crossing<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let StrongholdPieceExtra::RoomCrossing { room_type } = *piece.extra() else {
        unreachable!("piece extra is not a room crossing")
    };
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 10, 6, 10, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 4, 1, 0);
    air_box(frame, level, chunk_bounds, 4, 1, 10, 6, 3, 10);
    air_box(frame, level, chunk_bounds, 0, 1, 4, 0, 3, 6);
    air_box(frame, level, chunk_bounds, 10, 1, 4, 10, 3, 6);
    match room_type {
        0 => {
            let bricks = stone_bricks();
            put(frame, level, chunk_bounds, bricks, 5, 1, 5);
            put(frame, level, chunk_bounds, bricks, 5, 2, 5);
            put(frame, level, chunk_bounds, bricks, 5, 3, 5);
            put(frame, level, chunk_bounds, wall_torch("west"), 4, 3, 5);
            put(frame, level, chunk_bounds, wall_torch("east"), 6, 3, 5);
            put(frame, level, chunk_bounds, wall_torch("south"), 5, 3, 4);
            put(frame, level, chunk_bounds, wall_torch("north"), 5, 3, 6);
            let slab = state("minecraft:smooth_stone_slab");
            for (x, z) in [
                (4, 4),
                (4, 5),
                (4, 6),
                (6, 4),
                (6, 5),
                (6, 6),
                (5, 4),
                (5, 6),
            ] {
                put(frame, level, chunk_bounds, slab, x, 1, z);
            }
        }
        1 => {
            let bricks = stone_bricks();
            for i in 0..5 {
                put(frame, level, chunk_bounds, bricks, 3, 1, 3 + i);
                put(frame, level, chunk_bounds, bricks, 7, 1, 3 + i);
                put(frame, level, chunk_bounds, bricks, 3 + i, 1, 3);
                put(frame, level, chunk_bounds, bricks, 3 + i, 1, 7);
            }
            put(frame, level, chunk_bounds, bricks, 5, 1, 5);
            put(frame, level, chunk_bounds, bricks, 5, 2, 5);
            put(frame, level, chunk_bounds, bricks, 5, 3, 5);
            put(
                frame,
                level,
                chunk_bounds,
                state("minecraft:water"),
                5,
                4,
                5,
            );
        }
        2 => {
            let cobble = state("minecraft:cobblestone");
            for z in 1..=9 {
                put(frame, level, chunk_bounds, cobble, 1, 3, z);
                put(frame, level, chunk_bounds, cobble, 9, 3, z);
            }
            for x in 1..=9 {
                put(frame, level, chunk_bounds, cobble, x, 3, 1);
                put(frame, level, chunk_bounds, cobble, x, 3, 9);
            }
            put(frame, level, chunk_bounds, cobble, 5, 1, 4);
            put(frame, level, chunk_bounds, cobble, 5, 1, 6);
            put(frame, level, chunk_bounds, cobble, 5, 3, 4);
            put(frame, level, chunk_bounds, cobble, 5, 3, 6);
            put(frame, level, chunk_bounds, cobble, 4, 1, 5);
            put(frame, level, chunk_bounds, cobble, 6, 1, 5);
            put(frame, level, chunk_bounds, cobble, 4, 3, 5);
            put(frame, level, chunk_bounds, cobble, 6, 3, 5);
            for y in 1..=3 {
                put(frame, level, chunk_bounds, cobble, 4, y, 4);
                put(frame, level, chunk_bounds, cobble, 6, y, 4);
                put(frame, level, chunk_bounds, cobble, 4, y, 6);
                put(frame, level, chunk_bounds, cobble, 6, y, 6);
            }
            put(
                frame,
                level,
                chunk_bounds,
                state("minecraft:wall_torch"),
                5,
                3,
                5,
            );
            let planks = state("minecraft:oak_planks");
            for z in 2..=8 {
                put(frame, level, chunk_bounds, planks, 2, 3, z);
                put(frame, level, chunk_bounds, planks, 3, 3, z);
                if z <= 3 || z >= 7 {
                    put(frame, level, chunk_bounds, planks, 4, 3, z);
                    put(frame, level, chunk_bounds, planks, 5, 3, z);
                    put(frame, level, chunk_bounds, planks, 6, 3, z);
                }
                put(frame, level, chunk_bounds, planks, 7, 3, z);
                put(frame, level, chunk_bounds, planks, 8, 3, z);
            }
            let ladder = with_prop(state("minecraft:ladder"), "facing", "west");
            put(frame, level, chunk_bounds, ladder, 9, 1, 3);
            put(frame, level, chunk_bounds, ladder, 9, 2, 3);
            put(frame, level, chunk_bounds, ladder, 9, 3, 3);
            frame.create_chest(level, chunk_bounds, random, 3, 4, 8, CROSSING_LOOT);
        }
        _ => {}
    }
}

fn place_stairs_down<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 4, 10, 4, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 1, 7, 0);
    generate_small_door(frame, level, chunk_bounds, SmallDoorType::Opening, 1, 1, 4);
    let bricks = stone_bricks();
    let slab = state("minecraft:smooth_stone_slab");
    put(frame, level, chunk_bounds, bricks, 2, 6, 1);
    put(frame, level, chunk_bounds, bricks, 1, 5, 1);
    put(frame, level, chunk_bounds, slab, 1, 6, 1);
    put(frame, level, chunk_bounds, bricks, 1, 5, 2);
    put(frame, level, chunk_bounds, bricks, 1, 4, 3);
    put(frame, level, chunk_bounds, slab, 1, 5, 3);
    put(frame, level, chunk_bounds, bricks, 2, 4, 3);
    put(frame, level, chunk_bounds, bricks, 3, 3, 3);
    put(frame, level, chunk_bounds, slab, 3, 4, 3);
    put(frame, level, chunk_bounds, bricks, 3, 3, 2);
    put(frame, level, chunk_bounds, bricks, 3, 2, 1);
    put(frame, level, chunk_bounds, slab, 3, 3, 1);
    put(frame, level, chunk_bounds, bricks, 2, 2, 1);
    put(frame, level, chunk_bounds, bricks, 1, 1, 1);
    put(frame, level, chunk_bounds, slab, 1, 2, 1);
    put(frame, level, chunk_bounds, bricks, 1, 1, 2);
    put(frame, level, chunk_bounds, slab, 1, 1, 3)
}

fn place_straight<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let StrongholdPieceExtra::Straight { left, right } = *piece.extra() else {
        unreachable!("piece extra is not a straight corridor")
    };
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 4, 4, 6, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 1, 1, 0);
    generate_small_door(frame, level, chunk_bounds, SmallDoorType::Opening, 1, 1, 6);
    let east = wall_torch("east");
    let west = wall_torch("west");
    frame.maybe_generate_block(level, chunk_bounds, random, 0.1, 1, 2, 1, east);
    frame.maybe_generate_block(level, chunk_bounds, random, 0.1, 3, 2, 1, west);
    frame.maybe_generate_block(level, chunk_bounds, random, 0.1, 1, 2, 5, east);
    frame.maybe_generate_block(level, chunk_bounds, random, 0.1, 3, 2, 5, west);
    if left {
        air_box(frame, level, chunk_bounds, 0, 1, 2, 0, 3, 4);
    }
    if right {
        air_box(frame, level, chunk_bounds, 4, 1, 2, 4, 3, 4);
    }
}

fn place_straight_stairs_down<L: WorldGenLevel>(
    piece: &StrongholdPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = frame(piece);
    selected_box(frame, level, chunk_bounds, 0, 0, 0, 4, 10, 7, true, random);
    generate_small_door(frame, level, chunk_bounds, piece.entry_door(), 1, 7, 0);
    generate_small_door(frame, level, chunk_bounds, SmallDoorType::Opening, 1, 1, 7);
    let stairs = with_prop(state("minecraft:cobblestone_stairs"), "facing", "south");
    let bricks = stone_bricks();
    for i in 0..6 {
        put(frame, level, chunk_bounds, stairs, 1, 6 - i, 1 + i);
        put(frame, level, chunk_bounds, stairs, 2, 6 - i, 1 + i);
        put(frame, level, chunk_bounds, stairs, 3, 6 - i, 1 + i);
        if i < 5 {
            put(frame, level, chunk_bounds, bricks, 1, 5 - i, 1 + i);
            put(frame, level, chunk_bounds, bricks, 2, 5 - i, 1 + i);
            put(frame, level, chunk_bounds, bricks, 3, 5 - i, 1 + i);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::random::WorldgenRandom;
    use bbb_protocol::packets::BlockPos;

    use super::super::STRONGHOLD_ID;
    use super::*;
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
            -64
        }

        fn get_max_y(&self) -> i32 {
            319
        }

        fn get_sea_level(&self) -> i32 {
            63
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
                y: -64,
                z: -512,
            },
            max: BlockPos {
                x: 512,
                y: 319,
                z: 512,
            },
        }
    }

    fn box_at(min: [i32; 3], max: [i32; 3]) -> [i32; 6] {
        [min[0], min[1], min[2], max[0], max[1], max[2]]
    }

    fn start_with(
        kind: StrongholdPieceKind,
        extra: StrongholdPieceExtra,
        bounds: [i32; 6],
    ) -> StrongholdStructureStart {
        let start = StrongholdPiece::new(
            StrongholdPieceKind::Start,
            if kind == StrongholdPieceKind::Start {
                bounds
            } else {
                box_at([0, 0, 0], [4, 10, 4])
            },
            HorizontalDirection::South,
            0,
            SmallDoorType::Opening,
            StrongholdPieceExtra::StairsDown { source: true },
        );
        let portal = StrongholdPiece::new(
            StrongholdPieceKind::PortalRoom,
            if kind == StrongholdPieceKind::PortalRoom {
                bounds
            } else {
                box_at([32, 0, 32], [42, 7, 47])
            },
            HorizontalDirection::South,
            1,
            SmallDoorType::Opening,
            StrongholdPieceExtra::PortalRoom { mob: false },
        );
        let extra_piece = StrongholdPiece::new(
            kind,
            bounds,
            HorizontalDirection::South,
            1,
            SmallDoorType::Opening,
            extra,
        );
        let pieces = if kind == StrongholdPieceKind::Start {
            vec![start, portal]
        } else if kind == StrongholdPieceKind::PortalRoom {
            vec![start, extra_piece]
        } else {
            vec![start, extra_piece, portal]
        };
        StrongholdStructureStart::from_generation(STRONGHOLD_ID, [0, 0], 0, pieces)
    }

    #[test]
    fn start_stairs_write_stone_bricks_inside_the_writable_area() {
        let mut start = start_with(
            StrongholdPieceKind::Start,
            StrongholdPieceExtra::StairsDown { source: true },
            box_at([0, 0, 0], [4, 10, 4]),
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        let report = start.place_in_chunk(&mut level, wide_bounds(), &mut random);
        assert!(report.attempted);
        assert!(report.placed);
        assert!(level
            .cells
            .values()
            .any(|state| block_is(*state, "minecraft:stone_bricks")
                || block_is(*state, "minecraft:cracked_stone_bricks")
                || block_is(*state, "minecraft:mossy_stone_bricks")
                || block_is(*state, "minecraft:infested_stone_bricks")));
    }

    #[test]
    fn place_in_chunk_skips_pieces_that_miss_the_writable_area() {
        let mut start = start_with(
            StrongholdPieceKind::Start,
            StrongholdPieceExtra::StairsDown { source: true },
            box_at([0, 0, 0], [4, 10, 4]),
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        let far = JigsawBoundingBox {
            min: BlockPos {
                x: 256,
                y: -63,
                z: 256,
            },
            max: BlockPos {
                x: 271,
                y: 319,
                z: 271,
            },
        };
        let report = start.place_in_chunk(&mut level, far, &mut random);
        assert_eq!(report, StrongholdChunkPlacement::default());
        assert!(level.cells.is_empty());
    }

    #[test]
    fn chest_corridor_records_loot_when_the_chest_cell_is_writable() {
        let mut start = start_with(
            StrongholdPieceKind::ChestCorridor,
            StrongholdPieceExtra::ChestCorridor { chest: false },
            box_at([0, 0, 0], [4, 4, 6]),
        );
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        start.place_in_chunk(&mut level, wide_bounds(), &mut random);
        assert!(start.pieces().iter().any(|piece| {
            matches!(
                piece.extra(),
                StrongholdPieceExtra::ChestCorridor { chest: true }
            )
        }));
        assert!(level
            .loot
            .iter()
            .any(|(_, table, _)| *table == CORRIDOR_LOOT));
    }
}
