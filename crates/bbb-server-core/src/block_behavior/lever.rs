//! `LeverBlock` interaction, neighbour fan-out, and redstone members.

use super::LevelCtx;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockRedstoneSignalOverride};
use bbb_protocol::packets::BlockPos;

use super::{block_type_of, bool_property, footing};

const POWERED: &str = "powered";

#[must_use]
pub fn is_lever(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.signal_override)
        == Some(VanillaBlockRedstoneSignalOverride::LeverBlock)
}

pub(super) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) -> bool {
    pull(level, pos, block_state_id);
    true
}

pub(super) fn pull(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(next) = with_property(
        block_state_id,
        POWERED,
        if powered(block_state_id) {
            "false"
        } else {
            "true"
        },
    ) else {
        return;
    };
    level.set_block(
        pos,
        next,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    footing::face_attached_update_neighbours(level, next, pos);
    level.play_block_sound_with_volume_pitch(
        pos,
        "minecraft:block.lever.click",
        0.3,
        if powered(next) { 0.6 } else { 0.5 },
    );
    level.block_game_event(
        if powered(next) {
            crate::level_event::VanillaGameEvent::BLOCK_ACTIVATE
        } else {
            crate::level_event::VanillaGameEvent::BLOCK_DEACTIVATE
        },
        pos,
        None,
    );
}

#[must_use]
fn powered(block_state_id: i32) -> bool {
    bool_property(block_state_id, POWERED).unwrap_or(false)
}
