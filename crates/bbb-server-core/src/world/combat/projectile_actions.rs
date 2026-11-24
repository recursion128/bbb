//! Queued projectile hits, deflections, and on-death/on-hurt mob-effect actions.

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_support_facts, VanillaBlockSupportType,
};
use bbb_protocol::block_types::VANILLA_BLOCK_COBWEB_ID;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{BlockPos, Direction, SoundSource, Vec3d};
use bbb_server_types::block_queries::face_sturdy_at;
use bbb_server_types::fluid::block_state_is_air;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_worldgen::tree::block_pos_set::BlockPosSet;

use crate::connection::ConnectionId;
use crate::entity::projectile_action::mob_effect::{
    spawn_infested_silverfish, spawn_oozing_slime, PendingMobEffectAction, PendingRemovalMobEffect,
    RemovedLivingSnapshot,
};
use crate::entity::projectile_action::{
    PendingProjectileAction, PendingProjectileHurt, PendingProjectileMobEffect, StoredHurtResult,
};
use crate::entity::store::PendingEntitySound;
use crate::entity::{
    StoredDamageCausingEntity, StoredEntityDamageContext, StoredEntityDamageRules,
    StoredEntityDamageSource,
};
use crate::explosion::{
    ExplosionInteraction, ExplosionPresentation, ExplosionSource, ExplosionSourceEntity,
    ExplosionSourceEntityKind, PendingExplosion,
};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::{PendingHurtEntity, VanillaGameEvent};
use crate::world::{block_position, stored_block_state, LevelId, ServerWorld, WorldEvent};

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn queue_projectile_action_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        action: PendingProjectileAction,
    ) {
        self.level_mut(dimension)
            .entities
            .queue_command(crate::entity::EntityCommand::Projectile(action));
    }

    #[cfg(test)]
    pub(crate) fn drain_entity_lifecycle_events_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<crate::entity::EntityLifecycleEvent> {
        self.level_mut(dimension).entities.drain_lifecycle_events()
    }

    pub(in crate::world) fn apply_projectile_action(
        &mut self,
        dimension: LevelId,
        action: PendingProjectileAction,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        match action {
            PendingProjectileAction::Deflect {
                target_entity_id,
                new_owner,
                direction,
            } => {
                if let Some(entity) = self
                    .level_mut_by_id(&dimension)
                    .entities
                    .get_mut(target_entity_id)
                {
                    crate::entity::hurting_projectile::apply_projectile_deflection(
                        entity, new_owner, direction,
                    );
                }
            }
            PendingProjectileAction::Hurt(hurt) => {
                if hurt.damage_type == "minecraft:wind_charge" {
                    let owner = match hurt.causing {
                        Some(PendingHurtEntity::Player { uuid }) => Some(uuid),
                        _ => None,
                    };
                    self.set_owner_last_hurt_mob(&dimension, owner, &hurt.target);
                }
                let accepted = self.apply_projectile_hurt(dimension, hurt.clone(), None, events)?;
                self.projectile_post_attack(&dimension, &hurt, accepted, events)
                    .map_err(|detail| tracing::warn!("{detail}"))
                    .ok()?;
            }
            PendingProjectileAction::ArrowHit(hit) => {
                self.apply_arrow_hit(dimension, *hit, events)?;
            }
            PendingProjectileAction::IgniteThenHurt(ignite) => {
                let target_player = match ignite.hurt.target {
                    PendingHurtEntity::Player { uuid } => self.connection_of(uuid),
                    _ => None,
                };
                // SmallFireball.java:38-42.
                let old_fire_ticks = match ignite.hurt.target {
                    PendingHurtEntity::Player { .. } => target_player
                        .and_then(|id| self.levels.player_mut(id))
                        .map(|player| {
                            let old = player.remaining_fire_ticks;
                            player.ignite_environment(ignite.fire_ticks);
                            old
                        }),
                    PendingHurtEntity::Stored { entity_id } => self
                        .level_mut_by_id(&dimension)
                        .entities
                        .get_mut(entity_id)
                        .map(|entity| {
                            let old = entity.remaining_fire_ticks();
                            crate::entity::living::ignite_living_for_ticks(
                                entity,
                                ignite.fire_ticks,
                            );
                            old
                        }),
                    PendingHurtEntity::FallingBlock { .. } => None,
                };
                let succeeded =
                    self.apply_projectile_hurt(dimension, ignite.hurt.clone(), None, events)?;
                if let (false, true, Some(old)) =
                    (succeeded, ignite.restore_fire_on_refusal, old_fire_ticks)
                {
                    match ignite.hurt.target {
                        PendingHurtEntity::Player { .. } => {
                            if let Some(player) =
                                target_player.and_then(|id| self.levels.player_mut(id))
                            {
                                player.set_remaining_fire_ticks(old);
                            }
                        }
                        PendingHurtEntity::Stored { entity_id } => {
                            if let Some(entity) =
                                self.level_mut_by_id(&dimension).entities.get_mut(entity_id)
                            {
                                entity.set_remaining_fire_ticks(old);
                            }
                        }
                        PendingHurtEntity::FallingBlock { .. } => {}
                    }
                }
                self.projectile_post_attack(&dimension, &ignite.hurt, succeeded, events)
                    .map_err(|detail| tracing::warn!("{detail}"))
                    .ok()?;
            }
            PendingProjectileAction::HurtThenMobEffect { hurt, effect } => {
                let accepted =
                    self.apply_projectile_hurt(dimension, hurt.clone(), Some(&effect), events)?;
                if accepted {
                    self.apply_player_projectile_effect(dimension, &hurt.target, &effect, events);
                }
                self.projectile_post_attack(&dimension, &hurt, accepted, events)
                    .map_err(|detail| tracing::warn!("{detail}"))
                    .ok()?;
            }
            PendingProjectileAction::PlaceFire { pos } => {
                let should_place = self
                    .block_state(dimension, pos)
                    .is_some_and(block_state_is_air);
                if should_place {
                    // `BaseFireBlock.getState` (`SmallFireball.java:55-57`).
                    let chunks = &self.level_by_id(&dimension).chunks;
                    let tags = chunks
                        .registry_tags()
                        .unwrap_or_else(|| crate::registry_tags::RegistryTagSnapshot::builtin());
                    let fire =
                        crate::block_behavior::fire::state_for_placement(pos, tags, &mut |at| {
                            stored_block_state(chunks, at)
                        });
                    if let Some(fire) = fire {
                        self.set_block(dimension, pos, fire, SET_BLOCK_UPDATE_ALL);
                    }
                }
            }
            PendingProjectileAction::Land {
                origin,
                source_entity_id,
            } => match origin {
                crate::entity::projectile_action::ProjectileLandOrigin::Entity(position) => {
                    self.level_mut_by_id(&dimension)
                        .event_sink
                        .game_event_with_source_at(
                            VanillaGameEvent::PROJECTILE_LAND,
                            position,
                            Some(source_entity_id),
                            None,
                        );
                }
                crate::entity::projectile_action::ProjectileLandOrigin::Block(pos) => {
                    let state = stored_block_state(&self.level_by_id(&dimension).chunks, pos);
                    self.level_mut_by_id(&dimension)
                        .event_sink
                        .game_event_with_source(
                            VanillaGameEvent::PROJECTILE_LAND,
                            pos,
                            Some(source_entity_id),
                            Some(state),
                        );
                }
            },
            PendingProjectileAction::Discard { entity_id } => {
                self.level_mut_by_id(&dimension)
                    .entities
                    .remove(entity_id, crate::entity::RemovalReason::Discarded);
            }
            PendingProjectileAction::Destroy {
                entity_id,
                position,
            } => {
                self.level_mut_by_id(&dimension)
                    .entities
                    .remove(entity_id, crate::entity::RemovalReason::Discarded);
                self.level_mut_by_id(&dimension)
                    .event_sink
                    .game_event_with_source_at(
                        VanillaGameEvent::ENTITY_DAMAGE,
                        position,
                        Some(entity_id),
                        None,
                    );
            }
            PendingProjectileAction::HitBlockEffects(hit) => {
                self.projectile_hit_block_effects(&dimension, *hit, events);
            }
            PendingProjectileAction::TargetHit {
                player,
                projectile,
                signal,
            } => {
                if let Some(connection_id) = self.connection_of(player) {
                    self.trigger_target_hit(connection_id, &dimension, &projectile, signal, events);
                }
            }
        }
        Some(())
    }

    pub(in crate::world) fn apply_mob_effect_action(
        &mut self,
        dimension: LevelId,
        action: PendingMobEffectAction,
        events: &mut Vec<WorldEvent>,
    ) {
        match action {
            PendingMobEffectAction::Removed(mut action) => {
                let kind = if action.source.is_mob {
                    ExplosionSourceEntityKind::Mob
                } else {
                    ExplosionSourceEntityKind::LivingOther
                };
                let effects = std::mem::take(&mut action.effects);
                self.trigger_removal_mob_effects(dimension, action.source, kind, effects, events);
                self.level_mut_by_id(&dimension)
                    .entities
                    .publish_deferred_removed(action.removed_entity);
            }
            PendingMobEffectAction::Infested {
                position,
                velocities,
            } => {
                spawn_infested_children(self.level_mut_by_id(&dimension), position, velocities);
            }
        }
    }

    /// `MobEffect.onMobRemoved` for each effect `triggerOnDeathMobEffects`
    /// walks (LivingEntity.java:714-720): OozingMobEffect.java:35,
    /// WeavingMobEffect.java:26, WindChargedMobEffect.java:18.
    fn trigger_removal_mob_effects(
        &mut self,
        dimension: LevelId,
        source: RemovedLivingSnapshot,
        kind: ExplosionSourceEntityKind,
        effects: Vec<PendingRemovalMobEffect>,
        events: &mut Vec<WorldEvent>,
    ) {
        for effect in effects {
            match effect {
                PendingRemovalMobEffect::Oozing => {
                    self.spawn_oozing_slimes(dimension, source);
                }
                PendingRemovalMobEffect::Weaving => {
                    self.place_weaving_cobwebs(
                        dimension,
                        source.block_position,
                        source.entity_resource_id == "minecraft:player",
                    );
                }
                PendingRemovalMobEffect::WindCharged => {
                    // `WindChargedMobEffect` draws radius before its
                    // mob-sourced trigger explosion (`WindChargedMobEffect.java:18-39`).
                    let radius = 3.0 + self.random.next_float() * 2.0;
                    let direct = ExplosionSourceEntity {
                        custom_name: None,
                        entity_id: source.entity_id,
                        entity_resource_id: source.entity_resource_id.to_string(),
                        uuid: source.uuid,
                        position: source.position,
                        kind,
                    };
                    self.explode(
                        dimension,
                        PendingExplosion {
                            source: Some(ExplosionSource {
                                indirect: Some(direct.clone()),
                                causing: Some(direct.clone()),
                                direct,
                                in_water: source.was_touching_water,
                                wind_charge: false,
                            }),
                            center: source.bounding_box.center(),
                            radius,
                            fire: false,
                            interaction: ExplosionInteraction::Trigger,
                            damage_source: None,
                            damage_profile: crate::world::explosion::wind_charged_burst_profile(),
                            presentation: ExplosionPresentation::wind_charge(
                                "minecraft:entity.breeze.wind_burst",
                            ),
                        },
                        events,
                    );
                }
            }
        }
    }

    /// `LivingEntity.remove(KILLED)` (LivingEntity.java:697-700) of a dead
    /// session's avatar: its on-death effects fire and every effect is dropped.
    pub(in crate::world) fn trigger_player_on_death_effects(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let removed = player.effects.take_all();
        let effects: Vec<_> = removed
            .iter()
            .filter_map(|effect| match effect.resource_id() {
                "minecraft:oozing" => Some(PendingRemovalMobEffect::Oozing),
                "minecraft:weaving" => Some(PendingRemovalMobEffect::Weaving),
                "minecraft:wind_charged" => Some(PendingRemovalMobEffect::WindCharged),
                _ => None,
            })
            .collect();
        if effects.is_empty() {
            return;
        }
        let Ok(bounding_box) = player.bounding_box() else {
            return;
        };
        let position = player.position();
        let source = RemovedLivingSnapshot {
            entity_id: player.entity_id(),
            entity_resource_id: "minecraft:player",
            uuid: player.profile.uuid,
            is_mob: false,
            position,
            block_position: block_position(position),
            bounding_box,
            was_touching_water: player.was_touching_water,
        };
        let kind = ExplosionSourceEntityKind::Player {
            instabuild: player.abilities.instabuild,
            may_build: player.may_build(),
            spectator: player.game_type == bbb_protocol::game_types::GameType::Spectator,
        };
        let dimension = player.dimension;
        self.trigger_removal_mob_effects(dimension, source, kind, effects, events);
    }

    fn spawn_oozing_slimes(&mut self, dimension: LevelId, source: RemovedLivingSnapshot) {
        // Cramming clamps the constant-two request before any child is created
        // (`OozingMobEffect.java:29-64`).
        let max_cramming = match self.admin.game_rule_value("minecraft:max_entity_cramming") {
            Some(GameRuleValue::Integer(value)) => value,
            Some(GameRuleValue::Boolean(_)) | None => 24,
        };
        let count = if max_cramming < 1 {
            2
        } else {
            let query = source.bounding_box.inflate(2.0, 2.0, 2.0);
            let nearby = self
                .level_by_id(&dimension)
                .entities
                .ids_in_box_sections(query)
                .into_iter()
                .filter(|id| {
                    self.level_by_id(&dimension)
                        .entities
                        .get(*id)
                        .is_some_and(|entity| {
                            entity.entity_type_id()
                                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SLIME_ID
                                && entity.bounding_box().intersects(query)
                        })
                })
                .take(max_cramming as usize)
                .count() as i32;
            (max_cramming - nearby).clamp(0, 2)
        };
        let position = Vec3d {
            x: source.position.x,
            y: source.position.y + 0.5,
            z: source.position.z,
        };
        for _ in 0..count {
            let y_rot = self.random.next_float() * 360.0;
            let _ = spawn_oozing_slime(
                &mut self.level_mut_by_id(&dimension).entities,
                position,
                y_rot,
            );
        }
    }

    fn place_weaving_cobwebs(&mut self, dimension: LevelId, center: BlockPos, player_source: bool) {
        let random = self.random.clone();
        if !player_source && !self.game_rule_bool("minecraft:mob_griefing", true) {
            return;
        }
        // `WeavingMobEffect` draws the count before lazily walking at most 15
        // cube candidates (`WeavingMobEffect.java:26-51`).
        let wanted = random.next_int(2) + 2;
        let Some(cobweb) = vanilla_block_default_state_id(VANILLA_BLOCK_COBWEB_ID) else {
            return;
        };
        let mut positions = BlockPosSet::new();
        for _ in 0..15 {
            let pos = BlockPos {
                x: center.x + random.next_int(3) - 1,
                y: center.y + random.next_int(3) - 1,
                z: center.z + random.next_int(3) - 1,
            };
            if positions.contains(pos) {
                continue;
            }
            let below = BlockPos {
                y: pos.y - 1,
                ..pos
            };
            let Some(state) = self.block_state(dimension, pos) else {
                continue;
            };
            let Some(replaceable) =
                vanilla_block_state_support_facts(state).map(|facts| facts.replaceable())
            else {
                continue;
            };
            if !replaceable {
                continue;
            }
            let Some(below_state) = self.block_state(dimension, below) else {
                continue;
            };
            if !face_sturdy_at(
                below_state,
                below,
                Direction::Up,
                VanillaBlockSupportType::Full,
            ) {
                continue;
            }
            positions.add(pos);
            if positions.iter().count() >= wanted as usize {
                break;
            }
        }
        for pos in positions.iter() {
            let _ = self.set_block(dimension, pos, cobweb, SET_BLOCK_UPDATE_ALL);
            self.level_mut_by_id(&dimension)
                .event_sink
                .level_event(3018, pos, 0);
        }
    }

    pub(in crate::world) fn apply_projectile_hurt(
        &mut self,
        dimension: LevelId,
        hurt: PendingProjectileHurt,
        effect: Option<&PendingProjectileMobEffect>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        match hurt.target {
            PendingHurtEntity::Player { uuid } => {
                let Some(connection_id) = self.connection_of(uuid) else {
                    return Some(false);
                };
                let causing = hurt
                    .causing
                    .and_then(|entity| self.pending_hurt_source_entity(&dimension, entity));
                let direct = hurt.direct.and_then(|entity| {
                    self.pending_hurt_source_entity(&dimension, entity.clone())
                        .or_else(|| projectile_direct_source(entity, hurt.direct_resource_id))
                });
                self.hurt_connected_player_checked(
                    connection_id,
                    hurt.damage_type,
                    hurt.amount,
                    causing,
                    direct,
                    None,
                    events,
                )
            }
            PendingHurtEntity::Stored { .. } => {
                self.apply_projectile_hurt_to_stored(dimension, &hurt, effect, events)
            }
            PendingHurtEntity::FallingBlock { .. } => Some(false),
        }
    }

    fn apply_projectile_hurt_to_stored(
        &mut self,
        dimension: LevelId,
        hurt: &PendingProjectileHurt,
        effect: Option<&PendingProjectileMobEffect>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let Some(damage_type) = self
            .cold
            .damage_types
            .iter()
            .find(|entry| entry.resource_id == hurt.damage_type)
            .cloned()
        else {
            return Some(false);
        };
        let direct_entity_id = hurt.direct.as_ref().and_then(|entity| match entity {
            PendingHurtEntity::Player { uuid } => self
                .connection_of(*uuid)
                .and_then(|id| self.levels.player(id))
                .map(|player| player.entity_id()),
            PendingHurtEntity::Stored { entity_id }
            | PendingHurtEntity::FallingBlock { entity_id, .. } => Some(*entity_id),
        });
        // `LootContextParams.ATTACKING_ENTITY` is `source.getEntity()`, which
        // is a player when a player owns the projectile that landed.
        let attacker_views = self.session_attacker_views(
            match hurt.causing {
                Some(PendingHurtEntity::Player { uuid }) => Some(uuid),
                _ => None,
            },
            match hurt.direct {
                Some(PendingHurtEntity::Player { uuid }) => Some(uuid),
                _ => None,
            },
        );
        let (causing_entity, attacker_may_build, causing_entity_snapshot) = match hurt.causing {
            None => (None, true, None),
            Some(PendingHurtEntity::Player { uuid }) => {
                let Some(player) = self
                    .connection_of(uuid)
                    .and_then(|connection_id| self.levels.player(connection_id))
                else {
                    return Some(false);
                };
                (
                    Some(StoredDamageCausingEntity::Player {
                        uuid,
                        entity_id: player.entity_id(),
                        instabuild: player.abilities.instabuild,
                    }),
                    player.may_build(),
                    Some(ExplosionSourceEntity {
                        custom_name: None,
                        entity_id: player.entity_id(),
                        entity_resource_id: "minecraft:player".to_string(),
                        uuid,
                        position: player.position(),
                        kind: ExplosionSourceEntityKind::Player {
                            instabuild: player.abilities.instabuild,
                            may_build: player.may_build(),
                            spectator: player.game_type
                                == bbb_protocol::game_types::GameType::Spectator,
                        },
                    }),
                )
            }
            Some(PendingHurtEntity::FallingBlock {
                entity_id,
                uuid,
                position,
                ..
            }) => (
                Some(StoredDamageCausingEntity::Other { entity_id }),
                true,
                Some(ExplosionSourceEntity {
                    custom_name: None,
                    entity_id,
                    entity_resource_id: "minecraft:falling_block".to_string(),
                    uuid,
                    position,
                    kind: ExplosionSourceEntityKind::Other,
                }),
            ),
            Some(PendingHurtEntity::Stored { entity_id }) => {
                let causing_entity_snapshot = self
                    .level_by_id(&dimension)
                    .entities
                    .get(entity_id)
                    .map(crate::entity::damage::explosion_source_entity);
                let Some(causing) =
                    crate::entity::projectile_action::resolve_stored_damage_causing_entity(
                        &self.level_by_id(&dimension).entities,
                        entity_id,
                    )
                else {
                    return Some(false);
                };
                (Some(causing), true, causing_entity_snapshot)
            }
        };
        let rules = StoredEntityDamageRules {
            mob_griefing: self.game_rule_bool("minecraft:mob_griefing", true),
            entity_drops: self.game_rule_bool("minecraft:entity_drops", true),
            tnt_explodes: self.game_rule_bool("minecraft:tnt_explodes", true),
            fall_damage_immune: false,
        };
        let context = StoredEntityDamageContext {
            block_drops: self.game_rule_bool("minecraft:block_drops", true),
            attacker_may_build,
            attacker_spectator: causing_entity_snapshot.as_ref().and_then(|source| {
                match source.kind {
                    ExplosionSourceEntityKind::Player { spectator, .. } => Some(spectator),
                    _ => None,
                }
            }),
        };
        let result = self.with_stored_hurt_level_checked(&dimension, None, |entities, level, _| {
            match crate::entity::projectile_action::hurt_projectile_to_stored_with_causing_snapshot(
                (hurt, causing_entity_snapshot),
                effect,
                StoredEntityDamageSource {
                    damage_type: &damage_type,
                    direct_entity_id,
                    causing_entity,
                    attacker_views: attacker_views.views(),
                },
                rules,
                context,
                entities,
                level,
            ) {
                StoredHurtResult::Applied { outcome, .. } => Some(outcome.succeeded),
                StoredHurtResult::DropNoop => Some(false),
            }
        })?;
        // The direct projectile is still stored here; an arrow can be discarded
        // by the caller immediately after this hurt returns.
        let _ = self.run_entity_commands(&dimension, events);
        self.flush_player_hurt_entities(&dimension, events);
        result
    }
}

fn projectile_direct_source(
    direct: PendingHurtEntity,
    resource_id: &'static str,
) -> Option<crate::commands::CommandDamageSourceEntity> {
    let PendingHurtEntity::Stored { entity_id } = direct else {
        return None;
    };
    let (namespace, path) = resource_id.split_once(':')?;
    Some(crate::commands::CommandDamageSourceEntity::Stored {
        entity_id,
        entity_resource_id: resource_id.to_string(),
        uuid: None,
        position: None,
        is_mob: false,
        is_living_non_player: false,
        display_name: crate::commands::translatable(
            format!("entity.{namespace}.{path}"),
            Vec::new(),
        ),
    })
}

/// Entity creation, world-yaw draw, add, then hurt sound per child
/// (`InfestedMobEffect.java:38-50`).
pub(in crate::world) fn spawn_infested_children(
    level: &mut crate::world::ServerLevel,
    position: Vec3d,
    velocities: Vec<Vec3d>,
) {
    for velocity in velocities {
        let y_rot = level.random.next_float() * 360.0;
        if spawn_infested_silverfish(&mut level.entities, position, y_rot, velocity).is_some() {
            level.entities.play_entity_sound(PendingEntitySound {
                position,
                resource_id: "minecraft:entity.silverfish.hurt".to_owned(),
                source: SoundSource::Hostile,
                volume: 1.0,
                pitch: 1.0,
            });
        }
    }
}

#[cfg(test)]
mod projectile_tests {
    use super::*;

    #[test]
    fn discarded_projectile_keeps_its_direct_damage_identity() {
        let source = projectile_direct_source(
            PendingHurtEntity::Stored { entity_id: 41 },
            "minecraft:fireball",
        )
        .expect("the queued snapshot does not require a live store entry");
        assert!(matches!(
            source,
            crate::commands::CommandDamageSourceEntity::Stored {
                entity_id: 41,
                ref entity_resource_id,
                uuid: None,
                position: None,
                is_mob: false,
                is_living_non_player: false,
                ..
            } if entity_resource_id == "minecraft:fireball"
        ));
    }
}
