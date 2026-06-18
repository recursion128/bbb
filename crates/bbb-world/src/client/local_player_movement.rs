use bbb_protocol::packets::Vec3d as ProtocolVec3d;

use super::local_player::{LocalPlayerAbilitiesState, LocalPlayerInputState, LocalPlayerPoseState};
use super::local_player_collision::{
    local_player_collides, CollisionAxis as Axis, LocalPlayerBounds, COLLISION_EPSILON,
};
use crate::WorldStore;

pub(super) const LOCAL_INPUT_MOUSE_SENSITIVITY_DEGREES: f32 = 0.12;
pub(super) const LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND: f64 = 4.317;
pub(super) const LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND: f64 = 5.612;

const VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID: i32 = 22;
const VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID: i32 = 15;
const VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID: i32 = 26;
const VANILLA_MOB_EFFECT_SPEED_ID: i32 = 0;
const VANILLA_MOB_EFFECT_SLOWNESS_ID: i32 = 1;
const VANILLA_MOB_EFFECT_JUMP_BOOST_ID: i32 = 7;
const LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE: f64 = 0.1;
const LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE: f64 = 0.42;
const LOCAL_INPUT_DEFAULT_FLYING_SPEED_ATTRIBUTE: f64 = 0.05;
const LOCAL_INPUT_DEFAULT_FLY_SPEED_BLOCKS_PER_SECOND: f64 = 10.89;
const LOCAL_INPUT_SPRINT_SPEED_MULTIPLIER: f64 =
    LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND / LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND;
const LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER: f64 = 2.0;
const LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER: f64 = 0.3;
const LOCAL_INPUT_FLY_VERTICAL_SPEED_MULTIPLIER: f64 = 3.0;
const LOCAL_INPUT_FLY_VERTICAL_DAMPING: f64 = 0.6;
const LOCAL_PHYSICS_TICK_SECONDS: f64 = 0.05;
const LOCAL_GRAVITY_PER_TICK: f64 = 0.08;
const LOCAL_VERTICAL_FRICTION: f64 = 0.98;
const LOCAL_PLAYER_STEP_HEIGHT: f64 = 0.6;
const SUPPORT_EPSILON: f64 = 1.0e-3;
const EDGE_BACKOFF_STEP: f64 = 0.05;
const COLLISION_CLIP_STEPS: usize = 12;
const SPEED_EFFECT_MOVEMENT_SPEED_MULTIPLIER: f64 = 0.2;
const SLOWNESS_EFFECT_MOVEMENT_SPEED_MULTIPLIER: f64 = -0.15;
const JUMP_BOOST_VELOCITY_PER_LEVEL: f64 = 0.1;
const VANILLA_SPEED_EFFECT_MODIFIER_ID: &str = "minecraft:effect.speed";
const VANILLA_SLOWNESS_EFFECT_MODIFIER_ID: &str = "minecraft:effect.slowness";

pub(super) fn integrate_local_player_input_pose(
    world: &WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    dt_seconds: f64,
) -> LocalPlayerPoseState {
    pose = apply_local_player_input_look(pose, input);

    let mut remaining_seconds = dt_seconds.max(0.0);
    while remaining_seconds > COLLISION_EPSILON {
        let step_seconds = remaining_seconds.min(LOCAL_PHYSICS_TICK_SECONDS);
        pose = advance_local_player_physics_step(world, pose, input, step_seconds);
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

fn advance_local_player_physics_step(
    world: &WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    step_seconds: f64,
) -> LocalPlayerPoseState {
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
    let speed = local_player_horizontal_speed(world, input);
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
    if flying.is_none() && input.focused && input.jump && pose.on_ground {
        let jump_velocity = local_player_jump_velocity(world);
        if jump_velocity > 1.0e-5 {
            pose.delta_movement.y = pose.delta_movement.y.max(jump_velocity);
        }
    }

    let step_ticks = step_seconds / LOCAL_PHYSICS_TICK_SECONDS;
    let mut requested_x = move_x * speed * step_seconds;
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
    let mut requested_z = move_z * speed * step_seconds;
    if should_back_off_from_edge(world, pose, input, requested_y) {
        (requested_x, requested_z) =
            back_off_from_edge(world, pose.position, requested_x, requested_z);
    }
    let movement = clip_local_player_movement(
        world,
        pose.position,
        requested_x,
        requested_y,
        requested_z,
        pose.on_ground,
    );
    pose.position.x += movement.x;
    pose.position.y += movement.y;
    pose.position.z += movement.z;

    let tick_span = step_ticks.max(f64::EPSILON);
    let horizontal_collision = (movement.x - requested_x).abs() > COLLISION_EPSILON
        || (movement.z - requested_z).abs() > COLLISION_EPSILON;
    let vertical_collision = (movement.y - requested_y).abs() > COLLISION_EPSILON;
    let on_ground =
        vertical_collision && requested_y < 0.0 || local_player_supported(world, pose.position);

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
    if flying.is_some() {
        if vertical_collision {
            pose.delta_movement.y = 0.0;
        } else {
            pose.delta_movement.y *= LOCAL_INPUT_FLY_VERTICAL_DAMPING.powf(step_ticks);
        }
    } else if on_ground || vertical_collision {
        pose.delta_movement.y = 0.0;
    } else {
        pose.delta_movement.y = (pose.delta_movement.y - LOCAL_GRAVITY_PER_TICK * step_ticks)
            * LOCAL_VERTICAL_FRICTION.powf(step_ticks);
    }
    pose.on_ground = on_ground;
    pose.horizontal_collision = horizontal_collision;
    pose
}

fn clip_local_player_movement(
    world: &WorldStore,
    position: ProtocolVec3d,
    requested_x: f64,
    requested_y: f64,
    requested_z: f64,
    on_ground: bool,
) -> ProtocolVec3d {
    let clipped = clip_local_player_movement_without_step(
        world,
        position,
        requested_x,
        requested_y,
        requested_z,
    );
    if !on_ground
        || requested_y.abs() > COLLISION_EPSILON
        || ((clipped.x - requested_x).abs() <= COLLISION_EPSILON
            && (clipped.z - requested_z).abs() <= COLLISION_EPSILON)
    {
        return clipped;
    }

    let Some(stepped) =
        clip_local_player_step_up_movement(world, position, requested_x, requested_z)
    else {
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
    position: ProtocolVec3d,
    requested_x: f64,
    requested_y: f64,
    requested_z: f64,
) -> ProtocolVec3d {
    let mut bounds = LocalPlayerBounds::at(position);
    let clipped_y = clip_axis_delta(world, bounds, Axis::Y, requested_y);
    bounds = bounds.moved(0.0, clipped_y, 0.0);
    let clipped_x = clip_axis_delta(world, bounds, Axis::X, requested_x);
    bounds = bounds.moved(clipped_x, 0.0, 0.0);
    let clipped_z = clip_axis_delta(world, bounds, Axis::Z, requested_z);

    ProtocolVec3d {
        x: clipped_x,
        y: clipped_y,
        z: clipped_z,
    }
}

fn clip_local_player_step_up_movement(
    world: &WorldStore,
    position: ProtocolVec3d,
    requested_x: f64,
    requested_z: f64,
) -> Option<ProtocolVec3d> {
    let mut bounds = LocalPlayerBounds::at(position);
    let clipped_up = clip_axis_delta(world, bounds, Axis::Y, LOCAL_PLAYER_STEP_HEIGHT);
    if clipped_up <= COLLISION_EPSILON {
        return None;
    }
    bounds = bounds.moved(0.0, clipped_up, 0.0);
    let clipped_x = clip_axis_delta(world, bounds, Axis::X, requested_x);
    bounds = bounds.moved(clipped_x, 0.0, 0.0);
    let clipped_z = clip_axis_delta(world, bounds, Axis::Z, requested_z);
    bounds = bounds.moved(0.0, 0.0, clipped_z);
    let clipped_down = clip_axis_delta(world, bounds, Axis::Y, -clipped_up);

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
        && pose.on_ground
}

fn local_player_horizontal_speed(world: &WorldStore, input: LocalPlayerInputState) -> f64 {
    if let Some(abilities) = local_player_flying_abilities(world) {
        return local_player_flying_horizontal_speed(abilities, input);
    }

    let movement_speed_scale = local_player_movement_speed_attribute_value(world)
        / LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE;
    let mut speed = LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND * movement_speed_scale;
    if input.sprint {
        speed *= LOCAL_INPUT_SPRINT_SPEED_MULTIPLIER;
    }
    if input.sneak && !local_player_is_flying(world) {
        speed *= local_player_attribute_value(world, VANILLA_ATTRIBUTE_SNEAKING_SPEED_ID)
            .unwrap_or(LOCAL_INPUT_SNEAKING_SPEED_MULTIPLIER)
            .clamp(0.0, 1.0);
    }
    speed
}

fn local_player_movement_speed_attribute_value(world: &WorldStore) -> f64 {
    let mut speed = local_player_attribute_value(world, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap_or(LOCAL_INPUT_DEFAULT_MOVEMENT_SPEED_ATTRIBUTE);
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

fn local_player_jump_velocity(world: &WorldStore) -> f64 {
    let mut velocity = local_player_attribute_value(world, VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID)
        .unwrap_or(LOCAL_INPUT_DEFAULT_JUMP_STRENGTH_ATTRIBUTE)
        .max(0.0);
    if let Some(amplifier) = local_player_effect_amplifier(world, VANILLA_MOB_EFFECT_JUMP_BOOST_ID)
    {
        velocity += amplified_effect_amount(amplifier, JUMP_BOOST_VELOCITY_PER_LEVEL);
    }
    velocity
}

fn local_player_flying_horizontal_speed(
    abilities: LocalPlayerAbilitiesState,
    input: LocalPlayerInputState,
) -> f64 {
    let flying_speed_scale =
        f64::from(abilities.flying_speed).max(0.0) / LOCAL_INPUT_DEFAULT_FLYING_SPEED_ATTRIBUTE;
    let mut speed = LOCAL_INPUT_DEFAULT_FLY_SPEED_BLOCKS_PER_SECOND * flying_speed_scale;
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
    position: ProtocolVec3d,
    requested_x: f64,
    requested_z: f64,
) -> (f64, f64) {
    let mut backed_x = requested_x;
    let mut backed_z = requested_z;
    let step_x = backed_x.signum() * EDGE_BACKOFF_STEP;
    let step_z = backed_z.signum() * EDGE_BACKOFF_STEP;

    while backed_x != 0.0
        && local_player_can_fall_at_least(world, position, backed_x, 0.0, LOCAL_PLAYER_STEP_HEIGHT)
    {
        if backed_x.abs() <= EDGE_BACKOFF_STEP {
            backed_x = 0.0;
            break;
        }
        backed_x -= step_x;
    }

    while backed_z != 0.0
        && local_player_can_fall_at_least(world, position, 0.0, backed_z, LOCAL_PLAYER_STEP_HEIGHT)
    {
        if backed_z.abs() <= EDGE_BACKOFF_STEP {
            backed_z = 0.0;
            break;
        }
        backed_z -= step_z;
    }

    while backed_x != 0.0
        && backed_z != 0.0
        && local_player_can_fall_at_least(
            world,
            position,
            backed_x,
            backed_z,
            LOCAL_PLAYER_STEP_HEIGHT,
        )
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
    position: ProtocolVec3d,
    delta_x: f64,
    delta_z: f64,
    min_height: f64,
) -> bool {
    !local_player_collides(
        world,
        LocalPlayerBounds::at(position)
            .moved(delta_x, 0.0, delta_z)
            .edge_support_probe(min_height),
    )
}

fn clip_axis_delta(
    world: &WorldStore,
    bounds: LocalPlayerBounds,
    axis: Axis,
    requested: f64,
) -> f64 {
    if requested.abs() <= COLLISION_EPSILON {
        return 0.0;
    }
    if !local_player_collides(world, bounds.swept_axis(axis, requested)) {
        return requested;
    }

    let mut low = 0.0;
    let mut high = requested;
    for _ in 0..COLLISION_CLIP_STEPS {
        let midpoint = (low + high) * 0.5;
        if local_player_collides(world, bounds.swept_axis(axis, midpoint)) {
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

fn local_player_supported(world: &WorldStore, position: ProtocolVec3d) -> bool {
    local_player_collides(
        world,
        LocalPlayerBounds::at(position).moved(0.0, -SUPPORT_EPSILON, 0.0),
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
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
        AttributeSnapshot as ProtocolAttributeSnapshot, MobEffectFlags as ProtocolMobEffectFlags,
        UpdateAttributes as ProtocolUpdateAttributes, UpdateMobEffect as ProtocolUpdateMobEffect,
    };
    use uuid::Uuid;

    use crate::{
        entities::VANILLA_ENTITY_TYPE_PLAYER_ID, ChunkColumn, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    const AIR_BLOCK_STATE_ID: i32 = 0;
    const GRASS_BLOCK_STATE_ID: i32 = 9;
    const OAK_TOP_SLAB_BLOCK_STATE_ID: i32 = 13331;
    const OAK_BOTTOM_SLAB_BLOCK_STATE_ID: i32 = 13333;
    const OAK_TOP_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID: i32 = 3928;
    const OAK_BOTTOM_STRAIGHT_NORTH_STAIR_BLOCK_STATE_ID: i32 = 3918;
    const OAK_BOTTOM_STRAIGHT_SOUTH_STAIR_BLOCK_STATE_ID: i32 = 3938;
    const OAK_LEAVES_BLOCK_STATE_ID: i32 = 255;
    const SNOW_5_LAYERS_BLOCK_STATE_ID: i32 = 6923;
    const SNOW_6_LAYERS_BLOCK_STATE_ID: i32 = 6924;
    const OAK_CLOSED_NORTH_DOOR_BLOCK_STATE_ID: i32 = 5666;
    const OAK_TOP_CLOSED_NORTH_TRAPDOOR_BLOCK_STATE_ID: i32 = 7121;
    const STONE_PRESSURE_PLATE_BLOCK_STATE_ID: i32 = 6796;
    const OAK_NORTH_FENCE_BLOCK_STATE_ID: i32 = 6988;
    const OAK_CLOSED_NORTH_FENCE_GATE_BLOCK_STATE_ID: i32 = 8653;
    const OAK_OPEN_NORTH_FENCE_GATE_BLOCK_STATE_ID: i32 = 8651;
    const GLASS_NORTH_PANE_BLOCK_STATE_ID: i32 = 8323;
    const WHITE_CARPET_BLOCK_STATE_ID: i32 = 12896;
    const COBBLESTONE_NORTH_EAST_WALL_BLOCK_STATE_ID: i32 = 10236;
    const IRON_CHAIN_Y_AXIS_BLOCK_STATE_ID: i32 = 8249;
    const LADDER_SOUTH_BLOCK_STATE_ID: i32 = 5722;
    const END_ROD_NORTH_BLOCK_STATE_ID: i32 = 14636;
    const LANTERN_STANDING_BLOCK_STATE_ID: i32 = 20840;
    const CAMPFIRE_NORTH_LIT_BLOCK_STATE_ID: i32 = 20880;
    const CHEST_SINGLE_NORTH_BLOCK_STATE_ID: i32 = 3988;
    const CHEST_LEFT_NORTH_BLOCK_STATE_ID: i32 = 3990;
    const TRAPPED_CHEST_RIGHT_NORTH_BLOCK_STATE_ID: i32 = 11212;
    const ENDER_CHEST_NORTH_BLOCK_STATE_ID: i32 = 9576;
    const WHITE_BED_NORTH_FOOT_BLOCK_STATE_ID: i32 = 1934;
    const WATER_CAULDRON_LEVEL_3_BLOCK_STATE_ID: i32 = 9463;
    const HOPPER_NORTH_ENABLED_BLOCK_STATE_ID: i32 = 11314;
    const ENCHANTING_TABLE_BLOCK_STATE_ID: i32 = 9451;
    const STONECUTTER_NORTH_BLOCK_STATE_ID: i32 = 20801;
    const ANVIL_NORTH_BLOCK_STATE_ID: i32 = 11195;
    const COMPOSTER_LEVEL_7_BLOCK_STATE_ID: i32 = 21750;
    const COPPER_GRATE_BLOCK_STATE_ID: i32 = 27048;
    const WAXED_COPPER_GRATE_BLOCK_STATE_ID: i32 = 27056;
    const LIGHTNING_ROD_UP_UNPOWERED_BLOCK_STATE_ID: i32 = 27562;

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
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
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

        let expected_step = LOCAL_INPUT_DEFAULT_FLY_SPEED_BLOCKS_PER_SECOND
            * (0.1 / LOCAL_INPUT_DEFAULT_FLYING_SPEED_ATTRIBUTE)
            * LOCAL_INPUT_FLY_SPRINT_SPEED_MULTIPLIER
            * LOCAL_PHYSICS_TICK_SECONDS;
        assert_f64_near(pose.position.z, 0.5 + expected_step, 0.000001);
        assert_f64_near(pose.delta_movement.z, expected_step, 0.000001);
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

    fn mob_effect(entity_id: i32, effect_id: i32, amplifier: i32) -> ProtocolUpdateMobEffect {
        ProtocolUpdateMobEffect {
            entity_id,
            effect_id,
            amplifier,
            duration_ticks: 200,
            flags: ProtocolMobEffectFlags::default(),
        }
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
