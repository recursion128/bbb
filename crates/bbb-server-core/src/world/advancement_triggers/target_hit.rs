//! `TargetBlockTrigger` (`TargetBlockTrigger.java:15-53`): the signal a
//! projectile struck from a target block, and the projectile in the player's
//! context.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{int_bounds, LootCondition, LootEntity, ServerWorld, StoredLootEntity, WorldEvent};
use crate::connection::ConnectionId;
use crate::entity::Entity;
use crate::world::LevelId;
use bbb_server_datapack::loot::IntBounds;

pub(super) const TARGET_HIT: &str = "minecraft:target_hit";

pub(super) struct TargetHitCondition {
    player: Vec<LootCondition>,
    signal_strength: IntBounds,
    projectile: Vec<LootCondition>,
}

impl TargetHitCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                TARGET_HIT,
                lookup,
            )?,
            signal_strength: int_bounds(conditions.field("signal_strength"))?,
            projectile: super::construct_beacon::player_conditions(
                conditions.field("projectile"),
                "projectile",
                lookup,
            )?,
        })
    }
}

impl ServerWorld {
    /// `TargetBlockTrigger.trigger` (`:21-24`) and `TriggerInstance.matches`
    /// (`:46-48`).
    pub(in crate::world) fn trigger_target_hit(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        projectile: &Entity,
        signal: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.listens(connection_id, TARGET_HIT) {
            return;
        }
        let matching = {
            let Some(matches) = self.entity_matcher(connection_id, dimension, TARGET_HIT) else {
                return;
            };
            let projectile = LootEntity::Stored(StoredLootEntity {
                entity: projectile,
                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                    &self.level_by_id(dimension).entities,
                    None,
                    None,
                )),
                raid_at_position: None,
            });
            let matches_player = self.player_matcher(connection_id, TARGET_HIT);
            let compiled = &self.cold.advancement_triggers.target_hit;
            self.listening_criteria(connection_id, TARGET_HIT, |index, criterion| {
                let Some(condition) = compiled
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    return false;
                };
                condition.signal_strength.matches(signal)
                    && matches(projectile, &condition.projectile)
                    && matches_player(&condition.player)
            })
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
