//! The TNT behaviour: a primed TNT in front of the dispenser.

use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed};
use crate::level_event::VanillaGameEvent;
use crate::world::ServerWorld;

pub(super) fn dispense(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let primed = world.spawn_primed_tnt(source.level, target, None);
    if primed.is_none() {
        return Ok(Dispensed::optional(stack, false));
    }
    let sink = &mut world.level_mut_by_id(&source.level).event_sink;
    sink.play_block_sound(target, "minecraft:entity.tnt.primed");
    sink.game_event(VanillaGameEvent::ENTITY_PLACE, target, None);
    stack.shrink(1);
    Ok(Dispensed::optional(stack, true))
}
