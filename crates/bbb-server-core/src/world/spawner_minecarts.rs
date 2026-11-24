//! The `ServerLevel` half of `MinecartSpawner` and `BaseSpawner.serverTick`.

use crate::entity::block_drops::RecipeDropFacts;
use crate::entity::natural_spawn::mob_eligibility::{
    check_natural_spawn_rules, NaturalSpawnRulesProbe,
};
use crate::entity::save::load_fresh_entity_tree;
use crate::world::LevelId;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::spawn_predicate_level::{SpawnDimensionFacts, SpawnPredicateLevel};
use crate::chunk::ChunkStore;
use crate::connection::ClientboundPlayPacket;
use crate::entity::minecart_spawner::spawner_runtime::{
    set_spawner_mob_persistence_required, SpawnPolicy,
};
use crate::entity::{
    apply_spawner_tick_decision, check_spawn_placement_predicate, equip_spawner_mob,
    finalize_spawn, is_spawn_position_ok, resolve_spawner_minecart_attempt, Entity, EntityData,
    EntityStore, EquipmentPlan, SpawnContext, SpawnFluid, SpawnLevel, SpawnPlacementEnvironment,
    SpawnPlacementProbe, SpawnPredicateReason, SpawnReason, SpawnRequest, SpawnerRefusal,
};
use crate::entity_runtime_facts_26_1::vanilla_entity_runtime_facts_26_1;
use crate::level_event::{LevelEventSink, VanillaGameEvent};
use crate::loot_runtime::native_enchantments::upgrade_enchantment_level;
use crate::loot_runtime::{
    loot_table_by_id, loot_table_drops, param_sets, LootEntities, LootEntity, LootOrigin,
    LootParams, LootWeather, StoredLootEntity,
};
use crate::random::GameRandom;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_render_facts,
    vanilla_block_state_support_facts, VanillaBlockSupportDirection,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{vanilla_entity_type, VanillaEntityMobCategory};
use bbb_protocol::game_types::GameType;
use bbb_protocol::normalize_resource_location;
use bbb_protocol::packets::{BlockPos, EntityEvent, Vec3d};
#[cfg(test)]
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::registry_codec::animal_variants::{
    decode_animal_variant_element, HolderReference, HolderSetReference, SpawnCondition,
};
use bbb_registry_codec::RegistryJsonValue as Value;
use bbb_server_datapack::loot::{HolderSet, LootRegistries};
use bbb_server_datapack::recipe::ItemTagIndex;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{collect_block_collisions_with_moving_pistons, Aabb};
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};
use chrono::{Datelike, Local};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::command_storage::ServerCommandStorage;
use super::environment_attributes::{LootEnvironmentContext, ScalarWeather};
use super::level_blocks::LootChunks;
use super::level_keys::LevelNames;
use super::loot_context::entity_scoreboard::{LootEntityLevels, LootEntityScoreboard};
use super::maps::ServerMapStore;
use super::natural_spawn::{tag_member, valid_empty_spawn_state};
use super::pov_clip::in_line_of_sight;
use super::server_scoreboard::ServerScoreboard;
use super::simulation_level::mob_death_loot::LevelLootEnvironment;
use super::world_clock::ServerWorldClocks;
use super::{ServerLevel, ServerWorld, HORIZONTAL_LIMIT};

#[cfg(test)]
mod finalize_biome_tests;

mod application;
mod cat_context;
mod cat_spawns;
mod elder_guardian;
mod lightning_events;
mod patrol_events;
mod phantom_spawns;
pub(in crate::world) mod raid_events;
mod siege_events;
mod skeleton_trap;
mod summon_command;
mod tadpole_events;
mod trader_events;
mod zombie_events;

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
const SPAWN_ANIMATION_EVENT_ID: i8 = 20;

pub(super) struct MinecartSpawnerRuntime<'a> {
    pub(super) chunks: &'a mut ChunkStore,
    maps: &'a mut ServerMapStore,
    generate_structures: bool,
    pub(super) entities: &'a mut EntityStore,
    pub(super) player_boxes: &'a [Aabb],
    pub(super) difficulty: Difficulty,
    pub(super) dimension: Java26LevelKey,
    pub(super) registries: &'a crate::registry_sync::SynchronizedRegistries,
    pub(super) datapacks: Option<&'a DatapackStartup>,
    enchantment_registry: &'a [String],
    item_tags: &'a ItemTagIndex,
    pub(super) loot: Option<&'a LootRegistries>,
    pub(super) world_seed: i64,
    pub(super) biome_zoom_seed: i64,
    pub(super) game_time: i64,
    pub(super) overworld_clock_time: i64,
    pub(super) halloween: bool,
    pub(super) spawn_dimension: Option<SpawnDimensionFacts>,
    pub(super) spawn_environment: super::environment_attributes::SpawnEnvironmentContext<'a>,
    pub(super) player_positions: Vec<Vec3d>,
    scoreboard: &'a mut ServerScoreboard,
    level_names: Arc<LevelNames>,
    other_levels: LootEntityLevels<'a>,
    world_clocks: &'a ServerWorldClocks,
    loot_environment: LootEnvironmentContext<'a>,
    sky_darkening: Option<i32>,
    weather: LootWeather,
    command_storage: &'a ServerCommandStorage,
    world_border: Option<super::WorldBorder>,
}

pub(super) trait SpawnEnchantmentProviderEnvironment {
    fn registry_tags(&self) -> Option<&crate::registry_tags::RegistryTagSnapshot>;
    fn datapacks(&self) -> Option<&DatapackStartup>;
    fn item_holders(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver;
}

impl SpawnEnchantmentProviderEnvironment for MinecartSpawnerRuntime<'_> {
    fn registry_tags(&self) -> Option<&crate::registry_tags::RegistryTagSnapshot> {
        self.chunks.registry_tags()
    }

    fn datapacks(&self) -> Option<&DatapackStartup> {
        self.datapacks
    }

    fn item_holders(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self.entities.item_resolver()
    }
}

impl<'a> MinecartSpawnerRuntime<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        chunks: &'a mut ChunkStore,
        maps: &'a mut ServerMapStore,
        generate_structures: bool,
        entities: &'a mut EntityStore,
        player_boxes: &'a [Aabb],
        difficulty: Difficulty,
        dimension: impl Into<Java26LevelKey>,
        registries: &'a crate::registry_sync::SynchronizedRegistries,
        datapacks: Option<&'a DatapackStartup>,
        enchantment_registry: &'a [String],
        item_tags: &'a ItemTagIndex,
        loot: Option<&'a LootRegistries>,
        world_seed: i64,
        game_time: i64,
        overworld_clock_time: i64,
        halloween: bool,
        spawn_dimension: Option<SpawnDimensionFacts>,
        spawn_environment: super::environment_attributes::SpawnEnvironmentContext<'a>,
        scoreboard: &'a mut ServerScoreboard,
        level_names: Arc<LevelNames>,
        other_levels: LootEntityLevels<'a>,
        world_clocks: &'a ServerWorldClocks,
        loot_environment: LootEnvironmentContext<'a>,
        sky_darkening: Option<i32>,
        weather: LootWeather,
        command_storage: &'a ServerCommandStorage,
    ) -> Self {
        Self {
            chunks,
            maps,
            generate_structures,
            entities,
            player_boxes,
            difficulty,
            dimension: dimension.into(),
            registries,
            datapacks,
            enchantment_registry,
            item_tags,
            loot,
            world_seed,
            biome_zoom_seed: bbb_server_worldgen::noise::biome::biome_zoom_seed(world_seed),
            game_time,
            overworld_clock_time,
            halloween,
            spawn_dimension,
            spawn_environment,
            scoreboard,
            level_names,
            other_levels,
            world_clocks,
            loot_environment,
            sky_darkening,
            weather,
            command_storage,
            world_border: None,
            player_positions: player_boxes
                .iter()
                .map(|bounds| Vec3d {
                    x: (bounds.min_x + bounds.max_x) * 0.5,
                    y: bounds.min_y,
                    z: (bounds.min_z + bounds.max_z) * 0.5,
                })
                .collect(),
        }
    }

    /// Finalizes a fresh mob and publishes it, or drops it.
    pub(super) fn finalize_and_add(
        &mut self,
        mut tree: Box<Entity>,
        reason: SpawnReason,
        random: &GameRandom,
    ) -> Option<i32> {
        if finalize_spawn(self, &mut tree, reason, None, random).is_none() {
            self.entities.discard_fresh(tree);
            return None;
        }
        Some(self.entities.add_fresh(tree))
    }

    fn is_unobstructed(&self, root_id: i32, bounds: Aabb) -> bool {
        // `EntityGetter.isUnobstructed` (`EntityGetter.java:33-43`) checks
        // `blocksBuilding`, not `canBeCollidedWith`, and skips the fresh
        // root's own passengers, which share its vehicle.
        self.entities
            .first_entity_in_box(bounds, |entity| {
                !self.entities.has_indirect_passenger(root_id, entity.id())
                    && entity.blocks_building()
            })
            .is_none()
            && !self
                .player_boxes
                .iter()
                .any(|player| player.intersects(bounds))
    }

    // SpawnContext.java:10-13; BiomeManager.java:31-64.
    fn biome_at(&self, position: BlockPos) -> Option<&str> {
        let (x, y, z) = bbb_server_worldgen::noise::biome::fuzzy_biome_quart_position(
            self.biome_zoom_seed,
            position.x,
            position.y,
            position.z,
        );
        let biome = self.chunks.biome_id_at_quart_or_source(x, y, z)?;
        self.registries.registry_element_id(BIOME_REGISTRY, biome)
    }

    fn finalize_context(
        &self,
        position: BlockPos,
        entity_resource_id: &str,
    ) -> Option<SpawnContext<'a>> {
        let moon_brightness = self
            .spawn_environment
            .moon_brightness(self.chunks, position)?;
        let biome = self.biome_at(position)?;
        let mut variants =
            variant_registry_for_entity(entity_resource_id).map_or_else(Vec::new, |registry| {
                select_animal_variants(self.datapacks, self.registries, registry, |condition| {
                    spawn_condition_matches(self, position, biome, moon_brightness, condition)
                })
            });
        if let Some(registry) = sound_variant_registry_for_entity(entity_resource_id) {
            variants.extend(
                sound_variant_candidates(self.datapacks, self.registries, registry)
                    .unwrap_or_default(),
            );
        }
        Some(SpawnContext {
            game_time: self.game_time,
            difficulty: self.difficulty,
            special_multiplier: self.spawn_environment.special_multiplier(
                self.chunks,
                position,
                self.difficulty,
                self.overworld_clock_time,
            )?,
            dimension: self.dimension.resource_id().to_owned(),
            biome: Some(biome.to_owned()),
            biome_tags: animal_biome_tags(self.chunks.registry_tags(), biome),
            variants,
            halloween: self.halloween,
        })
    }
}

impl<'a> SpawnLevel<'a> for MinecartSpawnerRuntime<'a> {
    fn entities(&mut self) -> &mut EntityStore {
        self.entities
    }

    fn spawn_context(&self, resource_id: &str, pos: BlockPos) -> Option<SpawnContext<'a>> {
        self.finalize_context(pos, resource_id)
    }

    fn enchant(
        &self,
        provider_id: &str,
        stack: &RuntimeItemStack,
        special_multiplier: f32,
        random: &GameRandom,
    ) -> Option<RuntimeItemStack> {
        apply_enchantment_provider_to_stack(self, provider_id, stack, special_multiplier, random)
    }

    fn first_unridden(&self, resource_id: &str, search_box: Aabb) -> Option<i32> {
        first_unridden_alive(self.entities, resource_id, search_box)
    }

    fn no_collision(&self, bounds: Aabb) -> bool {
        MinecartSpawnerRuntime::no_collision(self, bounds)
    }
}

/// `EntitySelector.ENTITY_NOT_BEING_RIDDEN` over the first `resource_id`
/// entity in `search_box`.
pub(super) fn first_unridden_alive(
    entities: &EntityStore,
    resource_id: &str,
    search_box: Aabb,
) -> Option<i32> {
    entities
        .ids_in_box_sections(search_box)
        .into_iter()
        .find(|&id| {
            entities.get(id).is_some_and(|entity| {
                entity.is_alive()
                    && entity.living().is_none_or(|living| living.health() > 0.0)
                    && entities.passengers(id).is_empty()
                    && entity.data().entity_resource_id() == resource_id
                    && entity.bounding_box().intersects(search_box)
                    && entities.vehicle(id).is_none()
            })
        })
}

impl ServerWorld {
    /// The spawn runtime over `dimension`, beside that level's random and
    /// event sink.
    pub(super) fn with_spawn_runtime<T>(
        &mut self,
        dimension: &LevelId,
        player_boxes: &[Aabb],
        run: impl FnOnce(&mut MinecartSpawnerRuntime<'_>, &GameRandom, &mut LevelEventSink) -> T,
    ) -> Option<T> {
        let overworld_clock_time = self
            .registries
            .registry_element_index("minecraft:world_clock", "minecraft:overworld")
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.world_clock_total_ticks(index))
            .unwrap_or(0);
        let today = Local::now();
        let dimension_key = self.level_key(*dimension).clone();
        let dimension_type = self
            .cold
            .level_dimension_types
            .get(dimension)
            .map(String::as_str);
        let datapacks = self.cold.config.datapacks.as_deref();
        let spawn_dimension = self.level_spawn_dimension_facts(
            dimension,
            self.levels.get(dimension)?.weather.is_thundering(),
        );
        let level_names = Arc::clone(&self.levels.names);
        let (level, others) = self.levels.split_mut(dimension)?;
        let mut runtime = MinecartSpawnerRuntime::new(
            &mut level.chunks,
            &mut self.maps,
            self.cold.generate_structures,
            &mut level.entities,
            player_boxes,
            self.admin.settings().difficulty,
            dimension_key,
            &self.registries,
            datapacks,
            &self.datapack_tables.enchantment_registry,
            &self.cold.loot_item_tags,
            datapacks.map(|packs| &packs.loot),
            self.cold.seed,
            self.game_time,
            overworld_clock_time,
            today.month() == 10 && today.day() == 31,
            spawn_dimension,
            self.cold.dimension_sky_lights.spawn_context(
                dimension_type,
                &self.registries,
                &self.world_clocks,
            ),
            &mut self.scoreboard,
            level_names,
            LootEntityLevels::Other(others),
            &self.world_clocks,
            self.cold.dimension_sky_lights.loot_environment_context(
                dimension_type,
                &self.registries,
                &self.world_clocks,
                ScalarWeather::from_level(&level.weather),
            ),
            level.sky_darkening.map(i32::from),
            LootWeather {
                raining: level.weather.is_raining(),
                thundering: level.weather.is_thundering(),
            },
            &self.cold.command_storage,
        );
        runtime.world_border = Some(level.world_border);
        Some(run(&mut runtime, &mut level.random, &mut level.event_sink))
    }

    /// `Mob.finalizeSpawn` over a mob just added to `dimension`, lifted back
    /// out of the level around it.
    pub(super) fn finalize_added_mob(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        reason: SpawnReason,
    ) {
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            if let Some(mut entity) = runtime.entities.take_fresh(entity_id) {
                let _ = finalize_spawn(runtime, &mut entity, reason, None, random);
                runtime.entities.add_fresh(entity);
            }
        });
    }

    // `StructureTemplate.java:500-506`: finalize the snapped root before publication.
    pub(in crate::world) fn finalize_structure_template_entity(
        &mut self,
        dimension: &LevelId,
        tree: &mut Entity,
    ) -> bool {
        if !matches!(tree.data(), EntityData::GeneralLiving(data) if data.is_mob()) {
            return true;
        }
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            finalize_spawn(runtime, tree, SpawnReason::Structure, None, random).is_some()
        })
        .unwrap_or(false)
    }
}

impl ServerWorld {
    pub(in crate::world) fn run_spawner_minecart_attempt(
        &mut self,
        dimension: LevelId,
        attempt: crate::entity::PendingSpawnerMinecartAttempt,
    ) {
        let random = self.random.clone();
        let result = self.run_spawn_attempt(&dimension, &attempt.attempt.request, &random);
        let level = self
            .levels
            .get_mut(&dimension)
            .expect("every reachable dimension is a level this world runs");
        let decision =
            resolve_spawner_minecart_attempt(&mut level.entities, &attempt, result, &random);
        apply_spawner_tick_decision(
            attempt.minecart_entity_id,
            attempt.minecart_uuid,
            decision,
            &mut level.entities,
        );
    }

    /// One `BaseSpawner.serverTick` attempt, world half
    /// (`BaseSpawner.java:95-181`): the spawn request is applied against the
    /// dimension's own runtime and the three effects it raises are published.
    pub(super) fn run_spawn_attempt(
        &mut self,
        dimension: &LevelId,
        request: &SpawnRequest,
        random: &crate::random::GameRandom,
    ) -> Result<(), SpawnerRefusal> {
        self.run_spawn_attempt_with_policy(dimension, request, &SpawnPolicy::BASE_SPAWNER, random)
            .map(|_| ())
    }

    /// [`Self::run_spawn_attempt`] under an explicit spawn policy, answering
    /// the published root entity so the caller can track it.
    pub(super) fn run_spawn_attempt_with_policy(
        &mut self,
        dimension: &LevelId,
        request: &SpawnRequest,
        policy: &SpawnPolicy,
        random: &crate::random::GameRandom,
    ) -> Result<i32, SpawnerRefusal> {
        let player_boxes: Vec<_> = self
            .levels
            .players()
            .filter(|player| {
                player.dimension == *dimension
                    && !player.avatar_removed
                    && player.game_type != GameType::Spectator
            })
            .filter_map(|player| player.bounding_box().ok())
            .collect();
        let result = self
            .with_spawn_runtime(dimension, &player_boxes, |runtime, _, event_sink| {
                runtime.apply_spawn_request(request, policy, random, event_sink)
            })
            .expect("every reachable dimension is a level this world runs");
        if let Ok(root_id) = result {
            self.resolve_raider_raids_in_tree(dimension, root_id);
        }
        result
    }

    #[cfg(test)]
    pub(crate) fn spawn_spawner_minecart_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        position: bbb_protocol::packets::Vec3d,
    ) -> i32 {
        crate::entity::spawn_spawner_minecart(
            &mut self.level_mut(dimension).entities,
            position,
            0.0,
        )
        .expect("the test store accepts a fresh spawner minecart")
    }

    #[cfg(test)]
    pub(crate) fn queue_spawner_minecart_cycle_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) {
        let level = self.level_mut(dimension);
        let ServerLevel {
            entities,
            random: world_random,
            ..
        } = level;
        let entity = entities
            .get_mut(entity_id)
            .expect("the test spawner minecart is live");
        let uuid = entity.uuid();
        let pos = entity.block_position();
        let data =
            crate::entity::spawner_minecart_data_mut(entity).expect("expected spawner minecart");
        let decision = data.server_tick(pos, true, true, world_random);
        apply_spawner_tick_decision(entity_id, uuid, decision, entities);
    }

    #[cfg(test)]
    pub(crate) fn run_pending_spawner_minecart_attempts_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
    ) {
        let _ = self.run_entity_commands(&dimension.into(), &mut Vec::new());
    }

    #[cfg(test)]
    pub(crate) fn apply_spawner_enchantment_provider_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        provider_id: &str,
        stack: &RuntimeItemStack,
        local_special_multiplier: f32,
    ) -> Option<RuntimeItemStack> {
        self.with_spawn_runtime(&dimension.into(), &[], |runtime, random, _| {
            apply_enchantment_provider_to_stack(
                runtime,
                provider_id,
                stack,
                local_special_multiplier,
                random,
            )
        })?
    }

    #[cfg(test)]
    pub(crate) fn set_spawner_floor_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        position: BlockPos,
        state: i32,
    ) {
        let chunks = &mut self.level_mut(dimension).chunks;
        chunks
            .try_ensure_served_full_column(position.x >> 4, position.z >> 4)
            .unwrap();
        assert!(chunks
            .set_live_block_state(position.x, position.y, position.z, state)
            .unwrap()
            .is_some());
    }

    #[cfg(test)]
    pub(crate) fn stored_passengers_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        vehicle_id: i32,
    ) -> &[i32] {
        self.level(dimension).entities.passengers(vehicle_id)
    }

    #[cfg(test)]
    pub(crate) fn stored_entity_ids_for_test(&self, dimension: Java26BuiltinDimension) -> Vec<i32> {
        self.level(dimension).entities.ids()
    }

    #[cfg(test)]
    pub(crate) fn spawner_minecart_data_mut_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<&mut crate::entity::SpawnerMinecartData> {
        let entity = self.level_mut(dimension).entities.get_mut(entity_id)?;
        crate::entity::spawner_minecart_data_mut(entity)
    }

    #[cfg(test)]
    pub(crate) fn drain_spawner_effects_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
    ) -> (
        Vec<crate::level_event::PendingLevelEvent>,
        Vec<crate::entity::EntityLifecycleEvent>,
    ) {
        let level = self.level_mut(dimension);
        (
            level
                .event_sink
                .drain_level_broadcasts()
                .map(|event| match event {
                    crate::level_event::PendingLevelBroadcast::LevelEvent(event)
                    | crate::level_event::PendingLevelBroadcast::GlobalLevelEvent(event) => event,
                    crate::level_event::PendingLevelBroadcast::BeehiveEmergencyRelease {
                        ..
                    }
                    | crate::level_event::PendingLevelBroadcast::BlockDestruction { .. } => {
                        panic!("unexpected block destruction in spawner fixture")
                    }
                    crate::level_event::PendingLevelBroadcast::EntityAnimation { .. } => {
                        panic!("unexpected animation in spawner fixture")
                    }
                })
                .collect(),
            level.entities.drain_lifecycle_events(),
        )
    }
}

impl MinecartSpawnerRuntime<'_> {
    pub(super) fn predicate_level(&self) -> SpawnPredicateLevel<'_> {
        SpawnPredicateLevel {
            chunks: self.chunks,
            registries: self.registries,
            biome_zoom_seed: self.biome_zoom_seed,
            world_seed: self.world_seed,
            difficulty: self.difficulty,
            spawn_dimension: self.spawn_dimension.as_ref(),
            spawn_environment: &self.spawn_environment,
            players: &self.player_positions,
            generation_region: false,
            missing: std::cell::Cell::new(false),
        }
    }

    fn no_collision(&self, collision_box: Aabb) -> bool {
        block_collisions(self.chunks, collision_box).is_empty()
            && self
                .entities
                .first_entity_in_box(collision_box, |entity| entity.can_be_collided_with())
                .is_none()
    }

    fn check_spawn_predicate(
        &mut self,
        probe: SpawnPlacementProbe,
        reason: SpawnPredicateReason,
        random: &GameRandom,
    ) -> bool {
        let mut environment = spawn_environment(self.chunks, probe);
        if let Some(border) = self.world_border {
            environment.inside_world_border =
                border.contains_block_at(probe.position, self.game_time);
        }
        if !is_spawn_position_ok(probe.entry.placement, environment) {
            return false;
        }
        check_spawn_placement_predicate(&self.predicate_level(), probe, reason, random)
    }

    fn check_root_mob_spawn_rules(&self, root: &Entity) -> bool {
        let pos = root.block_position();
        check_natural_spawn_rules(
            root.entity_type_id(),
            NaturalSpawnRulesProbe {
                pathfinding_cost_from_light_levels: || pathfinding_light_cost(self, pos),
                block_at: |depth| {
                    block_state(
                        self.chunks,
                        BlockPos {
                            y: pos.y - depth as i32,
                            ..pos
                        },
                    )
                },
                below_is_sand: |state| tag_member(self.chunks, "minecraft:sand", state),
                air_at_and_below: [false; 3],
                // Fresh roots have not ticked; home and repellent state are unset.
                turtle_going_home: false,
                strider_is_in_lava: root.is_in_lava(),
                hoglin_near_repellent: false,
            },
        )
        .expect("the freshly loaded root is a supported Mob")
    }

    fn check_root_mob_obstruction(&self, tree: &Entity) -> bool {
        use crate::entity::natural_spawn::mob_eligibility::{
            natural_spawn_obstruction_rule, SpawnObstructionRule,
        };
        let entity_type_id = tree.entity_type_id();
        let bounds = tree.bounding_box();
        let pos = tree.block_position();
        let rule = natural_spawn_obstruction_rule(entity_type_id)
            .unwrap_or(SpawnObstructionRule::Standard);
        match rule {
            SpawnObstructionRule::Standard => {
                !box_contains_liquid(self.chunks, bounds) && self.is_unobstructed(tree.id(), bounds)
            }
            SpawnObstructionRule::WaterOrLava => self.is_unobstructed(tree.id(), bounds),
            SpawnObstructionRule::Ocelot => {
                pos.y >= self.chunks.sea_level()
                    && (is_block_id(
                        self.chunks,
                        below(pos),
                        bbb_protocol::block_types::VANILLA_BLOCK_GRASS_BLOCK_ID,
                    ) || tag_member(
                        self.chunks,
                        "minecraft:leaves",
                        block_state(self.chunks, below(pos)),
                    ))
                    && !box_contains_liquid(self.chunks, bounds)
                    && self.is_unobstructed(tree.id(), bounds)
            }
            SpawnObstructionRule::IronGolem => {
                iron_golem_blocks_clear(self.chunks, pos, entity_type_id)
                    && self.is_unobstructed(tree.id(), bounds)
            }
            SpawnObstructionRule::Ravager => !box_contains_liquid(self.chunks, bounds),
            SpawnObstructionRule::Warden => {
                !box_contains_liquid(self.chunks, bounds)
                    && self.is_unobstructed(tree.id(), bounds)
                    && vanilla_entity_type(entity_type_id).is_some_and(|facts| {
                        self.no_collision(Aabb::of_entity(
                            tree.position(),
                            facts.width,
                            facts.height,
                        ))
                    })
            }
        }
    }

    fn equip_root_mob(&mut self, tree: &mut Entity, equipment: &EquipmentPlan) -> bool {
        let Some(loot) = self.loot else {
            return false;
        };
        if loot_table_by_id(loot, &equipment.loot_table).is_none() {
            return false;
        }
        let stacks = {
            let position = tree.position();
            let resolver = self.entities.item_resolver();
            let Some(dimension) = self.level_names.id(&self.dimension) else {
                return false;
            };
            let bodies =
                crate::entity::store::EntityBodyReadView::new(self.entities, Some(tree), None);
            let chunks = RefCell::new(&mut *self.chunks);
            let maps = RefCell::new(&mut *self.maps);
            let environment =
                LevelLootEnvironment::empty(LootChunks::Chunks(&chunks), Some(resolver))
                    .with_enchantment_sources(self.datapacks, Some(self.enchantment_registry))
                    .with_recipes(self.datapacks.map(|packs| RecipeDropFacts {
                        recipes: &packs.recipes,
                        recipe_index: &packs.recipe_index,
                        item_tags: self.item_tags,
                    }))
                    .with_command_storage(self.command_storage)
                    .with_clocks(self.world_clocks)
                    .with_entity_scoreboard(LootEntityScoreboard {
                        scoreboard: self.scoreboard.board(),
                        names: &self.level_names,
                        dimension,
                        bodies: Some(bodies),
                        other_levels: self.other_levels,
                    })
                    .with_environment_attributes(self.loot_environment)
                    .with_position_queries(self.registries, self.world_seed, self.sky_darkening)
                    .with_maps(
                        &maps,
                        self.world_seed,
                        self.dimension.as_str(),
                        self.generate_structures,
                    );
            let params = LootParams {
                dimension: Some(self.dimension.as_str()),
                origin: Some(LootOrigin {
                    x: position.x,
                    y: position.y,
                    z: position.z,
                }),
                entities: LootEntities {
                    this_entity: LootEntity::Stored(StoredLootEntity {
                        entity: tree,
                        bodies: Some(bodies),
                        raid_at_position: None,
                    }),
                    ..LootEntities::default()
                },
                level: Some(&|pos| {
                    super::level_blocks::LoadedBlocks {
                        chunks: &chunks.borrow(),
                    }
                    .loaded_block_state(pos)
                }),
                block_entity_data: Some(&|pos| {
                    chunks
                        .borrow()
                        .block_entity_loot_data(pos, Some(resolver))
                        .map_err(|error| {
                            crate::loot_runtime::LootRuntimeReason::Catalog(error.to_string())
                        })
                }),
                item_environment: Some(&environment),
                weather: Some(self.weather),
                ..LootParams::new(param_sets::EQUIPMENT)
            };
            let Ok(stacks) =
                loot_table_drops(loot, &equipment.loot_table, &params, self.entities.random())
            else {
                return false;
            };
            stacks
        };
        let stacks = stacks
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect();
        equip_spawner_mob(tree, stacks, equipment)
    }
}

#[derive(Clone)]
struct SpawnerEnchantment {
    resource_id: String,
    weight: u32,
    max_level: i32,
    min_cost_base: i32,
    min_cost_per_level: i32,
    max_cost_base: i32,
    max_cost_per_level: i32,
    exclusive_set: Option<Value>,
}

#[derive(Clone)]
pub(super) struct SpawnerEnchantmentCandidate {
    pub(super) resource_id: String,
    pub(super) level: i32,
    weight: u32,
}

pub(super) fn apply_enchantment_provider_to_stack(
    environment: &impl SpawnEnchantmentProviderEnvironment,
    provider_id: &str,
    stack: &RuntimeItemStack,
    local_special_multiplier: f32,
    random: &GameRandom,
) -> Option<RuntimeItemStack> {
    let datapacks = environment.datapacks()?;
    let provider = datapacks
        .registries
        .element("minecraft:enchantment_provider", provider_id)?;
    let provider_json = provider.json()?;
    let single = bbb_registry_codec::RegistryElementValueRef::Json(provider_json.get("type")?)
        .java_string_ref()?
        .with_utf8(|id| match id {
            "single" | ":single" | "minecraft:single" => Some(true),
            "by_cost_with_difficulty"
            | ":by_cost_with_difficulty"
            | "minecraft:by_cost_with_difficulty" => Some(false),
            _ => None,
        })
        .flatten()?;
    let Some(original_enchantments) = stack.enchantments(environment.item_holders()).ok()? else {
        return Some(stack.clone());
    };
    let mut enchantments = original_enchantments.to_vec();
    if single {
        let enchantment =
            normalize_resource_location(provider_json.get("enchantment")?.utf8_string()?).ok()?;
        let max_level = datapacks
            .registries
            .element("minecraft:enchantment", &enchantment)?
            .json()?
            .get("max_level")?
            .as_i64()
            .and_then(|level| i32::try_from(level).ok())?;
        let level = sample_int_provider(provider_json.get("level")?, random)?.clamp(1, max_level);
        let holder_id = environment
            .item_holders()
            .holder_id("minecraft:enchantment", &enchantment)?;
        upgrade_enchantment_level(&mut enchantments, holder_id, level);
    } else {
        let min_cost = i32::try_from(provider_json.get("min_cost")?.as_i64()?).ok()?;
        let max_cost_span = i32::try_from(provider_json.get("max_cost_span")?.as_i64()?).ok()?;
        let span = (local_special_multiplier * max_cost_span as f32) as i32;
        let cost = min_cost
            .checked_add(random.next_int(span.checked_add(1).filter(|bound| *bound > 0)?))?;
        let selected = select_spawner_enchantments(
            environment,
            stack,
            &HolderSet::decode(bbb_registry_codec::RegistryElementValueRef::Json(
                provider_json.get("enchantments")?,
            ))
            .ok()?,
            cost,
            random,
        )?;
        for selected in selected {
            let holder_id = environment
                .item_holders()
                .holder_id("minecraft:enchantment", &selected.resource_id)?;
            upgrade_enchantment_level(&mut enchantments, holder_id, selected.level);
        }
    }
    if enchantments.as_slice() == original_enchantments.as_ref() {
        Some(stack.clone())
    } else {
        stack
            .clone()
            .with_enchantments(enchantments, environment.item_holders())
            .ok()
    }
}

fn sample_int_provider(value: &Value, random: &GameRandom) -> Option<i32> {
    if let Some(value) = value.as_i64() {
        return i32::try_from(value).ok();
    }
    let object = value;
    object.as_object()?;
    let provider_type = normalize_resource_location(object.get("type")?.utf8_string()?).ok()?;
    match provider_type.as_str() {
        "minecraft:constant" => i32::try_from(object.get("value")?.as_i64()?).ok(),
        "minecraft:uniform" => {
            let min = i32::try_from(object.get("min_inclusive")?.as_i64()?).ok()?;
            let max = i32::try_from(object.get("max_inclusive")?.as_i64()?).ok()?;
            let width = max.checked_sub(min)?.checked_add(1)?;
            min.checked_add(random.next_int((width > 0).then_some(width)?))
        }
        _ => None,
    }
}

pub(super) fn select_spawner_enchantments(
    environment: &impl SpawnEnchantmentProviderEnvironment,
    stack: &RuntimeItemStack,
    provider_set: &HolderSet,
    mut cost: i32,
    random: &GameRandom,
) -> Option<Vec<SpawnerEnchantmentCandidate>> {
    let item_resource_id = stack.item_resource_id()?;
    // EnchantmentHelper.java:554-562: no component returns before consuming RNG.
    let enchantability = match stack
        .effective_component(
            VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
            environment.item_holders(),
        )
        .ok()?
    {
        Some(component) => component.enchantable_value?,
        None => return Some(Vec::new()),
    };
    let random_bound = Some(enchantability / 4 + 1).filter(|bound| *bound > 0)?;
    cost = cost
        .wrapping_add(1)
        .wrapping_add(random.next_int(random_bound))
        .wrapping_add(random.next_int(random_bound));
    let span = (random.next_float() + random.next_float() - 1.0) * 0.15;
    cost = ((f64::from(cost as f32 + cost as f32 * span) + 0.5).floor() as i32).max(1);
    let holders = spawner_enchantment_holders(environment, provider_set);
    let registry = environment
        .datapacks()?
        .registries
        .registry("minecraft:enchantment")?;

    let mut facts = BTreeMap::new();
    let mut available = Vec::new();
    for holder in holders {
        let element = registry
            .elements
            .iter()
            .find(|element| element.id == holder)?;
        let Some(enchantment) =
            spawner_enchantment(environment, &element.id, element.json()?, item_resource_id)
        else {
            continue;
        };
        for level in (1..=enchantment.max_level).rev() {
            let min = enchantment.min_cost_base.wrapping_add(
                enchantment
                    .min_cost_per_level
                    .wrapping_mul(level.wrapping_sub(1)),
            );
            let max = enchantment.max_cost_base.wrapping_add(
                enchantment
                    .max_cost_per_level
                    .wrapping_mul(level.wrapping_sub(1)),
            );
            if (min..=max).contains(&cost) {
                available.push(SpawnerEnchantmentCandidate {
                    resource_id: enchantment.resource_id.clone(),
                    level,
                    weight: enchantment.weight,
                });
                break;
            }
        }
        facts.insert(enchantment.resource_id.clone(), enchantment);
    }
    let Some(first) = weighted_enchantment(&available, random) else {
        return Some(Vec::new());
    };
    let mut selected = vec![first];
    while random.next_int(50) <= cost.max(0) {
        let last = selected.last()?.resource_id.as_str();
        available.retain(|candidate| {
            enchantments_compatible(environment, &facts, last, &candidate.resource_id)
        });
        let Some(next) = weighted_enchantment(&available, random) else {
            break;
        };
        selected.push(next);
        cost /= 2;
    }
    Some(selected)
}

pub(super) fn spawner_enchantment_holders(
    environment: &impl SpawnEnchantmentProviderEnvironment,
    set: &HolderSet,
) -> Vec<String> {
    match set {
        HolderSet::Tag(tag) => environment
            .registry_tags()
            .map(|tags| {
                tags.members("minecraft:enchantment", tag)
                    .iter()
                    .map(|member| member.to_string())
                    .collect()
            })
            .unwrap_or_default(),
        HolderSet::Ids(ids) => ids.clone(),
    }
}

fn spawner_enchantment(
    environment: &impl SpawnEnchantmentProviderEnvironment,
    resource_id: &str,
    json: &Value,
    item_resource_id: &str,
) -> Option<SpawnerEnchantment> {
    if item_resource_id != "minecraft:book"
        && (!json_holder_set_contains(
            environment,
            "minecraft:item",
            json.get("supported_items")?,
            item_resource_id,
        ) || json.get("primary_items").is_some_and(|primary| {
            !json_holder_set_contains(environment, "minecraft:item", primary, item_resource_id)
        }))
    {
        return None;
    }
    let (min_cost_base, min_cost_per_level) = linear_cost(json.get("min_cost")?)?;
    let (max_cost_base, max_cost_per_level) = linear_cost(json.get("max_cost")?)?;
    Some(SpawnerEnchantment {
        resource_id: resource_id.to_owned(),
        weight: u32::try_from(json.get("weight")?.as_u64()?).ok()?,
        max_level: i32::try_from(json.get("max_level")?.as_i64()?).ok()?,
        min_cost_base,
        min_cost_per_level,
        max_cost_base,
        max_cost_per_level,
        exclusive_set: json.get("exclusive_set").cloned(),
    })
}

fn linear_cost(value: &Value) -> Option<(i32, i32)> {
    let object = value;
    object.as_object()?;
    Some((
        i32::try_from(object.get("base")?.as_i64()?).ok()?,
        i32::try_from(object.get("per_level_above_first")?.as_i64()?).ok()?,
    ))
}

fn weighted_enchantment(
    candidates: &[SpawnerEnchantmentCandidate],
    random: &GameRandom,
) -> Option<SpawnerEnchantmentCandidate> {
    let total = candidates.iter().try_fold(0_u32, |total, candidate| {
        total.checked_add(candidate.weight)
    })?;
    if total == 0 {
        return None;
    }
    let mut selection = random.next_int(i32::try_from(total).ok()?) as u32;
    for candidate in candidates {
        if selection < candidate.weight {
            return Some(candidate.clone());
        }
        selection -= candidate.weight;
    }
    None
}

fn enchantments_compatible(
    environment: &impl SpawnEnchantmentProviderEnvironment,
    facts: &BTreeMap<String, SpawnerEnchantment>,
    first: &str,
    second: &str,
) -> bool {
    if first == second {
        return false;
    }
    let Some(first_facts) = facts.get(first) else {
        return false;
    };
    let Some(second_facts) = facts.get(second) else {
        return false;
    };
    !first_facts.exclusive_set.as_ref().is_some_and(|set| {
        json_holder_set_contains(environment, "minecraft:enchantment", set, second)
    }) && !second_facts.exclusive_set.as_ref().is_some_and(|set| {
        json_holder_set_contains(environment, "minecraft:enchantment", set, first)
    })
}

pub(super) fn json_holder_set_contains(
    environment: &impl SpawnEnchantmentProviderEnvironment,
    registry: &str,
    set: &Value,
    resource_id: &str,
) -> bool {
    match set {
        Value::String(reference) => {
            let Ok(reference) = reference.try_to_string() else {
                return false;
            };
            if let Some(tag) = reference.strip_prefix('#') {
                let Ok(tag) = normalize_resource_location(tag.to_owned()) else {
                    return false;
                };
                environment
                    .registry_tags()
                    .is_some_and(|tags| tags.contains(registry, &tag, resource_id))
            } else {
                normalize_resource_location(reference).ok().as_deref() == Some(resource_id)
            }
        }
        Value::Array(values) => values
            .iter()
            .any(|value| json_holder_set_contains(environment, registry, value, resource_id)),
        _ => false,
    }
}

fn runtime_class(resource_id: &str) -> &str {
    vanilla_entity_runtime_facts_26_1(resource_id)
        .map(|facts| facts.runtime_class)
        .unwrap_or(resource_id)
}

fn block_state(chunks: &ChunkStore, pos: BlockPos) -> i32 {
    chunks
        .get_block_state(pos.x, pos.y, pos.z)
        .unwrap_or_else(|| chunks.air_state_id())
}

pub(super) fn animal_biome_tags(
    tags: Option<&crate::registry_tags::RegistryTagSnapshot>,
    biome: &str,
) -> Vec<&'static str> {
    const TAGS: [&str; 9] = [
        "minecraft:spawns_snow_foxes",
        "minecraft:spawns_white_rabbits",
        "minecraft:spawns_gold_rabbits",
        "minecraft:spawns_warm_variant_farm_animals",
        "minecraft:spawns_cold_variant_farm_animals",
        "minecraft:spawns_warm_variant_frogs",
        "minecraft:spawns_cold_variant_frogs",
        "minecraft:spawns_coral_variant_zombie_nautilus",
        "minecraft:more_frequent_drowned_spawns",
    ];
    TAGS.into_iter()
        .filter(|tag| tags.is_some_and(|tags| tags.contains(BIOME_REGISTRY, tag, biome)))
        .collect()
}

pub(super) fn variant_registry_for_entity(resource_id: &str) -> Option<&'static str> {
    Some(match resource_id {
        "minecraft:cat" => "minecraft:cat_variant",
        "minecraft:chicken" => "minecraft:chicken_variant",
        "minecraft:cow" => "minecraft:cow_variant",
        "minecraft:frog" => "minecraft:frog_variant",
        "minecraft:pig" => "minecraft:pig_variant",
        "minecraft:wolf" => "minecraft:wolf_variant",
        "minecraft:zombie_nautilus" => "minecraft:zombie_nautilus_variant",
        _ => return None,
    })
}

pub(super) fn sound_variant_registry_for_entity(resource_id: &str) -> Option<&'static str> {
    Some(match resource_id {
        "minecraft:cat" => "minecraft:cat_sound_variant",
        "minecraft:chicken" => "minecraft:chicken_sound_variant",
        "minecraft:cow" => "minecraft:cow_sound_variant",
        "minecraft:pig" => "minecraft:pig_sound_variant",
        _ => return None,
    })
}

pub(super) fn sound_variant_candidates<'registry>(
    datapacks: Option<&'registry DatapackStartup>,
    registries: &crate::registry_sync::SynchronizedRegistries,
    name: &'static str,
) -> Result<Vec<(&'static str, &'registry str, i32)>, String> {
    let registry = datapacks
        .and_then(|packs| packs.registries.registry(name))
        .ok_or_else(|| format!("{name} registry missing"))?;
    if registry.elements.is_empty() {
        return Err(format!("{name} registry empty"));
    }
    // CowSoundVariants.java:31-32 uses the complete registry.
    let candidates = registry
        .elements
        .iter()
        .map(|element| {
            let holder = registries
                .registry_element_index(name, &element.id)
                .ok_or_else(|| format!("{name} holder {} missing", element.id))?;
            Ok((name, element.id.as_str(), holder))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(candidates)
}

pub(super) fn select_animal_variants<'registry>(
    datapacks: Option<&'registry DatapackStartup>,
    registries: &crate::registry_sync::SynchronizedRegistries,
    registry_id: &'static str,
    mut condition_matches: impl FnMut(&SpawnCondition) -> bool,
) -> Vec<(&'static str, &'registry str, i32)> {
    let Some(registry) = datapacks.and_then(|datapacks| datapacks.registries.registry(registry_id))
    else {
        return Vec::new();
    };
    let mut matches = Vec::new();
    let mut highest = i32::MIN;
    for element in &registry.elements {
        let Ok(source) = element.decoded_loaded_source(usize::MAX) else {
            continue;
        };
        let Ok(variant) = decode_animal_variant_element(registry_id, &source) else {
            continue;
        };
        let Some(holder_id) = registries.registry_element_index(registry_id, &element.id) else {
            continue;
        };
        for selector in &variant.spawn_conditions().selectors {
            if match selector.condition.as_ref() {
                None => true,
                Some(condition) => condition_matches(condition),
            } {
                if selector.priority < highest {
                    continue;
                }
                if selector.priority > highest {
                    highest = selector.priority;
                    matches.clear();
                }
                matches.push((registry_id, element.id.as_str(), holder_id));
            }
        }
    }
    matches
}

fn spawn_condition_matches(
    runtime: &MinecartSpawnerRuntime<'_>,
    pos: BlockPos,
    biome: &str,
    moon_brightness: f32,
    condition: &SpawnCondition,
) -> bool {
    match condition {
        SpawnCondition::Biome { biomes } => holder_set_contains(
            runtime.chunks.registry_tags(),
            BIOME_REGISTRY,
            biomes,
            biome,
        ),
        SpawnCondition::Structure { structures } => structure_at_spawn(runtime, pos, structures),
        SpawnCondition::MoonBrightness { range } => {
            let value = f64::from(moon_brightness);
            range.min.is_none_or(|min| value >= min.get())
                && range.max.is_none_or(|max| value <= max.get())
        }
    }
}

pub(super) fn holder_set_contains(
    tags: Option<&crate::registry_tags::RegistryTagSnapshot>,
    registry: &str,
    holder_set: &HolderSetReference,
    resource_id: &str,
) -> bool {
    holder_set
        .references
        .iter()
        .any(|reference| match reference {
            HolderReference::Element(id) => id.as_str() == resource_id,
            HolderReference::Tag(tag) => {
                tags.is_some_and(|tags| tags.contains(registry, tag.as_str(), resource_id))
            }
        })
}

fn structure_at_spawn(
    runtime: &MinecartSpawnerRuntime<'_>,
    pos: BlockPos,
    structures: &HolderSetReference,
) -> bool {
    runtime
        .chunks
        .structure_component(pos.x.div_euclid(16), pos.z.div_euclid(16))
        .is_some_and(|target| {
            target.referenced_piece_contains(
                pos,
                |id| {
                    holder_set_contains(
                        runtime.chunks.registry_tags(),
                        "minecraft:worldgen/structure",
                        structures,
                        id,
                    )
                },
                |x, z| runtime.chunks.structure_component(x, z),
            )
        })
}

fn below(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y - 1,
        ..pos
    }
}

fn above(pos: BlockPos) -> BlockPos {
    BlockPos {
        y: pos.y + 1,
        ..pos
    }
}

fn is_block_id(chunks: &ChunkStore, pos: BlockPos, block_type_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state(chunks, pos)) == Some(block_type_id)
}

fn pathfinding_light_cost(runtime: &MinecartSpawnerRuntime<'_>, pos: BlockPos) -> f32 {
    let brightness = f32::from(raw_brightness(runtime, pos)) / 15.0;
    let curved = brightness / (4.0 - 3.0 * brightness);
    let ambient = match runtime.dimension.builtin() {
        Some(Java26BuiltinDimension::Nether) => 0.1,
        Some(Java26BuiltinDimension::End) => 0.25,
        Some(Java26BuiltinDimension::Overworld) => 0.0,
        None => runtime
            .spawn_dimension
            .as_ref()
            .map_or(0.0, |facts| facts.ambient_light),
    };
    ambient + (1.0 - ambient) * curved - 0.5
}

pub(super) fn spawn_environment(
    chunks: &ChunkStore,
    probe: SpawnPlacementProbe,
) -> SpawnPlacementEnvironment {
    let pos = probe.position;
    let above = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    let fluid_at = match block_state_fluid_state(block_state(chunks, pos)).family() {
        Some(FluidFamily::Water) => SpawnFluid::Water,
        Some(FluidFamily::Lava) => SpawnFluid::Lava,
        None => SpawnFluid::Empty,
    };
    SpawnPlacementEnvironment {
        inside_world_border: pos.x.abs() < HORIZONTAL_LIMIT && pos.z.abs() < HORIZONTAL_LIMIT,
        fluid_at,
        above_is_redstone_conductor: crate::redstone::is_redstone_conductor_with_registry(
            block_state(chunks, above),
            chunks.content_registry(),
        ),
        below_is_valid_spawn: valid_spawn_floor(chunks, below, probe.entity_type_id),
        at_is_valid_empty_spawn_block: valid_empty_spawn_block(chunks, pos, probe.entity_type_id),
        above_is_valid_empty_spawn_block: valid_empty_spawn_block(
            chunks,
            above,
            probe.entity_type_id,
        ),
    }
}

fn valid_spawn_floor(chunks: &ChunkStore, pos: BlockPos, entity_type_id: i32) -> bool {
    valid_spawn_floor_state(chunks, block_state(chunks, pos), entity_type_id)
}

/// `BlockState.isValidSpawn` (`BlockBehaviour.java:550-551`).
fn excluded_spawn_floor_blocks() -> &'static Vec<bool> {
    static EXCLUDED: std::sync::OnceLock<Vec<bool>> = std::sync::OnceLock::new();
    EXCLUDED.get_or_init(|| {
        use bbb_protocol::block_types::*;
        let mut flags = vec![false; VANILLA_BLOCK_TYPES.len()];
        for entry in VANILLA_BLOCK_TYPES {
            flags[entry.id as usize] = matches!(
                entry.id,
                VANILLA_BLOCK_BEDROCK_ID
                    | VANILLA_BLOCK_BARRIER_ID
                    | VANILLA_BLOCK_GLASS_ID
                    | VANILLA_BLOCK_MOVING_PISTON_ID
                    | VANILLA_BLOCK_CHORUS_FLOWER_ID
                    | VANILLA_BLOCK_SCAFFOLDING_ID
                    | VANILLA_BLOCK_TINTED_GLASS_ID
                    | VANILLA_BLOCK_FIREFLY_BUSH_ID
            ) || entry.resource_id.ends_with("_trapdoor")
                || entry.resource_id.ends_with("_stained_glass")
                || entry.resource_id.ends_with("_copper_grate");
        }
        flags
    })
}

pub(super) fn valid_spawn_floor_state(
    chunks: &ChunkStore,
    state: i32,
    entity_type_id: i32,
) -> bool {
    use bbb_protocol::block_types::*;
    use bbb_protocol::entity_types::*;
    let Some(block_id) = vanilla_block_state_block_type_id(state) else {
        return false;
    };
    // `Blocks.java:6886-6896` and every registration that replaces the
    // default `BlockBehaviour.Properties.isValidSpawn` predicate.
    if tag_member(chunks, "minecraft:leaves", state) {
        return matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_OCELOT_ID | VANILLA_ENTITY_TYPE_PARROT_ID
        );
    }
    if matches!(
        block_id,
        VANILLA_BLOCK_ICE_ID | VANILLA_BLOCK_FROSTED_ICE_ID
    ) {
        return entity_type_id == VANILLA_ENTITY_TYPE_POLAR_BEAR_ID;
    }
    if block_id == VANILLA_BLOCK_MAGMA_BLOCK_ID {
        return vanilla_entity_type(entity_type_id).is_some_and(|entity| entity.fire_immune);
    }
    if matches!(
        block_id,
        VANILLA_BLOCK_SOUL_SAND_ID
            | VANILLA_BLOCK_CARVED_PUMPKIN_ID
            | VANILLA_BLOCK_JACK_O_LANTERN_ID
            | VANILLA_BLOCK_REDSTONE_LAMP_ID
            | VANILLA_BLOCK_MUD_ID
    ) {
        return true;
    }
    if excluded_spawn_floor_blocks()
        .get(block_id as usize)
        .copied()
        .unwrap_or(false)
    {
        return false;
    }
    vanilla_block_state_support_facts(state)
        .zip(vanilla_block_state_render_facts(state))
        .is_some_and(|(support, render)| {
            support.collision_face_full(VanillaBlockSupportDirection::Up)
                && render.light_emission() < 14
        })
}

/// `IronGolem.checkSpawnObstruction` (`IronGolem.java:305-323`) up to its
/// `isUnobstructed` tail: a full collision face below, two ordinary
/// empty-spawn checks above, and an empty-fluid check at the root.
pub(super) fn iron_golem_blocks_clear(
    chunks: &ChunkStore,
    pos: BlockPos,
    entity_type_id: i32,
) -> bool {
    vanilla_block_state_support_facts(block_state(chunks, below(pos)))
        .is_some_and(|facts| facts.collision_face_full(VanillaBlockSupportDirection::Up))
        && valid_empty_spawn_block(chunks, above(pos), entity_type_id)
        && valid_empty_spawn_block(
            chunks,
            BlockPos {
                y: pos.y + 2,
                ..pos
            },
            entity_type_id,
        )
        && valid_empty_spawn_block_ignoring_fluid(chunks, pos, entity_type_id)
}

fn valid_empty_spawn_block(chunks: &ChunkStore, pos: BlockPos, entity_type_id: i32) -> bool {
    valid_empty_spawn_block_inner(chunks, pos, entity_type_id, true)
}

fn valid_empty_spawn_block_ignoring_fluid(
    chunks: &ChunkStore,
    pos: BlockPos,
    entity_type_id: i32,
) -> bool {
    valid_empty_spawn_block_inner(chunks, pos, entity_type_id, false)
}

fn valid_empty_spawn_block_inner(
    chunks: &ChunkStore,
    pos: BlockPos,
    entity_type_id: i32,
    check_fluid: bool,
) -> bool {
    let state = block_state(chunks, pos);
    valid_empty_spawn_state(chunks, state, entity_type_id, check_fluid)
}

fn raw_brightness(runtime: &MinecartSpawnerRuntime<'_>, pos: BlockPos) -> u8 {
    runtime
        .chunks
        .block_light_level(pos.x, pos.y, pos.z)
        .max(runtime.chunks.sky_light_level(pos.x, pos.y, pos.z))
}

fn block_collisions(chunks: &ChunkStore, bounds: Aabb) -> Vec<(BlockPos, Aabb)> {
    collect_block_collisions_with_moving_pistons(
        bounds,
        |pos| super::movement_collision::moving_piston_at(chunks, pos).unwrap_or(true),
        |pos| {
            let state = block_state(chunks, pos);
            super::movement_collision::static_collision_shape_with_live_piston(chunks, pos, state)
        },
    )
}

fn box_contains_liquid(chunks: &ChunkStore, bounds: Aabb) -> bool {
    let min_x = bounds.min_x.floor() as i32;
    let min_y = bounds.min_y.floor() as i32;
    let min_z = bounds.min_z.floor() as i32;
    let max_x = bounds.max_x.ceil() as i32;
    let max_y = bounds.max_y.ceil() as i32;
    let max_z = bounds.max_z.ceil() as i32;
    (min_x..max_x).any(|x| {
        (min_y..max_y).any(|y| {
            (min_z..max_z).any(|z| {
                !block_state_fluid_state(block_state(chunks, BlockPos { x, y, z })).is_empty()
            })
        })
    })
}
