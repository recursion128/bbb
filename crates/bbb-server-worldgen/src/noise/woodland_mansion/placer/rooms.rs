//! Per-floor template-name generators and room-content placement, ported from
//! `WoodlandMansionPieces.FloorRoomCollection`/`First/Second/ThirdFloorRoomCollection`
//! and `MansionPiecePlacer.addRoom1x1/1x2/2x2/2x2Secret`
//! (`WoodlandMansionPieces.java:47-98,902-1108`).

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use super::super::direction::{
    compose_rotation, offset, rotate_block_pos, step_relative, MansionDirection,
};
use super::super::{qualify_template, WoodlandMansionPiece};
use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_pieces::JigsawRotation;

/// `ThirdFloorRoomCollection` has no overrides over `SecondFloorRoomCollection`
/// (`WoodlandMansionPieces.java:1117-1152,1196-1197`), so both floors share one
/// arm here.
#[derive(Clone, Copy)]
pub enum FloorRoomKind {
    First,
    SecondOrThird,
}

impl FloorRoomKind {
    pub fn for_floor(floor_num: usize) -> Self {
        if floor_num == 0 {
            Self::First
        } else {
            Self::SecondOrThird
        }
    }

    pub fn get_1x1(self, random: &mut dyn RandomSource) -> String {
        match self {
            Self::First => format!("1x1_a{}", random.next_int(5) + 1),
            Self::SecondOrThird => format!("1x1_b{}", random.next_int(5) + 1),
        }
    }

    pub fn get_1x1_secret(self, random: &mut dyn RandomSource) -> String {
        format!("1x1_as{}", random.next_int(4) + 1)
    }

    pub fn get_1x2_side_entrance(
        self,
        random: &mut dyn RandomSource,
        is_stairs_room: bool,
    ) -> String {
        match self {
            Self::First => format!("1x2_a{}", random.next_int(9) + 1),
            Self::SecondOrThird => {
                if is_stairs_room {
                    "1x2_c_stairs".to_string()
                } else {
                    format!("1x2_c{}", random.next_int(4) + 1)
                }
            }
        }
    }

    pub fn get_1x2_front_entrance(
        self,
        random: &mut dyn RandomSource,
        is_stairs_room: bool,
    ) -> String {
        match self {
            Self::First => format!("1x2_b{}", random.next_int(5) + 1),
            Self::SecondOrThird => {
                if is_stairs_room {
                    "1x2_d_stairs".to_string()
                } else {
                    format!("1x2_d{}", random.next_int(5) + 1)
                }
            }
        }
    }

    pub fn get_1x2_secret(self, random: &mut dyn RandomSource) -> String {
        match self {
            Self::First => format!("1x2_s{}", random.next_int(2) + 1),
            // `nextInt(1)` always returns 0 but still consumes a draw
            // (`WoodlandMansionPieces.java:1140`).
            Self::SecondOrThird => format!("1x2_se{}", random.next_int(1) + 1),
        }
    }

    pub fn get_2x2(self, random: &mut dyn RandomSource) -> String {
        match self {
            Self::First => format!("2x2_a{}", random.next_int(4) + 1),
            Self::SecondOrThird => format!("2x2_b{}", random.next_int(5) + 1),
        }
    }

    pub fn get_2x2_secret(self) -> &'static str {
        "2x2_s1"
    }
}

/// `StructureTemplate.getZeroPositionWithTransform` with `Mirror.NONE`
/// (`StructureTemplate.java:592-613`).
fn zero_position_with_transform_no_mirror(
    zero_pos: BlockPos,
    rotation: JigsawRotation,
    size_x: i32,
    size_z: i32,
) -> BlockPos {
    let size_x = size_x - 1;
    let size_z = size_z - 1;
    match rotation {
        JigsawRotation::CounterClockwise90 => offset(zero_pos, 0, 0, size_x),
        JigsawRotation::Clockwise90 => offset(zero_pos, size_z, 0, 0),
        JigsawRotation::Clockwise180 => offset(zero_pos, size_x, 0, size_z),
        JigsawRotation::None => zero_pos,
    }
}

/// `MansionPiecePlacer.addRoom1x1` (`WoodlandMansionPieces.java:902-928`).
pub fn add_room_1x1(
    pieces: &mut Vec<WoodlandMansionPiece>,
    room_pos: BlockPos,
    rotation: JigsawRotation,
    door_dir: Option<MansionDirection>,
    rooms: FloorRoomKind,
    random: &mut dyn RandomSource,
) {
    let mut piece_rot = JigsawRotation::None;
    let mut room_type = rooms.get_1x1(random);
    if door_dir != Some(MansionDirection::East) {
        match door_dir {
            Some(MansionDirection::North) => {
                piece_rot = compose_rotation(piece_rot, JigsawRotation::CounterClockwise90)
            }
            Some(MansionDirection::West) => {
                piece_rot = compose_rotation(piece_rot, JigsawRotation::Clockwise180)
            }
            Some(MansionDirection::South) => {
                piece_rot = compose_rotation(piece_rot, JigsawRotation::Clockwise90)
            }
            _ => room_type = rooms.get_1x1_secret(random),
        }
    }

    let orientation =
        zero_position_with_transform_no_mirror(BlockPos { x: 1, y: 0, z: 0 }, piece_rot, 7, 7);
    piece_rot = compose_rotation(piece_rot, rotation);
    let orientation = rotate_block_pos(orientation, rotation);
    let pos = offset(room_pos, orientation.x, 0, orientation.z);
    pieces.push(WoodlandMansionPiece::new(
        qualify_template(&room_type),
        pos,
        piece_rot,
        RuinedPortalMirror::None,
    ));
}

/// `MansionPiecePlacer.addRoom1x2` (`WoodlandMansionPieces.java:930-1048`).
#[allow(clippy::too_many_arguments)]
pub fn add_room_1x2(
    pieces: &mut Vec<WoodlandMansionPiece>,
    room_pos: BlockPos,
    rotation: JigsawRotation,
    room_dir: Option<MansionDirection>,
    door_dir: Option<MansionDirection>,
    rooms: FloorRoomKind,
    is_stairs_room: bool,
    random: &mut dyn RandomSource,
) {
    use MansionDirection::{East, North, South, Up, West};
    let d = (door_dir, room_dir);
    let piece = |pieces: &mut Vec<WoodlandMansionPiece>,
                 pos: BlockPos,
                 rot: JigsawRotation,
                 mirror: RuinedPortalMirror,
                 name: String| {
        pieces.push(WoodlandMansionPiece::new(
            qualify_template(&name),
            pos,
            rot,
            mirror,
        ));
    };

    if d == (Some(East), Some(South)) {
        let pos = step_relative(room_pos, rotation, East, 1);
        piece(
            pieces,
            pos,
            rotation,
            RuinedPortalMirror::None,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(East), Some(North)) {
        let pos = step_relative(
            step_relative(room_pos, rotation, East, 1),
            rotation,
            South,
            6,
        );
        piece(
            pieces,
            pos,
            rotation,
            RuinedPortalMirror::LeftRight,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(West), Some(North)) {
        let pos = step_relative(
            step_relative(room_pos, rotation, East, 7),
            rotation,
            South,
            6,
        );
        let rot = compose_rotation(rotation, JigsawRotation::Clockwise180);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::None,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(West), Some(South)) {
        let pos = step_relative(room_pos, rotation, East, 7);
        piece(
            pieces,
            pos,
            rotation,
            RuinedPortalMirror::FrontBack,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(South), Some(East)) {
        let pos = step_relative(room_pos, rotation, East, 1);
        let rot = compose_rotation(rotation, JigsawRotation::Clockwise90);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::LeftRight,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(South), Some(West)) {
        let pos = step_relative(room_pos, rotation, East, 7);
        let rot = compose_rotation(rotation, JigsawRotation::Clockwise90);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::None,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(North), Some(West)) {
        let pos = step_relative(
            step_relative(room_pos, rotation, East, 7),
            rotation,
            South,
            6,
        );
        let rot = compose_rotation(rotation, JigsawRotation::Clockwise90);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::FrontBack,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(North), Some(East)) {
        let pos = step_relative(
            step_relative(room_pos, rotation, East, 1),
            rotation,
            South,
            6,
        );
        let rot = compose_rotation(rotation, JigsawRotation::CounterClockwise90);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::None,
            rooms.get_1x2_side_entrance(random, is_stairs_room),
        );
    } else if d == (Some(South), Some(North)) {
        let pos = step_relative(
            step_relative(room_pos, rotation, East, 1),
            rotation,
            North,
            8,
        );
        piece(
            pieces,
            pos,
            rotation,
            RuinedPortalMirror::None,
            rooms.get_1x2_front_entrance(random, is_stairs_room),
        );
    } else if d == (Some(North), Some(South)) {
        let pos = step_relative(
            step_relative(room_pos, rotation, East, 7),
            rotation,
            South,
            14,
        );
        let rot = compose_rotation(rotation, JigsawRotation::Clockwise180);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::None,
            rooms.get_1x2_front_entrance(random, is_stairs_room),
        );
    } else if d == (Some(West), Some(East)) {
        let pos = step_relative(room_pos, rotation, East, 15);
        let rot = compose_rotation(rotation, JigsawRotation::Clockwise90);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::None,
            rooms.get_1x2_front_entrance(random, is_stairs_room),
        );
    } else if d == (Some(East), Some(West)) {
        let pos = step_relative(
            step_relative(room_pos, rotation, West, 7),
            rotation,
            South,
            6,
        );
        let rot = compose_rotation(rotation, JigsawRotation::CounterClockwise90);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::None,
            rooms.get_1x2_front_entrance(random, is_stairs_room),
        );
    } else if d == (Some(Up), Some(East)) {
        let pos = step_relative(room_pos, rotation, East, 15);
        let rot = compose_rotation(rotation, JigsawRotation::Clockwise90);
        piece(
            pieces,
            pos,
            rot,
            RuinedPortalMirror::None,
            rooms.get_1x2_secret(random),
        );
    } else if d == (Some(Up), Some(South)) {
        let pos = step_relative(room_pos, rotation, East, 1);
        piece(
            pieces,
            pos,
            rotation,
            RuinedPortalMirror::None,
            rooms.get_1x2_secret(random),
        );
    }
}

/// `MansionPiecePlacer.addRoom2x2` (`WoodlandMansionPieces.java:1050-1098`).
pub fn add_room_2x2(
    pieces: &mut Vec<WoodlandMansionPiece>,
    room_pos: BlockPos,
    rotation: JigsawRotation,
    room_dir: Option<MansionDirection>,
    door_dir: Option<MansionDirection>,
    rooms: FloorRoomKind,
    random: &mut dyn RandomSource,
) {
    use MansionDirection::{East, North, South, West};
    let mut east = 0;
    let mut south = 0;
    let mut rot = rotation;
    let mut mirror = RuinedPortalMirror::None;
    match (door_dir, room_dir) {
        (Some(East), Some(South)) => east = -7,
        (Some(East), Some(North)) => {
            east = -7;
            south = 6;
            mirror = RuinedPortalMirror::LeftRight;
        }
        (Some(North), Some(East)) => {
            east = 1;
            south = 14;
            rot = compose_rotation(rotation, JigsawRotation::CounterClockwise90);
        }
        (Some(North), Some(West)) => {
            east = 7;
            south = 14;
            rot = compose_rotation(rotation, JigsawRotation::CounterClockwise90);
            mirror = RuinedPortalMirror::LeftRight;
        }
        (Some(South), Some(West)) => {
            east = 7;
            south = -8;
            rot = compose_rotation(rotation, JigsawRotation::Clockwise90);
        }
        (Some(South), Some(East)) => {
            east = 1;
            south = -8;
            rot = compose_rotation(rotation, JigsawRotation::Clockwise90);
            mirror = RuinedPortalMirror::LeftRight;
        }
        (Some(West), Some(North)) => {
            east = 15;
            south = 6;
            rot = compose_rotation(rotation, JigsawRotation::Clockwise180);
        }
        (Some(West), Some(South)) => {
            east = 15;
            mirror = RuinedPortalMirror::FrontBack;
        }
        _ => {}
    }

    let pos = step_relative(
        step_relative(room_pos, rotation, East, east),
        rotation,
        South,
        south,
    );
    pieces.push(WoodlandMansionPiece::new(
        qualify_template(&rooms.get_2x2(random)),
        pos,
        rot,
        mirror,
    ));
}

/// `MansionPiecePlacer.addRoom2x2Secret` (`WoodlandMansionPieces.java:1100-1108`).
pub fn add_room_2x2_secret(
    pieces: &mut Vec<WoodlandMansionPiece>,
    room_pos: BlockPos,
    rotation: JigsawRotation,
    rooms: FloorRoomKind,
) {
    let pos = step_relative(room_pos, rotation, MansionDirection::East, 1);
    pieces.push(WoodlandMansionPiece::new(
        qualify_template(rooms.get_2x2_secret()),
        pos,
        rotation,
        RuinedPortalMirror::None,
    ));
}
