//! `Entity.checkSupportingBlock` and the collision search it runs.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseWithoutItemOverride};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{visit_block_collisions_with_scratch, Aabb};
use std::cell::Cell;

use crate::entity::Entity;

pub(crate) fn set_on_ground_with_movement(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    on_ground: bool,
    horizontal_collision: bool,
    movement: Vec3d,
) {
    entity.on_ground = on_ground;
    entity.set_horizontal_collision(horizontal_collision);
    let _ = check_supporting_block(entity, level, on_ground, Some(movement), false);
}

pub(crate) fn set_on_ground_with_movement_checked(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    on_ground: bool,
    horizontal_collision: bool,
    movement: Vec3d,
) -> Option<()> {
    entity.on_ground = on_ground;
    entity.set_horizontal_collision(horizontal_collision);
    check_supporting_block(entity, level, on_ground, Some(movement), true)
}

fn check_supporting_block(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    on_ground: bool,
    movement: Option<Vec3d>,
    checked: bool,
) -> Option<()> {
    let unavailable = Cell::new(false);
    let mut supporting = entity.main_supporting_block_pos;
    let mut no_blocks = entity.on_ground_no_blocks;
    check_body_supporting_block_with_scratch(
        entity.position,
        entity.bounding_box,
        on_ground,
        movement,
        &mut supporting,
        &mut no_blocks,
        &|pos, out| {
            let start = out.len();
            if level
                .push_block_collision_boxes_for_entity(entity, pos, out)
                .is_none()
            {
                out.truncate(start);
                if checked {
                    unavailable.set(true);
                } else {
                    level.push_block_collision_boxes(pos, out);
                }
            }
        },
        &|pos| {
            level.is_moving_piston_at(pos).unwrap_or_else(|| {
                if checked {
                    unavailable.set(true);
                    false
                } else {
                    true
                }
            })
        },
    );
    if unavailable.get() {
        return None;
    }
    entity.main_supporting_block_pos = supporting;
    entity.on_ground_no_blocks = no_blocks;
    Some(())
}

pub fn set_spawn_on_ground(entity: &mut Entity, collision_boxes: &dyn Fn(BlockPos) -> Vec<Aabb>) {
    set_spawn_on_ground_with_moving_pistons(entity, collision_boxes, &|_| false);
}

pub fn set_spawn_on_ground_with_moving_pistons(
    entity: &mut Entity,
    collision_boxes: &dyn Fn(BlockPos) -> Vec<Aabb>,
    is_moving_piston_at: &dyn Fn(BlockPos) -> bool,
) {
    entity.on_ground = true;
    check_body_supporting_block_with_moving_pistons(
        entity.position,
        entity.bounding_box,
        true,
        None,
        &mut entity.main_supporting_block_pos,
        &mut entity.on_ground_no_blocks,
        collision_boxes,
        is_moving_piston_at,
    );
}

pub fn check_body_supporting_block(
    position: Vec3d,
    bounding_box: Aabb,
    on_ground: bool,
    movement: Option<Vec3d>,
    supporting: &mut Option<BlockPos>,
    on_ground_no_blocks: &mut bool,
    collision_boxes: &dyn Fn(BlockPos) -> Vec<Aabb>,
) {
    check_body_supporting_block_with_moving_pistons(
        position,
        bounding_box,
        on_ground,
        movement,
        supporting,
        on_ground_no_blocks,
        collision_boxes,
        &|_| false,
    );
}

// Multi-arg Vanilla `Entity.checkSupportingBlock` kernel with caller-supplied collision closures.
#[allow(clippy::too_many_arguments)]
pub fn check_body_supporting_block_with_moving_pistons(
    position: Vec3d,
    bounding_box: Aabb,
    on_ground: bool,
    movement: Option<Vec3d>,
    supporting: &mut Option<BlockPos>,
    on_ground_no_blocks: &mut bool,
    collision_boxes: &dyn Fn(BlockPos) -> Vec<Aabb>,
    is_moving_piston_at: &dyn Fn(BlockPos) -> bool,
) {
    check_body_supporting_block_with_scratch(
        position,
        bounding_box,
        on_ground,
        movement,
        supporting,
        on_ground_no_blocks,
        &|pos, out| out.extend(collision_boxes(pos)),
        is_moving_piston_at,
    );
}

#[allow(clippy::too_many_arguments)]
fn check_body_supporting_block_with_scratch(
    position: Vec3d,
    bounding_box: Aabb,
    on_ground: bool,
    movement: Option<Vec3d>,
    supporting: &mut Option<BlockPos>,
    on_ground_no_blocks: &mut bool,
    collision_boxes: &dyn Fn(BlockPos, &mut Vec<Aabb>),
    is_moving_piston_at: &dyn Fn(BlockPos) -> bool,
) {
    if !on_ground {
        *on_ground_no_blocks = false;
        *supporting = None;
        return;
    }
    let test_area = Aabb {
        min_y: bounding_box.min_y - 1.0e-6,
        max_y: bounding_box.min_y,
        ..bounding_box
    };
    let mut found =
        find_supporting_block(position, collision_boxes, is_moving_piston_at, test_area);
    if found.is_some() || *on_ground_no_blocks {
        *supporting = found;
    } else if let Some(movement) = movement {
        let shifted = test_area.moved(Vec3d {
            x: -movement.x,
            y: 0.0,
            z: -movement.z,
        });
        found = find_supporting_block(position, collision_boxes, is_moving_piston_at, shifted);
        *supporting = found;
    }
    *on_ground_no_blocks = found.is_none();
}

fn find_supporting_block(
    position: Vec3d,
    collision_boxes: &dyn Fn(BlockPos, &mut Vec<Aabb>),
    is_moving_piston_at: &dyn Fn(BlockPos) -> bool,
    box_to_test: Aabb,
) -> Option<BlockPos> {
    let mut best: Option<BlockPos> = None;
    let mut best_distance = f64::MAX;
    let _ = visit_block_collisions_with_scratch(
        box_to_test,
        is_moving_piston_at,
        collision_boxes,
        |pos, _| {
            let distance = dist_to_center_sqr(pos, position);
            let closer = distance < best_distance;
            let tie_break =
                distance == best_distance && best.is_some_and(|best| compare_pos(best, pos));
            if closer || tie_break {
                best = Some(pos);
                best_distance = distance;
            }
            true
        },
    );
    best
}

fn dist_to_center_sqr(pos: BlockPos, position: Vec3d) -> f64 {
    let dx = f64::from(pos.x) + 0.5 - position.x;
    let dy = f64::from(pos.y) + 0.5 - position.y;
    let dz = f64::from(pos.z) + 0.5 - position.z;
    dx * dx + dy * dy + dz * dz
}

fn compare_pos(left: BlockPos, right: BlockPos) -> bool {
    (left.y, left.z, left.x) < (right.y, right.z, right.x)
}

/// Dynamic tag membership comes from the caller's level.
#[derive(Debug, Clone, Copy)]
pub struct SupportingBlockFacts {
    pub state_id: i32,
    pub is_fence: bool,
    pub is_wall: bool,
}

fn floor_on_pos(position: Vec3d, offset: f32) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: (position.y - f64::from(offset)).floor() as i32,
        z: position.z.floor() as i32,
    }
}

pub(crate) fn body_get_on_pos(position: Vec3d, supporting: Option<BlockPos>) -> BlockPos {
    supporting.unwrap_or_else(|| floor_on_pos(position, 1.0e-5_f32))
}

pub fn body_on_pos(
    position: Vec3d,
    supporting: Option<BlockPos>,
    offset: f32,
    state_lookup: impl FnOnce(BlockPos) -> SupportingBlockFacts,
) -> BlockPos {
    let Some(supporting) = supporting else {
        return floor_on_pos(position, offset);
    };
    if offset.partial_cmp(&1.0e-5_f32) != Some(std::cmp::Ordering::Greater) {
        return supporting;
    }
    let facts = state_lookup(supporting);
    let is_gate = vanilla_block_state_block_type_id(facts.state_id)
        .and_then(vanilla_block_type)
        .is_some_and(|block| {
            block.use_without_item_override
                == Some(VanillaBlockUseWithoutItemOverride::FenceGateBlock)
        });
    if (offset <= 0.5 && facts.is_fence) || facts.is_wall || is_gate {
        supporting
    } else {
        BlockPos {
            y: (position.y - f64::from(offset)).floor() as i32,
            ..supporting
        }
    }
}

#[cfg(test)]
mod tests;
