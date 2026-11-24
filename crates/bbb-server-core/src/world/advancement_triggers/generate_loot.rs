//! `LootTableTrigger` (`LootTableTrigger.java:18-38`), fired as
//! `CriteriaTriggers.GENERATE_LOOT` when a player unpacks a block entity's loot
//! table.

use crate::connection::ConnectionId;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{LootCondition, ServerWorld, WorldEvent};

pub(super) const GENERATE_LOOT: &str = "minecraft:player_generates_container_loot";

pub(super) struct LootTableCondition {
    player: Vec<LootCondition>,
    loot_table: String,
}

impl LootTableCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                GENERATE_LOOT,
                lookup,
            )?,
            loot_table: bbb_server_datapack::loot::codec::id(
                conditions
                    .field("loot_table")
                    .ok_or("loot_table is missing")?,
            )
            .map_err(|error| error.to_string())?,
        })
    }
}

/// `LootTable.KEY_CODEC` reads an identifier, whose namespace defaults to
/// `minecraft`.
pub(in crate::world) fn namespaced(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

impl ServerWorld {
    /// `LootTableTrigger.trigger` for the table `loot_table` names.
    pub(in crate::world) fn trigger_generate_loot(
        &mut self,
        connection_id: ConnectionId,
        loot_table: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let matches_player = self.player_matcher(connection_id, GENERATE_LOOT);
        let loot_table = namespaced(loot_table);
        let mut matching = Vec::new();
        for (index, criterion) in player.cold.advancements.trigger_listeners(GENERATE_LOOT) {
            let Some(condition) = self
                .cold
                .advancement_triggers
                .generate_loot
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            else {
                continue;
            };
            if condition.loot_table == loot_table && matches_player(&condition.player) {
                matching.push((index, criterion.clone()));
            }
        }
        drop(matches_player);
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
