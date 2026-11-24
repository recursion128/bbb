use super::*;

pub(super) fn find_supporting_block(
    world: &WorldStore,
    entity_position: [f64; 3],
    bounds: WorldCollisionBox,
    context: LocalPlayerCollisionContext,
) -> Option<BlockPos> {
    if !entity_position
        .iter()
        .all(|coordinate| coordinate.is_finite())
    {
        return None;
    }
    let test_bounds = WorldCollisionBox {
        min: [
            bounds.min[0],
            bounds.min[1] - SUPPORTING_BLOCK_PROBE_DEPTH,
            bounds.min[2],
        ],
        max: [bounds.max[0], bounds.min[1], bounds.max[2]],
    };
    let min_x = block_floor(test_bounds.min[0] - COLLISION_EPSILON).saturating_sub(1);
    let max_x = block_floor(test_bounds.max[0] + COLLISION_EPSILON).saturating_add(1);
    let min_y = block_floor(test_bounds.min[1] - COLLISION_EPSILON).saturating_sub(1);
    let max_y = block_floor(test_bounds.max[1] + COLLISION_EPSILON).saturating_add(1);
    let min_z = block_floor(test_bounds.min[2] - COLLISION_EPSILON).saturating_sub(1);
    let max_z = block_floor(test_bounds.max[2] + COLLISION_EPSILON).saturating_add(1);

    let mut main_support = None;
    let mut main_support_distance = f64::MAX;
    // Selection is order-independent because exact distance ties use `BlockPos.compareTo`.
    world.visit_loaded_chunk_block_positions([min_x, min_y, min_z], [max_x, max_y, max_z], |pos| {
        let Some(block) = world.block_view(pos) else {
            return;
        };
        let Some(shape) = block_collision_shape_with_context(world, &block, pos, context) else {
            return;
        };
        if !bounds_intersects_block_shape(test_bounds, pos, shape) {
            return;
        }
        let dx = f64::from(pos.x) + 0.5 - entity_position[0];
        let dy = f64::from(pos.y) + 0.5 - entity_position[1];
        let dz = f64::from(pos.z) + 0.5 - entity_position[2];
        let distance = dx * dx + dy * dy + dz * dz;
        let wins_tie = distance == main_support_distance
            && main_support.is_none_or(|current: BlockPos| {
                (current.y, current.z, current.x) < (pos.y, pos.z, pos.x)
            });
        if distance < main_support_distance || wins_tie {
            main_support = Some(pos);
            main_support_distance = distance;
        }
    });
    main_support
}

pub(super) fn non_climbable_floor_height(world: &WorldStore, pos: BlockPos) -> f64 {
    let shape_max_y = |probe: Option<BlockView>| {
        let probe = probe?;
        let name = probe.block_name?;
        if matches!(
            name,
            "minecraft:ladder"
                | "minecraft:vine"
                | "minecraft:scaffolding"
                | "minecraft:weeping_vines"
                | "minecraft:weeping_vines_plant"
                | "minecraft:twisting_vines"
                | "minecraft:twisting_vines_plant"
                | "minecraft:cave_vines"
                | "minecraft:cave_vines_plant"
        ) || (block_name_is_trapdoor(name)
            && probe.block_properties.get("open").map(String::as_str) == Some("true"))
        {
            return None;
        }
        block_collision_shape_empty_context(&probe, probe.pos).and_then(|shape| {
            shape
                .boxes()
                .map(|shape_box| shape_box.max[1])
                .reduce(f64::max)
        })
    };
    if let Some(max_y) = shape_max_y(world.block_view(pos)) {
        return max_y;
    }
    let below = BlockPos {
        y: pos.y.saturating_sub(1),
        ..pos
    };
    shape_max_y(world.block_view(below)).map_or(f64::NEG_INFINITY, |max_y| {
        if max_y >= 1.0 {
            max_y - 1.0
        } else {
            f64::NEG_INFINITY
        }
    })
}

pub(super) fn empty_context_block_collision(world: &WorldStore, bounds: WorldCollisionBox) -> bool {
    let min_x = block_floor(bounds.min[0] + COLLISION_EPSILON).saturating_sub(1);
    let max_x = block_floor(bounds.max[0] - COLLISION_EPSILON).saturating_add(1);
    let min_y = block_floor(bounds.min[1] + COLLISION_EPSILON).saturating_sub(1);
    let max_y = block_floor(bounds.max[1] - COLLISION_EPSILON).saturating_add(1);
    let min_z = block_floor(bounds.min[2] + COLLISION_EPSILON).saturating_sub(1);
    let max_z = block_floor(bounds.max[2] - COLLISION_EPSILON).saturating_add(1);
    let dimension = world.dimension();
    let min_y = min_y.max(dimension.min_y);
    let max_y = max_y.min(
        dimension
            .min_y
            .saturating_add(dimension.height)
            .saturating_sub(1),
    );
    if min_x > max_x || min_y > max_y || min_z > max_z {
        return false;
    }
    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let pos = BlockPos { x, y, z };
                let Some(block) = world.block_view(pos) else {
                    continue;
                };
                let shape = if block.block_name == Some("minecraft:moving_piston") {
                    moving_piston_collision_shape(
                        world,
                        &block,
                        pos,
                        LocalPlayerCollisionContext::for_bounds(bounds),
                        false,
                    )
                } else {
                    block_collision_shape_empty_context(&block, pos)
                };
                if shape.is_some_and(|shape| bounds_intersects_block_shape(bounds, pos, shape)) {
                    return true;
                }
            }
        }
    }
    false
}
