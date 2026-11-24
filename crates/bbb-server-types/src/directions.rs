use bbb_protocol::packets::{BlockPos, Direction, Vec3d};

// Direction.java:291-293.
#[must_use]
pub fn horizontal_direction(y_rot: f32) -> Option<Direction> {
    if !y_rot.is_finite() {
        return None;
    }
    Some(match ((f64::from(y_rot) / 90.0 + 0.5).floor() as i32) & 3 {
        0 => Direction::South,
        1 => Direction::West,
        2 => Direction::North,
        3 => Direction::East,
        _ => unreachable!("masked horizontal direction"),
    })
}

/// `Direction.getYRot` (`Direction.java:134-141`).
#[must_use]
pub const fn facing_y_rot(direction: Direction) -> Option<f32> {
    match direction {
        Direction::North => Some(180.0),
        Direction::South => Some(0.0),
        Direction::West => Some(90.0),
        Direction::East => Some(-90.0),
        Direction::Down | Direction::Up => None,
    }
}

/// `Direction.getOpposite()` (Direction.java:175-177; the opposite indices come
/// from the second constructor argument of the enum constant table at
/// Direction.java:33-38).
#[must_use]
pub fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

/// `Direction.getClockWise()` (Direction.java:195-203), the quarter turn about
/// the Y axis.
///
/// Vanilla's throws for `UP` and `DOWN` (`:200`) because there is no Y-rotated
/// facing of a vertical direction. The one caller here,
/// `DiodeBlock.getAlternateSignal` (DiodeBlock.java:139), only ever hands it
/// `HorizontalDirectionalBlock.FACING`, which is horizontal by construction
/// (BlockStateProperties.java:57); the two vertical arms answer the direction
/// unchanged rather than panicking, which is the fail-closed reading — a diode
/// with a vertical facing would read one of its own faces as a side input, not
/// take the server down.
#[must_use]
pub fn clock_wise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::East,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        Direction::East => Direction::South,
        Direction::Up | Direction::Down => direction,
    }
}

/// `Direction.getCounterClockWise()` (Direction.java:245-253), the other
/// quarter turn about the Y axis, with the same vertical reading
/// [`clock_wise`] documents.
#[must_use]
pub fn counter_clock_wise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::West,
        Direction::South => Direction::East,
        Direction::West => Direction::South,
        Direction::East => Direction::North,
        Direction::Up | Direction::Down => direction,
    }
}

/// `BlockPos.relative(Direction)` (BlockPos.java:198-200): adding one unit
/// vector (the `Vec3i` of Direction.java:33-38). Java's `int` addition wraps
/// silently, so `wrapping_add` matches it here and a debug build does not panic
/// outside the world border.
#[must_use]
pub fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    let (dx, dy, dz) = match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    };
    BlockPos {
        x: pos.x.wrapping_add(dx),
        y: pos.y.wrapping_add(dy),
        z: pos.z.wrapping_add(dz),
    }
}

#[inline]
#[must_use]
pub const fn step_vec(direction: Direction) -> Vec3d {
    match direction {
        Direction::Down => Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        },
        Direction::Up => Vec3d {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Direction::North => Vec3d {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
        Direction::South => Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        Direction::West => Vec3d {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        },
        Direction::East => Vec3d {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    }
}
