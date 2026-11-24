//! The level half of a lightning strike: the villager's witch conversion,
//! which needs `finalizeSpawn`, a session's fire and the strike triggers.

use super::*;
use crate::entity::conversion::SingleMobConversionSnapshot;
use crate::entity::lightning_bolt::LightningAction;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::world::{LevelId, WorldEvent};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID;

impl ServerWorld {
    pub(in crate::world) fn apply_lightning_action(
        &mut self,
        dimension: &LevelId,
        action: LightningAction,
        events: &mut Vec<WorldEvent>,
    ) {
        match action {
            LightningAction::VillagerToWitch { villager_id } => {
                let _ = self.convert_villager_to_witch(dimension, villager_id);
            }
            LightningAction::Hurt {
                entity_id,
                amount,
                registry_tags,
            } => {
                let hurt = crate::entity::projectile_action::PendingProjectileHurt {
                    target: crate::level_event::PendingHurtEntity::Stored { entity_id },
                    damage_type: "minecraft:lightning_bolt",
                    amount,
                    causing: None,
                    direct: None,
                    direct_resource_id: "minecraft:lightning_bolt",
                    registry_tags,
                };
                let _ = self.apply_projectile_hurt(*dimension, hurt, None, events);
            }
            LightningAction::IgnitePlayer { player } => {
                // `Entity.java:2763-2766`.
                if let Some(player) = self
                    .connection_of(player)
                    .and_then(|id| self.levels.player_mut(id))
                {
                    player.set_remaining_fire_ticks(player.remaining_fire_ticks + 1);
                    if player.remaining_fire_ticks == 0 {
                        player.ignite_environment(160);
                    }
                }
            }
            LightningAction::Channeled { cause, victims } => {
                if let Some(connection_id) = self.connection_of(cause) {
                    self.trigger_channeled_lightning(connection_id, dimension, &victims, events);
                }
            }
            LightningAction::Faded { bolt, bystanders } => {
                self.trigger_lightning_strike(dimension, &bolt, &bystanders, events);
            }
        }
    }

    /// `convertTo(WITCH, single(this, false, false), ...)` with
    /// `finalizeSpawn(CONVERSION)` and `setPersistenceRequired`
    /// (`Villager.java:759-763`).
    fn convert_villager_to_witch(&mut self, dimension: &LevelId, villager_id: i32) -> Option<()> {
        let tree = self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            let source = runtime
                .entities
                .get(villager_id)
                .filter(|source| source.removal_reason().is_none())?;
            let snapshot = SingleMobConversionSnapshot::capture_with(source, false, false);
            let mut tree = create_natural_spawn_root(
                runtime.entities,
                VANILLA_ENTITY_TYPE_WITCH_ID,
                snapshot.position(),
            )?;
            snapshot.apply(&mut tree);
            let _ = finalize_spawn(runtime, &mut tree, SpawnReason::Conversion, None, random);
            crate::entity::conversion::set_persistence_required(&mut tree);
            Some(tree)
        })??;
        self.with_stored_hurt_level(dimension, |entities, level| {
            let witch_id = entities.add_fresh(tree);
            entities.with_entity_mut(villager_id, |source, entities| {
                crate::entity::villager::release_all_pois(source, entities, level);
                crate::entity::conversion::transfer_single_relations_from(
                    source, entities, witch_id,
                );
            })?;
            entities.remove(villager_id, crate::entity::RemovalReason::Discarded);
            Some(witch_id)
        })?;
        Some(())
    }
}
