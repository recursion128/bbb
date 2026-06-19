use bbb_protocol::packets::{
    BlockHitResult as ProtocolBlockHitResult, BlockPos as ProtocolBlockPos,
    Direction as ProtocolDirection, Vec3d as ProtocolVec3d,
};
use bbb_renderer::{CameraPose, SelectionOutline};
#[cfg(test)]
use bbb_world::LocalPlayerPoseState;
use bbb_world::{BlockPos, EntityPickTargetState, ItemAttackRange, WorldStore};

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

pub(crate) fn selection_outline_from_camera(
    world: &WorldStore,
    pose: Option<CameraPose>,
) -> Option<SelectionOutline> {
    let hit = crosshair_block_hit_from_camera(world, pose)?;
    match world.probe_block(hit.pos) {
        Some(probe) => selection_outline_for_probe(&probe),
        None => Some(selection_outline_for_block(hit.pos)),
    }
}

pub(crate) fn entity_target_outline_from_camera_at_partial_tick(
    world: &WorldStore,
    pose: Option<CameraPose>,
    entity_partial_tick: f32,
) -> Option<SelectionOutline> {
    let CrosshairTarget::Entity(hit) =
        crosshair_target_from_camera_at_partial_tick(world, pose, entity_partial_tick)?
    else {
        return None;
    };
    let partial_tick = clamp_entity_partial_tick(entity_partial_tick);
    world
        .entity_pick_targets_at_partial_tick(partial_tick)
        .into_iter()
        .find(|target| target.entity_id == hit.entity_id)
        .map(entity_pick_target_outline)
}

fn entity_pick_target_outline(target: EntityPickTargetState) -> SelectionOutline {
    let inflate = f64::from(target.bounds.pick_radius);
    SelectionOutline::from_box(
        [
            (target.position.x + f64::from(target.bounds.min[0]) - inflate) as f32,
            (target.position.y + f64::from(target.bounds.min[1]) - inflate) as f32,
            (target.position.z + f64::from(target.bounds.min[2]) - inflate) as f32,
        ],
        [
            (target.position.x + f64::from(target.bounds.max[0]) + inflate) as f32,
            (target.position.y + f64::from(target.bounds.max[1]) + inflate) as f32,
            (target.position.z + f64::from(target.bounds.max[2]) + inflate) as f32,
        ],
    )
}

fn crosshair_block_hit_from_camera(
    world: &WorldStore,
    pose: Option<CameraPose>,
) -> Option<CrosshairBlockHit> {
    crosshair_block_hit_from_ray(world, crosshair_ray_from_camera_pose(pose?))
}

fn crosshair_block_hit_from_ray(
    world: &WorldStore,
    ray: CrosshairRay,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit_from_ray(ray, DEFAULT_BLOCK_INTERACTION_RANGE, |pos| {
        world
            .probe_block(pos)
            .map(|probe| BlockOutlineTarget::from_probe(&probe))
    })
}

pub(crate) fn crosshair_target_from_camera_at_partial_tick(
    world: &WorldStore,
    pose: Option<CameraPose>,
    entity_partial_tick: f32,
) -> Option<CrosshairTarget> {
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.local_player().camera.entity_id;
    crosshair_target_from_ray(
        world,
        crosshair_ray_from_camera_pose(pose?),
        entity_partial_tick,
        |entity_id| {
            local_player_id.is_some_and(|id| id == entity_id)
                || camera_entity_id.is_some_and(|id| id == entity_id)
        },
    )
}

#[cfg(test)]
pub(crate) fn crosshair_target_from_world(
    world: &WorldStore,
    pose: Option<LocalPlayerPoseState>,
) -> Option<CrosshairTarget> {
    crosshair_target_from_world_at_partial_tick(world, pose, 1.0)
}

#[cfg(test)]
fn crosshair_target_from_world_at_partial_tick(
    world: &WorldStore,
    pose: Option<LocalPlayerPoseState>,
    entity_partial_tick: f32,
) -> Option<CrosshairTarget> {
    let local_player_id = world.local_player_id();
    crosshair_target_from_ray(
        world,
        crosshair_ray_from_player_pose(pose?),
        entity_partial_tick,
        |entity_id| local_player_id.is_some_and(|id| id == entity_id),
    )
}

fn crosshair_target_from_ray<F>(
    world: &WorldStore,
    ray: CrosshairRay,
    entity_partial_tick: f32,
    mut excluded_entity_id: F,
) -> Option<CrosshairTarget>
where
    F: FnMut(i32) -> bool,
{
    let eye = ray.eye;
    let block_hit = crosshair_block_hit_from_ray(world, ray);
    if let Some(attack_range) = selected_crosshair_attack_range(world, ray) {
        if let Some(target) = attack_range_target_from_ray(
            world,
            ray,
            attack_range,
            entity_partial_tick,
            &mut excluded_entity_id,
        ) {
            return Some(target);
        }
    }
    let block_distance_sq = block_hit
        .map(|hit| distance_sq(eye, block_hit_location(hit)))
        .unwrap_or(f64::INFINITY);
    let entity_max_distance = block_distance_sq
        .sqrt()
        .min(DEFAULT_BLOCK_INTERACTION_RANGE);
    let entity_hit = crosshair_entity_hit_from_ray(
        world,
        ray,
        entity_max_distance,
        entity_partial_tick,
        &mut excluded_entity_id,
    );

    choose_crosshair_target(eye, block_hit, entity_hit, DEFAULT_ENTITY_INTERACTION_RANGE)
}

#[derive(Debug, Clone, Copy)]
struct CrosshairAttackRange {
    min_reach: f64,
    max_reach: f64,
    hitbox_margin: f64,
    movement_extension: f64,
}

fn selected_crosshair_attack_range(
    world: &WorldStore,
    ray: CrosshairRay,
) -> Option<CrosshairAttackRange> {
    let range = world.local_selected_main_hand_attack_range()?;
    let mut attack_range = crosshair_attack_range_from_item(
        range,
        world
            .local_player()
            .abilities
            .is_some_and(|abilities| abilities.instabuild),
    );
    attack_range.movement_extension = crosshair_attack_range_movement_extension(world, ray);
    Some(attack_range)
}

fn crosshair_attack_range_from_item(
    range: ItemAttackRange,
    local_player_instabuild: bool,
) -> CrosshairAttackRange {
    let (min_reach, max_reach) = if local_player_instabuild {
        (range.min_creative_reach, range.max_creative_reach)
    } else {
        (range.min_reach, range.max_reach)
    };
    CrosshairAttackRange {
        min_reach: f64::from(min_reach.max(0.0)),
        max_reach: f64::from(max_reach.max(min_reach).max(0.0)),
        hitbox_margin: f64::from(range.hitbox_margin.max(0.0)),
        movement_extension: 0.0,
    }
}

fn crosshair_attack_range_movement_extension(world: &WorldStore, ray: CrosshairRay) -> f64 {
    let Some(pose) = world.local_player_pose() else {
        return 0.0;
    };
    let direction = look_direction_from_crosshair_ray(ray);
    if direction == [0.0, 0.0, 0.0] {
        return 0.0;
    }
    let movement = pose.delta_movement;
    let component =
        movement.x * direction[0] + movement.y * direction[1] + movement.z * direction[2];
    if component.is_finite() {
        component.max(0.0)
    } else {
        0.0
    }
}

fn attack_range_target_from_ray<F>(
    world: &WorldStore,
    ray: CrosshairRay,
    attack_range: CrosshairAttackRange,
    entity_partial_tick: f32,
    excluded_entity_id: &mut F,
) -> Option<CrosshairTarget>
where
    F: FnMut(i32) -> bool,
{
    let max_search_reach = attack_range.max_reach + attack_range.movement_extension;
    if max_search_reach <= 0.0 {
        return None;
    }

    let eye = ray.eye;
    let block_hit = raycast_crosshair_block_hit_from_ray(ray, max_search_reach, |pos| {
        world
            .probe_block(pos)
            .map(|probe| BlockOutlineTarget::from_probe(&probe))
    });
    let block_distance = block_hit
        .map(|hit| distance_sq(eye, block_hit_location(hit)).sqrt())
        .unwrap_or(f64::INFINITY);
    if block_distance < attack_range.min_reach {
        return block_hit
            .filter(|_| block_distance < DEFAULT_BLOCK_INTERACTION_RANGE)
            .map(CrosshairTarget::Block);
    }

    let entity_end_distance = block_distance.min(max_search_reach);
    let entity_hit = crosshair_entity_hit_between_from_ray(
        world,
        ray,
        attack_range.min_reach,
        entity_end_distance,
        attack_range.hitbox_margin,
        entity_partial_tick,
        excluded_entity_id,
    );
    if let Some(entity_hit) = entity_hit {
        return Some(CrosshairTarget::Entity(entity_hit.hit));
    }

    block_hit
        .filter(|_| block_distance < DEFAULT_BLOCK_INTERACTION_RANGE)
        .map(CrosshairTarget::Block)
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

fn crosshair_entity_hit_from_ray<F>(
    world: &WorldStore,
    ray: CrosshairRay,
    max_distance: f64,
    entity_partial_tick: f32,
    excluded_entity_id: &mut F,
) -> Option<RaycastEntityHit>
where
    F: FnMut(i32) -> bool,
{
    let targets = world
        .entity_pick_targets_at_partial_tick(clamp_entity_partial_tick(entity_partial_tick))
        .into_iter()
        .filter_map(|target| {
            if excluded_entity_id(target.entity_id) {
                None
            } else {
                Some(EntityRaycastTarget { target })
            }
        });
    raycast_crosshair_entity_hit(ray, max_distance, targets)
}

fn crosshair_entity_hit_between_from_ray<F>(
    world: &WorldStore,
    ray: CrosshairRay,
    min_distance: f64,
    max_distance: f64,
    hitbox_margin: f64,
    entity_partial_tick: f32,
    excluded_entity_id: &mut F,
) -> Option<RaycastEntityHit>
where
    F: FnMut(i32) -> bool,
{
    if max_distance < min_distance {
        return None;
    }
    let targets = world
        .entity_pick_targets_at_partial_tick(clamp_entity_partial_tick(entity_partial_tick))
        .into_iter()
        .filter_map(|target| {
            if excluded_entity_id(target.entity_id) {
                None
            } else {
                Some(EntityRaycastTarget { target })
            }
        });
    raycast_crosshair_entity_hit_between(
        world,
        ray,
        min_distance,
        max_distance,
        hitbox_margin,
        targets,
    )
}

fn clamp_entity_partial_tick(partial_tick: f32) -> f32 {
    if partial_tick.is_finite() {
        partial_tick.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
fn raycast_crosshair_block<F>(
    pose: LocalPlayerPoseState,
    max_distance: f64,
    mut material_at: F,
) -> Option<BlockPos>
where
    F: FnMut(BlockPos) -> Option<bbb_world::TerrainMaterialClass>,
{
    raycast_crosshair_block_hit_from_ray(
        crosshair_ray_from_player_pose(pose),
        max_distance,
        |pos| material_at(pos).map(BlockOutlineTarget::full_block),
    )
    .map(|hit| hit.pos)
}

#[cfg(test)]
fn raycast_crosshair_block_hit<F>(
    pose: LocalPlayerPoseState,
    max_distance: f64,
    target_at: F,
) -> Option<CrosshairBlockHit>
where
    F: FnMut(BlockPos) -> Option<BlockOutlineTarget>,
{
    raycast_crosshair_block_hit_from_ray(
        crosshair_ray_from_player_pose(pose),
        max_distance,
        target_at,
    )
}

fn raycast_crosshair_block_hit_from_ray<F>(
    ray: CrosshairRay,
    max_distance: f64,
    target_at: F,
) -> Option<CrosshairBlockHit>
where
    F: FnMut(BlockPos) -> Option<BlockOutlineTarget>,
{
    let direction = look_direction_from_crosshair_ray(ray);
    raycast_block_hit_from_direction(ray.eye, direction, max_distance, target_at)
}

fn raycast_block_hit_from_direction<F>(
    eye: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
    mut target_at: F,
) -> Option<CrosshairBlockHit>
where
    F: FnMut(BlockPos) -> Option<BlockOutlineTarget>,
{
    if max_distance <= 0.0 {
        return None;
    }

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
    ray: CrosshairRay,
    max_distance: f64,
    targets: I,
) -> Option<RaycastEntityHit>
where
    I: IntoIterator<Item = EntityRaycastTarget>,
{
    if max_distance <= 0.0 {
        return None;
    }

    let eye = ray.eye;
    let direction = look_direction_from_crosshair_ray(ray);
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

fn raycast_crosshair_entity_hit_between<I>(
    world: &WorldStore,
    ray: CrosshairRay,
    min_distance: f64,
    max_distance: f64,
    hitbox_margin: f64,
    targets: I,
) -> Option<RaycastEntityHit>
where
    I: IntoIterator<Item = EntityRaycastTarget>,
{
    if max_distance < min_distance || max_distance <= 0.0 {
        return None;
    }

    let eye = ray.eye;
    let direction = look_direction_from_crosshair_ray(ray);
    if direction == [0.0, 0.0, 0.0] {
        return None;
    }

    let mut nearest: Option<RaycastEntityHit> = None;
    for target in targets {
        let Some(intersection) = raycast_entity_target_intersection_between(
            world,
            eye,
            direction,
            min_distance.max(0.0),
            max_distance,
            hitbox_margin,
            target,
        ) else {
            continue;
        };
        let distance_sq = intersection.distance_sq;
        if nearest.is_some_and(|hit| hit.distance_sq <= distance_sq) {
            continue;
        }
        let location = ProtocolVec3d {
            x: intersection.location[0],
            y: intersection.location[1],
            z: intersection.location[2],
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

#[derive(Debug, Clone, Copy)]
struct EntityTargetIntersection {
    location: [f64; 3],
    distance_sq: f64,
}

fn raycast_entity_target_distance(
    eye: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
    target: EntityRaycastTarget,
) -> Option<f64> {
    let inflate = f64::from(target.target.bounds.pick_radius);
    let (min, max) = entity_target_box(target, inflate);
    ray_box_distance(eye, direction, max_distance, min, max)
}

fn raycast_entity_target_intersection_between(
    world: &WorldStore,
    eye: [f64; 3],
    direction: [f64; 3],
    min_distance: f64,
    max_distance: f64,
    hitbox_margin: f64,
    target: EntityRaycastTarget,
) -> Option<EntityTargetIntersection> {
    let from = [
        eye[0] + direction[0] * min_distance,
        eye[1] + direction[1] * min_distance,
        eye[2] + direction[2] * min_distance,
    ];
    let segment_distance = max_distance - min_distance;
    let (entity_min, entity_max) = entity_target_box(target, 0.0);
    if contains_point(entity_min, entity_max, from) {
        return Some(EntityTargetIntersection {
            location: from,
            distance_sq: distance_sq(eye, from),
        });
    }

    if let Some(distance) =
        ray_box_distance(from, direction, segment_distance, entity_min, entity_max)
    {
        let total_distance = min_distance + distance;
        let location = [
            eye[0] + direction[0] * total_distance,
            eye[1] + direction[1] * total_distance,
            eye[2] + direction[2] * total_distance,
        ];
        return Some(EntityTargetIntersection {
            location,
            distance_sq: distance_sq(eye, location),
        });
    }

    if hitbox_margin <= 0.0 {
        return None;
    }

    let (inflated_min, inflated_max) = entity_target_box(target, hitbox_margin);
    let outside_distance = ray_box_distance(
        from,
        direction,
        segment_distance,
        inflated_min,
        inflated_max,
    )?;
    let outside_total_distance = min_distance + outside_distance;
    let outside = [
        eye[0] + direction[0] * outside_total_distance,
        eye[1] + direction[1] * outside_total_distance,
        eye[2] + direction[2] * outside_total_distance,
    ];
    let center = box_center(entity_min, entity_max);
    let target = clip_segment_end_by_block(world, outside, center).unwrap_or(center);
    let to_target = [
        target[0] - outside[0],
        target[1] - outside[1],
        target[2] - outside[2],
    ];
    let target_distance =
        (to_target[0] * to_target[0] + to_target[1] * to_target[1] + to_target[2] * to_target[2])
            .sqrt();
    if target_distance <= f64::EPSILON {
        return None;
    }
    let surface_direction = [
        to_target[0] / target_distance,
        to_target[1] / target_distance,
        to_target[2] / target_distance,
    ];
    let surface_distance = ray_box_distance(
        outside,
        surface_direction,
        target_distance,
        entity_min,
        entity_max,
    )?;
    let location = [
        outside[0] + surface_direction[0] * surface_distance,
        outside[1] + surface_direction[1] * surface_distance,
        outside[2] + surface_direction[2] * surface_distance,
    ];
    Some(EntityTargetIntersection {
        location,
        distance_sq: distance_sq(eye, location),
    })
}

fn entity_target_box(target: EntityRaycastTarget, inflate: f64) -> ([f64; 3], [f64; 3]) {
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
    (min, max)
}

fn box_center(min: [f64; 3], max: [f64; 3]) -> [f64; 3] {
    [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ]
}

fn clip_segment_end_by_block(world: &WorldStore, from: [f64; 3], to: [f64; 3]) -> Option<[f64; 3]> {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if distance <= f64::EPSILON {
        return None;
    }
    let direction = [
        delta[0] / distance,
        delta[1] / distance,
        delta[2] / distance,
    ];
    raycast_block_hit_from_direction(from, direction, distance, |pos| {
        world
            .probe_block(pos)
            .map(|probe| BlockOutlineTarget::from_probe(&probe))
    })
    .map(block_hit_location)
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

#[cfg(test)]
fn eye_position_from_player_pose(pose: LocalPlayerPoseState) -> [f64; 3] {
    [
        pose.position.x,
        pose.position.y + pose.eye_height(),
        pose.position.z,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CrosshairRay {
    eye: [f64; 3],
    y_rot: f32,
    x_rot: f32,
}

#[cfg(test)]
fn crosshair_ray_from_player_pose(pose: LocalPlayerPoseState) -> CrosshairRay {
    CrosshairRay {
        eye: eye_position_from_player_pose(pose),
        y_rot: pose.y_rot,
        x_rot: pose.x_rot,
    }
}

fn crosshair_ray_from_camera_pose(pose: CameraPose) -> CrosshairRay {
    CrosshairRay {
        eye: [
            f64::from(pose.position[0]),
            f64::from(pose.position[1]) + f64::from(pose.eye_height),
            f64::from(pose.position[2]),
        ],
        y_rot: pose.y_rot,
        x_rot: pose.x_rot,
    }
}

fn look_direction_from_crosshair_ray(ray: CrosshairRay) -> [f64; 3] {
    let yaw = f64::from(ray.y_rot).to_radians();
    let pitch = f64::from(ray.x_rot).to_radians();
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
    use bbb_protocol::packets::{
        AddEntity, AttackRangeSummary, EntityDataValue, EntityDataValueKind, EntityPositionSync,
        ItemStackSummary, PlayerAbilities, SetEntityData, SetPlayerInventory,
        Vec3d as ProtocolVec3d,
    };
    use bbb_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use uuid::Uuid;

    const VANILLA_ATTACK_RANGE_COMPONENT_ID: i32 = 30;
    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;
    const VANILLA_GRASS_BLOCK_STATE_ID: i32 = 9;
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
        let eye_height = LocalPlayerPoseState::default().eye_height();
        assert_eq!(hit.entity_id, 10);
        assert_vec3_close(hit.location, [0.0, eye_height, 2.509999990463257]);
        assert_vec3_close(
            hit.relative_location,
            [0.0, eye_height - 1.0, -0.49000000953674316],
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
            [0.0, 2.0, 9.0],
        ));
        world.advance_entity_client_animations(1);
        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected dragon part entity hit");
        };
        let eye_height = LocalPlayerPoseState::default().eye_height();
        assert_eq!(hit.entity_id, 101);
        assert_vec3_close(hit.location, [0.0, eye_height, 2.0]);
        assert_vec3_close(hit.relative_location, [0.0, eye_height - 1.0, -0.5]);
    }

    #[test]
    fn crosshair_target_hits_ender_dragon_tail_and_wing_part_ids() {
        const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            100,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            [0.0, 2.0, 9.0],
        ));
        world.advance_entity_client_animations(1);

        let eye_height = LocalPlayerPoseState::default().eye_height();
        let tail_pose = player_pose_at([0.0, 4.5 - eye_height, 11.25], 0.0, 0.0);
        let CrosshairTarget::Entity(tail_hit) =
            crosshair_target_from_world(&world, Some(tail_pose)).unwrap()
        else {
            panic!("expected dragon tail part entity hit");
        };
        assert_eq!(tail_hit.entity_id, 104);
        assert_vec3_close(tail_hit.location, [0.0, 4.5, 11.5]);
        assert_vec3_close(tail_hit.relative_location, [0.0, 1.0, -1.0]);

        let wing_pose = player_pose_at([4.5, 5.0 - eye_height, 6.0], 0.0, 0.0);
        let CrosshairTarget::Entity(wing_hit) =
            crosshair_target_from_world(&world, Some(wing_pose)).unwrap()
        else {
            panic!("expected dragon wing part entity hit");
        };
        assert_eq!(wing_hit.entity_id, 107);
        assert_vec3_close(wing_hit.location, [4.5, 5.0, 7.0]);
        assert_vec3_close(wing_hit.relative_location, [0.0, 1.0, -2.0]);
    }

    #[test]
    fn crosshair_target_uses_entity_partial_tick_for_dragon_parts() {
        const VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID: i32 = 43;
        const ENDER_DRAGON_PHASE_DATA_ID: u8 = 16;
        const HOLDING_PATTERN_PHASE_ID: i32 = 0;

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            130,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            [1.0, 64.0, -2.0],
        ));
        world.advance_entity_client_animations(1);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 130,
            values: vec![EntityDataValue {
                data_id: ENDER_DRAGON_PHASE_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(HOLDING_PATTERN_PHASE_ID),
            }],
        }));
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id: 130,
            position: ProtocolVec3d {
                x: 1.0,
                y: 74.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 90.0,
            x_rot: 0.0,
            on_ground: false,
        }));
        world.advance_entity_client_animations(1);

        let pose = player_pose_at(
            [
                7.5,
                64.0 - LocalPlayerPoseState::default().eye_height(),
                -5.0,
            ],
            0.0,
            0.0,
        );

        assert_eq!(
            crosshair_target_from_world_at_partial_tick(&world, Some(pose), 0.0),
            None
        );
        let target = crosshair_target_from_world_at_partial_tick(&world, Some(pose), 1.0);

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected dragon part entity hit");
        };
        assert_eq!(hit.entity_id, 132);
        assert_vec3_close(hit.location, [7.5, 64.0, -3.5]);
        assert_vec3_close(hit.relative_location, [1.0, 0.0, -1.5]);
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
        let eye_height = LocalPlayerPoseState::default().eye_height();
        assert_eq!(hit.entity_id, 13);
        assert_vec3_close(hit.location, [0.5, eye_height, 2.9375]);
        assert_vec3_close(hit.relative_location, [0.0, eye_height - 1.5, -0.03125]);
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
    fn crosshair_target_uses_selected_item_attack_range_for_extended_entity_hit() {
        let mut world = WorldStore::new();
        set_selected_attack_range(&mut world, 0.0, 4.5, 0.0, 4.5, 0.0);
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 4.0],
        ));

        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected entity hit");
        };
        assert_eq!(hit.entity_id, 12);
        assert_vec3_close(
            hit.location,
            [0.0, LocalPlayerPoseState::default().eye_height(), 3.51],
        );
    }

    #[test]
    fn crosshair_target_attack_range_extends_max_reach_by_forward_movement() {
        let mut world = WorldStore::new();
        set_selected_attack_range(&mut world, 0.0, 3.5, 0.0, 3.5, 0.0);
        let pose = player_pose_with_delta(0.0, 0.0, 0.0, [0.0, 0.0, 0.25]);
        world.set_local_player_pose(pose);
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 4.0],
        ));

        let target = crosshair_target_from_world(&world, Some(pose));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected entity hit from movement-extended attack range");
        };
        assert_eq!(hit.entity_id, 12);
        assert_vec3_close(
            hit.location,
            [0.0, LocalPlayerPoseState::default().eye_height(), 3.51],
        );
    }

    #[test]
    fn crosshair_target_attack_range_does_not_extend_max_reach_by_backward_movement() {
        let mut world = WorldStore::new();
        set_selected_attack_range(&mut world, 0.0, 3.5, 0.0, 3.5, 0.0);
        let pose = player_pose_with_delta(0.0, 0.0, 0.0, [0.0, 0.0, -0.25]);
        world.set_local_player_pose(pose);
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 4.0],
        ));

        assert_eq!(crosshair_target_from_world(&world, Some(pose)), None);
    }

    #[test]
    fn crosshair_target_attack_range_uses_hitbox_margin() {
        let mut world = WorldStore::new();
        set_selected_attack_range(&mut world, 0.0, 3.3, 0.0, 3.3, 0.25);
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 4.0],
        ));

        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected entity hit from attack range margin");
        };
        assert_eq!(hit.entity_id, 12);
        assert_vec3_close(hit.location, [0.0, 1.5287837829456616, 3.51]);
    }

    #[test]
    fn crosshair_target_attack_range_uses_creative_reach_when_instabuild() {
        let mut world = WorldStore::new();
        set_selected_attack_range(&mut world, 0.0, 1.0, 0.0, 5.0, 0.0);
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 4.0],
        ));

        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Entity(hit) = target.unwrap() else {
            panic!("expected creative reach entity hit");
        };
        assert_eq!(hit.entity_id, 12);
    }

    #[test]
    fn crosshair_target_attack_range_keeps_block_when_block_clips_before_extended_entity() {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        insert_air_chunk(&mut world);
        set_selected_attack_range(&mut world, 0.0, 4.5, 0.0, 4.5, 0.0);
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x: 0, y: 1, z: 2 },
                block_state_id: VANILLA_GRASS_BLOCK_STATE_ID,
            })
        );
        world.apply_add_entity(protocol_add_entity(
            12,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 4.0],
        ));

        let target = crosshair_target_from_world(&world, Some(player_pose(0.0, 0.0, 0.0)));

        let CrosshairTarget::Block(hit) = target.unwrap() else {
            panic!("expected nearer block hit");
        };
        assert_eq!(hit.pos, BlockPos { x: 0, y: 1, z: 2 });
    }

    #[test]
    fn crosshair_target_keeps_block_when_block_is_nearer_than_entity() {
        let eye = [0.0, LocalPlayerPoseState::default().eye_height(), 0.0];
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
        let eye = [0.0, LocalPlayerPoseState::default().eye_height(), 0.0];
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
        let eye = [0.0, LocalPlayerPoseState::default().eye_height(), 0.0];
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
    fn camera_crosshair_raycast_uses_camera_eye_height() {
        let ray = crosshair_ray_from_camera_pose(CameraPose {
            position: [0.0, 0.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.2751,
        });
        let hit = raycast_crosshair_block_hit_from_ray(ray, 5.0, |pos| {
            if pos == (BlockPos { x: 0, y: 0, z: 3 }) {
                Some(BlockOutlineTarget::full_block(
                    bbb_world::TerrainMaterialClass::Opaque,
                ))
            } else {
                None
            }
        });

        let hit = hit.expect("camera ray should hit the low block");
        assert_eq!(hit.pos, BlockPos { x: 0, y: 0, z: 3 });
        assert!((hit.cursor[1] - 0.2751).abs() < 0.0001);
    }

    #[test]
    fn player_pose_crosshair_ray_uses_local_player_eye_height() {
        let pose = LocalPlayerPoseState {
            sneaking: true,
            ..player_pose_at([0.0, 64.0, 0.0], 0.0, 0.0)
        };

        let ray = crosshair_ray_from_player_pose(pose);

        assert_eq!(ray.eye, [0.0, 64.0 + pose.eye_height(), 0.0]);
    }

    #[test]
    fn selection_outline_uses_block_bounds() {
        assert_eq!(
            selection_outline_for_block(BlockPos { x: -2, y: 63, z: 4 }),
            SelectionOutline::from_box([-2.0, 63.0, 4.0], [-1.0, 64.0, 5.0])
        );
    }

    #[test]
    fn entity_target_outline_is_none_without_crosshair_entity_target() {
        let world = WorldStore::new();

        assert_eq!(
            entity_target_outline_from_camera_at_partial_tick(
                &world,
                Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0)),
                1.0,
            ),
            None
        );
    }

    #[test]
    fn entity_target_outline_uses_current_pick_target_bounds() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            [0.0, 1.0, 3.0],
        ));

        let outline = entity_target_outline_from_camera_at_partial_tick(
            &world,
            Some(camera_pose_at([0.0, 0.0, 0.0], 0.0, 0.0)),
            1.0,
        )
        .expect("expected entity target outline");

        assert_eq!(outline.boxes.len(), 1);
        assert_selection_box_close(outline.boxes[0].min, [-0.49, 1.0, 2.51]);
        assert_selection_box_close(outline.boxes[0].max, [0.49, 1.7, 3.49]);
    }

    fn player_pose(y_rot: f32, x_rot: f32, z: f64) -> LocalPlayerPoseState {
        player_pose_at([0.0, 0.0, z], y_rot, x_rot)
    }

    fn player_pose_with_delta(
        y_rot: f32,
        x_rot: f32,
        z: f64,
        delta_movement: [f64; 3],
    ) -> LocalPlayerPoseState {
        LocalPlayerPoseState {
            delta_movement: ProtocolVec3d {
                x: delta_movement[0],
                y: delta_movement[1],
                z: delta_movement[2],
            },
            ..player_pose(y_rot, x_rot, z)
        }
    }

    fn player_pose_at(position: [f64; 3], y_rot: f32, x_rot: f32) -> LocalPlayerPoseState {
        LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            y_rot,
            x_rot,
            ..LocalPlayerPoseState::default()
        }
    }

    fn camera_pose_at(position: [f32; 3], y_rot: f32, x_rot: f32) -> CameraPose {
        CameraPose {
            position,
            y_rot,
            x_rot,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        }
    }

    fn protocol_add_entity(id: i32, entity_type_id: i32, position: [f64; 3]) -> AddEntity {
        protocol_add_entity_with_data(id, entity_type_id, position, 0)
    }

    fn set_selected_attack_range(
        world: &mut WorldStore,
        min_reach: f32,
        max_reach: f32,
        min_creative_reach: f32,
        max_creative_reach: f32,
        hitbox_margin: f32,
    ) {
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: item_stack_with_attack_range(
                42,
                min_reach,
                max_reach,
                min_creative_reach,
                max_creative_reach,
                hitbox_margin,
            ),
        });
    }

    fn insert_air_chunk(world: &mut WorldStore) {
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    VANILLA_AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
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

    fn item_stack_with_attack_range(
        item_id: i32,
        min_reach: f32,
        max_reach: f32,
        min_creative_reach: f32,
        max_creative_reach: f32,
        hitbox_margin: f32,
    ) -> ItemStackSummary {
        let mut stack = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        };
        stack.component_patch.added = 1;
        stack.component_patch.added_type_ids = vec![VANILLA_ATTACK_RANGE_COMPONENT_ID];
        stack.component_patch.attack_range = Some(AttackRangeSummary {
            min_reach,
            max_reach,
            min_creative_reach,
            max_creative_reach,
            hitbox_margin,
            mob_factor: 1.0,
        });
        stack
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

    fn assert_selection_box_close(actual: [f32; 3], expected: [f32; 3]) {
        for axis in 0..3 {
            assert!(
                (actual[axis] - expected[axis]).abs() < 1.0e-5,
                "axis {axis}: expected {}, got {}",
                expected[axis],
                actual[axis]
            );
        }
    }
}
