//! Player damage predicates over admitted hits and their actual sources.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{
    double_bounds, param_sets, warn_condition_check, DoubleBounds, LootCondition, LootEntity,
    ServerWorld, SourceEntity, StoredLootEntity, WorldEvent,
};
use crate::connection::ConnectionId;
use crate::loot_runtime::{LootDamageSource, LootDamageSourceFacts};
use crate::world::combat::PlayerHurtSource;
use crate::world::LevelId;

pub(super) const ENTITY_HURT_PLAYER: &str = "minecraft:entity_hurt_player";

pub(super) const PLAYER_HURT_ENTITY: &str = "minecraft:player_hurt_entity";

pub(super) struct HurtCondition {
    entity: Vec<LootCondition>,
    player: Vec<LootCondition>,
    dealt: DoubleBounds,
    taken: DoubleBounds,
    source_entity: Option<Vec<LootCondition>>,
    blocked: Option<bool>,
    damage_type: Vec<LootCondition>,
}

impl HurtCondition {
    // EntityHurtPlayerTrigger.java:21-27; DamagePredicate.java:19-43.
    pub(super) fn parse(
        conditions: V<'_>,
        trigger: &str,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        let damage = conditions.field("damage");
        if let Some(damage) = damage {
            if !damage.is_compound() {
                return Err("damage predicate must be an object".into());
            }
        }
        let field = |name| damage.and_then(|damage| damage.field(name));
        let condition = |condition: LootCondition, param_set| {
            let conditions = vec![condition];
            warn_condition_check(trigger, &conditions, param_set);
            Ok::<_, String>(conditions)
        };
        Ok(Self {
            entity: if trigger == PLAYER_HURT_ENTITY {
                super::construct_beacon::player_conditions(
                    conditions.field("entity"),
                    trigger,
                    lookup,
                )?
            } else {
                Vec::new()
            },
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                trigger,
                lookup,
            )?,
            dealt: double_bounds(field("dealt"))?,
            taken: double_bounds(field("taken"))?,
            source_entity: field("source_entity")
                .map(|predicate| super::entity_predicate_conditions(predicate, trigger))
                .transpose()?,
            blocked: field("blocked")
                .map(|value| {
                    value
                        .as_bool()
                        .ok_or_else(|| "blocked is not a boolean".to_string())
                })
                .transpose()?,
            damage_type: field("type")
                .map(|predicate| {
                    super::validate_predicate_source(predicate)?;
                    condition(
                        LootCondition::DamageSourceProperties(Some(Box::new(
                            bbb_server_datapack::loot::DamageSourcePredicate::decode(predicate)
                                .map_err(|error| error.to_string())?,
                        ))),
                        param_sets::ENTITY,
                    )
                })
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl ServerWorld {
    pub(in crate::world) fn apply_player_hurt_entity(
        &mut self,
        hit: crate::entity::store::PendingPlayerHurtEntity,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(attacker) = self.connection_of(hit.player_uuid) else {
            return;
        };
        if !self.listens(attacker, PLAYER_HURT_ENTITY) {
            return;
        }
        let registry = std::sync::Arc::clone(&self.cold.damage_types);
        let Some(damage_type) = usize::try_from(hit.damage_type_id)
            .ok()
            .and_then(|id| registry.get(id))
        else {
            return;
        };
        let direct = hit.direct_entity_id.map(|id| {
            self.connection_by_entity_id(id)
                .map_or(SourceEntity::Stored(id), SourceEntity::Session)
        });
        self.trigger_player_hurt_entity(
            attacker,
            SourceEntity::Stored(hit.victim_entity_id),
            PlayerHurtSource {
                damage_type,
                causing: Some(SourceEntity::Session(attacker)),
                direct,
            },
            HurtTriggerDetails {
                dealt: hit.dealt,
                taken: hit.taken,
                blocked: false,
            },
            events,
        );
    }

    pub(in crate::world) fn flush_player_hurt_entities(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let hits = self
            .level_mut_by_id(dimension)
            .entities
            .drain_commands_where(|command| {
                matches!(command, crate::entity::EntityCommand::PlayerHurtEntity(_))
            });
        for hit in hits {
            if let crate::entity::EntityCommand::PlayerHurtEntity(hit) = hit {
                self.apply_player_hurt_entity(hit, events);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::world) struct HurtTriggerDetails {
    pub(in crate::world) dealt: f32,
    pub(in crate::world) taken: f32,
    pub(in crate::world) blocked: bool,
}

impl ServerWorld {
    // LivingEntity.java:1269-1270: a fully blocked hit still fires, even though
    // hurtServer returns false; the caller has already checked admission.
    pub(in crate::world) fn trigger_entity_hurt_player(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        source: PlayerHurtSource<'_>,
        details: HurtTriggerDetails,
        events: &mut Vec<WorldEvent>,
    ) {
        self.trigger_hurt(
            connection_id,
            dimension,
            (ENTITY_HURT_PLAYER, SourceEntity::Session(connection_id)),
            source,
            details,
            events,
        );
    }

    pub(in crate::world) fn trigger_player_hurt_entity(
        &mut self,
        connection_id: ConnectionId,
        victim: SourceEntity,
        source: PlayerHurtSource<'_>,
        details: HurtTriggerDetails,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let dimension = player.dimension;
        self.trigger_hurt(
            connection_id,
            &dimension,
            (PLAYER_HURT_ENTITY, victim),
            source,
            details,
            events,
        );
    }

    fn trigger_hurt(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        (trigger, victim): (&str, SourceEntity),
        source: PlayerHurtSource<'_>,
        details: HurtTriggerDetails,
        events: &mut Vec<WorldEvent>,
    ) {
        let HurtTriggerDetails {
            dealt,
            taken,
            blocked,
        } = details;
        if !self.listens(connection_id, trigger) {
            return;
        }
        let matching = {
            let Some(matches_source) = self.entity_matcher(connection_id, dimension, trigger)
            else {
                return;
            };
            let Some(matches_damage) =
                self.entity_context_matcher(connection_id, dimension, trigger, param_sets::ENTITY)
            else {
                return;
            };
            let session = |entity| match entity {
                Some(SourceEntity::Session(id)) => self.session_loot_view(id),
                _ => None,
            };
            let victim_view = session(Some(victim));
            let causing_view = session(source.causing);
            let direct_view = session(source.direct);
            let entity = |source, view| match (source, view) {
                (_, Some(view)) => LootEntity::Facts(view),
                (Some(SourceEntity::Stored(id)), None) => self
                    .levels
                    .values()
                    .find_map(|level| {
                        level.entities.get(id).map(|entity| {
                            LootEntity::Stored(StoredLootEntity {
                                entity,
                                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                                    &level.entities,
                                    None,
                                    None,
                                )),
                                raid_at_position: Some(
                                    level.raids.raid_at(entity.block_position()),
                                ),
                            })
                        })
                    })
                    .unwrap_or(LootEntity::Opaque),
                (Some(_), None) => LootEntity::Opaque,
                (None, None) => LootEntity::Absent,
            };
            let victim = entity(Some(victim), victim_view.as_ref());
            let causing = entity(source.causing, causing_view.as_ref());
            let direct = entity(source.direct, direct_view.as_ref());
            let damage_source = LootDamageSource::Present(LootDamageSourceFacts {
                damage_type: &source.damage_type.resource_id,
                is_direct: Some(source.causing == source.direct),
                tags: self.cold.config.datapacks.as_deref().map(|packs| {
                    packs
                        .tags
                        .registry(bbb_server_datapack::damage::DAMAGE_TYPE_REGISTRY)
                }),
                causing,
                direct: Some(direct),
            });
            let matches_player = self.player_matcher(connection_id, trigger);
            let compiled = if trigger == PLAYER_HURT_ENTITY {
                &self.cold.advancement_triggers.player_hurt_entity
            } else {
                &self.cold.advancement_triggers.entity_hurt_player
            };
            self.listening_criteria(connection_id, trigger, |index, criterion| {
                let Some(condition) = compiled
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    return false;
                };
                condition.dealt.matches(f64::from(dealt))
                    && condition.taken.matches(f64::from(taken))
                    && condition.blocked.is_none_or(|expected| expected == blocked)
                    && condition.source_entity.as_ref().is_none_or(|conditions| {
                        causing.present() && matches_source(causing, conditions)
                    })
                    && matches_damage(victim, &condition.damage_type, damage_source)
                    && matches_source(victim, &condition.entity)
                    && matches_player(&condition.player)
            })
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
