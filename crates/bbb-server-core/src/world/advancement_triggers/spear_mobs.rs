//! `SpearMobsTrigger` (`SpearMobsTrigger.java:11-38`): how many living
//! entities one kinetic use has struck.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{LootCondition, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;

pub(super) const SPEAR_MOBS: &str = "minecraft:spear_mobs";

pub(super) struct SpearMobsCondition {
    player: Vec<LootCondition>,
    count: Option<i32>,
}

impl SpearMobsCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                SPEAR_MOBS,
                lookup,
            )?,
            count: conditions
                .field("count")
                .map(|value| {
                    value
                        .codec_i32()
                        .filter(|count| *count > 0)
                        .ok_or("spear_mobs count must be a positive int")
                })
                .transpose()?,
        })
    }
}

impl ServerWorld {
    /// `SpearMobsTrigger.trigger` (`:17-19`).
    pub(in crate::world) fn trigger_spear_mobs(
        &mut self,
        connection_id: ConnectionId,
        count: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let matches_player = self.player_matcher(connection_id, SPEAR_MOBS);
        let compiled = &self.cold.advancement_triggers.spear_mobs;
        let matching = player
            .cold
            .advancements
            .trigger_listeners(SPEAR_MOBS)
            .filter(|(index, criterion)| {
                compiled
                    .get(index)
                    .and_then(|criteria| criteria.get(*criterion))
                    .is_some_and(|condition| {
                        condition.count.is_none_or(|required| count >= required)
                            && matches_player(&condition.player)
                    })
            })
            .map(|(index, criterion)| (index, criterion.clone()))
            .collect();
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
