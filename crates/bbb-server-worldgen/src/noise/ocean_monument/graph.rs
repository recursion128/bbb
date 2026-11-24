//! MonumentBuilding room graph and in-memory child pieces.
//!
//! Saved starts keep only the OMB envelope; Vanilla rebuilds children on load
//! (`OceanMonumentStructure.regeneratePiecesAfterLoad`,
//! `OceanMonumentStructure.java:55-72`).

use crate::random::WorldgenRandom;
use bbb_protocol::packets::BlockPos;

use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::scattered::{make_bounding_box, HorizontalDirection};

use super::{
    OceanMonumentPiece, OceanMonumentRoomCells, OceanMonumentRoomKind, OceanMonumentRoomPiece,
    RegeneratedMonumentChildren, BUILDING_DEPTH, BUILDING_HEIGHT, BUILDING_MIN_Y, BUILDING_WIDTH,
    DIR_DOWN, DIR_EAST, DIR_NORTH, DIR_SOUTH, DIR_UP, DIR_WEST, HORIZONTAL,
};

const GRID_SIZE: usize = 75;
const GRIDROOM_SOURCE_INDEX: usize = 2;
const GRIDROOM_TOP_CONNECT_INDEX: usize = 52;
const GRIDROOM_LEFTWING_CONNECT_INDEX: usize = 25;
const GRIDROOM_RIGHTWING_CONNECT_INDEX: usize = 29;

#[derive(Clone, Debug)]
struct RoomDefinition {
    index: i32,
    connections: [Option<usize>; 6],
    has_opening: [bool; 6],
    claimed: bool,
    is_source: bool,
    scan_index: i32,
}

impl RoomDefinition {
    fn new(index: i32) -> Self {
        Self {
            index,
            connections: [None; 6],
            has_opening: [false; 6],
            claimed: false,
            is_source: false,
            scan_index: 0,
        }
    }

    fn is_special(&self) -> bool {
        self.index >= 75
    }
}

/// `OceanMonumentPieces.getRoomIndex` (`OceanMonumentPieces.java:1447-1449`).
const fn room_index(room_x: i32, room_y: i32, room_z: i32) -> usize {
    (room_y * 25 + room_z * 5 + room_x) as usize
}

/// `Direction.values` / `getOpposite` / steps (`Direction.java:33-38`).
const DIRS: [(usize, usize, i32, i32, i32); 6] = [
    (DIR_DOWN, DIR_UP, 0, -1, 0),
    (DIR_UP, DIR_DOWN, 0, 1, 0),
    (DIR_NORTH, DIR_SOUTH, 0, 0, -1),
    (DIR_SOUTH, DIR_NORTH, 0, 0, 1),
    (DIR_WEST, DIR_EAST, -1, 0, 0),
    (DIR_EAST, DIR_WEST, 1, 0, 0),
];

pub fn regenerate_monument_children(
    seed: i64,
    chunk_x: i32,
    chunk_z: i32,
    envelope: &OceanMonumentPiece,
) -> RegeneratedMonumentChildren {
    let mut random = crate::random::large_feature_random(seed, chunk_x, chunk_z);
    let _ = HORIZONTAL[random.next_int(HORIZONTAL.len() as i32) as usize];
    let west = envelope.bounding_box()[0];
    let north = envelope.bounding_box()[2];
    generate_monument_children(&mut random, west, north, envelope.orientation())
}

/// `MonumentBuilding` ctor after orientation (`OceanMonumentPieces.java:181-221`).
pub fn generate_monument_children(
    random: &mut WorldgenRandom,
    west: i32,
    north: i32,
    orientation: HorizontalDirection,
) -> RegeneratedMonumentChildren {
    let envelope = make_bounding_box(
        west,
        BUILDING_MIN_Y,
        north,
        orientation,
        BUILDING_WIDTH,
        BUILDING_HEIGHT,
        BUILDING_DEPTH,
    );
    let frame = GeneratedPieceFrame::from_array(envelope, orientation);
    let (rooms, source, core) = generate_room_graph(random);
    let mut rooms = rooms;
    let cells = snapshot_cells(&rooms);
    rooms[source].claimed = true;
    let mut children = Vec::new();
    children.push(room_piece(
        OceanMonumentRoomKind::Entry,
        orientation,
        &rooms[source],
        1,
        1,
        1,
        0,
    ));
    children.push(room_piece(
        OceanMonumentRoomKind::Core,
        orientation,
        &rooms[core],
        2,
        2,
        2,
        0,
    ));

    for index in 0..rooms.len() {
        if rooms[index].claimed || rooms[index].is_special() {
            continue;
        }
        children.push(fit_room(random, orientation, &mut rooms, index));
    }

    let offset = frame.world_pos(9, 0, 22);
    for child in &mut children {
        move_box(&mut child.bounding_box, offset);
    }

    let left = from_corners(frame.world_pos(1, 1, 1), frame.world_pos(23, 8, 21));
    let right = from_corners(frame.world_pos(34, 1, 1), frame.world_pos(56, 8, 21));
    let penthouse = from_corners(frame.world_pos(22, 13, 22), frame.world_pos(35, 17, 35));
    let mut wing_random = random.next_bits(32) as i32;
    children.push(OceanMonumentRoomPiece {
        kind: OceanMonumentRoomKind::Wing,
        bounding_box: left,
        orientation,
        room_index: 1001,
        has_opening: [false; 6],
        connection_present: [false; 6],
        main_design: wing_random & 1,
    });
    wing_random = wing_random.wrapping_add(1);
    children.push(OceanMonumentRoomPiece {
        kind: OceanMonumentRoomKind::Wing,
        bounding_box: right,
        orientation,
        room_index: 1002,
        has_opening: [false; 6],
        connection_present: [false; 6],
        main_design: wing_random & 1,
    });
    children.push(OceanMonumentRoomPiece {
        kind: OceanMonumentRoomKind::Penthouse,
        bounding_box: penthouse,
        orientation,
        room_index: 1003,
        has_opening: [false; 6],
        connection_present: [false; 6],
        main_design: 0,
    });
    RegeneratedMonumentChildren {
        pieces: children,
        cells,
    }
}

fn snapshot_cells(rooms: &[RoomDefinition]) -> OceanMonumentRoomCells {
    OceanMonumentRoomCells::from_pairs(rooms.iter().map(|room| {
        (
            room.index,
            room.has_opening,
            std::array::from_fn(|face| {
                room.connections[face].map(|neighbor| rooms[neighbor].index)
            }),
        )
    }))
}

fn generate_room_graph(random: &mut WorldgenRandom) -> (Vec<RoomDefinition>, usize, usize) {
    let mut rooms = Vec::new();
    let mut grid = [None; GRID_SIZE];
    for x in 0..5 {
        for z in 0..4 {
            let index = room_index(x, 0, z);
            grid[index] = Some(rooms.len());
            rooms.push(RoomDefinition::new(index as i32));
        }
    }
    for x in 0..5 {
        for z in 0..4 {
            let index = room_index(x, 1, z);
            grid[index] = Some(rooms.len());
            rooms.push(RoomDefinition::new(index as i32));
        }
    }
    for x in 1..4 {
        for z in 0..2 {
            let index = room_index(x, 2, z);
            grid[index] = Some(rooms.len());
            rooms.push(RoomDefinition::new(index as i32));
        }
    }
    let source = grid[GRIDROOM_SOURCE_INDEX].expect("fixed monument grid room");

    for x in 0..5 {
        for z in 0..5 {
            for y in 0..3 {
                let Some(pos) = grid[room_index(x, y, z)] else {
                    continue;
                };
                for &(dir, opposite, dx, dy, dz) in &DIRS {
                    let neigh_x = x + dx;
                    let neigh_y = y + dy;
                    let neigh_z = z + dz;
                    if !(0..5).contains(&neigh_x)
                        || !(0..5).contains(&neigh_z)
                        || !(0..3).contains(&neigh_y)
                    {
                        continue;
                    }
                    let Some(neigh) = grid[room_index(neigh_x, neigh_y, neigh_z)] else {
                        continue;
                    };
                    let used = if neigh_z == z { dir } else { opposite };
                    rooms[pos].connections[used] = Some(neigh);
                    rooms[neigh].connections[opposite_of(used)] = Some(pos);
                }
            }
        }
    }

    let roof = rooms.len();
    rooms.push(RoomDefinition::new(1003));
    let left = rooms.len();
    rooms.push(RoomDefinition::new(1001));
    let right = rooms.len();
    rooms.push(RoomDefinition::new(1002));
    let top = grid[GRIDROOM_TOP_CONNECT_INDEX].expect("fixed monument grid room");
    let left_connect = grid[GRIDROOM_LEFTWING_CONNECT_INDEX].expect("fixed monument grid room");
    let right_connect = grid[GRIDROOM_RIGHTWING_CONNECT_INDEX].expect("fixed monument grid room");
    rooms[top].connections[DIR_UP] = Some(roof);
    rooms[roof].connections[DIR_DOWN] = Some(top);
    rooms[left_connect].connections[DIR_SOUTH] = Some(left);
    rooms[left].connections[DIR_NORTH] = Some(left_connect);
    rooms[right_connect].connections[DIR_SOUTH] = Some(right);
    rooms[right].connections[DIR_NORTH] = Some(right_connect);
    rooms[roof].claimed = true;
    rooms[left].claimed = true;
    rooms[right].claimed = true;
    rooms[source].is_source = true;

    let core_x = random.next_int(4);
    let core_grid = room_index(core_x, 0, 2);
    let core = grid[core_grid].expect("fixed monument grid room");
    claim_core(&mut rooms, core);
    rooms[core].claimed = true;

    let mut room_defs: Vec<usize> = grid.iter().flatten().copied().collect();
    update_openings(&mut rooms);
    shuffle(&mut room_defs, random);
    let mut scan = 1;
    for &index in &room_defs {
        let mut close_count = 0;
        let mut attempt_count = 0;
        while close_count < 2 && attempt_count < 5 {
            attempt_count += 1;
            let face = random.next_int(6) as usize;
            if !rooms[index].has_opening[face] {
                continue;
            }
            let opposite = opposite_of(face);
            let Some(neigh) = rooms[index].connections[face] else {
                continue;
            };
            rooms[index].has_opening[face] = false;
            rooms[neigh].has_opening[opposite] = false;
            let first = scan;
            scan += 1;
            if find_source(&mut rooms, index, first) {
                let second = scan;
                scan += 1;
                if find_source(&mut rooms, neigh, second) {
                    close_count += 1;
                    continue;
                }
            }
            rooms[index].has_opening[face] = true;
            rooms[neigh].has_opening[opposite] = true;
        }
    }
    (rooms, source, core)
}

fn claim_core(rooms: &mut [RoomDefinition], core: usize) {
    let east = link(rooms, core, DIR_EAST);
    let north = link(rooms, core, DIR_NORTH);
    let up = link(rooms, core, DIR_UP);
    let east_north = link(rooms, east, DIR_NORTH);
    let east_up = link(rooms, east, DIR_UP);
    let north_up = link(rooms, north, DIR_UP);
    let east_north_up = link(rooms, east_north, DIR_UP);
    rooms[east].claimed = true;
    rooms[north].claimed = true;
    rooms[east_north].claimed = true;
    rooms[up].claimed = true;
    rooms[east_up].claimed = true;
    rooms[north_up].claimed = true;
    rooms[east_north_up].claimed = true;
}

fn update_openings(rooms: &mut [RoomDefinition]) {
    for room in rooms.iter_mut() {
        for face in 0..6 {
            room.has_opening[face] = room.connections[face].is_some();
        }
    }
}

fn find_source(rooms: &mut [RoomDefinition], index: usize, scan: i32) -> bool {
    if rooms[index].is_source {
        return true;
    }
    rooms[index].scan_index = scan;
    for face in 0..6 {
        let Some(neigh) = rooms[index].connections[face] else {
            continue;
        };
        if rooms[index].has_opening[face]
            && rooms[neigh].scan_index != scan
            && find_source(rooms, neigh, scan)
        {
            return true;
        }
    }
    false
}

fn fit_room(
    random: &mut WorldgenRandom,
    orientation: HorizontalDirection,
    rooms: &mut [RoomDefinition],
    index: usize,
) -> OceanMonumentRoomPiece {
    if fit_double_xy(rooms, index) {
        claim_pair(rooms, index, DIR_EAST);
        claim_pair(rooms, index, DIR_UP);
        let east = link(rooms, index, DIR_EAST);
        let up = link(rooms, east, DIR_UP);
        rooms[up].claimed = true;
        return room_piece(
            OceanMonumentRoomKind::DoubleXY,
            orientation,
            &rooms[index],
            2,
            2,
            1,
            0,
        );
    }
    if fit_double_yz(rooms, index) {
        claim_pair(rooms, index, DIR_NORTH);
        claim_pair(rooms, index, DIR_UP);
        let north = link(rooms, index, DIR_NORTH);
        let up = link(rooms, north, DIR_UP);
        rooms[up].claimed = true;
        return room_piece(
            OceanMonumentRoomKind::DoubleYZ,
            orientation,
            &rooms[index],
            1,
            2,
            2,
            0,
        );
    }
    if rooms[index].has_opening[DIR_NORTH] {
        if let Some(north) = rooms[index].connections[DIR_NORTH] {
            if !rooms[north].claimed {
                rooms[index].claimed = true;
                rooms[north].claimed = true;
                return room_piece(
                    OceanMonumentRoomKind::DoubleZ,
                    orientation,
                    &rooms[index],
                    1,
                    1,
                    2,
                    0,
                );
            }
        }
    }
    if rooms[index].has_opening[DIR_EAST] {
        if let Some(east) = rooms[index].connections[DIR_EAST] {
            if !rooms[east].claimed {
                rooms[index].claimed = true;
                rooms[east].claimed = true;
                return room_piece(
                    OceanMonumentRoomKind::DoubleX,
                    orientation,
                    &rooms[index],
                    2,
                    1,
                    1,
                    0,
                );
            }
        }
    }
    if rooms[index].has_opening[DIR_UP] {
        if let Some(up) = rooms[index].connections[DIR_UP] {
            if !rooms[up].claimed {
                rooms[index].claimed = true;
                rooms[up].claimed = true;
                return room_piece(
                    OceanMonumentRoomKind::DoubleY,
                    orientation,
                    &rooms[index],
                    1,
                    2,
                    1,
                    0,
                );
            }
        }
    }
    if !rooms[index].has_opening[DIR_WEST]
        && !rooms[index].has_opening[DIR_EAST]
        && !rooms[index].has_opening[DIR_NORTH]
        && !rooms[index].has_opening[DIR_SOUTH]
        && !rooms[index].has_opening[DIR_UP]
    {
        rooms[index].claimed = true;
        return room_piece(
            OceanMonumentRoomKind::SimpleTop,
            orientation,
            &rooms[index],
            1,
            1,
            1,
            0,
        );
    }
    rooms[index].claimed = true;
    room_piece(
        OceanMonumentRoomKind::Simple,
        orientation,
        &rooms[index],
        1,
        1,
        1,
        random.next_int(3),
    )
}

fn fit_double_xy(rooms: &[RoomDefinition], index: usize) -> bool {
    if !rooms[index].has_opening[DIR_EAST] || !rooms[index].has_opening[DIR_UP] {
        return false;
    }
    let Some(east) = rooms[index].connections[DIR_EAST] else {
        return false;
    };
    let Some(up) = rooms[index].connections[DIR_UP] else {
        return false;
    };
    if rooms[east].claimed || rooms[up].claimed {
        return false;
    }
    rooms[east].has_opening[DIR_UP]
        && rooms[east].connections[DIR_UP].is_some_and(|east_up| !rooms[east_up].claimed)
}

fn fit_double_yz(rooms: &[RoomDefinition], index: usize) -> bool {
    if !rooms[index].has_opening[DIR_NORTH] || !rooms[index].has_opening[DIR_UP] {
        return false;
    }
    let Some(north) = rooms[index].connections[DIR_NORTH] else {
        return false;
    };
    let Some(up) = rooms[index].connections[DIR_UP] else {
        return false;
    };
    if rooms[north].claimed || rooms[up].claimed {
        return false;
    }
    rooms[north].has_opening[DIR_UP]
        && rooms[north].connections[DIR_UP].is_some_and(|north_up| !rooms[north_up].claimed)
}

fn claim_pair(rooms: &mut [RoomDefinition], index: usize, face: usize) {
    rooms[index].claimed = true;
    let neigh = link(rooms, index, face);
    rooms[neigh].claimed = true;
}

fn link(rooms: &[RoomDefinition], index: usize, face: usize) -> usize {
    rooms[index].connections[face].expect("monument grid neighbor")
}

fn room_piece(
    kind: OceanMonumentRoomKind,
    orientation: HorizontalDirection,
    room: &RoomDefinition,
    width: i32,
    height: i32,
    depth: i32,
    main_design: i32,
) -> OceanMonumentRoomPiece {
    OceanMonumentRoomPiece {
        kind,
        bounding_box: room_bounding_box(orientation, room.index, width, height, depth),
        orientation,
        room_index: room.index,
        has_opening: room.has_opening,
        connection_present: std::array::from_fn(|face| room.connections[face].is_some()),
        main_design,
    }
}

/// `OceanMonumentPiece.makeBoundingBox` (`OceanMonumentPieces.java:1470-1493`).
fn room_bounding_box(
    orientation: HorizontalDirection,
    index: i32,
    room_width: i32,
    room_height: i32,
    room_depth: i32,
) -> [i32; 6] {
    let room_x = index % 5;
    let room_z = (index / 5) % 5;
    let room_y = index / 25;
    let mut bounds = make_bounding_box(
        0,
        0,
        0,
        orientation,
        room_width * 8,
        room_height * 4,
        room_depth * 8,
    );
    let (dx, dy, dz) = match orientation {
        HorizontalDirection::North => (room_x * 8, room_y * 4, -(room_z + room_depth) * 8 + 1),
        HorizontalDirection::South => (room_x * 8, room_y * 4, room_z * 8),
        HorizontalDirection::West => (-(room_z + room_depth) * 8 + 1, room_y * 4, room_x * 8),
        HorizontalDirection::East => (room_z * 8, room_y * 4, room_x * 8),
    };
    move_box(
        &mut bounds,
        BlockPos {
            x: dx,
            y: dy,
            z: dz,
        },
    );
    bounds
}

fn move_box(bounds: &mut [i32; 6], offset: BlockPos) {
    bounds[0] = bounds[0].wrapping_add(offset.x);
    bounds[1] = bounds[1].wrapping_add(offset.y);
    bounds[2] = bounds[2].wrapping_add(offset.z);
    bounds[3] = bounds[3].wrapping_add(offset.x);
    bounds[4] = bounds[4].wrapping_add(offset.y);
    bounds[5] = bounds[5].wrapping_add(offset.z);
}

fn from_corners(a: BlockPos, b: BlockPos) -> [i32; 6] {
    [
        a.x.min(b.x),
        a.y.min(b.y),
        a.z.min(b.z),
        a.x.max(b.x),
        a.y.max(b.y),
        a.z.max(b.z),
    ]
}

fn opposite_of(face: usize) -> usize {
    DIRS[face].1
}

/// `Util.shuffle` (`Util.java:1026-1032`).
fn shuffle<T>(values: &mut [T], random: &mut WorldgenRandom) {
    let size = values.len();
    for index in (1..size).rev() {
        let swap_to = random.next_int((index + 1) as i32) as usize;
        values.swap(index, swap_to);
    }
}
