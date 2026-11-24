use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;

use super::{box_same, chunk_intersects, generate_water_box, state};

pub fn place<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    generate_entrance_archs(frame, level, chunk_bounds);
    generate_entrance_wall(frame, level, chunk_bounds)
}

/// `MonumentBuilding.generateEntranceArchs` (`OceanMonumentPieces.java:445-463`).
fn generate_entrance_archs<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    if !chunk_intersects(frame, chunk_bounds, 22, 5, 35, 17) {
        return;
    }
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let lamp = state("minecraft:sea_lantern");
    generate_water_box(frame, level, chunk_bounds, 25, 0, 0, 32, 8, 20);
    for i in 0..4 {
        let z = 5 + i * 4;
        box_same(frame, level, chunk_bounds, 24, 2, z, 24, 4, z, light);
        box_same(frame, level, chunk_bounds, 22, 4, z, 23, 4, z, light);
        frame.place_block(level, chunk_bounds, light, 25, 5, z);
        frame.place_block(level, chunk_bounds, light, 26, 6, z);
        frame.place_block(level, chunk_bounds, lamp, 26, 5, z);
        box_same(frame, level, chunk_bounds, 33, 2, z, 33, 4, z, light);
        box_same(frame, level, chunk_bounds, 34, 4, z, 35, 4, z, light);
        frame.place_block(level, chunk_bounds, light, 32, 5, z);
        frame.place_block(level, chunk_bounds, light, 31, 6, z);
        frame.place_block(level, chunk_bounds, lamp, 31, 5, z);
        box_same(frame, level, chunk_bounds, 27, 6, z, 30, 6, z, gray);
    }
}

/// `MonumentBuilding.generateEntranceWall` (`OceanMonumentPieces.java:465-522`).
fn generate_entrance_wall<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    if !chunk_intersects(frame, chunk_bounds, 15, 20, 42, 21) {
        return;
    }
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let black = state("minecraft:dark_prismarine");
    box_same(frame, level, chunk_bounds, 15, 0, 21, 42, 0, 21, gray);
    generate_water_box(frame, level, chunk_bounds, 26, 1, 21, 31, 3, 21);
    box_same(frame, level, chunk_bounds, 21, 12, 21, 36, 12, 21, gray);
    box_same(frame, level, chunk_bounds, 17, 11, 21, 40, 11, 21, gray);
    box_same(frame, level, chunk_bounds, 16, 10, 21, 41, 10, 21, gray);
    box_same(frame, level, chunk_bounds, 15, 7, 21, 42, 9, 21, gray);
    box_same(frame, level, chunk_bounds, 16, 6, 21, 41, 6, 21, gray);
    box_same(frame, level, chunk_bounds, 17, 5, 21, 40, 5, 21, gray);
    box_same(frame, level, chunk_bounds, 21, 4, 21, 36, 4, 21, gray);
    box_same(frame, level, chunk_bounds, 22, 3, 21, 26, 3, 21, gray);
    box_same(frame, level, chunk_bounds, 31, 3, 21, 35, 3, 21, gray);
    box_same(frame, level, chunk_bounds, 23, 2, 21, 25, 2, 21, gray);
    box_same(frame, level, chunk_bounds, 32, 2, 21, 34, 2, 21, gray);
    box_same(frame, level, chunk_bounds, 28, 4, 20, 29, 4, 21, light);
    frame.place_block(level, chunk_bounds, light, 27, 3, 21);
    frame.place_block(level, chunk_bounds, light, 30, 3, 21);
    frame.place_block(level, chunk_bounds, light, 26, 2, 21);
    frame.place_block(level, chunk_bounds, light, 31, 2, 21);
    frame.place_block(level, chunk_bounds, light, 25, 1, 21);
    frame.place_block(level, chunk_bounds, light, 32, 1, 21);

    for i in 0..7 {
        frame.place_block(level, chunk_bounds, black, 28 - i, 6 + i, 21);
        frame.place_block(level, chunk_bounds, black, 29 + i, 6 + i, 21);
    }
    for i in 0..4 {
        frame.place_block(level, chunk_bounds, black, 28 - i, 9 + i, 21);
        frame.place_block(level, chunk_bounds, black, 29 + i, 9 + i, 21);
    }
    frame.place_block(level, chunk_bounds, black, 28, 12, 21);
    frame.place_block(level, chunk_bounds, black, 29, 12, 21);
    for i in 0..3 {
        frame.place_block(level, chunk_bounds, black, 22 - i * 2, 8, 21);
        frame.place_block(level, chunk_bounds, black, 22 - i * 2, 9, 21);
        frame.place_block(level, chunk_bounds, black, 35 + i * 2, 8, 21);
        frame.place_block(level, chunk_bounds, black, 35 + i * 2, 9, 21);
    }

    generate_water_box(frame, level, chunk_bounds, 15, 13, 21, 42, 15, 21);
    generate_water_box(frame, level, chunk_bounds, 15, 1, 21, 15, 6, 21);
    generate_water_box(frame, level, chunk_bounds, 16, 1, 21, 16, 5, 21);
    generate_water_box(frame, level, chunk_bounds, 17, 1, 21, 20, 4, 21);
    generate_water_box(frame, level, chunk_bounds, 21, 1, 21, 21, 3, 21);
    generate_water_box(frame, level, chunk_bounds, 22, 1, 21, 22, 2, 21);
    generate_water_box(frame, level, chunk_bounds, 23, 1, 21, 24, 1, 21);
    generate_water_box(frame, level, chunk_bounds, 42, 1, 21, 42, 6, 21);
    generate_water_box(frame, level, chunk_bounds, 41, 1, 21, 41, 5, 21);
    generate_water_box(frame, level, chunk_bounds, 37, 1, 21, 40, 4, 21);
    generate_water_box(frame, level, chunk_bounds, 36, 1, 21, 36, 3, 21);
    generate_water_box(frame, level, chunk_bounds, 33, 1, 21, 34, 1, 21);
    generate_water_box(frame, level, chunk_bounds, 35, 1, 21, 35, 2, 21);
}
