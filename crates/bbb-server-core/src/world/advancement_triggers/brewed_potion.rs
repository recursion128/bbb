use crate::connection::ConnectionId;
use crate::loot_runtime::matches_conditions;
use bbb_protocol::potion_types::vanilla_potion_for_resource_id;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;

use super::{construct_beacon::player_conditions, ServerWorld, WorldEvent};

pub(super) const BREWED_POTION: &str = "minecraft:brewed_potion";

pub(super) struct BrewedPotionCondition {
    potion: Option<i32>,
    player: Vec<LootCondition>,
}

impl BrewedPotionCondition {
    // BrewedPotionTrigger.java:22-29,35-37.
    pub(super) fn parse(
        value: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        let potion = value
            .field("potion")
            .map(|value| {
                let id = bbb_server_datapack::loot::codec::id(value)
                    .map_err(|error| error.to_string())?;
                vanilla_potion_for_resource_id(&id)
                    .map(|potion| potion.id)
                    .ok_or_else(|| format!("unknown potion {id}"))
            })
            .transpose()?;
        Ok(Self {
            potion,
            player: player_conditions(value.field("player"), BREWED_POTION, lookup)?,
        })
    }
}

impl ServerWorld {
    // BrewedPotionTrigger.java:18-19,35-37; SimpleCriterionTrigger.java:43-70.
    pub(in crate::world) fn trigger_brewed_potion(
        &mut self,
        connection_id: ConnectionId,
        potion: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(BREWED_POTION)
            .next()
            .is_none()
        {
            return;
        }
        let Some(startup) = self.cold.config.datapacks.as_ref() else {
            return;
        };
        let Some(matching) = self.with_advancement_player_params(player, |level, params| {
            let advancement_triggers = &self.cold.advancement_triggers;
            let mut matching = Vec::new();
            for (index, criterion) in player.cold.advancements.trigger_listeners(BREWED_POTION) {
                let Some(condition) = advancement_triggers
                    .brewed_potion
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                if condition.potion.is_some_and(|expected| expected != potion) {
                    continue;
                }
                if matches_conditions(
                    &startup.loot,
                    BREWED_POTION,
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
