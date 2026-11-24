//! The glass-bottle and water-potion behaviours.

use bbb_protocol::block_types::VANILLA_BLOCK_MUD_ID;
use bbb_protocol::item_types::{VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_HONEY_BOTTLE_ID};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::bucket::single_item;
use super::{BlockSource, Dispensed, DISPENSE_SOUND};
use crate::block_behavior::beehive::{
    honey_level, is_beehive, reset_honey_state, MAX_HONEY_LEVELS,
};
use crate::block_behavior::default_block_state_id;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::block_use::{is_water_potion, water_potion_stack};
use crate::world::ServerWorld;

const CONVERTABLE_TO_MUD: &str = "minecraft:convertable_to_mud";

/// The glass-bottle behaviour: honey from a full hive, else water.
pub(super) fn fill(
    world: &mut ServerWorld,
    source: &BlockSource,
    slot: usize,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let state = world.with_level(source.level, |level| level.block_state(target));
    let filled = if is_beehive(state) && honey_level(state) >= MAX_HONEY_LEVELS {
        if let Some(reset) = reset_honey_state(state) {
            world.set_block(source.level, target, reset, SET_BLOCK_UPDATE_ALL);
            world.release_beehive_occupants(
                source.level,
                target,
                reset,
                crate::world::beehive_release::BeehiveRelease::Harvest,
            );
        }
        single_item(VANILLA_ITEM_HONEY_BOTTLE_ID)
    } else if block_state_fluid_state(state).is_same_family(FluidFamily::Water) {
        water_potion_stack()
    } else {
        world.drop_one(source, &mut stack)?;
        return Ok(Dispensed::optional(stack, false));
    };
    world.level_mut_by_id(&source.level).event_sink.game_event(
        VanillaGameEvent::FLUID_PICKUP,
        source.pos,
        None,
    );
    let filled = RuntimeItemStack::from_summary(filled);
    let stack = world.consume_with_remainder(source, slot, stack, filled)?;
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}

/// The potion behaviour: a water bottle turns `convertable_to_mud` to mud.
pub(super) fn potion(
    world: &mut ServerWorld,
    source: &BlockSource,
    slot: usize,
    stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let convertible = is_water_potion(stack.summary())
        && world.with_level(source.level, |level| {
            let state = level.block_state(target);
            level.block_state_in_tag(state, CONVERTABLE_TO_MUD)
        });
    if !convertible {
        return world.default_dispense(source, stack);
    }
    world.send_mud_splash(&source.level, source.pos);
    let level = world.level_mut_by_id(&source.level);
    level
        .event_sink
        .play_block_sound(source.pos, "minecraft:item.bottle.empty");
    level
        .event_sink
        .game_event(VanillaGameEvent::FLUID_PLACE, source.pos, None);
    world.with_level(source.level, |level| {
        level.set_block(
            target,
            default_block_state_id(VANILLA_BLOCK_MUD_ID),
            SET_BLOCK_UPDATE_ALL,
            UPDATE_LIMIT,
        )
    });
    let bottle = RuntimeItemStack::from_summary(single_item(VANILLA_ITEM_GLASS_BOTTLE_ID));
    let stack = world.consume_with_remainder(source, slot, stack, bottle)?;
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}
