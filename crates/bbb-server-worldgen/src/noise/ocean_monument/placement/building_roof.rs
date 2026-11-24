use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;

use super::{box_same, chunk_intersects, generate_water_box, state};

pub fn place<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    if !chunk_intersects(frame, chunk_bounds, 21, 21, 36, 36) {
        return;
    }
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let lamp = state("minecraft:sea_lantern");

    // `MonumentBuilding.generateRoofPiece` (`OceanMonumentPieces.java:524-558`).
    box_same(frame, level, chunk_bounds, 21, 0, 22, 36, 0, 36, gray);
    generate_water_box(frame, level, chunk_bounds, 21, 1, 22, 36, 23, 36);
    for i in 0..4 {
        box_same(
            frame,
            level,
            chunk_bounds,
            21 + i,
            13 + i,
            21 + i,
            36 - i,
            13 + i,
            21 + i,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            21 + i,
            13 + i,
            36 - i,
            36 - i,
            13 + i,
            36 - i,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            21 + i,
            13 + i,
            22 + i,
            21 + i,
            13 + i,
            35 - i,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            36 - i,
            13 + i,
            22 + i,
            36 - i,
            13 + i,
            35 - i,
            light,
        );
    }
    box_same(frame, level, chunk_bounds, 25, 16, 25, 32, 16, 32, gray);
    box_same(frame, level, chunk_bounds, 25, 17, 25, 25, 19, 25, light);
    box_same(frame, level, chunk_bounds, 32, 17, 25, 32, 19, 25, light);
    box_same(frame, level, chunk_bounds, 25, 17, 32, 25, 19, 32, light);
    box_same(frame, level, chunk_bounds, 32, 17, 32, 32, 19, 32, light);

    for (block, x, y, z) in [
        (light, 26, 20, 26),
        (light, 27, 21, 27),
        (lamp, 27, 20, 27),
        (light, 26, 20, 31),
        (light, 27, 21, 30),
        (lamp, 27, 20, 30),
        (light, 31, 20, 31),
        (light, 30, 21, 30),
        (lamp, 30, 20, 30),
        (light, 31, 20, 26),
        (light, 30, 21, 27),
        (lamp, 30, 20, 27),
    ] {
        frame.place_block(level, chunk_bounds, block, x, y, z);
    }
    box_same(frame, level, chunk_bounds, 28, 21, 27, 29, 21, 27, gray);
    box_same(frame, level, chunk_bounds, 27, 21, 28, 27, 21, 29, gray);
    box_same(frame, level, chunk_bounds, 28, 21, 30, 29, 21, 30, gray);
    box_same(frame, level, chunk_bounds, 30, 21, 28, 30, 21, 29, gray);
}
