//! `WoodlandMansionPieces.MansionPiecePlacer.createMansion` and its direct
//! helpers (`WoodlandMansionPieces.java:401-900`). Room-content placement and
//! roof generation live in the `rooms` and `roof` submodules.

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use super::direction::{above, step_relative, MansionDirection, HORIZONTAL_PLANE};
use super::grid::{
    MansionGrid, SimpleGrid, CORRIDOR, ROOM, ROOM_1X1, ROOM_1X2, ROOM_2X2, ROOM_CORRIDOR_FLAG,
    ROOM_DOOR_FLAG, ROOM_ID_MASK, ROOM_ORIGIN_FLAG, ROOM_STAIRS_FLAG, ROOM_TYPE_MASK, START_ROOM,
};
use super::{qualify_template, WoodlandMansionPiece};
use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_pieces::JigsawRotation;

pub mod roof;
pub mod rooms;

use rooms::FloorRoomKind;

/// `WoodlandMansionPieces.PlacementData` (`WoodlandMansionPieces.java:1111-1115`).
struct PlacementData {
    position: BlockPos,
    rotation: JigsawRotation,
    wall_type: &'static str,
}

fn push(
    pieces: &mut Vec<WoodlandMansionPiece>,
    name: &str,
    pos: BlockPos,
    rotation: JigsawRotation,
) {
    pieces.push(WoodlandMansionPiece::new(
        qualify_template(name),
        pos,
        rotation,
        RuinedPortalMirror::None,
    ));
}

fn rotated(rotation: JigsawRotation, applied: JigsawRotation) -> JigsawRotation {
    super::direction::compose_rotation(rotation, applied)
}

/// `MansionPiecePlacer.createMansion` (`WoodlandMansionPieces.java:412-617`).
pub fn create_mansion(
    origin: BlockPos,
    rotation: JigsawRotation,
    grid: &MansionGrid,
    random: &mut dyn RandomSource,
) -> Vec<WoodlandMansionPiece> {
    let mut pieces = Vec::new();
    let mut data = PlacementData {
        position: origin,
        rotation,
        wall_type: "wall_flat",
    };
    entrance(&mut pieces, &mut data);
    let mut second_data = PlacementData {
        position: above(data.position, 8),
        rotation: data.rotation,
        wall_type: "wall_window",
    };

    let base_grid = &grid.base_grid;
    let third_grid = &grid.third_floor_grid;
    let start_x = grid.entrance_x + 1;
    let start_y = grid.entrance_y + 1;
    let end_x = grid.entrance_x + 1;
    let end_y = grid.entrance_y;
    traverse_outer_walls(
        &mut pieces,
        &mut data,
        base_grid,
        MansionDirection::South,
        start_x,
        start_y,
        end_x,
        end_y,
    );
    traverse_outer_walls(
        &mut pieces,
        &mut second_data,
        base_grid,
        MansionDirection::South,
        start_x,
        start_y,
        end_x,
        end_y,
    );

    let mut third_data = PlacementData {
        position: above(data.position, 19),
        rotation: data.rotation,
        wall_type: "wall_window",
    };
    'search: for y in 0..third_grid.height() {
        for x in (0..third_grid.width()).rev() {
            if MansionGrid::is_house(third_grid, x, y) {
                third_data.position = step_relative(
                    third_data.position,
                    rotation,
                    MansionDirection::South,
                    8 + (y - start_y) * 8,
                );
                third_data.position = step_relative(
                    third_data.position,
                    rotation,
                    MansionDirection::East,
                    (x - start_x) * 8,
                );
                traverse_wall_piece(&mut pieces, &mut third_data);
                traverse_outer_walls(
                    &mut pieces,
                    &mut third_data,
                    third_grid,
                    MansionDirection::South,
                    x,
                    y,
                    x,
                    y,
                );
                break 'search;
            }
        }
    }

    roof::create_roof(
        &mut pieces,
        above(origin, 16),
        rotation,
        base_grid,
        Some(third_grid),
        start_x,
        start_y,
    );
    roof::create_roof(
        &mut pieces,
        above(origin, 27),
        rotation,
        third_grid,
        None,
        start_x,
        start_y,
    );

    for floor_num in 0..3usize {
        let floor_origin = above(origin, 8 * floor_num as i32 + i32::from(floor_num == 2) * 3);
        let rooms_grid = &grid.floor_rooms[floor_num];
        let floor_grid = if floor_num == 2 {
            third_grid
        } else {
            base_grid
        };
        let south_piece = if floor_num == 0 {
            "carpet_south_1"
        } else {
            "carpet_south_2"
        };
        let west_piece = if floor_num == 0 {
            "carpet_west_1"
        } else {
            "carpet_west_2"
        };

        place_corridor_carpets(
            &mut pieces,
            floor_origin,
            rotation,
            floor_grid,
            rooms_grid,
            start_x,
            start_y,
            south_piece,
            west_piece,
        );
        place_rooms(
            &mut pieces,
            floor_origin,
            rotation,
            floor_grid,
            rooms_grid,
            grid,
            floor_num,
            start_x,
            start_y,
            random,
        );
    }

    pieces
}

/// Corridor floor and carpet-piece loop inside `createMansion`
/// (`WoodlandMansionPieces.java:474-522`).
#[allow(clippy::too_many_arguments)]
fn place_corridor_carpets(
    pieces: &mut Vec<WoodlandMansionPiece>,
    floor_origin: BlockPos,
    rotation: JigsawRotation,
    grid: &SimpleGrid,
    rooms: &SimpleGrid,
    start_x: i32,
    start_y: i32,
    south_piece: &str,
    west_piece: &str,
) {
    use MansionDirection::{East, North, South, West};
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.get(x, y) != CORRIDOR {
                continue;
            }
            let mut pos = step_relative(floor_origin, rotation, South, 8 + (y - start_y) * 8);
            pos = step_relative(pos, rotation, East, (x - start_x) * 8);
            push(pieces, "corridor_floor", pos, rotation);
            if grid.get(x, y - 1) == CORRIDOR
                || (rooms.get(x, y - 1) & ROOM_CORRIDOR_FLAG) == ROOM_CORRIDOR_FLAG
            {
                let p = above(step_relative(pos, rotation, East, 1), 1);
                push(pieces, "carpet_north", p, rotation);
            }
            if grid.get(x + 1, y) == CORRIDOR
                || (rooms.get(x + 1, y) & ROOM_CORRIDOR_FLAG) == ROOM_CORRIDOR_FLAG
            {
                let p = above(
                    step_relative(step_relative(pos, rotation, South, 1), rotation, East, 5),
                    1,
                );
                push(pieces, "carpet_east", p, rotation);
            }
            if grid.get(x, y + 1) == CORRIDOR
                || (rooms.get(x, y + 1) & ROOM_CORRIDOR_FLAG) == ROOM_CORRIDOR_FLAG
            {
                let p = step_relative(step_relative(pos, rotation, South, 5), rotation, West, 1);
                push(pieces, south_piece, p, rotation);
            }
            if grid.get(x - 1, y) == CORRIDOR
                || (rooms.get(x - 1, y) & ROOM_CORRIDOR_FLAG) == ROOM_CORRIDOR_FLAG
            {
                let p = step_relative(step_relative(pos, rotation, West, 1), rotation, North, 1);
                push(pieces, west_piece, p, rotation);
            }
        }
    }
}

/// Interior-wall and room-content dispatch loop inside `createMansion`
/// (`WoodlandMansionPieces.java:524-616`).
#[allow(clippy::too_many_arguments)]
fn place_rooms(
    pieces: &mut Vec<WoodlandMansionPiece>,
    floor_origin: BlockPos,
    rotation: JigsawRotation,
    grid: &SimpleGrid,
    rooms: &SimpleGrid,
    mansion: &MansionGrid,
    floor_num: usize,
    start_x: i32,
    start_y: i32,
    random: &mut dyn RandomSource,
) {
    use MansionDirection::{East, North, South, Up, West};
    let wall_piece = if floor_num == 0 {
        "indoors_wall_1"
    } else {
        "indoors_wall_2"
    };
    let door_piece = if floor_num == 0 {
        "indoors_door_1"
    } else {
        "indoors_door_2"
    };
    let room_kind = FloorRoomKind::for_floor(floor_num);

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let mut third_floor_start_room = floor_num == 2 && grid.get(x, y) == START_ROOM;
            if grid.get(x, y) != ROOM && !third_floor_start_room {
                continue;
            }
            let room_data = rooms.get(x, y);
            let room_type = room_data & ROOM_TYPE_MASK;
            let room_id = room_data & ROOM_ID_MASK;
            // `WoodlandMansionPieces.java:535` gates on `ROOM_CORRIDOR_FLAG`, the
            // marker `setupThirdFloor` writes at the stairwell-connected cell.
            third_floor_start_room =
                third_floor_start_room && (room_data & ROOM_CORRIDOR_FLAG) == ROOM_CORRIDOR_FLAG;

            let mut door_dirs = Vec::new();
            if room_data & ROOM_DOOR_FLAG == ROOM_DOOR_FLAG {
                for direction in HORIZONTAL_PLANE {
                    if grid.get(x + direction.step_x(), y + direction.step_z()) == CORRIDOR {
                        door_dirs.push(direction);
                    }
                }
            }
            let door_dir = if !door_dirs.is_empty() {
                Some(door_dirs[random.next_int(door_dirs.len() as i32) as usize])
            } else if room_data & ROOM_ORIGIN_FLAG == ROOM_ORIGIN_FLAG {
                Some(Up)
            } else {
                None
            };

            let mut room_pos = step_relative(floor_origin, rotation, South, 8 + (y - start_y) * 8);
            room_pos = step_relative(room_pos, rotation, East, -1 + (x - start_x) * 8);

            if MansionGrid::is_house(grid, x - 1, y)
                && !mansion.is_room_id(x - 1, y, floor_num, room_id)
            {
                let piece = if door_dir == Some(West) {
                    door_piece
                } else {
                    wall_piece
                };
                push(pieces, piece, room_pos, rotation);
            }
            if grid.get(x + 1, y) == CORRIDOR && !third_floor_start_room {
                let pos = step_relative(room_pos, rotation, East, 8);
                let piece = if door_dir == Some(East) {
                    door_piece
                } else {
                    wall_piece
                };
                push(pieces, piece, pos, rotation);
            }
            if MansionGrid::is_house(grid, x, y + 1)
                && !mansion.is_room_id(x, y + 1, floor_num, room_id)
            {
                let pos = step_relative(
                    step_relative(room_pos, rotation, South, 7),
                    rotation,
                    East,
                    7,
                );
                let piece = if door_dir == Some(South) {
                    door_piece
                } else {
                    wall_piece
                };
                push(
                    pieces,
                    piece,
                    pos,
                    rotated(rotation, JigsawRotation::Clockwise90),
                );
            }
            if grid.get(x, y - 1) == CORRIDOR && !third_floor_start_room {
                let pos = step_relative(
                    step_relative(room_pos, rotation, North, 1),
                    rotation,
                    East,
                    7,
                );
                let piece = if door_dir == Some(North) {
                    door_piece
                } else {
                    wall_piece
                };
                push(
                    pieces,
                    piece,
                    pos,
                    rotated(rotation, JigsawRotation::Clockwise90),
                );
            }

            if room_type == ROOM_1X1 {
                rooms::add_room_1x1(pieces, room_pos, rotation, door_dir, room_kind, random);
            } else if room_type == ROOM_1X2 && door_dir.is_some() {
                let room_dir = mansion.get_1x2_room_direction(x, y, floor_num, room_id);
                let is_stairs_room = room_data & ROOM_STAIRS_FLAG == ROOM_STAIRS_FLAG;
                rooms::add_room_1x2(
                    pieces,
                    room_pos,
                    rotation,
                    room_dir,
                    door_dir,
                    room_kind,
                    is_stairs_room,
                    random,
                );
            } else if room_type == ROOM_2X2 {
                if door_dir == Some(Up) {
                    rooms::add_room_2x2_secret(pieces, room_pos, rotation, room_kind);
                } else if let Some(dir) = door_dir {
                    let mut room_dir = dir.clock_wise();
                    if !mansion.is_room_id(
                        x + room_dir.step_x(),
                        y + room_dir.step_z(),
                        floor_num,
                        room_id,
                    ) {
                        room_dir = room_dir.opposite();
                    }
                    rooms::add_room_2x2(
                        pieces,
                        room_pos,
                        rotation,
                        Some(room_dir),
                        door_dir,
                        room_kind,
                        random,
                    );
                }
            }
        }
    }
}

/// `MansionPiecePlacer.entrance` (`WoodlandMansionPieces.java:873-877`).
fn entrance(pieces: &mut Vec<WoodlandMansionPiece>, data: &mut PlacementData) {
    let west = super::direction::rotate_direction(MansionDirection::West, data.rotation);
    let pos = super::direction::relative(data.position, west, 9);
    push(pieces, "entrance", pos, data.rotation);
    data.position = step_relative(data.position, data.rotation, MansionDirection::South, 16);
}

/// `MansionPiecePlacer.traverseWallPiece` (`WoodlandMansionPieces.java:879-886`).
fn traverse_wall_piece(pieces: &mut Vec<WoodlandMansionPiece>, data: &mut PlacementData) {
    let pos = step_relative(data.position, data.rotation, MansionDirection::East, 7);
    push(pieces, data.wall_type, pos, data.rotation);
    data.position = step_relative(data.position, data.rotation, MansionDirection::South, 8);
}

/// `MansionPiecePlacer.traverseTurn` (`WoodlandMansionPieces.java:888-894`).
fn traverse_turn(pieces: &mut Vec<WoodlandMansionPiece>, data: &mut PlacementData) {
    data.position = step_relative(data.position, data.rotation, MansionDirection::South, -1);
    push(pieces, "wall_corner", data.position, data.rotation);
    data.position = step_relative(data.position, data.rotation, MansionDirection::South, -7);
    data.position = step_relative(data.position, data.rotation, MansionDirection::West, -6);
    data.rotation = rotated(data.rotation, JigsawRotation::Clockwise90);
}

/// `MansionPiecePlacer.traverseInnerTurn` (`WoodlandMansionPieces.java:896-900`);
/// the Vanilla `pieces` parameter is unused there too.
fn traverse_inner_turn(data: &mut PlacementData) {
    data.position = step_relative(data.position, data.rotation, MansionDirection::South, 6);
    data.position = step_relative(data.position, data.rotation, MansionDirection::East, 8);
    data.rotation = rotated(data.rotation, JigsawRotation::CounterClockwise90);
}

/// `MansionPiecePlacer.traverseOuterWalls` (`WoodlandMansionPieces.java:619-658`).
#[allow(clippy::too_many_arguments)]
fn traverse_outer_walls(
    pieces: &mut Vec<WoodlandMansionPiece>,
    data: &mut PlacementData,
    grid: &SimpleGrid,
    mut grid_direction: MansionDirection,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
) {
    let mut grid_x = start_x;
    let mut grid_y = start_y;
    let start_direction = grid_direction;

    loop {
        if !MansionGrid::is_house(
            grid,
            grid_x + grid_direction.step_x(),
            grid_y + grid_direction.step_z(),
        ) {
            traverse_turn(pieces, data);
            grid_direction = grid_direction.clock_wise();
            if grid_x != end_x || grid_y != end_y || start_direction != grid_direction {
                traverse_wall_piece(pieces, data);
            }
        } else if MansionGrid::is_house(
            grid,
            grid_x + grid_direction.step_x(),
            grid_y + grid_direction.step_z(),
        ) && MansionGrid::is_house(
            grid,
            grid_x + grid_direction.step_x() + grid_direction.counter_clock_wise().step_x(),
            grid_y + grid_direction.step_z() + grid_direction.counter_clock_wise().step_z(),
        ) {
            traverse_inner_turn(data);
            grid_x += grid_direction.step_x();
            grid_y += grid_direction.step_z();
            grid_direction = grid_direction.counter_clock_wise();
        } else {
            grid_x += grid_direction.step_x();
            grid_y += grid_direction.step_z();
            if grid_x != end_x || grid_y != end_y || start_direction != grid_direction {
                traverse_wall_piece(pieces, data);
            }
        }

        if grid_x == end_x && grid_y == end_y && start_direction == grid_direction {
            break;
        }
    }
}
