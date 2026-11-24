use crate::commands::{CommandSource, CommandSourceIdentity};
use crate::entity::living::mob_effects::{
    add_active_effect_in_level, remove_active_effect_in_level, remove_all_active_effects_in_level,
    ActiveMobEffect,
};
use crate::world::combat::EnchantmentEffectEntity as SourceEntity;
use crate::world::LevelId;

use super::*;

impl ServerWorld {
    /// `DamageCommand.damage`'s `target.hurtServer` (`DamageCommand.java:104`).
    #[expect(
        clippy::too_many_arguments,
        reason = "one parameter per DamageSource part"
    )]
    pub(crate) fn damage_by_command(
        &mut self,
        target: &CommandAttributeTarget,
        damage_type: &str,
        amount: f32,
        causing: Option<CommandDamageSourceEntity>,
        direct: Option<CommandDamageSourceEntity>,
        source_position: Option<[f64; 3]>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        match target {
            CommandAttributeTarget::Player(player) => self.hurt_player_by_command(
                player,
                crate::world::combat::LevelPlayerDamage {
                    damage_type,
                    amount,
                    causing,
                    direct,
                    source_position,
                },
                events,
            ),
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => self
                .hurt_stored_entity_by_command(
                    self.require_level_id(dimension),
                    *entity_id,
                    damage_type,
                    amount,
                    causing,
                    direct,
                    source_position,
                    events,
                )
                .is_some_and(|outcome| outcome.succeeded),
        }
    }

    /// `LivingEntity.addEffect` (`EffectCommands.java:164-171`): whether the
    /// target took the effect.
    pub(crate) fn give_command_effect(
        &mut self,
        source: &CommandSource,
        target: &CommandAttributeTarget,
        effect: ActiveMobEffect,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let source = match &source.identity {
            CommandSourceIdentity::Player => self.connection_of(source.uuid).and_then(|id| {
                self.levels
                    .player(id)
                    .map(|player| (player.dimension, SourceEntity::Session(id)))
            }),
            CommandSourceIdentity::StoredEntity {
                entity_id,
                dimension,
                ..
            } => self.level_id(dimension).and_then(|dimension| {
                self.levels
                    .get(&dimension)?
                    .entities
                    .get(*entity_id)
                    .filter(|entity| entity.uuid() == source.uuid)
                    .map(|_| (dimension, SourceEntity::Stored(*entity_id)))
            }),
            CommandSourceIdentity::Server | CommandSourceIdentity::Block { .. } => None,
        };
        match target {
            CommandAttributeTarget::Player(player) => {
                self.command_player_connection(player)
                    .and_then(|connection| {
                        self.add_player_mob_effect_from(connection, effect, source, events)
                    })
                    == Some(true)
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                self.add_stored_mob_effect(&self.require_level_id(dimension), *entity_id, effect)
                    == Some(true)
            }
        }
    }

    /// `removeEffect`, or `removeAllEffects` for `None`
    /// (`EffectCommands.java:196-200`, `:219-223`): whether anything went.
    pub(crate) fn clear_command_effect(
        &mut self,
        target: &CommandAttributeTarget,
        resource_id: Option<&str>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        match target {
            CommandAttributeTarget::Player(player) => {
                let Some(connection) = self.command_player_connection(player) else {
                    return false;
                };
                match resource_id {
                    Some(id) => self
                        .remove_player_mob_effect(connection, id, events)
                        .is_some(),
                    None => !self
                        .remove_all_player_mob_effects(connection, events)
                        .is_empty(),
                }
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => self.clear_stored_mob_effect(
                &self.require_level_id(dimension),
                *entity_id,
                resource_id,
            ),
        }
    }

    /// `LivingEntity.addEffect` on a stored entity; `None` when the entity or
    /// the level's tags cannot answer.
    pub(in crate::world) fn add_stored_mob_effect(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        effect: ActiveMobEffect,
    ) -> Option<bool> {
        self.with_stored_hurt_level(dimension, |store, level| {
            store.with_entity_mut(entity_id, |entity, store| {
                add_active_effect_in_level(entity, store, level, effect)
            })?
        })
    }

    /// `removeEffect`, or `removeAllEffects` for `None`, on a stored entity:
    /// whether anything was removed.
    fn clear_stored_mob_effect(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        resource_id: Option<&str>,
    ) -> bool {
        self.with_stored_hurt_level(dimension, |store, level| {
            let Some(mut entity) = store.detach(entity_id) else {
                return false;
            };
            let removed = match resource_id {
                Some(id) => remove_active_effect_in_level(&mut entity, store, level, id)
                    .flatten()
                    .is_some(),
                None => remove_all_active_effects_in_level(&mut entity, store, level)
                    .is_some_and(|removed| !removed.is_empty()),
            };
            store.attach(entity);
            removed
        })
    }
}
