//! Stronghold piece graph (`StrongholdPieces.java:45-218,221-1764`).

use crate::random::large_feature_random;
use bbb_registry_codec::random_source::RandomSource;

use super::geometry::{
    intersects, make_bounding_box, max_x, max_y, max_z, min_x, min_y, min_z, orient_box, x_span,
    y_span, z_span,
};
use super::{SmallDoorType, StrongholdPiece, StrongholdPieceExtra, StrongholdPieceKind};
use crate::noise::scattered::HorizontalDirection;

/// `Direction.Plane.HORIZONTAL.faces` (`Direction.java:577`).
const HORIZONTAL: [HorizontalDirection; 4] = [
    HorizontalDirection::North,
    HorizontalDirection::East,
    HorizontalDirection::South,
    HorizontalDirection::West,
];

/// `STRONGHOLD_PIECE_WEIGHTS` (`StrongholdPieces.java:45-67`).
const WEIGHTS: [(StrongholdPieceKind, i32, i32); 11] = [
    (StrongholdPieceKind::Straight, 40, 0),
    (StrongholdPieceKind::PrisonHall, 5, 5),
    (StrongholdPieceKind::LeftTurn, 20, 0),
    (StrongholdPieceKind::RightTurn, 20, 0),
    (StrongholdPieceKind::RoomCrossing, 10, 6),
    (StrongholdPieceKind::StraightStairsDown, 5, 5),
    (StrongholdPieceKind::StairsDown, 5, 5),
    (StrongholdPieceKind::FiveCrossing, 5, 4),
    (StrongholdPieceKind::ChestCorridor, 5, 4),
    (StrongholdPieceKind::Library, 10, 2),
    (StrongholdPieceKind::PortalRoom, 20, 1),
];

struct PieceWeight {
    kind: StrongholdPieceKind,
    weight: i32,
    max_place_count: i32,
    place_count: i32,
}

impl PieceWeight {
    /// `PieceWeight.doPlace` plus Library/PortalRoom depth gates
    /// (`StrongholdPieces.java:57-65,744-746`).
    fn do_place(&self, depth: i32) -> bool {
        let base = self.max_place_count == 0 || self.place_count < self.max_place_count;
        match self.kind {
            StrongholdPieceKind::Library => base && depth > 4,
            StrongholdPieceKind::PortalRoom => base && depth > 5,
            _ => base,
        }
    }

    fn is_valid(&self) -> bool {
        self.max_place_count == 0 || self.place_count < self.max_place_count
    }
}

struct Session {
    weights: Vec<PieceWeight>,
    current: Vec<usize>,
    imposed: Option<StrongholdPieceKind>,
    total_weight: i32,
    previous: Option<usize>,
    portal_room: Option<usize>,
    pieces: Vec<StrongholdPiece>,
    pending: Vec<usize>,
    start_min_x: i32,
    start_min_z: i32,
}

impl Session {
    fn reset_pieces(&mut self) {
        self.weights = WEIGHTS
            .iter()
            .map(|&(kind, weight, max_place_count)| PieceWeight {
                kind,
                weight,
                max_place_count,
                place_count: 0,
            })
            .collect();
        self.current = (0..self.weights.len()).collect();
        self.imposed = None;
        self.total_weight = 0;
        self.previous = None;
        self.portal_room = None;
        self.pieces.clear();
        self.pending.clear();
    }

    /// `StrongholdPieces.updatePieceWeight` (`StrongholdPieces.java:84-97`).
    fn update_piece_weight(&mut self) -> bool {
        let mut has_any = false;
        self.total_weight = 0;
        for &index in &self.current {
            let piece = &self.weights[index];
            if piece.max_place_count > 0 && piece.place_count < piece.max_place_count {
                has_any = true;
            }
            self.total_weight = self.total_weight.wrapping_add(piece.weight);
        }
        has_any
    }

    fn find_collision(&self, box_: [i32; 6]) -> Option<usize> {
        self.pieces
            .iter()
            .position(|piece| intersects(piece.bounding_box(), box_))
    }
}

/// `StrongholdStructure.generatePieces` (`StrongholdStructure.java:23-44`).
pub fn generate_pieces(
    level_seed: i64,
    chunk_x: i32,
    chunk_z: i32,
    sea_level: i32,
    min_y: i32,
) -> Vec<StrongholdPiece> {
    let mut session = Session {
        weights: Vec::new(),
        current: Vec::new(),
        imposed: None,
        total_weight: 0,
        previous: None,
        portal_room: None,
        pieces: Vec::new(),
        pending: Vec::new(),
        start_min_x: 0,
        start_min_z: 0,
    };
    let mut tries = 0_i32;
    loop {
        // Retry with a fresh stream until the layout reaches a portal room.
        let mut random =
            large_feature_random(level_seed.wrapping_add(i64::from(tries)), chunk_x, chunk_z);
        tries = tries.wrapping_add(1);
        session.reset_pieces();
        let start = create_start_piece(&mut random, chunk_x, chunk_z);
        session.start_min_x = min_x(start.bounding_box());
        session.start_min_z = min_z(start.bounding_box());
        session.pieces.push(start);
        add_children(0, &mut session, &mut random);
        while !session.pending.is_empty() {
            let pos = random.next_int(session.pending.len() as i32) as usize;
            let index = session.pending.remove(pos);
            add_children(index, &mut session, &mut random);
        }
        move_below_sea_level(&mut session, sea_level, min_y, &mut random, 10);
        if !session.pieces.is_empty() && session.portal_room.is_some() {
            return std::mem::take(&mut session.pieces);
        }
    }
}

/// `StartPiece` (`StrongholdPieces.java:1286-1288`) plus source `StairsDown`
/// (`StrongholdPieces.java:1195-1200`).
fn create_start_piece(
    random: &mut dyn RandomSource,
    chunk_x: i32,
    chunk_z: i32,
) -> StrongholdPiece {
    let direction = HORIZONTAL[random.next_int(4) as usize];
    let west = chunk_x.wrapping_mul(16).wrapping_add(2);
    let north = chunk_z.wrapping_mul(16).wrapping_add(2);
    StrongholdPiece::new(
        StrongholdPieceKind::Start,
        make_bounding_box(west, 64, north, direction, 5, 11, 5),
        direction,
        0,
        SmallDoorType::Opening,
        StrongholdPieceExtra::StairsDown { source: true },
    )
}

fn add_children(index: usize, session: &mut Session, random: &mut dyn RandomSource) {
    let kind = session.pieces[index].kind();
    let extra = session.pieces[index].extra().clone();
    match kind {
        StrongholdPieceKind::Start | StrongholdPieceKind::StairsDown => {
            // `StairsDown.addChildren` (`StrongholdPieces.java:1225-1231`).
            if matches!(extra, StrongholdPieceExtra::StairsDown { source: true }) {
                session.imposed = Some(StrongholdPieceKind::FiveCrossing);
            }
            generate_child_forward(index, session, random, 1, 1);
        }
        StrongholdPieceKind::ChestCorridor
        | StrongholdPieceKind::PrisonHall
        | StrongholdPieceKind::StraightStairsDown => {
            generate_child_forward(index, session, random, 1, 1);
        }
        StrongholdPieceKind::Straight => {
            // `Straight.addChildren` (`StrongholdPieces.java:1329-1338`).
            generate_child_forward(index, session, random, 1, 1);
            if let StrongholdPieceExtra::Straight { left, right } = extra {
                if left {
                    generate_child_left(index, session, random, 1, 2);
                }
                if right {
                    generate_child_right(index, session, random, 1, 2);
                }
            }
        }
        StrongholdPieceKind::LeftTurn => {
            // `LeftTurn.addChildren` (`StrongholdPieces.java:524-530`).
            let orientation = session.pieces[index].orientation();
            if matches!(
                orientation,
                HorizontalDirection::North | HorizontalDirection::East
            ) {
                generate_child_left(index, session, random, 1, 1);
            } else {
                generate_child_right(index, session, random, 1, 1);
            }
        }
        StrongholdPieceKind::RightTurn => {
            // `RightTurn.addChildren` (`StrongholdPieces.java:981-987`).
            let orientation = session.pieces[index].orientation();
            if matches!(
                orientation,
                HorizontalDirection::North | HorizontalDirection::East
            ) {
                generate_child_right(index, session, random, 1, 1);
            } else {
                generate_child_left(index, session, random, 1, 1);
            }
        }
        StrongholdPieceKind::RoomCrossing => {
            generate_child_forward(index, session, random, 4, 1);
            generate_child_left(index, session, random, 1, 4);
            generate_child_right(index, session, random, 1, 4);
        }
        StrongholdPieceKind::FiveCrossing => {
            // `FiveCrossing.addChildren` (`StrongholdPieces.java:412-435`).
            let orientation = session.pieces[index].orientation();
            let (z_off_a, z_off_b) = if matches!(
                orientation,
                HorizontalDirection::West | HorizontalDirection::North
            ) {
                (5, 3)
            } else {
                (3, 5)
            };
            generate_child_forward(index, session, random, 5, 1);
            if let StrongholdPieceExtra::FiveCrossing {
                left_low,
                left_high,
                right_low,
                right_high,
            } = extra
            {
                if left_low {
                    generate_child_left(index, session, random, z_off_a, 1);
                }
                if left_high {
                    generate_child_left(index, session, random, z_off_b, 7);
                }
                if right_low {
                    generate_child_right(index, session, random, z_off_a, 1);
                }
                if right_high {
                    generate_child_right(index, session, random, z_off_b, 7);
                }
            }
        }
        StrongholdPieceKind::PortalRoom => {
            // `PortalRoom.addChildren` (`StrongholdPieces.java:776-779`).
            session.portal_room = Some(index);
        }
        StrongholdPieceKind::Library | StrongholdPieceKind::FillerCorridor => {}
    }
}

/// `StrongholdPiece.generateSmallDoorChildForward` (`StrongholdPieces.java:1555-1613`).
fn generate_child_forward(
    index: usize,
    session: &mut Session,
    random: &mut dyn RandomSource,
    x_off: i32,
    y_off: i32,
) {
    let box_ = session.pieces[index].bounding_box();
    let orientation = session.pieces[index].orientation();
    let depth = session.pieces[index].gen_depth();
    let (foot_x, foot_y, foot_z) = match orientation {
        HorizontalDirection::North => (
            min_x(box_).wrapping_add(x_off),
            min_y(box_).wrapping_add(y_off),
            min_z(box_).wrapping_sub(1),
        ),
        HorizontalDirection::South => (
            min_x(box_).wrapping_add(x_off),
            min_y(box_).wrapping_add(y_off),
            max_z(box_).wrapping_add(1),
        ),
        HorizontalDirection::West => (
            min_x(box_).wrapping_sub(1),
            min_y(box_).wrapping_add(y_off),
            min_z(box_).wrapping_add(x_off),
        ),
        HorizontalDirection::East => (
            max_x(box_).wrapping_add(1),
            min_y(box_).wrapping_add(y_off),
            min_z(box_).wrapping_add(x_off),
        ),
    };
    generate_and_add_piece(session, random, foot_x, foot_y, foot_z, orientation, depth);
}

/// `StrongholdPiece.generateSmallDoorChildLeft` (`StrongholdPieces.java:1615-1673`).
fn generate_child_left(
    index: usize,
    session: &mut Session,
    random: &mut dyn RandomSource,
    y_off: i32,
    z_off: i32,
) {
    let box_ = session.pieces[index].bounding_box();
    let orientation = session.pieces[index].orientation();
    let depth = session.pieces[index].gen_depth();
    let (foot_x, foot_y, foot_z, child_dir) = match orientation {
        HorizontalDirection::North | HorizontalDirection::South => (
            min_x(box_).wrapping_sub(1),
            min_y(box_).wrapping_add(y_off),
            min_z(box_).wrapping_add(z_off),
            HorizontalDirection::West,
        ),
        HorizontalDirection::West | HorizontalDirection::East => (
            min_x(box_).wrapping_add(z_off),
            min_y(box_).wrapping_add(y_off),
            min_z(box_).wrapping_sub(1),
            HorizontalDirection::North,
        ),
    };
    generate_and_add_piece(session, random, foot_x, foot_y, foot_z, child_dir, depth);
}

/// `StrongholdPiece.generateSmallDoorChildRight` (`StrongholdPieces.java:1675-1732`).
fn generate_child_right(
    index: usize,
    session: &mut Session,
    random: &mut dyn RandomSource,
    y_off: i32,
    z_off: i32,
) {
    let box_ = session.pieces[index].bounding_box();
    let orientation = session.pieces[index].orientation();
    let depth = session.pieces[index].gen_depth();
    let (foot_x, foot_y, foot_z, child_dir) = match orientation {
        HorizontalDirection::North | HorizontalDirection::South => (
            max_x(box_).wrapping_add(1),
            min_y(box_).wrapping_add(y_off),
            min_z(box_).wrapping_add(z_off),
            HorizontalDirection::East,
        ),
        HorizontalDirection::West | HorizontalDirection::East => (
            min_x(box_).wrapping_add(z_off),
            min_y(box_).wrapping_add(y_off),
            max_z(box_).wrapping_add(1),
            HorizontalDirection::South,
        ),
    };
    generate_and_add_piece(session, random, foot_x, foot_y, foot_z, child_dir, depth);
}

/// `StrongholdPieces.generateAndAddPiece` (`StrongholdPieces.java:194-218`).
fn generate_and_add_piece(
    session: &mut Session,
    random: &mut dyn RandomSource,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: HorizontalDirection,
    depth: i32,
) {
    if depth > 50 {
        return;
    }
    if (i64::from(foot_x) - i64::from(session.start_min_x)).abs() > 112
        || (i64::from(foot_z) - i64::from(session.start_min_z)).abs() > 112
    {
        return;
    }
    if let Some(piece) = generate_piece_from_small_door(
        session,
        random,
        foot_x,
        foot_y,
        foot_z,
        direction,
        depth + 1,
    ) {
        session.pieces.push(piece);
        let index = session.pieces.len() - 1;
        session.pending.push(index);
    }
}

/// `StrongholdPieces.generatePieceFromSmallDoor` (`StrongholdPieces.java:137-192`).
fn generate_piece_from_small_door(
    session: &mut Session,
    random: &mut dyn RandomSource,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: HorizontalDirection,
    depth: i32,
) -> Option<StrongholdPiece> {
    if !session.update_piece_weight() {
        return None;
    }
    if let Some(imposed) = session.imposed.take() {
        if let Some(piece) = find_and_create(
            imposed, session, random, foot_x, foot_y, foot_z, direction, depth,
        ) {
            return Some(piece);
        }
    }
    let mut num_attempts = 0;
    while num_attempts < 5 {
        num_attempts += 1;
        let mut weight_selection = random.next_int(session.total_weight);
        let current = session.current.clone();
        for index in current {
            weight_selection -= session.weights[index].weight;
            if weight_selection >= 0 {
                continue;
            }
            if !session.weights[index].do_place(depth) || session.previous == Some(index) {
                break;
            }
            if let Some(piece) = find_and_create(
                session.weights[index].kind,
                session,
                random,
                foot_x,
                foot_y,
                foot_z,
                direction,
                depth,
            ) {
                session.weights[index].place_count += 1;
                session.previous = Some(index);
                if !session.weights[index].is_valid() {
                    session.current.retain(|&item| item != index);
                }
                return Some(piece);
            }
        }
    }
    let box_ = filler_find_piece_box(session, random, foot_x, foot_y, foot_z, direction)?;
    if min_y(box_) > 1 {
        Some(create_filler(depth, box_, direction))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn find_and_create(
    kind: StrongholdPieceKind,
    session: &Session,
    random: &mut dyn RandomSource,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: HorizontalDirection,
    depth: i32,
) -> Option<StrongholdPiece> {
    match kind {
        StrongholdPieceKind::Straight => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -1,
            -1,
            5,
            5,
            7,
            |random, box_| create_straight(depth, random, box_, direction),
        ),
        StrongholdPieceKind::PrisonHall => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -1,
            -1,
            9,
            5,
            11,
            |random, box_| {
                create_with_door(
                    StrongholdPieceKind::PrisonHall,
                    depth,
                    random,
                    box_,
                    direction,
                    StrongholdPieceExtra::None,
                )
            },
        ),
        StrongholdPieceKind::LeftTurn => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -1,
            -1,
            5,
            5,
            5,
            |random, box_| {
                create_with_door(
                    StrongholdPieceKind::LeftTurn,
                    depth,
                    random,
                    box_,
                    direction,
                    StrongholdPieceExtra::None,
                )
            },
        ),
        StrongholdPieceKind::RightTurn => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -1,
            -1,
            5,
            5,
            5,
            |random, box_| {
                create_with_door(
                    StrongholdPieceKind::RightTurn,
                    depth,
                    random,
                    box_,
                    direction,
                    StrongholdPieceExtra::None,
                )
            },
        ),
        StrongholdPieceKind::RoomCrossing => {
            try_create(
                session,
                random,
                foot_x,
                foot_y,
                foot_z,
                direction,
                depth,
                -4,
                -1,
                11,
                7,
                11,
                |random, box_| {
                    // Door is drawn before `Type` (`StrongholdPieces.java:1034-1036`).
                    let entry_door = random_small_door(random);
                    let room_type = random.next_int(5);
                    StrongholdPiece::new(
                        StrongholdPieceKind::RoomCrossing,
                        box_,
                        direction,
                        depth,
                        entry_door,
                        StrongholdPieceExtra::RoomCrossing { room_type },
                    )
                },
            )
        }
        StrongholdPieceKind::StraightStairsDown => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -1,
            -7,
            5,
            11,
            8,
            |random, box_| {
                create_with_door(
                    StrongholdPieceKind::StraightStairsDown,
                    depth,
                    random,
                    box_,
                    direction,
                    StrongholdPieceExtra::None,
                )
            },
        ),
        StrongholdPieceKind::StairsDown => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -1,
            -7,
            5,
            11,
            5,
            |random, box_| {
                create_with_door(
                    StrongholdPieceKind::StairsDown,
                    depth,
                    random,
                    box_,
                    direction,
                    StrongholdPieceExtra::StairsDown { source: false },
                )
            },
        ),
        StrongholdPieceKind::FiveCrossing => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -4,
            -3,
            10,
            9,
            11,
            |random, box_| create_five_crossing(depth, random, box_, direction),
        ),
        StrongholdPieceKind::ChestCorridor => try_create(
            session,
            random,
            foot_x,
            foot_y,
            foot_z,
            direction,
            depth,
            -1,
            -1,
            5,
            5,
            7,
            |random, box_| {
                create_with_door(
                    StrongholdPieceKind::ChestCorridor,
                    depth,
                    random,
                    box_,
                    direction,
                    StrongholdPieceExtra::ChestCorridor { chest: false },
                )
            },
        ),
        StrongholdPieceKind::Library => {
            create_library(session, random, foot_x, foot_y, foot_z, direction, depth)
        }
        StrongholdPieceKind::PortalRoom => {
            // `PortalRoom.createPiece` does not consume random (`StrongholdPieces.java:782-787`).
            let box_ = orient_box(foot_x, foot_y, foot_z, -4, -1, 0, 11, 8, 16, direction);
            if is_ok_box(box_) && session.find_collision(box_).is_none() {
                Some(StrongholdPiece::new(
                    StrongholdPieceKind::PortalRoom,
                    box_,
                    direction,
                    depth,
                    SmallDoorType::Opening,
                    StrongholdPieceExtra::PortalRoom { mob: false },
                ))
            } else {
                None
            }
        }
        StrongholdPieceKind::FillerCorridor | StrongholdPieceKind::Start => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn try_create(
    session: &Session,
    random: &mut dyn RandomSource,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: HorizontalDirection,
    _depth: i32,
    off_x: i32,
    off_y: i32,
    width: i32,
    height: i32,
    depth_size: i32,
    create: impl FnOnce(&mut dyn RandomSource, [i32; 6]) -> StrongholdPiece,
) -> Option<StrongholdPiece> {
    let box_ = orient_box(
        foot_x, foot_y, foot_z, off_x, off_y, 0, width, height, depth_size, direction,
    );
    if is_ok_box(box_) && session.find_collision(box_).is_none() {
        Some(create(random, box_))
    } else {
        None
    }
}

/// `Library.createPiece` tries tall 11 then short 6 (`StrongholdPieces.java:601-609`).
fn create_library(
    session: &Session,
    random: &mut dyn RandomSource,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: HorizontalDirection,
    depth: i32,
) -> Option<StrongholdPiece> {
    let tall = orient_box(foot_x, foot_y, foot_z, -4, -1, 0, 14, 11, 15, direction);
    let box_ = if is_ok_box(tall) && session.find_collision(tall).is_none() {
        tall
    } else {
        let short = orient_box(foot_x, foot_y, foot_z, -4, -1, 0, 14, 6, 15, direction);
        if !is_ok_box(short) || session.find_collision(short).is_some() {
            return None;
        }
        short
    };
    Some(create_with_door(
        StrongholdPieceKind::Library,
        depth,
        random,
        box_,
        direction,
        StrongholdPieceExtra::Library {
            tall: y_span(box_) > 6,
        },
    ))
}

fn create_with_door(
    kind: StrongholdPieceKind,
    gen_depth: i32,
    random: &mut dyn RandomSource,
    box_: [i32; 6],
    direction: HorizontalDirection,
    extra: StrongholdPieceExtra,
) -> StrongholdPiece {
    let entry_door = random_small_door(random);
    StrongholdPiece::new(kind, box_, direction, gen_depth, entry_door, extra)
}

fn create_straight(
    gen_depth: i32,
    random: &mut dyn RandomSource,
    box_: [i32; 6],
    direction: HorizontalDirection,
) -> StrongholdPiece {
    let entry_door = random_small_door(random);
    let left = random.next_int(2) == 0;
    let right = random.next_int(2) == 0;
    StrongholdPiece::new(
        StrongholdPieceKind::Straight,
        box_,
        direction,
        gen_depth,
        entry_door,
        StrongholdPieceExtra::Straight { left, right },
    )
}

fn create_five_crossing(
    gen_depth: i32,
    random: &mut dyn RandomSource,
    box_: [i32; 6],
    direction: HorizontalDirection,
) -> StrongholdPiece {
    let entry_door = random_small_door(random);
    let left_low = random.next_boolean();
    let left_high = random.next_boolean();
    let right_low = random.next_boolean();
    let right_high = random.next_int(3) > 0;
    StrongholdPiece::new(
        StrongholdPieceKind::FiveCrossing,
        box_,
        direction,
        gen_depth,
        entry_door,
        StrongholdPieceExtra::FiveCrossing {
            left_low,
            left_high,
            right_low,
            right_high,
        },
    )
}

fn create_filler(
    gen_depth: i32,
    box_: [i32; 6],
    direction: HorizontalDirection,
) -> StrongholdPiece {
    let steps = if matches!(
        direction,
        HorizontalDirection::North | HorizontalDirection::South
    ) {
        z_span(box_)
    } else {
        x_span(box_)
    };
    StrongholdPiece::new(
        StrongholdPieceKind::FillerCorridor,
        box_,
        direction,
        gen_depth,
        SmallDoorType::Opening,
        StrongholdPieceExtra::FillerCorridor { steps },
    )
}

/// `FillerCorridor.findPieceBox` (`StrongholdPieces.java:314-338`).
fn filler_find_piece_box(
    session: &Session,
    _random: &mut dyn RandomSource,
    foot_x: i32,
    foot_y: i32,
    foot_z: i32,
    direction: HorizontalDirection,
) -> Option<[i32; 6]> {
    let box_ = orient_box(foot_x, foot_y, foot_z, -1, -1, 0, 5, 5, 4, direction);
    let collision = session.find_collision(box_)?;
    let collision_box = session.pieces[collision].bounding_box();
    if min_y(collision_box) != min_y(box_) {
        return None;
    }
    for depth in (1..=2).rev() {
        let candidate = orient_box(foot_x, foot_y, foot_z, -1, -1, 0, 5, 5, depth, direction);
        if !intersects(collision_box, candidate) {
            return Some(orient_box(
                foot_x,
                foot_y,
                foot_z,
                -1,
                -1,
                0,
                5,
                5,
                depth + 1,
                direction,
            ));
        }
    }
    None
}

/// `StrongholdPiece.randomSmallDoor` (`StrongholdPieces.java:1539-1552`).
fn random_small_door(random: &mut dyn RandomSource) -> SmallDoorType {
    match random.next_int(5) {
        2 => SmallDoorType::WoodDoor,
        3 => SmallDoorType::Grates,
        4 => SmallDoorType::IronDoor,
        _ => SmallDoorType::Opening,
    }
}

/// `StrongholdPiece.isOkBox` (`StrongholdPieces.java:1735-1737`).
fn is_ok_box(box_: [i32; 6]) -> bool {
    min_y(box_) > 10
}

/// `StructurePiecesBuilder.moveBelowSeaLevel` (`StructurePiecesBuilder.java:32-42`).
fn move_below_sea_level(
    session: &mut Session,
    sea_level: i32,
    min_y: i32,
    random: &mut dyn RandomSource,
    offset: i32,
) {
    let Some(union) = super::geometry::union_boxes(
        &session
            .pieces
            .iter()
            .map(StrongholdPiece::bounding_box)
            .collect::<Vec<_>>(),
    ) else {
        return;
    };
    let max_y_limit = sea_level.wrapping_sub(offset);
    let mut y1_pos = y_span(union).wrapping_add(min_y).wrapping_add(1);
    if y1_pos < max_y_limit {
        y1_pos = y1_pos.wrapping_add(random.next_int(max_y_limit.wrapping_sub(y1_pos)));
    }
    let dy = y1_pos.wrapping_sub(max_y(union));
    for piece in &mut session.pieces {
        piece.offset_y(dy);
    }
}
