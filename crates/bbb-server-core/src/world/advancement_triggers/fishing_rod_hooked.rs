//! `FishingRodHookedTrigger` (`FishingRodHookedTrigger.java:25-78`): the player,
//! rod, hooked entity and caught stacks of a retrieve.

use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::construct_beacon::player_conditions;
use super::{ItemCriterion, KillEntity, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::loot_runtime::matches_item_predicate;
use crate::loot_runtime::{
    matches_conditions, param_sets, LootEntities, LootEntity, LootOrigin, LootParams, LootWeather,
};
use crate::world::level_blocks::LoadedBlocks;

pub(super) const FISHING_ROD_HOOKED: &str = "minecraft:fishing_rod_hooked";

pub(super) struct HookedCondition {
    player: Vec<LootCondition>,
    entity: Vec<LootCondition>,
    rod: ItemCriterion,
    item: ItemCriterion,
}

impl HookedCondition {
    pub(super) fn parse(
        conditions: V<'_>,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        Ok(Self {
            player: player_conditions(conditions.field("player"), FISHING_ROD_HOOKED, lookup)?,
            entity: player_conditions(conditions.field("entity"), FISHING_ROD_HOOKED, lookup)?,
            rod: super::item_criterion(conditions.field("rod"))?,
            item: super::item_criterion(conditions.field("item"))?,
        })
    }
}

impl ServerWorld {
    pub(in crate::world) fn fishing_rod_hooked_entity(
        &self,
        connection_id: ConnectionId,
        entity_id: i32,
    ) -> Option<KillEntity> {
        let player = self.levels.player(connection_id)?;
        player
            .cold
            .advancements
            .trigger_listeners(FISHING_ROD_HOOKED)
            .any(|(index, criterion)| {
                self.cold
                    .advancement_triggers
                    .fishing_rod_hooked
                    .get(&index)
                    .and_then(|criteria| criteria.get(criterion))
                    .is_some_and(|condition| !condition.entity.is_empty())
            })
            .then(|| self.killed_entity_of_id(&player.dimension, entity_id))
    }

    /// `FishingRodHookedTrigger.trigger` (`:25-27`): `items` are the caught
    /// stacks, or the hooked item entity's own stack.
    pub(in crate::world) fn trigger_fishing_rod_hooked(
        &mut self,
        connection_id: ConnectionId,
        rod: &RuntimeItemStack,
        items: &[RuntimeItemStack],
        entity: Option<KillEntity>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let level = self.level_by_id(&player.dimension);
        let startup = self.cold.config.datapacks.as_deref();
        let player_facts = std::cell::OnceCell::new();
        let blocks = LoadedBlocks {
            chunks: &level.chunks,
        };
        let block_at = |pos| blocks.loaded_block_state(pos);
        let block_entity_data = |pos| {
            blocks
                .chunks
                .block_entity_loot_data(pos, Some(level.entities.item_resolver()))
                .map_err(|error| crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string()))
        };
        let weather = LootWeather {
            raining: level.weather.is_raining(),
            thundering: level.weather.is_thundering(),
        };
        let entity_matches = |conditions: &[LootCondition], target: bool| {
            if conditions.is_empty() {
                return true;
            }
            let Some(startup) = startup else {
                return false;
            };
            let this_entity = if target {
                entity.as_ref().map_or(LootEntity::Opaque, KillEntity::loot)
            } else {
                LootEntity::Facts(player_facts.get_or_init(|| self.loot_view_for_player(player)))
            };
            let player_slots = |uuid, slot| self.player_loot_slot(Some(player), uuid, slot);
            let player_nbt = |uuid| self.player_loot_nbt(uuid);
            let item_environment = self
                .predicate_loot_environment(level, player.dimension)
                .with_player_slots(&player_slots)
                .with_player_nbt(&player_nbt);
            let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
                self.world_vehicle_slots(Some(player), requested, selected)
            };
            let params = LootParams {
                dimension: Some(level.dimension.as_str()),
                item_environment: Some(&item_environment),
                origin: Some(LootOrigin {
                    x: player.position().x,
                    y: player.position().y,
                    z: player.position().z,
                }),
                entities: LootEntities {
                    this_entity,
                    ..Default::default()
                },
                level: Some(&block_at),
                block_entity_data: Some(&block_entity_data),
                vehicle_slots: Some(&vehicle_slots),
                weather: Some(weather),
                ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
            };
            matches_conditions(
                &startup.loot,
                FISHING_ROD_HOOKED,
                conditions,
                &params,
                &level.random,
            ) == Ok(true)
        };
        let tags = self.cold.advancement_triggers.item_tags.as_ref();
        let matches = |criterion: &ItemCriterion, tool: &crate::loot_runtime::LootTool| match (
            criterion, tags,
        ) {
            (None, _) => true,
            (Some(predicate), Some(tags)) => {
                matches_item_predicate(predicate, tool, tags, level.entities.item_resolver())
                    == Ok(true)
            }
            (Some(_), _) => false,
        };
        let compiled = &self.cold.advancement_triggers.fishing_rod_hooked;
        let matching = player
            .cold
            .advancements
            .trigger_listeners(FISHING_ROD_HOOKED)
            .filter(|(index, criterion)| {
                compiled
                    .get(index)
                    .and_then(|criteria| criteria.get(*criterion))
                    .is_some_and(|condition| {
                        entity_matches(&condition.player, false)
                            && entity_matches(&condition.entity, true)
                            && (condition.rod.is_none()
                                || matches(
                                    &condition.rod,
                                    &loot_tool_for_runtime_stack(
                                        rod,
                                        &self.datapack_tables.enchantment_registry,
                                    ),
                                ))
                            && (condition.item.is_none()
                                || items.iter().any(|item| {
                                    matches(
                                        &condition.item,
                                        &loot_tool_for_runtime_stack(
                                            item,
                                            &self.datapack_tables.enchantment_registry,
                                        ),
                                    )
                                }))
                    })
            })
            .map(|(index, criterion)| (index, criterion.clone()))
            .collect();
        self.apply_advancement_matches(connection_id, matching, events);
    }
}
