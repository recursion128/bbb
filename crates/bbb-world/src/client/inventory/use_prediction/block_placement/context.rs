use bbb_protocol::packets::Direction;

use super::super::LocalBlockUseContext;
use crate::BlockPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DirectionAxis {
    X,
    Y,
    Z,
}

pub(super) fn direction_axis(direction: Direction) -> DirectionAxis {
    match direction {
        Direction::Down | Direction::Up => DirectionAxis::Y,
        Direction::North | Direction::South => DirectionAxis::Z,
        Direction::West | Direction::East => DirectionAxis::X,
    }
}

pub(super) fn direction_axis_name(direction: Direction) -> &'static str {
    match direction_axis(direction) {
        DirectionAxis::X => "x",
        DirectionAxis::Y => "y",
        DirectionAxis::Z => "z",
    }
}

pub(super) fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

pub(super) fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

pub(super) fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    match direction {
        Direction::Down => BlockPos {
            y: pos.y - 1,
            ..pos
        },
        Direction::Up => BlockPos {
            y: pos.y + 1,
            ..pos
        },
        Direction::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        Direction::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        Direction::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        Direction::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}

pub(super) fn clockwise(direction: Direction) -> Option<Direction> {
    match direction {
        Direction::North => Some(Direction::East),
        Direction::South => Some(Direction::West),
        Direction::West => Some(Direction::North),
        Direction::East => Some(Direction::South),
        Direction::Down | Direction::Up => None,
    }
}

pub(super) fn counterclockwise(direction: Direction) -> Option<Direction> {
    match direction {
        Direction::North => Some(Direction::West),
        Direction::South => Some(Direction::East),
        Direction::West => Some(Direction::South),
        Direction::East => Some(Direction::North),
        Direction::Down | Direction::Up => None,
    }
}

pub(super) fn horizontal_direction(y_rot: f32) -> Option<Direction> {
    if !y_rot.is_finite() {
        return None;
    }
    let data_value = ((f64::from(y_rot) / 90.0 + 0.5).floor() as i32) & 3;
    Some(match data_value {
        0 => Direction::South,
        1 => Direction::West,
        2 => Direction::North,
        3 => Direction::East,
        _ => unreachable!("masked horizontal direction"),
    })
}

pub(super) fn rotation_segment(degrees: f32) -> Option<i32> {
    if !degrees.is_finite() {
        return None;
    }
    Some((((degrees * (16.0 / 360.0)) + 0.5).floor() as i32) & 15)
}

pub(super) fn nearest_directions(x_rot: f32, y_rot: f32) -> Option<[Direction; 6]> {
    if !x_rot.is_finite() || !y_rot.is_finite() {
        return None;
    }
    let pitch = x_rot.to_radians();
    let yaw = -y_rot.to_radians();
    let pitch_sin = pitch.sin();
    let pitch_cos = pitch.cos();
    let yaw_sin = yaw.sin();
    let yaw_cos = yaw.cos();
    let x_pos = yaw_sin > 0.0;
    let y_pos = pitch_sin < 0.0;
    let z_pos = yaw_cos > 0.0;
    let x_yaw = yaw_sin.abs();
    let y_mag = pitch_sin.abs();
    let z_yaw = yaw_cos.abs();
    let x_mag = x_yaw * pitch_cos;
    let z_mag = z_yaw * pitch_cos;
    let axis_x = if x_pos {
        Direction::East
    } else {
        Direction::West
    };
    let axis_y = if y_pos {
        Direction::Up
    } else {
        Direction::Down
    };
    let axis_z = if z_pos {
        Direction::South
    } else {
        Direction::North
    };
    Some(if x_yaw > z_yaw {
        if y_mag > x_mag {
            direction_array(axis_y, axis_x, axis_z)
        } else if z_mag > y_mag {
            direction_array(axis_x, axis_z, axis_y)
        } else {
            direction_array(axis_x, axis_y, axis_z)
        }
    } else if y_mag > z_mag {
        direction_array(axis_y, axis_z, axis_x)
    } else if x_mag > y_mag {
        direction_array(axis_z, axis_x, axis_y)
    } else {
        direction_array(axis_z, axis_y, axis_x)
    })
}

pub(super) fn placement_directions(
    x_rot: f32,
    y_rot: f32,
    context: LocalBlockUseContext,
    replace_clicked: bool,
) -> Option<[Direction; 6]> {
    let mut directions = nearest_directions(x_rot, y_rot)?;
    if replace_clicked {
        return Some(directions);
    }
    let preferred = opposite(context.face);
    let index = directions
        .iter()
        .position(|direction| *direction == preferred)?;
    directions[..=index].rotate_right(1);
    Some(directions)
}

pub(super) fn world_click_y(context: LocalBlockUseContext) -> f64 {
    f64::from(context.pos.y) + context.cursor[1]
}

pub(super) fn front_and_top_name(front: Direction, top: Direction) -> Option<&'static str> {
    Some(match (front, top) {
        (Direction::Down, Direction::East) => "down_east",
        (Direction::Down, Direction::North) => "down_north",
        (Direction::Down, Direction::South) => "down_south",
        (Direction::Down, Direction::West) => "down_west",
        (Direction::Up, Direction::East) => "up_east",
        (Direction::Up, Direction::North) => "up_north",
        (Direction::Up, Direction::South) => "up_south",
        (Direction::Up, Direction::West) => "up_west",
        (Direction::West, Direction::Up) => "west_up",
        (Direction::East, Direction::Up) => "east_up",
        (Direction::North, Direction::Up) => "north_up",
        (Direction::South, Direction::Up) => "south_up",
        _ => return None,
    })
}

fn direction_array(axis_1: Direction, axis_2: Direction, axis_3: Direction) -> [Direction; 6] {
    [
        axis_1,
        axis_2,
        axis_3,
        opposite(axis_3),
        opposite(axis_2),
        opposite(axis_1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_direction_matches_vanilla_quadrants_and_boundaries() {
        assert_eq!(horizontal_direction(0.0), Some(Direction::South));
        assert_eq!(horizontal_direction(44.999), Some(Direction::South));
        assert_eq!(horizontal_direction(45.0), Some(Direction::West));
        assert_eq!(horizontal_direction(90.0), Some(Direction::West));
        assert_eq!(horizontal_direction(180.0), Some(Direction::North));
        assert_eq!(horizontal_direction(270.0), Some(Direction::East));
        assert_eq!(horizontal_direction(-90.0), Some(Direction::East));
        assert_eq!(horizontal_direction(f32::NAN), None);
    }

    #[test]
    fn rotation_segment_matches_vanilla_math_round_and_mask() {
        assert_eq!(rotation_segment(0.0), Some(0));
        assert_eq!(rotation_segment(180.0), Some(8));
        assert_eq!(rotation_segment(360.0), Some(0));
        assert_eq!(rotation_segment(-22.5), Some(15));
        assert_eq!(rotation_segment(-11.25), Some(0));
        assert_eq!(rotation_segment(f32::INFINITY), None);
    }

    #[test]
    fn nearest_direction_matches_vanilla_view_axes() {
        assert_eq!(
            nearest_directions(0.0, 0.0).map(|directions| directions[0]),
            Some(Direction::South)
        );
        assert_eq!(
            nearest_directions(0.0, 90.0).map(|directions| directions[0]),
            Some(Direction::West)
        );
        assert_eq!(
            nearest_directions(-90.0, 0.0).map(|directions| directions[0]),
            Some(Direction::Up)
        );
        assert_eq!(
            nearest_directions(90.0, 0.0).map(|directions| directions[0]),
            Some(Direction::Down)
        );
    }

    #[test]
    fn placement_direction_order_only_promotes_the_clicked_face_opposite_for_array_queries() {
        let context = LocalBlockUseContext {
            pos: BlockPos { x: 1, y: 2, z: 3 },
            face: Direction::Up,
            cursor: [0.5, 1.0, 0.5],
            inside: false,
            sequence: 1,
            secondary_use_active: false,
        };
        assert_eq!(
            nearest_directions(0.0, 0.0).map(|directions| directions[0]),
            Some(Direction::South)
        );
        assert_eq!(
            placement_directions(0.0, 0.0, context, false).map(|directions| directions[0]),
            Some(Direction::Down)
        );
        assert_eq!(
            placement_directions(0.0, 0.0, context, true).map(|directions| directions[0]),
            Some(Direction::South)
        );
    }
}
