//! `ComposterBlock` (`ComposterBlock.java`): the compostable a click adds
//! and the bone meal a ready composter gives back.

use crate::world::simulation_level::LevelCtx;
use crate::world::LevelId;
use bbb_protocol::packets::{
    BlockPos, DataComponentPatchSummary, InteractionHand, ItemStackSummary, Vec3d,
};

use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::BlockUseContext;
use crate::block_behavior::composter as composter_block;
use crate::block_behavior::BlockUseResult;
use crate::connection::ConnectionId;
use crate::world::{ServerWorld, WorldEvent};

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_composter_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_composter_item_on(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        &context.selected_item,
        context.update.hand,
    )
}

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_composter_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_composter_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        events,
    )
}

impl ServerWorld {
    pub(super) fn use_composter_item_on(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        selected_item: &bbb_server_types::item_stack::RuntimeItemStack,
        hand: InteractionHand,
    ) -> BlockUseResult {
        let dimension = dimension.into();
        let fill = composter_block::fill_level(block_state_id);
        if fill >= composter_block::READY
            || !composter_block::is_compostable(selected_item.item_id())
        {
            return BlockUseResult::TryWithEmptyHand;
        }
        if fill < composter_block::MAX_LEVEL {
            let item_id = selected_item.item_id().expect("compostable has an item");
            let before = block_state_id;
            let source_entity_id = self
                .levels
                .player(connection_id)
                .map(|player| player.entity_id());
            self.with_level(dimension, |level| {
                let after =
                    composter_block::add_item(level, pos, before, item_id, source_entity_id);
                LevelCtx::level_event(
                    level,
                    composter_block::FILL_LEVEL_EVENT,
                    pos,
                    i32::from(after != before),
                );
            });
            // `ComposterBlock.java:262`.
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
            let instabuild = self.levels[connection_id].abilities.instabuild;
            if !instabuild {
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("connection presence was checked");
                player.inventory.consume_in_hand(hand, 1, false);
            }
        }
        BlockUseResult::Consumed
    }

    pub(super) fn use_composter_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if composter_block::fill_level(block_state_id) != composter_block::READY {
            return false;
        }
        let source_entity_id = self
            .levels
            .player(connection_id)
            .map(|player| player.entity_id());
        let stack =
            bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BONE_MEAL_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            });
        let spawned = self.with_level(dimension, |level| {
            let dx = (level.level_random().next_float() - 0.5) * 0.7;
            let dz = (level.level_random().next_float() - 0.5) * 0.7;
            let position = Vec3d {
                x: f64::from(pos.x) + 0.5 + f64::from(dx),
                y: f64::from(pos.y) + 1.01,
                z: f64::from(pos.z) + 0.5 + f64::from(dz),
            };
            if let Err(error) = LevelCtx::spawn_block_item(level, position, stack) {
                LevelCtx::record_item_failure(level, pos, error);
                return false;
            }
            composter_block::empty(level, pos, block_state_id, source_entity_id);
            true
        });
        if !spawned {
            return false;
        }
        self.play_queued_composter_empty(dimension, pos, events);
        true
    }

    pub(super) fn play_queued_composter_empty(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        self.play_block_sound(dimension, pos, composter_block::EMPTY_SOUND, events);
    }
}
