//! Shared entity ground friction, slipperiness, and vehicle float/fluid-falling math.

use crate::collision::{CollisionBounds, CollisionBox};

/// `BlockBehaviour.Properties.friction` default (`BlockBehaviour.java`).
pub const DEFAULT_BLOCK_FRICTION: f32 = 0.6;

/// `LivingEntity.travelInAir` (`LivingEntity.java:2424`) base horizontal drag multiplier.
pub const AIR_HORIZONTAL_FRICTION: f32 = 0.91;

/// `LivingEntity.getFrictionInfluencedSpeed` (`LivingEntity.java:2659`) ground numerator.
pub const SLIPPERINESS_SPEED_NUMERATOR: f32 = 0.216_000_02;

/// `AbstractBoat.getGroundFriction` (`AbstractBoat.java:430`) vertical slice below hull.
pub const BOAT_GROUND_FRICTION_SLICE_DEPTH: f64 = 0.001;

/// `AbstractBoat.getDefaultGravity` (`AbstractBoat.java:526`).
pub const BOAT_DEFAULT_GRAVITY: f64 = 0.04;

/// `LivingEntity.travelInAir` (`LivingEntity.java:2424`) horizontal drag from block friction.
#[inline]
pub const fn air_horizontal_friction(block_friction: f32) -> f32 {
    block_friction * AIR_HORIZONTAL_FRICTION
}

/// `LivingEntity.getFrictionInfluencedSpeed` (`LivingEntity.java:2659`) ground multiplier.
#[inline]
pub fn ground_friction_speed_scale(block_friction: f32) -> f32 {
    SLIPPERINESS_SPEED_NUMERATOR / (block_friction * block_friction * block_friction)
}

/// `LivingEntity.getFrictionInfluencedSpeed` (`LivingEntity.java:2658-2660`) on ground.
#[inline]
pub fn friction_influenced_ground_speed(speed: f32, block_friction: f32) -> f32 {
    speed * ground_friction_speed_scale(block_friction)
}

/// `AbstractBoat.getGroundFriction` (`AbstractBoat.java:428-462`).
///
/// Averages the block friction of every non-lily-pad collision shape intersecting the
/// 1 mm slice below `bounds`. The caller invokes `intersects` with each candidate
/// block-local [`CollisionBox`] at `[x, y, z]` and returns `Some(block_friction)` when
/// intersecting (or `None` for lily pads, unloaded blocks, or non-intersecting shapes).
/// Returns `NaN` when no block shape intersects (`0.0F / 0`), matching Java float
/// division so `> 0.0F` picks `IN_AIR`.
pub fn boat_ground_friction<M: CollisionBounds>(
    bounds: M,
    mut cell: impl FnMut([i32; 3], &mut dyn FnMut(CollisionBox) -> bool) -> Option<f32>,
) -> f32 {
    let sample = CollisionBox {
        min_x: bounds.min_x(),
        min_y: bounds.min_y() - BOAT_GROUND_FRICTION_SLICE_DEPTH,
        min_z: bounds.min_z(),
        max_x: bounds.max_x(),
        max_y: bounds.min_y(),
        max_z: bounds.max_z(),
    };
    if !(sample.min_x.is_finite()
        && sample.min_y.is_finite()
        && sample.min_z.is_finite()
        && sample.max_x.is_finite()
        && sample.max_y.is_finite()
        && sample.max_z.is_finite())
    {
        return f32::NAN;
    }
    let x0 = (sample.min_x.floor() as i32).saturating_sub(1);
    let x1 = (sample.max_x.ceil() as i32).saturating_add(1);
    let y0 = (sample.min_y.floor() as i32).saturating_sub(1);
    let y1 = (sample.max_y.ceil() as i32).saturating_add(1);
    let z0 = (sample.min_z.floor() as i32).saturating_sub(1);
    let z1 = (sample.max_z.ceil() as i32).saturating_add(1);
    let mut friction = 0.0_f32;
    let mut count = 0_u32;
    for x in x0..x1 {
        for z in z0..z1 {
            // `AbstractBoat.java:444-445`: skip corners touching both x and z outer rims.
            let edges = u8::from(x == x0 || x == x1 - 1) + u8::from(z == z0 || z == z1 - 1);
            if edges == 2 {
                continue;
            }
            for y in y0..y1 {
                if edges > 0 && (y == y0 || y == y1 - 1) {
                    continue;
                }
                let offset = [f64::from(x), f64::from(y), f64::from(z)];
                let mut intersects = |shape: CollisionBox| shape.moved(offset).intersects(sample);
                if let Some(block_friction) = cell([x, y, z], &mut intersects) {
                    friction += block_friction;
                    count += 1;
                }
            }
        }
    }
    friction / count as f32
}

/// `AbstractBoat.Status` (`AbstractBoat.java:786-792`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoatPhysicsStatus {
    InWater,
    UnderWater,
    UnderFlowingWater,
    OnLand,
    InAir,
}

/// Inputs to the non-snap branch of `AbstractBoat.floatBoat` (`AbstractBoat.java:543-569`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoatFloatInput {
    pub status: Option<BoatPhysicsStatus>,
    pub position_y: f64,
    pub bb_height: f64,
    pub gravity: f64,
    pub water_level: f64,
    pub land_friction: f32,
    pub has_controlling_player: bool,
}

/// `AbstractBoat.floatBoat` (`AbstractBoat.java:543-569`) buoyancy and friction update.
///
/// Returns `(next_delta_movement, next_delta_rotation, next_land_friction)`.
pub fn boat_float_step(
    delta: [f64; 3],
    delta_rotation: f32,
    input: BoatFloatInput,
) -> ([f64; 3], f32, f32) {
    let mut vertical_speed = -input.gravity;
    let mut buoyancy = 0.0_f64;
    let mut land_friction = input.land_friction;
    let inv_friction = match input.status {
        Some(BoatPhysicsStatus::InWater) => {
            buoyancy = (input.water_level - input.position_y) / input.bb_height;
            0.9_f32
        }
        Some(BoatPhysicsStatus::UnderFlowingWater) => {
            vertical_speed = -7.0e-4;
            0.9_f32
        }
        Some(BoatPhysicsStatus::UnderWater) => {
            buoyancy = f64::from(0.01_f32);
            0.45_f32
        }
        Some(BoatPhysicsStatus::InAir) => 0.9_f32,
        Some(BoatPhysicsStatus::OnLand) => {
            let friction = land_friction;
            if input.has_controlling_player {
                land_friction /= 2.0;
            }
            friction
        }
        None => 0.05_f32,
    };
    let scale = f64::from(inv_friction);
    let mut next_delta = [
        delta[0] * scale,
        delta[1] + vertical_speed,
        delta[2] * scale,
    ];
    let next_delta_rotation = delta_rotation * inv_friction;
    if buoyancy > 0.0 {
        next_delta[1] = (next_delta[1] + buoyancy * (BOAT_DEFAULT_GRAVITY / 0.65)) * 0.75;
    }
    (next_delta, next_delta_rotation, land_friction)
}

/// `LivingEntity.getFluidFallingAdjustedMovement` (`LivingEntity.java:2626-2639`).
pub fn fluid_falling_adjusted_velocity(
    movement_y: f64,
    base_gravity: f64,
    is_falling: bool,
    sprinting: bool,
    step_ticks: f64,
) -> f64 {
    if sprinting {
        return movement_y;
    }
    let gravity_step = (base_gravity / 16.0) * step_ticks;
    if gravity_step == 0.0 {
        return movement_y;
    }
    if is_falling
        && (movement_y - 0.005 * step_ticks).abs() >= 0.003 * step_ticks
        && (movement_y - gravity_step).abs() < 0.003 * step_ticks
    {
        -0.003 * step_ticks
    } else {
        movement_y - gravity_step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boat_ground_friction_averages_intersecting_blocks_and_returns_nan_in_air() {
        let boat = CollisionBox::from_arrays([0.01, 1.0, 0.01], [0.99, 1.5625, 0.99]);
        let stone_friction = boat_ground_friction(boat, |pos, intersects| {
            (pos == [0, 0, 0] && intersects(CollisionBox::UNIT_CUBE)).then_some(0.6_f32)
        });
        assert!((stone_friction - 0.6).abs() < f32::EPSILON);

        let air_friction = boat_ground_friction(boat, |_, _| None);
        assert!(air_friction.is_nan());
    }

    #[test]
    fn slipperiness_helpers_match_vanilla_float_constants() {
        assert_eq!(air_horizontal_friction(0.6), 0.6_f32 * 0.91_f32);
        let expected_scale = 0.216_000_02_f32 / (0.98_f32 * 0.98_f32 * 0.98_f32);
        assert_eq!(ground_friction_speed_scale(0.98), expected_scale);
        assert_eq!(
            friction_influenced_ground_speed(0.1, 0.98),
            0.1_f32 * expected_scale
        );
    }

    #[test]
    fn boat_float_step_handles_land_player_halving_and_water_buoyancy() {
        let (delta, rot, next_land) = boat_float_step(
            [0.5, 0.0, -0.25],
            2.0,
            BoatFloatInput {
                status: Some(BoatPhysicsStatus::OnLand),
                position_y: 64.0,
                bb_height: 0.5625,
                gravity: BOAT_DEFAULT_GRAVITY,
                water_level: 0.0,
                land_friction: 0.6,
                has_controlling_player: true,
            },
        );
        assert_eq!(
            delta,
            [0.5 * f64::from(0.6_f32), -0.04, -0.25 * f64::from(0.6_f32)]
        );
        assert_eq!(rot, 2.0 * 0.6_f32);
        assert_eq!(next_land, 0.3_f32);

        let water_level = f64::from(69.0_f32 + 8.0_f32 / 9.0_f32);
        let (water_delta, _, _) = boat_float_step(
            [0.0, 0.0, 0.0],
            0.0,
            BoatFloatInput {
                status: Some(BoatPhysicsStatus::InWater),
                position_y: 69.5,
                bb_height: f64::from(0.5625_f32),
                gravity: BOAT_DEFAULT_GRAVITY,
                water_level,
                land_friction: 0.0,
                has_controlling_player: false,
            },
        );
        let buoyancy = (water_level - 69.5) / f64::from(0.5625_f32);
        let expected_y = (-0.04 + buoyancy * (0.04 / 0.65)) * 0.75;
        assert_eq!(water_delta[1], expected_y);
    }

    #[test]
    fn fluid_falling_adjusted_velocity_applies_terminal_clamp_and_sprint_bypass() {
        assert_eq!(
            fluid_falling_adjusted_velocity(-0.1, 0.08, true, true, 1.0),
            -0.1
        );
        assert_eq!(
            fluid_falling_adjusted_velocity(0.009, 0.16, true, false, 1.0),
            -0.003
        );
        assert_eq!(
            fluid_falling_adjusted_velocity(-0.006, 0.08, true, false, 1.0),
            -0.006 - 0.08 / 16.0
        );
    }
}
