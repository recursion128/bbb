//! `LightningStrikeTrigger` (`LightningStrikeTrigger.java:17-60`) and
//! `ChanneledLightningTrigger` (`ChanneledLightningTrigger.java:17-67`):
//! predicates over the entities a strike reached, in the player's own context.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;

use super::{
    matches_conditions, param_sets, LoadedBlocks, LootCondition, LootEntities, LootEntity,
    LootOrigin, LootParams, LootWeather, ServerWorld, StoredLootEntity, WorldEvent,
};
use crate::connection::ConnectionId;
use crate::entity::lightning_bolt::StrikeWitness;
use crate::entity::Entity;
use crate::loot_runtime::{LootContextParamSet, LootDamageSource};
use crate::world::LevelId;

pub(super) const LIGHTNING_STRIKE: &str = "minecraft:lightning_strike";
pub(super) const CHANNELED_LIGHTNING: &str = "minecraft:channeled_lightning";

pub(super) struct StrikeCondition {
    player: Vec<LootCondition>,
    lightning: Vec<LootCondition>,
    bystander: Option<Vec<LootCondition>>,
}

impl StrikeCondition {
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
                LIGHTNING_STRIKE,
                lookup,
            )?,
            lightning: predicate("lightning")?,
            bystander: conditions
                .field("bystander")
                .map(|_| predicate("bystander"))
                .transpose()?,
        })
    }
}

/// One `ContextAwarePredicate` per listed victim.
pub(super) struct VictimsCondition {
    player: Vec<LootCondition>,
    victims: Vec<Vec<LootCondition>>,
}

impl VictimsCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: super::construct_beacon::player_conditions(
                conditions.field("player"),
                CHANNELED_LIGHTNING,
                lookup,
            )?,
            victims: victim_predicates(conditions, lookup)?,
        })
    }
}

/// The `victims` list: one `ContextAwarePredicate` per entry.
pub(super) fn victim_predicates(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Vec<Vec<LootCondition>>, String> {
    match conditions.field("victims") {
        None => Ok(Vec::new()),
        Some(list) => (0..list
            .list_len()
            .ok_or_else(|| format!("{list:?} is not a victim list"))?)
            .map(|index| {
                let victim = list.list_element(index).unwrap();
                super::construct_beacon::player_conditions(Some(victim), "victims", lookup)
            })
            .collect(),
    }
}

impl ServerWorld {
    /// `LightningStrikeTrigger.trigger` for every player within 256 of the
    /// fading bolt (`LightningBolt.java:125-127`).
    pub(in crate::world) fn trigger_lightning_strike(
        &mut self,
        dimension: &LevelId,
        bolt: &Entity,
        bystanders: &[StrikeWitness],
        events: &mut Vec<WorldEvent>,
    ) {
        let position = bolt.position();
        let listeners: Vec<ConnectionId> = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                let at = player.position();
                let (dx, dy, dz) = (at.x - position.x, at.y - position.y, at.z - position.z);
                player.dimension == *dimension
                    && ((dx * dx + dy * dy + dz * dz).sqrt() as f32) < 256.0
                    && player
                        .cold
                        .advancements
                        .trigger_listeners(LIGHTNING_STRIKE)
                        .next()
                        .is_some()
            })
            .map(|(id, _)| *id)
            .collect();
        if listeners.is_empty() {
            return;
        }
        for connection_id in listeners {
            let matching = {
                let Some(matches) = self.entity_matcher(connection_id, dimension, LIGHTNING_STRIKE)
                else {
                    continue;
                };
                let bolt = LootEntity::Stored(StoredLootEntity {
                    entity: bolt,
                    bodies: Some(crate::entity::store::EntityBodyReadView::new(
                        &self.level_by_id(dimension).entities,
                        None,
                        None,
                    )),
                    raid_at_position: None,
                });
                let matches_player = self.player_matcher(connection_id, LIGHTNING_STRIKE);
                let compiled = &self.cold.advancement_triggers.lightning_strike;
                self.listening_criteria(connection_id, LIGHTNING_STRIKE, |index, criterion| {
                    let Some(condition) = compiled
                        .get(&index)
                        .and_then(|criteria| criteria.get(criterion))
                    else {
                        return false;
                    };
                    // `LightningStrikeTrigger.java:47-51`.
                    matches(bolt, &condition.lightning)
                        && condition.bystander.as_ref().is_none_or(|bystander| {
                            bystanders.iter().any(|witness| {
                                self.with_strike_witness(dimension, witness, |entity| {
                                    matches(entity, bystander)
                                })
                            })
                        })
                        && matches_player(&condition.player)
                })
            };
            self.apply_advancement_matches(connection_id, matching, events);
        }
    }

    /// `ChanneledLightningTrigger.trigger` (`:23-26`): every listed victim
    /// predicate is met by some struck entity (`:42-56`).
    pub(in crate::world) fn trigger_channeled_lightning(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        victims: &[StrikeWitness],
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.listens(connection_id, CHANNELED_LIGHTNING) {
            return;
        }
        let matching = {
            let Some(matches) = self.entity_matcher(connection_id, dimension, CHANNELED_LIGHTNING)
            else {
                return;
            };
            let matches_player = self.player_matcher(connection_id, CHANNELED_LIGHTNING);
            let compiled = &self.cold.advancement_triggers.channeled_lightning;
            self.listening_criteria(connection_id, CHANNELED_LIGHTNING, |index, criterion| {
                let Some(condition) = compiled
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    return false;
                };
                condition.victims.iter().all(|predicate| {
                    victims.iter().any(|witness| {
                        self.with_strike_witness(dimension, witness, |entity| {
                            matches(entity, predicate)
                        })
                    })
                }) && matches_player(&condition.player)
            })
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }

    pub(super) fn listens(&self, connection_id: ConnectionId, trigger: &str) -> bool {
        self.levels.player(connection_id).is_some_and(|player| {
            player
                .cold
                .advancements
                .trigger_listeners(trigger)
                .next()
                .is_some()
        })
    }

    pub(super) fn listening_criteria(
        &self,
        connection_id: ConnectionId,
        trigger: &str,
        accept: impl Fn(usize, &bbb_protocol::JavaString) -> bool,
    ) -> Vec<(usize, bbb_protocol::JavaString)> {
        self.levels
            .player(connection_id)
            .map(|player| {
                player
                    .cold
                    .advancements
                    .trigger_listeners(trigger)
                    .filter(|(index, criterion)| accept(*index, criterion))
                    .map(|(index, criterion)| (index, criterion.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn with_strike_witness<T>(
        &self,
        dimension: &LevelId,
        witness: &StrikeWitness,
        read: impl FnOnce(LootEntity<'_>) -> T,
    ) -> T {
        match witness {
            StrikeWitness::Stored(entity) => read(LootEntity::Stored(StoredLootEntity {
                entity,
                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                    &self.level_by_id(dimension).entities,
                    Some(entity),
                    None,
                )),
                raid_at_position: None,
            })),
            StrikeWitness::Player { entity_id } => {
                read(self.killed_entity_of_id(dimension, *entity_id).loot())
            }
        }
    }

    pub(super) fn player_matcher<'w>(
        &'w self,
        connection_id: ConnectionId,
        trigger: &'w str,
    ) -> impl Fn(&[LootCondition]) -> bool + 'w {
        let player = self.levels.player(connection_id);
        let matches = player
            .and_then(|player| self.entity_matcher(connection_id, &player.dimension, trigger));
        let entity = std::cell::OnceCell::new();
        move |conditions| {
            conditions.is_empty()
                || player
                    .zip(matches.as_ref())
                    .is_some_and(|(player, matches)| {
                        matches(
                            LootEntity::Facts(
                                entity.get_or_init(|| self.loot_view_for_player(player)),
                            ),
                            conditions,
                        )
                    })
        }
    }

    /// An entity predicate in the player's context
    /// (`EntityPredicate.createContext`): the origin is the player.
    pub(super) fn entity_matcher<'w>(
        &'w self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        trigger: &'w str,
    ) -> Option<impl Fn(LootEntity<'_>, &[LootCondition]) -> bool + 'w> {
        let matches = self.entity_context_matcher(
            connection_id,
            dimension,
            trigger,
            param_sets::ADVANCEMENT_ENTITY,
        )?;
        Some(
            move |entity: LootEntity<'_>, conditions: &[LootCondition]| {
                matches(entity, conditions, LootDamageSource::Absent)
            },
        )
    }

    pub(super) fn entity_context_matcher<'w>(
        &'w self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        trigger: &'w str,
        param_set: &'static LootContextParamSet,
    ) -> Option<impl Fn(LootEntity<'_>, &[LootCondition], LootDamageSource<'_>) -> bool + 'w> {
        let player = self.levels.player(connection_id)?;
        let level = self.levels.get(dimension)?;
        let dimension = *dimension;
        let startup = self.cold.config.datapacks.as_deref()?;
        let blocks = LoadedBlocks {
            chunks: &level.chunks,
        };
        let origin = LootOrigin {
            x: player.position().x,
            y: player.position().y,
            z: player.position().z,
        };
        Some(
            move |entity: LootEntity<'_>,
                  conditions: &[LootCondition],
                  damage_source: LootDamageSource<'_>| {
                let block_at = |pos| blocks.loaded_block_state(pos);
                let player_slots = |uuid, slot| self.player_loot_slot(Some(player), uuid, slot);
                let player_nbt = |uuid| self.player_loot_nbt(uuid);
                let item_environment = self
                    .predicate_loot_environment(level, dimension)
                    .with_player_slots(&player_slots)
                    .with_player_nbt(&player_nbt);
                let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
                    self.world_vehicle_slots(Some(player), requested, selected)
                };
                let params = LootParams {
                    dimension: Some(level.dimension.as_str()),
                    damage_source,
                    item_environment: Some(&item_environment),
                    vehicle_slots: Some(&vehicle_slots),
                    origin: Some(origin),
                    entities: LootEntities {
                        this_entity: entity,
                        ..Default::default()
                    },
                    level: Some(&block_at),
                    block_entity_data: Some(&|pos| {
                        level
                            .chunks
                            .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                            .map_err(|error| {
                                crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                            })
                    }),
                    weather: Some(LootWeather {
                        raining: level.weather.is_raining(),
                        thundering: level.weather.is_thundering(),
                    }),
                    ..LootParams::new(param_set)
                };
                matches_conditions(&startup.loot, trigger, conditions, &params, &level.random)
                    == Ok(true)
            },
        )
    }
}
