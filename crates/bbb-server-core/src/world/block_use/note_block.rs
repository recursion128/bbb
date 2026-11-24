//! `NoteBlock.useWithoutItem` (`NoteBlock.java:124-133`): the block retunes
//! and plays, and the player counts `tune_noteblock`.

use bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID;
use bbb_server_types::block_queries::block_type_of;
use bbb_server_types::stats::custom_stat;

use super::BlockUseContext;
use crate::block_behavior::note_block;
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn is_note_block(block_state_id: i32) -> bool {
    block_type_of(block_state_id) == Some(VANILLA_BLOCK_NOTE_BLOCK_ID)
}

/// The `without_item:` column.
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    let Some(player) = world.block_interaction_player(context.connection_id) else {
        return false;
    };
    let pos = context.update.hit.pos;
    let state = context.block_state_id;
    world.with_level(context.dimension, |level| {
        note_block::use_without_item(level, pos, state, player)
    });
    world.levels[context.connection_id].award_stat_once(
        &mut world.scoreboard,
        custom_stat("minecraft:tune_noteblock"),
    );
    true
}
