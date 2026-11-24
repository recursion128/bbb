//! `FenceBlock.useWithoutItem` leash bind.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseWithoutItemOverride};

use super::lead;
use super::BlockUseContext;
use crate::world::{ServerWorld, WorldEvent};

/// The `matches:` column: the generated `FenceBlock` `useWithoutItem` owner.
pub(super) fn is_fence(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::FenceBlock)
}

/// `FenceBlock.useWithoutItem` (`FenceBlock.java:71-75`): server
/// `LeadItem.bindPlayerMobs` (`LeadItem.java:37-65`); client PASS.
/// Bound consumes the click; there is no item, so no `ITEM_USED`.
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    lead::bind_player_mobs_at(world, context.connection_id, context.update.hit.pos, events)
}
