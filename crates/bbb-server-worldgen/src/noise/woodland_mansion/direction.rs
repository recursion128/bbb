//! Horizontal direction and rotation-composition helpers used only by the
//! woodland-mansion grid/placer port (`Direction.java:32-38,195-253,287-289`,
//! `Rotation.java:38-84,90-101`, `BlockPos.java:122-226`).

use bbb_protocol::packets::BlockPos;

use crate::noise::structure_pieces::JigsawRotation;

/// `Direction.Plane.HORIZONTAL` plus `UP`, the only values `WoodlandMansionPieces`
/// ever produces (`Direction.java:33-38`). `UP` marks a hidden-room door with no
/// grid step and is never rotated or turned in the ported code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MansionDirection {
    North,
    South,
    West,
    East,
    Up,
}

/// `Direction.Plane.HORIZONTAL` iteration order (`Direction.java:577`).
pub const HORIZONTAL_PLANE: [MansionDirection; 4] = [
    MansionDirection::North,
    MansionDirection::East,
    MansionDirection::South,
    MansionDirection::West,
];

impl MansionDirection {
    pub const fn step_x(self) -> i32 {
        match self {
            Self::West => -1,
            Self::East => 1,
            Self::North | Self::South | Self::Up => 0,
        }
    }

    pub const fn step_z(self) -> i32 {
        match self {
            Self::North => -1,
            Self::South => 1,
            Self::West | Self::East | Self::Up => 0,
        }
    }

    /// `Direction.getOpposite()` (`Direction.java:175-177,245-253`); never
    /// invoked on `Up` by the ported callers.
    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
            Self::Up => Self::Up,
        }
    }

    /// `Direction.getClockWise()` (`Direction.java:195-203`).
    pub const fn clock_wise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
            Self::Up => Self::Up,
        }
    }

    /// `Direction.getCounterClockWise()` (`Direction.java:245-253`).
    pub const fn counter_clock_wise(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::West => Self::South,
            Self::South => Self::East,
            Self::East => Self::North,
            Self::Up => Self::Up,
        }
    }

    /// `Direction.from2DDataValue` (`Direction.java:61-64,287-289`): `BY_2D_DATA`
    /// sorted by `data2d` is `[SOUTH(0), WEST(1), NORTH(2), EAST(3)]`.
    pub fn from_2d_data_value(value: i32) -> Self {
        match value.rem_euclid(4) {
            0 => Self::South,
            1 => Self::West,
            2 => Self::North,
            _ => Self::East,
        }
    }
}

/// `Rotation.rotate(Direction)` (`Rotation.java:90-101`); `Up` has axis Y so it
/// always passes through unchanged, matching every ported call site.
pub fn rotate_direction(direction: MansionDirection, rotation: JigsawRotation) -> MansionDirection {
    if matches!(direction, MansionDirection::Up) {
        return direction;
    }
    match rotation {
        JigsawRotation::None => direction,
        JigsawRotation::Clockwise90 => direction.clock_wise(),
        JigsawRotation::Clockwise180 => direction.opposite(),
        JigsawRotation::CounterClockwise90 => direction.counter_clock_wise(),
    }
}

/// `Rotation.getRotated(Rotation)` (`Rotation.java:38-84`): both sides carry
/// the same declaration-order index (`NONE`=0, `CLOCKWISE_90`=1,
/// `CLOCKWISE_180`=2, `COUNTERCLOCKWISE_90`=3), so composition is index
/// addition mod 4.
pub fn compose_rotation(base: JigsawRotation, applied: JigsawRotation) -> JigsawRotation {
    const ORDER: [JigsawRotation; 4] = [
        JigsawRotation::None,
        JigsawRotation::Clockwise90,
        JigsawRotation::Clockwise180,
        JigsawRotation::CounterClockwise90,
    ];
    let index = |rotation: JigsawRotation| {
        ORDER
            .iter()
            .position(|candidate| *candidate == rotation)
            .unwrap()
    };
    ORDER[(index(base) + index(applied)) % 4]
}

/// `BlockPos.relative(Direction, int)` (`BlockPos.java:202-206`).
pub fn relative(position: BlockPos, direction: MansionDirection, steps: i32) -> BlockPos {
    BlockPos {
        x: position
            .x
            .wrapping_add(direction.step_x().wrapping_mul(steps)),
        y: position.y,
        z: position
            .z
            .wrapping_add(direction.step_z().wrapping_mul(steps)),
    }
}

/// `Rotation.rotate(Direction)` composed with `BlockPos.relative`
/// (`Rotation.java:90-101`, `BlockPos.java:202-206`); this is the
/// `pos.relative(rotation.rotate(Direction.X), n)` idiom used throughout
/// `MansionPiecePlacer`.
pub fn step_relative(
    position: BlockPos,
    rotation: JigsawRotation,
    direction: MansionDirection,
    steps: i32,
) -> BlockPos {
    relative(position, rotate_direction(direction, rotation), steps)
}

/// `BlockPos.above(int)` (`BlockPos.java:154-156`).
pub fn above(position: BlockPos, steps: i32) -> BlockPos {
    BlockPos {
        x: position.x,
        y: position.y.wrapping_add(steps),
        z: position.z,
    }
}

/// `BlockPos.offset(int, int, int)` (`BlockPos.java:122-124`).
pub fn offset(position: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: position.x.wrapping_add(x),
        y: position.y.wrapping_add(y),
        z: position.z.wrapping_add(z),
    }
}

/// `BlockPos.rotate(Rotation)` (`BlockPos.java:219-226`).
pub fn rotate_block_pos(position: BlockPos, rotation: JigsawRotation) -> BlockPos {
    match rotation {
        JigsawRotation::None => position,
        JigsawRotation::Clockwise90 => BlockPos {
            x: position.z.wrapping_neg(),
            y: position.y,
            z: position.x,
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: position.x.wrapping_neg(),
            y: position.y,
            z: position.z.wrapping_neg(),
        },
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: position.z,
            y: position.y,
            z: position.x.wrapping_neg(),
        },
    }
}
