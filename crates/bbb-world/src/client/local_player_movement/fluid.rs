use super::super::local_player_collision::local_player_block_collision_is_empty;
use super::super::local_player_fluid::{
    local_player_bounds_contains_any_fluid, LocalPlayerFluidContactState,
};
use super::*;

pub(super) fn advance_local_player_fluid_physics_step(
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
