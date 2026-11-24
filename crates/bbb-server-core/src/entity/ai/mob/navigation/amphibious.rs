use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::packets::BlockPos;

use super::{walk_path_type, GroundPathType};

// Frog.FrogNodeEvaluator.getPathType precedes AmphibiousNodeEvaluator.getPathType
// (Frog.java; AmphibiousNodeEvaluator.java).
pub(super) fn frog_path_type(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<GroundPathType> {
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    if !level.has_ticking_column_at(below) || !level.has_ticking_column_at(pos) {
        return None;
    }
    let below_state = level.block_state(below);
    let below_resource_id = vanilla_block_state_block_type_id(below_state)
        .and_then(vanilla_block_type)
        .map(|block| block.resource_id)
        .or_else(|| {
            level
                .content_registry()
                .block_by_state_id(below_state)
                .map(|(block, _)| block.resource_id())
        })?;
    if level.registry_tag_membership(
        "minecraft:block",
        "minecraft:frog_prefer_jump_to",
        below_resource_id,
    ) {
        return Some(GroundPathType::Open);
    }
    path_type(level, pos)
}

pub(super) fn path_type(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<GroundPathType> {
    if level.ground_path_type_from_state(pos)? != GroundPathType::Water {
        return walk_path_type(level, pos);
    }
    for (dx, dy, dz) in [
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
        (-1, 0, 0),
        (1, 0, 0),
    ] {
        let adjacent = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        if !level.has_ticking_column_at(adjacent) {
            return None;
        }
        if level.ground_path_type_from_state(adjacent)? == GroundPathType::Blocked {
            return Some(GroundPathType::WaterBorder);
        }
    }
    Some(GroundPathType::Water)
}
