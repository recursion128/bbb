//! `BredAnimalsTrigger` (`BredAnimalsTrigger.java:22-72`): predicates over the
//! two parents, in either order, and the child.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{KillEntity, LootCondition, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;

pub(super) const BRED_ANIMALS: &str = "minecraft:bred_animals";

pub(super) struct BredCondition {
    player: Vec<LootCondition>,
    parent: Vec<LootCondition>,
    partner: Vec<LootCondition>,
    child: Option<Vec<LootCondition>>,
}

impl BredCondition {
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
                BRED_ANIMALS,
                lookup,
            )?,
            parent: predicate("parent")?,
            partner: predicate("partner")?,
            child: conditions
                .field("child")
                .map(|_| predicate("child"))
                .transpose()?,
        })
    }
}

impl ServerWorld {
    pub(in crate::world) fn trigger_bred_animals(
        &mut self,
        connection_id: ConnectionId,
        parent: KillEntity,
        partner: KillEntity,
        child: Option<KillEntity>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(BRED_ANIMALS)
            .next()
            .is_none()
        {
            return;
        }
        let dimension = player.dimension;
        let matching = {
            let Some(matches) = self.entity_matcher(connection_id, &dimension, BRED_ANIMALS) else {
                return;
            };
            let matches_player = self.player_matcher(connection_id, BRED_ANIMALS);
            let entity_matches = |entity: &KillEntity, conditions: &[LootCondition]| {
                matches(entity.loot(), conditions)
            };
            let compiled = &self.cold.advancement_triggers.bred;
            player
                .cold
                .advancements
                .trigger_listeners(BRED_ANIMALS)
                .filter(|(index, criterion)| {
                    let Some(condition) = compiled
                        .get(index)
                        .and_then(|criteria| criteria.get(*criterion))
                    else {
                        return false;
                    };
                    // `BredAnimalsTrigger.java:68-72`.
                    let child_matches = match (&condition.child, &child) {
                        (None, _) => true,
                        (Some(conditions), Some(child)) => entity_matches(child, conditions),
                        (Some(_), None) => false,
                    };
                    child_matches
                        && (entity_matches(&parent, &condition.parent)
                            && entity_matches(&partner, &condition.partner)
                            || entity_matches(&partner, &condition.parent)
                                && entity_matches(&parent, &condition.partner))
                        && matches_player(&condition.player)
                })
                .map(|(index, criterion)| (index, criterion.clone()))
                .collect()
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
