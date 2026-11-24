use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct BoatInput {
    pub(super) left: bool,
    pub(super) right: bool,
    pub(super) forward: bool,
    pub(super) backward: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HappyGhastInput {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) z: f64,
}

impl HappyGhastInput {
    pub(super) fn from_local_input(
        input: LocalPlayerInputState,
        controller_x_rot: f32,
        flying_speed: f64,
        controller_is_camera: bool,
        slow_factor: f64,
    ) -> Self {
        let accepts_input = input.focused && controller_is_camera;
        let (strafe, forward_axis) = if accepts_input {
            local_player_move_vector(input, slow_factor)
        } else {
            (0.0, 0.0)
        };
        let mut forward = 0.0_f32;
        let mut up = 0.0_f32;
        if forward_axis != 0.0 {
            let pitch = controller_x_rot * (std::f64::consts::PI / 180.0) as f32;
            forward = crate::entities::client_movement::vanilla_mth_cos(f64::from(pitch));
            up = -crate::entities::client_movement::vanilla_mth_sin(f64::from(pitch));
            if forward_axis < 0.0 {
                forward *= -0.5;
                up *= -0.5;
            }
        }
        if accepts_input && input.jump {
            up += 0.5;
        }
        let scale = f64::from(3.9_f32) * flying_speed;
        Self {
            x: f64::from(strafe) * scale,
            y: f64::from(up) * scale,
            z: f64::from(forward) * scale,
        }
    }
}

impl BoatInput {
    pub(super) fn from_local_input(input: LocalPlayerInputState) -> Self {
        Self {
            left: input.focused && input.left,
            right: input.focused && input.right,
            forward: input.focused && input.forward,
            backward: input.focused && input.backward,
        }
    }
}

pub(super) fn bool_axis(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

pub(super) fn local_player_move_vector(
    input: LocalPlayerInputState,
    slow_factor: f64,
) -> (f32, f32) {
    let mut x = bool_axis(input.left, input.right);
    let mut y = bool_axis(input.forward, input.backward);
    let raw_length = (x * x + y * y).sqrt();
    if raw_length < 1.0e-4_f32 {
        return (0.0, 0.0);
    }
    x /= raw_length;
    y /= raw_length;
    let scale = 0.98_f32 * slow_factor as f32;
    x *= scale;
    y *= scale;

    let length = (x * x + y * y).sqrt();
    if length <= 0.0 {
        return (x, y);
    }
    let direction_x = x / length;
    let direction_y = y / length;
    let abs_x = direction_x.abs();
    let abs_y = direction_y.abs();
    let tan = if abs_y > abs_x {
        abs_x / abs_y
    } else {
        abs_y / abs_x
    };
    let distance_to_square = (1.0_f32 + tan * tan).sqrt();
    let modified_length = (length * distance_to_square).min(1.0);
    (direction_x * modified_length, direction_y * modified_length)
}

pub(super) fn add_relative_movement(
    transform: &mut crate::entities::record::EntityTransform,
    input: HappyGhastInput,
    speed: f64,
) {
    let length_squared = input.x * input.x + input.y * input.y + input.z * input.z;
    if length_squared < 1.0e-7 {
        return;
    }
    let normalizer = if length_squared > 1.0 {
        length_squared.sqrt().recip()
    } else {
        1.0
    };
    let movement_x = input.x * normalizer * speed;
    let movement_y = input.y * normalizer * speed;
    let movement_z = input.z * normalizer * speed;
    let yaw = transform.y_rot * (std::f64::consts::PI / 180.0) as f32;
    let sin = crate::entities::client_movement::vanilla_mth_sin(f64::from(yaw));
    let cos = crate::entities::client_movement::vanilla_mth_cos(f64::from(yaw));
    transform.delta_movement.x += movement_x * f64::from(cos) - movement_z * f64::from(sin);
    transform.delta_movement.y += movement_y;
    transform.delta_movement.z += movement_z * f64::from(cos) + movement_x * f64::from(sin);
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LocalLivingMoveOutcome {
    pub(super) transform: crate::entities::record::EntityTransform,
    pub(super) supporting_block: Option<crate::BlockPos>,
    pub(super) horizontal_collision: bool,
    /// The `Entity.Movement` this move records for `applyEffectsFromBlocks`:
    /// pre-move origin, collision-clipped destination, and the requested
    /// vector that drives Vanilla's axis-dependent traversal order.
    pub(super) movement: EntityBlockEffectMovement,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn move_local_living_entity(
    world: &WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    max_up_step: f32,
    previous_supporting_block: crate::entities::record::EntitySupportingBlock,
    entity_collision_targets: &[crate::entities::EntityPickTargetState],
    mut transform: crate::entities::record::EntityTransform,
) -> LocalLivingMoveOutcome {
    let position = [
        transform.position.x,
        transform.position.y,
        transform.position.z,
    ];
    let requested = transform.delta_movement;
    let clipped = world.clip_locally_authoritative_entity_collision_movement(
        entity_id,
        entity_type_id,
        position,
        [requested.x, requested.y, requested.z],
        relative_min,
        relative_max,
        max_up_step,
        transform.on_ground.unwrap_or(false),
        entity_collision_targets,
    );
    transform.position.x += clipped[0];
    transform.position.y += clipped[1];
    transform.position.z += clipped[2];
    let x_collision = (requested.x - clipped[0]).abs() >= f64::from(1.0e-5_f32);
    let y_collision = requested.y != clipped[1];
    let z_collision = (requested.z - clipped[2]).abs() >= f64::from(1.0e-5_f32);
    let on_ground = y_collision && requested.y < 0.0;
    transform.on_ground = Some(on_ground);
    let supporting_block = on_ground
        .then(|| {
            world.entity_supporting_block_after_collision(
                entity_type_id,
                position,
                clipped,
                relative_min,
                relative_max,
                previous_supporting_block,
            )
        })
        .flatten();
    if x_collision {
        transform.delta_movement.x = 0.0;
    }
    if y_collision {
        transform.delta_movement.y = living_vertical_collision_response(
            world,
            [
                transform.position.x,
                transform.position.y,
                transform.position.z,
            ],
            supporting_block,
            requested.y,
            world.entities.is_descending(entity_id).unwrap_or(false),
        );
    }
    if z_collision {
        transform.delta_movement.z = 0.0;
    }
    let speed_factor = entity_block_speed_factor(
        world,
        [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ],
    );
    transform.delta_movement.x *= speed_factor;
    transform.delta_movement.z *= speed_factor;
    LocalLivingMoveOutcome {
        movement: EntityBlockEffectMovement::clipped(
            EntityVec3 {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            transform.position,
            requested,
        ),
        transform,
        supporting_block,
        horizontal_collision: x_collision || z_collision,
    }
}

/// `Entity.move` for the directly controlled Living vehicle: consume the
/// pending stuck-speed multiplier into the requested movement, run the shared
/// clipped move, and zero the post-move delta exactly as Vanilla does when a
/// multiplier was pending.
#[allow(clippy::too_many_arguments)]
pub(super) fn move_local_living_entity_consuming_stuck(
    world: &mut WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    max_up_step: f32,
    previous_supporting_block: crate::entities::record::EntitySupportingBlock,
    entity_collision_targets: &[crate::entities::EntityPickTargetState],
    mut transform: crate::entities::record::EntityTransform,
) -> LocalLivingMoveOutcome {
    let stuck_multiplier = world
        .entities
        .consume_block_effect_stuck_speed_multiplier(entity_id);
    if let Some(multiplier) = stuck_multiplier {
        transform.delta_movement.x *= multiplier.x;
        transform.delta_movement.y *= multiplier.y;
        transform.delta_movement.z *= multiplier.z;
    }
    let mut outcome = move_local_living_entity(
        world,
        entity_id,
        entity_type_id,
        relative_min,
        relative_max,
        max_up_step,
        previous_supporting_block,
        entity_collision_targets,
        transform,
    );
    if stuck_multiplier.is_some() {
        outcome.transform.delta_movement = EntityVec3::default();
    }
    world
        .entities
        .record_local_vehicle_move(entity_id, outcome.movement);
    outcome
}

pub(super) fn store_living_move(
    entities: &mut crate::entities::EntityStore,
    entity_id: i32,
    transform: crate::entities::record::EntityTransform,
    supporting_block: Option<crate::BlockPos>,
) -> Option<()> {
    let entity = entities.record_mut(entity_id)?;
    entity.transform = transform;
    entity.animations.set_ridden_body_y_rot(transform.y_rot);
    if transform.on_ground.unwrap_or(false) {
        entity.supporting_block.main = supporting_block;
        entity.supporting_block.on_ground_no_blocks = supporting_block.is_none();
    } else {
        entity.supporting_block.clear_physical();
    }
    Some(())
}

pub(super) fn fluid_falling_adjusted_velocity(
    movement_y: f64,
    gravity: f64,
    is_falling: bool,
    sprinting: bool,
) -> f64 {
    bbb_facts::movement::fluid_falling_adjusted_velocity(
        movement_y, gravity, is_falling, sprinting, 1.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn jump_out_of_fluid_if_free(
    world: &WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    entity_collision_targets: &[crate::entities::EntityPickTargetState],
    old_y: f64,
    horizontal_collision: bool,
    transform: &mut crate::entities::record::EntityTransform,
) {
    if !horizontal_collision {
        return;
    }
    let movement = transform.delta_movement;
    let test_position = [
        transform.position.x + movement.x,
        transform.position.y + movement.y + f64::from(0.6_f32) - transform.position.y + old_y,
        transform.position.z + movement.z,
    ];
    let collision_free = world.entity_collision_bounds_are_free(
        entity_id,
        entity_type_id,
        test_position,
        relative_min,
        relative_max,
        transform.on_ground.unwrap_or(false),
        entity_collision_targets,
    );
    let aabb_min = std::array::from_fn(|axis| test_position[axis] + relative_min[axis]);
    let aabb_max = std::array::from_fn(|axis| test_position[axis] + relative_max[axis]);
    if collision_free && !crate::client::entity_bounds_contains_any_fluid(world, aabb_min, aabb_max)
    {
        transform.delta_movement.y = f64::from(0.3_f32);
    }
}

pub(super) fn entity_movement_block_friction(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
) -> f32 {
    let pos = entity_movement_effect_block_pos(world, position, supporting_block);
    let block_name = world.block_view(pos).and_then(|block| block.block_name);
    crate::terrain::vanilla_block_friction(block_name)
}

pub(super) fn entity_block_jump_factor(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
) -> f32 {
    let here = crate::BlockPos {
        x: position[0].floor() as i32,
        y: position[1].floor() as i32,
        z: position[2].floor() as i32,
    };
    let here_name = world.block_view(here).and_then(|block| block.block_name);
    let here_factor = crate::terrain::vanilla_block_jump_factor(here_name);
    if here_factor != 1.0 {
        return here_factor;
    }
    let below = entity_movement_effect_block_pos(world, position, supporting_block);
    let below_name = world.block_view(below).and_then(|block| block.block_name);
    crate::terrain::vanilla_block_jump_factor(below_name)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_local_horse_jump(
    world: &WorldStore,
    vehicle_id: i32,
    amount: f32,
    input: HappyGhastInput,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    transform: &mut crate::entities::record::EntityTransform,
) -> Option<()> {
    let mut impulse = world
        .entities
        .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)?
        as f32
        * amount
        * entity_block_jump_factor(world, position, supporting_block);
    if let Some(amplifier) = world
        .entities
        .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
    {
        impulse += 0.1_f32 * (amplifier + 1) as f32;
    }
    transform.delta_movement.y = f64::from(impulse);
    if input.z > 0.0 {
        let yaw = transform.y_rot * std::f32::consts::PI / 180.0;
        let sin = crate::entities::client_movement::vanilla_mth_sin(f64::from(yaw));
        let cos = crate::entities::client_movement::vanilla_mth_cos(f64::from(yaw));
        transform.delta_movement.x += f64::from(-0.4_f32 * sin * amount);
        transform.delta_movement.z += f64::from(0.4_f32 * cos * amount);
    }
    Some(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_local_camel_dash(
    world: &mut WorldStore,
    vehicle_id: i32,
    entity_type_id: i32,
    amount: f32,
    movement_speed: f64,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    transform: &mut crate::entities::record::EntityTransform,
) -> Option<()> {
    let mut jump_momentum = world
        .entities
        .attribute_value(vehicle_id, VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)?
        as f32
        * entity_block_jump_factor(world, position, supporting_block);
    if let Some(amplifier) = world
        .entities
        .mob_effect_amplifier(vehicle_id, VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
    {
        jump_momentum += 0.1_f32 * (amplifier + 1) as f32;
    }
    let yaw = transform.y_rot * std::f32::consts::PI / 180.0;
    let sin = crate::entities::client_movement::vanilla_mth_sin(f64::from(yaw));
    let cos = crate::entities::client_movement::vanilla_mth_cos(f64::from(yaw));
    let horizontal_momentum = f64::from(22.2222_f32 * amount)
        * movement_speed
        * entity_block_speed_factor(world, position);
    transform.delta_movement.x += f64::from(-sin) * horizontal_momentum;
    transform.delta_movement.y += f64::from(1.4285_f32 * amount) * f64::from(jump_momentum);
    transform.delta_movement.z += f64::from(cos) * horizontal_momentum;
    let (dash_data_id, _) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(entity_type_id)?;
    let _ = world.entities.with_metadata_mut(vehicle_id, |metadata| {
        set_metadata_bool(metadata, dash_data_id, true);
    });
    world
        .entities
        .start_local_camel_dash_after_tick(vehicle_id)?;
    Some(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CamelRiddenPoseState {
    pub(super) is_sitting: bool,
    pub(super) is_in_transition: bool,
    pub(super) refuses_to_move: bool,
}

pub(super) fn camel_ridden_pose_state(
    world: &WorldStore,
    entity_id: i32,
) -> Option<CamelRiddenPoseState> {
    let entity_type_id = world.entities.entity_type_id(entity_id)?;
    let (data_id, default) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )?;
    let last_pose_change_tick = world.entities.metadata_long(entity_id, data_id, default)?;
    let game_time = world.world_time().map_or(0, |time| time.game_time);
    let is_sitting = last_pose_change_tick < 0;
    let pose_time = game_time.saturating_sub(last_pose_change_tick.saturating_abs());
    let is_in_transition = pose_time < if is_sitting { 40 } else { 52 };
    Some(CamelRiddenPoseState {
        is_sitting,
        is_in_transition,
        refuses_to_move: is_sitting || is_in_transition,
    })
}

pub(super) fn entity_movement_effect_block_pos(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
) -> crate::BlockPos {
    let position_y = (position[1] - f64::from(0.500001_f32)).floor() as i32;
    if let Some(mut support) = supporting_block {
        let keeps_support_y = world
            .block_view(support)
            .and_then(|block| block.block_name)
            .is_some_and(|name| {
                crate::terrain::block_name_is_fence(name)
                    || crate::terrain::block_name_is_wall(name)
                    || crate::terrain::block_name_is_fence_gate(name)
            });
        if !keeps_support_y {
            support.y = position_y;
        }
        support
    } else {
        crate::BlockPos {
            x: position[0].floor() as i32,
            y: position_y,
            z: position[2].floor() as i32,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn float_boat(
    world: &WorldStore,
    entity_id: i32,
    entity_type_id: i32,
    relative_min: [f64; 3],
    relative_max: [f64; 3],
    no_gravity: bool,
    first_passenger_is_player: bool,
    entity_collision_targets: &[crate::entities::EntityPickTargetState],
    movement: &mut crate::entities::record::EntityBoatMovement,
    transform: &mut crate::entities::record::EntityTransform,
) {
    if movement.old_status == Some(EntityBoatStatus::InAir)
        && !matches!(
            movement.status,
            Some(EntityBoatStatus::InAir | EntityBoatStatus::OnLand)
        )
    {
        movement.water_level = transform.position.y + relative_max[1];
        let position = [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ];
        let aabb_min = std::array::from_fn(|axis| position[axis] + relative_min[axis]);
        let aabb_max = std::array::from_fn(|axis| position[axis] + relative_max[axis]);
        let target_y =
            crate::fluid::boat_water_level_above(world, aabb_min, aabb_max, movement.last_yd)
                - (relative_max[1] - relative_min[1])
                + 0.101;
        let target_position = [transform.position.x, target_y, transform.position.z];
        if world.entity_collision_bounds_are_free(
            entity_id,
            entity_type_id,
            target_position,
            relative_min,
            relative_max,
            transform.on_ground.unwrap_or(false),
            entity_collision_targets,
        ) {
            transform.position.y = target_y;
            transform.delta_movement.y = 0.0;
            movement.last_yd = 0.0;
        }
        movement.status = Some(EntityBoatStatus::InWater);
        return;
    }

    let (next_delta, next_rotation, next_land_friction) = bbb_facts::movement::boat_float_step(
        [
            transform.delta_movement.x,
            transform.delta_movement.y,
            transform.delta_movement.z,
        ],
        movement.delta_rotation,
        bbb_facts::movement::BoatFloatInput {
            status: movement.status.map(|status| match status {
                EntityBoatStatus::InWater => bbb_facts::movement::BoatPhysicsStatus::InWater,
                EntityBoatStatus::UnderWater => bbb_facts::movement::BoatPhysicsStatus::UnderWater,
                EntityBoatStatus::UnderFlowingWater => {
                    bbb_facts::movement::BoatPhysicsStatus::UnderFlowingWater
                }
                EntityBoatStatus::OnLand => bbb_facts::movement::BoatPhysicsStatus::OnLand,
                EntityBoatStatus::InAir => bbb_facts::movement::BoatPhysicsStatus::InAir,
            }),
            position_y: transform.position.y,
            bb_height: relative_max[1] - relative_min[1],
            gravity: if no_gravity {
                0.0
            } else {
                bbb_facts::movement::BOAT_DEFAULT_GRAVITY
            },
            water_level: movement.water_level,
            land_friction: movement.land_friction,
            has_controlling_player: first_passenger_is_player,
        },
    );
    transform.delta_movement.x = next_delta[0];
    transform.delta_movement.y = next_delta[1];
    transform.delta_movement.z = next_delta[2];
    movement.delta_rotation = next_rotation;
    movement.land_friction = next_land_friction;
}

pub(super) fn control_boat(
    input: BoatInput,
    movement: &mut crate::entities::record::EntityBoatMovement,
    transform: &mut crate::entities::record::EntityTransform,
) {
    if input.left {
        movement.delta_rotation -= 1.0;
    }
    if input.right {
        movement.delta_rotation += 1.0;
    }
    let mut acceleration = 0.0_f32;
    if input.right != input.left && !input.forward && !input.backward {
        acceleration += 0.005;
    }
    transform.y_rot = wrap_degrees_f32(transform.y_rot + movement.delta_rotation);
    if input.forward {
        acceleration += 0.04;
    }
    if input.backward {
        acceleration -= 0.005;
    }
    let yaw = transform.y_rot * (std::f64::consts::PI / 180.0) as f32;
    transform.delta_movement.x += f64::from(
        crate::entities::client_movement::vanilla_mth_sin(-f64::from(yaw)) * acceleration,
    );
    transform.delta_movement.z +=
        f64::from(crate::entities::client_movement::vanilla_mth_cos(f64::from(yaw)) * acceleration);
}

pub(super) fn boat_vertical_collision_response(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    requested_y: f64,
) -> f64 {
    let effect_pos = crate::BlockPos {
        x: supporting_block.map_or(position[0].floor() as i32, |pos| pos.x),
        y: (position[1] - 0.2).floor() as i32,
        z: supporting_block.map_or(position[2].floor() as i32, |pos| pos.z),
    };
    let block_name = world
        .block_view(effect_pos)
        .and_then(|block| block.block_name);
    if requested_y >= 0.0 {
        return if block_name.is_some_and(|name| {
            name == "minecraft:slime_block" || crate::terrain::block_name_is_bed(name)
        }) {
            requested_y
        } else {
            0.0
        };
    }
    match block_name {
        Some("minecraft:slime_block") => -requested_y * 0.8,
        Some(name) if crate::terrain::block_name_is_bed(name) => {
            -requested_y * f64::from(0.66_f32) * 0.8
        }
        _ => 0.0,
    }
}

pub(super) fn living_vertical_collision_response(
    world: &WorldStore,
    position: [f64; 3],
    supporting_block: Option<crate::BlockPos>,
    requested_y: f64,
    suppressing_bounce: bool,
) -> f64 {
    let effect_pos = crate::BlockPos {
        x: supporting_block.map_or(position[0].floor() as i32, |pos| pos.x),
        y: (position[1] - 0.2).floor() as i32,
        z: supporting_block.map_or(position[2].floor() as i32, |pos| pos.z),
    };
    let block_name = world
        .block_view(effect_pos)
        .and_then(|block| block.block_name);
    if suppressing_bounce {
        return 0.0;
    }
    match block_name {
        Some("minecraft:slime_block") if requested_y < 0.0 => -requested_y,
        Some(name) if crate::terrain::block_name_is_bed(name) && requested_y < 0.0 => {
            -requested_y * f64::from(0.66_f32)
        }
        Some("minecraft:slime_block") => requested_y,
        Some(name) if crate::terrain::block_name_is_bed(name) => requested_y,
        _ => 0.0,
    }
}

pub(super) fn entity_block_speed_factor(world: &WorldStore, position: [f64; 3]) -> f64 {
    let block_pos = crate::BlockPos {
        x: position[0].floor() as i32,
        y: position[1].floor() as i32,
        z: position[2].floor() as i32,
    };
    let here = world
        .block_view(block_pos)
        .and_then(|block| block.block_name)
        .unwrap_or_default();
    if matches!(here, "minecraft:water" | "minecraft:bubble_column") {
        return 1.0;
    }
    let here_factor = f64::from(crate::terrain::vanilla_block_speed_factor(Some(here)));
    if here_factor != 1.0 {
        return here_factor;
    }
    let below = crate::BlockPos {
        y: (position[1] - f64::from(0.500001_f32)).floor() as i32,
        ..block_pos
    };
    world
        .block_view(below)
        .and_then(|block| block.block_name)
        .map_or(1.0, |name| {
            f64::from(crate::terrain::vanilla_block_speed_factor(Some(name)))
        })
}

pub(super) fn set_metadata_bool(
    metadata: &mut crate::entities::record::EntityMetadata,
    data_id: u8,
    value: bool,
) {
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(value),
        },
    );
}

pub(super) fn set_metadata_byte(
    metadata: &mut crate::entities::record::EntityMetadata,
    data_id: u8,
    value: i8,
) {
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(value),
        },
    );
}

pub(super) fn set_metadata_long(
    metadata: &mut crate::entities::record::EntityMetadata,
    data_id: u8,
    value: i64,
) {
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
            value: EntityDataValueKind::Long(value),
        },
    );
}

pub(super) fn set_metadata_pose(
    metadata: &mut crate::entities::record::EntityMetadata,
    data_id: u8,
    value: i32,
) {
    set_metadata_value(
        metadata,
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: EntityDataValueKind::Pose(value),
        },
    );
}

pub(super) fn set_metadata_value(
    metadata: &mut crate::entities::record::EntityMetadata,
    value: EntityDataValue,
) {
    if metadata.data_values.contains(&value) {
        return;
    }
    let data_id = value.data_id;
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

pub(super) fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

pub(super) fn is_vanilla_player_rideable_jumping_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_player_rideable_jumping(entity_type_id)
}

pub(super) fn is_vanilla_sprintable_vehicle_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_camel(entity_type_id)
}

pub(super) fn is_vanilla_custom_inventory_screen_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_has_custom_inventory_screen(entity_type_id)
}
