use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;

use super::{box_same, chunk_intersects, generate_water_box, state};

pub fn place<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    generate_lower_wall(frame, level, chunk_bounds, gray, light);
    generate_middle_wall(frame, level, chunk_bounds, gray, light);
    generate_upper_wall(frame, level, chunk_bounds, gray, light)
}

/// `MonumentBuilding.generateLowerWall` (`OceanMonumentPieces.java:560-610`).
fn generate_lower_wall<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    gray: i32,
    light: i32,
) {
    if chunk_intersects(frame, chunk_bounds, 0, 21, 6, 58) {
        box_same(frame, level, chunk_bounds, 0, 0, 21, 6, 0, 57, gray);
        generate_water_box(frame, level, chunk_bounds, 0, 1, 21, 6, 7, 57);
        box_same(frame, level, chunk_bounds, 4, 4, 21, 6, 4, 53, gray);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                i,
                i + 1,
                21,
                i,
                i + 1,
                57 - i,
                light,
            );
        }

        for z in (23..53).step_by(3) {
            frame.place_block(level, chunk_bounds, light, 5, 5, z);
        }

        frame.place_block(level, chunk_bounds, light, 5, 5, 52);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                i,
                i + 1,
                21,
                i,
                i + 1,
                57 - i,
                light,
            );
        }

        box_same(frame, level, chunk_bounds, 4, 1, 52, 6, 3, 52, gray);
        box_same(frame, level, chunk_bounds, 5, 1, 51, 5, 3, 53, gray);
    }

    if chunk_intersects(frame, chunk_bounds, 51, 21, 58, 58) {
        box_same(frame, level, chunk_bounds, 51, 0, 21, 57, 0, 57, gray);
        generate_water_box(frame, level, chunk_bounds, 51, 1, 21, 57, 7, 57);
        box_same(frame, level, chunk_bounds, 51, 4, 21, 53, 4, 53, gray);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                57 - i,
                i + 1,
                21,
                57 - i,
                i + 1,
                57 - i,
                light,
            );
        }

        for z in (23..53).step_by(3) {
            frame.place_block(level, chunk_bounds, light, 52, 5, z);
        }

        frame.place_block(level, chunk_bounds, light, 52, 5, 52);
        box_same(frame, level, chunk_bounds, 51, 1, 52, 53, 3, 52, gray);
        box_same(frame, level, chunk_bounds, 52, 1, 51, 52, 3, 53, gray);
    }

    if chunk_intersects(frame, chunk_bounds, 0, 51, 57, 57) {
        box_same(frame, level, chunk_bounds, 7, 0, 51, 50, 0, 57, gray);
        generate_water_box(frame, level, chunk_bounds, 7, 1, 51, 50, 10, 57);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                i + 1,
                i + 1,
                57 - i,
                56 - i,
                i + 1,
                57 - i,
                light,
            );
        }
    }
}

/// `MonumentBuilding.generateMiddleWall` (`OceanMonumentPieces.java:612-669`).
fn generate_middle_wall<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    gray: i32,
    light: i32,
) {
    if chunk_intersects(frame, chunk_bounds, 7, 21, 13, 50) {
        box_same(frame, level, chunk_bounds, 7, 0, 21, 13, 0, 50, gray);
        generate_water_box(frame, level, chunk_bounds, 7, 1, 21, 13, 10, 50);
        box_same(frame, level, chunk_bounds, 11, 8, 21, 13, 8, 53, gray);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                i + 7,
                i + 5,
                21,
                i + 7,
                i + 5,
                54,
                light,
            );
        }

        for z in (21..=45).step_by(3) {
            frame.place_block(level, chunk_bounds, light, 12, 9, z);
        }
    }

    if chunk_intersects(frame, chunk_bounds, 44, 21, 50, 54) {
        box_same(frame, level, chunk_bounds, 44, 0, 21, 50, 0, 50, gray);
        generate_water_box(frame, level, chunk_bounds, 44, 1, 21, 50, 10, 50);
        box_same(frame, level, chunk_bounds, 44, 8, 21, 46, 8, 53, gray);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                50 - i,
                i + 5,
                21,
                50 - i,
                i + 5,
                54,
                light,
            );
        }

        for z in (21..=45).step_by(3) {
            frame.place_block(level, chunk_bounds, light, 45, 9, z);
        }
    }

    if chunk_intersects(frame, chunk_bounds, 8, 44, 49, 54) {
        box_same(frame, level, chunk_bounds, 14, 0, 44, 43, 0, 50, gray);
        generate_water_box(frame, level, chunk_bounds, 14, 1, 44, 43, 10, 50);

        for x in (12..=45).step_by(3) {
            frame.place_block(level, chunk_bounds, light, x, 9, 45);
            frame.place_block(level, chunk_bounds, light, x, 9, 52);
            if matches!(x, 12 | 18 | 24 | 33 | 39 | 45) {
                frame.place_block(level, chunk_bounds, light, x, 9, 47);
                frame.place_block(level, chunk_bounds, light, x, 9, 50);
                frame.place_block(level, chunk_bounds, light, x, 10, 45);
                frame.place_block(level, chunk_bounds, light, x, 10, 46);
                frame.place_block(level, chunk_bounds, light, x, 10, 51);
                frame.place_block(level, chunk_bounds, light, x, 10, 52);
                frame.place_block(level, chunk_bounds, light, x, 11, 47);
                frame.place_block(level, chunk_bounds, light, x, 11, 50);
                frame.place_block(level, chunk_bounds, light, x, 12, 48);
                frame.place_block(level, chunk_bounds, light, x, 12, 49);
            }
        }

        for i in 0..3 {
            box_same(
                frame,
                level,
                chunk_bounds,
                8 + i,
                5 + i,
                54,
                49 - i,
                5 + i,
                54,
                gray,
            );
        }

        box_same(frame, level, chunk_bounds, 11, 8, 54, 46, 8, 54, light);
        box_same(frame, level, chunk_bounds, 14, 8, 44, 43, 8, 53, gray);
    }
}

/// `MonumentBuilding.generateUpperWall` (`OceanMonumentPieces.java:671-715`).
fn generate_upper_wall<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    gray: i32,
    light: i32,
) {
    if chunk_intersects(frame, chunk_bounds, 14, 21, 20, 43) {
        box_same(frame, level, chunk_bounds, 14, 0, 21, 20, 0, 43, gray);
        generate_water_box(frame, level, chunk_bounds, 14, 1, 22, 20, 14, 43);
        box_same(frame, level, chunk_bounds, 18, 12, 22, 20, 12, 39, gray);
        box_same(frame, level, chunk_bounds, 18, 12, 21, 20, 12, 21, light);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                i + 14,
                i + 9,
                21,
                i + 14,
                i + 9,
                43 - i,
                light,
            );
        }

        for z in (23..=39).step_by(3) {
            frame.place_block(level, chunk_bounds, light, 19, 13, z);
        }
    }

    if chunk_intersects(frame, chunk_bounds, 37, 21, 43, 43) {
        box_same(frame, level, chunk_bounds, 37, 0, 21, 43, 0, 43, gray);
        generate_water_box(frame, level, chunk_bounds, 37, 1, 22, 43, 14, 43);
        box_same(frame, level, chunk_bounds, 37, 12, 22, 39, 12, 39, gray);
        box_same(frame, level, chunk_bounds, 37, 12, 21, 39, 12, 21, light);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                43 - i,
                i + 9,
                21,
                43 - i,
                i + 9,
                43 - i,
                light,
            );
        }

        for z in (23..=39).step_by(3) {
            frame.place_block(level, chunk_bounds, light, 38, 13, z);
        }
    }

    if chunk_intersects(frame, chunk_bounds, 15, 37, 42, 43) {
        box_same(frame, level, chunk_bounds, 21, 0, 37, 36, 0, 43, gray);
        generate_water_box(frame, level, chunk_bounds, 21, 1, 37, 36, 14, 43);
        box_same(frame, level, chunk_bounds, 21, 12, 37, 36, 12, 39, gray);

        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                15 + i,
                i + 9,
                43 - i,
                42 - i,
                i + 9,
                43 - i,
                light,
            );
        }

        for x in (21..=36).step_by(3) {
            frame.place_block(level, chunk_bounds, light, x, 13, 38);
        }
    }
}
