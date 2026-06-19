use bbb_protocol::packets::{
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind as ProtocolEntityDataValueKind,
    Vec3d as ProtocolVec3d,
};

use super::local_player::{LocalPlayerAbilitiesState, LocalPlayerInputState, LocalPlayerPoseState};
use super::local_player_collision::{
    local_player_block_collision_is_empty, local_player_collides,
    local_player_collides_with_context, CollisionAxis as Axis, LocalPlayerBounds,
    LocalPlayerCollisionContext, COLLISION_EPSILON,
};
use super::local_player_fluid::{
    local_player_bounds_contains_any_fluid, local_player_fluid_contact,
    LocalPlayerFluidContactState,
};
use crate::{BlockPos, BlockProbe, WorldStore};

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
    let requested_y = match flying {
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

fn advance_local_player_fluid_physics_step(
    world: &WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    step_seconds: f64,
    move_x: f64,
    move_z: f64,
    initial_fluid_contact: LocalPlayerFluidContactState,
) -> LocalPlayerPoseState {
    let input = local_player_effective_movement_input(world, input);
    let step_ticks = step_seconds / LOCAL_PHYSICS_TICK_SECONDS;
    let old_y = pose.position.y;
    pose.delta_movement = local_player_velocity_with_fluid_current(
        pose.delta_movement,
        initial_fluid_contact.water_current,
        initial_fluid_contact.water_current_count,
        LOCAL_INPUT_WATER_CURRENT_PUSH_PER_TICK,
        step_ticks,
    );
    pose.delta_movement = local_player_velocity_with_fluid_current(
        pose.delta_movement,
        initial_fluid_contact.lava_current,
        initial_fluid_contact.lava_current_count,
        local_player_lava_current_push_per_tick(world),
        step_ticks,
    );
    pose.delta_movement = local_player_swimming_vertical_velocity(
        world,
        pose,
        input,
        initial_fluid_contact,
        pose.delta_movement,
    );

    if input.focused && input.jump {
        pose.delta_movement.y += LOCAL_INPUT_LIQUID_JUMP_VELOCITY_PER_TICK * step_ticks;
    }
    if input.focused && input.sneak && initial_fluid_contact.in_water() {
        pose.delta_movement.y -= LOCAL_INPUT_WATER_SNEAK_DESCEND_VELOCITY_PER_TICK * step_ticks;
    }

    let fluid_relative_speed = if initial_fluid_contact.in_water() {
        local_player_water_relative_speed_per_tick(world, pose.on_ground)
    } else {
        LOCAL_INPUT_FLUID_SPEED_PER_TICK
    };
    pose.delta_movement.x += move_x * fluid_relative_speed * step_ticks;
    pose.delta_movement.z += move_z * fluid_relative_speed * step_ticks;
    let is_falling = pose.delta_movement.y <= 0.0;
    let requested_x = pose.delta_movement.x * step_ticks;
    let requested_y = pose.delta_movement.y * step_ticks;
    let requested_z = pose.delta_movement.z * step_ticks;
    let collision_context = LocalPlayerCollisionContext::for_pose(world, pose, input.sneak);
    let movement = clip_local_player_movement_without_step(
        world,
        pose,
        requested_x,
        requested_y,
        requested_z,
        collision_context,
    );
    pose.position.x += movement.x;
    pose.position.y += movement.y;
    pose.position.z += movement.z;

    let tick_span = step_ticks.max(f64::EPSILON);
    let horizontal_collision = (movement.x - requested_x).abs() > COLLISION_EPSILON
        || (movement.z - requested_z).abs() > COLLISION_EPSILON;
    let vertical_collision = (movement.y - requested_y).abs() > COLLISION_EPSILON;
    let on_ground = vertical_collision && requested_y < 0.0
        || local_player_supported(world, pose, collision_context);
    let fluid_contact = local_player_fluid_contact(world, pose);
    pose.fall_distance = if fluid_contact.in_water() {
        0.0
    } else {
        next_local_player_fall_distance(pose.fall_distance, movement.y, on_ground)
    };

    pose.delta_movement = ProtocolVec3d {
        x: movement.x / tick_span,
        y: movement.y / tick_span,
        z: movement.z / tick_span,
    };
    if horizontal_collision {
        if (movement.x - requested_x).abs() > COLLISION_EPSILON {
            pose.delta_movement.x = 0.0;
        }
        if (movement.z - requested_z).abs() > COLLISION_EPSILON {
            pose.delta_movement.z = 0.0;
        }
    }
    if vertical_collision {
        pose.delta_movement.y = 0.0;
    }

    if initial_fluid_contact.in_water() {
        let horizontal_drag = water_horizontal_drag(world, input, pose.on_ground);
        pose.delta_movement.x *= horizontal_drag.powf(step_ticks);
        pose.delta_movement.y *= LOCAL_INPUT_WATER_VERTICAL_DRAG.powf(step_ticks);
        pose.delta_movement.z *= horizontal_drag.powf(step_ticks);
        pose.delta_movement = local_player_fluid_falling_adjusted_velocity(
            world,
            pose.delta_movement,
            is_falling,
            input.sprint,
            step_ticks,
        );
    } else {
        pose.delta_movement = local_player_lava_velocity_after_travel(
            world,
            pose.delta_movement,
            initial_fluid_contact.lava_height,
            is_falling,
            input.sprint,
            step_ticks,
        );
    }
    pose.delta_movement = local_player_jump_out_of_fluid_velocity(
        world,
        pose,
        old_y,
        pose.delta_movement,
        horizontal_collision,
    );
    if let Some(contact) = local_player_bubble_column_contact(world, pose) {
        pose.delta_movement =
            local_player_bubble_column_velocity(pose.delta_movement, contact, step_ticks);
        if matches!(contact, BubbleColumnContact::Inside { .. }) {
            pose.fall_distance = 0.0;
        }
    }

    pose.on_ground = on_ground;
    pose.horizontal_collision = horizontal_collision;
    pose
}

fn local_player_swimming_vertical_velocity(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    fluid_contact: LocalPlayerFluidContactState,
    mut velocity: ProtocolVec3d,
) -> ProtocolVec3d {
    if !pose.swimming || !fluid_contact.in_water() {
        return velocity;
    }

    let look_y = local_player_look_direction_y(pose);
    if look_y <= 0.0
        || input.focused && input.jump
        || local_player_has_fluid_at_swim_head(world, pose)
    {
        let approach = if look_y < LOCAL_INPUT_SWIM_LOOK_DOWN_THRESHOLD {
            LOCAL_INPUT_SWIM_LOOK_DOWN_VERTICAL_APPROACH
        } else {
            LOCAL_INPUT_SWIM_VERTICAL_APPROACH
        };
        velocity.y += (look_y - velocity.y) * approach;
    }

    velocity
}

fn local_player_look_direction_y(pose: LocalPlayerPoseState) -> f64 {
    -f64::from(pose.x_rot).to_radians().sin()
}

fn local_player_has_fluid_at_swim_head(world: &WorldStore, pose: LocalPlayerPoseState) -> bool {
    world
        .probe_block(BlockPos {
            x: local_player_block_floor(pose.position.x),
            y: local_player_block_floor(pose.position.y + LOCAL_INPUT_SWIM_HEAD_FLUID_OFFSET),
            z: local_player_block_floor(pose.position.z),
        })
        .is_some_and(|block| block.fluid.is_some())
}

fn local_player_block_floor(value: f64) -> i32 {
    value.floor() as i32
}

fn local_player_velocity_with_fluid_current(
    mut velocity: ProtocolVec3d,
    accumulated_current: ProtocolVec3d,
    current_count: u32,
    push_per_tick: f64,
    step_ticks: f64,
) -> ProtocolVec3d {
    if current_count == 0
        || vec3_length_squared(accumulated_current)
            < LOCAL_INPUT_FLUID_CURRENT_APPLY_THRESHOLD_SQUARED
    {
        return velocity;
    }

    let mut impulse = ProtocolVec3d {
        x: accumulated_current.x / f64::from(current_count),
        y: accumulated_current.y / f64::from(current_count),
        z: accumulated_current.z / f64::from(current_count),
    };
    impulse = scale_vec3(impulse, push_per_tick * step_ticks);

    let min_horizontal_velocity =
        LOCAL_INPUT_FLUID_CURRENT_MIN_HORIZONTAL_VELOCITY_PER_TICK * step_ticks;
    let min_push = LOCAL_INPUT_FLUID_CURRENT_MIN_PUSH_PER_TICK * step_ticks;
    if velocity.x.abs() < min_horizontal_velocity
        && velocity.z.abs() < min_horizontal_velocity
        && vec3_length(impulse) < min_push
    {
        impulse = scale_vec3(normalized_vec3(impulse), min_push);
    }

    velocity.x += impulse.x;
    velocity.y += impulse.y;
    velocity.z += impulse.z;
    velocity
}

fn local_player_jump_out_of_fluid_velocity(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    old_y: f64,
    velocity: ProtocolVec3d,
    horizontal_collision: bool,
) -> ProtocolVec3d {
    let jump_clearance_y = velocity.y + LOCAL_PLAYER_STEP_HEIGHT - pose.position.y + old_y;
    let clear_bounds =
        LocalPlayerBounds::for_pose(pose).moved(velocity.x, jump_clearance_y, velocity.z);
    if horizontal_collision
        && !local_player_collides(world, clear_bounds)
        && !local_player_bounds_contains_any_fluid(world, clear_bounds)
    {
        ProtocolVec3d {
            y: LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK,
            ..velocity
        }
    } else {
        velocity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BubbleColumnContact {
    Inside { drag_down: bool },
    Above { drag_down: bool },
}

fn local_player_bubble_column_velocity(
    mut velocity: ProtocolVec3d,
    contact: BubbleColumnContact,
    step_ticks: f64,
) -> ProtocolVec3d {
    velocity.y = match contact {
        BubbleColumnContact::Inside { drag_down: true } => (velocity.y
            - LOCAL_INPUT_BUBBLE_COLUMN_DRAG_DOWN_PER_TICK * step_ticks)
            .max(LOCAL_INPUT_BUBBLE_COLUMN_INSIDE_DRAG_DOWN_LIMIT),
        BubbleColumnContact::Inside { drag_down: false } => (velocity.y
            + LOCAL_INPUT_BUBBLE_COLUMN_INSIDE_PUSH_UP_PER_TICK * step_ticks)
            .min(LOCAL_INPUT_BUBBLE_COLUMN_INSIDE_PUSH_UP_LIMIT),
        BubbleColumnContact::Above { drag_down: true } => (velocity.y
            - LOCAL_INPUT_BUBBLE_COLUMN_DRAG_DOWN_PER_TICK * step_ticks)
            .max(LOCAL_INPUT_BUBBLE_COLUMN_ABOVE_DRAG_DOWN_LIMIT),
        BubbleColumnContact::Above { drag_down: false } => (velocity.y
            + LOCAL_INPUT_BUBBLE_COLUMN_ABOVE_PUSH_UP_PER_TICK * step_ticks)
            .min(LOCAL_INPUT_BUBBLE_COLUMN_ABOVE_PUSH_UP_LIMIT),
    };
    velocity
}

fn local_player_bubble_column_contact(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
) -> Option<BubbleColumnContact> {
    let bounds = LocalPlayerBounds::for_pose(pose);
    let min_x = bounds.min_x().floor() as i32;
    let max_x = bounds.max_x().ceil() as i32;
    let min_y = bounds.min_y().floor() as i32;
    let max_y = bounds.max_y().ceil() as i32;
    let min_z = bounds.min_z().floor() as i32;
    let max_z = bounds.max_z().ceil() as i32;
    let mut inside = None;

    for y in min_y..max_y {
        for z in min_z..max_z {
            for x in min_x..max_x {
                let pos = BlockPos { x, y, z };
                let Some(block) = world.probe_block(pos) else {
                    continue;
                };
                if block.block_name.as_deref() != Some("minecraft:bubble_column") {
                    continue;
                }
                let drag_down = block
                    .block_properties
                    .get("drag")
                    .map_or(true, |value| value == "true");
                if bubble_column_has_open_above(world, pos) {
                    return Some(BubbleColumnContact::Above { drag_down });
                }
                inside = Some(BubbleColumnContact::Inside { drag_down });
            }
        }
    }

    inside
}

fn bubble_column_has_open_above(world: &WorldStore, pos: BlockPos) -> bool {
    let Some(y) = pos.y.checked_add(1) else {
        return false;
    };
    world
        .probe_block(BlockPos { y, ..pos })
        .is_some_and(|block| block.fluid.is_none() && local_player_block_collision_is_empty(&block))
}

fn local_player_lava_current_push_per_tick(world: &WorldStore) -> f64 {
    if world.level_info().is_some_and(|level| {
        level.dimension == "minecraft:the_nether"
            || level.dimension_type_name.as_deref() == Some("minecraft:the_nether")
    }) {
        LOCAL_INPUT_FAST_LAVA_CURRENT_PUSH_PER_TICK
    } else {
        LOCAL_INPUT_LAVA_CURRENT_PUSH_PER_TICK
    }
}

fn local_player_water_relative_speed_per_tick(world: &WorldStore, on_ground: bool) -> f64 {
    let mut speed = LOCAL_INPUT_FLUID_SPEED_PER_TICK;
    let efficiency = local_player_water_movement_efficiency(world, on_ground);
    if efficiency > 0.0 {
        speed += (local_player_movement_speed_attribute_value(world) - speed) * efficiency;
    }
    speed
}

fn water_horizontal_drag(world: &WorldStore, input: LocalPlayerInputState, on_ground: bool) -> f64 {
    let mut drag = if input.sprint {
        LOCAL_INPUT_SPRINTING_WATER_HORIZONTAL_DRAG
    } else {
        LOCAL_INPUT_WATER_HORIZONTAL_DRAG
    };
    let efficiency = local_player_water_movement_efficiency(world, on_ground);
    if efficiency > 0.0 {
        drag += (LOCAL_INPUT_WATER_EFFICIENCY_HORIZONTAL_DRAG_TARGET - drag) * efficiency;
    }
    if local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID).is_some() {
        drag = LOCAL_INPUT_DOLPHINS_GRACE_WATER_HORIZONTAL_DRAG;
    }
    drag
}

fn local_player_water_movement_efficiency(world: &WorldStore, on_ground: bool) -> f64 {
    let mut efficiency =
        local_player_attribute_value(world, VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
    if !on_ground {
        efficiency *= 0.5;
    }
    efficiency
}

fn local_player_lava_velocity_after_travel(
    world: &WorldStore,
    mut velocity: ProtocolVec3d,
    lava_height: f64,
    is_falling: bool,
    sprinting: bool,
    step_ticks: f64,
) -> ProtocolVec3d {
    if lava_height <= LOCAL_PLAYER_FLUID_JUMP_THRESHOLD {
        velocity.x *= LOCAL_INPUT_LAVA_HORIZONTAL_DRAG.powf(step_ticks);
        velocity.y *= LOCAL_INPUT_LAVA_VERTICAL_DRAG.powf(step_ticks);
        velocity.z *= LOCAL_INPUT_LAVA_HORIZONTAL_DRAG.powf(step_ticks);
        velocity = local_player_fluid_falling_adjusted_velocity(
            world, velocity, is_falling, sprinting, step_ticks,
        );
    } else {
        velocity.x *= LOCAL_INPUT_LAVA_DEEP_DRAG.powf(step_ticks);
        velocity.y *= LOCAL_INPUT_LAVA_DEEP_DRAG.powf(step_ticks);
        velocity.z *= LOCAL_INPUT_LAVA_DEEP_DRAG.powf(step_ticks);
    }

    velocity.y -= local_player_effective_gravity_per_tick(world, velocity.y)
        * LOCAL_INPUT_LAVA_GRAVITY_SCALE
        * step_ticks;
    velocity
}

fn local_player_fluid_falling_adjusted_velocity(
    world: &WorldStore,
    mut velocity: ProtocolVec3d,
    is_falling: bool,
    sprinting: bool,
    step_ticks: f64,
) -> ProtocolVec3d {
    if sprinting {
        return velocity;
    }
    let gravity_step = local_player_effective_gravity_per_tick(world, velocity.y)
        * LOCAL_INPUT_FLUID_FALLING_GRAVITY_SCALE
        * step_ticks;
    if gravity_step == 0.0 {
        return velocity;
    }
    velocity.y = if is_falling
        && (velocity.y - 0.005 * step_ticks).abs() >= 0.003 * step_ticks
        && (velocity.y - gravity_step).abs() < 0.003 * step_ticks
    {
        -0.003 * step_ticks
    } else {
        velocity.y - gravity_step
    };
    velocity
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
        AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind as ProtocolEntityDataValueKind, GameEvent as ProtocolGameEvent,
        ItemStackSummary as ProtocolItemStackSummary, MobEffectFlags as ProtocolMobEffectFlags,
        PlayerHealth as ProtocolPlayerHealth, SetEntityData as ProtocolSetEntityData,
        SetPassengers as ProtocolSetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
        UpdateAttributes as ProtocolUpdateAttributes, UpdateMobEffect as ProtocolUpdateMobEffect,
    };
    use uuid::Uuid;

    use crate::{
        entities::{
            VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
        },
        BlockPos, ChunkColumn, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension, WorldLevelInfo,
    };

    const AIR_BLOCK_STATE_ID: i32 = 0;
    const GRASS_BLOCK_STATE_ID: i32 = 9;
    const OAK_TOP_SLAB_BLOCK_STATE_ID: i32 = 13331;
    const OAK_BOTTOM_SLAB_BLOCK_STATE_ID: i32 = 13333;
    const OAK_TOP_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID: i32 = 3928;
    const OAK_BOTTOM_STRAIGHT_NORTH_STAIR_BLOCK_STATE_ID: i32 = 3918;
    const OAK_BOTTOM_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID: i32 = 3938;
    const OAK_LEAVES_BLOCK_STATE_ID: i32 = 255;
    const PISTON_EXTENDED_NORTH_BLOCK_STATE_ID: i32 = 2257;
    const PISTON_EXTENDED_UP_BLOCK_STATE_ID: i32 = 2261;
    const PISTON_HEAD_SOUTH_LONG_BLOCK_STATE_ID: i32 = 2279;
    const MOVING_PISTON_NORTH_BLOCK_STATE_ID: i32 = 2309;
    const FARMLAND_MOISTURE_0_BLOCK_STATE_ID: i32 = 5319;
    const SNOW_5_LAYERS_BLOCK_STATE_ID: i32 = 6923;
    const SNOW_6_LAYERS_BLOCK_STATE_ID: i32 = 6924;
    const CACTUS_AGE_0_BLOCK_STATE_ID: i32 = 6929;
    const SOUL_SAND_BLOCK_STATE_ID: i32 = 6998;
    const CAKE_BITES_0_BLOCK_STATE_ID: i32 = 7027;
    const CAKE_BITES_3_BLOCK_STATE_ID: i32 = 7030;
    const END_PORTAL_FRAME_EYE_NORTH_BLOCK_STATE_ID: i32 = 9469;
    const END_PORTAL_FRAME_EMPTY_NORTH_BLOCK_STATE_ID: i32 = 9473;
    const DRAGON_EGG_BLOCK_STATE_ID: i32 = 9478;
    const SLIME_BLOCK_STATE_ID: i32 = 12532;
    const FLOWER_POT_BLOCK_STATE_ID: i32 = 10629;
    const POTTED_DANDELION_BLOCK_STATE_ID: i32 = 10641;
    const SKELETON_SKULL_BLOCK_STATE_ID: i32 = 10931;
    const SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID: i32 = 10948;
    const PIGLIN_HEAD_BLOCK_STATE_ID: i32 = 11171;
    const PIGLIN_WALL_HEAD_NORTH_BLOCK_STATE_ID: i32 = 11188;
    const OAK_CLOSED_NORTH_DOOR_BLOCK_STATE_ID: i32 = 5666;
    const OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID: i32 = 7121;
    const OAK_TOP_OPEN_NORTH_TRAPDOOR_BLOCK_STATE_ID: i32 = 7117;
    const OAK_TOP_OPEN_SOUTH_TRAPDOOR_BLOCK_STATE_ID: i32 = 7133;
    const STONE_PRESSURE_PLATE_BLOCK_STATE_ID: i32 = 6796;
    const OAK_NORTH_FENCE_BLOCK_STATE_ID: i32 = 6988;
    const OAK_CLOSED_NORTH_FENCE_GATE_BLOCK_STATE_ID: i32 = 8653;
    const OAK_OPEN_NORTH_FENCE_GATE_BLOCK_STATE_ID: i32 = 8651;
    const LILY_PAD_BLOCK_STATE_ID: i32 = 8920;
    const GLASS_NORTH_PANE_BLOCK_STATE_ID: i32 = 8323;
    const WHITE_CARPET_BLOCK_STATE_ID: i32 = 12896;
    const COBBLESTONE_NORTH_EAST_WALL_BLOCK_STATE_ID: i32 = 10236;
    const IRON_CHAIN_Y_AXIS_BLOCK_STATE_ID: i32 = 8249;
    const LADDER_NORTH_BLOCK_STATE_ID: i32 = 5720;
    const LADDER_SOUTH_BLOCK_STATE_ID: i32 = 5722;
    const END_ROD_NORTH_BLOCK_STATE_ID: i32 = 14636;
    const DIRT_PATH_BLOCK_STATE_ID: i32 = 14815;
    const SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID: i32 = 20707;
    const SCAFFOLDING_BOTTOM_DISTANCE_1_BLOCK_STATE_ID: i32 = 20709;
    const LANTERN_STANDING_BLOCK_STATE_ID: i32 = 20840;
    const CAMPFIRE_NORTH_LIT_BLOCK_STATE_ID: i32 = 20880;
    const HONEY_BLOCK_STATE_ID: i32 = 21816;
    const POWDER_SNOW_BLOCK_STATE_ID: i32 = 24689;
    const FREEZE_IMMUNE_WEARABLE_ITEM_ID: i32 = 42;
    const LEATHER_BOOTS_ITEM_ID: i32 = 43;
    const PLAYER_FEET_EQUIPMENT_SLOT_ID: i32 = 36;
    const CHEST_SINGLE_NORTH_BLOCK_STATE_ID: i32 = 3988;
    const CHEST_LEFT_NORTH_BLOCK_STATE_ID: i32 = 3990;
    const TRAPPED_CHEST_RIGHT_NORTH_BLOCK_STATE_ID: i32 = 11212;
    const ENDER_CHEST_NORTH_BLOCK_STATE_ID: i32 = 9576;
    const WHITE_BED_NORTH_FOOT_BLOCK_STATE_ID: i32 = 1934;
    const WATER_CAULDRON_LEVEL_3_BLOCK_STATE_ID: i32 = 9463;
    const DAYLIGHT_DETECTOR_POWER_0_BLOCK_STATE_ID: i32 = 11295;
    const HOPPER_NORTH_ENABLED_BLOCK_STATE_ID: i32 = 11314;
    const ENCHANTING_TABLE_BLOCK_STATE_ID: i32 = 9451;
    const GRINDSTONE_FLOOR_NORTH_BLOCK_STATE_ID: i32 = 20772;
    const GRINDSTONE_WALL_NORTH_BLOCK_STATE_ID: i32 = 20776;
    const GRINDSTONE_CEILING_NORTH_BLOCK_STATE_ID: i32 = 20780;
    const LECTERN_NORTH_NO_BOOK_BLOCK_STATE_ID: i32 = 20787;
    const BELL_FLOOR_NORTH_BLOCK_STATE_ID: i32 = 20806;
    const BELL_CEILING_NORTH_BLOCK_STATE_ID: i32 = 20814;
    const BELL_SINGLE_WALL_NORTH_BLOCK_STATE_ID: i32 = 20822;
    const BELL_DOUBLE_WALL_NORTH_BLOCK_STATE_ID: i32 = 20830;
    const BREWING_STAND_EMPTY_BLOCK_STATE_ID: i32 = 9459;
    const STONECUTTER_NORTH_BLOCK_STATE_ID: i32 = 20801;
    const ANVIL_NORTH_BLOCK_STATE_ID: i32 = 11195;
    const COMPOSTER_LEVEL_7_BLOCK_STATE_ID: i32 = 21750;
    const COPPER_GRATE_BLOCK_STATE_ID: i32 = 27048;
    const WAXED_COPPER_GRATE_BLOCK_STATE_ID: i32 = 27056;
    const SCULK_SENSOR_INACTIVE_BLOCK_STATE_ID: i32 = 24691;
    const CALIBRATED_SCULK_SENSOR_NORTH_INACTIVE_BLOCK_STATE_ID: i32 = 24787;
    const SCULK_SHRIEKER_IDLE_BLOCK_STATE_ID: i32 = 25304;
    const LIGHTNING_ROD_UP_UNPOWERED_BLOCK_STATE_ID: i32 = 27562;
    const TURTLE_EGG_ONE_BLOCK_STATE_ID: i32 = 15090;
    const TURTLE_EGG_TWO_BLOCK_STATE_ID: i32 = 15093;
    const SNIFFER_EGG_BLOCK_STATE_ID: i32 = 15102;
    const DRIED_GHAST_NORTH_DRY_BLOCK_STATE_ID: i32 = 15106;
    const SEA_PICKLE_ONE_DRY_BLOCK_STATE_ID: i32 = 15268;
    const SEA_PICKLE_FOUR_DRY_BLOCK_STATE_ID: i32 = 15274;
    const CANDLE_ONE_DRY_UNLIT_BLOCK_STATE_ID: i32 = 23099;
    const CANDLE_FOUR_DRY_UNLIT_BLOCK_STATE_ID: i32 = 23111;
    const CANDLE_CAKE_UNLIT_BLOCK_STATE_ID: i32 = 23369;
    const POINTED_DRIPSTONE_TIP_UP_BLOCK_STATE_ID: i32 = 27740;
    const POINTED_DRIPSTONE_TIP_DOWN_BLOCK_STATE_ID: i32 = 27742;
    const POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID: i32 = 27752;
    const DECORATED_POT_NORTH_DRY_BLOCK_STATE_ID: i32 = 29602;
    const BIG_DRIPLEAF_NORTH_NONE_DRY_BLOCK_STATE_ID: i32 = 27864;
    const BIG_DRIPLEAF_NORTH_PARTIAL_DRY_BLOCK_STATE_ID: i32 = 27868;
    const BIG_DRIPLEAF_NORTH_FULL_DRY_BLOCK_STATE_ID: i32 = 27870;
    const BIG_DRIPLEAF_STEM_NORTH_DRY_BLOCK_STATE_ID: i32 = 27896;
    const MUD_BLOCK_STATE_ID: i32 = 27922;
    const HEAVY_CORE_DRY_BLOCK_STATE_ID: i32 = 29702;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID: i32 = 89;
    const SOURCE_LAVA_BLOCK_STATE_ID: i32 = 102;
    const FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID: i32 = 105;
    const VANILLA_MOB_EFFECT_BAD_OMEN_ID: i32 = 30;

    #[test]
    fn local_player_input_stops_at_full_block_wall_and_reports_collision() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 2, GRASS_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 2, 2, GRASS_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.x, 0.5, 0.000001);
        assert!(
            pose.position.z <= 1.70001,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_gravity_lands_on_floor_and_reports_grounded() {
        let mut world = flat_collision_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert!(
            pose.position.y >= 0.9999,
            "position was {:?}",
            pose.position
        );
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_gravity_attribute_scales_airborne_fall_velocity() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_GRAVITY_ID,
                base: 0.04,
                modifiers: Vec::new(),
            }],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 3.0, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.0392, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_no_gravity_metadata_suppresses_airborne_gravity() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_no_gravity(&mut world, 123, true));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 3.0, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_slow_falling_clamps_downward_gravity() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_SLOW_FALLING_ID,
            0,
        )));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            delta_movement: vec3(0.0, -0.08, 0.0),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 2.92, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.0882, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_climbable_limits_downward_velocity_and_resets_fall_distance() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.05, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_sneak_suppresses_climbable_sliding() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.2, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.0784, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_sneak_does_not_suppress_scaffolding_climbable_sliding() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            0,
            SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.05, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_scaffolding_top_supports_player_when_not_descending() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            0,
            SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.0, 0.5),
            delta_movement: vec3(0.0, -0.2, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 2.0, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_sneak_descends_through_scaffolding_top() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            0,
            SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.0, 0.5),
            delta_movement: vec3(0.0, -0.2, 0.0),
            fall_distance: 2.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.8, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.2744, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_scaffolding_unstable_bottom_supports_inside_player() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            0,
            SCAFFOLDING_BOTTOM_DISTANCE_1_BLOCK_STATE_ID,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.125, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_open_trapdoor_above_matching_ladder_counts_as_climbable() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, LADDER_SOUTH_BLOCK_STATE_ID);
        set_test_block(
            &mut world,
            0,
            1,
            0,
            OAK_TOP_OPEN_SOUTH_TRAPDOOR_BLOCK_STATE_ID,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.05, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.2254, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_open_trapdoor_requires_matching_ladder_facing_for_climbable() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, LADDER_SOUTH_BLOCK_STATE_ID);
        set_test_block(
            &mut world,
            0,
            1,
            0,
            OAK_TOP_OPEN_NORTH_TRAPDOOR_BLOCK_STATE_ID,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.1, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.1, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.1764, 0.000001);
        assert_f64_near(pose.fall_distance, 2.1, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_closed_trapdoor_above_matching_ladder_is_not_climbable() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, LADDER_NORTH_BLOCK_STATE_ID);
        set_test_block(
            &mut world,
            0,
            1,
            0,
            OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID,
        );
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.1, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.1, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.1764, 0.000001);
        assert_f64_near(pose.fall_distance, 2.1, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_jump_on_climbable_applies_vanilla_upward_velocity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.2, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_climbable_clamps_horizontal_velocity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, LADDER_SOUTH_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.65, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.15, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_levitation_moves_vertical_velocity_toward_effect_target() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_LEVITATION_ID,
            1,
        )));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 3.0, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0196, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_steps_onto_bottom_slab() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 2, OAK_BOTTOM_SLAB_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 0.35);

        assert_f64_near(pose.position.y, 1.5, 0.0005);
        assert!(pose.position.z > 1.7, "position was {:?}", pose.position);
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_does_not_step_through_top_slab() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 2, OAK_TOP_SLAB_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z <= 1.7005,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_steps_up_bottom_stair_from_low_side() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            2,
            OAK_BOTTOM_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID,
        );
        let pose = advance_forward_from_standard_start(&mut world, 0.45);

        assert_f64_near(pose.position.y, 2.0, 0.0005);
        assert!(pose.position.z > 2.2, "position was {:?}", pose.position);
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_does_not_step_up_bottom_stair_from_high_side() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            2,
            OAK_BOTTOM_STRAIGHT_NORTH_STAIR_BLOCK_STATE_ID,
        );
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z <= 1.7005,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_does_not_step_through_top_stair() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            2,
            OAK_TOP_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID,
        );
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z <= 1.7005,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_does_not_walk_through_leaves() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, OAK_LEAVES_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 2, 1, OAK_LEAVES_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_steps_over_thin_ground_shapes() {
        let cases = [
            ("white carpet", WHITE_CARPET_BLOCK_STATE_ID, 1.0625),
            ("five snow layers", SNOW_5_LAYERS_BLOCK_STATE_ID, 1.5),
        ];

        for (name, block_state_id, expected_y) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 0.2);

            assert_f64_near(pose.position.y, expected_y, 0.0005);
            assert!(
                pose.position.z > 1.0,
                "{name} position was {:?}",
                pose.position
            );
            assert!(!pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_steps_onto_low_campfire_and_lantern_shapes() {
        let cases = [
            ("campfire", CAMPFIRE_NORTH_LIT_BLOCK_STATE_ID, 1.4375),
            ("standing lantern", LANTERN_STANDING_BLOCK_STATE_ID, 1.5625),
        ];

        for (name, block_state_id, expected_y) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 0.2);

            assert_f64_near(pose.position.y, expected_y, 0.0005);
            assert!(
                pose.position.z > 1.0,
                "{name} position was {:?}",
                pose.position
            );
            assert!(!pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_lands_on_vanilla_low_ground_shapes() {
        let cases = [
            ("farmland", FARMLAND_MOISTURE_0_BLOCK_STATE_ID, 1.9375),
            ("dirt path", DIRT_PATH_BLOCK_STATE_ID, 1.9375),
            ("soul sand", SOUL_SAND_BLOCK_STATE_ID, 1.875),
            ("mud", MUD_BLOCK_STATE_ID, 1.875),
            ("cake", CAKE_BITES_0_BLOCK_STATE_ID, 1.5),
            ("lily pad", LILY_PAD_BLOCK_STATE_ID, 1.09375),
            ("flower pot", FLOWER_POT_BLOCK_STATE_ID, 1.375),
            ("potted dandelion", POTTED_DANDELION_BLOCK_STATE_ID, 1.375),
            ("one candle", CANDLE_ONE_DRY_UNLIT_BLOCK_STATE_ID, 1.375),
            ("four candles", CANDLE_FOUR_DRY_UNLIT_BLOCK_STATE_ID, 1.375),
            ("one sea pickle", SEA_PICKLE_ONE_DRY_BLOCK_STATE_ID, 1.375),
            (
                "four sea pickles",
                SEA_PICKLE_FOUR_DRY_BLOCK_STATE_ID,
                1.4375,
            ),
            (
                "pointed dripstone tip up",
                POINTED_DRIPSTONE_TIP_UP_BLOCK_STATE_ID,
                1.6875,
            ),
            (
                "pointed dripstone tip down",
                POINTED_DRIPSTONE_TIP_DOWN_BLOCK_STATE_ID,
                2.0,
            ),
            (
                "pointed dripstone base",
                POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID,
                2.0,
            ),
            ("skeleton skull", SKELETON_SKULL_BLOCK_STATE_ID, 1.5),
            ("piglin head", PIGLIN_HEAD_BLOCK_STATE_ID, 1.5),
            ("one turtle egg", TURTLE_EGG_ONE_BLOCK_STATE_ID, 1.4375),
            ("two turtle eggs", TURTLE_EGG_TWO_BLOCK_STATE_ID, 1.4375),
            ("sniffer egg", SNIFFER_EGG_BLOCK_STATE_ID, 2.0),
            ("dried ghast", DRIED_GHAST_NORTH_DRY_BLOCK_STATE_ID, 1.625),
            (
                "daylight detector",
                DAYLIGHT_DETECTOR_POWER_0_BLOCK_STATE_ID,
                1.375,
            ),
            ("sculk sensor", SCULK_SENSOR_INACTIVE_BLOCK_STATE_ID, 1.5),
            (
                "calibrated sculk sensor",
                CALIBRATED_SCULK_SENSOR_NORTH_INACTIVE_BLOCK_STATE_ID,
                1.5,
            ),
            ("sculk shrieker", SCULK_SHRIEKER_IDLE_BLOCK_STATE_ID, 1.5),
            ("heavy core", HEAVY_CORE_DRY_BLOCK_STATE_ID, 1.5),
            (
                "empty end portal frame",
                END_PORTAL_FRAME_EMPTY_NORTH_BLOCK_STATE_ID,
                1.8125,
            ),
            (
                "eye end portal frame",
                END_PORTAL_FRAME_EYE_NORTH_BLOCK_STATE_ID,
                2.0,
            ),
            ("candle cake", CANDLE_CAKE_UNLIT_BLOCK_STATE_ID, 1.875),
            ("decorated pot", DECORATED_POT_NORTH_DRY_BLOCK_STATE_ID, 2.0),
        ];

        for (name, block_state_id, expected_y) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            world.set_local_player_pose(LocalPlayerPoseState {
                position: vec3(0.5, 3.0, 1.5),
                on_ground: false,
                ..LocalPlayerPoseState::default()
            });

            let pose = world
                .advance_local_player_input(
                    LocalPlayerInputState {
                        focused: true,
                        ..LocalPlayerInputState::default()
                    },
                    2.0,
                )
                .unwrap();

            assert_f64_near(pose.position.y, expected_y, 0.0005);
            assert!(pose.on_ground, "{name}");
            assert!(!pose.horizontal_collision, "{name}");
        }
    }

    #[test]
    fn local_player_lands_on_big_dripleaf_leaf_collision_shapes() {
        let cases = [
            (
                "none tilt",
                BIG_DRIPLEAF_NORTH_NONE_DRY_BLOCK_STATE_ID,
                1.9375,
            ),
            (
                "partial tilt",
                BIG_DRIPLEAF_NORTH_PARTIAL_DRY_BLOCK_STATE_ID,
                1.8125,
            ),
        ];

        for (name, block_state_id, expected_y) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 0, block_state_id);
            world.set_local_player_pose(LocalPlayerPoseState {
                position: vec3(0.5, 3.0, 0.5),
                on_ground: false,
                ..LocalPlayerPoseState::default()
            });

            let pose = world
                .advance_local_player_input(
                    LocalPlayerInputState {
                        focused: true,
                        ..LocalPlayerInputState::default()
                    },
                    2.0,
                )
                .unwrap();

            assert_f64_near(pose.position.y, expected_y, 0.0005);
            assert!(pose.on_ground, "{name}");
            assert!(!pose.horizontal_collision, "{name}");
        }
    }

    #[test]
    fn local_player_ignores_full_tilt_big_dripleaf_and_stem_collision() {
        let cases = [
            ("full tilt leaf", BIG_DRIPLEAF_NORTH_FULL_DRY_BLOCK_STATE_ID),
            ("stem", BIG_DRIPLEAF_STEM_NORTH_DRY_BLOCK_STATE_ID),
        ];

        for (name, block_state_id) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 0, block_state_id);
            world.set_local_player_pose(LocalPlayerPoseState {
                position: vec3(0.5, 3.0, 0.5),
                on_ground: false,
                ..LocalPlayerPoseState::default()
            });

            let pose = world
                .advance_local_player_input(
                    LocalPlayerInputState {
                        focused: true,
                        ..LocalPlayerInputState::default()
                    },
                    2.0,
                )
                .unwrap();

            assert_f64_near(pose.position.y, 1.0, 0.0005);
            assert!(pose.on_ground, "{name}");
            assert!(!pose.horizontal_collision, "{name}");
        }
    }

    #[test]
    fn local_player_respects_cake_bite_collision_width() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, CAKE_BITES_3_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.1, 3.0, 1.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                2.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_walks_up_to_narrow_cactus_and_honey_columns() {
        let cases = [
            ("cactus", CACTUS_AGE_0_BLOCK_STATE_ID),
            ("honey block", HONEY_BLOCK_STATE_ID),
            ("dragon egg", DRAGON_EGG_BLOCK_STATE_ID),
        ];

        for (name, block_state_id) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z > 0.7005 && pose.position.z <= 0.763,
                "{name} position was {:?}",
                pose.position
            );
            assert_f64_near(pose.position.y, 1.0, 0.0005);
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_does_not_collide_with_pressure_plate_outline() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, STONE_PRESSURE_PLATE_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.position.z > 1.0, "position was {:?}", pose.position);
        assert!(!pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_does_not_walk_through_copper_grates() {
        let cases = [
            ("copper grate", COPPER_GRATE_BLOCK_STATE_ID),
            ("waxed copper grate", WAXED_COPPER_GRATE_BLOCK_STATE_ID),
        ];

        for (name, block_state_id) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z <= 0.7005,
                "{name} position was {:?}",
                pose.position
            );
            assert_f64_near(pose.position.y, 1.0, 0.0005);
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_respects_extended_piston_base_shape() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, PISTON_EXTENDED_NORTH_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.7005 && pose.position.z <= 0.9505,
            "position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_respects_piston_head_rod_extending_outside_block() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 2, PISTON_HEAD_SOUTH_LONG_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z > 0.7005 && pose.position.z <= 1.4505,
            "position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_lands_on_up_extended_piston_base_plate() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, PISTON_EXTENDED_UP_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                2.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.75, 0.0005);
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_does_not_collide_with_moving_piston_without_block_entity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, MOVING_PISTON_NORTH_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 0.2);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.position.z > 1.0, "position was {:?}", pose.position);
        assert!(!pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_does_not_walk_through_chain_or_rod_shapes() {
        let cases = [
            (
                "vertical chain",
                IRON_CHAIN_Y_AXIS_BLOCK_STATE_ID,
                0.9,
                1.107,
            ),
            (
                "vertical lightning rod",
                LIGHTNING_ROD_UP_UNPOWERED_BLOCK_STATE_ID,
                0.9,
                1.076,
            ),
            ("north end rod", END_ROD_NORTH_BLOCK_STATE_ID, 0.0, 0.7005),
        ];

        for (name, block_state_id, min_z, max_z) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z > min_z && pose.position.z <= max_z,
                "{name} position was {:?}",
                pose.position
            );
            assert_f64_near(pose.position.y, 1.0, 0.0005);
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_does_not_walk_through_ladder_sheet_on_approached_side() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, LADDER_SOUTH_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert!(
            pose.position.z <= 0.7005,
            "position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_moves_through_scaffolding_side_instead_of_full_block() {
        let mut world = flat_collision_world();
        set_test_block(
            &mut world,
            0,
            1,
            1,
            SCAFFOLDING_BOTTOM_DISTANCE_0_BLOCK_STATE_ID,
        );
        let pose = advance_forward_from_standard_start(&mut world, 0.1);

        assert!(pose.position.z > 0.8, "position was {:?}", pose.position);
        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(!pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_does_not_step_over_tall_snow_layer() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, SNOW_6_LAYERS_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z <= 0.7005,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_does_not_walk_through_door_and_trapdoor_shapes() {
        let cases = [
            ("closed door", OAK_CLOSED_NORTH_DOOR_BLOCK_STATE_ID, 1.5135),
            (
                "closed top trapdoor",
                OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID,
                0.7005,
            ),
        ];

        for (name, block_state_id, max_z) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z <= max_z,
                "{name} position was {:?}",
                pose.position
            );
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_does_not_walk_through_fence_pane_or_wall_connections() {
        let cases = [
            ("north fence", OAK_NORTH_FENCE_BLOCK_STATE_ID),
            ("north pane", GLASS_NORTH_PANE_BLOCK_STATE_ID),
            (
                "north/east wall",
                COBBLESTONE_NORTH_EAST_WALL_BLOCK_STATE_ID,
            ),
        ];

        for (name, block_state_id) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z <= 0.7005,
                "{name} position was {:?}",
                pose.position
            );
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_respects_closed_and_open_fence_gate_collision() {
        let mut closed_world = flat_collision_world();
        set_test_block(
            &mut closed_world,
            0,
            1,
            1,
            OAK_CLOSED_NORTH_FENCE_GATE_BLOCK_STATE_ID,
        );
        let closed_pose = advance_forward_from_standard_start(&mut closed_world, 1.0);

        assert!(
            closed_pose.position.z <= 1.0755,
            "position was {:?}",
            closed_pose.position
        );
        assert!(closed_pose.horizontal_collision);
        assert!(closed_pose.on_ground);

        let mut open_world = flat_collision_world();
        set_test_block(
            &mut open_world,
            0,
            1,
            1,
            OAK_OPEN_NORTH_FENCE_GATE_BLOCK_STATE_ID,
        );
        let open_pose = advance_forward_from_standard_start(&mut open_world, 0.35);

        assert!(
            open_pose.position.z > 1.5,
            "position was {:?}",
            open_pose.position
        );
        assert!(!open_pose.horizontal_collision);
        assert!(open_pose.on_ground);
    }

    #[test]
    fn local_player_does_not_walk_through_common_object_shapes() {
        let cases = [
            ("single chest", CHEST_SINGLE_NORTH_BLOCK_STATE_ID, 0.763),
            ("double chest left", CHEST_LEFT_NORTH_BLOCK_STATE_ID, 0.763),
            (
                "trapped double chest right",
                TRAPPED_CHEST_RIGHT_NORTH_BLOCK_STATE_ID,
                0.763,
            ),
            ("ender chest", ENDER_CHEST_NORTH_BLOCK_STATE_ID, 0.763),
            (
                "water cauldron",
                WATER_CAULDRON_LEVEL_3_BLOCK_STATE_ID,
                0.7005,
            ),
            ("north hopper", HOPPER_NORTH_ENABLED_BLOCK_STATE_ID, 0.7005),
            ("enchanting table", ENCHANTING_TABLE_BLOCK_STATE_ID, 0.7005),
            ("anvil", ANVIL_NORTH_BLOCK_STATE_ID, 0.7005),
            (
                "north wall skull",
                SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID,
                1.2005,
            ),
            (
                "north wall grindstone",
                GRINDSTONE_WALL_NORTH_BLOCK_STATE_ID,
                0.7005,
            ),
            (
                "pointed dripstone base",
                POINTED_DRIPSTONE_BASE_UP_BLOCK_STATE_ID,
                0.909,
            ),
        ];

        for (name, block_state_id, max_z) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z <= max_z,
                "{name} position was {:?}",
                pose.position
            );
            assert_f64_near(pose.position.y, 1.0, 0.0005);
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_respects_piglin_wall_head_wider_collision() {
        let mut normal_world = flat_collision_world();
        set_test_block(
            &mut normal_world,
            0,
            1,
            1,
            SKELETON_WALL_SKULL_NORTH_BLOCK_STATE_ID,
        );
        normal_world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(1.06, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        let normal_pose = normal_world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert!(
            normal_pose.position.z > 1.5,
            "normal wall skull position was {:?}",
            normal_pose.position
        );
        assert!(!normal_pose.horizontal_collision);
        assert!(normal_pose.on_ground);

        let mut piglin_world = flat_collision_world();
        set_test_block(
            &mut piglin_world,
            0,
            1,
            1,
            PIGLIN_WALL_HEAD_NORTH_BLOCK_STATE_ID,
        );
        piglin_world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(1.06, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        let piglin_pose = piglin_world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert!(
            piglin_pose.position.z <= 1.2005,
            "piglin wall head position was {:?}",
            piglin_pose.position
        );
        assert!(piglin_pose.horizontal_collision);
        assert!(piglin_pose.on_ground);
    }

    #[test]
    fn local_player_does_not_walk_through_grindstone_floor_and_ceiling_faces() {
        let cases = [
            (
                "north floor grindstone",
                GRINDSTONE_FLOOR_NORTH_BLOCK_STATE_ID,
            ),
            (
                "north ceiling grindstone",
                GRINDSTONE_CEILING_NORTH_BLOCK_STATE_ID,
            ),
        ];

        for (name, block_state_id) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z <= 0.8255,
                "{name} position was {:?}",
                pose.position
            );
            assert_f64_near(pose.position.y, 1.0, 0.0005);
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_steps_onto_lectern_base_but_stops_at_center_column() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, LECTERN_NORTH_NO_BOOK_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.125, 0.0005);
        assert!(
            pose.position.z > 0.7005 && pose.position.z <= 0.9505,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_does_not_walk_through_bell_attachments() {
        let cases = [
            ("floor bell", BELL_FLOOR_NORTH_BLOCK_STATE_ID, 0.9505),
            (
                "single-wall bell",
                BELL_SINGLE_WALL_NORTH_BLOCK_STATE_ID,
                0.7005,
            ),
            (
                "double-wall bell",
                BELL_DOUBLE_WALL_NORTH_BLOCK_STATE_ID,
                0.7005,
            ),
        ];

        for (name, block_state_id, max_z) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 1.0);

            assert!(
                pose.position.z <= max_z,
                "{name} position was {:?}",
                pose.position
            );
            assert_f64_near(pose.position.y, 1.0, 0.0005);
            assert!(pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_collision_detects_ceiling_bell_body() {
        let mut world = flat_collision_world();
        let pos = BlockPos { x: 0, y: 1, z: 1 };
        set_test_block(
            &mut world,
            pos.x,
            pos.y,
            pos.z,
            BELL_CEILING_NORTH_BLOCK_STATE_ID,
        );

        assert!(world.local_player_pose_collides_with_block(
            pos,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.0, 1.5),
                ..LocalPlayerPoseState::default()
            },
        ));
    }

    #[test]
    fn local_player_steps_onto_common_low_object_shapes() {
        let cases = [
            ("north foot bed", WHITE_BED_NORTH_FOOT_BLOCK_STATE_ID),
            ("north stonecutter", STONECUTTER_NORTH_BLOCK_STATE_ID),
        ];

        for (name, block_state_id) in cases {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 1, 1, block_state_id);
            let pose = advance_forward_from_standard_start(&mut world, 0.2);

            assert_f64_near(pose.position.y, 1.5625, 0.0005);
            assert!(
                pose.position.z > 1.0,
                "{name} position was {:?}",
                pose.position
            );
            assert!(!pose.horizontal_collision, "{name}");
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_steps_onto_brewing_stand_base_but_stops_at_center_rod() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, BREWING_STAND_EMPTY_BLOCK_STATE_ID);
        let pose = advance_forward_from_standard_start(&mut world, 1.0);

        assert_f64_near(pose.position.y, 1.125, 0.0005);
        assert!(
            pose.position.z > 1.0 && pose.position.z <= 1.1376,
            "position was {:?}",
            pose.position
        );
        assert!(pose.horizontal_collision);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_composter_collision_uses_level_zero_shape_for_every_level() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 1, COMPOSTER_LEVEL_7_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 1.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                2.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.125, 0.0005);
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_sneak_backs_off_from_block_edge() {
        let mut world = single_floor_block_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.z <= 1.3005,
            "position was {:?}",
            pose.position
        );
        assert!(pose.sneaking);
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_sneak_uses_crouching_collision_height() {
        let mut standing_world = flat_collision_world();
        set_test_block(&mut standing_world, 0, 2, 1, OAK_TOP_SLAB_BLOCK_STATE_ID);
        standing_world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let standing = standing_world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert!(standing.horizontal_collision);
        assert!(
            standing.position.z < 1.0,
            "position was {:?}",
            standing.position
        );
        assert!(!standing.sneaking);
        assert!(!standing.swimming);

        let mut crouching_world = flat_collision_world();
        set_test_block(&mut crouching_world, 0, 2, 1, OAK_TOP_SLAB_BLOCK_STATE_ID);
        crouching_world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let crouching = crouching_world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert!(
            crouching.position.z > 1.0,
            "position was {:?}",
            crouching.position
        );
        assert!(crouching.sneaking);
        assert!(!crouching.swimming);
        assert!(!crouching.horizontal_collision);
    }

    #[test]
    fn local_player_low_ceiling_forces_crouching_pose() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 2, 0, OAK_TOP_SLAB_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
            * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
            * LOCAL_PHYSICS_TICK_SECONDS;
        assert!(pose.sneaking);
        assert!(!pose.swimming);
        assert!(!pose.horizontal_collision);
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    }

    #[test]
    fn local_player_low_tunnel_forces_crawling_swimming_pose() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
            * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
            * LOCAL_PHYSICS_TICK_SECONDS;
        assert!(!pose.sneaking);
        assert!(pose.swimming);
        assert!(!pose.horizontal_collision);
        assert_f64_near(pose.body_height(), 0.6, 0.000001);
        assert_f64_near(pose.eye_height(), 0.4, 0.000001);
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    }

    #[test]
    fn local_player_underwater_sprint_uses_swimming_pose() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(!pose.sneaking);
        assert!(pose.swimming);
        assert_f64_near(pose.body_height(), 0.6, 0.000001);
        assert_f64_near(pose.eye_height(), 0.4, 0.000001);
    }

    #[test]
    fn local_player_low_ceiling_does_not_force_crouch_while_flying() {
        let mut world = flat_collision_world();
        apply_flying_abilities(&mut world, 0.05);
        set_test_block(&mut world, 0, 2, 0, OAK_TOP_SLAB_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(!pose.sneaking);
        assert!(!pose.swimming);
        assert_f64_near(pose.position.y, 1.0, 0.000001);
    }

    #[test]
    fn local_player_low_tunnel_does_not_force_crawling_while_flying() {
        let mut world = flat_collision_world();
        apply_flying_abilities(&mut world, 0.05);
        set_test_block(&mut world, 0, 2, 0, GRASS_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(!pose.sneaking);
        assert!(!pose.swimming);
        assert_f64_near(pose.position.y, 1.0, 0.000001);
    }

    #[test]
    fn local_player_sneak_backs_off_from_near_ground_edge_while_falling() {
        let mut world = single_floor_block_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 1.25),
            fall_distance: 0.2,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(
            pose.position.z <= 1.3005,
            "position was {:?}",
            pose.position
        );
        assert_f64_near(pose.position.y, 1.2, 0.0005);
        assert!(!pose.on_ground);
        assert!(pose.fall_distance < LOCAL_PLAYER_STEP_HEIGHT);
    }

    #[test]
    fn local_player_sneak_edge_backoff_stops_after_step_height_fall_distance() {
        let mut world = single_floor_block_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.3, 1.25),
            fall_distance: LOCAL_PLAYER_STEP_HEIGHT,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
            * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
            * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 1.25 + expected_step, 0.000001);
        assert_eq!(pose.fall_distance, LOCAL_PLAYER_STEP_HEIGHT);
    }

    #[test]
    fn local_player_fall_distance_accumulates_downward_motion_and_resets_on_ground() {
        let mut world = flat_collision_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.0, 0.5),
            delta_movement: vec3(0.0, -0.1, 0.0),
            fall_distance: 0.2,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let falling = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(falling.position.y, 1.9, 0.000001);
        assert_f64_near(falling.fall_distance, 0.3, 0.000001);
        assert!(!falling.on_ground);

        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.05, 0.5),
            delta_movement: vec3(0.0, -0.1, 0.0),
            fall_distance: falling.fall_distance,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let landed = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(landed.position.y, 1.0, 0.0005);
        assert!(landed.on_ground);
        assert_f64_near(landed.fall_distance, 0.0, 0.000001);
    }

    #[test]
    fn local_player_fall_distance_resets_when_touching_water_surface() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.95, 0.5),
            delta_movement: vec3(0.0, -0.1, 0.0),
            fall_distance: 0.4,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(!pose.on_ground);
        assert_f64_near(pose.position.y, 1.85, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    }

    #[test]
    fn local_player_fall_distance_does_not_reset_above_low_flowing_water_surface() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.7, 0.5),
            delta_movement: vec3(0.0, -0.1, 0.0),
            fall_distance: 0.4,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(!pose.on_ground);
        assert_f64_near(pose.position.y, 1.6, 0.000001);
        assert_f64_near(pose.fall_distance, 0.5, 0.000001);
    }

    #[test]
    fn local_player_in_water_uses_fluid_relative_acceleration_and_drag() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            fall_distance: 0.4,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_in_flowing_water_applies_vanilla_current_before_drag() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, FLOWING_WATER_LEVEL_3_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.514, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.0112, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    }

    #[test]
    fn local_player_water_movement_efficiency_half_applies_when_airborne() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
                base: 1.0,
                modifiers: Vec::new(),
            }],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.56, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.0403800018, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    }

    #[test]
    fn local_player_water_movement_efficiency_fully_applies_on_ground() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
                base: 1.0,
                modifiers: Vec::new(),
            }],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.6, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.054600006, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    }

    #[test]
    fn local_player_dolphins_grace_overrides_water_horizontal_drag() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_DOLPHINS_GRACE_ID,
            0,
        )));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.0192, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    }

    #[test]
    fn local_player_no_gravity_metadata_suppresses_water_falling_gravity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(apply_no_gravity(&mut world, 123, true));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_bad_omen_does_not_apply_dolphins_grace_water_drag() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_BAD_OMEN_ID, 0,)));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    }

    #[test]
    fn local_player_sprinting_in_water_uses_vanilla_sprint_drag_without_sinking_gravity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.018, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_low_food_prevents_water_sprint_and_swimming_pose() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        apply_player_health(&mut world, 6);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(!pose.swimming);
        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.016, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    }

    #[test]
    fn local_player_swimming_pitch_down_pulls_velocity_toward_look_y() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            x_rot: 30.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.swimming);
        assert_f64_near(pose.position.y, 1.0575, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.034, 0.000001);
        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.018, 0.000001);
    }

    #[test]
    fn local_player_swimming_pitch_up_rises_when_fluid_above() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            x_rot: -30.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.swimming);
        assert_f64_near(pose.position.y, 1.13, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.024, 0.000001);
    }

    #[test]
    fn local_player_swimming_pitch_up_near_surface_does_not_auto_rise() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            x_rot: -30.0,
            swimming: true,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.swimming);
        assert_f64_near(pose.position.y, 1.1, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_jump_and_sneak_in_water_apply_liquid_vertical_impulses() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let upward = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(upward.position.y, 1.14, 0.000001);
        assert_f64_near(upward.delta_movement.y, 0.027, 0.000001);

        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });
        let downward = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(downward.position.y, 1.06, 0.000001);
        assert_f64_near(downward.delta_movement.y, -0.037, 0.000001);
    }

    #[test]
    fn local_player_inside_upward_bubble_column_applies_vanilla_push() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
        set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.delta_movement.y, 0.055, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
    }

    #[test]
    fn local_player_above_upward_bubble_column_uses_surface_push() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.delta_movement.y, 0.095, 0.000001);
    }

    #[test]
    fn local_player_drag_down_bubble_column_applies_downward_velocity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(true));
        set_test_block(&mut world, 0, 2, 0, SOURCE_WATER_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.delta_movement.y, -0.035, 0.000001);
    }

    #[test]
    fn local_player_flying_ignores_bubble_column_push() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
        apply_flying_abilities(&mut world, 0.05);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_in_water_jumps_out_when_horizontal_collision_has_clear_space() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.45, 0.69),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.horizontal_collision);
        assert_f64_near(
            pose.delta_movement.y,
            LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK,
            0.000001,
        );
    }

    #[test]
    fn local_player_in_water_does_not_jump_out_when_clearance_still_contains_fluid() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.69),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.horizontal_collision);
        assert_f64_near(pose.delta_movement.y, -0.005, 0.000001);
    }

    #[test]
    fn local_player_in_lava_jumps_out_when_horizontal_collision_has_clear_space() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.45, 0.69),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.horizontal_collision);
        assert_f64_near(
            pose.delta_movement.y,
            LOCAL_INPUT_FLUID_JUMP_OUT_VELOCITY_PER_TICK,
            0.000001,
        );
    }

    #[test]
    fn local_player_in_lava_uses_lava_drag_and_gravity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.01, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
    }

    #[test]
    fn local_player_no_gravity_metadata_suppresses_lava_gravity() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(apply_no_gravity(&mut world, 123, true));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.01, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_in_lava_ignores_water_movement_efficiency() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
                base: 1.0,
                modifiers: Vec::new(),
            }],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.52, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.01, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
    }

    #[test]
    fn local_player_in_lava_current_uses_vanilla_minimum_push_before_drag() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.5045, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.00225, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
    }

    #[test]
    fn local_player_in_lava_current_skips_minimum_push_when_already_moving() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            delta_movement: vec3(0.01, 0.0, 0.0),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.x, 0.51, 0.000001);
        assert_f64_near(pose.position.z, 0.5023333333333333, 0.000001);
        assert_f64_near(pose.delta_movement.x, 0.005, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.0011666666666666668, 0.000001);
    }

    #[test]
    fn local_player_in_nether_lava_current_uses_fast_lava_push() {
        let mut world = flat_collision_world();
        world.level = Some(WorldLevelInfo {
            dimension: "minecraft:the_nether".to_string(),
            dimension_type_id: 1,
            dimension_type_name: Some("minecraft:the_nether".to_string()),
            sea_level: 32,
            is_debug: false,
            is_flat: false,
        });
        set_test_block(&mut world, 0, 1, 0, SOURCE_LAVA_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, FLOWING_LAVA_LEVEL_3_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.z, 0.507, 0.000001);
        assert_f64_near(pose.delta_movement.z, 0.0035, 0.000001);
        assert_f64_near(pose.delta_movement.y, -0.02, 0.000001);
    }

    #[test]
    fn local_player_flying_in_water_uses_flying_movement_not_fluid_travel() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        apply_flying_abilities(&mut world, 0.05);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.1, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = f64::from(0.05_f32);
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(
            pose.delta_movement.z,
            expected_step * LOCAL_INPUT_FLY_AIR_DRAG,
            0.000001,
        );
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_without_sneak_walks_off_block_edge() {
        let mut world = single_floor_block_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                0.5,
            )
            .unwrap();

        assert!(pose.position.z > 1.3, "position was {:?}", pose.position);
        assert!(
            pose.position.y < 1.0 || !pose.on_ground,
            "pose was {:?}",
            pose
        );
    }

    #[test]
    fn local_player_sneak_backs_off_from_block_corner_diagonally() {
        let mut world = single_floor_block_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    right: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        assert!(
            pose.position.x >= -0.3005 && pose.position.z <= 1.3005,
            "position was {:?}",
            pose.position
        );
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_sneak_edge_backoff_does_not_apply_while_flying() {
        let mut world = single_floor_block_world();
        apply_flying_abilities(&mut world, 0.05);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                0.5,
            )
            .unwrap();

        assert!(pose.position.z > 1.3, "position was {:?}", pose.position);
        assert!(!pose.sneaking);
    }

    #[test]
    fn local_player_flying_hovers_without_gravity() {
        let mut world = flat_collision_world();
        apply_flying_abilities(&mut world, 0.05);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                1.0,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 3.0, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_flying_jump_and_sneak_move_vertically() {
        let mut world = flat_collision_world();
        apply_flying_abilities(&mut world, 0.05);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let upward = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        assert_f64_near(upward.position.y, 3.15, 0.000001);
        assert_f64_near(upward.delta_movement.y, 0.09, 0.000001);
        assert!(!upward.on_ground);

        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });
        let downward = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        assert_f64_near(downward.position.y, 2.85, 0.000001);
        assert_f64_near(downward.delta_movement.y, -0.09, 0.000001);
        assert!(!downward.sneaking);
        assert!(!downward.on_ground);
    }

    #[test]
    fn local_player_flying_vertical_momentum_damps_without_input() {
        let mut world = flat_collision_world();
        apply_flying_abilities(&mut world, 0.05);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            delta_movement: vec3(0.0, 0.09, 0.0),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 3.09, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.054, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_flying_horizontal_velocity_accumulates_and_drags() {
        let mut world = WorldStore::new();
        apply_flying_abilities(&mut world, 0.05);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 64.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let first = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        let acceleration = f64::from(0.05_f32);
        assert_f64_near(first.position.z, 0.5 + acceleration, 0.000001);
        assert_f64_near(
            first.delta_movement.z,
            acceleration * LOCAL_INPUT_FLY_AIR_DRAG,
            0.000001,
        );

        let second = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        let second_step = first.delta_movement.z + acceleration;
        assert_f64_near(
            second.position.z,
            0.5 + acceleration + second_step,
            0.000001,
        );
        assert_f64_near(
            second.delta_movement.z,
            second_step * LOCAL_INPUT_FLY_AIR_DRAG,
            0.000001,
        );
    }

    #[test]
    fn local_player_flying_uses_abilities_speed_and_sprint_multiplier() {
        let mut world = WorldStore::new();
        apply_flying_abilities(&mut world, 0.1);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 64.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = f64::from(0.1_f32) * LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(
            pose.delta_movement.z,
            expected_step * LOCAL_INPUT_FLY_AIR_DRAG,
            0.000001,
        );
        assert_f64_near(pose.position.y, 64.0, 0.000001);
    }

    #[test]
    fn local_player_sneak_moves_over_supported_ground() {
        let mut world = flat_collision_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                0.2,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0005);
        let expected_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER * 0.2;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_sneak_uses_default_sneaking_speed_multiplier() {
        let mut world = flat_collision_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
            * LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER
            * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_movement_speed_attribute_scales_horizontal_movement() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: 0.2,
                modifiers: Vec::new(),
            }],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 2.0 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_slow_block_speed_factor_scales_horizontal_movement() {
        for (name, block_state_id, player_y) in [
            ("soul sand", SOUL_SAND_BLOCK_STATE_ID, 0.875),
            ("honey block", HONEY_BLOCK_STATE_ID, 0.9375),
        ] {
            let mut world = flat_collision_world();
            set_test_block(&mut world, 0, 0, 0, block_state_id);
            world.set_local_player_pose(LocalPlayerPoseState {
                position: vec3(0.5, player_y, 0.5),
                on_ground: true,
                ..LocalPlayerPoseState::default()
            });

            let pose = world
                .advance_local_player_input(
                    LocalPlayerInputState {
                        focused: true,
                        forward: true,
                        ..LocalPlayerInputState::default()
                    },
                    LOCAL_PHYSICS_TICK_SECONDS,
                )
                .unwrap();

            let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
                * SLOW_BLOCK_SPEED_FACTOR
                * LOCAL_PHYSICS_TICK_SECONDS;
            assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
            assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
            assert!(pose.on_ground, "{name}");
        }
    }

    #[test]
    fn local_player_movement_efficiency_offsets_slow_block_speed_factor() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID,
                base: 1.0,
                modifiers: Vec::new(),
            }],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 0.875, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_block_speed_factor_does_not_fallback_through_water() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 0, SOURCE_WATER_BLOCK_STATE_ID);
        let factor = local_player_block_speed_factor(
            &world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.0, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_f64_near(factor, DEFAULT_BLOCK_SPEED_FACTOR, 0.000001);
    }

    #[test]
    fn local_player_block_speed_factor_does_not_fallback_through_bubble_column() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, SOUL_SAND_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 0, bubble_column_block_state_id(false));
        let factor = local_player_block_speed_factor(
            &world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.0, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_f64_near(factor, DEFAULT_BLOCK_SPEED_FACTOR, 0.000001);
    }

    #[test]
    fn local_player_ticks_frozen_applies_powder_snow_slowdown() {
        let mut half_frozen_world = flat_collision_world();
        attach_local_player_entity(&mut half_frozen_world, 123);
        assert!(apply_ticks_frozen(&mut half_frozen_world, 123, 70));

        let half_frozen_pose =
            advance_forward_from_standard_start(&mut half_frozen_world, LOCAL_PHYSICS_TICK_SECONDS);
        let half_frozen_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.75 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(
            half_frozen_pose.position.z,
            0.5 + half_frozen_step,
            0.000001,
        );
        assert_f64_near(
            half_frozen_pose.delta_movement.z,
            half_frozen_step,
            0.000001,
        );

        let mut fully_frozen_world = flat_collision_world();
        attach_local_player_entity(&mut fully_frozen_world, 124);
        assert!(apply_ticks_frozen(&mut fully_frozen_world, 124, 280));

        let fully_frozen_pose = advance_forward_from_standard_start(
            &mut fully_frozen_world,
            LOCAL_PHYSICS_TICK_SECONDS,
        );
        let fully_frozen_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.5 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(
            fully_frozen_pose.position.z,
            0.5 + fully_frozen_step,
            0.000001,
        );
        assert_f64_near(
            fully_frozen_pose.delta_movement.z,
            fully_frozen_step,
            0.000001,
        );
    }

    #[test]
    fn local_player_powder_snow_speed_modifier_does_not_double_apply_synced_attribute_modifier() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE,
                modifiers: vec![ProtocolAttributeModifier {
                    id: VANILLA_POWDER_SNOW_SPEED_MODIFIER_ID.to_string(),
                    amount: -0.025,
                    operation_id: 0,
                }],
            }],
        }));
        assert!(apply_ticks_frozen(&mut world, 123, 70));

        let pose = advance_forward_from_standard_start(&mut world, LOCAL_PHYSICS_TICK_SECONDS);
        let expected_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.75 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    }

    #[test]
    fn local_player_in_powder_snow_increments_ticks_frozen() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

        local_player_update_powder_snow_freezing(
            &mut world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.2, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(world.entities.ticks_frozen(123), Some(1));
        assert_eq!(
            world
                .entities
                .metadata(123)
                .unwrap()
                .data_values
                .iter()
                .find(|value| {
                    value.data_id == crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID
                })
                .map(|value| value.serializer_id),
            Some(VANILLA_ENTITY_DATA_INT_SERIALIZER_ID)
        );
    }

    #[test]
    fn local_player_in_powder_snow_caps_ticks_frozen_at_required_ticks() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_ticks_frozen(
            &mut world,
            123,
            LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE,
        ));
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

        local_player_update_powder_snow_freezing(
            &mut world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.2, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(
            world.entities.ticks_frozen(123),
            Some(LOCAL_PLAYER_TICKS_REQUIRED_TO_FREEZE)
        );
    }

    #[test]
    fn local_player_out_of_powder_snow_thaws_two_ticks_per_step() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_ticks_frozen(&mut world, 123, 5));

        local_player_update_powder_snow_freezing(
            &mut world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.2, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(world.entities.ticks_frozen(123), Some(3));
    }

    #[test]
    fn local_player_out_of_powder_snow_thaw_clamps_at_zero() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_ticks_frozen(&mut world, 123, 1));

        local_player_update_powder_snow_freezing(
            &mut world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.2, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(world.entities.ticks_frozen(123), Some(0));
    }

    #[test]
    fn local_player_spectator_in_powder_snow_thaws_instead_of_freezing() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_ticks_frozen(&mut world, 123, 5));
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        world.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 3.0,
        });

        local_player_update_powder_snow_freezing(
            &mut world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.2, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(world.entities.ticks_frozen(123), Some(3));
    }

    #[test]
    fn local_player_freeze_immune_wearable_in_powder_snow_thaws_instead_of_freezing() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_ticks_frozen(&mut world, 123, 5));
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
            item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 1),
        });

        local_player_update_powder_snow_freezing(
            &mut world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.2, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(world.entities.ticks_frozen(123), Some(3));
    }

    #[test]
    fn local_player_empty_freeze_immune_wearable_stack_in_powder_snow_still_freezes() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_ticks_frozen(&mut world, 123, 5));
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
            item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 0),
        });

        local_player_update_powder_snow_freezing(
            &mut world,
            LocalPlayerPoseState {
                position: vec3(0.5, 1.2, 0.5),
                ..LocalPlayerPoseState::default()
            },
        );

        assert_eq!(world.entities.ticks_frozen(123), Some(6));
    }

    #[test]
    fn local_player_without_powder_snow_walkable_boots_sinks_through_powder_snow() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.0, 0.5),
            ..LocalPlayerPoseState::default()
        });

        world
            .advance_local_player_input(
                LocalPlayerInputState::default(),
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        world
            .advance_local_player_input(
                LocalPlayerInputState::default(),
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let pose = world.local_player_pose().unwrap();
        assert!(pose.position.y < 2.0);
        assert!(!pose.on_ground);
        assert!(pose.delta_movement.y < 0.0);
    }

    #[test]
    fn local_player_leather_boots_stand_on_powder_snow_top_collision() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        equip_powder_snow_walkable_boots(&mut world, 1);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.0, 0.5),
            ..LocalPlayerPoseState::default()
        });

        world
            .advance_local_player_input(
                LocalPlayerInputState::default(),
                LOCAL_PHYSICS_TICK_SECONDS * 2.0,
            )
            .unwrap();

        let pose = world.local_player_pose().unwrap();
        assert_f64_near(pose.position.y, 2.0, 0.000001);
        assert!(pose.on_ground);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
    }

    #[test]
    fn local_player_freeze_immune_non_leather_boots_do_not_walk_on_powder_snow() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        world.set_freeze_immune_wearable_item_ids(BTreeSet::from([FREEZE_IMMUNE_WEARABLE_ITEM_ID]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
            item: item_stack(FREEZE_IMMUNE_WEARABLE_ITEM_ID, 1),
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.0, 0.5),
            ..LocalPlayerPoseState::default()
        });

        world
            .advance_local_player_input(
                LocalPlayerInputState::default(),
                LOCAL_PHYSICS_TICK_SECONDS * 2.0,
            )
            .unwrap();

        let pose = world.local_player_pose().unwrap();
        assert!(pose.position.y < 2.0);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_sneak_descends_through_powder_snow_even_with_leather_boots() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        equip_powder_snow_walkable_boots(&mut world, 1);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 2.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS * 2.0,
            )
            .unwrap();

        let pose = world.local_player_pose().unwrap();
        assert!(pose.position.y < 2.0);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_high_fall_distance_uses_powder_snow_falling_collision_shape() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);

        assert!(!world.local_player_pose_collides_with_block(
            BlockPos { x: 0, y: 1, z: 0 },
            LocalPlayerPoseState {
                position: vec3(0.5, 1.85, 0.5),
                fall_distance: 2.5,
                ..LocalPlayerPoseState::default()
            },
        ));
        assert!(world.local_player_pose_collides_with_block(
            BlockPos { x: 0, y: 1, z: 0 },
            LocalPlayerPoseState {
                position: vec3(0.5, 1.85, 0.5),
                fall_distance: 2.500001,
                ..LocalPlayerPoseState::default()
            },
        ));
    }

    #[test]
    fn local_player_leather_boots_jump_climbs_out_of_powder_snow() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        equip_powder_snow_walkable_boots(&mut world, 1);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.2, 0.000001);
        assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_without_leather_boots_jump_does_not_climb_out_of_powder_snow() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.2, 0.000001);
        assert!(pose.delta_movement.y < 0.0);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_leather_boots_horizontal_collision_climbs_out_of_powder_snow() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 0, POWDER_SNOW_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 1, 1, GRASS_BLOCK_STATE_ID);
        equip_powder_snow_walkable_boots(&mut world, 1);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert!(pose.horizontal_collision);
        assert_f64_near(pose.delta_movement.y, 0.1176, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_advance_thaws_ticks_frozen_per_physics_step() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(apply_ticks_frozen(&mut world, 123, 7));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        world
            .advance_local_player_input(
                LocalPlayerInputState::default(),
                LOCAL_PHYSICS_TICK_SECONDS * 3.0,
            )
            .unwrap();

        assert_eq!(world.entities.ticks_frozen(123), Some(1));
    }

    #[test]
    fn local_player_blindness_prevents_sprint_speed() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_BLINDNESS_ID, 0,)));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    }

    #[test]
    fn local_player_low_food_prevents_sprint_speed_unless_mayfly() {
        let mut survival_world = flat_collision_world();
        apply_player_health(&mut survival_world, 6);
        survival_world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let survival_pose = survival_world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        let walk_step = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(survival_pose.position.z, 0.5 + walk_step, 0.000001);
        assert_f64_near(survival_pose.delta_movement.z, walk_step, 0.000001);

        let mut creative_world = flat_collision_world();
        apply_player_health(&mut creative_world, 6);
        creative_world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        creative_world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let creative_pose = creative_world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();
        let sprint_step = LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(creative_pose.position.z, 0.5 + sprint_step, 0.000001);
        assert_f64_near(creative_pose.delta_movement.z, sprint_step, 0.000001);
    }

    #[test]
    fn local_player_effective_sprint_uses_sprintable_vehicle_instead_of_food_when_mounted() {
        let input = LocalPlayerInputState {
            focused: true,
            forward: true,
            sprint: true,
            ..LocalPlayerInputState::default()
        };

        let mut camel_world = WorldStore::new();
        apply_player_health(&mut camel_world, 1);
        mount_local_player_on_entity(
            &mut camel_world,
            99,
            10,
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            vec![99],
        );
        assert!(camel_world.local_player_effective_sprint(input));

        let mut horse_world = WorldStore::new();
        apply_player_health(&mut horse_world, 20);
        mount_local_player_on_entity(
            &mut horse_world,
            99,
            20,
            VANILLA_ENTITY_TYPE_HORSE_ID,
            vec![99],
        );
        assert!(!horse_world.local_player_effective_sprint(input));

        let mut boat_world = WorldStore::new();
        apply_player_health(&mut boat_world, 20);
        mount_local_player_on_entity(
            &mut boat_world,
            99,
            30,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            vec![99],
        );
        assert!(!boat_world.local_player_effective_sprint(input));
    }

    #[test]
    fn local_player_effective_sprint_requires_controlling_sprintable_vehicle_passenger() {
        let mut world = WorldStore::new();
        apply_player_health(&mut world, 20);
        world.apply_add_entity(ProtocolAddEntity {
            id: 123,
            uuid: Uuid::from_u128(0x87654321876543218765432187654321),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        mount_local_player_on_entity(
            &mut world,
            99,
            10,
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            vec![123, 99],
        );

        assert!(!world.local_player_effective_sprint(LocalPlayerInputState {
            focused: true,
            forward: true,
            sprint: true,
            ..LocalPlayerInputState::default()
        }));
    }

    #[test]
    fn local_player_speed_and_slowness_effects_scale_horizontal_movement() {
        let mut speed_world = flat_collision_world();
        attach_local_player_entity(&mut speed_world, 123);
        assert!(speed_world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_SPEED_ID,
            1,
        )));
        let speed_pose =
            advance_forward_from_standard_start(&mut speed_world, LOCAL_PHYSICS_TICK_SECONDS);
        let speed_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 1.4 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(speed_pose.position.z, 0.5 + speed_step, 0.000001);
        assert_f64_near(speed_pose.delta_movement.z, speed_step, 0.000001);

        let mut slowness_world = flat_collision_world();
        attach_local_player_entity(&mut slowness_world, 124);
        assert!(slowness_world.apply_update_mob_effect(mob_effect(
            124,
            VANILLA_MOB_EFFECT_SLOWNESS_ID,
            0,
        )));
        let slowness_pose =
            advance_forward_from_standard_start(&mut slowness_world, LOCAL_PHYSICS_TICK_SECONDS);
        let slowness_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.85 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(slowness_pose.position.z, 0.5 + slowness_step, 0.000001);
        assert_f64_near(slowness_pose.delta_movement.z, slowness_step, 0.000001);
    }

    #[test]
    fn local_player_speed_effect_does_not_double_apply_synced_attribute_modifier() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE,
                modifiers: vec![ProtocolAttributeModifier {
                    id: VANILLA_SPEED_EFFECT_MODIFIER_ID.to_string(),
                    amount: 0.4,
                    operation_id: 2,
                }],
            }],
        }));
        assert!(world.apply_update_mob_effect(mob_effect(123, VANILLA_MOB_EFFECT_SPEED_ID, 1,)));

        let pose = advance_forward_from_standard_start(&mut world, LOCAL_PHYSICS_TICK_SECONDS);
        let expected_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 1.4 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
    }

    #[test]
    fn local_player_sneak_uses_sneaking_speed_attribute_when_present() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID,
                base: 0.4,
                modifiers: Vec::new(),
            }],
        }));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_step =
            LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * 0.4 * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_jump_uses_jump_strength_attribute_and_jump_boost_effect() {
        let mut world = flat_collision_world();
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
                base: 0.5,
                modifiers: Vec::new(),
            }],
        }));
        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_JUMP_BOOST_ID,
            1,
        )));
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.7, 0.000001);
        assert!(!pose.on_ground);
        assert!(pose.delta_movement.y > 0.0);
    }

    #[test]
    fn local_player_honey_block_jump_factor_scales_base_jump() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, HONEY_BLOCK_STATE_ID);
        let honey_top_y = 15.0 / 16.0;
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, honey_top_y, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_jump = LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE * HONEY_BLOCK_JUMP_FACTOR;
        assert_f64_near(pose.position.y, honey_top_y + expected_jump, 0.000001);
        assert!(!pose.on_ground);
        assert!(pose.delta_movement.y > 0.0);
    }

    #[test]
    fn local_player_honey_block_jump_factor_keeps_jump_boost_additive() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, HONEY_BLOCK_STATE_ID);
        attach_local_player_entity(&mut world, 123);
        assert!(world.apply_update_mob_effect(mob_effect(
            123,
            VANILLA_MOB_EFFECT_JUMP_BOOST_ID,
            0,
        )));
        let honey_top_y = 15.0 / 16.0;
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, honey_top_y, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_jump = LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE * HONEY_BLOCK_JUMP_FACTOR
            + JUMP_BOOST_VELOCITY_PER_LEVEL;
        assert_f64_near(pose.position.y, honey_top_y + expected_jump, 0.000001);
        assert!(!pose.on_ground);
        assert!(pose.delta_movement.y > 0.0);
    }

    #[test]
    fn local_player_bounces_after_landing_on_slime_block() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, SLIME_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0001);
        assert_f64_near(pose.delta_movement.y, 0.5, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_sneak_suppresses_slime_block_bounce() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 0, 0, SLIME_BLOCK_STATE_ID);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.2, 0.5),
            delta_movement: vec3(0.0, -0.5, 0.0),
            fall_distance: 2.0,
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    sneak: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.0, 0.0001);
        assert_f64_near(pose.delta_movement.y, 0.0, 0.000001);
        assert_f64_near(pose.fall_distance, 0.0, 0.000001);
        assert!(pose.on_ground);
    }

    #[test]
    fn local_player_sprint_jump_adds_vanilla_horizontal_impulse() {
        let mut world = flat_collision_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    jump: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_z = 0.5
            + LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS
            + SPRINT_JUMP_HORIZONTAL_IMPULSE;
        assert_f64_near(
            pose.position.y,
            1.0 + LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE,
            0.000001,
        );
        assert_f64_near(pose.position.z, expected_z, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_z - 0.5, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_ineligible_sprint_jump_does_not_add_horizontal_impulse() {
        let mut world = flat_collision_world();
        apply_player_health(&mut world, 6);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    jump: true,
                    sprint: true,
                    ..LocalPlayerInputState::default()
                },
                LOCAL_PHYSICS_TICK_SECONDS,
            )
            .unwrap();

        let expected_z =
            0.5 + LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(
            pose.position.y,
            1.0 + LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE,
            0.000001,
        );
        assert_f64_near(pose.position.z, expected_z, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_z - 0.5, 0.000001);
        assert!(!pose.on_ground);
    }

    #[test]
    fn local_player_jump_starts_only_from_ground() {
        let mut world = flat_collision_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        let jump_pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                0.05,
            )
            .unwrap();

        assert!(
            jump_pose.position.y > 1.0,
            "position was {:?}",
            jump_pose.position
        );
        assert!(!jump_pose.on_ground);
        assert!(jump_pose.delta_movement.y > 0.0);

        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 3.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });
        let airborne_pose = world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    jump: true,
                    ..LocalPlayerInputState::default()
                },
                0.1,
            )
            .unwrap();

        assert!(
            airborne_pose.position.y < 3.0,
            "position was {:?}",
            airborne_pose.position
        );
        assert!(!airborne_pose.on_ground);
    }

    fn advance_forward_from_standard_start(
        world: &mut WorldStore,
        seconds: f64,
    ) -> LocalPlayerPoseState {
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            y_rot: 0.0,
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });
        world
            .advance_local_player_input(
                LocalPlayerInputState {
                    focused: true,
                    forward: true,
                    ..LocalPlayerInputState::default()
                },
                seconds,
            )
            .unwrap()
    }

    fn flat_collision_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(empty_test_chunk());
        for x in 0..3 {
            for z in 0..8 {
                set_test_block(&mut world, x, 0, z, GRASS_BLOCK_STATE_ID);
            }
        }
        world
    }

    fn single_floor_block_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(empty_test_chunk());
        set_test_block(&mut world, 0, 0, 0, GRASS_BLOCK_STATE_ID);
        world
    }

    fn attach_local_player_entity(world: &mut WorldStore, id: i32) {
        world.local_player_id = Some(id);
        world.apply_add_entity(ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn mount_local_player_on_entity(
        world: &mut WorldStore,
        player_id: i32,
        vehicle_id: i32,
        entity_type_id: i32,
        passenger_ids: Vec<i32>,
    ) {
        attach_local_player_entity(world, player_id);
        world.apply_add_entity(ProtocolAddEntity {
            id: vehicle_id,
            uuid: Uuid::from_u128(0x22345678123456781234567812345678),
            entity_type_id,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id,
            passenger_ids,
        }));
    }

    fn mob_effect(entity_id: i32, effect_id: i32, amplifier: i32) -> ProtocolUpdateMobEffect {
        ProtocolUpdateMobEffect {
            entity_id,
            effect_id,
            amplifier,
            duration_ticks: 200,
            flags: ProtocolMobEffectFlags::default(),
        }
    }

    fn apply_player_health(world: &mut WorldStore, food: i32) {
        world.apply_player_health(ProtocolPlayerHealth {
            health: 20.0,
            food,
            saturation: 5.0,
        });
    }

    fn apply_no_gravity(world: &mut WorldStore, entity_id: i32, no_gravity: bool) -> bool {
        world.apply_set_entity_data(ProtocolSetEntityData {
            id: entity_id,
            values: vec![ProtocolEntityDataValue {
                data_id: crate::entities::VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
                serializer_id: 8,
                value: ProtocolEntityDataValueKind::Boolean(no_gravity),
            }],
        })
    }

    fn apply_ticks_frozen(world: &mut WorldStore, entity_id: i32, ticks_frozen: i32) -> bool {
        world.apply_set_entity_data(ProtocolSetEntityData {
            id: entity_id,
            values: vec![ProtocolEntityDataValue {
                data_id: crate::entities::VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
                serializer_id: 1,
                value: ProtocolEntityDataValueKind::Int(ticks_frozen),
            }],
        })
    }

    fn item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: bbb_protocol::packets::DataComponentPatchSummary::default(),
        }
    }

    fn equip_powder_snow_walkable_boots(world: &mut WorldStore, count: i32) {
        world.set_powder_snow_walkable_foot_item_ids(BTreeSet::from([LEATHER_BOOTS_ITEM_ID]));
        world.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: PLAYER_FEET_EQUIPMENT_SLOT_ID,
            item: item_stack(LEATHER_BOOTS_ITEM_ID, count),
        });
    }

    fn apply_flying_abilities(world: &mut WorldStore, flying_speed: f32) {
        world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: false,
            flying: true,
            can_fly: true,
            instabuild: false,
            flying_speed,
            walking_speed: 0.1,
        });
    }

    fn bubble_column_block_state_id(drag_down: bool) -> i32 {
        let mut properties = BTreeMap::new();
        properties.insert("drag".to_string(), drag_down.to_string());
        crate::registries::BlockStateRegistry::vanilla_26_1()
            .find_by_name_and_properties("minecraft:bubble_column", &properties)
            .expect("vanilla 26.1 bubble_column block state exists")
            .id
    }

    fn empty_test_chunk() -> ChunkColumn {
        ChunkColumn {
            pos: crate::ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        }
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_test_block(world: &mut WorldStore, x: i32, y: i32, z: i32, block_state_id: i32) {
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x, y, z },
                block_state_id,
            })
        );
    }

    fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
        ProtocolVec3d { x, y, z }
    }

    fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {actual} to be within {epsilon} of {expected}"
        );
    }
}
