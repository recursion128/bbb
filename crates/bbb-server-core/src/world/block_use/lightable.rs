//! Candle, candle-cake and campfire `useItemOn`: empty-hand candle dousing and
//! campfire food. Lighting and shovel dousing are the items' own `useOn`.

use bbb_protocol::item_types::{VANILLA_ITEM_FIRE_CHARGE_ID, VANILLA_ITEM_FLINT_AND_STEEL_ID};
use bbb_protocol::packets::ItemStackSummary;

use super::BlockUseContext;
use crate::block_behavior::candle as candle_block;
use crate::block_behavior::BlockUseResult;
use crate::block_entity::BlockEntity;
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::stats::custom_stat;

/// The `item_on:` column for [`candle_block::is_candle`].
pub(super) fn candle_use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_candle_item_on(context)
}

/// The `item_on:` column for candle cakes.
pub(super) fn candle_cake_use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_candle_cake_item_on(context)
}

/// The `without_item:` column for candle cakes.
pub(super) fn use_candle_cake_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_candle_cake_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
    )
}

/// The `item_on:` column for [`crate::block_behavior::campfire::is_campfire`].
pub(super) fn campfire_use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_campfire_item_on(context)
}

impl ServerWorld {
    /// `CandleBlock.useItemOn` (`CandleBlock.java:80-95`).
    fn use_candle_item_on(&mut self, context: &BlockUseContext<'_>) -> BlockUseResult {
        let pos = context.update.hit.pos;
        let state = context.block_state_id;
        if context.selected_item.is_empty() {
            let player = self
                .levels
                .player(context.connection_id)
                .expect("connection presence was checked");
            let source_entity_id = Some(player.entity_id());
            if player.may_build() && candle_block::is_lit(state) {
                self.with_level(context.dimension, |level| {
                    candle_block::extinguish(level, pos, state, source_entity_id);
                });
                return BlockUseResult::Consumed;
            }
        }
        BlockUseResult::TryWithEmptyHand
    }

    /// `CandleCakeBlock.useItemOn` (`CandleCakeBlock.java:70-87`). Flint and
    /// fire charge `PASS` so the item lights; an empty hand on the candle
    /// (`:101-103`) douses. Eating stays on `useWithoutItem`.
    fn use_candle_cake_item_on(&mut self, context: &BlockUseContext<'_>) -> BlockUseResult {
        let pos = context.update.hit.pos;
        let state = context.block_state_id;
        let item_id = context.selected_item.item_id();
        if matches!(
            item_id,
            Some(VANILLA_ITEM_FLINT_AND_STEEL_ID | VANILLA_ITEM_FIRE_CHARGE_ID)
        ) {
            return BlockUseResult::Pass;
        }
        if context.selected_item.is_empty()
            && candle_hit(context.update.hit.cursor_y)
            && candle_block::is_lit(state)
        {
            let source_entity_id = self.player_source_entity_id(context);
            self.with_level(context.dimension, |level| {
                candle_block::extinguish(level, pos, state, source_entity_id);
            });
            return BlockUseResult::Consumed;
        }
        BlockUseResult::TryWithEmptyHand
    }

    /// `CampfireBlock.useItemOn` (`CampfireBlock.java:90-111`): placing food.
    fn use_campfire_item_on(&mut self, context: &BlockUseContext<'_>) -> BlockUseResult {
        if let Some(placed) = self.place_campfire_food(context) {
            if placed {
                // `:103`.
                self.levels[context.connection_id].award_stat_once(
                    &mut self.scoreboard,
                    custom_stat("minecraft:interact_with_campfire"),
                );
                // `:104`'s `SUCCESS_SERVER`; `:107` is `CONSUME`.
                self.mark_use_item_on_server_swing();
            }
            return BlockUseResult::Consumed;
        }
        BlockUseResult::TryWithEmptyHand
    }

    /// `CampfireBlockEntity.placeFood` (`CampfireBlockEntity.java:167-187`)
    /// behind the two tests that guard it (`CampfireBlock.java:99-101`).
    ///
    /// `None` is Vanilla falling out of either -- the block carries no campfire
    /// block entity, or no campfire recipe accepts the held item -- which
    /// leaves the item legs to answer the use.
    fn place_campfire_food(&mut self, context: &BlockUseContext<'_>) -> Option<bool> {
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        self.level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::campfire)?;
        let selected_item = context.projected_item()?;
        let cooking_time = self.campfire_cooking_time(selected_item)?;
        let player = self
            .levels
            .player(context.connection_id)
            .expect("connection presence was checked");
        let has_infinite_materials = player.abilities.instabuild;
        let source_entity_id = player.entity_id();
        let placed = crate::world::using_item::runtime_item_in_hand(
            self.levels.player_at(context.connection_id),
            context.update.hand,
        )
        .with_count(1);
        let campfire = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::campfire_mut)
            .expect("the campfire block entity was just read");
        // `:168-170`: first empty slot. None left is `false` so the click
        // stays `CONSUME` (`:107`, `:186`) instead of falling through to the
        // item legs.
        let Some(slot) = campfire
            .items()
            .iter()
            .position(bbb_server_types::item_stack::RuntimeItemStack::is_empty)
        else {
            return Some(false);
        };
        // `:179`: `consumeAndReturn` puts one item in the slot and takes one
        // out of the hand, which for a creative session is only the first half.
        campfire.set_cooking(slot, 0, cooking_time);
        campfire.set_slot(slot, placed);
        self.levels[context.connection_id]
            .inventory
            .consume_in_hand(context.update.hand, 1, has_infinite_materials);
        let level = self.level_mut_by_id(&dimension);
        // `:180`.
        level.event_sink.game_event_with_source(
            VanillaGameEvent::BLOCK_CHANGE,
            pos,
            Some(source_entity_id),
            Some(context.block_state_id),
        );
        level.chunks.block_entities_mut().set_changed(pos);
        // `markUpdated` (`:189-192`) sends a block update over the unchanged
        // state, which is what carries the block entity to the sessions
        // watching the chunk.
        level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
        Some(true)
    }

    /// `RecipePropertySet.CAMPFIRE_INPUT` (`CampfireBlock.java:101`) and the
    /// recipe `placeFood` reads the cooking time off (`:171-177`) are one scan
    /// rather than two that agree: the property set's members are the flattened
    /// item sets of the very campfire ingredients this walks
    /// (`RecipePropertySet.java:46-48` over `RecipeManager.java:54-55`), an
    /// `Ingredient` is a bare `HolderSet<Item>` whose `test` is that same
    /// membership (`Ingredient.java:63-65`), and the recipe's own `matches` is
    /// its ingredient alone (`SingleItemRecipe.java:30-32`). Empty falls out of
    /// both, air being the one item no ingredient may hold
    /// (`RecipeMap.java:48-50`, `Ingredient.java:43-45`).
    fn campfire_cooking_time(&self, item: &ItemStackSummary) -> Option<i32> {
        // `CampfireBlock.java:101` and `:171-177` are one lookup: the property
        // set's members are the flattened item sets of these very ingredients
        // (`RecipePropertySet.java:46-48`), and the recipe's own `matches` is
        // its ingredient alone (`SingleItemRecipe.java:30-32`).
        self.campfire_recipe(item).map(|recipe| recipe.cooking_time)
    }

    fn player_source_entity_id(&self, context: &BlockUseContext<'_>) -> Option<i32> {
        self.levels
            .player(context.connection_id)
            .map(|player| player.entity_id())
    }
}

/// `CandleCakeBlock.candleHit` (`CandleCakeBlock.java:101-103`).
fn candle_hit(cursor_y: f32) -> bool {
    cursor_y > 0.5
}
