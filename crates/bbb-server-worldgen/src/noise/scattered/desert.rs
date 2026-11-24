//! Desert-pyramid `postProcess` plus `afterPlace`.

use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::vanilla_block_state_rotate_y;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::{
    update_height_position_to_lowest_ground_height, GeneratedPieceFrame,
};
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_set_property, catalog_default_state, WorldGenLevel};

use super::{ScatteredFeatureChunkPlacement, ScatteredFeatureExtra, ScatteredFeaturePiece};

const CORRIDOR_LOOT: &str = "minecraft:chests/desert_pyramid";
const ARCHAEOLOGY_LOOT: &str = "minecraft:archaeology/desert_pyramid";
const UPDATE_CLIENTS: i32 = 2;

/// Vanilla `Direction.Plane.HORIZONTAL` (`Direction.java:577`) as
/// `(step_x, step_z, 2d)`.
const HORIZONTAL: [(i32, i32, usize); 4] = [(0, -1, 2), (1, 0, 3), (0, 1, 0), (-1, 0, 1)];

pub fn place_desert_pyramid<L: WorldGenLevel>(
    piece: &mut ScatteredFeaturePiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> ScatteredFeatureChunkPlacement {
    let mut bounding_box = piece.bounding_box();
    let mut height_position = piece.height_position();
    let offset = -random.next_int(3);
    if !update_height_position_to_lowest_ground_height(
        level,
        &mut bounding_box,
        &mut height_position,
        offset,
    ) {
        return ScatteredFeatureChunkPlacement {
            attempted: true,
            placed: false,
        };
    }
    *piece.bounding_box_mut() = bounding_box;
    *piece.height_position_mut() = height_position;
    let frame = GeneratedPieceFrame::from_array(piece.bounding_box(), piece.orientation());
    let width = piece.width();
    let depth = piece.depth();
    let sandstone = state("minecraft:sandstone");
    let cut = state("minecraft:cut_sandstone");
    let chiseled = state("minecraft:chiseled_sandstone");
    let air = state("minecraft:air");
    let orange = state("minecraft:orange_terracotta");
    let blue = state("minecraft:blue_terracotta");
    let slab = state("minecraft:sandstone_slab");
    let sand = state("minecraft:sand");
    let tnt = state("minecraft:tnt");
    let plate = state("minecraft:stone_pressure_plate");
    let north = stair("north");
    let south = stair("south");
    let east = stair("east");
    let west = stair("west");
    generate_box(
        frame,
        level,
        chunk_bounds,
        0,
        -4,
        0,
        width - 1,
        0,
        depth - 1,
        sandstone,
        sandstone,
        false,
    );
    for pos in 1..=9 {
        generate_box(
            frame,
            level,
            chunk_bounds,
            pos,
            pos,
            pos,
            width - 1 - pos,
            pos,
            depth - 1 - pos,
            sandstone,
            sandstone,
            false,
        );
        generate_box(
            frame,
            level,
            chunk_bounds,
            pos + 1,
            pos,
            pos + 1,
            width - 2 - pos,
            pos,
            depth - 2 - pos,
            air,
            air,
            false,
        );
    }
    for x in 0..width {
        for z in 0..depth {
            frame.fill_column_down(level, sandstone, x, -5, z, chunk_bounds);
        }
    }
    generate_box(
        frame,
        level,
        chunk_bounds,
        0,
        0,
        0,
        4,
        9,
        4,
        sandstone,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        1,
        10,
        1,
        3,
        10,
        3,
        sandstone,
        sandstone,
        false,
    );
    place(frame, level, chunk_bounds, north, 2, 10, 0);
    place(frame, level, chunk_bounds, south, 2, 10, 4);
    place(frame, level, chunk_bounds, east, 0, 10, 2);
    place(frame, level, chunk_bounds, west, 4, 10, 2);
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 5,
        0,
        0,
        width - 1,
        9,
        4,
        sandstone,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 4,
        10,
        1,
        width - 2,
        10,
        3,
        sandstone,
        sandstone,
        false,
    );
    place(frame, level, chunk_bounds, north, width - 3, 10, 0);
    place(frame, level, chunk_bounds, south, width - 3, 10, 4);
    place(frame, level, chunk_bounds, east, width - 5, 10, 2);
    place(frame, level, chunk_bounds, west, width - 1, 10, 2);
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        0,
        0,
        12,
        4,
        4,
        sandstone,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        9,
        1,
        0,
        11,
        3,
        4,
        air,
        air,
        false,
    );
    for (x, y, z) in [
        (9, 1, 1),
        (9, 2, 1),
        (9, 3, 1),
        (10, 3, 1),
        (11, 3, 1),
        (11, 2, 1),
        (11, 1, 1),
    ] {
        place(frame, level, chunk_bounds, cut, x, y, z);
    }
    generate_box(
        frame,
        level,
        chunk_bounds,
        4,
        1,
        1,
        8,
        3,
        3,
        sandstone,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        4,
        1,
        2,
        8,
        2,
        2,
        air,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        12,
        1,
        1,
        16,
        3,
        3,
        sandstone,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        12,
        1,
        2,
        16,
        2,
        2,
        air,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        5,
        4,
        5,
        width - 6,
        4,
        depth - 6,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        9,
        4,
        9,
        11,
        4,
        11,
        air,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        1,
        8,
        8,
        3,
        8,
        cut,
        cut,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        12,
        1,
        8,
        12,
        3,
        8,
        cut,
        cut,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        1,
        12,
        8,
        3,
        12,
        cut,
        cut,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        12,
        1,
        12,
        12,
        3,
        12,
        cut,
        cut,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        1,
        1,
        5,
        4,
        4,
        11,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 5,
        1,
        5,
        width - 2,
        4,
        11,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        6,
        7,
        9,
        6,
        7,
        11,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 7,
        7,
        9,
        width - 7,
        7,
        11,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        5,
        5,
        9,
        5,
        7,
        11,
        cut,
        cut,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 6,
        5,
        9,
        width - 6,
        7,
        11,
        cut,
        cut,
        false,
    );
    place(frame, level, chunk_bounds, air, 5, 5, 10);
    place(frame, level, chunk_bounds, air, 5, 6, 10);
    place(frame, level, chunk_bounds, air, 6, 6, 10);
    place(frame, level, chunk_bounds, air, width - 6, 5, 10);
    place(frame, level, chunk_bounds, air, width - 6, 6, 10);
    place(frame, level, chunk_bounds, air, width - 7, 6, 10);
    generate_box(
        frame,
        level,
        chunk_bounds,
        2,
        4,
        4,
        2,
        6,
        4,
        air,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 3,
        4,
        4,
        width - 3,
        6,
        4,
        air,
        air,
        false,
    );
    place(frame, level, chunk_bounds, north, 2, 4, 5);
    place(frame, level, chunk_bounds, north, 2, 3, 4);
    place(frame, level, chunk_bounds, north, width - 3, 4, 5);
    place(frame, level, chunk_bounds, north, width - 3, 3, 4);
    generate_box(
        frame,
        level,
        chunk_bounds,
        1,
        1,
        3,
        2,
        2,
        3,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 3,
        1,
        3,
        width - 2,
        2,
        3,
        sandstone,
        sandstone,
        false,
    );
    place(frame, level, chunk_bounds, sandstone, 1, 1, 2);
    place(frame, level, chunk_bounds, sandstone, width - 2, 1, 2);
    place(frame, level, chunk_bounds, slab, 1, 2, 2);
    place(frame, level, chunk_bounds, slab, width - 2, 2, 2);
    place(frame, level, chunk_bounds, west, 2, 1, 2);
    place(frame, level, chunk_bounds, east, width - 3, 1, 2);
    generate_box(
        frame,
        level,
        chunk_bounds,
        4,
        3,
        5,
        4,
        3,
        17,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 5,
        3,
        5,
        width - 5,
        3,
        17,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        3,
        1,
        5,
        4,
        2,
        16,
        air,
        air,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        width - 6,
        1,
        5,
        width - 5,
        2,
        16,
        air,
        air,
        false,
    );
    let mut z = 5;
    while z <= 17 {
        place(frame, level, chunk_bounds, cut, 4, 1, z);
        place(frame, level, chunk_bounds, chiseled, 4, 2, z);
        place(frame, level, chunk_bounds, cut, width - 5, 1, z);
        place(frame, level, chunk_bounds, chiseled, width - 5, 2, z);
        z += 2;
    }
    for (x, y, z) in [
        (10, 0, 7),
        (10, 0, 8),
        (9, 0, 9),
        (11, 0, 9),
        (8, 0, 10),
        (12, 0, 10),
        (7, 0, 10),
        (13, 0, 10),
        (9, 0, 11),
        (11, 0, 11),
        (10, 0, 12),
        (10, 0, 13),
    ] {
        place(frame, level, chunk_bounds, orange, x, y, z);
    }
    place(frame, level, chunk_bounds, blue, 10, 0, 10);
    let mut x = 0;
    while x < width {
        place_side_pattern(frame, level, chunk_bounds, x, cut, orange, chiseled);
        if width == 1 {
            break;
        }
        x += width - 1;
    }
    let mut x = 2;
    let front_step = width - 5;
    while x <= width - 3 {
        place_front_pattern(frame, level, chunk_bounds, x, cut, orange, chiseled);
        if front_step <= 0 {
            break;
        }
        x += front_step;
    }
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        4,
        0,
        12,
        6,
        0,
        cut,
        cut,
        false,
    );
    place(frame, level, chunk_bounds, air, 8, 6, 0);
    place(frame, level, chunk_bounds, air, 12, 6, 0);
    place(frame, level, chunk_bounds, orange, 9, 5, 0);
    place(frame, level, chunk_bounds, chiseled, 10, 5, 0);
    place(frame, level, chunk_bounds, orange, 11, 5, 0);
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        -14,
        8,
        12,
        -11,
        12,
        cut,
        cut,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        -10,
        8,
        12,
        -10,
        12,
        chiseled,
        chiseled,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        -9,
        8,
        12,
        -9,
        12,
        cut,
        cut,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        8,
        -8,
        8,
        12,
        -1,
        12,
        sandstone,
        sandstone,
        false,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        9,
        -11,
        9,
        11,
        -1,
        11,
        air,
        air,
        false,
    );
    place(frame, level, chunk_bounds, plate, 10, -11, 10);
    generate_box(
        frame,
        level,
        chunk_bounds,
        9,
        -13,
        9,
        11,
        -13,
        11,
        tnt,
        air,
        false,
    );
    place(frame, level, chunk_bounds, air, 8, -11, 10);
    place(frame, level, chunk_bounds, air, 8, -10, 10);
    place(frame, level, chunk_bounds, chiseled, 7, -10, 10);
    place(frame, level, chunk_bounds, cut, 7, -11, 10);
    place(frame, level, chunk_bounds, air, 12, -11, 10);
    place(frame, level, chunk_bounds, air, 12, -10, 10);
    place(frame, level, chunk_bounds, chiseled, 13, -10, 10);
    place(frame, level, chunk_bounds, cut, 13, -11, 10);
    place(frame, level, chunk_bounds, air, 10, -11, 8);
    place(frame, level, chunk_bounds, air, 10, -10, 8);
    place(frame, level, chunk_bounds, chiseled, 10, -10, 7);
    place(frame, level, chunk_bounds, cut, 10, -11, 7);
    place(frame, level, chunk_bounds, air, 10, -11, 12);
    place(frame, level, chunk_bounds, air, 10, -10, 12);
    place(frame, level, chunk_bounds, chiseled, 10, -10, 13);
    place(frame, level, chunk_bounds, cut, 10, -11, 13);
    let ScatteredFeatureExtra::DesertPyramid {
        has_placed_chest0,
        has_placed_chest1,
        has_placed_chest2,
        has_placed_chest3,
    } = piece.extra_mut()
    else {
        unreachable!("piece extra is not a desert pyramid")
    };
    let mut chests = [
        *has_placed_chest0,
        *has_placed_chest1,
        *has_placed_chest2,
        *has_placed_chest3,
    ];
    for (step_x, step_z, index) in HORIZONTAL {
        if !chests[index] {
            chests[index] = frame.create_chest(
                level,
                chunk_bounds,
                random,
                10 + step_x * 2,
                -11,
                10 + step_z * 2,
                CORRIDOR_LOOT,
            );
        }
    }
    let ScatteredFeatureExtra::DesertPyramid {
        has_placed_chest0,
        has_placed_chest1,
        has_placed_chest2,
        has_placed_chest3,
    } = piece.extra_mut()
    else {
        unreachable!("piece extra is not a desert pyramid")
    };
    *has_placed_chest0 = chests[0];
    *has_placed_chest1 = chests[1];
    *has_placed_chest2 = chests[2];
    *has_placed_chest3 = chests[3];
    let world_seed = level.get_seed();
    let mut level_random = WorldgenRandom::named(world_seed, "minecraft:desert_pyramid");
    let mut sand_positions = Vec::new();
    let roof_pos = add_cellar(
        frame,
        level,
        chunk_bounds,
        &mut level_random,
        &mut sand_positions,
        sandstone,
        cut,
        chiseled,
        sand,
        orange,
        blue,
    );
    after_place(
        frame,
        level,
        chunk_bounds,
        world_seed,
        &sand_positions,
        roof_pos,
        sand,
    );
    ScatteredFeatureChunkPlacement {
        attempted: true,
        placed: true,
    }
}

#[allow(clippy::too_many_arguments)]
fn add_cellar<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    level_random: &mut WorldgenRandom,
    sand_positions: &mut Vec<BlockPos>,
    sandstone: i32,
    cut: i32,
    chiseled: i32,
    sand: i32,
    orange: i32,
    blue: i32,
) -> BlockPos {
    let room = BlockPos {
        x: 16,
        y: -4,
        z: 13,
    };
    let stairs = state("minecraft:sandstone_stairs");
    let ccw = vanilla_block_state_rotate_y(stairs, 3).unwrap_or(stairs);
    place(frame, level, chunk_bounds, ccw, 13, -1, 17);
    place(frame, level, chunk_bounds, ccw, 14, -2, 17);
    place(frame, level, chunk_bounds, ccw, 15, -3, 17);
    let x = room.x;
    let y = room.y;
    let z = room.z;
    place(frame, level, chunk_bounds, sand, x - 4, y + 4, z + 4);
    place(frame, level, chunk_bounds, sand, x - 3, y + 4, z + 4);
    place(frame, level, chunk_bounds, sand, x - 2, y + 4, z + 4);
    place(frame, level, chunk_bounds, sand, x - 1, y + 4, z + 4);
    place(frame, level, chunk_bounds, sand, x, y + 4, z + 4);
    place(frame, level, chunk_bounds, sand, x - 2, y + 3, z + 4);
    let variant = level_random.next_boolean();
    place(
        frame,
        level,
        chunk_bounds,
        if variant { sand } else { sandstone },
        x - 1,
        y + 3,
        z + 4,
    );
    place(
        frame,
        level,
        chunk_bounds,
        if !variant { sand } else { sandstone },
        x,
        y + 3,
        z + 4,
    );
    place(frame, level, chunk_bounds, sand, x - 1, y + 2, z + 4);
    place(frame, level, chunk_bounds, sandstone, x, y + 2, z + 4);
    place(frame, level, chunk_bounds, sand, x, y + 1, z + 4);
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        y + 1,
        z - 3,
        x - 3,
        y + 1,
        z + 2,
        cut,
        cut,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x + 3,
        y + 1,
        z - 3,
        x + 3,
        y + 1,
        z + 2,
        cut,
        cut,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        y + 1,
        z - 3,
        x + 3,
        y + 1,
        z - 2,
        cut,
        cut,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        y + 1,
        z + 3,
        x + 3,
        y + 1,
        z + 3,
        cut,
        cut,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        y + 2,
        z - 3,
        x - 3,
        y + 2,
        z + 2,
        chiseled,
        chiseled,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x + 3,
        y + 2,
        z - 3,
        x + 3,
        y + 2,
        z + 2,
        chiseled,
        chiseled,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        y + 2,
        z - 3,
        x + 3,
        y + 2,
        z - 2,
        chiseled,
        chiseled,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        y + 2,
        z + 3,
        x + 3,
        y + 2,
        z + 3,
        chiseled,
        chiseled,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        -1,
        z - 3,
        x - 3,
        -1,
        z + 2,
        cut,
        cut,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x + 3,
        -1,
        z - 3,
        x + 3,
        -1,
        z + 2,
        cut,
        cut,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        -1,
        z - 3,
        x + 3,
        -1,
        z - 2,
        cut,
        cut,
        true,
    );
    generate_box(
        frame,
        level,
        chunk_bounds,
        x - 3,
        -1,
        z + 3,
        x + 3,
        -1,
        z + 3,
        cut,
        cut,
        true,
    );
    for py in (y + 1)..=(y + 3) {
        for px in (x - 2)..=(x + 2) {
            for pz in (z - 2)..=(z + 2) {
                sand_positions.push(frame.world_pos(px, py, pz));
            }
        }
    }
    let roof_pos = place_collapsed_roof(
        frame,
        level,
        chunk_bounds,
        level_random,
        x - 2,
        y + 4,
        z - 2,
        x + 2,
        z + 2,
        sandstone,
        sand,
    );
    place(frame, level, chunk_bounds, blue, x, y, z);
    place(frame, level, chunk_bounds, orange, x + 1, y, z - 1);
    place(frame, level, chunk_bounds, orange, x + 1, y, z + 1);
    place(frame, level, chunk_bounds, orange, x - 1, y, z - 1);
    place(frame, level, chunk_bounds, orange, x - 1, y, z + 1);
    place(frame, level, chunk_bounds, orange, x + 2, y, z);
    place(frame, level, chunk_bounds, orange, x - 2, y, z);
    place(frame, level, chunk_bounds, orange, x, y, z + 2);
    place(frame, level, chunk_bounds, orange, x, y, z - 2);
    place(frame, level, chunk_bounds, orange, x + 3, y, z);
    sand_positions.push(frame.world_pos(x + 3, y + 1, z));
    sand_positions.push(frame.world_pos(x + 3, y + 2, z));
    place(frame, level, chunk_bounds, cut, x + 4, y + 1, z);
    place(frame, level, chunk_bounds, chiseled, x + 4, y + 2, z);
    place(frame, level, chunk_bounds, orange, x - 3, y, z);
    sand_positions.push(frame.world_pos(x - 3, y + 1, z));
    sand_positions.push(frame.world_pos(x - 3, y + 2, z));
    place(frame, level, chunk_bounds, cut, x - 4, y + 1, z);
    place(frame, level, chunk_bounds, chiseled, x - 4, y + 2, z);
    place(frame, level, chunk_bounds, orange, x, y, z + 3);
    sand_positions.push(frame.world_pos(x, y + 1, z + 3));
    sand_positions.push(frame.world_pos(x, y + 2, z + 3));
    place(frame, level, chunk_bounds, orange, x, y, z - 3);
    sand_positions.push(frame.world_pos(x, y + 1, z - 3));
    sand_positions.push(frame.world_pos(x, y + 2, z - 3));
    place(frame, level, chunk_bounds, cut, x, y + 1, z - 4);
    place(frame, level, chunk_bounds, chiseled, x, -2, z - 4);
    roof_pos
}

#[allow(clippy::too_many_arguments)]
fn place_collapsed_roof<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    level_random: &mut WorldgenRandom,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
    sandstone: i32,
    sand: i32,
) -> BlockPos {
    for x in x0..=x1 {
        for z in z0..=z1 {
            let state = if level_random.next_float() < 0.33 {
                sandstone
            } else {
                sand
            };
            place(frame, level, chunk_bounds, state, x, y0, z);
        }
    }
    let world_seed = level.get_seed();
    let origin = frame.world_pos(x0, y0, z0);
    let mut roof_random = WorldgenRandom::at(world_seed, origin.x, origin.y, origin.z);
    let roof_x = x0 + roof_random.next_int(x1 - x0 + 1);
    let roof_z = z0 + roof_random.next_int(z1 - z0 + 1);
    frame.world_pos(roof_x, y0, roof_z)
}

fn after_place<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    world_seed: i64,
    sand_positions: &[BlockPos],
    roof_pos: BlockPos,
    sand: i32,
) {
    let unique = sand_positions
        .iter()
        .copied()
        .map(|pos| (pos.x, pos.y, pos.z))
        .collect::<std::collections::BTreeSet<_>>();
    let suspicious = catalog_default_state("minecraft:suspicious_sand");
    if chunk_bounds.contains(roof_pos) {
        place_suspicious_sand(level, roof_pos, suspicious);
    }
    let mut shuffled = unique.iter().copied().collect::<Vec<_>>();
    let center = BlockPos {
        x: (frame.bounding_box.min.x + frame.bounding_box.max.x) / 2,
        y: (frame.bounding_box.min.y + frame.bounding_box.max.y) / 2,
        z: (frame.bounding_box.min.z + frame.bounding_box.max.z) / 2,
    };
    let mut positional = WorldgenRandom::at(world_seed, center.x, center.y, center.z);
    shuffle(&mut shuffled, &mut positional);
    let suspicious_count = unique.len().min((5 + positional.next_int(3)) as usize);
    for (index, (x, y, z)) in shuffled.into_iter().enumerate() {
        let pos = BlockPos { x, y, z };
        if !chunk_bounds.contains(pos) {
            continue;
        }
        if index < suspicious_count {
            place_suspicious_sand(level, pos, suspicious);
        } else {
            level.set_block(pos, sand, UPDATE_CLIENTS);
        }
    }
}

fn place_suspicious_sand<L: WorldGenLevel>(level: &mut L, pos: BlockPos, suspicious: i32) {
    level.set_block(pos, suspicious, UPDATE_CLIENTS);
    level.set_brushable_block_loot_table(pos, ARCHAEOLOGY_LOOT, block_pos_as_long(pos));
}

fn place_side_pattern<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x: i32,
    cut: i32,
    orange: i32,
    chiseled: i32,
) {
    place(frame, level, chunk_bounds, cut, x, 2, 1);
    place(frame, level, chunk_bounds, orange, x, 2, 2);
    place(frame, level, chunk_bounds, cut, x, 2, 3);
    place(frame, level, chunk_bounds, cut, x, 3, 1);
    place(frame, level, chunk_bounds, orange, x, 3, 2);
    place(frame, level, chunk_bounds, cut, x, 3, 3);
    place(frame, level, chunk_bounds, orange, x, 4, 1);
    place(frame, level, chunk_bounds, chiseled, x, 4, 2);
    place(frame, level, chunk_bounds, orange, x, 4, 3);
    place(frame, level, chunk_bounds, cut, x, 5, 1);
    place(frame, level, chunk_bounds, orange, x, 5, 2);
    place(frame, level, chunk_bounds, cut, x, 5, 3);
    place(frame, level, chunk_bounds, orange, x, 6, 1);
    place(frame, level, chunk_bounds, chiseled, x, 6, 2);
    place(frame, level, chunk_bounds, orange, x, 6, 3);
    place(frame, level, chunk_bounds, orange, x, 7, 1);
    place(frame, level, chunk_bounds, orange, x, 7, 2);
    place(frame, level, chunk_bounds, orange, x, 7, 3);
    place(frame, level, chunk_bounds, cut, x, 8, 1);
    place(frame, level, chunk_bounds, cut, x, 8, 2);
    place(frame, level, chunk_bounds, cut, x, 8, 3);
}

fn place_front_pattern<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x: i32,
    cut: i32,
    orange: i32,
    chiseled: i32,
) {
    place(frame, level, chunk_bounds, cut, x - 1, 2, 0);
    place(frame, level, chunk_bounds, orange, x, 2, 0);
    place(frame, level, chunk_bounds, cut, x + 1, 2, 0);
    place(frame, level, chunk_bounds, cut, x - 1, 3, 0);
    place(frame, level, chunk_bounds, orange, x, 3, 0);
    place(frame, level, chunk_bounds, cut, x + 1, 3, 0);
    place(frame, level, chunk_bounds, orange, x - 1, 4, 0);
    place(frame, level, chunk_bounds, chiseled, x, 4, 0);
    place(frame, level, chunk_bounds, orange, x + 1, 4, 0);
    place(frame, level, chunk_bounds, cut, x - 1, 5, 0);
    place(frame, level, chunk_bounds, orange, x, 5, 0);
    place(frame, level, chunk_bounds, cut, x + 1, 5, 0);
    place(frame, level, chunk_bounds, orange, x - 1, 6, 0);
    place(frame, level, chunk_bounds, chiseled, x, 6, 0);
    place(frame, level, chunk_bounds, orange, x + 1, 6, 0);
    place(frame, level, chunk_bounds, orange, x - 1, 7, 0);
    place(frame, level, chunk_bounds, orange, x, 7, 0);
    place(frame, level, chunk_bounds, orange, x + 1, 7, 0);
    place(frame, level, chunk_bounds, cut, x - 1, 8, 0);
    place(frame, level, chunk_bounds, cut, x, 8, 0);
    place(frame, level, chunk_bounds, cut, x + 1, 8, 0);
}

#[allow(clippy::too_many_arguments)]
fn generate_box<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
    edge: i32,
    fill: i32,
    skip_air: bool,
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
        edge,
        fill,
        skip_air,
    )
}

fn place<L: WorldGenLevel>(
    frame: GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    state: i32,
    x: i32,
    y: i32,
    z: i32,
) {
    frame.place_block(level, chunk_bounds, state, x, y, z)
}

fn stair(facing: &'static str) -> i32 {
    block_set_property(state("minecraft:sandstone_stairs"), "facing", facing)
}

fn state(id: &'static str) -> i32 {
    catalog_default_state(id)
}

fn shuffle<T>(values: &mut [T], random: &mut dyn RandomSource) {
    for i in (1..values.len()).rev() {
        let j = random.next_int(i as i32 + 1) as usize;
        values.swap(i, j);
    }
}

/// `BlockPos.asLong` (`BlockPos.java:111-116`).
fn block_pos_as_long(pos: BlockPos) -> i64 {
    const HORIZONTAL: i32 = 26;
    const Y_LEN: i32 = 64 - 2 * HORIZONTAL;
    const X_OFF: i32 = Y_LEN + HORIZONTAL;
    const Z_OFF: i32 = Y_LEN;
    let x = i64::from(pos.x) & ((1_i64 << HORIZONTAL) - 1);
    let y = i64::from(pos.y) & ((1_i64 << Y_LEN) - 1);
    let z = i64::from(pos.z) & ((1_i64 << HORIZONTAL) - 1);
    (x << X_OFF) | y | (z << Z_OFF)
}
