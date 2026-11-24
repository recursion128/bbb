use super::{BlockShapeCtx, LevelCtx};

use bbb_protocol::block_types::VanillaCanSurviveOverride;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{below, with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::fluid::block_state_is_solid;

use super::block_type_of;
use super::{can_survive_override, int_property, BlockStateAt};

const BITES: &str = "bites";
/// `CakeBlock.MAX_BITES` (`CakeBlock.java:34`).
pub const MAX_BITES: i32 = 6;
/// `CakeBlock.eat`'s `FoodData.eat` arguments (`CakeBlock.java:100`).
pub const EAT_NUTRITION: i32 = 2;
pub const EAT_SATURATION_MODIFIER: f32 = 0.1;

/// `CakeBlock.canSurvive` is unique to that class.
#[must_use]
pub fn is_cake(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(can_survive_override)
        == Some(VanillaCanSurviveOverride::CakeBlock)
}

#[must_use]
pub fn analog_output(block_state_id: i32) -> i32 {
    (7 - bites(block_state_id)) * 2
}

#[must_use]
pub(crate) fn bites(block_state_id: i32) -> i32 {
    int_property(block_state_id, BITES).unwrap_or(0)
}

#[must_use]
pub fn can_survive(pos: BlockPos, block_state_at: BlockStateAt<'_>) -> bool {
    block_state_is_solid(block_state_at(below(pos)))
}

pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if direction_to_neighbour == Direction::Down
        && !can_survive(pos, &mut |at| level.block_state(at))
    {
        super::air_block_state_id()
    } else {
        block_state_id
    }
}

pub(crate) fn apply_eat(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) {
    let bites = bites(block_state_id);
    level.game_event_from_entity(VanillaGameEvent::EAT, pos, source_entity_id, None);
    if bites < MAX_BITES {
        let Some(next) = with_property(block_state_id, BITES, &(bites + 1).to_string()) else {
            return;
        };
        level.set_block(
            pos,
            next,
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    } else {
        level.set_block(
            pos,
            super::air_block_state_id(),
            SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.game_event_from_entity(VanillaGameEvent::BLOCK_DESTROY, pos, source_entity_id, None);
    }
}

/// `ItemTags.CANDLES` (`CakeBlock.java:65`).
pub(crate) const CANDLES_TAG: &str = "minecraft:candles";

/// `CandleCakeBlock.byCandle` (`CandleCakeBlock.java:151-153`): static mapping
/// from a candle item id to its corresponding candle-cake block type id.
#[must_use]
pub(crate) fn candle_cake_block_type_id(candle_item_id: i32) -> Option<i32> {
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_BLACK_CANDLE_CAKE_ID, VANILLA_BLOCK_BLUE_CANDLE_CAKE_ID,
        VANILLA_BLOCK_BROWN_CANDLE_CAKE_ID, VANILLA_BLOCK_CANDLE_CAKE_ID,
        VANILLA_BLOCK_CYAN_CANDLE_CAKE_ID, VANILLA_BLOCK_GRAY_CANDLE_CAKE_ID,
        VANILLA_BLOCK_GREEN_CANDLE_CAKE_ID, VANILLA_BLOCK_LIGHT_BLUE_CANDLE_CAKE_ID,
        VANILLA_BLOCK_LIGHT_GRAY_CANDLE_CAKE_ID, VANILLA_BLOCK_LIME_CANDLE_CAKE_ID,
        VANILLA_BLOCK_MAGENTA_CANDLE_CAKE_ID, VANILLA_BLOCK_ORANGE_CANDLE_CAKE_ID,
        VANILLA_BLOCK_PINK_CANDLE_CAKE_ID, VANILLA_BLOCK_PURPLE_CANDLE_CAKE_ID,
        VANILLA_BLOCK_RED_CANDLE_CAKE_ID, VANILLA_BLOCK_WHITE_CANDLE_CAKE_ID,
        VANILLA_BLOCK_YELLOW_CANDLE_CAKE_ID,
    };
    use bbb_protocol::item_types::{
        VANILLA_ITEM_BLACK_CANDLE_ID, VANILLA_ITEM_BLUE_CANDLE_ID, VANILLA_ITEM_BROWN_CANDLE_ID,
        VANILLA_ITEM_CANDLE_ID, VANILLA_ITEM_CYAN_CANDLE_ID, VANILLA_ITEM_GRAY_CANDLE_ID,
        VANILLA_ITEM_GREEN_CANDLE_ID, VANILLA_ITEM_LIGHT_BLUE_CANDLE_ID,
        VANILLA_ITEM_LIGHT_GRAY_CANDLE_ID, VANILLA_ITEM_LIME_CANDLE_ID,
        VANILLA_ITEM_MAGENTA_CANDLE_ID, VANILLA_ITEM_ORANGE_CANDLE_ID, VANILLA_ITEM_PINK_CANDLE_ID,
        VANILLA_ITEM_PURPLE_CANDLE_ID, VANILLA_ITEM_RED_CANDLE_ID, VANILLA_ITEM_WHITE_CANDLE_ID,
        VANILLA_ITEM_YELLOW_CANDLE_ID,
    };
    Some(match candle_item_id {
        VANILLA_ITEM_CANDLE_ID => VANILLA_BLOCK_CANDLE_CAKE_ID,
        VANILLA_ITEM_WHITE_CANDLE_ID => VANILLA_BLOCK_WHITE_CANDLE_CAKE_ID,
        VANILLA_ITEM_ORANGE_CANDLE_ID => VANILLA_BLOCK_ORANGE_CANDLE_CAKE_ID,
        VANILLA_ITEM_MAGENTA_CANDLE_ID => VANILLA_BLOCK_MAGENTA_CANDLE_CAKE_ID,
        VANILLA_ITEM_LIGHT_BLUE_CANDLE_ID => VANILLA_BLOCK_LIGHT_BLUE_CANDLE_CAKE_ID,
        VANILLA_ITEM_YELLOW_CANDLE_ID => VANILLA_BLOCK_YELLOW_CANDLE_CAKE_ID,
        VANILLA_ITEM_LIME_CANDLE_ID => VANILLA_BLOCK_LIME_CANDLE_CAKE_ID,
        VANILLA_ITEM_PINK_CANDLE_ID => VANILLA_BLOCK_PINK_CANDLE_CAKE_ID,
        VANILLA_ITEM_GRAY_CANDLE_ID => VANILLA_BLOCK_GRAY_CANDLE_CAKE_ID,
        VANILLA_ITEM_LIGHT_GRAY_CANDLE_ID => VANILLA_BLOCK_LIGHT_GRAY_CANDLE_CAKE_ID,
        VANILLA_ITEM_CYAN_CANDLE_ID => VANILLA_BLOCK_CYAN_CANDLE_CAKE_ID,
        VANILLA_ITEM_PURPLE_CANDLE_ID => VANILLA_BLOCK_PURPLE_CANDLE_CAKE_ID,
        VANILLA_ITEM_BLUE_CANDLE_ID => VANILLA_BLOCK_BLUE_CANDLE_CAKE_ID,
        VANILLA_ITEM_BROWN_CANDLE_ID => VANILLA_BLOCK_BROWN_CANDLE_CAKE_ID,
        VANILLA_ITEM_GREEN_CANDLE_ID => VANILLA_BLOCK_GREEN_CANDLE_CAKE_ID,
        VANILLA_ITEM_RED_CANDLE_ID => VANILLA_BLOCK_RED_CANDLE_CAKE_ID,
        VANILLA_ITEM_BLACK_CANDLE_ID => VANILLA_BLOCK_BLACK_CANDLE_CAKE_ID,
        _ => return None,
    })
}
