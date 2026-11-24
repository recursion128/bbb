#[cfg(test)]
use bbb_client_render::SelectionOutline;
use bbb_client_render::{CameraPose, SelectionBox};
#[cfg(test)]
use bbb_client_world::VANILLA_ENDER_DRAGON_PHASE_HOLDING_PATTERN_ID as ENDER_DRAGON_HOLDING_PATTERN_PHASE_ID;
use bbb_client_world::{
    BlockPos, EntityPickTargetState, ItemAttackRange, LocalItemUsePovFluidMode,
    LocalItemUsePovQuery, LocalItemUsePovResult, LocalPlayerPoseState, WorldStore,
};
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID, VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID,
};
#[cfg(test)]
use bbb_protocol::entity_types::*;
#[cfg(test)]
use bbb_protocol::packets::BlockPos as ProtocolBlockPos;
use bbb_protocol::packets::{
    BlockHitResult as ProtocolBlockHitResult, Direction as ProtocolDirection,
    Vec3d as ProtocolVec3d,
};

use bbb_client_world::block_outline::{
    selection_outline_for_block, selection_outline_for_world_view, BlockOutlineTarget,
};

const DEFAULT_BLOCK_INTERACTION_RANGE: f64 = 4.5;
const DEFAULT_ENTITY_INTERACTION_RANGE: f64 = 3.0;
const DEBUG_LOOKING_AT_RANGE: f64 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CrosshairBlockHit {
    pub(crate) pos: BlockPos,
    pub(crate) face: ProtocolDirection,
    pub(crate) cursor: [f64; 3],
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BlockSelectionProjection {
    pub(crate) pos: BlockPos,
    pub(crate) boxes: Vec<SelectionBox>,
}

pub(crate) fn block_selection_from_hit(
    world: &WorldStore,
    hit: Option<CrosshairBlockHit>,
) -> Option<BlockSelectionProjection> {
    // Vanilla `shouldRenderBlockOutline`: the outline requires
    // `getCameraEntity() instanceof Player`. A camera following the player is
    // the player itself (`GameRenderer.tick` defaults the camera entity); only
    // a server `SetCamera` onto a non-player entity fails the check.
    let camera = world.local_player().camera;
    if !camera.follows_player
        && !camera.entity_id.is_some_and(|id| {
            world.entity_type_id(id)
                == Some(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID)
        })
    {
        return None;
    }
    let hit = hit?;
    if world.local_player_is_spectator() && !world.block_provides_menu(hit.pos) {
        return None;
    }
    if !world.world_border().contains_block_pos(hit.pos) {
        return None;
    }
    if world.local_player_is_adventure()
        && !world.local_selected_main_hand_can_break_block_in_adventure_mode(hit.pos)
        && !world.local_selected_main_hand_can_place_on_block_in_adventure_mode(hit.pos)
    {
        return None;
    }
    let outline = match world.block_view(hit.pos) {
        Some(block) => selection_outline_for_world_view(world, &block),
        None => Some(selection_outline_for_block(hit.pos)),
    }?;
    Some(BlockSelectionProjection {
        pos: hit.pos,
        boxes: outline.boxes,
    })
}

pub(crate) fn can_render_crosshair_for_spectator(
    world: &WorldStore,
    target: Option<CrosshairTarget>,
) -> bool {
    match target {
        Some(CrosshairTarget::Entity(hit)) => world.entity_provides_menu(hit.entity_id),
        Some(CrosshairTarget::Block(hit)) => world.block_provides_menu(hit.pos),
        None => false,
    }
}

pub(crate) fn debug_looking_at_block_hit_from_camera(
    world: &WorldStore,
    pose: Option<CameraPose>,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit_from_ray(
        crosshair_ray_from_camera_pose(pose?),
        DEBUG_LOOKING_AT_RANGE,
        |pos| {
            world
                .block_view(pos)
                .map(|block| BlockOutlineTarget::from_world_view(world, &block))
        },
    )
}

pub(crate) fn debug_looking_at_fluid_hit_from_camera(
    world: &WorldStore,
    pose: Option<CameraPose>,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit_from_ray(
        crosshair_ray_from_camera_pose(pose?),
        DEBUG_LOOKING_AT_RANGE,
        |pos| {
            let probe = world.block_view(pos)?;
            Some(BlockOutlineTarget::from_fluid_height(
                probe.fluid?.own_height(),
            ))
        },
    )
}

pub(crate) fn item_use_pov_block_hit_from_world(
    world: &WorldStore,
    pose: LocalPlayerPoseState,
    query: LocalItemUsePovQuery,
) -> Option<CrosshairBlockHit> {
    raycast_crosshair_block_hit_from_ray(
        crosshair_ray_from_player_pose(pose, world.local_player_eye_height(pose)),
        query.max_distance,
        |pos| {
            let probe = world.block_view(pos)?;
            let fluid_height = probe.fluid.and_then(|fluid| {
                let selected = match query.fluid_mode {
                    LocalItemUsePovFluidMode::None => false,
                    LocalItemUsePovFluidMode::SourceOnly => fluid.amount == 8 && !fluid.falling,
                    LocalItemUsePovFluidMode::Any => true,
                };
                if selected {
                    world
                        .fluid_state_and_height_at(pos)
                        .map(|(_, height)| height)
                } else {
                    None
                }
            });
            Some(match fluid_height {
                Some(height) => {
                    BlockOutlineTarget::from_world_view_with_fluid_height(world, &probe, height)
                }
                None => BlockOutlineTarget::from_world_view(world, &probe),
            })
        },
    )
}

// Vanilla `Attributes.BLOCK_INTERACTION_RANGE` / `ENTITY_INTERACTION_RANGE` max is 64.0;
// keep a defensive finite cap for any synthetic caller.
const MAX_INTERACTION_RANGE: f64 = 1_024.0;
const MAX_RAYCAST_GRID_STEPS: usize = 4_096;

fn block_interaction_range(world: &WorldStore) -> f64 {
    world
        .local_player_attribute_value(VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, MAX_INTERACTION_RANGE))
        .unwrap_or(DEFAULT_BLOCK_INTERACTION_RANGE)
}

fn entity_interaction_range(world: &WorldStore) -> f64 {
    world
        .local_player_attribute_value(VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, MAX_INTERACTION_RANGE))
        .unwrap_or(DEFAULT_ENTITY_INTERACTION_RANGE)
}

fn crosshair_block_hit_from_ray(
    world: &WorldStore,
    ray: CrosshairRay,
    max_distance: f64,
) -> Option<CrosshairBlockHit> {
    if max_distance <= 0.0 {
        return None;
    }
    raycast_crosshair_block_hit_from_ray(ray, max_distance, |pos| {
        world
            .block_view(pos)
            .map(|block| BlockOutlineTarget::from_world_view(world, &block))
    })
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CrosshairPick {
    pub(crate) block_hit: Option<CrosshairBlockHit>,
    pub(crate) target: Option<CrosshairTarget>,
}

pub(crate) fn crosshair_target_from_camera(
    world: &WorldStore,
    pose: Option<CameraPose>,
) -> Option<CrosshairTarget> {
    crosshair_pick_from_camera(world, pose).target
}

/// The camera ray may be interpolated; ProjectileUtil clips the current entity boxes.
pub(crate) fn crosshair_pick_from_camera(
    world: &WorldStore,
    pose: Option<CameraPose>,
) -> CrosshairPick {
    let Some(pose) = pose else {
        return CrosshairPick::default();
    };
    let ray = crosshair_ray_from_camera_pose(pose);
    let block_range = block_interaction_range(world);
    let entity_range = entity_interaction_range(world);
    let search_range = block_range.max(entity_range);
    let raw_block_hit = crosshair_block_hit_from_ray(world, ray, search_range);
    let block_hit = raw_block_hit
        .filter(|hit| distance_sq(ray.eye, block_hit_location(*hit)) <= block_range * block_range);
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.camera_entity_id();
    let target = crosshair_target_from_ray(
        world,
        ray,
        raw_block_hit,
        block_range,
        entity_range,
        |entity_id| {
            local_player_id.is_some_and(|id| id == entity_id)
                || camera_entity_id.is_some_and(|id| id == entity_id)
        },
    );
    CrosshairPick { block_hit, target }
}

/// Vanilla `DebugRenderer.getTargetedEntity(cameraEntity, 8)` probe used by
/// the brain debug renderer. Unlike the gameplay crosshair target, this ray is
/// not shortened by a nearer block or the normal interaction reach.
pub(crate) fn debug_targeted_entity_from_camera(
    world: &WorldStore,
    pose: Option<CameraPose>,
) -> Option<i32> {
    let local_player_id = world.local_player_id();
    let camera_entity_id = world.camera_entity_id();
    let mut excluded_entity_id = |entity_id| {
        local_player_id.is_some_and(|id| id == entity_id)
            || camera_entity_id.is_some_and(|id| id == entity_id)
    };
    crosshair_entity_hit_from_ray(
        world,
        crosshair_ray_from_camera_pose(pose?),
        8.0,
        &mut excluded_entity_id,
    )
    .map(|hit| hit.hit.entity_id)
}

#[cfg(test)]
pub(crate) fn crosshair_target_from_world(
    world: &WorldStore,
    pose: Option<LocalPlayerPoseState>,
) -> Option<CrosshairTarget> {
    let local_player_id = world.local_player_id();
    let pose = pose?;
    let ray = crosshair_ray_from_player_pose(pose, world.local_player_eye_height(pose));
    let block_range = block_interaction_range(world);
    let entity_range = entity_interaction_range(world);
    crosshair_target_from_ray(
        world,
        ray,
        crosshair_block_hit_from_ray(world, ray, block_range.max(entity_range)),
        block_range,
        entity_range,
        |entity_id| local_player_id.is_some_and(|id| id == entity_id),
    )
}

fn crosshair_target_from_ray<F>(
    world: &WorldStore,
    ray: CrosshairRay,
    raw_block_hit: Option<CrosshairBlockHit>,
    block_range: f64,
    entity_range: f64,
    mut excluded_entity_id: F,
) -> Option<CrosshairTarget>
where
    F: FnMut(i32) -> bool,
{
    let eye = ray.eye;
    if let Some(attack_range) = active_crosshair_attack_range(world, ray) {
        if let Some(target) = attack_range_target_from_ray(
            world,
            ray,
            attack_range,
            block_range,
            &mut excluded_entity_id,
        ) {
            return Some(target);
        }
    }
    let raw_block_distance_sq = raw_block_hit
        .map(|hit| distance_sq(eye, block_hit_location(hit)))
        .unwrap_or(f64::INFINITY);
    let entity_max_distance = raw_block_distance_sq.sqrt().min(entity_range);
    let entity_hit = if entity_max_distance > 0.0 {
        crosshair_entity_hit_from_ray(world, ray, entity_max_distance, &mut excluded_entity_id)
    } else {
        None
    };
    let in_range_block_hit =
        raw_block_hit.filter(|_| raw_block_distance_sq <= block_range * block_range);

    choose_crosshair_target_with_occlusion(
        in_range_block_hit,
        raw_block_distance_sq,
        entity_hit,
        entity_range,
    )
}

#[derive(Debug, Clone, Copy)]
struct CrosshairAttackRange {
    min_reach: f64,
    max_reach: f64,
    hitbox_margin: f64,
    movement_extension: f64,
}

fn active_crosshair_attack_range(
    world: &WorldStore,
    ray: CrosshairRay,
) -> Option<CrosshairAttackRange> {
    let range = world.local_active_item_attack_range()?;
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
    block_range: f64,
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
            .block_view(pos)
            .map(|block| BlockOutlineTarget::from_world_view(world, &block))
    });
    let block_distance = block_hit
        .map(|hit| distance_sq(eye, block_hit_location(hit)).sqrt())
        .unwrap_or(f64::INFINITY);
    if block_distance < attack_range.min_reach {
        return block_hit
            .filter(|_| block_distance < block_range)
            .map(CrosshairTarget::Block);
    }

    let entity_end_distance = block_distance.min(max_search_reach);
    let entity_hit = crosshair_entity_hit_between_from_ray(
        world,
        ray,
        attack_range.min_reach,
        entity_end_distance,
        attack_range.hitbox_margin,
        excluded_entity_id,
    );
    if let Some(entity_hit) = entity_hit {
        return Some(CrosshairTarget::Entity(entity_hit.hit));
    }

    block_hit
        .filter(|_| block_distance < block_range)
        .map(CrosshairTarget::Block)
}

#[cfg(test)]
fn choose_crosshair_target(
    eye: [f64; 3],
    block_hit: Option<CrosshairBlockHit>,
    entity_hit: Option<RaycastEntityHit>,
    entity_interaction_range: f64,
) -> Option<CrosshairTarget> {
    let block_distance_sq = block_hit
        .map(|hit| distance_sq(eye, block_hit_location(hit)))
        .unwrap_or(f64::INFINITY);
    choose_crosshair_target_with_occlusion(
        block_hit,
        block_distance_sq,
        entity_hit,
        entity_interaction_range,
    )
}

fn choose_crosshair_target_with_occlusion(
    block_hit: Option<CrosshairBlockHit>,
    occluding_block_distance_sq: f64,
    entity_hit: Option<RaycastEntityHit>,
    entity_interaction_range: f64,
) -> Option<CrosshairTarget> {
    match (block_hit, entity_hit) {
        (Some(_block), Some(entity)) if entity.distance_sq < occluding_block_distance_sq => {
            entity_in_interaction_range(entity, entity_interaction_range)
                .then_some(CrosshairTarget::Entity(entity.hit))
        }
        (Some(block), _) => Some(CrosshairTarget::Block(block)),
        (None, Some(entity)) if entity.distance_sq < occluding_block_distance_sq => {
            entity_in_interaction_range(entity, entity_interaction_range)
                .then_some(CrosshairTarget::Entity(entity.hit))
        }
        (None, _) => None,
    }
}

fn entity_in_interaction_range(entity: RaycastEntityHit, entity_interaction_range: f64) -> bool {
    entity.distance_sq < entity_interaction_range * entity_interaction_range
}

fn crosshair_entity_hit_from_ray<F>(
    world: &WorldStore,
    ray: CrosshairRay,
    max_distance: f64,
    excluded_entity_id: &mut F,
) -> Option<RaycastEntityHit>
where
    F: FnMut(i32) -> bool,
{
    let targets = world
        .entity_pick_targets()
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
    excluded_entity_id: &mut F,
) -> Option<RaycastEntityHit>
where
    F: FnMut(i32) -> bool,
{
    if max_distance < min_distance {
        return None;
    }
    let targets = world
        .entity_pick_targets()
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

#[cfg(test)]
fn raycast_crosshair_block<F>(
    pose: LocalPlayerPoseState,
    max_distance: f64,
    mut material_at: F,
) -> Option<BlockPos>
where
    F: FnMut(BlockPos) -> Option<bbb_client_world::TerrainMaterialClass>,
{
    raycast_crosshair_block_hit_from_ray(
        crosshair_ray_from_player_pose(pose, tests::eye_height(pose)),
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
        crosshair_ray_from_player_pose(pose, tests::eye_height(pose)),
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
    if !max_distance.is_finite()
        || max_distance <= 0.0
        || !eye.iter().all(|coord| coord.is_finite())
        || !direction.iter().all(|coord| coord.is_finite())
        || direction == [0.0, 0.0, 0.0]
    {
        return None;
    }

    let max_distance = max_distance.min(MAX_INTERACTION_RANGE);
    let mut pos = block_pos_containing_checked(eye)?;
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
    let mut steps = 0usize;
    while steps < MAX_RAYCAST_GRID_STEPS {
        let Some(step) = cursor.next_step(max_distance) else {
            break;
        };
        steps += 1;
        let Some(next_pos) = offset_block_pos_axis_checked(pos, step.axis, step.delta) else {
            break;
        };
        pos = next_pos;
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
            .block_view(pos)
            .map(|block| BlockOutlineTarget::from_world_view(world, &block))
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
        let prev = self.next_distance;
        let step = GridStep {
            axis,
            delta: self.delta,
            distance: prev,
        };
        let next = prev + self.distance_delta;
        self.next_distance = if next.is_finite() && next > prev {
            next
        } else {
            f64::INFINITY
        };
        step
    }
}

#[derive(Debug, Clone, Copy)]
struct GridStep {
    axis: u8,
    delta: i32,
    distance: f64,
}

fn offset_block_pos_axis_checked(pos: BlockPos, axis: u8, delta: i32) -> Option<BlockPos> {
    Some(match axis {
        0 => BlockPos {
            x: pos.x.checked_add(delta)?,
            ..pos
        },
        1 => BlockPos {
            y: pos.y.checked_add(delta)?,
            ..pos
        },
        _ => BlockPos {
            z: pos.z.checked_add(delta)?,
            ..pos
        },
    })
}

fn block_pos_containing_checked(point: [f64; 3]) -> Option<BlockPos> {
    let to_i32 = |v: f64| -> Option<i32> {
        let floored = v.floor();
        if !floored.is_finite() || floored < f64::from(i32::MIN) || floored > f64::from(i32::MAX) {
            return None;
        }
        Some(floored as i32)
    };
    Some(BlockPos {
        x: to_i32(point[0])?,
        y: to_i32(point[1])?,
        z: to_i32(point[2])?,
    })
}

fn block_hit_cursor(eye: [f64; 3], direction: [f64; 3], distance: f64, pos: BlockPos) -> [f64; 3] {
    [
        (eye[0] + direction[0] * distance) - f64::from(pos.x),
        (eye[1] + direction[1] * distance) - f64::from(pos.y),
        (eye[2] + direction[2] * distance) - f64::from(pos.z),
    ]
}

fn block_hit_location(hit: CrosshairBlockHit) -> [f64; 3] {
    [
        f64::from(hit.pos.x) + hit.cursor[0],
        f64::from(hit.pos.y) + hit.cursor[1],
        f64::from(hit.pos.z) + hit.cursor[2],
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

pub(crate) fn protocol_block_hit_result_from_crosshair_hit(
    hit: CrosshairBlockHit,
) -> ProtocolBlockHitResult {
    ProtocolBlockHitResult {
        pos: hit.pos,
        direction: hit.face,
        cursor_x: hit.cursor[0] as f32,
        cursor_y: hit.cursor[1] as f32,
        cursor_z: hit.cursor[2] as f32,
        inside: hit.inside,
        world_border_hit: false,
    }
}

pub(crate) fn local_item_use_pov_result_from_crosshair_hit(
    hit: CrosshairBlockHit,
) -> LocalItemUsePovResult {
    let [x, y, z] = block_hit_location(hit);
    LocalItemUsePovResult::Block {
        hit: protocol_block_hit_result_from_crosshair_hit(hit),
        location: ProtocolVec3d { x, y, z },
    }
}

fn eye_position_from_player_pose(pose: LocalPlayerPoseState, eye_height: f64) -> [f64; 3] {
    [
        pose.position.x,
        pose.position.y + eye_height,
        pose.position.z,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CrosshairRay {
    eye: [f64; 3],
    y_rot: f32,
    x_rot: f32,
}

fn crosshair_ray_from_player_pose(pose: LocalPlayerPoseState, eye_height: f64) -> CrosshairRay {
    CrosshairRay {
        eye: eye_position_from_player_pose(pose, eye_height),
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
mod tests;
