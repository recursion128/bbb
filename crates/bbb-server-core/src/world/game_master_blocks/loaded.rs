use crate::block_behavior::command_block;
use bbb_protocol::packets::BlockPos;

use crate::world::simulation_level::LevelCtx;

pub(in crate::world) fn after_load(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    previous_automatic: Option<bool>,
) {
    let Some(entity) = level.chunks.block_entities().get(pos) else {
        return;
    };
    let automatic = entity.command_block().map(|block| block.automatic());
    let mode = entity.structure_block().map(|block| block.mode());
    let state = level.block_state(pos);
    if let Some(automatic) = automatic {
        if let Some(block) = level.command_block_mut(pos) {
            block.set_automatic(previous_automatic.unwrap_or(false));
        }
        command_block::set_automatic(level, pos, state, automatic);
    }
    if let Some(mode) = mode {
        let name = match mode {
            bbb_protocol::packets::StructureBlockMode::Save => "save",
            bbb_protocol::packets::StructureBlockMode::Load => "load",
            bbb_protocol::packets::StructureBlockMode::Corner => "corner",
            bbb_protocol::packets::StructureBlockMode::Data => "data",
        };
        if let Some(updated) = crate::fluid::with_property(state, "mode", name) {
            level.set_block(pos, updated, 2, crate::neighbor_updater::UPDATE_LIMIT);
        }
    }
}
