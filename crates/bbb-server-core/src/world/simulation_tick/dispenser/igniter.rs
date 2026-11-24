//! The flint-and-steel behaviour: fire in front, else light a campfire,
//! candle or candle cake, else prime TNT.

use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed};
use crate::block_behavior::fire::igniter_fire_state;
use crate::block_behavior::{campfire, candle};
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::ServerWorld;

pub(super) fn dispense(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let lit = world.with_level(source.level, |level| {
        if let Some(fire) = igniter_fire_state(level, target) {
            level.set_block(target, fire, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
            level.game_event_from_entity(VanillaGameEvent::BLOCK_PLACE, target, None, None);
            return true;
        }
        let state = level.block_state(target);
        if campfire::can_light(state)
            || candle::can_light(state)
            || candle::candle_cake_can_light(state)
        {
            if let Some(lit) = with_property(state, "lit", "true") {
                level.set_block(target, lit, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
                level.game_event_from_entity(VanillaGameEvent::BLOCK_CHANGE, target, None, None);
            }
            return true;
        }
        false
    });
    // `TntBlock.prime` then `removeBlock`; anything else is not TNT and fails.
    let success = lit || world.prime_tnt_from_redstone(source.level, target);
    if success {
        world.hurt_dispensed(source.level, &mut stack, 1);
    }
    Ok(Dispensed::optional(stack, success))
}
