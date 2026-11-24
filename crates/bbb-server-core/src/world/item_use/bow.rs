//! `BowItem` (`BowItem.java:17-115`): drawn while held, loosed on release.

use bbb_protocol::packets::InteractionHand;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::projectile_weapon::VolleyShape;
use super::{ItemUseContext, ItemUseResult};
use crate::connection::ConnectionId;
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{ServerWorld, WorldEvent};

/// `BowItem.getUseDuration` (`BowItem.java:85-87`).
pub(in crate::world) const USE_DURATION: i32 = 72000;

/// `BowItem.use` (`BowItem.java:95-104`).
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    _events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    if !world.player_has_projectile(context.connection_id, false) {
        return ItemUseResult::Fail;
    }
    world.start_using_item(context.connection_id, context.hand);
    ItemUseResult::Consume
}

/// `BowItem.getPowerForTime` (`BowItem.java:74-82`).
fn power_for_time(time_held: i32) -> f32 {
    let pow = time_held as f32 / 20.0;
    ((pow * pow + pow * 2.0) / 3.0).min(1.0)
}

/// `BowItem.releaseUsing` (`BowItem.java:26-59`).
pub(in crate::world) fn release_using(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    hand: InteractionHand,
    remaining: i32,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if !world.player_has_projectile(connection_id, false) {
        return false;
    }
    let pow = power_for_time(USE_DURATION - remaining);
    if f64::from(pow) < 0.1 {
        return false;
    }
    let weapon = runtime_item_in_hand(&world.levels[connection_id], hand).clone();
    let projectiles = world.player_draw_projectiles(connection_id, &weapon, false);
    if !projectiles.is_empty() {
        world.player_shoot_volley(
            connection_id,
            hand,
            projectiles,
            VolleyShape {
                crossbow: false,
                power: pow * 3.0,
                uncertainty: 1.0,
                crit: pow == 1.0,
            },
            events,
        );
    }
    let pitch = 1.0 / (world.random.next_float() * 0.4 + 1.2) + pow * 0.5;
    let _ = world.play_entity_sound(
        connection_id,
        None,
        "minecraft:entity.arrow.shoot",
        1.0,
        pitch,
        events,
    );
    if let Some(item_id) = weapon.item_id() {
        world.levels[connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    true
}
