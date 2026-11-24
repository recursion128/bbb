//! `CrossbowItem` (`CrossbowItem.java:36-306`): charged while held into
//! `CHARGED_PROJECTILES`, shot by the next use.

use bbb_protocol::packets::InteractionHand;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::projectile_weapon::VolleyShape;
use super::{ItemUseContext, ItemUseResult};
use crate::connection::ConnectionId;
use crate::entity::ai::mob::ranged::perform::{crossbow_charge_duration, crossbow_charging_sounds};
use crate::entity::projectile_weapon::{
    charged_projectiles, is_firework, with_charged_projectiles,
};
use crate::world::advancement_triggers::SHOT_CROSSBOW;
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{ServerWorld, WorldEvent};

/// `CrossbowItem.getUseDuration` (`CrossbowItem.java:241-243`).
pub(in crate::world) const USE_DURATION: i32 = 72000;

/// `CrossbowItem.use` (`CrossbowItem.java:65-79`).
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let stack = runtime_item_in_hand(&world.levels[context.connection_id], context.hand).clone();
    let charged = charged_projectiles(&stack);
    if !charged.is_empty() {
        let power = if charged.iter().any(is_firework) {
            1.6
        } else {
            3.15
        };
        perform_shooting(world, context.connection_id, context.hand, power, events);
        return ItemUseResult::Consume;
    }
    if world.player_has_projectile(context.connection_id, true) {
        world.start_using_item(context.connection_id, context.hand);
        return ItemUseResult::Consume;
    }
    ItemUseResult::Fail
}

/// `CrossbowItem.performShooting` (`CrossbowItem.java:171-190`) for a session.
fn perform_shooting(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    hand: InteractionHand,
    power: f32,
    events: &mut Vec<WorldEvent>,
) {
    let stack = runtime_item_in_hand(&world.levels[connection_id], hand).clone();
    let charged = charged_projectiles(&stack);
    world.update_hand_stack(connection_id, hand, with_charged_projectiles(&stack, &[]));
    world.player_shoot_volley(
        connection_id,
        hand,
        charged,
        VolleyShape {
            crossbow: true,
            power,
            uncertainty: 1.0,
            crit: true,
        },
        events,
    );
    let weapon = runtime_item_in_hand(&world.levels[connection_id], hand).clone();
    world.trigger_item_criterion(connection_id, SHOT_CROSSBOW, weapon.summary(), events);
    if let Some(item_id) = stack.item_id() {
        world.levels[connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
}

fn charge_duration(
    world: &ServerWorld,
    connection_id: ConnectionId,
    stack: &RuntimeItemStack,
) -> i32 {
    let dimension = world.levels[connection_id].dimension;
    let level = world.level_by_id(&dimension);
    crossbow_charge_duration(
        level.entities.weapon_effects(),
        stack,
        &world.random,
        &world.value_effect_level(&dimension),
    )
}

/// `CrossbowItem.releaseUsing` (`CrossbowItem.java:86-89`).
pub(in crate::world) fn release_using(
    world: &ServerWorld,
    connection_id: ConnectionId,
    stack: &RuntimeItemStack,
    remaining: i32,
) -> bool {
    let charge = charge_duration(world, connection_id, stack);
    let pow = ((USE_DURATION - remaining) as f32 / charge as f32).min(1.0);
    pow >= 1.0 && !charged_projectiles(stack).is_empty()
}

/// `CrossbowItem.onUseTick` (`CrossbowItem.java:202-238`): the loading
/// sounds as the charge passes a fifth and a half, and the load at full.
pub(in crate::world) fn on_use_tick(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    hand: InteractionHand,
    stack: &RuntimeItemStack,
    remaining: i32,
    events: &mut Vec<WorldEvent>,
) {
    let dimension = world.levels[connection_id].dimension;
    let sounds = crossbow_charging_sounds(
        world.level_by_id(&dimension).entities.weapon_effects(),
        stack,
    );
    let charge = charge_duration(world, connection_id, stack) as f32;
    let used = (USE_DURATION - remaining) as f32;
    let percent = used / charge;
    // The item's `startSoundPlayed`/`midLoadSoundPlayed` latches, as the tick
    // the charge crosses each mark.
    let previous = (used - 1.0) / charge;
    let crossed = |mark: f32| {
        percent >= mark
            && !matches!(
                previous.partial_cmp(&mark),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            )
    };
    for (mark, sound) in [(0.2, &sounds.start), (0.5, &sounds.mid)] {
        if let Some(sound) = sound.as_ref().filter(|_| crossed(mark)) {
            world.play_entity_sound_holder(connection_id, None, sound.clone(), 0.5, 1.0, events);
        }
    }
    if percent >= 1.0 && charged_projectiles(stack).is_empty() {
        let drawn = world.player_draw_projectiles(connection_id, stack, true);
        if drawn.is_empty() {
            return;
        }
        world.update_hand_stack(connection_id, hand, with_charged_projectiles(stack, &drawn));
        if let Some(sound) = sounds.end {
            let pitch = 1.0 / (world.random.next_float() * 0.5 + 1.0) + 0.2;
            world.play_entity_sound_holder(connection_id, None, sound, 1.0, pitch, events);
        }
    }
}
