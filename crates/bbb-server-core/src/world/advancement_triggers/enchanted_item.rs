//! `EnchantedItemTrigger`: the enchanting table's criterion.

use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_stack;
use crate::loot_runtime::{matches_conditions, matches_item_predicate};
use bbb_protocol::packets::ItemStackSummary;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::IntBounds;
use bbb_server_datapack::loot::LootCondition;

use super::{int_bounds, item_criterion, ItemCriterion, ServerWorld, WorldEvent};

pub(super) const ENCHANTED_ITEM: &str = "minecraft:enchanted_item";

pub(super) struct EnchantedItemCondition {
    player: Vec<LootCondition>,
    item: ItemCriterion,
    levels: IntBounds,
}

impl EnchantedItemCondition {
    /// `EnchantedItemTrigger.TriggerInstance.CODEC` (`EnchantedItemTrigger.java:23-30`).
    pub(super) fn parse(
        value: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                value.field("player"),
                ENCHANTED_ITEM,
                lookup,
            )?,
            item: item_criterion(value.field("item"))?,
            levels: int_bounds(value.field("levels"))?,
        })
    }
}

impl ServerWorld {
    /// `EnchantedItemTrigger.trigger` (`EnchantedItemTrigger.java:17-19`).
    pub(in crate::world) fn trigger_enchanted_item(
        &mut self,
        connection_id: ConnectionId,
        stack: &ItemStackSummary,
        levels: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(ENCHANTED_ITEM)
            .next()
            .is_none()
        {
            return;
        }
        let Some(startup) = self.cold.config.datapacks.as_ref() else {
            return;
        };
        let tool = loot_tool_for_stack(stack, &self.datapack_tables.enchantment_registry);
        let Some(matching) = self.with_advancement_player_params(player, |level, mut params| {
            let advancement_triggers = &self.cold.advancement_triggers;
            let mut matching = Vec::new();
            params.weather = None;
            let tags = advancement_triggers.item_tags.as_ref();
            for (index, criterion) in player.cold.advancements.trigger_listeners(ENCHANTED_ITEM) {
                let Some(condition) = advancement_triggers
                    .enchanted_item
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                // `TriggerInstance.matches` (`EnchantedItemTrigger.java:37-39`).
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
                    && condition.levels.matches(levels)
                    && matches_conditions(
                        &startup.loot,
                        ENCHANTED_ITEM,
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
