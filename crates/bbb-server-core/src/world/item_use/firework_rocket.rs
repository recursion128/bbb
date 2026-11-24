//! `FireworkRocketItem.use` (`FireworkRocketItem.java:58-75`): a rocket
//! attached to a gliding player.

use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};
use crate::entity::firework_rocket::spawn_launched_firework;
use crate::world::{ServerWorld, WorldEvent};

pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    _events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let player = &world.levels[context.connection_id];
    if !player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING) {
        return ItemUseResult::Pass;
    }
    let position = player.position();
    let center = player
        .bounding_box()
        .map_or(position, |bounds| bounds.center());
    let owner = player.profile.uuid;
    let player_entity_id = player.entity_id();
    let item = RuntimeItemStack::from_summary(ItemStackSummary {
        count: 1,
        ..context.item.clone()
    });
    let store = &mut world.level_mut_by_id(&context.dimension).entities;
    // FireworkRocketItem.java:62-64.
    if crate::entity::leash::drop_all_leash_connections(store, player_entity_id, center) {
        store.play_entity_sound(crate::entity::PendingEntitySound {
            position,
            resource_id: crate::entity::leash::LEAD_BREAK_SOUND.to_owned(),
            source: bbb_protocol::packets::SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    if let Some(id) =
        spawn_launched_firework(store, position, item.clone(), owner, Some(player_entity_id))
    {
        crate::entity::projectile_weapon::queue_spawn_effects(store, id, item);
    }
    let player = &mut world.levels[context.connection_id];
    let infinite = player.abilities.instabuild;
    player.inventory.consume_in_hand(context.hand, 1, infinite);
    if let Some(item_id) = context.item.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    ItemUseResult::Success
}

#[cfg(test)]
#[path = "firework_rocket_tests.rs"]
mod tests;
