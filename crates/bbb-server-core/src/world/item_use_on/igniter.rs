//! `FlintAndSteelItem.useOn` (`FlintAndSteelItem.java:26-58`) and
//! `FireChargeItem.useOn` (`FireChargeItem.java:30-55`): light a candle,
//! candle cake or campfire, otherwise place fire against the clicked face;
//! the fire's own `onPlace` lights a nether portal frame.

use bbb_server_types::directions::relative;

use super::ItemUseOnContext;
use crate::block_behavior::campfire;
use crate::block_behavior::candle::{
    self, candle_cake_can_light, FIRE_CHARGE_SOUND, FLINT_AND_STEEL_SOUND,
};
use crate::block_behavior::fire::igniter_fire_state;
use crate::connection::ConnectionId;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};

const UPDATE_ALL_IMMEDIATE: i32 = 11;

pub(super) fn flint_and_steel(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if let Some(state) = lightable_state(world, context) {
        let source = Some(world.player_entity_id(context));
        world.with_level(context.dimension, |level| {
            candle::light_with_flint(level, context.pos(), state, source);
        });
        world.hurt_and_break_in_hand(context.connection_id, context.hand(), 1, events);
        return true;
    }
    let pitch = world.random.next_float() * 0.4 + 0.8;
    if !world.place_igniter_fire(
        context,
        FLINT_AND_STEEL_SOUND,
        pitch,
        UPDATE_ALL_IMMEDIATE,
        // `FlintAndSteelItem.java:34` plays with the player, who predicted it.
        Some(context.connection_id),
        events,
    ) {
        return false;
    }
    let target = relative(context.pos(), context.update.hit.direction);
    world.trigger_item_used_on_location(
        context.connection_id,
        "minecraft:placed_block",
        target,
        context.stack,
        events,
    );
    world.hurt_and_break_in_hand(context.connection_id, context.hand(), 1, events);
    true
}

pub(super) fn fire_charge(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if let Some(state) = lightable_state(world, context) {
        let source = Some(world.player_entity_id(context));
        world.with_level(context.dimension, |level| {
            candle::light_with_fire_charge(level, context.pos(), state, source);
        });
        world.shrink_used_stack(context);
        return true;
    }
    let pitch = (world.random.next_float() - world.random.next_float()) * 0.2 + 1.0;
    if !world.place_igniter_fire(
        context,
        FIRE_CHARGE_SOUND,
        pitch,
        SET_BLOCK_UPDATE_ALL,
        // `FireChargeItem.java:58-61` plays with no player: the user hears it too.
        None,
        events,
    ) {
        return false;
    }
    world.shrink_used_stack(context);
    true
}

/// The clicked state when `CampfireBlock.canLight`, `CandleBlock.canLight` or
/// `CandleCakeBlock.canLight` holds (`FlintAndSteelItem.java:31`).
fn lightable_state(world: &mut ServerWorld, context: &ItemUseOnContext<'_>) -> Option<i32> {
    let state = world.block_state_or_air(context.dimension, context.pos());
    (campfire::can_light(state) || candle::can_light(state) || candle_cake_can_light(state))
        .then_some(state)
}

impl ServerWorld {
    fn place_igniter_fire(
        &mut self,
        context: &ItemUseOnContext<'_>,
        sound: &str,
        pitch: f32,
        flags: i32,
        except: Option<ConnectionId>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let target = relative(context.pos(), context.update.hit.direction);
        let Some(fire) =
            self.with_level(context.dimension, |level| igniter_fire_state(level, target))
        else {
            return false;
        };
        self.play_block_sound_as(
            context.dimension,
            target,
            sound,
            (1.0, pitch),
            except,
            events,
        );
        self.set_block(context.dimension, target, fire, flags);
        let source = self.player_entity_id(context);
        self.level_mut_by_id(&context.dimension)
            .event_sink
            .game_event_with_source(VanillaGameEvent::BLOCK_PLACE, target, Some(source), None);
        true
    }
}
