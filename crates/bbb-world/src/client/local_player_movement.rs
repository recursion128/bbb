use bbb_protocol::packets::{
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind as ProtocolEntityDataValueKind,
    Vec3d as ProtocolVec3d,
};

use super::local_player::{LocalPlayerAbilitiesState, LocalPlayerInputState, LocalPlayerPoseState};
use super::local_player_collision::{
    local_player_collides, local_player_collides_with_context, CollisionAxis as Axis,
    LocalPlayerBounds, LocalPlayerCollisionContext, COLLISION_EPSILON,
};
use super::local_player_fluid::local_player_fluid_contact;
use crate::{BlockPos, BlockProbe, WorldStore};

mod fluid;
#[cfg(test)]
mod tests;

use self::fluid::advance_local_player_fluid_physics_step;

pub(super) const LOCAL_INPUT_MOUSE_SENSITIVITY_DEGREES: f32 = 0.12;
pub(super) const LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND: f64 = 4.317;
pub(super) const LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND: f64 = 5.612;

const VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID: i32 = 22;
const VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID: i32 = 21;
const VANILLA_ATTRIBUTE_GRAVITY_ID: i32 = 14;
const VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID: i32 = 15;
const VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID: i32 = 26;
const VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID: i32 = 32;
const VANILLA_MOB_EFFECT_SPEED_ID: i32 = 0;
const VANILLA_MOB_EFFECT_SLOWNESS_ID: i32 = 1;
const VANILLA_MOB_EFFECT_JUMP_BOOST_ID: i32 = 7;
const VANILLA_MOB_EFFECT_BLINDNESS_ID: i32 = 14;
const VANILLA_MOB_EFFECT_LEVITATION_ID: i32 = 24;
const VANILLA_MOB_EFFECT_SLOW_FALLING_ID: i32 = 27;
const VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID: i32 = 29;
const VANILLA_MOB_EFFECT_WEAVING_ID: i32 = 36;
const LOCAL_PLAYER_SPRINT_MIN_FOOD_LEVEL: i32 = 7;
const LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE: f64 = 0.1;
const LOCAL_INPUT_DEFAULT_GRAVITY_ATTRIBUTE: f64 = 0.08;
const LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE: f64 = 0.42;
const LOCAL_INPUT_SPRINT_SPEED_MULTIPLIER: f64 =
    LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND / LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND;
const LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER: f64 = 2.0;
const LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER: f64 = 0.3;
const LOCAL_INPUT_FLY_VERTICAL_SPEED_MULTIPLIER: f64 = 3.0;
const LOCAL_INPUT_FLY_VERTICAL_DAMPING: f64 = 0.6;
const LOCAL_INPUT_FLY_AIR_DRAG: f64 = 0.91;
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
const LOCAL_INPUT_FLUID_FALLING_GRAVITY_SCALE: f64 = 1.0 / 16.0;
const LOCAL_INPUT_LAVA_GRAVITY_SCALE: f64 = 0.25;
const LOCAL_INPUT_WATER_CURRENT_PUSH_PER_TICK: f64 = 0.014;
const LOCAL_INPUT_LAVA_CURRENT_PUSH_PER_TICK: f64 = 0.0023333333333333335;
const LOCAL_INPUT_FAST_LAVA_CURRENT_PUSH_PER_TICK: f64 = 0.007;
const LOCAL_INPUT_FLUID_CURRENT_MIN_PUSH_PER_TICK: f64 = 0.0045;
const LOCAL_INPUT_FLUID_CURRENT_MIN_HORIZONTAL_VELOCITY_PER_TICK: f64 = 0.003;
const LOCAL_INPUT_FLUID_CURRENT_APPLY_THRESHOLD_SQUARED: f64 = 1.0e-5;
const LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK: f64 = 0.3;
const LOCAL_INPUT_SWIM_LOOK_DOWN_VERTICAL_APPROACH: f64 = 0.085;
const LOCAL_INPUT_SWIM_VERTICAL_APPROACH: f64 = 0.06;
const LOCAL_INPUT_SWIM_LOOK_DOWN_THRESHOLD: f64 = -0.2;
const LOCAL_INPUT_SWIM_HEAD_FLUID_OFFSET: f64 = 0.9;
const LOCAL_INPUT_BUBBLE_COLUMN_INSIDE_PUSH_UP_PER_TICK: f64 = 0.06;
const LOCAL_INPUT_BUBBLE_COLUMN_INSIDE_PUSH_UP_LIMIT: f64 = 0.7;
const LOCAL_INPUT_BUBBLE_COLUMN_ABOVE_PUSH_UP_PER_TICK: f64 = 0.1;
const LOCAL_INPUT_BUBBLE_COLUMN_ABOVE_PUSH_UP_LIMIT: f64 = 1.8;
const LOCAL_INPUT_BUBBLE_COLUMN_DRAG_DOWN_PER_TICK: f64 = 0.03;
const LOCAL_INPUT_BUBBLE_COLUMN_INSIDE_DRAG_DOWN_LIMIT: f64 = -0.3;
const LOCAL_INPUT_BUBBLE_COLUMN_ABOVE_DRAG_DOWN_LIMIT: f64 = -0.9;
const LOCAL_PLAYER_FLUID_JUMP_THRESHOLD: f64 = 0.4;
const LOCAL_PLAYER_STEP_HEIGHT: f64 = 0.6;
const SUPPORT_EPSILON: f64 = 1.0e-3;
const EDGE_BACKOFF_STEP: f64 = 0.05;
const COLLISION_CLIP_STEPS: usize = 12;
const SPEED_EFFECT_MOVEMENT_SPEED_MULTIPLIER: f64 = 0.2;
const SLOWNESS_EFFECT_MOVEMENT_SPEED_MULTIPLIER: f64 = -0.15;
const JUMP_BOOST_VELOCITY_PER_LEVEL: f64 = 0.1;
const LEVITATION_TARGET_VELOCITY_PER_LEVEL: f64 = 0.05;
const LEVITATION_APPROACH_FACTOR_PER_TICK: f64 = 0.2;
const VANILLA_SPEED_EFFECT_MODIFIER_ID: &str = "minecraft:effect.speed";
const VANILLA_SLOWNESS_EFFECT_MODIFIER_ID: &str = "minecraft:effect.slowness";
const VANILLA_POWDER_SNOW_SPEED_MODIFIER_ID: &str = "minecraft:powder_snow";
const VANILLA_ENTITY_DATA_INT_SERIALIZER_ID: i32 = 1;
const LOCAL_PLAYER_POWDER_SNOW_BLOCK_NAME: &str = "minecraft:powder_snow";
const LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE: i32 = 140;
const POWDER_SNOW_MOVEMENT_SPEED_MODIFIER_AT_FULL_FREEZE: f64 = -0.05;
const SLOW_BLOCK_SPEED_FACTOR: f64 = 0.4;
const DEFAULT_BLOCK_SPEED_FACTOR: f64 = 1.0;
const HONEY_BLOCK_JUMP_FACTOR: f64 = 0.5;
const DEFAULT_BLOCK_JUMP_FACTOR: f64 = 1.0;
const SPRINT_JUMP_HORIZONTAL_IMPULSE: f64 = 0.2;
const LOCAL_PLAYER_CLIMBABLE_MAX_HORIZONTAL_VELOCITY_PER_TICK: f64 = 0.15;
const LOCAL_PLAYER_CLIMBABLE_MAX_DOWNWARD_VELOCITY_PER_TICK: f64 = -0.15;
const LOCAL_PLAYER_CLIMBABLE_UPWARD_VELOCITY_PER_TICK: f64 = 0.2;
const LOCAL_PLAYER_ENTITY_INSIDE_BLOCK_EPSILON: f64 = 1.0e-5;

pub(super) fn integrate_local_player_input_pose_with_world_effects(
    world: &mut WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    dt_seconds: f64,
) -> LocalPlayerPoseState {
    pose = apply_local_player_input_look(pose, input);

    let mut remaining_seconds = dt_seconds.max(0.0);
    while remaining_seconds > COLLISION_EPSILON {
        let step_seconds = remaining_seconds.min(LOCAL_PHYSICS_TICK_SECONDS);
        pose = advance_local_player_physics_step(world, pose, input, step_seconds);
        local_player_update_powder_snow_freezing(world, pose);
        remaining_seconds -= step_seconds;
    }

    pose
}

pub(super) fn apply_local_player_input_look(
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
) -> LocalPlayerPoseState {
    if input.focused {
        pose.y_rot = wrap_degrees_f32(
            pose.y_rot + input.mouse_delta_x as f32 * LOCAL_INPUT_MOUSE_SENSITIVITY_DEGREES,
        );
        pose.x_rot = (pose.x_rot
            + input.mouse_delta_y as f32 * LOCAL_INPUT_MOUSE_SENSITIVITY_DEGREES)
            .clamp(-90.0, 90.0);
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
    world: &WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    step_seconds: f64,
) -> LocalPlayerPoseState {
    let input = local_player_effective_movement_input(world, input);
    let forward_input = if input.focused {
        axis(input.forward, input.backward)
    } else {
        0.0
    };
    let strafe_input = if input.focused {
        axis(input.right, input.left)
    } else {
        0.0
    };
    let yaw = f64::from(pose.y_rot).to_radians();
    let forward = (-yaw.sin(), yaw.cos());
    let right = (-yaw.cos(), -yaw.sin());
    let mut move_x = forward.0 * forward_input + right.0 * strafe_input;
    let mut move_z = forward.1 * forward_input + right.1 * strafe_input;
    let horizontal_len = (move_x * move_x + move_z * move_z).sqrt();
    if horizontal_len > f64::EPSILON {
        move_x /= horizontal_len;
        move_z /= horizontal_len;
    }
    let use_speed_multiplier = local_player_using_item_speed_multiplier(world);
    move_x *= use_speed_multiplier;
    move_z *= use_speed_multiplier;

    let flying = local_player_flying_abilities(world);
    pose = local_player_update_body_pose(world, pose, input, flying);
    let initial_fluid_contact = local_player_fluid_contact(world, pose);
    if flying.is_none() && (initial_fluid_contact.in_water() || initial_fluid_contact.in_lava()) {
        return advance_local_player_fluid_physics_step(
            world,
            pose,
            input,
            step_seconds,
            move_x,
            move_z,
            initial_fluid_contact,
        );
    }
    let climbable_before = if flying.is_none() {
        local_player_climbable_kind(world, pose)
    } else {
        None
    };
    let on_climbable_before = climbable_before.is_some();
    if let Some(kind) = climbable_before {
        pose.fall_distance = 0.0;
        pose.delta_movement =
            local_player_climbable_limited_velocity(pose.delta_movement, input, kind);
    }
    let powder_snow_climb_out_before = flying.is_none()
        && world.local_player_can_walk_on_powder_snow()
        && local_player_inside_powder_snow(world, pose);

    let mut jump_horizontal_impulse = (0.0, 0.0);
    if flying.is_none() && input.focused && input.jump && pose.on_ground {
        let jump_velocity = local_player_jump_velocity(world, pose);
        if jump_velocity > 1.0e-5 {
            pose.delta_movement.y = pose.delta_movement.y.max(jump_velocity);
            if input.sprint {
                jump_horizontal_impulse = local_player_sprint_jump_horizontal_impulse(pose);
            }
        }
    }

    let step_ticks = step_seconds / LOCAL_PHYSICS_TICK_SECONDS;
    let (mut requested_x, mut requested_z, flying_horizontal_velocity) = match flying {
        Some(abilities) => {
            let (requested_x, requested_z, next_velocity_x, next_velocity_z) =
                local_player_flying_air_travel(
                    pose.delta_movement.x,
                    pose.delta_movement.z,
                    move_x,
                    move_z,
                    abilities,
                    input,
                    step_ticks,
                );
            (
                requested_x,
                requested_z,
                Some((next_velocity_x, next_velocity_z)),
            )
        }
        None => {
            let speed = local_player_horizontal_speed(world, pose, input);
            (
                move_x * speed * step_seconds + jump_horizontal_impulse.0 * step_ticks,
                move_z * speed * step_seconds + jump_horizontal_impulse.1 * step_ticks,
                None,
            )
        }
    };
    if on_climbable_before {
        let max_horizontal = LOCAL_PLAYER_CLIMBABLE_MAX_HORIZONTAL_VELOCITY_PER_TICK * step_ticks;
        requested_x = requested_x.clamp(-max_horizontal, max_horizontal);
        requested_z = requested_z.clamp(-max_horizontal, max_horizontal);
    }
    let mut requested_y = match flying {
        Some(abilities) => {
            let vertical_input = if input.focused {
                axis(input.jump, input.sneak)
            } else {
                0.0
            };
            (pose.delta_movement.y
                + vertical_input
                    * f64::from(abilities.flying_speed).max(0.0)
                    * LOCAL_INPUT_FLY_VERTICAL_SPEED_MULTIPLIER)
                * step_ticks
        }
        None => pose.delta_movement.y * step_ticks,
    };
    if flying.is_none() {
        if let Some(stuck_multiplier) = local_player_stuck_speed_multiplier(world, pose) {
            pose.fall_distance = 0.0;
            requested_x *= stuck_multiplier.x;
            requested_y *= stuck_multiplier.y;
            requested_z *= stuck_multiplier.z;
        }
    }
    if should_back_off_from_edge(world, pose, input, requested_y) {
        (requested_x, requested_z) = back_off_from_edge(world, pose, requested_x, requested_z);
    }
    let collision_context = LocalPlayerCollisionContext::for_pose(world, pose, input.sneak);
    let movement = clip_local_player_movement(
        world,
        pose,
        requested_x,
        requested_y,
        requested_z,
        pose.on_ground,
        collision_context,
    );
    pose.position.x += movement.x;
    pose.position.y += movement.y;
    pose.position.z += movement.z;

    let tick_span = step_ticks.max(f64::EPSILON);
    let x_collision = (movement.x - requested_x).abs() > COLLISION_EPSILON;
    let z_collision = (movement.z - requested_z).abs() > COLLISION_EPSILON;
    let horizontal_collision = x_collision || z_collision;
    let vertical_collision = (movement.y - requested_y).abs() > COLLISION_EPSILON;
    let on_ground = vertical_collision && requested_y < 0.0
        || local_player_supported(world, pose, collision_context);
    let bounced_on_slime = flying.is_none()
        && vertical_collision
        && requested_y < 0.0
        && !input.sneak
        && local_player_standing_on_block(world, pose, "minecraft:slime_block");
    let on_climbable_after = flying.is_none() && local_player_climbable_kind(world, pose).is_some();
    let climbable_upward_impulse =
        on_climbable_after && (horizontal_collision || input.focused && input.jump);
    let powder_snow_upward_impulse =
        powder_snow_climb_out_before && (horizontal_collision || input.focused && input.jump);
    let fluid_contact = local_player_fluid_contact(world, pose);
    pose.fall_distance = if fluid_contact.in_water() || on_climbable_before || on_climbable_after {
        0.0
    } else {
        next_local_player_fall_distance(pose.fall_distance, movement.y, on_ground)
    };

    pose.delta_movement = ProtocolVec3d {
        x: movement.x / tick_span,
        y: movement.y / tick_span,
        z: movement.z / tick_span,
    };
    if x_collision {
        pose.delta_movement.x = 0.0;
    }
    if z_collision {
        pose.delta_movement.z = 0.0;
    }
    if flying.is_some() {
        if let Some((next_velocity_x, next_velocity_z)) = flying_horizontal_velocity {
            if !x_collision {
                pose.delta_movement.x = next_velocity_x;
            }
            if !z_collision {
                pose.delta_movement.z = next_velocity_z;
            }
        }
        if vertical_collision {
            pose.delta_movement.y = 0.0;
        } else {
            pose.delta_movement.y *= LOCAL_INPUT_FLY_VERTICAL_DAMPING.powf(step_ticks);
        }
    } else if on_ground || vertical_collision {
        pose.delta_movement.y = if bounced_on_slime {
            (-requested_y / tick_span).max(0.0)
        } else {
            0.0
        };
    } else {
        let vertical_velocity = if climbable_upward_impulse || powder_snow_upward_impulse {
            LOCAL_PLAYER_CLIMBABLE_UPWARD_VELOCITY_PER_TICK
        } else {
            pose.delta_movement.y
        };
        pose.delta_movement.y =
            local_player_airborne_vertical_velocity(world, vertical_velocity, step_ticks);
    }
    pose.on_ground = on_ground;
    pose.horizontal_collision = horizontal_collision;
    pose
}

fn local_player_block_floor(value: f64) -> i32 {
    value.floor() as i32
}

fn scale_vec3(vec: ProtocolVec3d, scale: f64) -> ProtocolVec3d {
    ProtocolVec3d {
        x: vec.x * scale,
        y: vec.y * scale,
        z: vec.z * scale,
    }
}

fn normalized_vec3(vec: ProtocolVec3d) -> ProtocolVec3d {
    let length = vec3_length(vec);
    if length <= f64::EPSILON {
        ProtocolVec3d::default()
    } else {
        scale_vec3(vec, 1.0 / length)
    }
}

fn vec3_length(vec: ProtocolVec3d) -> f64 {
    vec3_length_squared(vec).sqrt()
}

fn vec3_length_squared(vec: ProtocolVec3d) -> f64 {
    vec.x * vec.x + vec.y * vec.y + vec.z * vec.z
}

fn clip_local_player_movement(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    requested_x: f64,
    requested_y: f64,
    requested_z: f64,
    on_ground: bool,
    collision_context: LocalPlayerCollisionContext,
) -> ProtocolVec3d {
    let clipped = clip_local_player_movement_without_step(
        world,
        pose,
        requested_x,
        requested_y,
        requested_z,
        collision_context,
    );
    if !on_ground
        || requested_y.abs() > COLLISION_EPSILON
        || ((clipped.x - requested_x).abs() <= COLLISION_EPSILON
            && (clipped.z - requested_z).abs() <= COLLISION_EPSILON)
    {
        return clipped;
    }

    let Some(stepped) = clip_local_player_step_up_movement(
        world,
        pose,
        requested_x,
        requested_z,
        collision_context,
    ) else {
        return clipped;
    };
    if horizontal_distance_sqr(stepped) > horizontal_distance_sqr(clipped) + COLLISION_EPSILON {
        stepped
    } else {
        clipped
    }
}

fn clip_local_player_movement_without_step(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    requested_x: f64,
    requested_y: f64,
    requested_z: f64,
    collision_context: LocalPlayerCollisionContext,
) -> ProtocolVec3d {
    let mut bounds = LocalPlayerBounds::for_pose(pose);
    let clipped_y = clip_axis_delta(world, bounds, Axis::Y, requested_y, collision_context);
    bounds = bounds.moved(0.0, clipped_y, 0.0);
    let clipped_x = clip_axis_delta(world, bounds, Axis::X, requested_x, collision_context);
    bounds = bounds.moved(clipped_x, 0.0, 0.0);
    let clipped_z = clip_axis_delta(world, bounds, Axis::Z, requested_z, collision_context);

    ProtocolVec3d {
        x: clipped_x,
        y: clipped_y,
        z: clipped_z,
    }
}

fn clip_local_player_step_up_movement(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    requested_x: f64,
    requested_z: f64,
    collision_context: LocalPlayerCollisionContext,
) -> Option<ProtocolVec3d> {
    let mut bounds = LocalPlayerBounds::for_pose(pose);
    let clipped_up = clip_axis_delta(
        world,
        bounds,
        Axis::Y,
        LOCAL_PLAYER_STEP_HEIGHT,
        collision_context,
    );
    if clipped_up <= COLLISION_EPSILON {
        return None;
    }
    bounds = bounds.moved(0.0, clipped_up, 0.0);
    let clipped_x = clip_axis_delta(world, bounds, Axis::X, requested_x, collision_context);
    bounds = bounds.moved(clipped_x, 0.0, 0.0);
    let clipped_z = clip_axis_delta(world, bounds, Axis::Z, requested_z, collision_context);
    bounds = bounds.moved(0.0, 0.0, clipped_z);
    let clipped_down = clip_axis_delta(world, bounds, Axis::Y, -clipped_up, collision_context);

    Some(ProtocolVec3d {
        x: clipped_x,
        y: clipped_up + clipped_down,
        z: clipped_z,
    })
}

fn horizontal_distance_sqr(movement: ProtocolVec3d) -> f64 {
    movement.x * movement.x + movement.z * movement.z
}

fn should_back_off_from_edge(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    requested_y: f64,
) -> bool {
    input.focused
        && input.sneak
        && !local_player_is_flying(world)
        && requested_y <= 0.0
        && local_player_is_above_ground(world, pose)
}

fn local_player_is_above_ground(world: &WorldStore, pose: LocalPlayerPoseState) -> bool {
    pose.on_ground
        || (pose.fall_distance < LOCAL_PLAYER_STEP_HEIGHT
            && !local_player_can_fall_at_least(
                world,
                pose,
                0.0,
                0.0,
                LOCAL_PLAYER_STEP_HEIGHT - pose.fall_distance,
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

fn local_player_update_body_pose(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    flying: Option<LocalPlayerAbilitiesState>,
) -> LocalPlayerPoseState {
    if flying.is_some() || world.local_player_vehicle_id().is_some() {
        return local_player_with_body_pose(pose, LocalPlayerBodyPose::Standing);
    }

    let swimming_pose = local_player_with_body_pose(pose, LocalPlayerBodyPose::Swimming);
    if !local_player_pose_fits(world, swimming_pose) {
        return pose;
    }

    let desired_pose =
        local_player_with_body_pose(pose, local_player_desired_body_pose(world, pose, input));
    if local_player_pose_fits(world, desired_pose) {
        return desired_pose;
    }

    let crouching_pose = local_player_with_body_pose(pose, LocalPlayerBodyPose::Crouching);
    if local_player_pose_fits(world, crouching_pose) {
        crouching_pose
    } else {
        swimming_pose
    }
}

fn local_player_pose_fits(world: &WorldStore, pose: LocalPlayerPoseState) -> bool {
    !local_player_collides(world, LocalPlayerBounds::for_pose(pose))
}

fn local_player_desired_body_pose(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
) -> LocalPlayerBodyPose {
    if local_player_should_swim(world, pose, input) {
        LocalPlayerBodyPose::Swimming
    } else if input.focused && input.sneak {
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
    if !input.focused || !input.sprint {
        return false;
    }

    let fluid_contact = local_player_fluid_contact(world, pose);
    fluid_contact.in_water() && (pose.swimming || fluid_contact.eye_in_water)
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
    if input.sprint && !local_player_can_sprint(world, input) {
        input.sprint = false;
    }
    input
}

pub(super) fn local_player_effective_sprint(
    world: &WorldStore,
    input: LocalPlayerInputState,
) -> bool {
    input.sprint && local_player_can_sprint(world, input)
}

fn local_player_can_sprint(world: &WorldStore, input: LocalPlayerInputState) -> bool {
    let sprint_source_is_eligible = if world.local_player_vehicle_id().is_some() {
        world.local_player_sprintable_vehicle_id().is_some()
    } else {
        local_player_has_enough_food_to_sprint(world)
    };

    input.focused
        && input.forward
        && !input.backward
        && local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_BLINDNESS_ID).is_none()
        && sprint_source_is_eligible
        && !local_player_is_slow_due_to_using_item(world)
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
        .map(|effects| f64::from(effects.speed_multiplier))
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

fn local_player_horizontal_speed(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
) -> f64 {
    let movement_speed_scale = local_player_movement_speed_attribute_value(world)
        / LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE;
    let mut speed = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * movement_speed_scale;
    if input.sprint {
        speed *= LOCAL_INPUT_SPRINT_SPEED_MULTIPLIER;
    }
    if pose.sneaking || pose.swimming {
        speed *= local_player_attribute_value(world, VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID)
            .unwrap_or(LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER)
            .clamp(0.0, 1.0);
    }
    speed *= local_player_block_speed_factor(world, pose);
    speed
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
    let here_block = world.probe_block(here_pos);
    let here_factor = here_block
        .as_ref()
        .and_then(|block| block.block_name.as_deref())
        .map(block_speed_factor)
        .unwrap_or(DEFAULT_BLOCK_SPEED_FACTOR);
    if here_block
        .as_ref()
        .and_then(|block| block.block_name.as_deref())
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
        .probe_block(below_pos)
        .and_then(|block| block.block_name.as_deref().map(block_speed_factor))
        .unwrap_or(DEFAULT_BLOCK_SPEED_FACTOR)
}

fn local_player_stuck_speed_multiplier(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
) -> Option<ProtocolVec3d> {
    let bounds =
        LocalPlayerBounds::for_pose(pose).deflated(LOCAL_PLAYER_ENTITY_INSIDE_BLOCK_EPSILON);
    let min_x = local_player_block_floor(bounds.min_x());
    let max_x = local_player_block_floor(bounds.max_x());
    let min_y = local_player_block_floor(bounds.min_y());
    let max_y = local_player_block_floor(bounds.max_y());
    let min_z = local_player_block_floor(bounds.min_z());
    let max_z = local_player_block_floor(bounds.max_z());

    let mut berry_bush = false;
    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let Some(block_name) = world
                    .probe_block(BlockPos { x, y, z })
                    .and_then(|block| block.block_name)
                else {
                    continue;
                };
                match block_name.as_str() {
                    "minecraft:cobweb" => return Some(local_player_cobweb_stuck_multiplier(world)),
                    "minecraft:sweet_berry_bush" => berry_bush = true,
                    _ => {}
                }
            }
        }
    }

    berry_bush.then_some(ProtocolVec3d {
        x: 0.8,
        y: 0.75,
        z: 0.8,
    })
}

fn local_player_cobweb_stuck_multiplier(world: &WorldStore) -> ProtocolVec3d {
    if local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_WEAVING_ID).is_some() {
        ProtocolVec3d {
            x: 0.5,
            y: 0.25,
            z: 0.5,
        }
    } else {
        ProtocolVec3d {
            x: 0.25,
            y: 0.05,
            z: 0.25,
        }
    }
}

fn block_speed_factor(block_name: &str) -> f64 {
    match block_name {
        "minecraft:soul_sand" | "minecraft:honey_block" => SLOW_BLOCK_SPEED_FACTOR,
        _ => DEFAULT_BLOCK_SPEED_FACTOR,
    }
}

fn local_player_movement_speed_attribute_value(world: &WorldStore) -> f64 {
    let mut speed = local_player_attribute_value(world, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
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

fn local_player_powder_snow_speed_modifier(world: &WorldStore) -> f64 {
    if local_player_attribute_has_modifier(
        world,
        VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
        VANILLA_POWDER_SNOW_SPEED_MODIFIER_ID,
    ) {
        return 0.0;
    }

    let ticks_frozen = world
        .local_player_id
        .and_then(|id| world.entities.ticks_frozen(id))
        .unwrap_or(0)
        .max(0);
    let frozen_percent = (f64::from(ticks_frozen.min(LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE))
        / f64::from(LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE))
    .clamp(0.0, 1.0);
    POWDER_SNOW_MOVEMENT_SPEED_MODIFIER_AT_FULL_FREEZE * frozen_percent
}

fn local_player_update_powder_snow_freezing(world: &mut WorldStore, pose: LocalPlayerPoseState) {
    let Some(local_player_id) = world.local_player_id else {
        return;
    };
    let Some(current_ticks) = world.entities.ticks_frozen(local_player_id) else {
        return;
    };

    let next_ticks =
        if local_player_inside_powder_snow(world, pose) && local_player_can_freeze(world) {
            (current_ticks + 1).min(LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE)
        } else {
            (current_ticks - 2).max(0)
        };
    if next_ticks == current_ticks {
        return;
    }

    let _ =
        world
            .entities
            .with_metadata_mut(local_player_id, |metadata| {
                if let Some(value) = metadata.data_values.iter_mut().find(|value| {
                    value.data_id == crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID
                }) {
                    value.serializer_id = VANILLA_ENTITY_DATA_INT_SERIALIZER_ID;
                    value.value = ProtocolEntityDataValueKind::Int(next_ticks);
                } else {
                    metadata.data_values.push(ProtocolEntityDataValue {
                        data_id: crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
                        serializer_id: VANILLA_ENTITY_DATA_INT_SERIALIZER_ID,
                        value: ProtocolEntityDataValueKind::Int(next_ticks),
                    });
                }
                metadata.data_values.sort_by_key(|value| value.data_id);
            });
}

fn local_player_inside_powder_snow(world: &WorldStore, pose: LocalPlayerPoseState) -> bool {
    let pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y),
        z: local_player_block_floor(pose.position.z),
    };
    world
        .probe_block(pos)
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
        .probe_block(here_pos)
        .and_then(|block| block.block_name.as_deref().map(block_jump_factor))
        .unwrap_or(DEFAULT_BLOCK_JUMP_FACTOR);
    if (here_factor - DEFAULT_BLOCK_JUMP_FACTOR).abs() > f64::EPSILON {
        return here_factor;
    }

    let below_pos = BlockPos {
        y: local_player_block_floor(pose.position.y - 0.500001),
        ..here_pos
    };
    world
        .probe_block(below_pos)
        .and_then(|block| block.block_name.as_deref().map(block_jump_factor))
        .unwrap_or(DEFAULT_BLOCK_JUMP_FACTOR)
}

fn block_jump_factor(block_name: &str) -> f64 {
    match block_name {
        "minecraft:honey_block" => HONEY_BLOCK_JUMP_FACTOR,
        _ => DEFAULT_BLOCK_JUMP_FACTOR,
    }
}

fn local_player_standing_on_block(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    name: &str,
) -> bool {
    let pos = BlockPos {
        x: local_player_block_floor(pose.position.x),
        y: local_player_block_floor(pose.position.y - SUPPORT_EPSILON),
        z: local_player_block_floor(pose.position.z),
    };
    world
        .probe_block(pos)
        .and_then(|block| block.block_name)
        .is_some_and(|block_name| block_name == name)
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
    let Some(block) = world.probe_block(pos) else {
        return None;
    };
    let block_name = block.block_name.as_deref()?;
    if block_name == "minecraft:scaffolding" {
        return Some(LocalPlayerClimbableKind::Scaffolding);
    }
    if is_climbable_block_name(block_name)
        || local_player_trapdoor_usable_as_ladder(world, pos, &block)
    {
        return Some(LocalPlayerClimbableKind::Ordinary);
    }
    None
}

fn is_climbable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:ladder"
            | "minecraft:vine"
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
    block: &BlockProbe,
) -> bool {
    if !block
        .block_name
        .as_deref()
        .is_some_and(is_trapdoor_block_name)
    {
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
    let Some(below_block) = world.probe_block(below_pos) else {
        return false;
    };
    below_block.block_name.as_deref() == Some("minecraft:ladder")
        && below_block
            .block_properties
            .get("facing")
            .map(String::as_str)
            == Some(facing)
}

fn is_trapdoor_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_trapdoor"))
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
    current_velocity: f64,
    step_ticks: f64,
) -> f64 {
    let mut velocity = current_velocity;
    if let Some(amplifier) = local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_LEVITATION_ID)
    {
        let target = amplified_effect_amount(amplifier, LEVITATION_TARGET_VELOCITY_PER_LEVEL);
        velocity += (target - velocity) * LEVITATION_APPROACH_FACTOR_PER_TICK * step_ticks;
    } else {
        velocity -= local_player_effective_gravity_per_tick(world, current_velocity) * step_ticks;
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

fn local_player_flying_air_travel(
    current_velocity_x: f64,
    current_velocity_z: f64,
    move_x: f64,
    move_z: f64,
    abilities: LocalPlayerAbilitiesState,
    input: LocalPlayerInputState,
    step_ticks: f64,
) -> (f64, f64, f64, f64) {
    let speed = local_player_flying_air_acceleration(abilities, input);
    let velocity_x = current_velocity_x + move_x * speed;
    let velocity_z = current_velocity_z + move_z * speed;
    let drag = LOCAL_INPUT_FLY_AIR_DRAG.powf(step_ticks);
    (
        velocity_x * step_ticks,
        velocity_z * step_ticks,
        velocity_x * drag,
        velocity_z * drag,
    )
}

fn local_player_flying_air_acceleration(
    abilities: LocalPlayerAbilitiesState,
    input: LocalPlayerInputState,
) -> f64 {
    let mut speed = f64::from(abilities.flying_speed).max(0.0);
    if input.sprint {
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

fn back_off_from_edge(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    requested_x: f64,
    requested_z: f64,
) -> (f64, f64) {
    let mut backed_x = requested_x;
    let mut backed_z = requested_z;
    let step_x = backed_x.signum() * EDGE_BACKOFF_STEP;
    let step_z = backed_z.signum() * EDGE_BACKOFF_STEP;

    while backed_x != 0.0
        && local_player_can_fall_at_least(world, pose, backed_x, 0.0, LOCAL_PLAYER_STEP_HEIGHT)
    {
        if backed_x.abs() <= EDGE_BACKOFF_STEP {
            backed_x = 0.0;
            break;
        }
        backed_x -= step_x;
    }

    while backed_z != 0.0
        && local_player_can_fall_at_least(world, pose, 0.0, backed_z, LOCAL_PLAYER_STEP_HEIGHT)
    {
        if backed_z.abs() <= EDGE_BACKOFF_STEP {
            backed_z = 0.0;
            break;
        }
        backed_z -= step_z;
    }

    while backed_x != 0.0
        && backed_z != 0.0
        && local_player_can_fall_at_least(world, pose, backed_x, backed_z, LOCAL_PLAYER_STEP_HEIGHT)
    {
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
) -> bool {
    !local_player_collides(
        world,
        LocalPlayerBounds::for_pose(pose)
            .moved(delta_x, 0.0, delta_z)
            .edge_support_probe(min_height),
    )
}

fn clip_axis_delta(
    world: &WorldStore,
    bounds: LocalPlayerBounds,
    axis: Axis,
    requested: f64,
    collision_context: LocalPlayerCollisionContext,
) -> f64 {
    if requested.abs() <= COLLISION_EPSILON {
        return 0.0;
    }
    if !local_player_collides_with_context(
        world,
        bounds.swept_axis(axis, requested),
        collision_context,
    ) {
        return requested;
    }

    let mut low = 0.0;
    let mut high = requested;
    for _ in 0..COLLISION_CLIP_STEPS {
        let midpoint = (low + high) * 0.5;
        if local_player_collides_with_context(
            world,
            bounds.swept_axis(axis, midpoint),
            collision_context,
        ) {
            high = midpoint;
        } else {
            low = midpoint;
        }
    }
    if low.abs() <= COLLISION_EPSILON {
        0.0
    } else {
        low
    }
}

fn local_player_supported(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    collision_context: LocalPlayerCollisionContext,
) -> bool {
    local_player_collides_with_context(
        world,
        LocalPlayerBounds::for_pose(pose).moved(0.0, -SUPPORT_EPSILON, 0.0),
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
