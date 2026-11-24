//! `RedStoneOreBlock.useItemOn` (`RedStoneOreBlock.java:55-71`): the server
//! lights an unlit ore; a held `BlockItem` whose `canPlace` is true PASSes.

use super::BlockUseContext;
use crate::block_behavior::redstone_ore as redstone_ore_block;
use crate::block_behavior::BlockUseResult;
use crate::world::{ServerWorld, WorldEvent};

/// The `item_on:` column: [`ServerWorld::use_redstone_ore_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_redstone_ore_item_on(context)
}

impl ServerWorld {
    /// `RedStoneOreBlock.useItemOn` (`RedStoneOreBlock.java:55-71`) plus
    /// `interact` (`:74-78`).
    pub(super) fn use_redstone_ore_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
    ) -> BlockUseResult {
        let resolved = self.with_level(context.dimension, |level| {
            redstone_ore_block::interact(level, context.update.hit.pos, context.block_state_id)
        });
        if resolved.is_none() {
            return BlockUseResult::Pass;
        }
        // `:69-71`: only a held `BlockItem` whose `canPlace` succeeds PASSes.
        let can_place = context.selected_item.item_id().is_some_and(|item_id| {
            let is_block_item = bbb_protocol::item_types::vanilla_item_type(item_id).is_some_and(
                |entry| {
                    entry.block_item_behavior.is_some()
                        || matches!(
                            entry.use_on_override,
                            Some(
                                bbb_protocol::item_types::VanillaItemUseOnOverride::SolidBucketItem
                                    | bbb_protocol::item_types::VanillaItemUseOnOverride::PlaceOnWaterBlockItem
                            )
                        )
                },
            );
            is_block_item
                && matches!(
                    self.block_placement_context(
                        context.connection_id,
                        context.dimension,
                        context.update,
                        item_id,
                    ),
                    Ok(Some(_))
                )
        });
        if can_place {
            BlockUseResult::Pass
        } else {
            BlockUseResult::Consumed
        }
    }
}
