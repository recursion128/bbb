//! `CuredZombieVillagerTrigger` (`CuredZombieVillagerTrigger.java:21-45`):
//! predicates over the zombie villager and the villager it became.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{KillEntity, LootCondition, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;

pub(super) const CURED_ZOMBIE_VILLAGER: &str = "minecraft:cured_zombie_villager";

pub(super) struct CuredCondition {
    player: Vec<LootCondition>,
    zombie: Vec<LootCondition>,
    villager: Vec<LootCondition>,
}

impl CuredCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        let predicate = |field: &str| {
            super::construct_beacon::player_conditions(conditions.field(field), field, lookup)
        };
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                CURED_ZOMBIE_VILLAGER,
                lookup,
            )?,
            zombie: predicate("zombie")?,
            villager: predicate("villager")?,
        })
    }
}

impl ServerWorld {
    pub(in crate::world) fn trigger_cured_zombie_villager(
        &mut self,
        connection_id: ConnectionId,
        zombie: KillEntity,
        villager: KillEntity,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(CURED_ZOMBIE_VILLAGER)
            .next()
            .is_none()
        {
            return;
        }
        let dimension = player.dimension;
        let matching = {
            let Some(matches) =
                self.entity_matcher(connection_id, &dimension, CURED_ZOMBIE_VILLAGER)
            else {
                return;
            };
            let matches_player = self.player_matcher(connection_id, CURED_ZOMBIE_VILLAGER);
            let entity_matches = |entity: &KillEntity, conditions: &[LootCondition]| {
                matches(entity.loot(), conditions)
            };
            let compiled = &self.cold.advancement_triggers.cured_zombie_villager;
            player
                .cold
                .advancements
                .trigger_listeners(CURED_ZOMBIE_VILLAGER)
                .filter(|(index, criterion)| {
                    let Some(condition) = compiled
                        .get(index)
                        .and_then(|criteria| criteria.get(*criterion))
                    else {
                        return false;
                    };
                    entity_matches(&zombie, &condition.zombie)
                        && entity_matches(&villager, &condition.villager)
                        && matches_player(&condition.player)
                })
                .map(|(index, criterion)| (index, criterion.clone()))
                .collect()
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
