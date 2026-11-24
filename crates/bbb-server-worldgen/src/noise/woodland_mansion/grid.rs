//! Deterministic room-layout grids, ported verbatim from
//! `WoodlandMansionPieces.SimpleGrid` and `WoodlandMansionPieces.MansionGrid`
//! (`WoodlandMansionPieces.java:100-399,1154-1194`).

use bbb_registry_codec::random_source::RandomSource;

use super::direction::{MansionDirection, HORIZONTAL_PLANE};

pub const CLEAR: i32 = 0;
pub const CORRIDOR: i32 = 1;
pub const ROOM: i32 = 2;
pub const START_ROOM: i32 = 3;
const TEST_ROOM: i32 = 4;
pub const BLOCKED: i32 = 5;
pub const ROOM_1X1: i32 = 65536;
pub const ROOM_1X2: i32 = 131072;
pub const ROOM_2X2: i32 = 262144;
pub const ROOM_ORIGIN_FLAG: i32 = 1_048_576;
pub const ROOM_DOOR_FLAG: i32 = 2_097_152;
pub const ROOM_STAIRS_FLAG: i32 = 4_194_304;
pub const ROOM_CORRIDOR_FLAG: i32 = 8_388_608;
pub const ROOM_TYPE_MASK: i32 = 983_040;
pub const ROOM_ID_MASK: i32 = 65_535;

/// `WoodlandMansionPieces.SimpleGrid` (`WoodlandMansionPieces.java:1154-1194`).
/// Java stores `int[width][height]`, i.e. the first index is `x`.
#[derive(Clone, Debug)]
pub struct SimpleGrid {
    cells: Vec<i32>,
    width: i32,
    height: i32,
    value_if_outside: i32,
}

impl SimpleGrid {
    pub fn new(width: i32, height: i32, value_if_outside: i32) -> Self {
        Self {
            cells: vec![0; (width * height) as usize],
            width,
            height,
            value_if_outside,
        }
    }

    pub const fn width(&self) -> i32 {
        self.width
    }

    pub const fn height(&self) -> i32 {
        self.height
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && x < self.width && y >= 0 && y < self.height)
            .then(|| (x * self.height + y) as usize)
    }

    pub fn set(&mut self, x: i32, y: i32, value: i32) {
        if let Some(index) = self.index(x, y) {
            self.cells[index] = value;
        }
    }

    pub fn set_range(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, value: i32) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, value);
            }
        }
    }

    pub fn get(&self, x: i32, y: i32) -> i32 {
        self.index(x, y)
            .map_or(self.value_if_outside, |index| self.cells[index])
    }

    pub fn setif(&mut self, x: i32, y: i32, if_value: i32, value: i32) {
        if self.get(x, y) == if_value {
            self.set(x, y, value);
        }
    }

    pub fn edges_to(&self, x: i32, y: i32, if_value: i32) -> bool {
        self.get(x - 1, y) == if_value
            || self.get(x + 1, y) == if_value
            || self.get(x, y + 1) == if_value
            || self.get(x, y - 1) == if_value
    }
}

/// `WoodlandMansionPieces.MansionGrid` (`WoodlandMansionPieces.java:100-399`).
pub struct MansionGrid {
    pub base_grid: SimpleGrid,
    pub third_floor_grid: SimpleGrid,
    pub floor_rooms: [SimpleGrid; 3],
    pub entrance_x: i32,
    pub entrance_y: i32,
}

impl MansionGrid {
    pub fn new(random: &mut dyn RandomSource) -> Self {
        let entrance_x = 7;
        let entrance_y = 4;
        let mut base_grid = SimpleGrid::new(11, 11, 5);
        base_grid.set_range(
            entrance_x,
            entrance_y,
            entrance_x + 1,
            entrance_y + 1,
            START_ROOM,
        );
        base_grid.set_range(
            entrance_x - 1,
            entrance_y,
            entrance_x - 1,
            entrance_y + 1,
            ROOM,
        );
        base_grid.set_range(
            entrance_x + 2,
            entrance_y - 2,
            entrance_x + 3,
            entrance_y + 3,
            BLOCKED,
        );
        base_grid.set_range(
            entrance_x + 1,
            entrance_y - 2,
            entrance_x + 1,
            entrance_y - 1,
            CORRIDOR,
        );
        base_grid.set_range(
            entrance_x + 1,
            entrance_y + 2,
            entrance_x + 1,
            entrance_y + 3,
            CORRIDOR,
        );
        base_grid.set(entrance_x - 1, entrance_y - 1, CORRIDOR);
        base_grid.set(entrance_x - 1, entrance_y + 2, CORRIDOR);
        base_grid.set_range(0, 0, 11, 1, BLOCKED);
        base_grid.set_range(0, 9, 11, 11, BLOCKED);
        recursive_corridor(
            &mut base_grid,
            entrance_x,
            entrance_y - 2,
            MansionDirection::West,
            6,
            random,
        );
        recursive_corridor(
            &mut base_grid,
            entrance_x,
            entrance_y + 3,
            MansionDirection::West,
            6,
            random,
        );
        recursive_corridor(
            &mut base_grid,
            entrance_x - 2,
            entrance_y - 1,
            MansionDirection::West,
            3,
            random,
        );
        recursive_corridor(
            &mut base_grid,
            entrance_x - 2,
            entrance_y + 2,
            MansionDirection::West,
            3,
            random,
        );

        while clean_edges(&mut base_grid) {}

        let mut floor_room_0 = SimpleGrid::new(11, 11, 5);
        let mut floor_room_1 = SimpleGrid::new(11, 11, 5);
        let mut floor_room_2 = SimpleGrid::new(11, 11, 5);
        identify_rooms(&base_grid, &mut floor_room_0, random);
        identify_rooms(&base_grid, &mut floor_room_1, random);
        floor_room_0.set_range(
            entrance_x + 1,
            entrance_y,
            entrance_x + 1,
            entrance_y + 1,
            ROOM_CORRIDOR_FLAG,
        );
        floor_room_1.set_range(
            entrance_x + 1,
            entrance_y,
            entrance_x + 1,
            entrance_y + 1,
            ROOM_CORRIDOR_FLAG,
        );

        let mut third_floor_grid = SimpleGrid::new(base_grid.width(), base_grid.height(), 5);
        setup_third_floor(
            &base_grid,
            &mut floor_room_1,
            &mut third_floor_grid,
            &mut floor_room_2,
            random,
        );
        identify_rooms(&third_floor_grid, &mut floor_room_2, random);

        Self {
            base_grid,
            third_floor_grid,
            floor_rooms: [floor_room_0, floor_room_1, floor_room_2],
            entrance_x,
            entrance_y,
        }
    }

    /// `MansionGrid.isHouse` (`WoodlandMansionPieces.java:160-163`).
    pub fn is_house(grid: &SimpleGrid, x: i32, y: i32) -> bool {
        matches!(grid.get(x, y), CORRIDOR | ROOM | START_ROOM | TEST_ROOM)
    }

    /// `MansionGrid.isRoomId` (`WoodlandMansionPieces.java:165-167`); the
    /// Vanilla `grid` parameter is unused there too.
    pub fn is_room_id(&self, x: i32, y: i32, floor: usize, room_id: i32) -> bool {
        is_room_id(&self.floor_rooms[floor], x, y, room_id)
    }

    /// `MansionGrid.get1x2RoomDirection` (`WoodlandMansionPieces.java:169-179`);
    /// the Vanilla `grid` parameter is unused there too.
    pub fn get_1x2_room_direction(
        &self,
        x: i32,
        y: i32,
        floor_num: usize,
        room_id: i32,
    ) -> Option<MansionDirection> {
        get_1x2_room_direction(&self.floor_rooms[floor_num], x, y, room_id)
    }
}

fn is_room_id(room_grid: &SimpleGrid, x: i32, y: i32, room_id: i32) -> bool {
    (room_grid.get(x, y) & ROOM_ID_MASK) == room_id
}

fn get_1x2_room_direction(
    room_grid: &SimpleGrid,
    x: i32,
    y: i32,
    room_id: i32,
) -> Option<MansionDirection> {
    HORIZONTAL_PLANE.into_iter().find(|&direction| {
        is_room_id(
            room_grid,
            x + direction.step_x(),
            y + direction.step_z(),
            room_id,
        )
    })
}

/// `MansionGrid.recursiveCorridor` (`WoodlandMansionPieces.java:181-209`).
fn recursive_corridor(
    grid: &mut SimpleGrid,
    x: i32,
    y: i32,
    heading: MansionDirection,
    depth: i32,
    random: &mut dyn RandomSource,
) {
    if depth <= 0 {
        return;
    }
    grid.set(x, y, CORRIDOR);
    grid.setif(x + heading.step_x(), y + heading.step_z(), CLEAR, CORRIDOR);

    for _ in 0..8 {
        let next_dir = MansionDirection::from_2d_data_value(random.next_int(4));
        if next_dir != heading.opposite()
            && (next_dir != MansionDirection::East || !random.next_boolean())
        {
            let nx = x + heading.step_x();
            let ny = y + heading.step_z();
            if grid.get(nx + next_dir.step_x(), ny + next_dir.step_z()) == CLEAR
                && grid.get(nx + next_dir.step_x() * 2, ny + next_dir.step_z() * 2) == CLEAR
            {
                recursive_corridor(
                    grid,
                    x + heading.step_x() + next_dir.step_x(),
                    y + heading.step_z() + next_dir.step_z(),
                    next_dir,
                    depth - 1,
                    random,
                );
                break;
            }
        }
    }

    let cw = heading.clock_wise();
    let ccw = heading.counter_clock_wise();
    grid.setif(x + cw.step_x(), y + cw.step_z(), CLEAR, ROOM);
    grid.setif(x + ccw.step_x(), y + ccw.step_z(), CLEAR, ROOM);
    grid.setif(
        x + heading.step_x() + cw.step_x(),
        y + heading.step_z() + cw.step_z(),
        CLEAR,
        ROOM,
    );
    grid.setif(
        x + heading.step_x() + ccw.step_x(),
        y + heading.step_z() + ccw.step_z(),
        CLEAR,
        ROOM,
    );
    grid.setif(
        x + heading.step_x() * 2,
        y + heading.step_z() * 2,
        CLEAR,
        ROOM,
    );
    grid.setif(x + cw.step_x() * 2, y + cw.step_z() * 2, CLEAR, ROOM);
    grid.setif(x + ccw.step_x() * 2, y + ccw.step_z() * 2, CLEAR, ROOM);
}

/// `MansionGrid.cleanEdges` (`WoodlandMansionPieces.java:211-241`).
fn clean_edges(grid: &mut SimpleGrid) -> bool {
    let mut touched = false;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if grid.get(x, y) != CLEAR {
                continue;
            }
            let direct = i32::from(MansionGrid::is_house(grid, x + 1, y))
                + i32::from(MansionGrid::is_house(grid, x - 1, y))
                + i32::from(MansionGrid::is_house(grid, x, y + 1))
                + i32::from(MansionGrid::is_house(grid, x, y - 1));
            if direct >= 3 {
                grid.set(x, y, ROOM);
                touched = true;
            } else if direct == 2 {
                let diagonal = i32::from(MansionGrid::is_house(grid, x + 1, y + 1))
                    + i32::from(MansionGrid::is_house(grid, x - 1, y + 1))
                    + i32::from(MansionGrid::is_house(grid, x + 1, y - 1))
                    + i32::from(MansionGrid::is_house(grid, x - 1, y - 1));
                if diagonal <= 1 {
                    grid.set(x, y, ROOM);
                    touched = true;
                }
            }
        }
    }
    touched
}

/// `MansionGrid.setupThirdFloor` (`WoodlandMansionPieces.java:243-299`).
/// `floor` is `floorRooms[1]`; `floor_rooms_2` is the not-yet-populated
/// `floorRooms[2]` that only receives the corridor-flag marker here.
fn setup_third_floor(
    base_grid: &SimpleGrid,
    floor: &mut SimpleGrid,
    third_floor_grid: &mut SimpleGrid,
    floor_rooms_2: &mut SimpleGrid,
    random: &mut dyn RandomSource,
) {
    let mut potential_rooms = Vec::new();
    for y in 0..third_floor_grid.height() {
        for x in 0..third_floor_grid.width() {
            let room_data = floor.get(x, y);
            if room_data & ROOM_TYPE_MASK == ROOM_1X2
                && room_data & ROOM_DOOR_FLAG == ROOM_DOOR_FLAG
            {
                potential_rooms.push((x, y));
            }
        }
    }

    if potential_rooms.is_empty() {
        third_floor_grid.set_range(
            0,
            0,
            third_floor_grid.width(),
            third_floor_grid.height(),
            BLOCKED,
        );
        return;
    }

    let (room_x, room_y) = potential_rooms[random.next_int(potential_rooms.len() as i32) as usize];
    let room_data = floor.get(room_x, room_y);
    floor.set(room_x, room_y, room_data | ROOM_STAIRS_FLAG);
    let room_id = room_data & ROOM_ID_MASK;
    let room_dir = get_1x2_room_direction(floor, room_x, room_y, room_id)
        .expect("a ROOM_1x2 door-flagged room always has a same-id 1x2 neighbor");
    let room_end_x = room_x + room_dir.step_x();
    let room_end_y = room_y + room_dir.step_z();

    for y in 0..third_floor_grid.height() {
        for x in 0..third_floor_grid.width() {
            if !MansionGrid::is_house(base_grid, x, y) {
                third_floor_grid.set(x, y, BLOCKED);
            } else if x == room_x && y == room_y {
                third_floor_grid.set(x, y, START_ROOM);
            } else if x == room_end_x && y == room_end_y {
                third_floor_grid.set(x, y, START_ROOM);
                floor_rooms_2.set(x, y, ROOM_CORRIDOR_FLAG);
            }
        }
    }

    let mut potential_corridors = Vec::new();
    for direction in HORIZONTAL_PLANE {
        if third_floor_grid.get(
            room_end_x + direction.step_x(),
            room_end_y + direction.step_z(),
        ) == CLEAR
        {
            potential_corridors.push(direction);
        }
    }

    if potential_corridors.is_empty() {
        third_floor_grid.set_range(
            0,
            0,
            third_floor_grid.width(),
            third_floor_grid.height(),
            BLOCKED,
        );
        floor.set(room_x, room_y, room_data);
    } else {
        let corridor_dir =
            potential_corridors[random.next_int(potential_corridors.len() as i32) as usize];
        recursive_corridor(
            third_floor_grid,
            room_end_x + corridor_dir.step_x(),
            room_end_y + corridor_dir.step_z(),
            corridor_dir,
            4,
            random,
        );
        while clean_edges(third_floor_grid) {}
    }
}

/// `MansionGrid.identifyRooms` (`WoodlandMansionPieces.java:301-398`).
fn identify_rooms(
    from_grid: &SimpleGrid,
    room_grid: &mut SimpleGrid,
    random: &mut dyn RandomSource,
) {
    let mut room_positions = Vec::new();
    for y in 0..from_grid.height() {
        for x in 0..from_grid.width() {
            if from_grid.get(x, y) == ROOM {
                room_positions.push((x, y));
            }
        }
    }

    super::shuffle(&mut room_positions, random);
    let mut room_id = 10;

    for (x, y) in room_positions {
        if room_grid.get(x, y) != CLEAR {
            continue;
        }
        let (mut x0, mut x1, mut y0, mut y1) = (x, x, y, y);
        let mut room_type = ROOM_1X1;
        if room_grid.get(x + 1, y) == CLEAR
            && room_grid.get(x, y + 1) == CLEAR
            && room_grid.get(x + 1, y + 1) == CLEAR
            && from_grid.get(x + 1, y) == ROOM
            && from_grid.get(x, y + 1) == ROOM
            && from_grid.get(x + 1, y + 1) == ROOM
        {
            x1 += 1;
            y1 += 1;
            room_type = ROOM_2X2;
        } else if room_grid.get(x - 1, y) == CLEAR
            && room_grid.get(x, y + 1) == CLEAR
            && room_grid.get(x - 1, y + 1) == CLEAR
            && from_grid.get(x - 1, y) == ROOM
            && from_grid.get(x, y + 1) == ROOM
            && from_grid.get(x - 1, y + 1) == ROOM
        {
            x0 -= 1;
            y1 += 1;
            room_type = ROOM_2X2;
        } else if room_grid.get(x - 1, y) == CLEAR
            && room_grid.get(x, y - 1) == CLEAR
            && room_grid.get(x - 1, y - 1) == CLEAR
            && from_grid.get(x - 1, y) == ROOM
            && from_grid.get(x, y - 1) == ROOM
            && from_grid.get(x - 1, y - 1) == ROOM
        {
            x0 -= 1;
            y0 -= 1;
            room_type = ROOM_2X2;
        } else if room_grid.get(x + 1, y) == CLEAR && from_grid.get(x + 1, y) == ROOM {
            x1 += 1;
            room_type = ROOM_1X2;
        } else if room_grid.get(x, y + 1) == CLEAR && from_grid.get(x, y + 1) == ROOM {
            y1 += 1;
            room_type = ROOM_1X2;
        } else if room_grid.get(x - 1, y) == CLEAR && from_grid.get(x - 1, y) == ROOM {
            x0 -= 1;
            room_type = ROOM_1X2;
        } else if room_grid.get(x, y - 1) == CLEAR && from_grid.get(x, y - 1) == ROOM {
            y0 -= 1;
            room_type = ROOM_1X2;
        }

        let mut door_x = if random.next_boolean() { x0 } else { x1 };
        let mut door_y = if random.next_boolean() { y0 } else { y1 };
        let mut door_flag = ROOM_DOOR_FLAG;
        if !from_grid.edges_to(door_x, door_y, CORRIDOR) {
            door_x = if door_x == x0 { x1 } else { x0 };
            door_y = if door_y == y0 { y1 } else { y0 };
            if !from_grid.edges_to(door_x, door_y, CORRIDOR) {
                door_y = if door_y == y0 { y1 } else { y0 };
                if !from_grid.edges_to(door_x, door_y, CORRIDOR) {
                    door_x = if door_x == x0 { x1 } else { x0 };
                    door_y = if door_y == y0 { y1 } else { y0 };
                    if !from_grid.edges_to(door_x, door_y, CORRIDOR) {
                        door_flag = 0;
                        door_x = x0;
                        door_y = y0;
                    }
                }
            }
        }

        for ry in y0..=y1 {
            for rx in x0..=x1 {
                if rx == door_x && ry == door_y {
                    room_grid.set(rx, ry, ROOM_ORIGIN_FLAG | door_flag | room_type | room_id);
                } else {
                    room_grid.set(rx, ry, room_type | room_id);
                }
            }
        }

        room_id += 1;
    }
}
