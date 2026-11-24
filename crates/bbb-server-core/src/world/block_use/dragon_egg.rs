//! `DragonEggBlock.useWithoutItem` teleport.

use crate::world::simulation_level::LevelCtx;
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_DRAGON_EGG_ID;
use bbb_protocol::packets::BlockPos;

use super::BlockUseContext;
use crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS;
use crate::fluid::below;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::interaction::WorldBorder;

/// `DragonEggBlock.teleport` search budget (`DragonEggBlock.java:55`).
const TELEPORT_TRIES: i32 = 1000;

/// The `matches:` column: `Blocks.DRAGON_EGG`.
pub(super) fn is_dragon_egg(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id) == Some(VANILLA_BLOCK_DRAGON_EGG_ID)
}

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_dragon_egg_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_dragon_egg_without_item(
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
    )
}

fn teleport(level: &mut LevelCtx<'_, '_>, border: WorldBorder, pos: BlockPos, state: i32) {
    for _ in 0..TELEPORT_TRIES {
        let dx = level.level_random().next_int(16) - level.level_random().next_int(16);
        let dy = level.level_random().next_int(8) - level.level_random().next_int(8);
        let dz = level.level_random().next_int(16) - level.level_random().next_int(16);
        let dest = BlockPos {
            x: pos.x + dx,
            y: pos.y + dy,
            z: pos.z + dz,
        };
        if block_state_is_air(level.block_state(dest))
            && !block_state_is_air(level.block_state(below(dest)))
            && border.contains_block(dest)
            && dest.y >= level.get_min_y()
            && dest.y <= level.get_max_y()
        {
            // `:73-74`: dest flags 2; `Level.removeBlock(pos, false)` is air flags 3.
            level.set_block(
                dest,
                state,
                SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            let _ = level.remove_block(pos);
            return;
        }
    }
}

impl ServerWorld {
    pub(in crate::world) fn use_dragon_egg_without_item(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
    ) -> bool {
        let dimension = dimension.into();
        let border = self.level_by_id(&dimension).world_border;
        self.with_level(dimension, |level| {
            teleport(level, border, pos, block_state_id);
        });
        true
    }
}
