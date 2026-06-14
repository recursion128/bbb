use bbb_control::PlayerPose;
use bbb_protocol::packets::{
    BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos,
    Direction as ProtocolDirection, Vec3d as ProtocolVec3d,
};
use bbb_renderer::{CameraPose, SelectionOutline};
use bbb_world::{BlockPos, EntityPickTargetState, WorldStore};

use crate::block_outline::{
    selection_outline_for_block, selection_outline_for_probe, BlockOutlineTarget,
};

const DEFAULT_BLOCK_INTERACTION_RANGE: f64 = 4.5;
const DEFAULT_ENTITY_INTERACTION_RANGE: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CrosshairBlockHit {
    pub(crate) pos: BlockPos,
    pub(crate) face: ProtocolDirection,
    pub(crate) cursor: [f32; 3],
    pub(crate) inside: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CrosshairEntityHit {
    pub(crate) entity_id: i32,
    pub(crate) location: ProtocolVec3d,
    pub(crate) relative_location: ProtocolVec3d,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CrosshairTarget {
    Block(CrosshairBlockHit),
    Entity(CrosshairEntityHit),
}

pub(crate) fn selection_outline_from_crosshair(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<SelectionOutline> {
    let hit = crosshair_block_hit_from_world(world, pose)?;
    match world.probe_block(hit.pos) {
        Some(probe) => selection_outline_for_probe(&probe),
        None => Some(selection_outline_for_block(hit.pos)),
    }
}

pub(crate) fn crosshair_block_hit_from_world(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit(pose?, DEFAULT_BLOCK_INTERACTION_RANGE, |pos| {
        world
            .probe_block(pos)
            .map(|probe| BlockOutlineTarget::from_probe(&probe))
    })
}

pub(crate) fn crosshair_target_from_world(
    world: &WorldStore,
    pose: Option<PlayerPose>,
) -> Option<CrosshairTarget> {
    let pose = pose?;
    let eye = eye_position_from_player_pose(pose);
    let block_hit = crosshair_block_hit_from_world(world, Some(pose));
    let block_distance_sq = block_hit
        .map(|hit| distance_sq(eye, block_hit_location(hit)))
        .unwrap_or(f64::INFINITY);
    let entity_max_distance = block_distance_sq
        .sqrt()
        .min(DEFAULT_BLOCK_INTERACTION_RANGE);
    let entity_hit = crosshair_entity_hit_from_world(world, Some(pose), entity_max_distance);

    choose_crosshair_target(eye, block_hit, entity_hit, DEFAULT_ENTITY_INTERACTION_RANGE)
}

fn choose_crosshair_target(
    eye: [f64; 3],
    block_hit: Option<CrosshairBlockHit>,
    entity_hit: Option<RaycastEntityHit>,
    entity_interaction_range: f64,
) -> Option<CrosshairTarget> {
    let block_distance_sq = block_hit
        .map(|hit| distance_sq(eye, block_hit_location(hit)))
        .unwrap_or(f64::INFINITY);
    match (block_hit, entity_hit) {
        (Some(_block), Some(entity)) if entity.distance_sq < block_distance_sq => {
            entity_in_interaction_range(entity, entity_interaction_range)
                .then_some(CrosshairTarget::Entity(entity.hit))
        }
        (Some(block), _) => Some(CrosshairTarget::Block(block)),
        (None, Some(entity)) => entity_in_interaction_range(entity, entity_interaction_range)
            .then_some(CrosshairTarget::Entity(entity.hit)),
        (None, None) => None,
    }
}

fn entity_in_interaction_range(entity: RaycastEntityHit, entity_interaction_range: f64) -> bool {
    entity.distance_sq < entity_interaction_range * entity_interaction_range
}

fn crosshair_entity_hit_from_world(
    world: &WorldStore,
    pose: Option<PlayerPose>,
    max_distance: f64,
) -> Option<RaycastEntityHit> {
    let local_player_id = world.local_player_id();
    let targets = world
        .entity_pick_targets()
        .into_iter()
        .filter_map(|target| {
            if local_player_id.is_some_and(|id| id == target.entity_id) {
                None
            } else {
                Some(EntityRaycastTarget { target })
            }
        });
    raycast_crosshair_entity_hit(pose?, max_distance, targets)
}

fn raycast_crosshair_block<F>(
    pose: PlayerPose,
    max_distance: f64,
    mut material_at: F,
) -> Option<BlockPos>
where
    F: FnMut(BlockPos) -> Option<bbb_world::TerrainMaterialClass>,
{
    raycast_crosshair_block_hit(pose, max_distance, |pos| {
        material_at(pos).map(BlockOutlineTarget::full_block)
    })
    .map(|hit| hit.pos)
}

fn raycast_crosshair_block_hit<F>(
    pose: PlayerPose,
    max_distance: f64,
    mut target_at: F,
) -> Option<CrosshairBlockHit>
where
    F: FnMut(BlockPos) -> Option<BlockOutlineTarget>,
{
    if max_distance <= 0.0 {
        return None;
    }

    let eye = eye_position_from_player_pose(pose);
    let direction = look_direction_from_player_pose(pose);
    if direction == [0.0, 0.0, 0.0] {
        return None;
    }

    let mut pos = block_pos_containing(eye);
    if let Some(hit) =
        target_at(pos).and_then(|target| target.clip(eye, direction, max_distance, pos))
    {
        return Some(CrosshairBlockHit {
            pos,
            face: hit.face,
            cursor: block_hit_cursor(eye, direction, hit.distance, pos),
            inside: hit.inside,
        });
    }

    let mut cursor = RayGridCursor::new(eye, direction);
    while let Some(step) = cursor.next_step(max_distance) {
        pos = offset_block_pos_axis(pos, step.axis, step.delta);
        if let Some(hit) =
            target_at(pos).and_then(|target| target.clip(eye, direction, max_distance, pos))
        {
            return Some(CrosshairBlockHit {
                pos,
                face: hit.face,
                cursor: block_hit_cursor(eye, direction, hit.distance, pos),
                inside: hit.inside,
            });
        }
    }

    None
}

#[derive(Debug, Clone, Copy)]
struct EntityRaycastTarget {
    target: EntityPickTargetState,
}

#[derive(Debug, Clone, Copy)]
struct RaycastEntityHit {
    hit: CrosshairEntityHit,
    distance_sq: f64,
}

fn raycast_crosshair_entity_hit<I>(
    pose: PlayerPose,
    max_distance: f64,
    targets: I,
) -> Option<RaycastEntityHit>
where
    I: IntoIterator<Item = EntityRaycastTarget>,
{
    if max_distance <= 0.0 {
        return None;
    }

    let eye = eye_position_from_player_pose(pose);
    let direction = look_direction_from_player_pose(pose);
    if direction == [0.0, 0.0, 0.0] {
        return None;
    }

    let mut nearest: Option<RaycastEntityHit> = None;
    for target in targets {
        let Some(distance) = raycast_entity_target_distance(eye, direction, max_distance, target)
        else {
            continue;
        };
        let distance_sq = distance * distance;
        if nearest.is_some_and(|hit| hit.distance_sq <= distance_sq) {
            continue;
        }
        let location = ProtocolVec3d {
            x: eye[0] + direction[0] * distance,
            y: eye[1] + direction[1] * distance,
            z: eye[2] + direction[2] * distance,
        };
        nearest = Some(RaycastEntityHit {
            hit: CrosshairEntityHit {
                entity_id: target.target.entity_id,
                location,
                relative_location: ProtocolVec3d {
                    x: location.x - target.target.position.x,
                    y: location.y - target.target.position.y,
                    z: location.z - target.target.position.z,
                },
            },
            distance_sq,
        });
    }
    nearest
}

fn raycast_entity_target_distance(
    eye: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
    target: EntityRaycastTarget,
) -> Option<f64> {
    let inflate = f64::from(target.target.bounds.pick_radius);
    let min = [
        target.target.position.x + f64::from(target.target.bounds.min[0]) - inflate,
        target.target.position.y + f64::from(target.target.bounds.min[1]) - inflate,
        target.target.position.z + f64::from(target.target.bounds.min[2]) - inflate,
    ];
    let max = [
        target.target.position.x + f64::from(target.target.bounds.max[0]) + inflate,
        target.target.position.y + f64::from(target.target.bounds.max[1]) + inflate,
        target.target.position.z + f64::from(target.target.bounds.max[2]) + inflate,
    ];
    ray_box_distance(eye, direction, max_distance, min, max)
}

fn ray_box_distance(
    eye: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
    min: [f64; 3],
    max: [f64; 3],
) -> Option<f64> {
    if contains_point(min, max, eye) {
        return Some(0.0);
    }

    let mut enter = 0.0;
    let mut exit = max_distance;
    for axis in 0..3 {
        let component = direction[axis];
        if component.abs() <= f64::EPSILON {
            if eye[axis] < min[axis] || eye[axis] > max[axis] {
                return None;
            }
            continue;
        }

        let t0 = (min[axis] - eye[axis]) / component;
        let t1 = (max[axis] - eye[axis]) / component;
        let near = t0.min(t1);
        let far = t0.max(t1);
        if near > enter {
            enter = near;
        }
        if far < exit {
            exit = far;
        }
        if enter > exit {
            return None;
        }
    }

    if enter <= max_distance && exit >= 0.0 {
        Some(enter.max(0.0))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy)]
struct RayGridCursor {
    x: AxisStep,
    y: AxisStep,
    z: AxisStep,
}

impl RayGridCursor {
    fn new(origin: [f64; 3], direction: [f64; 3]) -> Self {
        Self {
            x: AxisStep::new(origin[0], direction[0]),
            y: AxisStep::new(origin[1], direction[1]),
            z: AxisStep::new(origin[2], direction[2]),
        }
    }

    fn next_step(&mut self, max_distance: f64) -> Option<GridStep> {
        let axis = if self.x.next_distance < self.y.next_distance {
            if self.x.next_distance < self.z.next_distance {
                0
            } else {
                2
            }
        } else if self.y.next_distance < self.z.next_distance {
            1
        } else {
            2
        };

        let step = match axis {
            0 => self.x.advance(axis),
            1 => self.y.advance(axis),
            _ => self.z.advance(axis),
        };
        if step.distance <= max_distance {
            Some(step)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AxisStep {
    delta: i32,
    next_distance: f64,
    distance_delta: f64,
}

impl AxisStep {
    fn new(origin: f64, direction: f64) -> Self {
        let delta = if direction > 0.0 {
            1
        } else if direction < 0.0 {
            -1
        } else {
            0
        };
        if delta == 0 {
            return Self {
                delta,
                next_distance: f64::INFINITY,
                distance_delta: f64::INFINITY,
            };
        }

        let boundary = if delta > 0 {
            origin.floor() + 1.0
        } else {
            origin.floor()
        };
        let next_distance = (boundary - origin) / direction;
        Self {
            delta,
            next_distance: next_distance.max(0.0),
            distance_delta: 1.0 / direction.abs(),
        }
    }

    fn advance(&mut self, axis: u8) -> GridStep {
        let step = GridStep {
            axis,
            delta: self.delta,
            distance: self.next_distance,
        };
        self.next_distance += self.distance_delta;
        step
    }
}

#[derive(Debug, Clone, Copy)]
struct GridStep {
    axis: u8,
    delta: i32,
    distance: f64,
}

fn offset_block_pos_axis(pos: BlockPos, axis: u8, delta: i32) -> BlockPos {
    match axis {
        0 => BlockPos {
            x: pos.x + delta,
            ..pos
        },
        1 => BlockPos {
            y: pos.y + delta,
            ..pos
        },
        _ => BlockPos {
            z: pos.z + delta,
            ..pos
        },
    }
}

fn block_pos_containing(point: [f64; 3]) -> BlockPos {
    BlockPos {
        x: point[0].floor() as i32,
        y: point[1].floor() as i32,
        z: point[2].floor() as i32,
    }
}

fn block_hit_cursor(eye: [f64; 3], direction: [f64; 3], distance: f64, pos: BlockPos) -> [f32; 3] {
    [
        ((eye[0] + direction[0] * distance) - f64::from(pos.x)).clamp(0.0, 1.0) as f32,
        ((eye[1] + direction[1] * distance) - f64::from(pos.y)).clamp(0.0, 1.0) as f32,
        ((eye[2] + direction[2] * distance) - f64::from(pos.z)).clamp(0.0, 1.0) as f32,
    ]
}

fn block_hit_location(hit: CrosshairBlockHit) -> [f64; 3] {
    [
        f64::from(hit.pos.x) + f64::from(hit.cursor[0]),
        f64::from(hit.pos.y) + f64::from(hit.cursor[1]),
        f64::from(hit.pos.z) + f64::from(hit.cursor[2]),
    ]
}

fn distance_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn contains_point(min: [f64; 3], max: [f64; 3], point: [f64; 3]) -> bool {
    point[0] >= min[0]
        && point[0] <= max[0]
        && point[1] >= min[1]
        && point[1] <= max[1]
        && point[2] >= min[2]
        && point[2] <= max[2]
}

pub(crate) fn protocol_block_pos_from_world(pos: BlockPos) -> ProtocolBlockPos {
    ProtocolBlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

pub(crate) fn protocol_block_hit_result_from_crosshair_hit(
    hit: CrosshairBlockHit,
) -> ProtocolBlockHitResult {
    ProtocolBlockHitResult {
        pos: protocol_block_pos_from_world(hit.pos),
        direction: hit.face,
        cursor_x: hit.cursor[0],
        cursor_y: hit.cursor[1],
        cursor_z: hit.cursor[2],
        inside: hit.inside,
        world_border_hit: false,
    }
}

fn eye_position_from_player_pose(pose: PlayerPose) -> [f64; 3] {
    [
        pose.position.x,
        pose.position.y + f64::from(CameraPose::STANDING_EYE_HEIGHT),
        pose.position.z,
    ]
}

fn look_direction_from_player_pose(pose: PlayerPose) -> [f64; 3] {
    let yaw = f64::from(pose.y_rot).to_radians();
    let pitch = f64::from(pose.x_rot).to_radians();
    let cos_pitch = pitch.cos();
    let x = -yaw.sin() * cos_pitch;
    let y = -pitch.sin();
    let z = yaw.cos() * cos_pitch;
    let len = (x * x + y * y + z * z).sqrt();
    if len <= f64::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [x / len, y / len, z / len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_control::NetVec3;
    use bbb_protocol::packets::{
        AddEntity, EntityDataValue, EntityDataValueKind, SetEntityData, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    const VANILLA_ENTITY_TYPE_ARMOR_STAND_ID: i32 = 5;
    const VANILLA_ENTITY_TYPE_INTERACTION_ID: i32 = 69;
    const VANILLA_ENTITY_TYPE_ITEM_FRAME_ID: i32 = 73;
    const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
    const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;

    #[test]
    fn crosshair_raycast_hits_first_selectable_block() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
    }

    #[test]
    fn crosshair_raycast_reports_hit_face() {
        let pose = player_pose(0.0, 0.0, 0.0);

        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(BlockOutlineTarget::full_block(
                    bbb_world::TerrainMaterialClass::Opaque,
                ))
            } else {
                None
            }
        });

        assert_eq!(
            hit,
            Some(CrosshairBlockHit {
                pos: BlockPos { x: 0, y: 1, z: 3 },
                face: ProtocolDirection::North,
                cursor: [0.0, 0.62, 0.0],
                inside: false,
            })
        );
    }

    #[test]
    fn crosshair_raycast_clips_partial_outline_shape() {
        let pose = player_pose(0.0, 0.0, 0.0);

        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(BlockOutlineTarget::from_box(
                    bbb_world::TerrainMaterialClass::Opaque,
                    [0.0, 0.5, 0.0],
                    [1.0, 1.0, 1.0],
                ))
            } else {
                None
            }
        });

        assert_eq!(
            hit,
            Some(CrosshairBlockHit {
                pos: BlockPos { x: 0, y: 1, z: 3 },
                face: ProtocolDirection::North,
                cursor: [0.0, 0.62, 0.0],
                inside: false,
            })
        );
    }

    #[test]
    fn crosshair_raycast_skips_partial_outline_shape() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block_hit(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(BlockOutlineTarget::from_box(
                    bbb_world::TerrainMaterialClass::Opaque,
                    [0.0, 0.0, 0.0],
                    [1.0, 0.5, 1.0],
                ))
            } else if pos == (BlockPos { x: 0, y: 1, z: 4 }) {
                Some(BlockOutlineTarget::full_block(
                    bbb_world::TerrainMaterialClass::Opaque,
                ))
            } else {
                None
            }
        });

        assert_eq!(
            hit,
            Some(CrosshairBlockHit {
                pos: BlockPos { x: 0, y: 1, z: 4 },
                face: ProtocolDirection::North,
                cursor: [0.0, 0.62, 0.0],
                inside: false,
            })
        );
    }

    #[test]
    fn crosshair_raycast_visits_blocks_by_grid_boundary() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 1 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 1 }));
    }

    #[test]
    fn crosshair_raycast_ignores_fluid_blocks() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 2 }) {
                Some(bbb_world::TerrainMaterialClass::Fluid)
            } else if pos == (BlockPos { x: 0, y: 1, z: 3 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(hit, Some(BlockPos { x: 0, y: 1, z: 3 }));
    }

    #[test]
    fn crosshair_raycast_uses_vanilla_default_block_interaction_range() {
        let pose = player_pose(0.0, 0.0, 0.0);
        let hit = raycast_crosshair_block(pose, DEFAULT_BLOCK_INTERACTION_RANGE, |pos| {
            if pos == (BlockPos { x: 0, y: 1, z: 5 }) {
                Some(bbb_world::TerrainMaterialClass::Opaque)
            } else {
                Some(bbb_world::TerrainMaterialClass::Empty)
            }
        });

        assert_eq!(DEFAULT_BLOCK_INTERACTION_RANGE, 4.5);
        assert_eq!(hit, None);
    }

    #[test]
    fn crosshair_target_hits_pickable_entity_from_world_state() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));
        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected entity hit");
        };
        assert_eq!(hit.entity_id, 10);
        assert_vec3_close(hit.location, [0.0, 1.6200000047683716, 2.509999990463257]);
        assert_vec3_close(
            hit.relative_location,
            [0.0, 0.6200000047683716, -0.49000000953674316],
        );
    }

    #[test]
    fn crosshair_target_skips_unknown_and_non_pickable_entity_types() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            [0.0, 1.0, 3.0],
        ));
        world.apply_add_entity(protocol_add_entity(11, 999, [0.0, 1.0, 2.0]));

        assert_eq!(
            crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0))),
            None
        );
    }

    #[test]
    fn crosshair_target_skips_marker_armor_stand() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            [0.0, 1.0, 3.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 12,
            values: vec![EntityDataValue {
                data_id: 16,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(16),
            }],
        }));

        assert_eq!(
            crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0))),
            None
        );
    }

    #[test]
    fn crosshair_target_hits_interaction_entity_with_metadata_bounds() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_INTERACTION_ID,
            [0.0, 1.0, 3.0],
        ));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 12,
            values: vec![
                EntityDataValue {
                    data_id: 8,
                    serializer_id: 3,
                    value: EntityDataValueKind::Float(2.0),
                },
                EntityDataValue {
                    data_id: 9,
                    serializer_id: 3,
                    value: EntityDataValueKind::Float(1.25),
                },
            ],
        }));

        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected interaction entity hit");
        };
        assert_eq!(hit.entity_id, 12);
        assert_vec3_close(hit.relative_location, [0.0, 0.6200000047683716, -1.0]);
    }

    #[test]
    fn crosshair_target_hits_ender_dragon_part_id_from_world_state() {
        const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            100,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            [0.0, 1.0, 9.0],
        ));
        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected dragon part entity hit");
        };
        assert_eq!(hit.entity_id, 101);
        assert_vec3_close(hit.location, [0.0, 1.6200000047683716, 2.0]);
        assert_vec3_close(hit.relative_location, [0.0, 0.6200000047683716, -0.5]);
    }

    #[test]
    fn crosshair_target_hits_direction_aware_item_frame_bounds() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity_with_data(
            13,
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            [0.0, 1.0, 2.0],
            2,
        ));

        let target =
            crosshair_target_from_world(&world, Some(player_pose_at([0.5, 0.0, 0.0], 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected item frame entity hit");
        };
        assert_eq!(hit.entity_id, 13);
        assert_vec3_close(hit.location, [0.5, 1.6200000047683716, 2.9375]);
        assert_vec3_close(hit.relative_location, [0.0, 0.12000000476837158, -0.03125]);
    }

    #[test]
    fn crosshair_target_skips_entity_beyond_vanilla_default_entity_interaction_range() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 4.0],
        ));

        assert_eq!(
            DEFAULT_ENTITY_INTERACTION_RANGE, 3.0,
            "vanilla Player.DEFAULT_ENTITY_INTERACTION_RANGE"
        );
        assert_eq!(
            crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0))),
            None
        );
    }

    #[test]
    fn crosshair_target_keeps_block_when_block_is_nearer_than_entity() {
        let eye = [0.0, 1.6200000047683716, 0.0];
        let block = CrosshairBlockHit {
            pos: BlockPos { x: 0, y: 1, z: 2 },
            face: ProtocolDirection::North,
            cursor: [0.0, 0.62, 0.0],
            inside: false,
        };
        let entity = RaycastEntityHit {
            hit: CrosshairEntityHit {
                entity_id: 10,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: eye[1],
                    z: 3.0,
                },
                relative_location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.62,
                    z: -0.5,
                },
            },
            distance_sq: 9.0,
        };

        assert_eq!(
            choose_crosshair_target(
                eye,
                Some(block),
                Some(entity),
                DEFAULT_ENTITY_INTERACTION_RANGE
            ),
            Some(CrosshairTarget::Block(block))
        );
    }

    #[test]
    fn crosshair_target_prefers_entity_when_entity_is_nearer_than_block() {
        let eye = [0.0, 1.6200000047683716, 0.0];
        let block = CrosshairBlockHit {
            pos: BlockPos { x: 0, y: 1, z: 3 },
            face: ProtocolDirection::North,
            cursor: [0.0, 0.62, 0.0],
            inside: false,
        };
        let entity_hit = CrosshairEntityHit {
            entity_id: 10,
            location: ProtocolVec3d {
                x: 0.0,
                y: eye[1],
                z: 2.0,
            },
            relative_location: ProtocolVec3d {
                x: 0.0,
                y: 0.62,
                z: -0.5,
            },
        };
        let entity = RaycastEntityHit {
            hit: entity_hit,
            distance_sq: 4.0,
        };

        assert_eq!(
            choose_crosshair_target(
                eye,
                Some(block),
                Some(entity),
                DEFAULT_ENTITY_INTERACTION_RANGE
            ),
            Some(CrosshairTarget::Entity(entity_hit))
        );
    }

    #[test]
    fn crosshair_target_misses_when_nearer_entity_exceeds_entity_interaction_range() {
        let eye = [0.0, 1.6200000047683716, 0.0];
        let block = CrosshairBlockHit {
            pos: BlockPos { x: 0, y: 1, z: 4 },
            face: ProtocolDirection::North,
            cursor: [0.0, 0.62, 0.0],
            inside: false,
        };
        let entity = RaycastEntityHit {
            hit: CrosshairEntityHit {
                entity_id: 10,
                location: ProtocolVec3d {
                    x: 0.0,
                    y: eye[1],
                    z: 3.5,
                },
                relative_location: ProtocolVec3d {
                    x: 0.0,
                    y: 0.62,
                    z: -0.5,
                },
            },
            distance_sq: 3.5 * 3.5,
        };

        assert_eq!(
            choose_crosshair_target(
                eye,
                Some(block),
                Some(entity),
                DEFAULT_ENTITY_INTERACTION_RANGE
            ),
            None
        );
    }

    #[test]
    fn selection_outline_uses_block_bounds() {
        assert_eq!(
            selection_outline_for_block(BlockPos { x: -2, y: 63, z: 4 }),
            SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 64.0, 5.0])
        );
    }

    fn player_pose(y_rot: f32, x_rot: f32, z: f64) -> PlayerPose {
        player_pose_at([0.0, 0.0, z], y_rot, x_rot)
    }

    fn player_pose_at(position: [f64; 3], y_rot: f32, x_rot: f32) -> PlayerPose {
        PlayerPose {
            position: NetVec3 {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            y_rot,
            x_rot,
            ..PlayerPose::default()
        }
    }

    fn protocol_add_entity(id: i32, entity_type_id: i32, position: [f64; 3]) -> AddEntity {
        protocol_add_entity_with_data(id, entity_type_id, position, 0)
    }

    fn protocol_add_entity_with_data(
        id: i32,
        entity_type_id: i32,
        position: [f64; 3],
        data: i32,
    ) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
            entity_type_id,
            position: ProtocolVec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data,
        }
    }

    fn assert_vec3_close(actual: ProtocolVec3d, expected: [f64; 3]) {
        assert!(
            (actual.x - expected[0]).abs() < 1.0e-6,
            "x: expected {}, got {}",
            expected[0],
            actual.x
        );
        assert!(
            (actual.y - expected[1]).abs() < 1.0e-6,
            "y: expected {}, got {}",
            expected[1],
            actual.y
        );
        assert!(
            (actual.z - expected[2]).abs() < 1.0e-6,
            "z: expected {}, got {}",
            expected[2],
            actual.z
        );
    }
}
