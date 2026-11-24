//! The block, tool and occupants of a player-mined hive.

use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::loot_runtime::{matches_conditions, matches_item_predicate};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::IntBounds;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{int_bounds, item_criterion, ItemCriterion, ServerWorld, WorldEvent};

pub(super) const BEE_NEST_DESTROYED: &str = "minecraft:bee_nest_destroyed";

pub(super) struct BeeNestDestroyedCondition {
    player: Vec<LootCondition>,
    item: ItemCriterion,
    block: Option<i32>,
    bees_inside: IntBounds,
}

impl BeeNestDestroyedCondition {
    // BeeNestDestroyedTrigger.java:29-36.
    pub(super) fn parse(
        value: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                value.field("player"),
                BEE_NEST_DESTROYED,
                lookup,
            )?,
            item: item_criterion(value.field("item"))?,
            block: value
                .field("block")
                .map(|value| {
                    let name = bbb_server_datapack::loot::codec::id(value)
                        .map_err(|error| error.to_string())?;
                    vanilla_block_type_for_resource_id(&name)
                        .map(|block| block.id)
                        .ok_or_else(|| format!("unknown block {name}"))
                })
                .transpose()?,
            bees_inside: int_bounds(value.field("num_bees_inside"))?,
        })
    }
}

impl ServerWorld {
    // BeeNestDestroyedTrigger.java:21-23.
    pub(in crate::world) fn trigger_bee_nest_destroyed(
        &mut self,
        connection_id: ConnectionId,
        state: i32,
        stack: &RuntimeItemStack,
        bees_inside: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(BEE_NEST_DESTROYED)
            .next()
            .is_none()
        {
            return;
        }
        let Some(startup) = self.cold.config.datapacks.as_ref() else {
            return;
        };
        let tool = loot_tool_for_runtime_stack(stack, &self.datapack_tables.enchantment_registry);
        let Some(matching) = self.with_advancement_player_params(player, |level, params| {
            let advancement_triggers = &self.cold.advancement_triggers;
            let mut matching = Vec::new();
            let tags = advancement_triggers.item_tags.as_ref();
            for (index, criterion) in player
                .cold
                .advancements
                .trigger_listeners(BEE_NEST_DESTROYED)
            {
                let Some(condition) = advancement_triggers
                    .bee_nest_destroyed
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                // BeeNestDestroyedTrigger.java:50-56.
                let item_matches = match (&condition.item, tags) {
                    (None, _) => true,
                    (Some(predicate), Some(tags)) => {
                        matches_item_predicate(
                            predicate,
                            &tool,
                            tags,
                            level.entities.item_resolver(),
                        ) == Ok(true)
                    }
                    (Some(_), _) => false,
                };
                if item_matches
                    && condition
                        .block
                        .is_none_or(|block| Some(block) == vanilla_block_state_block_type_id(state))
                    && condition.bees_inside.matches(bees_inside)
                    && matches_conditions(
                        &startup.loot,
                        BEE_NEST_DESTROYED,
                        &condition.player,
                        &params,
                        &level.random,
                    ) == Ok(true)
                {
                    matching.push((index, criterion.clone()));
                }
            }
            matching
        }) else {
            return;
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
