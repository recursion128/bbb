//! Shared Vanilla `SheetedDecalTextureGenerator` position/normal projection.
//!
//! Block breaking, block-entity crumbling, and special item foil all use the
//! same direction selection and rotation sequence. Keep that math here so the
//! three geometry paths cannot silently diverge.

use glam::{EulerRot, Mat4, Quat, Vec3};

pub(crate) fn sheeted_decal_uv(
    local_position: [f32; 3],
    local_normal: [f32; 3],
    texture_scale: f32,
) -> [f32; 2] {
    let direction = approximate_nearest_direction(Vec3::from_array(local_normal));
    let local_position = Vec3::from_array(local_position);
    let world_pos = Mat4::from_rotation_y(std::f32::consts::PI).transform_vector3(local_position);
    let world_pos =
        Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2).transform_vector3(world_pos);
    let world_pos = direction_rotation(direction) * world_pos;
    [-world_pos.x * texture_scale, -world_pos.y * texture_scale]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecalDirection {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

/// `Direction.getApproximateNearest`: iterate the declaration order and keep
/// the first direction on a dot-product tie.
fn approximate_nearest_direction(normal: Vec3) -> DecalDirection {
    let candidates = [
        (DecalDirection::Down, Vec3::NEG_Y),
        (DecalDirection::Up, Vec3::Y),
        (DecalDirection::North, Vec3::NEG_Z),
        (DecalDirection::South, Vec3::Z),
        (DecalDirection::West, Vec3::NEG_X),
        (DecalDirection::East, Vec3::X),
    ];
    let mut result = DecalDirection::North;
    let mut highest_dot = f32::MIN_POSITIVE;
    for (direction, vector) in candidates {
        let dot = normal.dot(vector);
        if dot > highest_dot {
            highest_dot = dot;
            result = direction;
        }
    }
    result
}

fn direction_rotation(direction: DecalDirection) -> Quat {
    match direction {
        DecalDirection::Down => Quat::from_rotation_x(std::f32::consts::PI),
        DecalDirection::Up => Quat::IDENTITY,
        DecalDirection::North => Quat::from_euler(
            EulerRot::XYZ,
            std::f32::consts::FRAC_PI_2,
            0.0,
            std::f32::consts::PI,
        ),
        DecalDirection::South => Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        DecalDirection::West => Quat::from_euler(
            EulerRot::XYZ,
            std::f32::consts::FRAC_PI_2,
            0.0,
            std::f32::consts::FRAC_PI_2,
        ),
        DecalDirection::East => Quat::from_euler(
            EulerRot::XYZ,
            std::f32::consts::FRAC_PI_2,
            0.0,
            -std::f32::consts::FRAC_PI_2,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_matches_axis_aligned_vanilla_samples() {
        assert_close(
            sheeted_decal_uv([0.2, 0.0, 0.7], [0.0, -1.0, 0.0], 1.0),
            [0.2, -0.7],
        );
        assert_close(
            sheeted_decal_uv([0.2, 1.0, 0.7], [0.0, 1.0, 0.0], 1.0),
            [0.2, 0.7],
        );
        assert_close(
            sheeted_decal_uv([0.25, 0.75, 0.0], [0.0, 0.0, -1.0], 1.0),
            [-0.25, -0.75],
        );
        assert_close(
            sheeted_decal_uv([1.0, 0.5, 0.3], [1.0, 0.0, 0.0], 1.0),
            [0.3, -0.5],
        );
    }

    #[test]
    fn nearest_direction_uses_vanilla_declaration_order_for_ties() {
        assert_eq!(
            approximate_nearest_direction(Vec3::new(1.0, 1.0, 0.0)),
            DecalDirection::Up
        );
        assert_eq!(
            approximate_nearest_direction(Vec3::new(1.0, 0.0, 1.0)),
            DecalDirection::South
        );
    }

    fn assert_close(actual: [f32; 2], expected: [f32; 2]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 1.0e-5, "{actual} != {expected}");
        }
    }
}
