//! `LeadItem.useOn` at the item leg of block interaction.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::item_types::{vanilla_item_type, VanillaItemUseOnOverride};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, UseItemOn};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;

use crate::connection::ConnectionId;
use crate::level_event::VanillaGameEvent;
use bbb_server_types::stats::StatKey;

use super::{ServerWorld, WorldEvent};

/// `LeadItem.bindPlayerMobs` (`LeadItem.java:37-65`) plus the SUCCESS_SERVER
/// tail (`:54-57`): `BLOCK_ATTACH` and the server swing. True only when Bound.
/// The item path awards `ITEM_USED` itself; the without-item path must not.
pub(super) fn bind_player_mobs_at(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    pos: BlockPos,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let dimension = world.player_level_id(connection_id);
    let player_entity_id = world.levels[connection_id].entity_id();
    let outcome = crate::entity::leash::bind_player_mobs(
        &mut world.level_mut_by_id(&dimension).entities,
        player_entity_id,
        pos,
    );
    world.apply_level_entity_lifecycle_by_key(&dimension, events);
    if outcome != crate::entity::leash::LeadBindOutcome::Bound {
        return false;
    }
    // `:56`.
    world
        .level_mut_by_id(&dimension)
        .event_sink
        .game_event_with_source(
            VanillaGameEvent::BLOCK_ATTACH,
            pos,
            Some(player_entity_id),
            None,
        );
    world.mark_use_item_on_server_swing();
    true
}

/// `LeadItem.useOn` (`LeadItem.java:23-35`) over `bindPlayerMobs` (`:37-65`).
/// True only when something bound: `SUCCESS_SERVER` consumes the click,
/// awards `ITEM_USED` (`ItemStack.java:367-369`) and swings on the server;
/// every other answer is the branch's own `PASS`.
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    connection_id: super::ConnectionId,
    update: UseItemOn,
    stack: &ItemStackSummary,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let item_id = stack.item_id;
    let is_lead = item_id
        .and_then(vanilla_item_type)
        .is_some_and(|item| item.use_on_override == Some(VanillaItemUseOnOverride::LeadItem));
    if !is_lead {
        return false;
    }
    let dimension = world.player_level_id(connection_id);
    let Some(state) = world.block_state(dimension, update.hit.pos) else {
        return false;
    };
    // `:27`: `state.is(BlockTags.FENCES)`.
    let fence = vanilla_block_state_block_type_id(state).is_some_and(|block_id| {
        world
            .level_by_id(&dimension)
            .chunks
            .registry_tags()
            .is_some_and(|tags| tags.contains_block("minecraft:fences", block_id))
    });
    if !fence {
        return false;
    }
    if !bind_player_mobs_at(world, connection_id, update.hit.pos, events) {
        return false;
    }
    if let Some(item_id) = item_id {
        world.levels[connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    true
}
