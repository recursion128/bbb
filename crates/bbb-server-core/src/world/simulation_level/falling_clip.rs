use crate::entity::Entity;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::geometry::clip::clip_with;

use super::LevelCtx;

pub(super) fn collider_source_only(
    level: &LevelCtx<'_, '_>,
    entity: &Entity,
    from: Vec3d,
    to: Vec3d,
) -> Option<Option<BlockPos>> {
    let chunks = &*level.chunks;
    let hit = clip_with(from, to, |pos| {
        let state = super::super::level_blocks::stored_block_state(chunks, pos);
        if vanilla_block_state_block_type_id(state).is_none() {
            chunks.content_registry().block_by_state_id(state)?;
        }
        let block = super::super::movement_collision::falling_block_collision_shape(
            chunks,
            entity,
            pos,
            super::super::movement_collision::current_piston_no_clip(),
        )?;
        let fluid = super::super::pov_clip::source_fluid_shape(
            chunks,
            pos,
            block_state_fluid_state(state),
        )?;
        Some((block, fluid))
    })?;
    Some(hit.map(|hit| hit.pos))
}
