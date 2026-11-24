//! Consuming block interactions, at the clicked cell after the action.

use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::loot_runtime::{matches_conditions, param_sets, LootOrigin, LootParams};
use bbb_protocol::packets::{BlockPos, InteractionHand};
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;

use super::{context_predicate, ServerWorld, WorldEvent};
use crate::world::using_item::runtime_item_in_hand;

pub(super) const DEFAULT_BLOCK_USE: &str = "minecraft:default_block_use";
pub(super) const ANY_BLOCK_USE: &str = "minecraft:any_block_use";

pub(super) struct BlockUseCondition {
    player: Vec<LootCondition>,
    location: Vec<LootCondition>,
}

impl BlockUseCondition {
    pub(super) fn parse(
        value: V<'_>,
        trigger: &str,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                value.field("player"),
                trigger,
                lookup,
            )?,
            location: context_predicate(
                value.field("location"),
                "location",
                if trigger == DEFAULT_BLOCK_USE {
                    param_sets::BLOCK_USE
                } else {
                    param_sets::ADVANCEMENT_LOCATION
                },
                lookup,
            )?,
        })
    }
}

impl ServerWorld {
    pub(in crate::world) fn trigger_default_block_use(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_block_interaction(connection_id, pos, None, events);
    }

    pub(in crate::world) fn trigger_any_block_use(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_block_interaction(connection_id, pos, Some(hand), events);
    }

    // DefaultBlockInteractionTrigger.java:23-32; AnyBlockInteractionTrigger.java:24-34.
    fn trigger_block_interaction(
        &mut self,
        connection_id: ConnectionId,
        pos: BlockPos,
        hand: Option<InteractionHand>,
        events: &mut Vec<WorldEvent>,
    ) {
        let trigger = if hand.is_some() {
            ANY_BLOCK_USE
        } else {
            DEFAULT_BLOCK_USE
        };
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
        let dimension = player.dimension;
        let Some(startup) = self.cold.config.datapacks.clone() else {
            return;
        };
        let tool = hand.map(|hand| {
            loot_tool_for_runtime_stack(
                runtime_item_in_hand(player, hand),
                &self.datapack_tables.enchantment_registry,
            )
        });
        let state = self
            .block_state_by_key(&dimension, pos)
            .unwrap_or_else(|| self.level_by_id(&dimension).chunks.air_state_id());
        let player = self.levels.player_at(connection_id);
        let Some(matching) = self.with_advancement_player_params(player, |level, player_params| {
            let advancement_triggers = &self.cold.advancement_triggers;
            let mut matching = Vec::new();
            let location_params = LootParams {
                dimension: player_params.dimension,
                item_environment: player_params.item_environment,
                vehicle_slots: player_params.vehicle_slots,
                origin: Some(LootOrigin::at_center_of(pos)),
                block_state_id: Some(state),
                tool: tool.as_ref(),
                entities: player_params.entities,
                level: player_params.level,
                block_entity_data: player_params.block_entity_data,
                weather: player_params.weather,
                ..LootParams::new(if hand.is_some() {
                    param_sets::ADVANCEMENT_LOCATION
                } else {
                    param_sets::BLOCK_USE
                })
            };
            let compiled = advancement_triggers.block_use.get(trigger);
            for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
                let Some(condition) = compiled
                    .and_then(|criteria| criteria.get(&index))
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                if matches_conditions(
                    &startup.loot,
                    trigger,
                    &condition.player,
                    &player_params,
                    &level.random,
                ) == Ok(true)
                    && matches_conditions(
                        &startup.loot,
                        trigger,
                        &condition.location,
                        &location_params,
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
