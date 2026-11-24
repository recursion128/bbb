//! `UsedEnderEyeTrigger` (`UsedEnderEyeTrigger.java:15-34`): the horizontal
//! distance from a session to the structure its thrown eye points at.

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{double_bounds, DoubleBounds, LootCondition, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;

pub(super) const USED_ENDER_EYE: &str = "minecraft:used_ender_eye";

pub(super) struct EyeCondition {
    player: Vec<LootCondition>,
    distance: DoubleBounds,
}

impl EyeCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                USED_ENDER_EYE,
                lookup,
            )?,
            distance: double_bounds(conditions.field("distance"))?,
        })
    }
}

impl ServerWorld {
    /// `UsedEnderEyeTrigger.trigger` (`:15-20`).
    pub(in crate::world) fn trigger_used_ender_eye(
        &mut self,
        connection_id: ConnectionId,
        feature: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let position = player.position();
        let dx = position.x - f64::from(feature.x);
        let dz = position.z - f64::from(feature.z);
        let distance_sqr = dx * dx + dz * dz;
        let matches_player = self.player_matcher(connection_id, USED_ENDER_EYE);
        let compiled = &self.cold.advancement_triggers.used_ender_eye;
        let matching = player
            .cold
            .advancements
            .trigger_listeners(USED_ENDER_EYE)
            .filter(|(index, criterion)| {
                compiled
                    .get(index)
                    .and_then(|criteria| criteria.get(*criterion))
                    .is_some_and(|condition| {
                        condition.distance.matches_sqr(distance_sqr)
                            && matches_player(&condition.player)
                    })
            })
            .map(|(index, criterion)| (index, criterion.clone()))
            .collect();
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
