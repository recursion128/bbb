use crate::block_behavior::bed::is_bed;
use crate::chunk::ChunkStore;
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::entity_types::{vanilla_entity_type, VANILLA_ENTITY_TYPE_PLAYER_ID};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::block_queries::bool_property;
use bbb_server_types::geometry::{mth_cos, mth_sin, no_block_collision_with_moving_pistons, Aabb};

use crate::world::{movement_collision, natural_spawn, ServerWorld};

impl ServerWorld {
    pub(in crate::world) fn sleep_stand_up_position(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        entity_type_id: i32,
        bed_facing: Option<Direction>,
        yaw: f32,
    ) -> Option<Option<Vec3d>> {
        let level = self.level_by_id(dimension);
        stand_up_position(
            &level.chunks,
            level.world_border,
            self.game_time,
            pos,
            entity_type_id,
            bed_facing,
            yaw,
        )
    }
}

pub(in crate::world) fn stand_up_position(
    chunks: &ChunkStore,
    border: bbb_server_types::interaction::WorldBorder,
    game_time: i64,
    pos: BlockPos,
    entity_type_id: i32,
    bed_facing: Option<Direction>,
    yaw: f32,
) -> Option<Option<Vec3d>> {
    let dimensions = vanilla_entity_type(entity_type_id)?;
    let mut candidates = Vec::with_capacity(25);
    if let Some(facing) = bed_facing {
        let forward = match facing {
            Direction::North => [0, -1],
            Direction::South => [0, 1],
            Direction::West => [-1, 0],
            Direction::East => [1, 0],
            _ => return None,
        };
        if !yaw.is_finite() {
            return None;
        }
        let mut side = [-forward[1], forward[0]];
        // Direction.java:395-399: radians are rounded to float before the sine table.
        let radians = f64::from(yaw * (std::f64::consts::PI / 180.0) as f32);
        let facing_angle = side[0] as f32 * -mth_sin(radians) + side[1] as f32 * mth_cos(radians);
        if facing_angle > 0.0 {
            side = [-side[0], -side[1]];
        }
        let [fx, fz] = forward;
        let [sx, sz] = side;
        // BedBlock.java:325-338: surround candidates precede the two bed-top candidates.
        let surround = [
            [sx, sz],
            [sx - fx, sz - fz],
            [sx - fx * 2, sz - fz * 2],
            [-fx * 2, -fz * 2],
            [-sx - fx * 2, -sz - fz * 2],
            [-sx - fx, -sz - fz],
            [-sx, -sz],
            [-sx + fx, -sz + fz],
            [fx, fz],
            [sx + fx, sz + fz],
        ];
        candidates.extend(surround.map(|[x, z]| offset(pos, x, 0, z)));
        let below = offset(pos, 0, -1, 0);
        if is_bed(checked_state(chunks, below)?) {
            // BedBlock.java:239-267: a bunk visits the lower surround before either top.
            candidates.extend(surround.map(|[x, z]| offset(below, x, 0, z)));
        }
        candidates.extend([pos, offset(pos, -fx, 0, -fz)]);
    } else {
        // RespawnAnchorBlock.java:50-65: horizontal, below, above, then directly above.
        let horizontal = [
            [0, -1],
            [-1, 0],
            [0, 1],
            [1, 0],
            [-1, -1],
            [1, -1],
            [-1, 1],
            [1, 1],
        ];
        for y in [0, -1, 1] {
            candidates.extend(horizontal.map(|[x, z]| offset(pos, x, y, z)));
        }
        candidates.push(offset(pos, 0, 1, 0));
    }
    // Both owners exhaust safe candidates before repeating the complete unsafe walk.
    for check_dangerous in [true, false] {
        for candidate in &candidates {
            if let Some(position) = dismount_position(
                chunks,
                *candidate,
                entity_type_id,
                [dimensions.width, dimensions.height],
                check_dangerous,
            )? {
                let bounds = Aabb::of_entity(position, dimensions.width, dimensions.height);
                let epsilon = f64::from(1.0e-5_f32);
                // WorldBorder.java:60-73: maximum coordinates use a float epsilon.
                if bounds.min_x >= border.min_x_at(game_time)
                    && bounds.min_z >= border.min_z_at(game_time)
                    && bounds.max_x - epsilon < border.max_x_at(game_time)
                    && bounds.max_z - epsilon < border.max_z_at(game_time)
                {
                    return Some(Some(position));
                }
            }
        }
    }
    Some(None)
}

fn offset(pos: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: pos.x.wrapping_add(x),
        y: pos.y.wrapping_add(y),
        z: pos.z.wrapping_add(z),
    }
}

pub(in crate::world) fn checked_state(chunks: &ChunkStore, pos: BlockPos) -> Option<i32> {
    let state = if chunks.is_inside_build_height(pos.y) {
        chunks.get_block_state(pos.x, pos.y, pos.z)?
    } else {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_VOID_AIR_ID,
        )?
    };
    if vanilla_block_state_block_type_id(state).is_none() {
        chunks.content_registry().block_by_state_id(state)?;
    }
    Some(state)
}

fn tag_member(chunks: &ChunkStore, state: i32, tag: &str) -> Option<bool> {
    let block_id = vanilla_block_state_block_type_id(state).or_else(|| {
        chunks
            .content_registry()
            .block_by_state_id(state)
            .map(|(block, _)| block.block_id)
    })?;
    Some(
        chunks
            .registry_tags()
            .is_some_and(|tags| tags.contains_block(tag, block_id)),
    )
}

fn non_climbable_shape(chunks: &ChunkStore, pos: BlockPos) -> Option<Vec<Aabb>> {
    let state = checked_state(chunks, pos)?;
    let open_trapdoor = vanilla_block_state_block_type_id(state)
        .and_then(vanilla_block_type)
        .is_some_and(|block| {
            block.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::TrapDoorBlock)
                && bool_property(state, "open") == Some(true)
        });
    // DismountHelper.java:55-59: only the floor query ignores climbable and open trapdoor shapes.
    if tag_member(chunks, state, "minecraft:climbable")? || open_trapdoor {
        Some(Vec::new())
    } else {
        movement_collision::block_collision_shape_checked(chunks, pos, None)
    }
}

fn dangerous(chunks: &ChunkStore, state: i32, entity_type_id: i32) -> Option<bool> {
    // The shared hazard reader consumes the fire tag; absence is not a negative membership.
    tag_member(chunks, state, "minecraft:fire")?;
    Some(natural_spawn::is_block_dangerous(
        chunks,
        state,
        entity_type_id,
    ))
}

fn dismount_position(
    chunks: &ChunkStore,
    pos: BlockPos,
    entity_type_id: i32,
    dimensions: [f32; 2],
    check_dangerous: bool,
) -> Option<Option<Vec3d>> {
    let state = checked_state(chunks, pos)?;
    if check_dangerous && dangerous(chunks, state, entity_type_id)? {
        return Some(None);
    }
    let shape = non_climbable_shape(chunks, pos)?;
    // BlockGetter.java:96-102: a partial block below is not a floor for the empty current block.
    let floor = if shape.is_empty() {
        let below = non_climbable_shape(chunks, offset(pos, 0, -1, 0))?;
        let top = below
            .iter()
            .map(|shape| shape.max_y)
            .fold(f64::NEG_INFINITY, f64::max);
        if top >= 1.0 {
            top - 1.0
        } else {
            f64::NEG_INFINITY
        }
    } else {
        shape
            .iter()
            .map(|shape| shape.max_y)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    if !floor.is_finite() || floor >= 1.0 {
        return Some(None);
    }
    if check_dangerous
        && floor <= 0.0
        && dangerous(
            chunks,
            checked_state(chunks, offset(pos, 0, -1, 0))?,
            entity_type_id,
        )?
    {
        return Some(None);
    }
    let position = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + floor,
        z: f64::from(pos.z) + 0.5,
    };
    let bounds = Aabb::of_entity(position, dimensions[0], dimensions[1]);
    if !no_block_collision_with_moving_pistons(
        bounds,
        |pos| movement_collision::moving_piston_at(chunks, pos),
        |pos| {
            checked_state(chunks, pos)?;
            movement_collision::block_collision_shape_checked(chunks, pos, None)
        },
    )? {
        return Some(None);
    }
    if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
        && (tag_member(chunks, state, "minecraft:invalid_spawn_inside")?
            || tag_member(
                chunks,
                checked_state(chunks, offset(pos, 0, 1, 0))?,
                "minecraft:invalid_spawn_inside",
            )?)
    {
        return Some(None);
    }
    Some(Some(position))
}

#[cfg(test)]
mod tests;
