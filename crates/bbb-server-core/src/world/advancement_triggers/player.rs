use crate::connection::ConnectionId;
use crate::loot_runtime::{
    matches_conditions, param_sets, LootEntities, LootEntity, LootOrigin, LootParams, LootWeather,
};

use super::{ServerWorld, WorldEvent};
use crate::world::level_blocks::LoadedBlocks;

impl ServerWorld {
    pub(in crate::world) fn tick_player_advancement_criteria(
        &mut self,
        events: &mut Vec<WorldEvent>,
    ) {
        for connection_id in self.levels.player_keys().copied().collect::<Vec<_>>() {
            self.fire_player_trigger(connection_id, "minecraft:tick", events);
            self.trigger_levitation(connection_id, events);
            self.track_lava_vehicle_travel(connection_id, events);
            let player = &self.levels[connection_id];
            // A climbable check can load the destination column after a teleport.
            let has_fall = player.block_contact.fall_distance != 0.0
                || player.block_contact.starting_to_fall_position.is_some();
            // Player.java:442-453; LivingEntity.java:3057-3059.
            let resets_fall = (player.abilities.flying
                && self
                    .level_by_id(&player.dimension)
                    .entities
                    .external_player_vehicle(player.entity_id())
                    .is_none())
                || player.has_effect_id(
                    bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SLOW_FALLING_ID,
                )
                || player.has_effect_id(
                    bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_LEVITATION_ID,
                );
            // LivingEntity.java:2383-2391; LivingEntity.java:2641-2643.
            let travels_in_air = !player
                .fluid_interaction
                .is_in_fluid(crate::entity::FluidTag::Water)
                && !player
                    .fluid_interaction
                    .is_in_fluid(crate::entity::FluidTag::Lava)
                && !player.shared_flag(
                    bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
                );
            if has_fall
                && (resets_fall
                    || (travels_in_air
                        && self.player_on_climbable_checked(connection_id) == Some(true)))
            {
                self.reset_player_fall_distance(connection_id, events);
            }
            let player = &mut self.levels[connection_id];
            // ServerPlayer.java:841-848.
            if player.block_contact.fall_distance > 0.0
                && player.block_contact.starting_to_fall_position.is_none()
            {
                player.block_contact.starting_to_fall_position = Some(player.position());
                self.trigger_fall_after_explosion(connection_id, events);
            }
            // ServerPlayer.java:715-717.
            if self.levels[connection_id].tick_count % 20 == 0 {
                self.fire_player_trigger(connection_id, "minecraft:location", events);
            }
        }
    }

    fn trigger_fall_after_explosion(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        const TRIGGER: &str = "minecraft:fall_after_explosion";
        if !self.listens(connection_id, TRIGGER) {
            return;
        }
        let player = &self.levels[connection_id];
        let Some(impact) = player.cold.fall_impulse.impact_position else {
            return;
        };
        if impact.y > player.position().y {
            return;
        }
        let cause = player
            .cold
            .impulse_explosion_cause
            .as_ref()
            .map(|(source, retained_nbt)| {
                if let Some(connection) = self.player_reference(source.uuid) {
                    if let Some(view) = self.session_loot_view(connection) {
                        return view;
                    }
                }
                if let Some((dimension, id)) = self.entity_reference(&player.dimension, source.uuid)
                {
                    if let super::KillEntity::Facts(view) = self.killed_entity_of_id(&dimension, id)
                    {
                        return *view;
                    }
                }
                // The explosion retains the removed projectile's known facts.
                crate::loot_runtime::LootEntityFacts {
                    nbt: retained_nbt.clone(),
                    entity_type: bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(
                        &source.entity_resource_id,
                    )
                    .map(|entity| entity.resource_id)
                    .or_else(|| {
                        self.plugins()
                            .content_registry()
                            .entity_by_resource_id(&source.entity_resource_id)
                            .and_then(|spec| spec.try_resource_id_static().ok())
                    }),
                    living: Some(
                        source.is_living_non_player()
                            || matches!(
                                source.kind,
                                crate::explosion::ExplosionSourceEntityKind::Player { .. }
                            ),
                    ),
                    position: Some(crate::loot_runtime::LootEntityPosition {
                        x: source.position.x,
                        y: source.position.y,
                        z: source.position.z,
                    }),
                    ..Default::default()
                }
            });
        self.trigger_distance_with_cause(connection_id, TRIGGER, impact, cause.as_ref(), events);
    }

    // ServerPlayer.java:850-862.
    fn track_lava_vehicle_travel(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = &self.levels[connection_id];
        let entities = &self.level_by_id(&player.dimension).entities;
        let riding_in_lava = entities
            .external_player_vehicle(player.entity_id())
            .and_then(|id| entities.get(id))
            .is_some_and(|vehicle| vehicle.is_in_lava());
        let player = &mut self.levels[connection_id];
        if riding_in_lava {
            if let Some(start) = player.cold.entered_lava_on_vehicle_position {
                self.trigger_distance(
                    connection_id,
                    "minecraft:ride_entity_in_lava",
                    start,
                    events,
                );
            } else {
                player.cold.entered_lava_on_vehicle_position = Some(player.position());
            }
        } else {
            player.cold.entered_lava_on_vehicle_position = None;
        }
    }

    // PlayerTrigger.java:19-26; SimpleCriterionTrigger.java:43-70.
    pub(in crate::world) fn fire_player_trigger(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
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
        let Some(startup) = self.cold.config.datapacks.as_ref() else {
            return;
        };
        let dimension = player.dimension;
        let player_id = player.entity_id();
        let Some(compiled) = self.cold.advancement_triggers.player.get(trigger) else {
            return;
        };
        let Some((level, others)) = self.levels.split_mut(&dimension) else {
            return;
        };
        let names = others.names();
        let super::super::ServerLevel {
            chunks,
            entities,
            raids,
            weather,
            random,
            dimension: level_dimension,
            sky_darkening,
            ..
        } = level;
        let entities = &*entities;
        let raids = &*raids;
        let weather = &*weather;
        let random = &*random;
        let Some(player) = entities.player(player_id) else {
            return;
        };
        let bodies = crate::entity::store::EntityBodyReadView::new(entities, None, None);
        let mut matching = Vec::new();
        let mut predicate_listeners = Vec::new();
        for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
            let Some(conditions) = compiled
                .get(&index)
                .and_then(|criteria| criteria.get(criterion))
            else {
                continue;
            };
            if conditions.is_empty() {
                matching.push((index, criterion.clone()));
            } else {
                predicate_listeners.push((index, criterion, conditions));
            }
        }
        if !predicate_listeners.is_empty() {
            use crate::loot_runtime::LootRuntimeReason;
            use crate::world::loot_context::entity_scoreboard::{
                LootEntityLevels, LootEntityScoreboard,
            };
            use crate::world::simulation_level::mob_death_loot::LevelLootEnvironment;
            let tags = chunks.registry_tags_shared().map_or(
                crate::entity::EntityRegistryTagView::Builtin26_1,
                crate::entity::EntityRegistryTagView::Frozen,
            );
            let entity = super::super::loot_context::player_loot_view_in_bodies(
                player,
                Some(bodies),
                Some(raids),
                &self.datapack_tables.enchantment_registry,
                &self.datapack_tables.enchantment_slots,
                Self::player_known_movement_in_bodies(player, bodies, &tags),
            );
            let player_slots = |uuid, slot| {
                let player = entities
                    .player_by_uuid(uuid)
                    .or_else(|| {
                        others
                            .iter()
                            .find_map(|(_, level)| level.entities.player_by_uuid(uuid))
                    })
                    .ok_or_else(|| {
                        LootRuntimeReason::Catalog("player slots need an online player".into())
                    })?;
                Ok(super::super::loot_context::player_nbt::read_player_slot(
                    player,
                    slot,
                    &self.datapack_tables.enchantment_slots,
                ))
            };
            let player_nbt = |uuid| {
                let resolver = crate::world::registry_runtime::PlayerItemStackResolver::new(
                    Some(startup),
                    &self.registries,
                );
                super::super::loot_context::player_nbt::read_player_nbt_with_levels(
                    uuid,
                    names,
                    &resolver,
                    Some(&dimension),
                    Some(bodies),
                    LootEntityLevels::Other(others),
                )
            };
            let chunks = std::cell::RefCell::new(&mut *chunks);
            let maps = std::cell::RefCell::new(&mut self.maps);
            let actual_weather = LootWeather {
                raining: weather.is_raining(),
                thundering: weather.is_thundering(),
            };
            let item_environment = LevelLootEnvironment::for_world_predicates(
                crate::world::level_blocks::LootChunks::Chunks(&chunks),
                entities.item_resolver(),
                Some(startup.as_ref()),
                self.datapack_tables.enchantment_registry.as_slice(),
                Some(crate::entity::block_drops::RecipeDropFacts {
                    recipes: &startup.recipes,
                    recipe_index: &startup.recipe_index,
                    item_tags: &self.cold.loot_item_tags,
                }),
                &self.world_clocks,
                &self.cold.command_storage,
                &self.registries,
                self.cold.seed,
                sky_darkening.map(i32::from),
                self.cold.dimension_sky_lights.loot_environment_context(
                    self.cold
                        .level_dimension_types
                        .get(&dimension)
                        .map(String::as_str),
                    &self.registries,
                    &self.world_clocks,
                    crate::world::environment_attributes::ScalarWeather::from_level(weather),
                ),
                LootEntityScoreboard {
                    scoreboard: self.scoreboard.board(),
                    names,
                    dimension,
                    bodies: Some(bodies),
                    other_levels: LootEntityLevels::Other(others),
                },
            )
            .with_player_slots(&player_slots)
            .with_player_nbt(&player_nbt)
            .with_advancement_tree(&self.cold.advancement_tree)
            .with_maps(
                &maps,
                self.cold.seed,
                level_dimension.as_str(),
                self.cold.generate_structures,
            );
            let loaded_block_state = |pos| {
                LoadedBlocks {
                    chunks: &chunks.borrow(),
                }
                .loaded_block_state(pos)
            };
            let block_entity_data = |pos| {
                chunks
                    .borrow()
                    .block_entity_loot_data(pos, Some(entities.item_resolver()))
                    .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
            };
            // Player-trigger relations stay in this store; other levels are directory reads.
            let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
                let actual = bodies
                    .get(requested.id())
                    .filter(|actual| actual.uuid() == requested.uuid())
                    .ok_or_else(|| {
                        LootRuntimeReason::Catalog(
                            "vehicle slots need the actual source level/body".into(),
                        )
                    })?;
                super::super::simulation_level::block_entity_loot::vehicle_slots_with_environment(
                    &startup.loot,
                    Some(&item_environment),
                    Some(level_dimension.as_str()),
                    &loaded_block_state,
                    &block_entity_data,
                    actual_weather,
                    random,
                    Some(bodies),
                    actual,
                    selected,
                )
            };
            let params = LootParams {
                dimension: Some(level_dimension.as_str()),
                item_environment: Some(&item_environment),
                vehicle_slots: Some(&vehicle_slots),
                origin: Some(LootOrigin {
                    x: player.position().x,
                    y: player.position().y,
                    z: player.position().z,
                }),
                entities: LootEntities {
                    this_entity: LootEntity::Facts(&entity),
                    ..Default::default()
                },
                level: Some(&loaded_block_state),
                block_entity_data: Some(&block_entity_data),
                weather: Some(actual_weather),
                ..LootParams::new(param_sets::ADVANCEMENT_ENTITY)
            };
            for (index, criterion, conditions) in predicate_listeners {
                if matches_conditions(&startup.loot, trigger, conditions, &params, random)
                    == Ok(true)
                {
                    matching.push((index, criterion.clone()));
                }
            }
        }
        self.apply_advancement_matches(connection_id, matching, events);
    }
}

#[cfg(test)]
mod tests;
