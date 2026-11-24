//! `SpawnEggItem.use` (`SpawnEggItem.java:110-136`): the liquid-source arm a
//! right click at nothing takes.

use crate::fluid::liquid_block_family;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::stats::StatKey;

use crate::world::block_use::spawn_egg::{spawn_mob, SpawnMobOutcome};
use crate::world::pov_clip::{player_pov_clip, ClipFluid};
use crate::world::{ServerWorld, WorldEvent};

use super::{ItemUseContext, ItemUseResult};

/// `SpawnEggItem.use` (`SpawnEggItem.java:110-136`): the player's POV ray with
/// `ClipContext.Fluid.SOURCE_ONLY`, then a liquid block cell and the two
/// game-mode gates before the same `spawnMob` `useOn` runs, with
/// `tryMoveDown` and `movedUp` both false so the entity lands in the fluid's
/// own cell.
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    _events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    // `:112-115`: anything but a block hit is a pass, which includes the fluid
    // shapes the ray stopped on only in `SOURCE_ONLY` mode.
    let Some(hit) = player_pov_clip(world, context.connection_id, ClipFluid::SourceOnly) else {
        return ItemUseResult::Pass;
    };
    // `:119-121`: the cell has to hold a `LiquidBlock`, whatever the ray's
    // fluid shape was.
    let block_state = world.block_state_or_air(context.dimension, hit.pos);
    if liquid_block_family(block_state).is_none() {
        return ItemUseResult::Pass;
    }
    // `:123`: `Level.mayInteract` then `Player.mayUseItemAt` (`Player.java:1593-1600`).
    let target = relative(hit.pos, opposite(hit.direction));
    if !world.player_may_interact_block(context.connection_id, hit.pos)
        || !world.player_may_place_block(context.connection_id, &context.item, target)
    {
        return ItemUseResult::Fail;
    }
    let outcome = spawn_mob(
        world,
        context.connection_id,
        context.hand,
        &context.item,
        context.dimension,
        hit.pos,
        false,
        false,
    );
    // `:124-129`: `spawnMob`'s `FAIL` is this leg's answer too; its `SUCCESS`
    // awards the stat here rather than in `ItemStack.useOn`, and no
    // `item_used_on_block` trigger belongs to this path
    // (`ServerPlayerGameMode.java:397-399` is `useItemOn`'s).
    if matches!(outcome, SpawnMobOutcome::Refused) {
        return ItemUseResult::Fail;
    }
    let player = world
        .levels
        .player_mut(context.connection_id)
        .expect("connection presence was checked");
    if let Some(item_id) = context.item.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    ItemUseResult::Success
}
