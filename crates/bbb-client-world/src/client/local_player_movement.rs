use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID, VANILLA_ATTRIBUTE_GRAVITY_ID,
    VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID, VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID,
    VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID,
    VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
    VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
};
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_BLINDNESS_ID, VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID,
    VANILLA_MOB_EFFECT_JUMP_BOOST_ID, VANILLA_MOB_EFFECT_LEVITATION_ID,
    VANILLA_MOB_EFFECT_SLOWNESS_ID, VANILLA_MOB_EFFECT_SLOW_FALLING_ID,
    VANILLA_MOB_EFFECT_SPEED_ID,
};
use bbb_protocol::packets::Vec3d as ProtocolVec3d;

use super::local_player::{LocalPlayerAbilitiesState, LocalPlayerInputState, LocalPlayerPoseState};
use super::local_player_collision::{
    local_player_collides, local_player_collides_with_context,
    world_collision_box_intersects_segment_bounds, LocalPlayerCollisionContext, WorldCollisionBox,
    COLLISION_EPSILON,
};
use super::local_player_fluid::local_player_fluid_contact;
use crate::entities::{
    vanilla_living_entity_type, EntityBlockEffectMovement, EntityClientMovementEffectState,
    EntityClientMovementSoundKind, EntityClientMovementSoundState, LocalPlayerBlockEffectStep,
};
use crate::{
    block_name_is_bed, block_name_is_trapdoor, BlockPos, BlockView, EntityVec3, WorldStore,
};

mod fluid;
mod sounds;
#[cfg(test)]
mod tests;

use self::fluid::advance_local_player_fluid_physics_step;
pub(crate) use self::fluid::local_player_lava_current_push_per_tick;
use self::sounds::advance_local_player_movement_sounds;

pub(super) const LOCAL_INPUT_TURN_DEGREES_PER_UNIT: f32 = 0.15;

const LOCAL_PLAYER_SPRINT_MIN_FOOD_LEVEL: i32 = 7;
const LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE: f64 = 0.1;
const LOCAL_INPUT_DEFAULT_GRAVITY_ATTRIBUTE: f64 = 0.08;
const LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE: f64 = 0.42;
const LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER: f64 = 2.0;
const LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER: f64 = 0.3;
const LOCAL_INPUT_FLY_VERTICAL_SPEED_MULTIPLIER: f64 = 3.0;
const LOCAL_INPUT_FLY_VERTICAL_DAMPING: f64 = 0.6;
const LOCAL_PHYSICS_TICK_SECONDS: f64 = 0.05;
const LOCAL_VERTICAL_FRICTION: f64 = 0.98;
const LOCAL_INPUT_FLUID_SPEED_PER_TICK: f64 = 0.02;
const LOCAL_INPUT_LIQUID_JUMP_VELOCITY_PER_TICK: f64 = 0.04;
const LOCAL_INPUT_WATER_SNEAK_DESCEND_VELOCITY_PER_TICK: f64 = 0.04;
const LOCAL_INPUT_WATER_HORIZONTAL_DRAG: f64 = 0.8;
const LOCAL_INPUT_SPRINTING_WATER_HORIZONTAL_DRAG: f64 = 0.9;
const LOCAL_INPUT_WATER_EFFICIENCY_HORIZONTAL_DRAG_TARGET: f64 = 0.54600006;
const LOCAL_INPUT_WATER_VERTICAL_DRAG: f64 = 0.8;
const LOCAL_INPUT_DOLPHINS_GRACE_WATER_HORIZONTAL_DRAG: f64 = 0.96;
const LOCAL_INPUT_LAVA_HORIZONTAL_DRAG: f64 = 0.5;
const LOCAL_INPUT_LAVA_VERTICAL_DRAG: f64 = 0.8;
const LOCAL_INPUT_LAVA_DEEP_DRAG: f64 = 0.5;
const LOCAL_INPUT_LAVA_GRAVITY_SCALE: f64 = 0.25;
const LOCAL_INPUT_WATER_CURRENT_PUSH_PER_TICK: f64 = bbb_facts::fluid::WATER_CURRENT_SCALE;
const LOCAL_INPUT_LAVA_CURRENT_PUSH_PER_TICK: f64 = bbb_facts::fluid::LAVA_CURRENT_SCALE;
const LOCAL_INPUT_FAST_LAVA_CURRENT_PUSH_PER_TICK: f64 = bbb_facts::fluid::FAST_LAVA_CURRENT_SCALE;
const LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK: f64 = 0.3;
const LOCAL_INPUT_SWIM_LOOK_DOWN_VERTICAL_APPROACH: f64 = 0.085;
const LOCAL_INPUT_SWIM_VERTICAL_APPROACH: f64 = 0.06;
const LOCAL_INPUT_SWIM_LOOK_DOWN_THRESHOLD: f64 = -0.2;
const LOCAL_INPUT_SWIM_HEAD_FLUID_OFFSET: f64 = 0.9;
const LOCAL_PLAYER_FLUID_JUMP_THRESHOLD: f64 = 0.4;
const LOCAL_PLAYER_STEP_HEIGHT: f64 = 0.6;
const SUPPORT_EPSILON: f64 = 1.0e-3;
const EDGE_BACKOFF_STEP: f64 = 0.05;
const VANILLA_HORIZONTAL_COLLISION_EPSILON: f64 = 1.0e-5;
const SPEED_EFFECT_MOVEMENT_SPEED_MULTIPLIER: f64 = 0.2;
const SLOWNESS_EFFECT_MOVEMENT_SPEED_MULTIPLIER: f64 = -0.15;
const JUMP_BOOST_VELOCITY_PER_LEVEL: f64 = 0.1;
const LEVITATION_TARGET_VELOCITY_PER_LEVEL: f64 = 0.05;
const LEVITATION_APPROACH_FACTOR_PER_TICK: f64 = 0.2;
const VANILLA_SPEED_EFFECT_MODIFIER_ID: &str = "minecraft:effect.speed";
// LivingEntity.SPRINTING_MODIFIER_ID
const VANILLA_SPRINTING_MODIFIER_ID: &str = "minecraft:sprinting";
const VANILLA_SLOWNESS_EFFECT_MODIFIER_ID: &str = "minecraft:effect.slowness";
const VANILLA_POWDER_SNOW_SPEED_MODIFIER_ID: &str = "minecraft:powder_snow";
const LOCAL_PLAYER_POWDER_SNOW_BLOCK_NAME: &str = "minecraft:powder_snow";
const LOCAL_PLAYER_HONEY_BLOCK_NAME: &str = "minecraft:honey_block";
const LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE: i32 = 140;
const POWDER_SNOW_MOVEMENT_SPEED_MODIFIER_AT_FULL_FREEZE: f64 = -0.05;
const DEFAULT_BLOCK_SPEED_FACTOR: f64 = 1.0;
#[cfg(test)]
const HONEY_BLOCK_JUMP_FACTOR: f64 = 0.5;
const DEFAULT_BLOCK_JUMP_FACTOR: f64 = 1.0;
const SPRINT_JUMP_HORIZONTAL_IMPULSE: f64 = 0.2;
// Vanilla `LivingEntity.aiStep` held-jump lockout: `this.noJumpDelay = 10`.
const LOCAL_PLAYER_JUMP_DELAY_TICKS: u8 = 10;
// Vanilla `LocalPlayer.modifyInput` base input scale.
const LOCAL_PLAYER_INPUT_SCALE: f64 = 0.98;
// `LivingEntity.getFrictionInfluencedSpeed`: getSpeed() * (0.21600002F / f³).
const LOCAL_PLAYER_GROUND_ACCELERATION_FACTOR: f64 = 0.21600002;
// `Player.getFlyingSpeed` off-vehicle air control while not creative-flying.
const LOCAL_PLAYER_AIR_ACCELERATION: f64 = 0.02;
const LOCAL_PLAYER_SPRINT_AIR_ACCELERATION: f64 = 0.025999999;
// `LivingEntity.travelInAir`: friction = blockFriction * 0.91.
const LOCAL_HORIZONTAL_FRICTION: f64 = 0.91;
// `LivingEntity.updateFallFlyingMovement`: multiply(0.99, 0.98, 0.99).
const LOCAL_PLAYER_FALL_FLYING_HORIZONTAL_FRICTION: f64 = 0.99;
// Vanilla `SPEED_MODIFIER_SPRINTING`: +30% MULTIPLY_TOTAL on movement speed.
const LOCAL_PLAYER_SPRINT_SPEED_MODIFIER: f64 = 1.3;
const LOCAL_PLAYER_CLIMBABLE_MAX_HORIZONTAL_VELOCITY_PER_TICK: f64 = 0.15;
const LOCAL_PLAYER_CLIMBABLE_MAX_DOWNWARD_VELOCITY_PER_TICK: f64 = -0.15;
const LOCAL_PLAYER_CLIMBABLE_UPWARD_VELOCITY_PER_TICK: f64 = 0.2;
const LOCAL_PLAYER_SUFFOCATION_ESCAPE_SPEED: f64 = 0.1;
const LOCAL_PLAYER_AUTO_JUMP_MIN_MOVEMENT_SQUARED: f32 = 0.001;
const LOCAL_PLAYER_AUTO_JUMP_MIN_OBSTACLE_HEIGHT: f32 = 0.5;
const LOCAL_PLAYER_AUTO_JUMP_BASE_HEIGHT: f32 = 1.2;
const LOCAL_PLAYER_AUTO_JUMP_BOOST_HEIGHT_PER_LEVEL: f32 = 0.75;
const LOCAL_PLAYER_AUTO_JUMP_LOOK_AHEAD_TICKS: f32 = 7.0;
const LOCAL_PLAYER_AUTO_JUMP_SEGMENT_HEIGHT: f64 = 0.51;
const LOCAL_PLAYER_AUTO_JUMP_FACING_REJECTION_DOT: f32 = -0.15;
const LOCAL_PLAYER_MINOR_COLLISION_MAX_ANGLE_RADIANS: f32 = 0.13962634;
const LOCAL_PLAYER_BED_BOUNCE_FACTOR: f32 = 0.66;

pub(super) fn integrate_local_player_input_pose_with_world_effects(
    world: &mut WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    dt_seconds: f64,
) -> LocalPlayerPoseState {
    pose = apply_local_player_input_look(pose, input);
    let steps = (dt_seconds.max(0.0) / LOCAL_PHYSICS_TICK_SECONDS)
        .ceil()
        .max(1.0);
    let margin = 16.0
        + steps
            * (pose
                .delta_movement
                .x
                .abs()
                .max(pose.delta_movement.y.abs())
                .max(pose.delta_movement.z.abs())
                + 2.0);
    let entity_collision_targets = world
        .entities
        .client_movement_entity_collision_targets_in_aabb(
            [
                pose.position.x - margin,
                pose.position.y - margin,
                pose.position.z - margin,
            ],
            [
                pose.position.x + margin,
                pose.position.y + margin,
                pose.position.z + margin,
            ],
        );

    let mut remaining_seconds = dt_seconds.max(0.0);
    while remaining_seconds > COLLISION_EPSILON {
        let step_seconds = remaining_seconds.min(LOCAL_PHYSICS_TICK_SECONDS);
        let mut step_input = input;
        if world.consume_local_player_auto_jump_tick() {
            step_input.jump = true;
        }
        // Vanilla `Entity.move` consumes the stuck-speed multiplier the
        // previous tick's `applyEffectsFromBlocks` recorded on the player
        // entity, then the post-travel `aiStep` slot runs the callback with
        // this step's recorded move.
        let stuck_multiplier = (!world.local_player_is_spectator())
            .then(|| {
                world.local_player_id.and_then(|id| {
                    world
                        .entities
                        .consume_block_effect_stuck_speed_multiplier(id)
                })
            })
            .flatten();
        let (next_pose, step_move) = advance_local_player_physics_step(
            world,
            pose,
            step_input,
            step_seconds,
            stuck_multiplier,
            &entity_collision_targets,
        );
        pose = next_pose;
        update_local_player_auto_jump(
            world,
            pose,
            step_input,
            (step_move.to.x - step_move.from.x) as f32,
            (step_move.to.z - step_move.from.z) as f32,
            &entity_collision_targets,
        );
        advance_local_player_movement_sounds(world, pose, step_move);
        world.apply_local_player_block_effects(&mut pose, step_move);
        remaining_seconds -= step_seconds;
    }

    pose
}

impl WorldStore {
    /// Mirrors `LocalPlayer.aiStep` (`LocalPlayer.java:839-841`) when double-tap
    /// Space toggles creative flight on while grounded.
    pub fn local_player_jump_from_ground_for_creative_flight(&mut self, sprinting: bool) -> bool {
        let Some(mut pose) = self.local_player.pose else {
            return false;
        };
        if !pose.on_ground {
            return false;
        }
        apply_local_player_jump_from_ground(self, &mut pose, sprinting);
        self.local_player.pose = Some(pose);
        true
    }
}

pub(super) fn apply_local_player_input_look(
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
) -> LocalPlayerPoseState {
    if input.focused {
        pose.y_rot = wrap_degrees_f32(
            pose.y_rot + input.mouse_delta_x as f32 * LOCAL_INPUT_TURN_DEGREES_PER_UNIT,
        );
        pose.x_rot = (pose.x_rot + input.mouse_delta_y as f32 * LOCAL_INPUT_TURN_DEGREES_PER_UNIT)
            .clamp(-90.0, 90.0);
    }
    pose
}

fn move_local_player_out_of_suffocating_blocks(
    world: &WorldStore,
    pose: &mut LocalPlayerPoseState,
    descending: bool,
) {
    let offset = world.local_player_dimensions(*pose).width * 0.35;
    for (x, z) in [
        (pose.position.x - offset, pose.position.z + offset),
        (pose.position.x - offset, pose.position.z - offset),
        (pose.position.x + offset, pose.position.z - offset),
        (pose.position.x + offset, pose.position.z + offset),
    ] {
        move_local_player_towards_closest_space(world, pose, x, z, descending);
    }
}

fn move_local_player_towards_closest_space(
    world: &WorldStore,
    pose: &mut LocalPlayerPoseState,
    x: f64,
    z: f64,
    descending: bool,
) {
    let pos = BlockPos {
        x: local_player_block_floor(x),
        y: local_player_block_floor(pose.position.y),
        z: local_player_block_floor(z),
    };
    if !world.local_player_suffocates_at(*pose, pos, descending) {
        return;
    }

    let xd = x - f64::from(pos.x);
    let zd = z - f64::from(pos.z);
    let mut closest = f64::MAX;
    let mut escape = None;
    // Exact Vanilla order resolves equal edge distances west, east, north,
    // then south because candidates only replace on a strict improvement.
    for (dx, dz, distance) in [(-1, 0, xd), (1, 0, 1.0 - xd), (0, -1, zd), (0, 1, 1.0 - zd)] {
        let adjacent = BlockPos {
            x: pos.x.saturating_add(dx),
            y: pos.y,
            z: pos.z.saturating_add(dz),
        };
        if distance < closest && !world.local_player_suffocates_at(*pose, adjacent, descending) {
            closest = distance;
            escape = Some((dx, dz));
        }
    }
    match escape {
        Some((dx, 0)) => {
            pose.delta_movement.x = f64::from(dx) * LOCAL_PLAYER_SUFFOCATION_ESCAPE_SPEED;
        }
        Some((0, dz)) => {
            pose.delta_movement.z = f64::from(dz) * LOCAL_PLAYER_SUFFOCATION_ESCAPE_SPEED;
        }
        _ => {}
    }
}

fn update_local_player_auto_jump(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    xa: f32,
    za: f32,
    entity_collision_targets: &[crate::EntityPickTargetState],
) {
    if !can_local_player_auto_jump(world, pose, input) {
        return;
    }

    let move_begin = [pose.position.x, pose.position.y, pose.position.z];
    let move_end = [
        move_begin[0] + f64::from(xa),
        move_begin[1],
        move_begin[2] + f64::from(za),
    ];
    let mut move_x = f64::from(xa);
    let mut move_z = f64::from(za);
    let effective_input = local_player_effective_movement_input(world, input);
    let current_speed = local_player_ground_speed(world, effective_input) as f32;
    let mut move_distance_squared = (move_x * move_x + move_z * move_z) as f32;
    if move_distance_squared <= LOCAL_PLAYER_AUTO_JUMP_MIN_MOVEMENT_SQUARED {
        let raw_x = axis_f32(input.left, input.right);
        let raw_z = axis_f32(input.forward, input.backward);
        let raw_length = (raw_x * raw_x + raw_z * raw_z).sqrt();
        let (input_x, input_z) = if raw_length > 0.0 {
            (raw_x / raw_length, raw_z / raw_length)
        } else {
            (0.0, 0.0)
        };
        let input_x = current_speed * input_x;
        let input_z = current_speed * input_z;
        let yaw = pose.y_rot.to_radians();
        let sin = yaw.sin();
        let cos = yaw.cos();
        move_x = f64::from(input_x * cos - input_z * sin);
        move_z = f64::from(input_z * cos + input_x * sin);
        move_distance_squared = (move_x * move_x + move_z * move_z) as f32;
        if move_distance_squared <= LOCAL_PLAYER_AUTO_JUMP_MIN_MOVEMENT_SQUARED {
            return;
        }
    }

    let inverse_move_distance = move_distance_squared.sqrt().recip();
    let move_direction = [
        move_x * f64::from(inverse_move_distance),
        0.0,
        move_z * f64::from(inverse_move_distance),
    ];
    let yaw = pose.y_rot.to_radians();
    let pitch = pose.x_rot.to_radians();
    let facing_x = -yaw.sin() * pitch.cos();
    let facing_z = yaw.cos() * pitch.cos();
    let facing_dot =
        (f64::from(facing_x) * move_direction[0] + f64::from(facing_z) * move_direction[2]) as f32;
    if facing_dot < LOCAL_PLAYER_AUTO_JUMP_FACING_REJECTION_DOT {
        return;
    }

    let mut ceiling_pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y + world.local_player_dimensions(pose).height),
        z: local_player_block_floor(pose.position.z),
    };
    if world
        .local_player_block_collision_shape_max_y(pose, ceiling_pos, input.sneak)
        .is_some()
    {
        return;
    }
    ceiling_pos.y = ceiling_pos.y.saturating_add(1);
    if world
        .local_player_block_collision_shape_max_y(pose, ceiling_pos, input.sneak)
        .is_some()
    {
        return;
    }

    let mut jump_height = LOCAL_PLAYER_AUTO_JUMP_BASE_HEIGHT;
    if let Some(amplifier) = local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
    {
        jump_height +=
            (amplifier.saturating_add(1) as f32) * LOCAL_PLAYER_AUTO_JUMP_BOOST_HEIGHT_PER_LEVEL;
    }
    let look_ahead_distance = (current_speed * LOCAL_PLAYER_AUTO_JUMP_LOOK_AHEAD_TICKS)
        .max(inverse_move_distance.recip());
    let segment_end = [
        move_end[0] + move_direction[0] * f64::from(look_ahead_distance),
        move_end[1],
        move_end[2] + move_direction[2] * f64::from(look_ahead_distance),
    ];
    let dimensions = world.local_player_dimensions(pose);
    let player_width = dimensions.width;
    let test_bounds = WorldCollisionBox {
        min: [
            move_begin[0].min(segment_end[0]) - player_width,
            move_begin[1],
            move_begin[2].min(segment_end[2]) - player_width,
        ],
        max: [
            move_begin[0].max(segment_end[0]) + player_width,
            move_begin[1] + dimensions.height,
            move_begin[2].max(segment_end[2]) + player_width,
        ],
    };
    let segment_begin = [
        move_begin[0],
        move_begin[1] + LOCAL_PLAYER_AUTO_JUMP_SEGMENT_HEIGHT,
        move_begin[2],
    ];
    let segment_end = [
        segment_end[0],
        segment_end[1] + LOCAL_PLAYER_AUTO_JUMP_SEGMENT_HEIGHT,
        segment_end[2],
    ];
    let right_direction = [-move_direction[2], 0.0, move_direction[0]];
    let right_offset = [
        right_direction[0] * player_width * 0.5,
        0.0,
        right_direction[2] * player_width * 0.5,
    ];
    let left_segment_begin = [
        segment_begin[0] - right_offset[0],
        segment_begin[1],
        segment_begin[2] - right_offset[2],
    ];
    let left_segment_end = [
        segment_end[0] - right_offset[0],
        segment_end[1],
        segment_end[2] - right_offset[2],
    ];
    let right_segment_begin = [
        segment_begin[0] + right_offset[0],
        segment_begin[1],
        segment_begin[2] + right_offset[2],
    ];
    let right_segment_end = [
        segment_end[0] + right_offset[0],
        segment_end[1],
        segment_end[2] + right_offset[2],
    ];

    let collisions = world.local_player_auto_jump_collision_boxes(
        pose,
        test_bounds,
        input.sneak,
        entity_collision_targets,
    );
    let mut obstacle_height = None;
    for collision_box in collisions {
        if !world_collision_box_intersects_segment_bounds(
            collision_box,
            left_segment_begin,
            left_segment_end,
        ) && !world_collision_box_intersects_segment_bounds(
            collision_box,
            right_segment_begin,
            right_segment_end,
        ) {
            continue;
        }
        let mut height = collision_box.max[1] as f32;
        let obstacle_pos = BlockPos {
            x: local_player_block_floor((collision_box.min[0] + collision_box.max[0]) * 0.5),
            y: local_player_block_floor((collision_box.min[1] + collision_box.max[1]) * 0.5),
            z: local_player_block_floor((collision_box.min[2] + collision_box.max[2]) * 0.5),
        };
        let max_steps = world.dimension.height.max(1);
        let mut steps = 1_i32;
        while steps <= max_steps && (steps as f32) < jump_height {
            let above_pos = BlockPos {
                y: obstacle_pos.y.saturating_add(steps),
                ..obstacle_pos
            };
            if let Some(shape_max_y) =
                world.local_player_block_collision_shape_max_y(pose, above_pos, input.sneak)
            {
                height = shape_max_y as f32;
                if height - pose.position.y as f32 > jump_height {
                    return;
                }
            }
            if steps > 1 {
                ceiling_pos.y = ceiling_pos.y.saturating_add(1);
                if world
                    .local_player_block_collision_shape_max_y(pose, ceiling_pos, input.sneak)
                    .is_some()
                {
                    return;
                }
            }
            steps = steps.saturating_add(1);
        }
        obstacle_height = Some(height);
        break;
    }

    if let Some(obstacle_height) = obstacle_height {
        let delta_y = obstacle_height - pose.position.y as f32;
        if delta_y > LOCAL_PLAYER_AUTO_JUMP_MIN_OBSTACLE_HEIGHT && delta_y <= jump_height {
            world.arm_local_player_auto_jump();
        }
    }
}

fn can_local_player_auto_jump(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
) -> bool {
    world.local_player_auto_jump_enabled()
        && !world.local_player_auto_jump_is_armed()
        && pose.on_ground
        && !input.sneak
        && world.local_player_vehicle_id().is_none()
        && input.focused
        && (axis_f32(input.left, input.right) != 0.0
            || axis_f32(input.forward, input.backward) != 0.0)
        && local_player_block_jump_factor(world, pose) >= DEFAULT_BLOCK_JUMP_FACTOR
}

/// Applies the narrow `Entity.move(MoverType.SELF, 0, y, 0)` used by
/// `TridentItem.releaseUsing` for a grounded Riptide launch.
pub(super) fn move_local_player_self_upward(
    world: &mut WorldStore,
    mut pose: LocalPlayerPoseState,
    requested_y: f64,
) -> LocalPlayerPoseState {
    let margin = requested_y.abs() + 4.0;
    let entity_collision_targets = world
        .entities
        .client_movement_entity_collision_targets_in_aabb(
            [
                pose.position.x - 4.0,
                pose.position.y - margin,
                pose.position.z - 4.0,
            ],
            [
                pose.position.x + 4.0,
                pose.position.y + margin,
                pose.position.z + 4.0,
            ],
        );
    let stuck_multiplier = (!world.local_player_is_spectator())
        .then(|| {
            world.local_player_id.and_then(|id| {
                world
                    .entities
                    .consume_block_effect_stuck_speed_multiplier(id)
            })
        })
        .flatten();
    let requested_y = stuck_multiplier.map_or(requested_y, |stuck| requested_y * stuck.y);
    if stuck_multiplier.is_some() {
        pose.delta_movement = ProtocolVec3d::default();
    }
    let movement = world.clip_local_player_collision_movement(
        pose,
        [0.0, requested_y, 0.0],
        0.0,
        false,
        &entity_collision_targets,
    );
    pose.position.y += movement[1];
    pose.on_ground = false;
    pose.horizontal_collision = false;
    pose.minor_horizontal_collision = false;
    pose
}

/// Advances the client-private Riptide spin state in Vanilla's post-travel
/// `LivingEntity.aiStep` slot. Touch damage remains server-authoritative; the
/// client still ends the private timer and reflects velocity on a living hit.
pub(super) fn advance_local_player_auto_spin_attack_after_move(
    world: &mut WorldStore,
    old_pose: LocalPlayerPoseState,
    mut pose: LocalPlayerPoseState,
) -> LocalPlayerPoseState {
    if !world.advance_local_player_auto_spin_attack_tick() {
        return pose;
    }

    let old_bounds = world.local_player_bounds(old_pose);
    let current_bounds = world.local_player_bounds(pose);
    let query_min = [
        old_bounds.min[0].min(current_bounds.min[0]),
        old_bounds.min[1].min(current_bounds.min[1]),
        old_bounds.min[2].min(current_bounds.min[2]),
    ];
    let query_max = [
        old_bounds.max[0].max(current_bounds.max[0]),
        old_bounds.max[1].max(current_bounds.max[1]),
        old_bounds.max[2].max(current_bounds.max[2]),
    ];
    let living_hit = world.local_player_id.is_some_and(|local_player_id| {
        world
            .entities
            .any_entity_bounds_intersecting_except(
                query_min,
                query_max,
                Some(local_player_id),
                vanilla_living_entity_type,
            )
            .unwrap_or(false)
    });
    if living_hit {
        world.stop_local_player_auto_spin_attack();
        pose.delta_movement.x *= -0.2;
        pose.delta_movement.y *= -0.2;
        pose.delta_movement.z *= -0.2;
    } else if pose.horizontal_collision {
        world.stop_local_player_auto_spin_attack();
    }
    pose
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPlayerBodyPose {
    Standing,
    Crouching,
    Swimming,
}

fn advance_local_player_physics_step(
    world: &mut WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    step_seconds: f64,
    stuck_multiplier: Option<EntityVec3>,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> (LocalPlayerPoseState, EntityBlockEffectMovement) {
    // Vanilla `Player.isImmobile` (`Player.java:870-872`) includes `isSleeping()`:
    // a sleeping player cannot accept movement/jump input or slide out of bed.
    if world
        .local_player_id
        .and_then(|id| world.entities.sleeping_pos(id))
        .is_some()
    {
        pose.delta_movement = ProtocolVec3d::default();
        pose.fall_distance = 0.0;
        let origin = EntityVec3 {
            x: pose.position.x,
            y: pose.position.y,
            z: pose.position.z,
        };
        return (
            pose,
            EntityBlockEffectMovement::clipped(origin, origin, EntityVec3::default()),
        );
    }
    let input = local_player_effective_movement_input(world, input);
    // Vanilla `LivingEntity.aiStep` decrements `noJumpDelay` at the top of
    // every tick and clears it outright while the jump input is released.
    if pose.no_jump_delay_ticks > 0 {
        pose.no_jump_delay_ticks -= 1;
    }
    if !(input.focused && input.jump) {
        pose.no_jump_delay_ticks = 0;
    }
    // Vanilla `LivingEntity.aiStep` velocity deadzone (`LivingEntity.java:2988-3007`).
    if pose.delta_movement.x * pose.delta_movement.x + pose.delta_movement.z * pose.delta_movement.z
        < 9.0e-6
    {
        pose.delta_movement.x = 0.0;
        pose.delta_movement.z = 0.0;
    }
    if pose.delta_movement.y.abs() < 0.003 {
        pose.delta_movement.y = 0.0;
    }
    let flying = local_player_flying_abilities(world);
    if world.local_player_is_spectator() {
        // `Player.tick` sets `noPhysics` and clears grounded state before
        // `LivingEntity.aiStep`; `Entity.move` then preserves the full
        // requested vector and clears its collision flags.
        pose.on_ground = false;
    }
    world.local_player.swimming = local_player_should_swim(world, pose, input);
    pose = local_player_update_body_pose(world, pose, input, flying, entity_collision_targets);
    if !world.local_player_is_spectator() {
        move_local_player_out_of_suffocating_blocks(world, &mut pose, input.sneak);
    }
    // Vanilla `LivingEntity.aiStep` (`LivingEntity.java:3057-3059`): Slow Falling
    // and Levitation reset accumulated fall distance before travel.
    if local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_SLOW_FALLING_ID).is_some()
        || local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_LEVITATION_ID).is_some()
    {
        pose.fall_distance = 0.0;
    }
    let initial_fluid_contact = local_player_fluid_contact(world, pose);
    if flying.is_none() && (initial_fluid_contact.in_water() || initial_fluid_contact.in_lava()) {
        return advance_local_player_fluid_physics_step(
            world,
            pose,
            input,
            step_seconds,
            initial_fluid_contact,
            stuck_multiplier,
            entity_collision_targets,
        );
    }
    let step_ticks = step_seconds / LOCAL_PHYSICS_TICK_SECONDS;
    if let Some(abilities) = flying {
        // Vanilla `Player.aiStep` resets the fall distance on every flying
        // tick, and `LocalPlayer.aiStep` adds the vertical fly input to the
        // carried delta before the shared `LivingEntity.aiStep` chain runs.
        pose.fall_distance = 0.0;
        let vertical_input = if input.focused {
            axis(input.jump, input.sneak)
        } else {
            0.0
        };
        if vertical_input != 0.0 {
            pose.delta_movement.y += vertical_input
                * f64::from(abilities.flying_speed).max(0.0)
                * LOCAL_INPUT_FLY_VERTICAL_SPEED_MULTIPLIER
                * step_ticks;
        }
    }

    // `Player.onClimbable` reports false while creative-flying, which also
    // disables the climbable clamp and the wall-climb impulse below.
    let climbable_before = if flying.is_none() {
        local_player_climbable_kind(world, pose)
    } else {
        None
    };
    let on_climbable_before = climbable_before.is_some();
    let powder_snow_climb_out_before = world.local_player_can_walk_on_powder_snow()
        && local_player_inside_powder_snow(world, pose);

    // Vanilla `LivingEntity.aiStep` (`LivingEntity.java:3023`) gates the jump
    // branch on `isAffectedByFluids()`, which `Player` returns as `!abilities.flying`.
    if flying.is_none()
        && input.focused
        && input.jump
        && pose.on_ground
        && pose.no_jump_delay_ticks == 0
    {
        // Vanilla arms the 10-tick held-jump lockout on the `jumpFromGround`
        // call itself, before the tiny-jump-power no-op check inside it.
        pose.no_jump_delay_ticks = LOCAL_PLAYER_JUMP_DELAY_TICKS;
        apply_local_player_jump_from_ground(world, &mut pose, input.sprinting);
    }

    // Vanilla `LivingEntity.travel` routes fall flying between the fluid and
    // air branches; creative flight cannot coexist with the shared glide flag
    // (`Player.canGlide` requires `!abilities.flying`). On a climbable,
    // `travelFallFlying` clears the flag locally (the client-side
    // `stopFallFlying` half) and falls through to the air chain.
    if flying.is_none() && local_player_is_fall_flying(world) {
        if on_climbable_before {
            if let Some(local_player_id) = world.local_player_id {
                world.entities.stop_fall_flying(local_player_id);
            }
        } else {
            return advance_local_player_fall_flying_physics_step(
                world,
                pose,
                input,
                step_seconds,
                stuck_multiplier,
                entity_collision_targets,
            );
        }
    }

    // `Player.travel` snapshots the pre-travel vertical delta while
    // creative-flying and rescales it by 0.6 after the shared travel chain,
    // discarding whatever the gravity/collision tail produced.
    let flying_pre_travel_vertical = flying.map(|_| pose.delta_movement.y);

    // Vanilla `LivingEntity.travelInAir`: the block friction below the feet
    // only applies while grounded, and `moveRelative` accumulates the scaled
    // input into the carried delta instead of producing a displacement.
    let pos_below = local_player_block_pos_below_affecting_movement(pose);
    let block_friction = if pose.on_ground {
        local_player_ground_block_friction_at(world, pos_below)
    } else {
        1.0
    };
    let horizontal_friction = block_friction * LOCAL_HORIZONTAL_FRICTION;
    let acceleration = if pose.on_ground {
        local_player_ground_speed(world, input) * LOCAL_PLAYER_GROUND_ACCELERATION_FACTOR
            / (block_friction * block_friction * block_friction)
    } else if let Some(abilities) = flying {
        // `Player.getFlyingSpeed` while creative-flying off-vehicle.
        local_player_flying_air_acceleration(abilities, input)
    } else if input.sprinting {
        LOCAL_PLAYER_SPRINT_AIR_ACCELERATION
    } else {
        LOCAL_PLAYER_AIR_ACCELERATION
    };
    let (input_x, input_z) =
        local_player_modified_input(world, pose, input, initial_fluid_contact.in_water());
    let (acceleration_x, acceleration_z) =
        local_player_input_acceleration(pose, input_x, input_z, acceleration);
    pose.delta_movement.x += acceleration_x * step_ticks;
    pose.delta_movement.z += acceleration_z * step_ticks;
    if let Some(kind) = climbable_before {
        // `handleOnClimbable` clamps the accumulated delta after
        // `moveRelative` and owns the climbable fall-distance reset.
        pose.fall_distance = 0.0;
        pose.delta_movement =
            local_player_climbable_limited_velocity(pose.delta_movement, input, kind);
    }

    let mut requested_x = pose.delta_movement.x * step_ticks;
    let mut requested_y = pose.delta_movement.y * step_ticks;
    let mut requested_z = pose.delta_movement.z * step_ticks;
    // Vanilla `Entity.move` multiplies the requested vector by the consumed
    // stuck-speed multiplier and zeroes the stored delta at consumption; the
    // fall-distance reset belongs to the block callback (`makeStuckInBlock`),
    // and the gravity chain below rebuilds from that zero vector.
    if let Some(stuck) = stuck_multiplier {
        requested_x *= stuck.x;
        requested_y *= stuck.y;
        requested_z *= stuck.z;
        pose.delta_movement = ProtocolVec3d::default();
    }
    if should_back_off_from_edge(world, pose, input, requested_y, entity_collision_targets) {
        (requested_x, requested_z) = back_off_from_edge(
            world,
            pose,
            requested_x,
            requested_z,
            entity_collision_targets,
        );
    }
    let collision_context = LocalPlayerCollisionContext::for_pose(world, pose, input.sneak);
    let step_height = local_player_step_height(world);
    let movement = world.clip_local_player_collision_movement(
        pose,
        [requested_x, requested_y, requested_z],
        step_height as f32,
        input.sneak,
        entity_collision_targets,
    );
    let step_move = EntityBlockEffectMovement::clipped(
        EntityVec3 {
            x: pose.position.x,
            y: pose.position.y,
            z: pose.position.z,
        },
        EntityVec3 {
            x: pose.position.x + movement[0],
            y: pose.position.y + movement[1],
            z: pose.position.z + movement[2],
        },
        EntityVec3 {
            x: requested_x,
            y: requested_y,
            z: requested_z,
        },
    );
    if pose.fall_distance != 0.0 && world.local_player_fall_reset_ray_hits(pose, movement) {
        pose.fall_distance = 0.0;
    }
    pose.position.x += movement[0];
    pose.position.y += movement[1];
    pose.position.z += movement[2];

    let x_collision = local_player_horizontal_axis_collided(requested_x, movement[0]);
    let z_collision = local_player_horizontal_axis_collided(requested_z, movement[2]);
    let horizontal_collision = x_collision || z_collision;
    let minor_horizontal_collision = horizontal_collision
        && local_player_horizontal_collision_is_minor(pose, input_x, input_z, movement);
    let vertical_collision = movement[1] != requested_y;
    // Vanilla `Entity.move` (`Entity.java:757-761`): any vertical request sets
    // `onGround = verticalCollision && requested_y < 0.0` (so ceiling bumps
    // with `requested_y > 0.0` never stay grounded). Preserve groundedness only
    // when `requested_y == 0.0` and the feet remain supported.
    let on_ground = if requested_y != 0.0 {
        vertical_collision && requested_y < 0.0
    } else {
        pose.on_ground && local_player_supported(world, pose, collision_context)
    };
    pose.on_ground = on_ground;
    let on_climbable_after = flying.is_none() && local_player_climbable_kind(world, pose).is_some();
    let fluid_contact = local_player_fluid_contact(world, pose);
    let landing_fall_distance = on_ground.then_some(
        pose.fall_distance
            + if movement[1] < 0.0 && !fluid_contact.in_water() {
                -movement[1]
            } else {
                0.0
            },
    );
    let bounce = apply_local_player_landing_block_effects(
        world,
        pose,
        input.sneak,
        vertical_collision,
        landing_fall_distance,
    );
    pose.fall_distance = if fluid_contact.in_water() || on_climbable_before || on_climbable_after {
        0.0
    } else {
        next_local_player_fall_distance(pose.fall_distance, movement[1], on_ground)
    };

    // `Entity.move` tail on the carried delta: zero the collided horizontal
    // axes, let the supporting block's `updateEntityMovementAfterFallOn`
    // resolve the vertical axis (the base impl zeroes it, slime reflects a
    // non-sneaking fall), then multiply in the block speed factor.
    if x_collision {
        pose.delta_movement.x = 0.0;
    }
    if z_collision {
        pose.delta_movement.z = 0.0;
    }
    if vertical_collision {
        pose.delta_movement.y =
            local_player_vertical_velocity_after_fall_on(pose.delta_movement.y, bounce);
    }
    // `Player.getBlockSpeedFactor` is pinned to 1.0 while creative-flying.
    let block_speed_factor = if flying.is_some() {
        DEFAULT_BLOCK_SPEED_FACTOR
    } else {
        local_player_block_speed_factor(world, pose)
    };
    pose.delta_movement.x *= block_speed_factor;
    pose.delta_movement.z *= block_speed_factor;

    // `travelInAir` post-move tail: the wall-climb impulse, then levitation
    // or gravity, then the friction multiply (0.98 vertical inside the
    // airborne chain, blockFriction * 0.91 horizontal).
    if (horizontal_collision || (input.focused && input.jump))
        && (on_climbable_after || powder_snow_climb_out_before)
    {
        pose.delta_movement.y = LOCAL_PLAYER_CLIMBABLE_UPWARD_VELOCITY_PER_TICK;
    }
    pose.delta_movement.y = if let Some(pre_travel_vertical) = flying_pre_travel_vertical {
        // The `Player.travel` creative-flight wrapper: the vertical delta is
        // rebuilt from the pre-travel snapshot times 0.6, replacing the
        // levitation/gravity chain outcome entirely.
        pre_travel_vertical * LOCAL_INPUT_FLY_VERTICAL_DAMPING.powf(step_ticks)
    } else {
        local_player_airborne_vertical_velocity(
            world,
            pos_below,
            pose.position.y,
            pose.delta_movement.y,
            step_ticks,
        )
    };
    let horizontal_drag = horizontal_friction.powf(step_ticks);
    pose.delta_movement.x *= horizontal_drag;
    pose.delta_movement.z *= horizontal_drag;

    pose.horizontal_collision = horizontal_collision;
    pose.minor_horizontal_collision = minor_horizontal_collision;
    (pose, step_move)
}

/// Vanilla `LivingEntity.travelFallFlying`: the glide aerodynamics rebuild
/// the carried delta from the look angle before `Entity.move`; movement
/// input is not consumed (`updateFallFlyingMovement` has no `moveRelative`),
/// and the fly-into-wall collision damage stays server-owned.
fn advance_local_player_fall_flying_physics_step(
    world: &mut WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    step_seconds: f64,
    stuck_multiplier: Option<EntityVec3>,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> (LocalPlayerPoseState, EntityBlockEffectMovement) {
    let step_ticks = step_seconds / LOCAL_PHYSICS_TICK_SECONDS;
    let initial_fluid_contact = local_player_fluid_contact(world, pose);
    let (input_x, input_z) =
        local_player_modified_input(world, pose, input, initial_fluid_contact.in_water());
    let pre_glide_horiz_speed = pose.delta_movement.x.hypot(pose.delta_movement.z);
    pose.delta_movement = local_player_fall_flying_updated_movement(world, pose, step_ticks);

    let mut requested_x = pose.delta_movement.x * step_ticks;
    let mut requested_y = pose.delta_movement.y * step_ticks;
    let mut requested_z = pose.delta_movement.z * step_ticks;
    if let Some(stuck) = stuck_multiplier {
        requested_x *= stuck.x;
        requested_y *= stuck.y;
        requested_z *= stuck.z;
        pose.delta_movement = ProtocolVec3d::default();
    }
    if should_back_off_from_edge(world, pose, input, requested_y, entity_collision_targets) {
        (requested_x, requested_z) = back_off_from_edge(
            world,
            pose,
            requested_x,
            requested_z,
            entity_collision_targets,
        );
    }
    let collision_context = LocalPlayerCollisionContext::for_pose(world, pose, input.sneak);
    let step_height = local_player_step_height(world);
    let movement = world.clip_local_player_collision_movement(
        pose,
        [requested_x, requested_y, requested_z],
        step_height as f32,
        input.sneak,
        entity_collision_targets,
    );
    let step_move = EntityBlockEffectMovement::clipped(
        EntityVec3 {
            x: pose.position.x,
            y: pose.position.y,
            z: pose.position.z,
        },
        EntityVec3 {
            x: pose.position.x + movement[0],
            y: pose.position.y + movement[1],
            z: pose.position.z + movement[2],
        },
        EntityVec3 {
            x: requested_x,
            y: requested_y,
            z: requested_z,
        },
    );
    if pose.fall_distance != 0.0 && world.local_player_fall_reset_ray_hits(pose, movement) {
        pose.fall_distance = 0.0;
    }
    pose.position.x += movement[0];
    pose.position.y += movement[1];
    pose.position.z += movement[2];

    let x_collision = local_player_horizontal_axis_collided(requested_x, movement[0]);
    let z_collision = local_player_horizontal_axis_collided(requested_z, movement[2]);
    let horizontal_collision = x_collision || z_collision;
    let minor_horizontal_collision = horizontal_collision
        && local_player_horizontal_collision_is_minor(pose, input_x, input_z, movement);
    let vertical_collision = movement[1] != requested_y;
    // Vanilla `Entity.move` (`Entity.java:757-761`): any vertical request sets
    // `onGround = verticalCollision && requested_y < 0.0` (so ceiling bumps
    // with `requested_y > 0.0` never stay grounded). Preserve groundedness only
    // when `requested_y == 0.0` and the feet remain supported.
    let on_ground = if requested_y != 0.0 {
        vertical_collision && requested_y < 0.0
    } else {
        pose.on_ground && local_player_supported(world, pose, collision_context)
    };
    pose.on_ground = on_ground;
    let fluid_contact = local_player_fluid_contact(world, pose);
    let landing_fall_distance = on_ground.then_some(
        pose.fall_distance
            + if movement[1] < 0.0 && !fluid_contact.in_water() {
                -movement[1]
            } else {
                0.0
            },
    );
    let bounce = apply_local_player_landing_block_effects(
        world,
        pose,
        input.sneak,
        vertical_collision,
        landing_fall_distance,
    );
    pose.fall_distance = if fluid_contact.in_water() {
        0.0
    } else {
        next_local_player_fall_distance(pose.fall_distance, movement[1], on_ground)
    };

    // `Entity.move` tail on the carried delta: zero the collided horizontal
    // axes and let the supporting block resolve the vertical axis. The block
    // speed factor is pinned to 1.0 while fall flying
    // (`Player.getBlockSpeedFactor`), and `travelFallFlying` has no
    // climbable clamp, wall-climb impulse, gravity, or friction tail — the
    // aerodynamics above already folded gravity and the 0.99/0.98 drag in.
    if x_collision {
        pose.delta_movement.x = 0.0;
    }
    if z_collision {
        pose.delta_movement.z = 0.0;
    }
    if horizontal_collision
        && !world.local_player_is_spectator()
        && !world.local_player_is_creative()
    {
        // Vanilla `LivingEntity.handleFallFlyingCollisions` (`LivingEntity.java:2567-2575`).
        let new_horiz_speed = pose.delta_movement.x.hypot(pose.delta_movement.z);
        let wall_dmg = ((pre_glide_horiz_speed - new_horiz_speed) * 10.0 - 3.0) as f32;
        if wall_dmg > 0.0 {
            append_local_player_movement_sound(
                world,
                pose,
                EntityClientMovementSoundKind::PlayerFall {
                    big: (wall_dmg as i32) > 4,
                },
            );
        }
    }
    if vertical_collision {
        pose.delta_movement.y =
            local_player_vertical_velocity_after_fall_on(pose.delta_movement.y, bounce);
    }

    pose.horizontal_collision = horizontal_collision;
    pose.minor_horizontal_collision = minor_horizontal_collision;
    (pose, step_move)
}

/// `LivingEntity.updateFallFlyingMovement`: pitch-derived lift against
/// gravity, downward-motion conversion along the look direction, climb
/// conversion that drains horizontal speed while looking up, horizontal
/// steering toward the look direction, then the 0.99/0.98/0.99 drag.
fn local_player_fall_flying_updated_movement(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    step_ticks: f64,
) -> ProtocolVec3d {
    let mut movement = pose.delta_movement;
    let pitch_radians = f64::from(pose.x_rot).to_radians();
    let yaw_radians = f64::from(pose.y_rot).to_radians();
    // `Entity.calculateViewVector` for the look angle.
    let look_x = -yaw_radians.sin() * pitch_radians.cos();
    let look_z = yaw_radians.cos() * pitch_radians.cos();
    let look_horizontal = (look_x * look_x + look_z * look_z).sqrt();
    let horizontal_speed = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let gravity = local_player_effective_gravity_per_tick(world, movement.y);
    let lift = pitch_radians.cos() * pitch_radians.cos();
    movement.y += gravity * (-1.0 + lift * 0.75) * step_ticks;
    if movement.y < 0.0 && look_horizontal > 0.0 {
        let convert = movement.y * -0.1 * lift * step_ticks;
        movement.x += look_x * convert / look_horizontal;
        movement.y += convert;
        movement.z += look_z * convert / look_horizontal;
    }
    if pitch_radians < 0.0 && look_horizontal > 0.0 {
        let convert = horizontal_speed * -pitch_radians.sin() * 0.04 * step_ticks;
        movement.x -= look_x * convert / look_horizontal;
        movement.y += convert * 3.2;
        movement.z -= look_z * convert / look_horizontal;
    }
    if look_horizontal > 0.0 {
        movement.x += (look_x / look_horizontal * horizontal_speed - movement.x) * 0.1 * step_ticks;
        movement.z += (look_z / look_horizontal * horizontal_speed - movement.z) * 0.1 * step_ticks;
    }
    ProtocolVec3d {
        x: movement.x * LOCAL_PLAYER_FALL_FLYING_HORIZONTAL_FRICTION.powf(step_ticks),
        y: movement.y * LOCAL_VERTICAL_FRICTION.powf(step_ticks),
        z: movement.z * LOCAL_PLAYER_FALL_FLYING_HORIZONTAL_FRICTION.powf(step_ticks),
    }
}

fn local_player_is_fall_flying(world: &WorldStore) -> bool {
    world
        .local_player_id
        .and_then(|id| world.entities.is_fall_flying(id))
        .unwrap_or(false)
}

/// Vanilla `LocalPlayer.modifyInput`: the raw ±1 key axes scaled by 0.98,
/// the item-use multiplier off-vehicle, the `SNEAKING_SPEED` attribute while
/// moving slowly (crouching or visually crawling), then projected onto the
/// unit square so straight input keeps its magnitude while diagonal input
/// caps at unit length.
fn local_player_modified_input(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    in_water: bool,
) -> (f64, f64) {
    if !input.focused {
        return (0.0, 0.0);
    }
    let mut strafe = axis(input.right, input.left);
    let mut forward = axis(input.forward, input.backward);
    if strafe == 0.0 && forward == 0.0 {
        return (0.0, 0.0);
    }
    // Vanilla `KeyboardInput.tick` (`KeyboardInput.java:35`) normalizes the raw
    // move vector to unit length before `LocalPlayer.modifyInput` applies the
    // 0.98 and slow-movement multipliers and projects onto the unit square.
    let raw_length = (strafe * strafe + forward * forward).sqrt();
    if raw_length > 0.0 {
        strafe /= raw_length;
        forward /= raw_length;
    }
    let mut scale = LOCAL_PLAYER_INPUT_SCALE;
    scale *= local_player_using_item_speed_multiplier(world);
    // `LocalPlayer.isMovingSlowly`: crouching or visually crawling, where
    // `isVisuallyCrawling` is the swimming pose out of water — swimming in
    // water never applies the sneaking-speed attribute.
    if pose.sneaking || (pose.swimming && !in_water) {
        scale *= local_player_attribute_value(world, VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID)
            .unwrap_or(LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER)
            .clamp(0.0, 1.0);
    }
    local_player_unit_square_input(strafe * scale, forward * scale)
}

/// `LocalPlayer.modifyInputSpeedForSquareMovement`.
fn local_player_unit_square_input(strafe: f64, forward: f64) -> (f64, f64) {
    let length = (strafe * strafe + forward * forward).sqrt();
    if length <= 0.0 {
        return (strafe, forward);
    }
    let direction = (strafe / length, forward / length);
    let abs_strafe = direction.0.abs();
    let abs_forward = direction.1.abs();
    let tangent = if abs_forward > abs_strafe {
        abs_strafe / abs_forward
    } else {
        abs_forward / abs_strafe
    };
    let modified_length = (length * (1.0 + tangent * tangent).sqrt()).min(1.0);
    (direction.0 * modified_length, direction.1 * modified_length)
}

/// `Entity.getInputVector`: normalize only above unit length, scale by the
/// speed, then rotate by the yaw.
fn local_player_input_acceleration(
    pose: LocalPlayerPoseState,
    strafe: f64,
    forward_input: f64,
    speed: f64,
) -> (f64, f64) {
    let length_squared = strafe * strafe + forward_input * forward_input;
    if length_squared < 1.0e-7 {
        return (0.0, 0.0);
    }
    let (mut strafe, mut forward_input) = (strafe, forward_input);
    if length_squared > 1.0 {
        let length = length_squared.sqrt();
        strafe /= length;
        forward_input /= length;
    }
    strafe *= speed;
    forward_input *= speed;
    let yaw = f64::from(pose.y_rot).to_radians();
    let forward = (-yaw.sin(), yaw.cos());
    let right = (-yaw.cos(), -yaw.sin());
    (
        forward.0 * forward_input + right.0 * strafe,
        forward.1 * forward_input + right.1 * strafe,
    )
}

/// `LivingEntity.getSpeed` for the local player: the synced movement-speed
/// attribute with Vanilla's +30% sprint modifier.
fn local_player_ground_speed(world: &WorldStore, input: LocalPlayerInputState) -> f64 {
    let mut speed = local_player_movement_speed_attribute_value(world);
    if input.sprinting {
        speed *= LOCAL_PLAYER_SPRINT_SPEED_MODIFIER;
    }
    speed
}

/// `Entity.getBlockPosBelowThatAffectsMyMovement` (`Entity.java`).
fn local_player_block_pos_below_affecting_movement(pose: LocalPlayerPoseState) -> BlockPos {
    BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y - 0.500001),
        z: local_player_block_floor(pose.position.z),
    }
}

/// `Entity.getBlockPosBelowThatAffectsMyMovement` friction sample.
fn local_player_ground_block_friction_at(world: &WorldStore, below_pos: BlockPos) -> f64 {
    f64::from(crate::terrain::vanilla_block_friction(
        world
            .block_view(below_pos)
            .and_then(|block| block.block_name),
    ))
}

pub(super) fn apply_local_player_jump_from_ground(
    world: &WorldStore,
    pose: &mut LocalPlayerPoseState,
    sprinting: bool,
) {
    let jump_velocity = local_player_jump_velocity(world, *pose);
    if jump_velocity > 1.0e-5 {
        pose.delta_movement.y = pose.delta_movement.y.max(jump_velocity);
        if sprinting {
            let (impulse_x, impulse_z) = local_player_sprint_jump_horizontal_impulse(*pose);
            pose.delta_movement.x += impulse_x;
            pose.delta_movement.z += impulse_z;
        }
    }
}

/// Vanilla `Entity.getFluidJumpThreshold` (`Entity.java:3600-3602`).
pub(super) fn local_player_fluid_jump_threshold(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
) -> f64 {
    if world.local_player_eye_height(pose) < 0.4 {
        0.0
    } else {
        LOCAL_PLAYER_FLUID_JUMP_THRESHOLD
    }
}

fn local_player_block_floor(value: f64) -> i32 {
    value.floor() as i32
}

fn local_player_horizontal_axis_collided(requested: f64, resolved: f64) -> bool {
    (resolved - requested).abs() >= VANILLA_HORIZONTAL_COLLISION_EPSILON
}

fn local_player_horizontal_collision_is_minor(
    pose: LocalPlayerPoseState,
    input_x: f64,
    input_z: f64,
    movement: [f64; 3],
) -> bool {
    // `LocalPlayer.xxa`/`zza` and Mth's yaw trig are floats even though the
    // resulting world-space dot product uses doubles.
    // The native movement helper keeps a right-positive strafe axis, while
    // Vanilla's `xxa` is the left-positive `ClientInput.moveVector.x`.
    let input_x = f64::from(-(input_x as f32));
    let input_z = f64::from(input_z as f32);
    let yaw = pose.y_rot.to_radians();
    let yaw_sin = f64::from(yaw.sin());
    let yaw_cos = f64::from(yaw.cos());
    let desired_x = input_x * yaw_cos - input_z * yaw_sin;
    let desired_z = input_z * yaw_cos + input_x * yaw_sin;
    let desired_length_squared = desired_x * desired_x + desired_z * desired_z;
    let movement_length_squared = movement[0] * movement[0] + movement[2] * movement[2];
    if desired_length_squared < f64::from(1.0e-5_f32)
        || movement_length_squared < f64::from(1.0e-5_f32)
    {
        return false;
    }
    let dot = desired_x * movement[0] + desired_z * movement[2];
    let angle = (dot / (desired_length_squared * movement_length_squared).sqrt())
        .clamp(-1.0, 1.0)
        .acos();
    angle < f64::from(LOCAL_PLAYER_MINOR_COLLISION_MAX_ANGLE_RADIANS)
}

fn should_back_off_from_edge(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    requested_y: f64,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> bool {
    input.focused
        && input.sneak
        && !local_player_is_flying(world)
        && requested_y <= 0.0
        && local_player_is_above_ground(world, pose, entity_collision_targets)
}

fn local_player_is_above_ground(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> bool {
    let step_height = local_player_step_height(world);
    pose.on_ground
        || (pose.fall_distance < step_height
            && !local_player_can_fall_at_least(
                world,
                pose,
                0.0,
                0.0,
                step_height - pose.fall_distance,
                entity_collision_targets,
            ))
}

fn next_local_player_fall_distance(previous: f64, vertical_movement: f64, on_ground: bool) -> f64 {
    if on_ground {
        return 0.0;
    }
    if vertical_movement < 0.0 {
        previous + -vertical_movement
    } else {
        previous
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPlayerFallBounce {
    None,
    Slime,
    Bed,
}

fn apply_local_player_landing_block_effects(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    suppressing_bounce: bool,
    vertical_collision: bool,
    landing_fall_distance: Option<f64>,
) -> LocalPlayerFallBounce {
    let Some(effect_block) = world.local_player_legacy_effect_block(pose, suppressing_bounce)
    else {
        return LocalPlayerFallBounce::None;
    };
    let Some(block_name) = effect_block.block_name else {
        return LocalPlayerFallBounce::None;
    };

    if let Some(fall_distance) = landing_fall_distance.filter(|distance| *distance > 0.0) {
        if block_name == LOCAL_PLAYER_HONEY_BLOCK_NAME {
            append_local_player_movement_sound(
                world,
                pose,
                EntityClientMovementSoundKind::PlayerHoneyBlockSlide,
            );
            emit_local_player_fall_damage_sounds(
                world,
                pose,
                fall_distance,
                0.2,
                Some(effect_block),
            );
        } else if block_name == LOCAL_PLAYER_POWDER_SNOW_BLOCK_NAME {
            if let Some(big) = player_powder_snow_fall_sound_size(fall_distance) {
                append_local_player_movement_sound(
                    world,
                    pose,
                    EntityClientMovementSoundKind::PlayerPowderSnowFall { big },
                );
            }
        } else {
            let damage_modifier = if block_name == "minecraft:hay_block" {
                0.2
            } else if block_name_is_bed(block_name) {
                0.5
            } else if block_name == "minecraft:slime_block" && !suppressing_bounce {
                0.0
            } else {
                1.0
            };
            if damage_modifier > 0.0 {
                emit_local_player_fall_damage_sounds(
                    world,
                    pose,
                    fall_distance,
                    damage_modifier,
                    None,
                );
            }
        }
    }

    if !vertical_collision || suppressing_bounce {
        return LocalPlayerFallBounce::None;
    }
    if block_name == "minecraft:slime_block" {
        LocalPlayerFallBounce::Slime
    } else if block_name_is_bed(block_name) {
        LocalPlayerFallBounce::Bed
    } else {
        LocalPlayerFallBounce::None
    }
}

fn emit_local_player_fall_damage_sounds(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    fall_distance: f64,
    damage_modifier: f64,
    override_block: Option<BlockView>,
) {
    if world.local_player_is_spectator()
        || world
            .local_player
            .abilities
            .is_some_and(|abilities| abilities.invulnerable)
    {
        return;
    }
    let safe_fall_distance =
        local_player_attribute_value(world, VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID).unwrap_or(3.0);
    let fall_damage_multiplier =
        local_player_attribute_value(world, VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID)
            .unwrap_or(1.0);
    let base_damage = fall_distance + 1.0e-6 - safe_fall_distance;
    let dmg = (base_damage * damage_modifier * fall_damage_multiplier).floor() as i32;
    if dmg <= 0 {
        return;
    }
    append_local_player_movement_sound(
        world,
        pose,
        EntityClientMovementSoundKind::PlayerFall { big: dmg > 4 },
    );
    let block_for_sound = override_block.or_else(|| {
        world.block_view(BlockPos {
            x: local_player_block_floor(pose.position.x),
            y: local_player_block_floor(pose.position.y - 0.2),
            z: local_player_block_floor(pose.position.z),
        })
    });
    if let Some(block) = block_for_sound.filter(|block| {
        !block
            .block_name
            .is_some_and(crate::terrain::block_name_is_air)
    }) {
        append_local_player_movement_sound(
            world,
            pose,
            EntityClientMovementSoundKind::PlayerBlockFall {
                block_state_id: block.block_state_id,
            },
        );
    }
}

fn append_local_player_movement_sound(
    world: &mut WorldStore,
    pose: LocalPlayerPoseState,
    kind: EntityClientMovementSoundKind,
) {
    let Some(entity_id) = world.local_player_id else {
        return;
    };
    if world.entities.is_silent(entity_id).unwrap_or(false) {
        return;
    }
    world.entities.push_entity_client_movement_effect_state(
        EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
            entity_id,
            kind,
            position: EntityVec3 {
                x: pose.position.x,
                y: pose.position.y,
                z: pose.position.z,
            },
        }),
    );
}

fn local_player_vertical_velocity_after_fall_on(
    vertical_velocity: f64,
    bounce: LocalPlayerFallBounce,
) -> f64 {
    if vertical_velocity >= 0.0 {
        return 0.0;
    }
    match bounce {
        LocalPlayerFallBounce::None => 0.0,
        LocalPlayerFallBounce::Slime => -vertical_velocity,
        LocalPlayerFallBounce::Bed => {
            -vertical_velocity * f64::from(LOCAL_PLAYER_BED_BOUNCE_FACTOR)
        }
    }
}

/// Vanilla `PowderSnowBlock.fallOn`: distances below four are silent, then
/// `LivingEntity.getFallSounds` switches from small to big at seven.
fn player_powder_snow_fall_sound_size(fall_distance: f64) -> Option<bool> {
    (fall_distance >= 4.0).then_some(fall_distance >= 7.0)
}

fn local_player_update_body_pose(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    flying: Option<LocalPlayerAbilitiesState>,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> LocalPlayerPoseState {
    // Vanilla `Player.updatePlayerPose` (`Player.java:342-370`): only spectator
    // and passenger bypass fit checks; creative flying still drops to crouch or
    // crawl in tight gaps while suppressing manual shift-crouch in `desiredPose`.
    if world.local_player_is_spectator() || world.local_player_vehicle_id().is_some() {
        return local_player_with_body_pose(pose, LocalPlayerBodyPose::Standing);
    }

    let swimming_pose = local_player_with_body_pose(pose, LocalPlayerBodyPose::Swimming);
    if !local_player_pose_fits(world, swimming_pose, input.sneak, entity_collision_targets) {
        return pose;
    }

    let desired_pose =
        local_player_with_body_pose(pose, local_player_desired_body_pose(world, input, flying));
    if local_player_pose_fits(world, desired_pose, input.sneak, entity_collision_targets) {
        return desired_pose;
    }

    let crouching_pose = local_player_with_body_pose(pose, LocalPlayerBodyPose::Crouching);
    if local_player_pose_fits(world, crouching_pose, input.sneak, entity_collision_targets) {
        crouching_pose
    } else if world.local_player_glides() {
        // FALL_FLYING / SPIN_ATTACK always fit where the crawl would.
        local_player_with_body_pose(pose, LocalPlayerBodyPose::Standing)
    } else {
        swimming_pose
    }
}

fn local_player_pose_fits(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    descending: bool,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> bool {
    world.local_player_collision_bounds_are_free(
        pose,
        world
            .local_player_body_pose_dimensions(pose, false)
            .bounds_at(pose.position)
            .deflated(COLLISION_EPSILON),
        descending,
        entity_collision_targets,
    )
}

fn local_player_desired_body_pose(
    world: &WorldStore,
    input: LocalPlayerInputState,
    flying: Option<LocalPlayerAbilitiesState>,
) -> LocalPlayerBodyPose {
    if world.local_player.swimming {
        LocalPlayerBodyPose::Swimming
    } else if input.focused && input.sneak && flying.is_none() {
        LocalPlayerBodyPose::Crouching
    } else {
        LocalPlayerBodyPose::Standing
    }
}

fn local_player_should_swim(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
) -> bool {
    // Vanilla `Player.updateSwimming` (`Player.java:1410-1415`) disables swimming
    // while creative-flying.
    if local_player_flying_abilities(world).is_some()
        || !input.focused
        || !input.sprinting
        || world.local_player_vehicle_id().is_some()
    {
        return false;
    }

    let fluid_contact = local_player_fluid_contact(world, pose);
    if !fluid_contact.in_water() {
        return false;
    }
    if world.local_player.swimming {
        return true;
    }
    // Vanilla `Entity.updateSwimming` (`Entity.java:1548-1550`) requires water
    // at the feet block position in addition to `isUnderWater()`.
    let feet_pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y),
        z: local_player_block_floor(pose.position.z),
    };
    fluid_contact.eye_in_water
        && world
            .fluid_state_and_height_at(feet_pos)
            .is_some_and(|(fluid, _)| fluid.kind == crate::TerrainFluidKind::Water)
}

fn local_player_with_body_pose(
    mut pose: LocalPlayerPoseState,
    body_pose: LocalPlayerBodyPose,
) -> LocalPlayerPoseState {
    pose.sneaking = matches!(body_pose, LocalPlayerBodyPose::Crouching);
    pose.swimming = matches!(body_pose, LocalPlayerBodyPose::Swimming);
    pose
}

fn local_player_effective_movement_input(
    world: &WorldStore,
    mut input: LocalPlayerInputState,
) -> LocalPlayerInputState {
    // Vanilla `LocalPlayer.aiStep` (`LocalPlayer.java:688-701, 740-742`): when
    // spectating another entity (`!isControlledCamera()`), local player movement
    // and vertical flight inputs do not move the detached player body.
    if !world.local_player.camera.follows_player {
        input.forward = false;
        input.backward = false;
        input.left = false;
        input.right = false;
        input.jump = false;
        input.sneak = false;
        input.sprinting = false;
        return input;
    }
    if input.sprinting && local_player_should_stop_sprinting(world, input) {
        input.sprinting = false;
    }
    input
}

fn local_player_is_sprinting_possible(world: &WorldStore, allowed_in_shallow_water: bool) -> bool {
    let sprint_source_is_eligible = if world.local_player_vehicle_id().is_some() {
        world.local_player_sprintable_vehicle_id().is_some()
    } else {
        local_player_has_enough_food_to_sprint(world)
    };

    let in_shallow_water = world.local_player().pose.is_some_and(|pose| {
        let contact = local_player_fluid_contact(world, pose);
        contact.in_water() && !contact.eye_in_water
    });
    local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_BLINDNESS_ID).is_none()
        && sprint_source_is_eligible
        && (allowed_in_shallow_water || !in_shallow_water)
}

pub(super) fn local_player_can_start_sprinting(
    world: &WorldStore,
    input: LocalPlayerInputState,
) -> bool {
    let Some(pose) = world.local_player().pose else {
        return false;
    };
    let contact = local_player_fluid_contact(world, pose);
    let under_water = contact.in_water() && contact.eye_in_water;
    let visually_crawling = pose.swimming && !contact.in_water();
    input.focused
        && input.forward
        && !input.backward
        && local_player_is_sprinting_possible(
            world,
            world
                .local_player()
                .abilities
                .is_some_and(|abilities| abilities.flying),
        )
        && !local_player_is_slow_due_to_using_item(world)
        && (!local_player_is_fall_flying(world) || under_water)
        && (!(pose.sneaking || visually_crawling) || under_water)
}

pub(super) fn local_player_should_stop_sprinting(
    world: &WorldStore,
    input: LocalPlayerInputState,
) -> bool {
    let Some(pose) = world.local_player().pose else {
        return true;
    };
    let contact = local_player_fluid_contact(world, pose);
    if world.local_player.swimming {
        !local_player_is_sprinting_possible(world, true)
            || !contact.in_water()
            || (!(input.forward && !input.backward) && !pose.on_ground && !input.sneak)
    } else {
        // Matches Vanilla 26.1 `LocalPlayer.shouldStopRunSprinting`
        // (`LocalPlayer.java:917-921`): item use, sneaking, and fall-flying
        // block starting a sprint (`canStartSprinting`), not sustaining an
        // active sprint latch while forward impulse remains active.
        !local_player_is_sprinting_possible(
            world,
            world
                .local_player()
                .abilities
                .is_some_and(|abilities| abilities.flying),
        ) || !input.focused
            || !input.forward
            || input.backward
            || pose.horizontal_collision && !pose.minor_horizontal_collision
    }
}

fn local_player_is_slow_due_to_using_item(world: &WorldStore) -> bool {
    world
        .local_using_item_use_effects()
        .is_some_and(|effects| !effects.can_sprint)
}

fn local_player_using_item_speed_multiplier(world: &WorldStore) -> f64 {
    if world.local_player_vehicle_id().is_some() {
        return 1.0;
    }
    world
        .local_using_item_use_effects()
        .map(|effects| {
            let multiplier = f64::from(effects.speed_multiplier);
            if multiplier.is_finite() {
                multiplier.clamp(0.0, 1.0)
            } else {
                0.2
            }
        })
        .unwrap_or(1.0)
}

fn local_player_has_enough_food_to_sprint(world: &WorldStore) -> bool {
    if world
        .local_player
        .abilities
        .is_some_and(|abilities| abilities.can_fly)
    {
        return true;
    }
    world
        .local_player
        .health
        .is_none_or(|health| health.food >= LOCAL_PLAYER_SPRINT_MIN_FOOD_LEVEL)
}

fn local_player_block_speed_factor(world: &WorldStore, pose: LocalPlayerPoseState) -> f64 {
    let raw_factor = local_player_raw_block_speed_factor(world, pose);
    let movement_efficiency =
        local_player_attribute_value(world, VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
    raw_factor + (DEFAULT_BLOCK_SPEED_FACTOR - raw_factor) * movement_efficiency
}

fn local_player_raw_block_speed_factor(world: &WorldStore, pose: LocalPlayerPoseState) -> f64 {
    let here_pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y),
        z: local_player_block_floor(pose.position.z),
    };
    let here_block = world.block_view(here_pos);
    let here_factor = here_block
        .as_ref()
        .and_then(|block| block.block_name)
        .map(|name| f64::from(crate::terrain::vanilla_block_speed_factor(Some(name))))
        .unwrap_or(DEFAULT_BLOCK_SPEED_FACTOR);
    if here_block
        .as_ref()
        .and_then(|block| block.block_name)
        .is_some_and(|block_name| {
            matches!(block_name, "minecraft:water" | "minecraft:bubble_column")
        })
    {
        return here_factor;
    }
    if (here_factor - DEFAULT_BLOCK_SPEED_FACTOR).abs() > f64::EPSILON {
        return here_factor;
    }

    let below_pos = BlockPos {
        y: local_player_block_floor(pose.position.y - 0.500001),
        ..here_pos
    };
    world
        .block_view(below_pos)
        .and_then(|block| {
            block
                .block_name
                .map(|name| f64::from(crate::terrain::vanilla_block_speed_factor(Some(name))))
        })
        .unwrap_or(DEFAULT_BLOCK_SPEED_FACTOR)
}

fn local_player_movement_speed_attribute_value(world: &WorldStore) -> f64 {
    // The local sprint flag owns `LivingEntity.SPEED_MODIFIER_SPRINTING`
    // (applied by the callers); the server's echo of it must not apply twice.
    let mut speed = world
        .local_player_id
        .and_then(|id| {
            world.entities.attribute_value_without_modifier(
                id,
                VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                VANILLA_SPRINTING_MODIFIER_ID,
            )
        })
        .unwrap_or(LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE);
    speed += local_player_powder_snow_speed_modifier(world);
    if !local_player_attribute_has_modifier(
        world,
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        VANILLA_SPEED_EFFECT_MODIFIER_ID,
    ) {
        speed *= local_player_effect_total_multiplier(
            world,
            VANILLA_MOB_EFFECT_SPEED_ID,
            SPEED_EFFECT_MOVEMENT_SPEED_MULTIPLIER,
        );
    }
    if !local_player_attribute_has_modifier(
        world,
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        VANILLA_SLOWNESS_EFFECT_MODIFIER_ID,
    ) {
        speed *= local_player_effect_total_multiplier(
            world,
            VANILLA_MOB_EFFECT_SLOWNESS_ID,
            SLOWNESS_EFFECT_MOVEMENT_SPEED_MULTIPLIER,
        );
    }
    speed.max(0.0)
}

impl WorldStore {
    /// Vanilla `AbstractClientPlayer.getFieldOfViewModifier` ability and
    /// movement-speed contribution. Item-use zoom belongs to the presentation
    /// owner and is intentionally not folded into this packet-owned projection.
    pub fn local_player_abilities_fov_modifier(&self, effective_sprinting: bool) -> f32 {
        let Some(abilities) = self.local_player.abilities else {
            return 1.0;
        };
        let mut modifier = if abilities.flying { 1.1 } else { 1.0 };
        if abilities.walking_speed != 0.0 {
            let mut movement_speed = local_player_movement_speed_attribute_value(self);
            if effective_sprinting {
                movement_speed *= LOCAL_PLAYER_SPRINT_SPEED_MODIFIER;
            }
            let speed_factor = movement_speed as f32 / abilities.walking_speed;
            modifier *= (speed_factor + 1.0) * 0.5;
        }
        if modifier.is_finite() {
            modifier
        } else {
            1.0
        }
    }

    /// Vanilla `LivingEntity.aiStep` for the locally authoritative player:
    /// after travel, `applyEffectsFromBlocks()` runs with the step's recorded
    /// clipped move. A spectator has `noPhysics`, so `isAffectedByBlocks`
    /// skips the callback entirely; `canFreeze` extras (spectator mode and
    /// freeze-immune wearables) fold into the query's freeze immunity.
    pub(super) fn apply_local_player_block_effects(
        &mut self,
        pose: &mut LocalPlayerPoseState,
        movement: EntityBlockEffectMovement,
    ) {
        if self.chunks.is_empty() || self.local_player_is_spectator() {
            return;
        }
        let Some(player_id) = self.local_player_id else {
            return;
        };
        let dimensions = self.local_player_dimensions(*pose);
        let half_width = (dimensions.width / 2.0) as f32;
        let step = LocalPlayerBlockEffectStep {
            player_id,
            position: EntityVec3 {
                x: pose.position.x,
                y: pose.position.y,
                z: pose.position.z,
            },
            delta_movement: EntityVec3 {
                x: pose.delta_movement.x,
                y: pose.delta_movement.y,
                z: pose.delta_movement.z,
            },
            fall_distance: pose.fall_distance,
            bounds: crate::EntityPickBoundsState {
                min: [-half_width, 0.0, -half_width],
                max: [half_width, dimensions.height as f32, half_width],
                pick_radius: 0.0,
            },
            on_ground: pose.on_ground,
            stepping_carefully: pose.sneaking,
            extra_freeze_immune: !local_player_can_freeze(self),
            movement,
        };
        let mut block_effect_world = crate::client::EntityBlockEffectWorld::new(
            self.dimension,
            &self.registries,
            &self.chunks,
            self.entity_block_effect_weather(),
        );
        let Some(outcome) = self
            .entities
            .apply_local_player_block_effect_step(step, &mut block_effect_world)
        else {
            return;
        };
        pose.delta_movement = ProtocolVec3d {
            x: outcome.delta_movement.x,
            y: outcome.delta_movement.y,
            z: outcome.delta_movement.z,
        };
        pose.fall_distance = outcome.fall_distance;
        // Vanilla `HoneyBlock.maybeDoSlideEffects` (`HoneyBlock.java:124-126`):
        // local client plays `block.honey_block.slide` with 1/5 chance per tick.
        if outcome.honey_block_sliding {
            let play = self
                .entities
                .with_entity_random_mut(player_id, |random| random.next_int_bound(5) == 0)
                .unwrap_or(false);
            if play {
                append_local_player_movement_sound(
                    self,
                    *pose,
                    EntityClientMovementSoundKind::PlayerHoneyBlockSlide,
                );
            }
        }
    }
}

fn local_player_powder_snow_speed_modifier(world: &WorldStore) -> f64 {
    if local_player_attribute_has_modifier(
        world,
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        VANILLA_POWDER_SNOW_SPEED_MODIFIER_ID,
    ) {
        return 0.0;
    }

    POWDER_SNOW_MOVEMENT_SPEED_MODIFIER_AT_FULL_FREEZE
        * f64::from(world.local_player_percent_frozen())
}

impl WorldStore {
    /// Vanilla `Entity.getTicksFrozen` for the local player.
    pub fn local_player_ticks_frozen(&self) -> i32 {
        self.local_player_id
            .and_then(|id| self.entities.ticks_frozen(id))
            .unwrap_or(0)
            .max(0)
    }

    /// Vanilla `Entity.getPercentFrozen`:
    /// `min(ticksFrozen, getTicksRequiredToFreeze()) / getTicksRequiredToFreeze()`.
    pub fn local_player_percent_frozen(&self) -> f32 {
        (self.local_player_ticks_frozen() as f32 / LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE as f32)
            .clamp(0.0, 1.0)
    }
}

fn local_player_inside_powder_snow(world: &WorldStore, pose: LocalPlayerPoseState) -> bool {
    let pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y),
        z: local_player_block_floor(pose.position.z),
    };
    world
        .block_view(pos)
        .and_then(|block| block.block_name)
        .is_some_and(|block_name| block_name == LOCAL_PLAYER_POWDER_SNOW_BLOCK_NAME)
}

fn local_player_can_freeze(world: &WorldStore) -> bool {
    !world.local_player_is_spectator() && !world.local_player_has_freeze_immune_wearable()
}

fn local_player_jump_velocity(world: &WorldStore, pose: LocalPlayerPoseState) -> f64 {
    let mut velocity = local_player_attribute_value(world, VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)
        .unwrap_or(LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE)
        .max(0.0)
        * local_player_block_jump_factor(world, pose);
    if let Some(amplifier) = local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
    {
        velocity += amplified_effect_amount(amplifier, JUMP_BOOST_VELOCITY_PER_LEVEL);
    }
    velocity
}

fn local_player_sprint_jump_horizontal_impulse(pose: LocalPlayerPoseState) -> (f64, f64) {
    let yaw = f64::from(pose.y_rot).to_radians();
    (
        -yaw.sin() * SPRINT_JUMP_HORIZONTAL_IMPULSE,
        yaw.cos() * SPRINT_JUMP_HORIZONTAL_IMPULSE,
    )
}

fn local_player_block_jump_factor(world: &WorldStore, pose: LocalPlayerPoseState) -> f64 {
    let here_pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y),
        z: local_player_block_floor(pose.position.z),
    };
    let here_factor = world
        .block_view(here_pos)
        .and_then(|block| {
            block
                .block_name
                .map(|name| f64::from(crate::terrain::vanilla_block_jump_factor(Some(name))))
        })
        .unwrap_or(DEFAULT_BLOCK_JUMP_FACTOR);
    if (here_factor - DEFAULT_BLOCK_JUMP_FACTOR).abs() > f64::EPSILON {
        return here_factor;
    }

    let below_pos = BlockPos {
        y: local_player_block_floor(pose.position.y - 0.500001),
        ..here_pos
    };
    world
        .block_view(below_pos)
        .and_then(|block| {
            block
                .block_name
                .map(|name| f64::from(crate::terrain::vanilla_block_jump_factor(Some(name))))
        })
        .unwrap_or(DEFAULT_BLOCK_JUMP_FACTOR)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPlayerClimbableKind {
    Ordinary,
    Scaffolding,
}

fn local_player_climbable_kind(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
) -> Option<LocalPlayerClimbableKind> {
    let pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y),
        z: local_player_block_floor(pose.position.z),
    };
    let block = world.block_view(pos)?;
    if block_is_in_climbable_tag(world, &block) {
        if block.block_name == Some("minecraft:scaffolding") {
            return Some(LocalPlayerClimbableKind::Scaffolding);
        }
        return Some(LocalPlayerClimbableKind::Ordinary);
    }
    if local_player_trapdoor_usable_as_ladder(world, pos, &block) {
        return Some(LocalPlayerClimbableKind::Ordinary);
    }
    None
}

pub(crate) fn local_player_is_on_climbable(world: &WorldStore, pose: LocalPlayerPoseState) -> bool {
    local_player_climbable_kind(world, pose).is_some()
}

fn block_is_in_climbable_tag(world: &WorldStore, block: &BlockView) -> bool {
    if world
        .registries()
        .holder_ids_for_tag("minecraft:block", "minecraft:climbable")
        .is_some()
    {
        let block_id = bbb_protocol::block_state_types::vanilla_block_state_block_type_id(
            block.block_state_id,
        )
        .or_else(|| {
            world
                .content_registry()
                .block_by_state_id(block.block_state_id)
                .map(|(spec, _)| spec.block_id)
        });
        return block_id.is_some_and(|id| {
            world
                .registries()
                .holder_is_in_tag("minecraft:block", "minecraft:climbable", id)
        });
    }
    block
        .block_name
        .is_some_and(default_vanilla_climbable_block_name)
}

fn default_vanilla_climbable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:ladder"
            | "minecraft:vine"
            | "minecraft:scaffolding"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
    )
}

fn local_player_trapdoor_usable_as_ladder(
    world: &WorldStore,
    pos: BlockPos,
    block: &BlockView,
) -> bool {
    if !block.block_name.is_some_and(block_name_is_trapdoor) {
        return false;
    }
    if block.block_properties.get("open").map(String::as_str) != Some("true") {
        return false;
    }
    let Some(facing) = block.block_properties.get("facing").map(String::as_str) else {
        return false;
    };
    let Some(y) = pos.y.checked_sub(1) else {
        return false;
    };
    let below_pos = BlockPos { y, ..pos };
    let Some(below_block) = world.block_view(below_pos) else {
        return false;
    };
    below_block.block_name == Some("minecraft:ladder")
        && below_block
            .block_properties
            .get("facing")
            .map(String::as_str)
            == Some(facing)
}

/// Facts behind one vanilla `Entity.spawnSprintParticle` call for the local
/// player: the on-pos block, the clamp inputs and the trailing velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalSprintParticleState {
    pub block_state_id: i32,
    pub biome_id: Option<i32>,
    pub on_pos: BlockPos,
    pub entity_block_pos: BlockPos,
    pub position: ProtocolVec3d,
    pub width: f64,
    pub velocity_x: f64,
    pub velocity_z: f64,
}

impl WorldStore {
    /// Vanilla `Entity.canSpawnSprintParticle` plus the `LocalPlayer` flying
    /// override, resolved with the `spawnSprintParticle` on-pos facts: the
    /// sprinting shared flag, the water/lava/spectator/crouch/alive gates and
    /// the `getOnPosLegacy` block whose render shape must not be invisible.
    pub fn local_player_sprint_particle_state(&self) -> Option<LocalSprintParticleState> {
        let player = self.local_player();
        if !player.sprinting {
            return None;
        }
        if player.abilities.is_some_and(|abilities| abilities.flying) {
            return None;
        }
        if self.local_player_is_spectator() {
            return None;
        }
        if player.health.is_some_and(|health| health.health <= 0.0) {
            return None;
        }
        let pose = player.pose?;
        if pose.sneaking {
            return None;
        }
        let contact = local_player_fluid_contact(self, pose);
        if contact.in_water() || contact.in_lava() {
            return None;
        }
        let probe = self.local_player_sprint_effect_probe(pose)?;
        if probe
            .block_name
            .is_none_or(crate::terrain::block_name_has_invisible_render_shape)
        {
            return None;
        }
        Some(LocalSprintParticleState {
            block_state_id: probe.block_state_id,
            biome_id: self.biome_id_at(probe.pos),
            on_pos: probe.pos,
            entity_block_pos: BlockPos {
                x: pose.position.x.floor() as i32,
                y: pose.position.y.floor() as i32,
                z: pose.position.z.floor() as i32,
            },
            position: pose.position,
            width: self.local_player_dimensions(pose).width,
            velocity_x: pose.delta_movement.x * -4.0,
            velocity_z: pose.delta_movement.z * -4.0,
        })
    }

    /// `Entity.getOnPosLegacy` for the sprint particle: the supporting-block
    /// override on the ground and the plain `y - 0.2` sample in the air.
    fn local_player_sprint_effect_probe(&self, pose: LocalPlayerPoseState) -> Option<BlockView> {
        if pose.on_ground {
            self.local_player_legacy_effect_block(pose, pose.delta_movement.y < 0.0)
        } else {
            self.block_view(self.legacy_on_pos(
                None,
                EntityVec3 {
                    x: pose.position.x,
                    y: pose.position.y,
                    z: pose.position.z,
                },
            ))
        }
    }
}

fn local_player_climbable_limited_velocity(
    velocity: ProtocolVec3d,
    input: LocalPlayerInputState,
    kind: LocalPlayerClimbableKind,
) -> ProtocolVec3d {
    let downward_limit = if input.sneak && kind != LocalPlayerClimbableKind::Scaffolding {
        0.0
    } else {
        LOCAL_PLAYER_CLIMBABLE_MAX_DOWNWARD_VELOCITY_PER_TICK
    };
    ProtocolVec3d {
        x: velocity.x.clamp(
            -LOCAL_PLAYER_CLIMBABLE_MAX_HORIZONTAL_VELOCITY_PER_TICK,
            LOCAL_PLAYER_CLIMBABLE_MAX_HORIZONTAL_VELOCITY_PER_TICK,
        ),
        y: velocity.y.max(downward_limit),
        z: velocity.z.clamp(
            -LOCAL_PLAYER_CLIMBABLE_MAX_HORIZONTAL_VELOCITY_PER_TICK,
            LOCAL_PLAYER_CLIMBABLE_MAX_HORIZONTAL_VELOCITY_PER_TICK,
        ),
    }
}

fn local_player_airborne_vertical_velocity(
    world: &WorldStore,
    pos_below: BlockPos,
    position_y: f64,
    current_velocity: f64,
    step_ticks: f64,
) -> f64 {
    let has_loaded_world = world.level_info().is_some() || !world.chunks.is_empty();
    let mut velocity = current_velocity;
    if let Some(amplifier) = local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_LEVITATION_ID)
    {
        let target = amplified_effect_amount(amplifier, LEVITATION_TARGET_VELOCITY_PER_LEVEL);
        velocity += (target - velocity) * LEVITATION_APPROACH_FACTOR_PER_TICK * step_ticks;
    } else if !has_loaded_world || world.probe_chunk(pos_below.chunk()).is_some() {
        velocity -= local_player_effective_gravity_per_tick(world, current_velocity) * step_ticks;
    } else if position_y > f64::from(world.dimension.min_y) {
        // Vanilla `LivingEntity.travelInAir` (`LivingEntity.java:2430-2436`)
        // clamps client-side falling speed in unloaded chunks.
        velocity = -0.1;
    } else {
        velocity = 0.0;
    }
    velocity * LOCAL_VERTICAL_FRICTION.powf(step_ticks)
}

fn local_player_effective_gravity_per_tick(world: &WorldStore, current_velocity: f64) -> f64 {
    if local_player_no_gravity(world) {
        return 0.0;
    }
    let gravity = local_player_attribute_value(world, VANILLA_ATTRIBUTE_GRAVITY_ID)
        .unwrap_or(LOCAL_INPUT_DEFAULT_GRAVITY_ATTRIBUTE);
    if current_velocity <= 0.0
        && local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_SLOW_FALLING_ID).is_some()
    {
        gravity.min(0.01)
    } else {
        gravity
    }
}

fn local_player_no_gravity(world: &WorldStore) -> bool {
    world
        .local_player_id
        .and_then(|id| world.entities.no_gravity(id))
        .unwrap_or(false)
}

/// `Player.getFlyingSpeed` while creative-flying off-vehicle: the abilities
/// flying speed, doubled while sprinting.
fn local_player_flying_air_acceleration(
    abilities: LocalPlayerAbilitiesState,
    input: LocalPlayerInputState,
) -> f64 {
    let mut speed = f64::from(abilities.flying_speed).max(0.0);
    if input.sprinting {
        speed *= LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER;
    }
    speed
}

fn local_player_attribute_value(world: &WorldStore, attribute_id: i32) -> Option<f64> {
    world
        .local_player_id
        .and_then(|id| world.entities.attribute_value(id, attribute_id))
}

fn local_player_attribute_has_modifier(
    world: &WorldStore,
    attribute_id: i32,
    modifier_id: &str,
) -> bool {
    world.local_player_id.is_some_and(|id| {
        world
            .entities
            .attribute_has_modifier(id, attribute_id, modifier_id)
    })
}

fn local_player_effect_amplifier(world: &WorldStore, effect_id: i32) -> Option<i32> {
    world
        .local_player_id
        .and_then(|id| world.entity_effect(id, effect_id))
        .map(|effect| effect.amplifier)
}

fn local_player_effect_total_multiplier(
    world: &WorldStore,
    effect_id: i32,
    amount_per_level: f64,
) -> f64 {
    local_player_effect_amplifier(world, effect_id)
        .map(|amplifier| 1.0 + amplified_effect_amount(amplifier, amount_per_level))
        .unwrap_or(1.0)
}

fn amplified_effect_amount(amplifier: i32, amount_per_level: f64) -> f64 {
    amount_per_level * (f64::from(amplifier) + 1.0)
}

fn local_player_flying_abilities(world: &WorldStore) -> Option<LocalPlayerAbilitiesState> {
    world
        .local_player()
        .abilities
        .filter(|abilities| abilities.flying)
}

fn local_player_is_flying(world: &WorldStore) -> bool {
    local_player_flying_abilities(world).is_some()
}

fn local_player_step_height(world: &WorldStore) -> f64 {
    local_player_attribute_value(world, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID)
        .filter(|value| value.is_finite())
        .map(|value| value.max(0.0))
        .unwrap_or(LOCAL_PLAYER_STEP_HEIGHT)
}

fn back_off_from_edge(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    requested_x: f64,
    requested_z: f64,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> (f64, f64) {
    const MAX_SNEAK_BACKOFF_DELTA: f64 = 2.0;
    const MAX_SNEAK_BACKOFF_STEPS: usize = 64;
    if !requested_x.is_finite()
        || !requested_z.is_finite()
        || requested_x.abs() > MAX_SNEAK_BACKOFF_DELTA
        || requested_z.abs() > MAX_SNEAK_BACKOFF_DELTA
    {
        return (requested_x, requested_z);
    }
    let step_height = local_player_step_height(world);
    let mut backed_x = requested_x;
    let mut backed_z = requested_z;
    let step_x = backed_x.signum() * EDGE_BACKOFF_STEP;
    let step_z = backed_z.signum() * EDGE_BACKOFF_STEP;

    let mut steps = 0usize;
    while backed_x != 0.0
        && steps < MAX_SNEAK_BACKOFF_STEPS
        && local_player_can_fall_at_least(
            world,
            pose,
            backed_x,
            0.0,
            step_height,
            entity_collision_targets,
        )
    {
        steps += 1;
        if backed_x.abs() <= EDGE_BACKOFF_STEP {
            backed_x = 0.0;
            break;
        }
        backed_x -= step_x;
    }

    let mut steps = 0usize;
    while backed_z != 0.0
        && steps < MAX_SNEAK_BACKOFF_STEPS
        && local_player_can_fall_at_least(
            world,
            pose,
            0.0,
            backed_z,
            step_height,
            entity_collision_targets,
        )
    {
        steps += 1;
        if backed_z.abs() <= EDGE_BACKOFF_STEP {
            backed_z = 0.0;
            break;
        }
        backed_z -= step_z;
    }

    let mut steps = 0usize;
    while backed_x != 0.0
        && backed_z != 0.0
        && steps < MAX_SNEAK_BACKOFF_STEPS
        && local_player_can_fall_at_least(
            world,
            pose,
            backed_x,
            backed_z,
            step_height,
            entity_collision_targets,
        )
    {
        steps += 1;
        if backed_x.abs() <= EDGE_BACKOFF_STEP {
            backed_x = 0.0;
        } else {
            backed_x -= step_x;
        }

        if backed_z.abs() <= EDGE_BACKOFF_STEP {
            backed_z = 0.0;
        } else {
            backed_z -= step_z;
        }
    }

    (backed_x, backed_z)
}

fn local_player_can_fall_at_least(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    delta_x: f64,
    delta_z: f64,
    min_height: f64,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> bool {
    world.local_player_collision_bounds_are_free(
        pose,
        world
            .local_player_bounds(pose)
            .moved(delta_x, 0.0, delta_z)
            .edge_support_probe(min_height),
        true,
        entity_collision_targets,
    )
}

fn local_player_supported(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    collision_context: LocalPlayerCollisionContext,
) -> bool {
    if world.local_player_is_spectator() {
        return false;
    }
    local_player_collides_with_context(
        world,
        world
            .local_player_bounds(pose)
            .moved(0.0, -SUPPORT_EPSILON, 0.0),
        collision_context,
    )
}

fn axis(positive: bool, negative: bool) -> f64 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn axis_f32(positive: bool, negative: bool) -> f32 {
    axis(positive, negative) as f32
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}
