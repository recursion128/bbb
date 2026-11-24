//! Player item durability criteria over the stack before the damage change.

use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::item_durability::damage_and_max;
use crate::loot_runtime::matches_item_predicate;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::IntBounds;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{int_bounds, item_criterion, ItemCriterion, LootCondition, ServerWorld, WorldEvent};

pub(super) const ITEM_DURABILITY_CHANGED: &str = "minecraft:item_durability_changed";

pub(super) struct ItemDurabilityCondition {
    player: Vec<LootCondition>,
    item: ItemCriterion,
    durability: IntBounds,
    delta: IntBounds,
}

impl ItemDurabilityCondition {
    pub(super) fn parse(
        value: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                value.field("player"),
                ITEM_DURABILITY_CHANGED,
                lookup,
            )?,
            item: item_criterion(value.field("item"))?,
            durability: int_bounds(value.field("durability"))?,
            delta: int_bounds(value.field("delta"))?,
        })
    }
}

impl ServerWorld {
    // ItemStack.java:462-473; ItemDurabilityTrigger.java:44-49.
    pub(in crate::world) fn trigger_item_durability_changed(
        &mut self,
        connection_id: ConnectionId,
        old_stack: &RuntimeItemStack,
        raw_new_damage: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(ITEM_DURABILITY_CHANGED)
            .next()
            .is_none()
        {
            return;
        }
        let Some((old_damage, max_damage)) = damage_and_max(old_stack.summary()) else {
            return;
        };
        let matches_player = self.player_matcher(connection_id, ITEM_DURABILITY_CHANGED);
        let tool =
            loot_tool_for_runtime_stack(old_stack, &self.datapack_tables.enchantment_registry);
        let publication = self
            .level_by_id(&player.dimension)
            .entities
            .item_publication();
        let tags = self.cold.advancement_triggers.item_tags.as_ref();
        let mut matching = Vec::new();
        for (index, criterion) in player
            .cold
            .advancements
            .trigger_listeners(ITEM_DURABILITY_CHANGED)
        {
            let Some(condition) = self
                .cold
                .advancement_triggers
                .item_durability
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            else {
                continue;
            };
            let item_matches = match (&condition.item, tags) {
                (None, _) => true,
                (Some(predicate), Some(tags)) => {
                    matches_item_predicate(predicate, &tool, tags, publication.resolver())
                        == Ok(true)
                }
                (Some(_), _) => false,
            };
            if item_matches
                && condition
                    .durability
                    .matches(max_damage.wrapping_sub(raw_new_damage))
                && condition
                    .delta
                    .matches(old_damage.wrapping_sub(raw_new_damage))
                && matches_player(&condition.player)
            {
                matching.push((index, criterion.clone()));
            }
        }
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
