use super::super::local_player_fluid::{
    local_player_bounds_contains_any_fluid, EntityFluidContactState,
};
use super::*;

pub(super) fn advance_local_player_fluid_physics_step(
    world: &WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    step_seconds: f64,
    initial_fluid_contact: EntityFluidContactState,
    stuck_multiplier: Option<EntityVec3>,
    entity_collision_targets: &[crate::EntityPickTargetState],
) -> (LocalPlayerPoseState, EntityBlockEffectMovement) {
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

    // Vanilla ordering: `LivingEntity.aiStep` applies the held-jump liquid
    // rise and `LocalPlayer.aiStep` the sneak descend, then `Player.travel`
    // steers the swimming vertical toward the look angle, and only then
    // `travelInFluid` snapshots the falling flag before `moveRelative`.
    if input.focused && input.jump {
        pose.delta_movement.y += LOCAL_INPUT_LIQUID_JUMP_VELOCITY_PER_TICK * step_ticks;
    }
    if input.focused && input.sneak && initial_fluid_contact.in_water() {
        pose.delta_movement.y -= LOCAL_INPUT_WATER_SNEAK_DESCEND_VELOCITY_PER_TICK * step_ticks;
    }
    pose.delta_movement = local_player_swimming_vertical_velocity(
        world,
        pose,
        input,
        initial_fluid_contact,
        pose.delta_movement,
    );
    let is_falling = pose.delta_movement.y <= 0.0;

    let fluid_relative_speed = if initial_fluid_contact.in_water() {
        local_player_water_relative_speed_per_tick(world, pose.on_ground)
    } else {
        LOCAL_INPUT_FLUID_SPEED_PER_TICK
    };
    // `moveRelative` consumes the same `modifyInput`-shaped axes as the
    // land chain and accumulates into the carried delta.
    let (input_x, input_z) =
        local_player_modified_input(world, pose, input, initial_fluid_contact.in_water());
    let (acceleration_x, acceleration_z) =
        local_player_input_acceleration(pose, input_x, input_z, fluid_relative_speed);
    pose.delta_movement.x += acceleration_x * step_ticks;
    pose.delta_movement.z += acceleration_z * step_ticks;

    let mut requested_x = pose.delta_movement.x * step_ticks;
    let mut requested_y = pose.delta_movement.y * step_ticks;
    let mut requested_z = pose.delta_movement.z * step_ticks;
    // Vanilla `Entity.move` consumes the stuck-speed multiplier (zeroing the
    // stored delta) and edge-backs-off inside fluids too.
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
    let movement = world.clip_local_player_collision_movement(
        pose,
        [requested_x, requested_y, requested_z],
        LOCAL_PLAYER_STEP_HEIGHT as f32,
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
    pose.position.x += movement[0];
    pose.position.y += movement[1];
    pose.position.z += movement[2];

    let x_collision = local_player_horizontal_axis_collided(requested_x, movement[0]);
    let z_collision = local_player_horizontal_axis_collided(requested_z, movement[2]);
    let horizontal_collision = x_collision || z_collision;
    let vertical_collision = movement[1] != requested_y;
    let on_ground = vertical_collision && requested_y < 0.0
        || local_player_supported(world, pose, collision_context);
    let bounced_on_slime = vertical_collision
        && requested_y < 0.0
        && !input.sneak
        && local_player_standing_on_block(world, pose, "minecraft:slime_block");
    let fluid_contact = local_player_fluid_contact(world, pose);
    pose.fall_distance = if fluid_contact.in_water() {
        0.0
    } else {
        next_local_player_fall_distance(pose.fall_distance, movement[1], on_ground)
    };

    // `Entity.move` tail on the carried delta: zero the collided horizontal
    // axes, let the supporting block's `updateEntityMovementAfterFallOn`
    // resolve the vertical axis, then multiply in the block speed factor.
    if x_collision {
        pose.delta_movement.x = 0.0;
    }
    if z_collision {
        pose.delta_movement.z = 0.0;
    }
    if vertical_collision {
        pose.delta_movement.y = if bounced_on_slime && pose.delta_movement.y < 0.0 {
            -pose.delta_movement.y
        } else {
            0.0
        };
    }
    let block_speed_factor = local_player_block_speed_factor(world, pose);
    pose.delta_movement.x *= block_speed_factor;
    pose.delta_movement.z *= block_speed_factor;

    // `travelInWater` post-move: a horizontal collision on a climbable sets
    // the vertical delta to 0.2 before the drag multiply (the water branch
    // keys on the collision alone, unlike `travelInAir`'s jump-or-collide).
    if initial_fluid_contact.in_water()
        && horizontal_collision
        && local_player_climbable_kind(world, pose).is_some()
    {
        pose.delta_movement.y = LOCAL_PLAYER_CLIMBABLE_UPWARD_VELOCITY_PER_TICK;
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
    pose.on_ground = on_ground;
    pose.horizontal_collision = horizontal_collision;
    (pose, step_move)
}

fn local_player_swimming_vertical_velocity(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    fluid_contact: EntityFluidContactState,
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

pub(crate) fn local_player_lava_current_push_per_tick(world: &WorldStore) -> f64 {
    let fast_lava = world
        .sampled_client_environment_attributes()
        .ok()
        .and_then(|attributes| attributes.fast_lava)
        .unwrap_or_else(|| {
            if world
                .registry_content(crate::DIMENSION_TYPE_REGISTRY)
                .is_some()
            {
                return false;
            }
            world.level_info().is_some_and(|level| {
                level.dimension == "minecraft:the_nether"
                    || level.dimension_type_name.as_deref() == Some("minecraft:the_nether")
            })
        });
    if fast_lava {
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
