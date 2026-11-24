//! Actual Item owner and picker contexts for the two pickup criteria.

use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_stack;
use crate::entity::EntityReference;
use crate::loot_runtime::{
    matches_conditions, matches_item_predicate, param_sets, LootEntities, LootEntity, LootOrigin,
    LootParams, LootWeather,
};
use crate::world::LevelId;
use bbb_protocol::packets::ItemStackSummary;
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;

use super::{ServerWorld, WorldEvent};
use crate::world::level_blocks::LoadedBlocks;

pub(super) const THROWN_ITEM_PICKED_UP_BY_ENTITY: &str =
    "minecraft:thrown_item_picked_up_by_entity";

pub(super) const THROWN_ITEM_PICKED_UP_BY_PLAYER: &str =
    "minecraft:thrown_item_picked_up_by_player";

pub(super) struct PickedUpItemCondition {
    player: Vec<LootCondition>,
    item: super::ItemCriterion,
    entity: Vec<LootCondition>,
}

/// `PickedUpItemTrigger.TriggerInstance.CODEC` (`PickedUpItemTrigger.java:29-36`).
pub(super) fn picked_up_item_condition(
    conditions: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<PickedUpItemCondition, String> {
    Ok(PickedUpItemCondition {
        player: super::construct_beacon::player_conditions(
            conditions.field("player"),
            "picked_up_item",
            lookup,
        )?,
        item: super::item_criterion(conditions.field("item"))?,
        entity: super::construct_beacon::player_conditions(
            conditions.field("entity"),
            "picked_up_item entity",
            lookup,
        )?,
    })
}

enum PickupOwner {
    Session(ConnectionId),
    Stored(LevelId, i32),
}

impl ServerWorld {
    pub(in crate::world) fn apply_thrown_item_pickup(
        &mut self,
        dimension: &LevelId,
        record: crate::entity::PendingThrownItemPickup,
        events: &mut Vec<WorldEvent>,
    ) {
        let mut reference = Some(record.thrower);
        let Some(Some(PickupOwner::Session(owner))) = self.resolve_pickup_owner(&mut reference)
        else {
            return;
        };
        let stack = record.item.summary().clone();
        let entity = self.killed_entity_of_stored(dimension, record.entity_id);
        self.trigger_thrown_item_picked_up(
            THROWN_ITEM_PICKED_UP_BY_ENTITY,
            owner,
            &stack,
            entity.loot(),
            events,
        );
    }

    fn resolve_pickup_owner(
        &self,
        reference: &mut Option<EntityReference>,
    ) -> Option<Option<PickupOwner>> {
        let Some(current) = *reference else {
            return Some(None);
        };
        match current {
            EntityReference::Stored { entity_id, uuid } => {
                for (dimension, level) in self.levels.iter() {
                    if let Some(body) = level.entities.get(entity_id) {
                        if uuid.is_none_or(|uuid| uuid == body.uuid())
                            && body.removal_reason().is_none()
                        {
                            return Some(Some(PickupOwner::Stored(dimension, entity_id)));
                        }
                    }
                    if uuid.is_some_and(|uuid| {
                        level.entities.entity_id_for_uuid(uuid) == Some(entity_id)
                    }) && level.entities.get(entity_id).is_none()
                    {
                        return None;
                    }
                }
            }
            EntityReference::Uuid(_) => {}
        }
        let uuid = current.uuid()?;
        if let Some((&connection_id, _)) = self
            .levels
            .player_entries()
            .find(|(_, player)| player.profile.uuid == uuid && !player.avatar_removed)
        {
            *reference = Some(EntityReference::Uuid(uuid));
            return Some(Some(PickupOwner::Session(connection_id)));
        }
        for (dimension, level) in self.levels.iter() {
            if let Some(id) = level.entities.visible_entity_id_for_uuid(uuid) {
                let body = level.entities.get(id)?;
                if body.removal_reason().is_none() {
                    *reference = Some(EntityReference::Stored {
                        entity_id: id,
                        uuid: Some(uuid),
                    });
                    return Some(Some(PickupOwner::Stored(dimension, id)));
                }
            }
            if level
                .entities
                .entity_id_for_uuid(uuid)
                .is_some_and(|id| level.entities.get(id).is_none())
            {
                return None;
            }
        }
        *reference = Some(EntityReference::Uuid(uuid));
        Some(None)
    }

    /// `LivingEntity.onItemPickup` then `ServerPlayer.onItemPickup` for the
    /// item `item_id` with `thrower` and the picked `stack`.
    pub(in crate::world) fn session_item_pickup_criteria(
        &mut self,
        picker: ConnectionId,
        dimension: &LevelId,
        item_id: i32,
        thrower: Option<EntityReference>,
        stack: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let mut reference = thrower;
        let owner = self.resolve_pickup_owner(&mut reference)?;
        if let Some(body) = self.level_mut_by_id(dimension).entities.get_mut(item_id) {
            crate::entity::item::set_thrower_reference(body, reference)?;
        }
        if let Some(PickupOwner::Session(owner)) = owner.as_ref() {
            let entity = self.session_loot_view(picker)?;
            self.trigger_thrown_item_picked_up(
                THROWN_ITEM_PICKED_UP_BY_ENTITY,
                *owner,
                stack,
                LootEntity::Facts(&entity),
                events,
            );
        }
        if let Some(owner) = owner {
            match owner {
                PickupOwner::Session(owner) => {
                    let entity = self.session_loot_view(owner)?;
                    self.trigger_thrown_item_picked_up(
                        THROWN_ITEM_PICKED_UP_BY_PLAYER,
                        picker,
                        stack,
                        LootEntity::Facts(&entity),
                        events,
                    );
                }
                PickupOwner::Stored(dimension, id) => {
                    let entity = self.killed_entity_of_stored(&dimension, id);
                    self.trigger_thrown_item_picked_up(
                        THROWN_ITEM_PICKED_UP_BY_PLAYER,
                        picker,
                        stack,
                        entity.loot(),
                        events,
                    );
                }
            }
        }
        Some(())
    }

    fn trigger_thrown_item_picked_up(
        &mut self,
        trigger: &'static str,
        connection_id: ConnectionId,
        stack: &ItemStackSummary,
        entity: LootEntity<'_>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(trigger)
            .next()
            .is_none()
        {
            return;
        }
        let dimension = player.dimension;
        let Some(startup) = self.cold.config.datapacks.clone() else {
            return;
        };
        let player_view = self
            .session_loot_view(connection_id)
            .expect("held pickup criterion owner");
        let listeners: Vec<_> = player
            .cold
            .advancements
            .trigger_listeners(trigger)
            .map(|(index, criterion)| (index, criterion.clone()))
            .collect();
        let position = player.position();
        let tool = loot_tool_for_stack(stack, &self.datapack_tables.enchantment_registry);
        let matching = {
            let (levels, advancement_triggers) = (&self.levels, &self.cold.advancement_triggers);
            let Some(level) = levels.get(&dimension) else {
                return;
            };
            let blocks = LoadedBlocks {
                chunks: &level.chunks,
            };
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
                item_environment: Some(&item_environment),
                vehicle_slots: Some(&vehicle_slots),
                origin: Some(LootOrigin {
                    x: position.x,
                    y: position.y,
                    z: position.z,
                }),
                entities: LootEntities {
                    this_entity: entity,
                    ..Default::default()
                },
                level: Some(&|pos| blocks.loaded_block_state(pos)),
                block_entity_data: Some(&|pos| {
                    blocks
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
                ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
            };
            let tags = advancement_triggers.item_tags.as_ref();
            let mut matching = Vec::new();
            let player_params = LootParams {
                dimension: Some(level.dimension.as_str()),
                item_environment: Some(&item_environment),
                vehicle_slots: Some(&vehicle_slots),
                entities: LootEntities {
                    this_entity: LootEntity::Facts(&player_view),
                    ..Default::default()
                },
                weather: Some(LootWeather {
                    raining: level.weather.is_raining(),
                    thundering: level.weather.is_thundering(),
                }),
                origin: Some(LootOrigin {
                    x: position.x,
                    y: position.y,
                    z: position.z,
                }),
                level: Some(&|pos| blocks.loaded_block_state(pos)),
                block_entity_data: Some(&|pos| {
                    blocks
                        .chunks
                        .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                        .map_err(|error| {
                            crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                        })
                }),
                ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
            };
            for (index, criterion) in listeners {
                let Some(condition) = advancement_triggers
                    .picked_up_item
                    .get(&index)
                    .and_then(|criteria| criteria.get(&criterion))
                else {
                    continue;
                };
                // `:51`: the item term is read before the entity context.
                let item_matches = match (&condition.item, tags) {
                    (None, _) => true,
                    (Some(predicate), Some(tags)) => {
                        matches_item_predicate(
                            predicate,
                            &tool,
                            tags,
                            level.entities.item_resolver(),
                        ) == Ok(true)
                    }
                    (Some(_), _) => false,
                };
                if item_matches
                    && matches_conditions(
                        &startup.loot,
                        trigger,
                        &condition.entity,
                        &params,
                        &level.random,
                    ) == Ok(true)
                    && matches_conditions(
                        &startup.loot,
                        trigger,
                        &condition.player,
                        &player_params,
                        &level.random,
                    ) == Ok(true)
                {
                    matching.push((index, criterion));
                }
            }
            matching
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
