use super::LevelCtx;
use crate::fluid::with_property;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::packets::BlockPos;

use super::{block_type_of, bool_property, BlockPlacedBy, SET_BLOCK_UPDATE_CLIENTS};

const HAS_RECORD: &str = "has_record";

/// `JukeboxBlock.useItemOn` is unique to that class.
#[must_use]
pub fn is_jukebox(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::JukeboxBlock)
}

#[must_use]
pub fn has_record(block_state_id: i32) -> bool {
    bool_property(block_state_id, HAS_RECORD).unwrap_or(false)
}

pub(super) fn set_placed_by(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    placed_by: &BlockPlacedBy<'_>,
) {
    let has_record_item = is_jukebox(block_state_id)
        && placed_by
            .block_entity_data
            .is_some_and(|data| data.iter().any(|entry| entry.name == "RecordItem"));
    if let Some(state) = has_record_item
        .then(|| with_has_record(block_state_id, true))
        .flatten()
    {
        level.set_block(
            pos,
            state,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}

#[must_use]
pub fn signal(playing: bool) -> i32 {
    if playing {
        crate::redstone::SIGNAL_MAX
    } else {
        0
    }
}

#[must_use]
pub fn with_has_record(block_state_id: i32, has_record: bool) -> Option<i32> {
    with_property(
        block_state_id,
        HAS_RECORD,
        if has_record { "true" } else { "false" },
    )
}
