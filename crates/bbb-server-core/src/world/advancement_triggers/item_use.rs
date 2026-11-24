//! `ConsumeItemTrigger`, `UsingItemTrigger`, `UsedTotemTrigger` and
//! `FilledBucketTrigger`: one item predicate over the stack a session
//! consumed, is using, was saved by or filled (`ConsumeItemTrigger.java:20-47`,
//! `UsingItemTrigger.java:21-38`, `UsedTotemTrigger.java:20-44`,
//! `FilledBucketTrigger.java:17-38`).

use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_stack;
use crate::loot_runtime::matches_item_predicate;
use bbb_protocol::packets::ItemStackSummary;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{LootCondition, ServerWorld, WorldEvent};

pub(in crate::world) const CONSUME_ITEM: &str = "minecraft:consume_item";
pub(in crate::world) const USING_ITEM: &str = "minecraft:using_item";
pub(in crate::world) const USED_TOTEM: &str = "minecraft:used_totem";
pub(in crate::world) const FILLED_BUCKET: &str = "minecraft:filled_bucket";
/// `ShotCrossbowTrigger` (`ShotCrossbowTrigger.java:16-35`), the same shape.
pub(in crate::world) const SHOT_CROSSBOW: &str = "minecraft:shot_crossbow";

pub(super) struct ItemCondition {
    player: Vec<LootCondition>,
    item: super::ItemCriterion,
}

impl ItemCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                CONSUME_ITEM,
                lookup,
            )?,
            item: super::item_criterion(conditions.field("item"))?,
        })
    }
}

impl ServerWorld {
    /// `ConsumeItemTrigger.trigger` / `UsingItemTrigger.trigger` for `trigger`.
    pub(in crate::world) fn trigger_item_criterion(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
        stack: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(trigger)
            .next()
            .is_none()
        {
            return;
        }
        let matches_player = self.player_matcher(connection_id, trigger);
        let tool = loot_tool_for_stack(stack, &self.datapack_tables.enchantment_registry);
        let publication = self
            .level_by_id(&player.dimension)
            .entities
            .item_publication();
        let tags = self.cold.advancement_triggers.item_tags.as_ref();
        let compiled = self.cold.advancement_triggers.item_criteria.get(trigger);
        let mut matching = Vec::new();
        for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
            let Some(condition) = compiled
                .and_then(|criteria| criteria.get(&index))
                .and_then(|criteria| criteria.get(criterion))
            else {
                continue;
            };
            let matched = match (&condition.item, tags) {
                (None, _) => true,
                (Some(predicate), Some(tags)) => {
                    matches_item_predicate(predicate, &tool, tags, publication.resolver())
                        == Ok(true)
                }
                (Some(_), _) => false,
            };
            if matched && matches_player(&condition.player) {
                matching.push((index, criterion.clone()));
            }
        }
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
