//! `CakeBlock` (`CakeBlock.java`): a candle on an uneaten cake, and the bite
//! a hungry player takes.

use crate::world::LevelId;
use bbb_protocol::block_types::VANILLA_BLOCK_CAKE_ID;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::BlockUseContext;
use crate::block_behavior::cake as cake_block;
use crate::block_behavior::{default_block_state_id, BlockUseResult};
use crate::connection::ConnectionId;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};

/// The `item_on:` column: `CakeBlock.useItemOn` (`CakeBlock.java:55-75`).
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    let item = context.selected_item.item_id();
    let is_candle = context.selected_item.item_resource_id().is_some_and(|id| {
        world
            .registries
            .registry_tags()
            .contains("minecraft:item", cake_block::CANDLES_TAG, id)
    });
    let candle_cake_id = item
        .filter(|_| is_candle)
        .and_then(cake_block::candle_cake_block_type_id);
    let (Some(item), Some(candle_cake_id)) = (item, candle_cake_id) else {
        return BlockUseResult::TryWithEmptyHand;
    };
    if cake_block::bites(context.block_state_id) != 0 {
        return BlockUseResult::TryWithEmptyHand;
    }
    let pos = context.update.hit.pos;
    let player = &mut world.levels[context.connection_id];
    let infinite = player.abilities.instabuild;
    player
        .inventory
        .consume_in_hand(context.update.hand, 1, infinite);
    player.award_stat_once(
        &mut world.scoreboard,
        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item),
    );
    let source = player.entity_id();
    world.play_block_sound(
        context.dimension,
        pos,
        "minecraft:block.cake.add_candle",
        events,
    );
    let state = default_block_state_id(candle_cake_id);
    world.set_block(context.dimension, pos, state, SET_BLOCK_UPDATE_ALL);
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source(VanillaGameEvent::BLOCK_CHANGE, pos, Some(source), None);
    BlockUseResult::Consumed
}

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_cake_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_cake_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
    )
}

impl ServerWorld {
    pub(super) fn use_cake_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
    ) -> bool {
        let dimension = dimension.into();
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if !(player.abilities.invulnerable || player.food.needs_food()) {
            return false;
        }
        player.award_stat_once(
            &mut self.scoreboard,
            bbb_server_types::stats::custom_stat("minecraft:eat_cake_slice"),
        );
        player.food.eat(
            cake_block::EAT_NUTRITION,
            cake_block::EAT_SATURATION_MODIFIER,
        );
        let source_entity_id = Some(player.entity_id());
        self.with_level(dimension, |level| {
            cake_block::apply_eat(level, pos, block_state_id, source_entity_id);
        });
        true
    }

    pub(super) fn use_candle_cake_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        candle_cake_state: i32,
    ) -> bool {
        let dimension = dimension.into();
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if !(player.abilities.invulnerable || player.food.needs_food()) {
            return false;
        }
        player.award_stat_once(
            &mut self.scoreboard,
            bbb_server_types::stats::custom_stat("minecraft:eat_cake_slice"),
        );
        player.food.eat(
            cake_block::EAT_NUTRITION,
            cake_block::EAT_SATURATION_MODIFIER,
        );
        let source_entity_id = Some(player.entity_id());
        self.with_level(dimension, |level| {
            cake_block::apply_eat(
                level,
                pos,
                default_block_state_id(VANILLA_BLOCK_CAKE_ID),
                source_entity_id,
            );
            level.drop_resources(pos, candle_cake_state);
        });
        true
    }
}
