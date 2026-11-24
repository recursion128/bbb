use crate::block_entity::vault::{VaultKeyDecision, VaultState};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::{BlockUseContext, BlockUseResult, ServerWorld, WorldEvent};
use crate::world::simulation_tick::vault::VaultLevel;

pub(super) fn is_vault(state: i32) -> bool {
    vanilla_block_state_block_type_id(state)
        .and_then(vanilla_block_type)
        .and_then(|block| block.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::VaultBlock)
}

// VaultBlock.java:45-67: a nonempty hand on ACTIVE consumes the interaction even on refusal.
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    if context.selected_item.is_empty()
        || VaultState::from_block_state(context.block_state_id) != Some(VaultState::Active)
    {
        return BlockUseResult::TryWithEmptyHand;
    }
    let resolver = world.item_stack_resolver();
    let dimension = context.dimension;
    let pos = context.update.hit.pos;
    let uuid = world.levels[context.connection_id].profile.uuid;
    let time = world.game_time();
    let Some(vault) = world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .and_then(|entity| entity.vault_mut())
    else {
        return BlockUseResult::TryWithEmptyHand;
    };
    let decision = vault.key_decision(
        &resolver,
        VaultState::Active,
        time,
        uuid,
        &context.selected_item,
    );
    let reward = matches!(decision, Ok(VaultKeyDecision::DrawReward)).then(|| {
        (
            vault.config().loot_table().to_owned(),
            vault.config().key_item().count(),
        )
    });
    let result = (|| -> bbb_server_types::Result<()> {
        match decision? {
            VaultKeyDecision::Skip => {}
            VaultKeyDecision::Fail(sound) => {
                VaultLevel {
                    world,
                    dimension: &dimension,
                    pos,
                    events,
                }
                .sound(sound);
            }
            VaultKeyDecision::DrawReward => {
                let (table, key_count) = reward.expect("vault reward admission");
                let items = world.draw_vault_items(
                    &dimension,
                    pos,
                    &table,
                    Some((context.connection_id, &context.selected_item)),
                )?;
                if items.is_empty() {
                    return Ok(());
                }
                let mut entity = world
                    .level_mut_by_id(&dimension)
                    .chunks
                    .block_entities_mut()
                    .take(pos)
                    .expect("vault admission");
                let player = world
                    .levels
                    .player_mut(context.connection_id)
                    .expect("vault session");
                if let Some(item_id) = context.selected_item.item_id() {
                    player.award_stat_once(
                        &mut world.scoreboard,
                        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
                    );
                }
                if !player.abilities.instabuild {
                    player
                        .inventory
                        .item_in_hand_mut(context.update.hand)
                        .split(key_count);
                }
                let result = entity.vault_mut().expect("vault admission").unlock(
                    uuid,
                    items,
                    time,
                    VaultState::Active,
                    &mut VaultLevel {
                        world,
                        dimension: &dimension,
                        pos,
                        events,
                    },
                );
                world.put_back_block_entity(&dimension, entity);
                result?;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        tracing::warn!("vault key: {error}");
    }
    // `VaultBlock.java:63` returns `InteractionResult.SUCCESS_SERVER`.
    world.mark_use_item_on_server_swing();
    BlockUseResult::Consumed
}
