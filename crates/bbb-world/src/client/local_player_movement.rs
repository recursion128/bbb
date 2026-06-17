use bbb_protocol::packets::Vec3d as ProtocolVec3d;

use super::local_player::{LocalPlayerInputState, LocalPlayerPoseState};
use crate::{BlockPos, BlockProbe, TerrainMaterialClass, WorldStore};

pub(super) const LOCAL_INPUT_MOUSE_SENSITIVITY_DEGREES: f32 = 0.12;
pub(super) const LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND: f64 = 4.317;
pub(super) const LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND: f64 = 5.612;

const LOCAL_PHYSICS_TICK_SECONDS: f64 = 0.05;
const LOCAL_GRAVITY_PER_TICK: f64 = 0.08;
const LOCAL_JUMP_VELOCITY_PER_TICK: f64 = 0.42;
const LOCAL_VERTICAL_FRICTION: f64 = 0.98;
const LOCAL_PLAYER_HALF_WIDTH: f64 = 0.3;
const LOCAL_PLAYER_HEIGHT: f64 = 1.8;
const LOCAL_PLAYER_STEP_HEIGHT: f64 = 0.6;
const COLLISION_EPSILON: f64 = 1.0e-7;
const SUPPORT_EPSILON: f64 = 1.0e-3;
const COLLISION_CLIP_STEPS: usize = 12;

pub(super) fn integrate_local_player_input_pose(
    world: &WorldStore,
    mut pose: LocalPlayerPoseState,
    input: LocalPlayerInputState,
    dt_seconds: f64,
) -> LocalPlayerPoseState {
    if input.focused {
        pose.y_rot = wrap_degrees_f32(
            pose.y_rot + input.mouse_delta_x as f32 * LOCAL_INPUT_MOUSE_SENSITIVITY_DEGREES,
        );
        pose.x_rot = (pose.x_rot
            + input.mouse_delta_y as f32 * LOCAL_INPUT_MOUSE_SENSITIVITY_DEGREES)
            .clamp(-90.0, 90.0);
    }

    let mut remaining_seconds = dt_seconds.max(0.0);
    while remaining_seconds > COLLISION_EPSILON {
        let step_seconds = remaining_seconds.min(LOCAL_PHYSICS_TICK_SECONDS);
        pose = advance_local_player_physics_step(world, pose, input, step_seconds);
        remaining_seconds -= step_seconds;
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
    let speed = if input.sprint {
        LOCAL_INPUT_SPRINT_SPEED_BLOCKS_PER_SECOND
    } else {
        LOCAL_INPUT_WALK_SPEED_BLOCKS_PER_SECOND
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

    if input.focused && input.jump && pose.on_ground {
        pose.delta_movement.y = pose.delta_movement.y.max(LOCAL_JUMP_VELOCITY_PER_TICK);
    }

    let step_ticks = step_seconds / LOCAL_PHYSICS_TICK_SECONDS;
    let requested_x = move_x * speed * step_seconds;
    let requested_y = pose.delta_movement.y * step_ticks;
    let requested_z = move_z * speed * step_seconds;
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
    if on_ground || vertical_collision {
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

fn local_player_collides(world: &WorldStore, bounds: LocalPlayerBounds) -> bool {
    let min_x = block_floor(bounds.min_x + COLLISION_EPSILON);
    let max_x = block_floor(bounds.max_x - COLLISION_EPSILON);
    let min_y = block_floor(bounds.min_y + COLLISION_EPSILON);
    let max_y = block_floor(bounds.max_y - COLLISION_EPSILON);
    let min_z = block_floor(bounds.min_z + COLLISION_EPSILON);
    let max_z = block_floor(bounds.max_z - COLLISION_EPSILON);

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let Some(block) = world.probe_block(BlockPos { x, y, z }) else {
                    continue;
                };
                if block_collides_with_local_player_bounds(&block, BlockPos { x, y, z }, bounds) {
                    return true;
                }
            }
        }
    }
    false
}

fn block_collides_with_local_player_bounds(
    block: &BlockProbe,
    pos: BlockPos,
    bounds: LocalPlayerBounds,
) -> bool {
    if let Some(shape) = block_collision_shape(block) {
        return bounds_intersects_block_shape(bounds, pos, shape);
    }
    false
}

fn block_collision_shape(block: &BlockProbe) -> Option<BlockCollisionShape> {
    if is_slab_block(block) {
        return match block.block_properties.get("type").map(String::as_str) {
            Some("bottom") => Some(BlockCollisionShape {
                min_y: 0.0,
                max_y: 0.5,
            }),
            Some("top") => Some(BlockCollisionShape {
                min_y: 0.5,
                max_y: 1.0,
            }),
            Some("double") => Some(BlockCollisionShape::FULL),
            _ => None,
        };
    }
    match block.material {
        TerrainMaterialClass::Opaque | TerrainMaterialClass::Translucent => {
            Some(BlockCollisionShape::FULL)
        }
        TerrainMaterialClass::Invisible => {
            if matches!(block.block_name.as_deref(), Some("minecraft:barrier")) {
                Some(BlockCollisionShape::FULL)
            } else {
                None
            }
        }
        TerrainMaterialClass::Empty
        | TerrainMaterialClass::Cutout
        | TerrainMaterialClass::Fluid => None,
    }
}

fn is_slab_block(block: &BlockProbe) -> bool {
    block
        .block_name
        .as_deref()
        .is_some_and(|name| name.ends_with("_slab"))
}

fn bounds_intersects_block_shape(
    bounds: LocalPlayerBounds,
    pos: BlockPos,
    shape: BlockCollisionShape,
) -> bool {
    let min_x = f64::from(pos.x);
    let max_x = min_x + 1.0;
    let min_y = f64::from(pos.y) + shape.min_y;
    let max_y = f64::from(pos.y) + shape.max_y;
    let min_z = f64::from(pos.z);
    let max_z = min_z + 1.0;

    bounds.max_x > min_x + COLLISION_EPSILON
        && bounds.min_x < max_x - COLLISION_EPSILON
        && bounds.max_y > min_y + COLLISION_EPSILON
        && bounds.min_y < max_y - COLLISION_EPSILON
        && bounds.max_z > min_z + COLLISION_EPSILON
        && bounds.min_z < max_z - COLLISION_EPSILON
}

fn block_floor(value: f64) -> i32 {
    value.floor() as i32
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

#[derive(Debug, Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy)]
struct LocalPlayerBounds {
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
}

#[derive(Debug, Clone, Copy)]
struct BlockCollisionShape {
    min_y: f64,
    max_y: f64,
}

impl BlockCollisionShape {
    const FULL: Self = Self {
        min_y: 0.0,
        max_y: 1.0,
    };
}

impl LocalPlayerBounds {
    fn at(position: ProtocolVec3d) -> Self {
        Self {
            min_x: position.x - LOCAL_PLAYER_HALF_WIDTH,
            min_y: position.y,
            min_z: position.z - LOCAL_PLAYER_HALF_WIDTH,
            max_x: position.x + LOCAL_PLAYER_HALF_WIDTH,
            max_y: position.y + LOCAL_PLAYER_HEIGHT,
            max_z: position.z + LOCAL_PLAYER_HALF_WIDTH,
        }
    }

    fn moved(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            min_x: self.min_x + x,
            min_y: self.min_y + y,
            min_z: self.min_z + z,
            max_x: self.max_x + x,
            max_y: self.max_y + y,
            max_z: self.max_z + z,
        }
    }

    fn moved_axis(self, axis: Axis, amount: f64) -> Self {
        match axis {
            Axis::X => self.moved(amount, 0.0, 0.0),
            Axis::Y => self.moved(0.0, amount, 0.0),
            Axis::Z => self.moved(0.0, 0.0, amount),
        }
    }

    fn swept_axis(self, axis: Axis, amount: f64) -> Self {
        let moved = self.moved_axis(axis, amount);
        match axis {
            Axis::X => Self {
                min_x: self.min_x.min(moved.min_x),
                max_x: self.max_x.max(moved.max_x),
                ..self
            },
            Axis::Y => Self {
                min_y: self.min_y.min(moved.min_y),
                max_y: self.max_y.max(moved.max_y),
                ..self
            },
            Axis::Z => Self {
                min_z: self.min_z.min(moved.min_z),
                max_z: self.max_z.max(moved.max_z),
                ..self
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkColumn, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };

    const AIR_BLOCK_STATE_ID: i32 = 0;
    const GRASS_BLOCK_STATE_ID: i32 = 9;
    const OAK_TOP_SLAB_BLOCK_STATE_ID: i32 = 13331;
    const OAK_BOTTOM_SLAB_BLOCK_STATE_ID: i32 = 13333;

    #[test]
    fn local_player_input_stops_at_full_block_wall_and_reports_collision() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 2, GRASS_BLOCK_STATE_ID);
        set_test_block(&mut world, 0, 2, 2, GRASS_BLOCK_STATE_ID);
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
                1.0,
            )
            .unwrap();

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
                0.35,
            )
            .unwrap();

        assert_f64_near(pose.position.y, 1.5, 0.0005);
        assert!(pose.position.z > 1.7, "position was {:?}", pose.position);
        assert!(pose.on_ground);
        assert!(!pose.horizontal_collision);
    }

    #[test]
    fn local_player_does_not_step_through_top_slab() {
        let mut world = flat_collision_world();
        set_test_block(&mut world, 0, 1, 2, OAK_TOP_SLAB_BLOCK_STATE_ID);
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
                1.0,
            )
            .unwrap();

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

    fn flat_collision_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(empty_test_chunk());
        for x in 0..3 {
            for z in 0..3 {
                set_test_block(&mut world, x, 0, z, GRASS_BLOCK_STATE_ID);
            }
        }
        world
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
