use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;

use super::super::{
    OceanMonumentRoomCells, OceanMonumentRoomKind, OceanMonumentRoomPiece, DIR_DOWN, DIR_EAST,
    DIR_NORTH, DIR_SOUTH, DIR_UP, DIR_WEST,
};
use super::{
    box_same, generate_box_on_fill_only, generate_default_floor, generate_water_box, place_block,
    state,
};

pub fn place<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    cells: &OceanMonumentRoomCells,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let _ = random;
    let frame = GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation());
    match room.kind() {
        OceanMonumentRoomKind::DoubleX => place_double_x(room, cells, &frame, level, chunk_bounds),
        OceanMonumentRoomKind::DoubleXY => {
            place_double_xy(room, cells, &frame, level, chunk_bounds)
        }
        OceanMonumentRoomKind::DoubleY => place_double_y(room, cells, &frame, level, chunk_bounds),
        OceanMonumentRoomKind::DoubleYZ => {
            place_double_yz(room, cells, &frame, level, chunk_bounds)
        }
        OceanMonumentRoomKind::DoubleZ => place_double_z(room, cells, &frame, level, chunk_bounds),
        _ => {}
    }
}

/// `OceanMonumentDoubleXRoom.postProcess` (`OceanMonumentPieces.java:829-884`).
fn place_double_x<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    cells: &OceanMonumentRoomCells,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let lamp = state("minecraft:sea_lantern");
    let west = room.room_index();
    let Some(east) = cells.neighbor(west, DIR_EAST) else {
        return;
    };
    if west / 25 > 0 {
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            8,
            0,
            cells.opening(east, DIR_DOWN),
            gray,
            light,
        );
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            0,
            cells.opening(west, DIR_DOWN),
            gray,
            light,
        );
    }
    if !cells.neighbor_present(west, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 4, 1, 7, 4, 6, gray);
    }
    if !cells.neighbor_present(east, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 8, 4, 1, 14, 4, 6, gray);
    }
    box_same(frame, level, chunk_bounds, 0, 3, 0, 0, 3, 7, light);
    box_same(frame, level, chunk_bounds, 15, 3, 0, 15, 3, 7, light);
    box_same(frame, level, chunk_bounds, 1, 3, 0, 15, 3, 0, light);
    box_same(frame, level, chunk_bounds, 1, 3, 7, 14, 3, 7, light);
    box_same(frame, level, chunk_bounds, 0, 2, 0, 0, 2, 7, gray);
    box_same(frame, level, chunk_bounds, 15, 2, 0, 15, 2, 7, gray);
    box_same(frame, level, chunk_bounds, 1, 2, 0, 15, 2, 0, gray);
    box_same(frame, level, chunk_bounds, 1, 2, 7, 14, 2, 7, gray);
    box_same(frame, level, chunk_bounds, 0, 1, 0, 0, 1, 7, light);
    box_same(frame, level, chunk_bounds, 15, 1, 0, 15, 1, 7, light);
    box_same(frame, level, chunk_bounds, 1, 1, 0, 15, 1, 0, light);
    box_same(frame, level, chunk_bounds, 1, 1, 7, 14, 1, 7, light);
    box_same(frame, level, chunk_bounds, 5, 1, 0, 10, 1, 4, light);
    box_same(frame, level, chunk_bounds, 6, 2, 0, 9, 2, 3, gray);
    box_same(frame, level, chunk_bounds, 5, 3, 0, 10, 3, 4, light);
    place_block(frame, level, chunk_bounds, lamp, 6, 2, 3);
    place_block(frame, level, chunk_bounds, lamp, 9, 2, 3);
    if cells.opening(west, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 0, 4, 2, 0);
    }
    if cells.opening(west, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 7, 4, 2, 7);
    }
    if cells.opening(west, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 3, 0, 2, 4);
    }
    if cells.opening(east, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 11, 1, 0, 12, 2, 0);
    }
    if cells.opening(east, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 11, 1, 7, 12, 2, 7);
    }
    if cells.opening(east, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 15, 1, 3, 15, 2, 4);
    }
}

/// `OceanMonumentDoubleXYRoom.postProcess` (`OceanMonumentPieces.java:906-1005`).
fn place_double_xy<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    cells: &OceanMonumentRoomCells,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let lamp = state("minecraft:sea_lantern");
    let west = room.room_index();
    let Some(east) = cells.neighbor(west, DIR_EAST) else {
        return;
    };
    let Some(west_up) = cells.neighbor(west, DIR_UP) else {
        return;
    };
    let Some(east_up) = cells.neighbor(east, DIR_UP) else {
        return;
    };
    if west / 25 > 0 {
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            8,
            0,
            cells.opening(east, DIR_DOWN),
            gray,
            light,
        );
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            0,
            cells.opening(west, DIR_DOWN),
            gray,
            light,
        );
    }
    if !cells.neighbor_present(west_up, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 8, 1, 7, 8, 6, gray);
    }
    if !cells.neighbor_present(east_up, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 8, 8, 1, 14, 8, 6, gray);
    }
    for y in 1..=7 {
        let block = if y == 2 || y == 6 { gray } else { light };
        box_same(frame, level, chunk_bounds, 0, y, 0, 0, y, 7, block);
        box_same(frame, level, chunk_bounds, 15, y, 0, 15, y, 7, block);
        box_same(frame, level, chunk_bounds, 1, y, 0, 15, y, 0, block);
        box_same(frame, level, chunk_bounds, 1, y, 7, 14, y, 7, block);
    }
    box_same(frame, level, chunk_bounds, 2, 1, 3, 2, 7, 4, light);
    box_same(frame, level, chunk_bounds, 3, 1, 2, 4, 7, 2, light);
    box_same(frame, level, chunk_bounds, 3, 1, 5, 4, 7, 5, light);
    box_same(frame, level, chunk_bounds, 13, 1, 3, 13, 7, 4, light);
    box_same(frame, level, chunk_bounds, 11, 1, 2, 12, 7, 2, light);
    box_same(frame, level, chunk_bounds, 11, 1, 5, 12, 7, 5, light);
    box_same(frame, level, chunk_bounds, 5, 1, 3, 5, 3, 4, light);
    box_same(frame, level, chunk_bounds, 10, 1, 3, 10, 3, 4, light);
    box_same(frame, level, chunk_bounds, 5, 7, 2, 10, 7, 5, light);
    box_same(frame, level, chunk_bounds, 5, 5, 2, 5, 7, 2, light);
    box_same(frame, level, chunk_bounds, 10, 5, 2, 10, 7, 2, light);
    box_same(frame, level, chunk_bounds, 5, 5, 5, 5, 7, 5, light);
    box_same(frame, level, chunk_bounds, 10, 5, 5, 10, 7, 5, light);
    place_block(frame, level, chunk_bounds, light, 6, 6, 2);
    place_block(frame, level, chunk_bounds, light, 9, 6, 2);
    place_block(frame, level, chunk_bounds, light, 6, 6, 5);
    place_block(frame, level, chunk_bounds, light, 9, 6, 5);
    box_same(frame, level, chunk_bounds, 5, 4, 3, 6, 4, 4, light);
    box_same(frame, level, chunk_bounds, 9, 4, 3, 10, 4, 4, light);
    place_block(frame, level, chunk_bounds, lamp, 5, 4, 2);
    place_block(frame, level, chunk_bounds, lamp, 5, 4, 5);
    place_block(frame, level, chunk_bounds, lamp, 10, 4, 2);
    place_block(frame, level, chunk_bounds, lamp, 10, 4, 5);
    if cells.opening(west, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 0, 4, 2, 0);
    }
    if cells.opening(west, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 7, 4, 2, 7);
    }
    if cells.opening(west, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 3, 0, 2, 4);
    }
    if cells.opening(east, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 11, 1, 0, 12, 2, 0);
    }
    if cells.opening(east, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 11, 1, 7, 12, 2, 7);
    }
    if cells.opening(east, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 15, 1, 3, 15, 2, 4);
    }
    if cells.opening(west_up, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 5, 0, 4, 6, 0);
    }
    if cells.opening(west_up, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 5, 7, 4, 6, 7);
    }
    if cells.opening(west_up, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 5, 3, 0, 6, 4);
    }
    if cells.opening(east_up, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 11, 5, 0, 12, 6, 0);
    }
    if cells.opening(east_up, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 11, 5, 7, 12, 6, 7);
    }
    if cells.opening(east_up, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 15, 5, 3, 15, 6, 4);
    }
}

/// `OceanMonumentDoubleYRoom.postProcess` (`OceanMonumentPieces.java:1027-1093`).
fn place_double_y<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    cells: &OceanMonumentRoomCells,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let source = room.room_index();
    if source / 25 > 0 {
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            0,
            cells.opening(source, DIR_DOWN),
            gray,
            light,
        );
    }
    let Some(above) = cells.neighbor(source, DIR_UP) else {
        return;
    };
    if !cells.neighbor_present(above, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 8, 1, 6, 8, 6, gray);
    }
    box_same(frame, level, chunk_bounds, 0, 4, 0, 0, 4, 7, light);
    box_same(frame, level, chunk_bounds, 7, 4, 0, 7, 4, 7, light);
    box_same(frame, level, chunk_bounds, 1, 4, 0, 6, 4, 0, light);
    box_same(frame, level, chunk_bounds, 1, 4, 7, 6, 4, 7, light);
    box_same(frame, level, chunk_bounds, 2, 4, 1, 2, 4, 2, light);
    box_same(frame, level, chunk_bounds, 1, 4, 2, 1, 4, 2, light);
    box_same(frame, level, chunk_bounds, 5, 4, 1, 5, 4, 2, light);
    box_same(frame, level, chunk_bounds, 6, 4, 2, 6, 4, 2, light);
    box_same(frame, level, chunk_bounds, 2, 4, 5, 2, 4, 6, light);
    box_same(frame, level, chunk_bounds, 1, 4, 5, 1, 4, 5, light);
    box_same(frame, level, chunk_bounds, 5, 4, 5, 5, 4, 6, light);
    box_same(frame, level, chunk_bounds, 6, 4, 5, 6, 4, 5, light);
    let mut definition = source;
    for y in (1..=5).step_by(4) {
        let z = 0;
        if cells.opening(definition, DIR_SOUTH) {
            box_same(frame, level, chunk_bounds, 2, y, z, 2, y + 2, z, light);
            box_same(frame, level, chunk_bounds, 5, y, z, 5, y + 2, z, light);
            box_same(frame, level, chunk_bounds, 3, y + 2, z, 4, y + 2, z, light);
        } else {
            box_same(frame, level, chunk_bounds, 0, y, z, 7, y + 2, z, light);
            box_same(frame, level, chunk_bounds, 0, y + 1, z, 7, y + 1, z, gray);
        }
        let north_z = 7;
        if cells.opening(definition, DIR_NORTH) {
            box_same(
                frame,
                level,
                chunk_bounds,
                2,
                y,
                north_z,
                2,
                y + 2,
                north_z,
                light,
            );
            box_same(
                frame,
                level,
                chunk_bounds,
                5,
                y,
                north_z,
                5,
                y + 2,
                north_z,
                light,
            );
            box_same(
                frame,
                level,
                chunk_bounds,
                3,
                y + 2,
                north_z,
                4,
                y + 2,
                north_z,
                light,
            );
        } else {
            box_same(
                frame,
                level,
                chunk_bounds,
                0,
                y,
                north_z,
                7,
                y + 2,
                north_z,
                light,
            );
            box_same(
                frame,
                level,
                chunk_bounds,
                0,
                y + 1,
                north_z,
                7,
                y + 1,
                north_z,
                gray,
            );
        }
        let x = 0;
        if cells.opening(definition, DIR_WEST) {
            box_same(frame, level, chunk_bounds, x, y, 2, x, y + 2, 2, light);
            box_same(frame, level, chunk_bounds, x, y, 5, x, y + 2, 5, light);
            box_same(frame, level, chunk_bounds, x, y + 2, 3, x, y + 2, 4, light);
        } else {
            box_same(frame, level, chunk_bounds, x, y, 0, x, y + 2, 7, light);
            box_same(frame, level, chunk_bounds, x, y + 1, 0, x, y + 1, 7, gray);
        }
        let east_x = 7;
        if cells.opening(definition, DIR_EAST) {
            box_same(
                frame,
                level,
                chunk_bounds,
                east_x,
                y,
                2,
                east_x,
                y + 2,
                2,
                light,
            );
            box_same(
                frame,
                level,
                chunk_bounds,
                east_x,
                y,
                5,
                east_x,
                y + 2,
                5,
                light,
            );
            box_same(
                frame,
                level,
                chunk_bounds,
                east_x,
                y + 2,
                3,
                east_x,
                y + 2,
                4,
                light,
            );
        } else {
            box_same(
                frame,
                level,
                chunk_bounds,
                east_x,
                y,
                0,
                east_x,
                y + 2,
                7,
                light,
            );
            box_same(
                frame,
                level,
                chunk_bounds,
                east_x,
                y + 1,
                0,
                east_x,
                y + 1,
                7,
                gray,
            );
        }
        definition = above;
    }
}

/// `OceanMonumentDoubleYZRoom.postProcess` (`OceanMonumentPieces.java:1115-1212`).
fn place_double_yz<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    cells: &OceanMonumentRoomCells,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let black = state("minecraft:dark_prismarine");
    let lamp = state("minecraft:sea_lantern");
    let south = room.room_index();
    let Some(north) = cells.neighbor(south, DIR_NORTH) else {
        return;
    };
    let Some(south_up) = cells.neighbor(south, DIR_UP) else {
        return;
    };
    let Some(north_up) = cells.neighbor(north, DIR_UP) else {
        return;
    };
    if south / 25 > 0 {
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            8,
            cells.opening(north, DIR_DOWN),
            gray,
            light,
        );
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            0,
            cells.opening(south, DIR_DOWN),
            gray,
            light,
        );
    }
    if !cells.neighbor_present(south_up, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 8, 1, 6, 8, 7, gray);
    }
    if !cells.neighbor_present(north_up, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 8, 8, 6, 8, 14, gray);
    }
    for y in 1..=7 {
        let block = if y == 2 || y == 6 { gray } else { light };
        box_same(frame, level, chunk_bounds, 0, y, 0, 0, y, 15, block);
        box_same(frame, level, chunk_bounds, 7, y, 0, 7, y, 15, block);
        box_same(frame, level, chunk_bounds, 1, y, 0, 6, y, 0, block);
        box_same(frame, level, chunk_bounds, 1, y, 15, 6, y, 15, block);
    }
    for y in 1..=7 {
        let block = if y == 2 || y == 6 { lamp } else { black };
        box_same(frame, level, chunk_bounds, 3, y, 7, 4, y, 8, block);
    }
    if cells.opening(south, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 0, 4, 2, 0);
    }
    if cells.opening(south, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 7, 1, 3, 7, 2, 4);
    }
    if cells.opening(south, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 3, 0, 2, 4);
    }
    if cells.opening(north, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 15, 4, 2, 15);
    }
    if cells.opening(north, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 11, 0, 2, 12);
    }
    if cells.opening(north, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 7, 1, 11, 7, 2, 12);
    }
    if cells.opening(south_up, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 5, 0, 4, 6, 0);
    }
    if cells.opening(south_up, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 7, 5, 3, 7, 6, 4);
        box_same(frame, level, chunk_bounds, 5, 4, 2, 6, 4, 5, light);
        box_same(frame, level, chunk_bounds, 6, 1, 2, 6, 3, 2, light);
        box_same(frame, level, chunk_bounds, 6, 1, 5, 6, 3, 5, light);
    }
    if cells.opening(south_up, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 5, 3, 0, 6, 4);
        box_same(frame, level, chunk_bounds, 1, 4, 2, 2, 4, 5, light);
        box_same(frame, level, chunk_bounds, 1, 1, 2, 1, 3, 2, light);
        box_same(frame, level, chunk_bounds, 1, 1, 5, 1, 3, 5, light);
    }
    if cells.opening(north_up, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 5, 15, 4, 6, 15);
    }
    if cells.opening(north_up, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 5, 11, 0, 6, 12);
        box_same(frame, level, chunk_bounds, 1, 4, 10, 2, 4, 13, light);
        box_same(frame, level, chunk_bounds, 1, 1, 10, 1, 3, 10, light);
        box_same(frame, level, chunk_bounds, 1, 1, 13, 1, 3, 13, light);
    }
    if cells.opening(north_up, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 7, 5, 11, 7, 6, 12);
        box_same(frame, level, chunk_bounds, 5, 4, 10, 6, 4, 13, light);
        box_same(frame, level, chunk_bounds, 6, 1, 10, 6, 3, 10, light);
        box_same(frame, level, chunk_bounds, 6, 1, 13, 6, 3, 13, light);
    }
}

/// `OceanMonumentDoubleZRoom.postProcess` (`OceanMonumentPieces.java:1234-1307`).
fn place_double_z<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    cells: &OceanMonumentRoomCells,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let lamp = state("minecraft:sea_lantern");
    let south = room.room_index();
    let Some(north) = cells.neighbor(south, DIR_NORTH) else {
        return;
    };
    if south / 25 > 0 {
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            8,
            cells.opening(north, DIR_DOWN),
            gray,
            light,
        );
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            0,
            cells.opening(south, DIR_DOWN),
            gray,
            light,
        );
    }
    if !cells.neighbor_present(south, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 4, 1, 6, 4, 7, gray);
    }
    if !cells.neighbor_present(north, DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 4, 8, 6, 4, 14, gray);
    }
    box_same(frame, level, chunk_bounds, 0, 3, 0, 0, 3, 15, light);
    box_same(frame, level, chunk_bounds, 7, 3, 0, 7, 3, 15, light);
    box_same(frame, level, chunk_bounds, 1, 3, 0, 7, 3, 0, light);
    box_same(frame, level, chunk_bounds, 1, 3, 15, 6, 3, 15, light);
    box_same(frame, level, chunk_bounds, 0, 2, 0, 0, 2, 15, gray);
    box_same(frame, level, chunk_bounds, 7, 2, 0, 7, 2, 15, gray);
    box_same(frame, level, chunk_bounds, 1, 2, 0, 7, 2, 0, gray);
    box_same(frame, level, chunk_bounds, 1, 2, 15, 6, 2, 15, gray);
    box_same(frame, level, chunk_bounds, 0, 1, 0, 0, 1, 15, light);
    box_same(frame, level, chunk_bounds, 7, 1, 0, 7, 1, 15, light);
    box_same(frame, level, chunk_bounds, 1, 1, 0, 7, 1, 0, light);
    box_same(frame, level, chunk_bounds, 1, 1, 15, 6, 1, 15, light);
    box_same(frame, level, chunk_bounds, 1, 1, 1, 1, 1, 2, light);
    box_same(frame, level, chunk_bounds, 6, 1, 1, 6, 1, 2, light);
    box_same(frame, level, chunk_bounds, 1, 3, 1, 1, 3, 2, light);
    box_same(frame, level, chunk_bounds, 6, 3, 1, 6, 3, 2, light);
    box_same(frame, level, chunk_bounds, 1, 1, 13, 1, 1, 14, light);
    box_same(frame, level, chunk_bounds, 6, 1, 13, 6, 1, 14, light);
    box_same(frame, level, chunk_bounds, 1, 3, 13, 1, 3, 14, light);
    box_same(frame, level, chunk_bounds, 6, 3, 13, 6, 3, 14, light);
    box_same(frame, level, chunk_bounds, 2, 1, 6, 2, 3, 6, light);
    box_same(frame, level, chunk_bounds, 5, 1, 6, 5, 3, 6, light);
    box_same(frame, level, chunk_bounds, 2, 1, 9, 2, 3, 9, light);
    box_same(frame, level, chunk_bounds, 5, 1, 9, 5, 3, 9, light);
    box_same(frame, level, chunk_bounds, 3, 2, 6, 4, 2, 6, light);
    box_same(frame, level, chunk_bounds, 3, 2, 9, 4, 2, 9, light);
    box_same(frame, level, chunk_bounds, 2, 2, 7, 2, 2, 8, light);
    box_same(frame, level, chunk_bounds, 5, 2, 7, 5, 2, 8, light);
    place_block(frame, level, chunk_bounds, lamp, 2, 2, 5);
    place_block(frame, level, chunk_bounds, lamp, 5, 2, 5);
    place_block(frame, level, chunk_bounds, lamp, 2, 2, 10);
    place_block(frame, level, chunk_bounds, lamp, 5, 2, 10);
    place_block(frame, level, chunk_bounds, light, 2, 3, 5);
    place_block(frame, level, chunk_bounds, light, 5, 3, 5);
    place_block(frame, level, chunk_bounds, light, 2, 3, 10);
    place_block(frame, level, chunk_bounds, light, 5, 3, 10);
    if cells.opening(south, DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 0, 4, 2, 0);
    }
    if cells.opening(south, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 7, 1, 3, 7, 2, 4);
    }
    if cells.opening(south, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 3, 0, 2, 4);
    }
    if cells.opening(north, DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 15, 4, 2, 15);
    }
    if cells.opening(north, DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 11, 0, 2, 12);
    }
    if cells.opening(north, DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 7, 1, 11, 7, 2, 12);
    }
}

#[cfg(test)]
mod tests {
    use crate::random::WorldgenRandom;

    use crate::block_is;
    use crate::noise::generated_structure_piece::{bounding_box_from_array, GeneratedPieceFrame};

    use super::super::super::{
        OceanMonumentRoomCells, OceanMonumentRoomKind, OceanMonumentRoomPiece,
        RegeneratedMonumentChildren, DIR_DOWN,
    };
    use super::super::test_support::{monument_children_with, PlacementLevel};
    use super::place;

    fn seed0_children() -> RegeneratedMonumentChildren {
        monument_children_with(&[
            OceanMonumentRoomKind::DoubleX,
            OceanMonumentRoomKind::DoubleYZ,
            OceanMonumentRoomKind::DoubleZ,
        ])
        .1
    }

    fn place_room(room: &OceanMonumentRoomPiece, cells: &OceanMonumentRoomCells) -> PlacementLevel {
        let bounds = bounding_box_from_array(room.bounding_box());
        let mut level = PlacementLevel::new(0);
        let mut random = WorldgenRandom::with_seed(0);
        place(room, cells, &mut level, bounds, &mut random);
        level
    }

    fn local_is(
        level: &PlacementLevel,
        frame: GeneratedPieceFrame,
        x: i32,
        y: i32,
        z: i32,
        block: &str,
    ) -> bool {
        block_is(level.get_block(frame.world_pos(x, y, z)), block)
    }

    fn is_floor_block(
        level: &PlacementLevel,
        frame: GeneratedPieceFrame,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        local_is(level, frame, x, y, z, "minecraft:prismarine")
            || local_is(level, frame, x, y, z, "minecraft:prismarine_bricks")
    }

    #[test]
    fn double_x_or_z_places_a_vanilla_lamp() {
        let children = seed0_children();
        let (room, lamp) = children
            .pieces
            .iter()
            .find_map(|room| match room.kind() {
                OceanMonumentRoomKind::DoubleX => Some((room, (6, 2, 3))),
                OceanMonumentRoomKind::DoubleZ => Some((room, (2, 2, 5))),
                _ => None,
            })
            .expect("seed-0 monument has a DoubleX or DoubleZ");
        let level = place_room(room, &children.cells);
        let frame = GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation());
        assert!(
            local_is(
                &level,
                frame,
                lamp.0,
                lamp.1,
                lamp.2,
                "minecraft:sea_lantern"
            ),
            "lamp at {:?} in {:?}",
            lamp,
            room.kind()
        );
    }

    #[test]
    fn double_yz_places_dark_prismarine_column() {
        let children = seed0_children();
        let room = children
            .pieces
            .iter()
            .find(|room| room.kind() == OceanMonumentRoomKind::DoubleYZ)
            .expect("seed-0 monument has a DoubleYZ");
        let level = place_room(room, &children.cells);
        let frame = GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation());
        assert!(
            local_is(&level, frame, 3, 1, 7, "minecraft:dark_prismarine"),
            "DoubleYZ inner column at (3,1,7)"
        );
    }

    #[test]
    fn upper_double_x_or_z_writes_both_default_floors() {
        let children = seed0_children();
        let room = children
            .pieces
            .iter()
            .find(|room| {
                matches!(
                    room.kind(),
                    OceanMonumentRoomKind::DoubleX | OceanMonumentRoomKind::DoubleZ
                ) && room.room_index() / 25 > 0
            })
            .expect("seed-0 monument has an upper-tier DoubleX or DoubleZ");
        let east_or_north = match room.kind() {
            OceanMonumentRoomKind::DoubleX => children
                .cells
                .neighbor(room.room_index(), super::DIR_EAST)
                .expect("DoubleX east cell"),
            OceanMonumentRoomKind::DoubleZ => children
                .cells
                .neighbor(room.room_index(), super::DIR_NORTH)
                .expect("DoubleZ north cell"),
            _ => unreachable!(),
        };
        let (x8, z8) = match room.kind() {
            OceanMonumentRoomKind::DoubleX => (8, 0),
            OceanMonumentRoomKind::DoubleZ => (0, 8),
            _ => unreachable!(),
        };
        let level = place_room(room, &children.cells);
        let frame = GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation());
        assert!(
            is_floor_block(&level, frame, 0, 0, 0),
            "0-offset floor at (0,0,0)"
        );
        assert!(
            is_floor_block(&level, frame, x8, 0, z8),
            "8-offset floor at ({x8},0,{z8})"
        );
        let opening = children.cells.opening(east_or_north, DIR_DOWN);
        let sample = (x8 + 3, 0, z8 + 2);
        if opening {
            assert!(
                local_is(
                    &level,
                    frame,
                    sample.0,
                    sample.1,
                    sample.2,
                    "minecraft:prismarine_bricks"
                ),
                "open 8-offset floor writes light at {sample:?}"
            );
        } else {
            assert!(
                local_is(
                    &level,
                    frame,
                    sample.0,
                    sample.1,
                    sample.2,
                    "minecraft:prismarine"
                ),
                "closed 8-offset floor writes gray at {sample:?}"
            );
        }
    }
}
