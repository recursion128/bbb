//! `KilledByArrowTrigger` (`KilledByArrowTrigger.java:26-116`): the victims
//! one arrow killed, the weapon that fired it and how many types it took.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{
    int_bounds, item_criterion, matches_item_predicate, ItemCriterion, KillEntity, LootCondition,
    ServerWorld, WorldEvent,
};
use crate::connection::ConnectionId;
use crate::entity::Entity;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_server_datapack::loot::IntBounds;
use bbb_server_types::item_stack::RuntimeItemStack;

pub(super) const KILLED_BY_ARROW: &str = "minecraft:killed_by_arrow";

pub(super) struct KilledCondition {
    player: Vec<LootCondition>,
    victims: Vec<Vec<LootCondition>>,
    unique_entity_types: IntBounds,
    fired_from_weapon: ItemCriterion,
}

impl KilledCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                KILLED_BY_ARROW,
                lookup,
            )?,
            victims: super::lightning::victim_predicates(conditions, lookup)?,
            unique_entity_types: int_bounds(conditions.field("unique_entity_types"))?,
            fired_from_weapon: item_criterion(conditions.field("fired_from_weapon"))?,
        })
    }
}

impl ServerWorld {
    /// `KilledByArrowTrigger.trigger` (`:32-42`): each victim predicate takes a
    /// distinct victim (`:80-105`), then the weapon and the type count.
    pub(in crate::world) fn trigger_killed_by_arrow(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        victims: &[i32],
        weapon: Option<&RuntimeItemStack>,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.listens(connection_id, KILLED_BY_ARROW) {
            return;
        }
        let mut types: Vec<i32> = victims
            .iter()
            .filter_map(|id| {
                self.level_by_id(dimension)
                    .entities
                    .get(*id)
                    .map(Entity::entity_type_id)
                    .or_else(|| {
                        self.connection_by_entity_id(*id)
                            .map(|_| VANILLA_ENTITY_TYPE_PLAYER_ID)
                    })
            })
            .collect();
        types.sort_unstable();
        types.dedup();
        let unique_types = types.len() as i32;
        let publication = self.level_by_id(dimension).entities.item_publication();
        let victims: Vec<KillEntity> = victims
            .iter()
            .map(|id| self.killed_entity_of_id(dimension, *id))
            .collect();
        let weapon = weapon.map(|stack| {
            crate::entity::block_drops::loot_tool_for_runtime_stack(
                stack,
                &self.datapack_tables.enchantment_registry,
            )
        });
        let matching = {
            let Some(matches) = self.entity_matcher(connection_id, dimension, KILLED_BY_ARROW)
            else {
                return;
            };
            let matches_player = self.player_matcher(connection_id, KILLED_BY_ARROW);
            let compiled = &self.cold.advancement_triggers.killed_by_arrow;
            let tags = self.cold.advancement_triggers.item_tags.as_ref();
            self.listening_criteria(connection_id, KILLED_BY_ARROW, |index, criterion| {
                let Some(condition) = compiled
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    return false;
                };
                let weapon_matches = match (&condition.fired_from_weapon, &weapon, tags) {
                    (None, _, _) => true,
                    (Some(predicate), Some(tool), Some(tags)) => {
                        matches_item_predicate(predicate, tool, tags, publication.resolver())
                            == Ok(true)
                    }
                    (Some(_), _, _) => false,
                };
                let mut unclaimed: Vec<&KillEntity> = victims.iter().collect();
                let victims_match = condition.victims.iter().all(|predicate| {
                    match unclaimed
                        .iter()
                        .position(|victim| matches(victim.loot(), predicate))
                    {
                        Some(found) => {
                            unclaimed.remove(found);
                            true
                        }
                        None => false,
                    }
                });
                weapon_matches
                    && victims_match
                    && condition.unique_entity_types.matches(unique_types)
                    && matches_player(&condition.player)
            })
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
