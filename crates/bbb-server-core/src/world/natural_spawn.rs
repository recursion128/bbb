use crate::world::LevelId;
use std::cell::Cell;
use std::sync::Arc;

use crate::chunk::ChunkStore;
#[cfg(test)]
use crate::entity::natural_spawn::mob_eligibility::NaturalSpawnObstructionProbe;
use crate::entity::natural_spawn::mob_eligibility::{
    check_natural_spawn_rules, natural_spawn_obstruction_rule, NaturalSpawnRulesProbe,
    SpawnObstructionRule,
};
use crate::entity::natural_spawn::{
    create_natural_spawn_root, finalize_and_publish_natural_spawn, natural_spawn_existing_facts,
    natural_spawn_pre_finalize_mob_facts, ExistingNaturalSpawnEntity, NaturalSpawnCost,
    NaturalSpawnPublishResult, NaturalSpawnState,
};
use crate::entity::{
    check_spawn_placement_predicate, is_spawn_position_ok, Entity, EntityStore, SpawnContext,
    SpawnFluid, SpawnGroupData, SpawnLevel, SpawnPlacementEnvironment, SpawnPlacementProbe,
    SpawnPredicateReason, SpawnReason,
};
use crate::random::GameRandom;
use crate::redstone::{is_signal_source, WireSignal};
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_support_facts,
    VanillaBlockSupportDirection,
};
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{vanilla_entity_type, VanillaEntityMobCategory};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_registry_codec::registry_codec::animal_variants::{HolderSetReference, SpawnCondition};
use bbb_server_datapack::dimension_type::{
    builtin_dimension_type_runtime_facts, dimension_type_runtime_facts,
};
use bbb_server_datapack::mob_spawn::{DatapackMobSpawnCatalog, MobSpawnList, MobSpawnerData};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::block_queries::bool_property;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{
    no_block_collision_with_scratch, static_block_collision_box_iter, Aabb,
};
use bbb_server_types::interaction::WorldBorder;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
use bbb_server_worldgen::noise::nether_fortress::fortress_natural_enemy_spawns;
use chrono::{Datelike, Local};
use rustc_hash::FxHashSet;
use uuid::Uuid;

use super::simulation_level::SimulationChunkLevels;
use super::spawn_predicate_level::{
    spawn_dimension_facts, SpawnDimensionFacts, SpawnPredicateLevel,
};
use super::spawner_minecarts::{
    animal_biome_tags, apply_enchantment_provider_to_stack, holder_set_contains,
    select_animal_variants, sound_variant_candidates, sound_variant_registry_for_entity,
    variant_registry_for_entity, SpawnEnchantmentProviderEnvironment,
};
use super::{PlayerState, ServerLevel, ServerWorld, RULE_SPECTATORS_GENERATE_CHUNKS};

const BLOCK_REGISTRY: &str = "minecraft:block";
pub(super) const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
const STRUCTURE_REGISTRY: &str = "minecraft:worldgen/structure";
const PREVENT_MOB_SPAWNING_INSIDE: &str = "minecraft:prevent_mob_spawning_inside";
const NATURAL_SPAWN_CHUNK_RADIUS: i32 = 8;
const PLAYER_SPAWN_DISTANCE_SQUARED: f64 = 16_384.0;
const MIN_PLAYER_DISTANCE_SQUARED: f64 = 576.0;

pub(super) type MobSpawnCatalog = Arc<DatapackMobSpawnCatalog>;

pub(super) fn load_mob_spawns(
    datapacks: Option<&DatapackStartup>,
) -> bbb_server_types::Result<Option<MobSpawnCatalog>> {
    datapacks
        .map(|startup| {
            DatapackMobSpawnCatalog::from_generation_with_registry(
                &startup.registries,
                &startup.content_registry,
            )
        })
        .transpose()
        .map(|catalog| catalog.map(Arc::new))
        .map_err(|error| {
            bbb_server_types::ServerWorldError::InvalidConfiguration(error.to_string())
        })
}

#[derive(Clone, Copy)]
pub(crate) struct SpawnPlayer {
    uuid: Uuid,
    position: Vec3d,
    spectator: bool,
    creative: bool,
    dead_or_dying: bool,
    bounds: Aabb,
}

impl SpawnPlayer {
    fn from_world(player: &PlayerState) -> Option<Self> {
        Some(Self {
            uuid: player.profile.uuid,
            position: player.position(),
            spectator: player.game_type == bbb_protocol::game_types::GameType::Spectator,
            creative: player.game_type == bbb_protocol::game_types::GameType::Creative,
            dead_or_dying: player.damage.is_dead_or_dying(),
            bounds: player.bounding_box().ok()?,
        })
    }
}

#[derive(Clone)]
pub(crate) struct NaturalSpawnTickState {
    players: Vec<SpawnPlayer>,
    alive_players: Vec<Vec3d>,
    categories: Vec<VanillaEntityMobCategory>,
    state: NaturalSpawnState,
    biome_zoom_seed: i64,
    spawn_dimension: Option<SpawnDimensionFacts>,
    overworld_clock_time: i64,
    halloween: bool,
    fatal: bool,
}

impl ServerWorld {
    /// `DistanceManager.java:158-185`; `ChunkMap.java:937-949`.
    #[cfg(test)]
    pub(crate) fn natural_spawn_chunks_by_key(
        &self,
        dimension: &LevelId,
    ) -> (i32, Vec<(i32, i32)>) {
        let (count, spawning, _) = self.natural_spawn_chunks_and_players_by_key(dimension);
        (count, spawning)
    }

    pub(crate) fn natural_spawn_chunks_and_players_by_key(
        &self,
        dimension: &LevelId,
    ) -> (i32, Vec<(i32, i32)>, Vec<SpawnPlayer>) {
        let players = self.spawn_players(dimension);
        if players.is_empty() {
            return (0, Vec::new(), players);
        }
        let spectators_generate_chunks = matches!(
            self.admin.game_rule_value(RULE_SPECTATORS_GENERATE_CHUNKS),
            Some(GameRuleValue::Boolean(true))
        );
        let mut eligible = players
            .iter()
            .filter(|player| !player.spectator || spectators_generate_chunks);
        let Some(first) = eligible.next() else {
            return (0, Vec::new(), players);
        };
        let level = self.level_by_id(dimension);
        let is_spawning_chunk = |chunk_x: i32, chunk_z: i32| {
            level.chunks.is_loaded(chunk_x, chunk_z)
                && level.chunks.is_ticking_full(chunk_x, chunk_z)
                && level.simulation_levels.is_block_ticking((chunk_x, chunk_z))
                && players_close_to_chunk(&players, chunk_x, chunk_z)
        };
        if eligible.next().is_some() {
            let counted = natural_spawn_counted_chunks(&players, spectators_generate_chunks);
            let spawning = counted
                .iter()
                .copied()
                .filter(|&(chunk_x, chunk_z)| is_spawning_chunk(chunk_x, chunk_z))
                .collect();
            return (
                i32::try_from(counted.len()).unwrap_or(i32::MAX),
                spawning,
                players,
            );
        }
        let player_chunk_x = (first.position.x.floor() as i32).div_euclid(16);
        let player_chunk_z = (first.position.z.floor() as i32).div_euclid(16);
        let side = 2 * NATURAL_SPAWN_CHUNK_RADIUS + 1;
        let mut spawning = Vec::new();
        for chunk_x in player_chunk_x - NATURAL_SPAWN_CHUNK_RADIUS
            ..=player_chunk_x + NATURAL_SPAWN_CHUNK_RADIUS
        {
            for chunk_z in player_chunk_z - NATURAL_SPAWN_CHUNK_RADIUS
                ..=player_chunk_z + NATURAL_SPAWN_CHUNK_RADIUS
            {
                if is_spawning_chunk(chunk_x, chunk_z) {
                    spawning.push((chunk_x, chunk_z));
                }
            }
        }
        (side * side, spawning, players)
    }

    // `NaturalSpawner.java:67-118`.
    #[cfg(test)]
    pub(crate) fn prepare_natural_spawn_tick(
        &self,
        dimension: &LevelId,
        spawnable_chunk_count: i32,
    ) -> Option<NaturalSpawnTickState> {
        let players = self.spawn_players(dimension);
        self.prepare_natural_spawn_tick_with_players(dimension, spawnable_chunk_count, players)
    }

    // `NaturalSpawner.java:67-118`.
    pub(crate) fn prepare_natural_spawn_tick_with_players(
        &self,
        dimension: &LevelId,
        spawnable_chunk_count: i32,
        players: Vec<SpawnPlayer>,
    ) -> Option<NaturalSpawnTickState> {
        if !self.game_rules.spawn_mobs() {
            return None;
        }
        let catalog = self.cold.mob_spawns.as_deref()?;
        let level = self.level_by_id(dimension);
        let mut state = NaturalSpawnState::new(spawnable_chunk_count);
        let mut nearby = Vec::new();
        for entity in level.entities.bodies() {
            let facts = natural_spawn_existing_facts(&level.entities, entity)?;
            if facts.category == VanillaEntityMobCategory::Misc
                || (facts.is_mob
                    && (facts.persistence_required || facts.requires_custom_persistence))
            {
                continue;
            }
            let chunk_x = facts.position.x.div_euclid(16);
            let chunk_z = facts.position.z.div_euclid(16);
            if !level.chunks.is_loaded(chunk_x, chunk_z)
                || !level.chunks.is_ticking_full(chunk_x, chunk_z)
            {
                continue;
            }
            if !players.is_empty() {
                fill_player_ids_close_to_chunk(&players, chunk_x, chunk_z, &mut nearby);
            } else {
                nearby.clear();
            }
            let cost_charge = match biome_spawn_cost(
                catalog,
                &self.registries,
                &level.chunks,
                facts.position,
                facts.entity_type_id,
                (chunk_x, chunk_z),
            ) {
                Ok(cost) => cost.map(|cost| cost.charge),
                Err(()) => return None,
            };
            state.observe_existing(ExistingNaturalSpawnEntity {
                category: facts.category,
                position: facts.position,
                is_mob: facts.is_mob,
                persistence_required: facts.persistence_required,
                requires_custom_persistence: facts.requires_custom_persistence,
                cost_charge,
                nearby_players: &nearby,
            });
        }
        let categories = state.filtered_spawning_categories(
            true,
            self.game_rules
                .spawning_monsters(self.admin.settings().difficulty),
            self.game_time.rem_euclid(400) == 0,
        );
        let biome_zoom_seed = biome_zoom_seed(self.cold.seed);
        let (alive_players, spawn_dimension, overworld_clock_time, halloween) =
            if categories.is_empty() {
                (Vec::new(), None, 0, false)
            } else {
                let overworld_clock_time = self
                    .registries
                    .registry_element_index("minecraft:world_clock", "minecraft:overworld")
                    .and_then(|index| usize::try_from(index).ok())
                    .and_then(|index| self.world_clock_total_ticks(index))
                    .unwrap_or(0);
                let thundering = level.weather.is_thundering();
                let spawn_dimension = level.sky_darkening.and_then(|sky_darken| {
                    let mut facts = self.level_spawn_dimension_facts(dimension, thundering)?;
                    facts.sky_darken = sky_darken;
                    Some(facts)
                });
                (
                    alive_player_positions(&players),
                    spawn_dimension,
                    overworld_clock_time,
                    is_halloween_today(),
                )
            };
        Some(NaturalSpawnTickState {
            players,
            alive_players,
            categories,
            state,
            biome_zoom_seed,
            spawn_dimension,
            overworld_clock_time,
            halloween,
            fatal: false,
        })
    }

    pub(super) fn shuffle_natural_spawn_chunks(&mut self, chunks: &mut [(i32, i32)]) {
        shuffle(chunks, &self.random);
    }

    // `ServerChunkCache.java:370-427`; `NaturalSpawner.java:121-147`.
    pub(super) fn spawn_naturally_in_chunk(
        &mut self,
        dimension: &LevelId,
        chunk: (i32, i32),
        tick: &mut NaturalSpawnTickState,
    ) {
        if tick.fatal {
            return;
        }
        if !self
            .level_by_id(dimension)
            .simulation_levels
            .is_entity_ticking(chunk)
        {
            return;
        }
        if !world_border_contains_chunk(
            self.level_by_id(dimension).world_border,
            chunk,
            self.game_time,
        ) {
            return;
        }
        if tick.categories.is_empty() {
            return;
        }
        let nearby = player_ids_close_to_chunk(&tick.players, chunk.0, chunk.1);
        let categories: Vec<_> = tick
            .categories
            .iter()
            .copied()
            .filter(|category| tick.state.can_spawn_for_category_local(*category, &nearby))
            .collect();
        if categories.is_empty() {
            return;
        }

        let difficulty = self.admin.settings().difficulty;
        let datapacks = self.cold.config.datapacks.as_deref();
        let catalog = self
            .cold
            .mob_spawns
            .as_deref()
            .expect("a prepared natural-spawn tick retains its startup catalog");
        let registries = &self.registries;
        let world_seed = self.cold.seed;
        let biome_zoom_seed = tick.biome_zoom_seed;
        let game_time = self.game_time;
        let overworld_clock_time = tick.overworld_clock_time;
        let halloween = tick.halloween;
        let dimension_type_id = self
            .cold
            .level_dimension_types
            .get(dimension)
            .map(String::as_str);
        let spawn_dimension = match tick.spawn_dimension.clone() {
            Some(facts) => facts,
            None => {
                let thundering = self.level_by_id(dimension).weather.is_thundering();
                let Some(mut facts) = self.level_spawn_dimension_facts(dimension, thundering)
                else {
                    tick.fatal = true;
                    return;
                };
                let Some(sky_darken) = self.level_by_id(dimension).sky_darkening else {
                    tick.fatal = true;
                    return;
                };
                facts.sky_darken = sky_darken;
                tick.spawn_dimension = Some(facts.clone());
                facts
            }
        };
        let default_spawn = (
            self.cold.default_spawn.dimension,
            Vec3d {
                y: self.cold.default_spawn.position.y + 0.5,
                ..self.cold.default_spawn.position
            },
        );
        let level = self
            .levels
            .get_mut(dimension)
            .expect("a ticked dimension remains served");
        let ServerLevel {
            chunks,
            entities,
            simulation_levels,
            world_border,
            random: world_random,
            dimension: dimension_key,
            ..
        } = level;
        let environment = NaturalEnvironment {
            dimension_id: *dimension,
            spawn_environment: self.cold.dimension_sky_lights.spawn_context(
                dimension_type_id,
                &self.registries,
                &self.world_clocks,
            ),
            chunks,
            simulation_levels,
            world_border: *world_border,
            players: &tick.players,
            alive_players: &tick.alive_players,
            generation_region: false,
            difficulty,
            dimension_resource_id: dimension_key.as_str(),
            registries,
            datapacks,
            world_seed,
            biome_zoom_seed,
            game_time,
            overworld_clock_time,
            halloween,
            spawn_dimension: Some(spawn_dimension),
            items: entities.item_publication(),
        };
        tick.fatal = {
            let mut runtime = NaturalSpawner {
                environment,
                entities,
                catalog,
                state: &mut tick.state,
                start_chunk: chunk,
                default_spawn,
                fatal: false,
            };
            for category in categories {
                if runtime.fatal {
                    break;
                }
                let Some(start) =
                    random_position_within(runtime.environment.chunks, chunk, world_random)
                else {
                    runtime.fatal = true;
                    break;
                };
                if start.y > runtime.environment.chunks.min_y()
                    && runtime
                        .spawn_category_for_position(category, start, world_random)
                        .is_err()
                {
                    runtime.fatal = true;
                }
            }
            runtime.fatal
        };
    }

    fn spawn_players(&self, dimension: &LevelId) -> Vec<SpawnPlayer> {
        self.levels
            .players()
            .filter(|player| &player.dimension == dimension && !player.avatar_removed)
            .filter_map(SpawnPlayer::from_world)
            .collect()
    }
}

fn natural_spawn_counted_chunks(
    players: &[SpawnPlayer],
    spectators_generate_chunks: bool,
) -> FxHashSet<(i32, i32)> {
    let mut counted = FxHashSet::default();
    for player in players
        .iter()
        .filter(|player| !player.spectator || spectators_generate_chunks)
    {
        let player_chunk_x = (player.position.x.floor() as i32).div_euclid(16);
        let player_chunk_z = (player.position.z.floor() as i32).div_euclid(16);
        for chunk_x in player_chunk_x - NATURAL_SPAWN_CHUNK_RADIUS
            ..=player_chunk_x + NATURAL_SPAWN_CHUNK_RADIUS
        {
            for chunk_z in player_chunk_z - NATURAL_SPAWN_CHUNK_RADIUS
                ..=player_chunk_z + NATURAL_SPAWN_CHUNK_RADIUS
            {
                counted.insert((chunk_x, chunk_z));
            }
        }
    }
    counted
}

mod cycle;

struct NaturalSpawner<'a> {
    environment: NaturalEnvironment<'a>,
    entities: &'a mut EntityStore,
    catalog: &'a DatapackMobSpawnCatalog,
    state: &'a mut NaturalSpawnState,
    start_chunk: (i32, i32),
    default_spawn: (LevelId, Vec3d),
    fatal: bool,
}

impl NaturalSpawner<'_> {
    fn initial_position_allows_spawning(&mut self, position: BlockPos) -> bool {
        !self.fatal
            && block_state(self.environment.chunks, position).is_some_and(|state| {
                !crate::redstone::is_redstone_conductor_with_registry(
                    state,
                    self.environment.chunks.content_registry(),
                )
            })
    }

    fn position_is_eligible(
        &mut self,
        _category: VanillaEntityMobCategory,
        position: BlockPos,
    ) -> bool {
        if self.fatal {
            return false;
        }
        let chunk = (position.x.div_euclid(16), position.z.div_euclid(16));
        if !self.environment.chunks.is_loaded(chunk.0, chunk.1)
            || !self.environment.chunks.is_ticking_full(chunk.0, chunk.1)
        {
            return false;
        }
        let Some(distance_squared) =
            nearest_player_distance_squared(self.environment.players, position)
        else {
            return false;
        };
        if distance_squared <= MIN_PLAYER_DISTANCE_SQUARED {
            return false;
        }
        let target = Vec3d {
            x: f64::from(position.x) + 0.5,
            y: f64::from(position.y),
            z: f64::from(position.z) + 0.5,
        };
        if self.default_spawn.0 == self.environment.dimension_id
            && distance_squared_between(self.default_spawn.1, target) < MIN_PLAYER_DISTANCE_SQUARED
        {
            return false;
        }
        candidate_chunk_allows_spawning(
            self.environment.simulation_levels,
            self.environment.world_border,
            self.environment.game_time,
            self.start_chunk,
            chunk,
        )
    }

    fn select_spawn_data(
        &mut self,
        category: VanillaEntityMobCategory,
        position: BlockPos,
        random: &GameRandom,
    ) -> Option<MobSpawnerData> {
        if self.fatal {
            return None;
        }
        if category == VanillaEntityMobCategory::WaterAmbient
            && biome_matches_tag(
                self.environment.registries,
                self.environment.chunks,
                position,
                self.environment.biome_zoom_seed,
                "minecraft:reduce_water_ambient_spawns",
            )
            && random.next_float() < 0.98
        {
            return None;
        }
        let list = match mobs_at(
            self.catalog,
            self.environment.registries,
            self.environment.chunks,
            category,
            position,
            self.environment.biome_zoom_seed,
        ) {
            Ok(list) => list,
            Err(()) => {
                self.fatal = true;
                return None;
            }
        };
        let data = select_weighted_spawn(list, random)?.clone();
        Some(data)
    }

    fn is_valid_spawn_data(
        &mut self,
        category: VanillaEntityMobCategory,
        data: &MobSpawnerData,
        position: BlockPos,
        random: &GameRandom,
    ) -> bool {
        if self.fatal {
            return false;
        }
        let Some((mob_category, can_spawn_far_from_player, can_summon, width, height)) =
            vanilla_entity_type(data.entity_type_id)
                .map(|entity_type| {
                    (
                        entity_type.mob_category,
                        entity_type.can_spawn_far_from_player,
                        entity_type.can_summon,
                        entity_type.width,
                        entity_type.height,
                    )
                })
                .or_else(|| {
                    self.entities
                        .plugins()
                        .content_registry()
                        .entity_by_id(data.entity_type_id)
                        .map(|spec| {
                            (
                                spec.category,
                                false,
                                spec.can_summon,
                                spec.width,
                                spec.height,
                            )
                        })
                })
        else {
            self.fatal = true;
            return false;
        };
        let Some(distance_squared) =
            nearest_player_distance_squared(self.environment.players, position)
        else {
            return false;
        };
        let despawn_distance = crate::entity::vanilla_mob_category_spawn_facts_26_1(mob_category)
            .map(|facts| f64::from(facts.despawn_distance));
        if mob_category == VanillaEntityMobCategory::Misc
            || !can_spawn_far_from_player
                && despawn_distance.is_none_or(|distance| distance_squared > distance * distance)
            || !can_summon
        {
            return false;
        }
        let list = match mobs_at(
            self.catalog,
            self.environment.registries,
            self.environment.chunks,
            category,
            position,
            self.environment.biome_zoom_seed,
        ) {
            Ok(list) => list,
            Err(()) => {
                self.fatal = true;
                return false;
            }
        };
        if !list.entries.iter().any(|entry| entry.data == *data) {
            return false;
        }
        let entry = crate::entity::vanilla_natural_spawn_type_facts_by_id_26_1(data.entity_type_id)
            .map(|facts| facts.placement)
            .unwrap_or_else(|| {
                crate::entity::minecart_spawner::spawn_placement::spawn_placement_by_id(
                    data.entity_type_id,
                )
            });
        let probe = SpawnPlacementProbe {
            entity_type_id: data.entity_type_id,
            position,
            entry,
        };
        let Some(placement_environment) = spawn_environment(&self.environment, probe) else {
            return false;
        };
        let predicate_level = self.environment.predicate_level();
        if !is_spawn_position_ok(entry.placement, placement_environment)
            || !check_spawn_placement_predicate(
                &predicate_level,
                probe,
                SpawnPredicateReason::Natural,
                random,
            )
            || predicate_level.missing()
            || !collision_free(
                self.environment.chunks,
                self.entities,
                self.environment.players,
                {
                    let scale =
                        crate::entity::minecart_spawner::spawn_placement::spawn_dimensions_scale(
                            data.entity_type_id,
                        );
                    spawn_aabb(width * scale, height * scale, position)
                },
                false,
            )
        {
            return false;
        }
        let cost = match biome_spawn_cost(
            self.catalog,
            self.environment.registries,
            self.environment.chunks,
            position,
            data.entity_type_id,
            self.start_chunk,
        ) {
            Ok(cost) => cost,
            Err(()) => {
                self.fatal = true;
                return false;
            }
        };
        self.state.can_spawn(position, cost)
    }

    fn create_mob(&mut self, data: &MobSpawnerData) -> Option<Box<Entity>> {
        if self.fatal {
            return None;
        }
        let staged = create_natural_spawn_root(
            self.entities,
            data.entity_type_id,
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        );
        if staged.is_none() {
            self.fatal = true;
        }
        staged
    }

    fn finish_mob(
        &mut self,
        mut tree: Box<Entity>,
        position: BlockPos,
        y_rot: f32,
        group_data: Option<&SpawnGroupData>,
        random: &GameRandom,
    ) -> Option<NaturalSpawnPublishResult> {
        if self.fatal {
            self.entities.discard_fresh(tree);
            return None;
        }
        let target = Vec3d {
            x: f64::from(position.x) + 0.5,
            y: f64::from(position.y),
            z: f64::from(position.z) + 0.5,
        };
        tree.set_pos(target);
        tree.set_rot(y_rot, 0.0);
        let Some(distance_squared) =
            nearest_player_distance_squared(self.environment.players, position)
        else {
            self.entities.discard_fresh(tree);
            return None;
        };
        let Some(entity_facts) = natural_spawn_pre_finalize_mob_facts(&tree, distance_squared)
        else {
            self.entities.discard_fresh(tree);
            self.fatal = true;
            return None;
        };
        let entity_type = tree.entity_type_id();
        let category = vanilla_entity_type(entity_type)
            .map(|facts| facts.mob_category)
            .or_else(|| {
                self.entities
                    .plugins()
                    .content_registry()
                    .entity_by_id(entity_type)
                    .map(|spec| spec.category)
            })
            .unwrap_or(VanillaEntityMobCategory::Misc);
        let despawn_distance = crate::entity::vanilla_mob_category_spawn_facts_26_1(category)
            .map(|facts| f64::from(facts.despawn_distance));
        if despawn_distance.is_none_or(|distance| {
            distance_squared > distance * distance && entity_facts.remove_when_far_away
        }) {
            self.entities.discard_fresh(tree);
            return None;
        }
        let Some(rules) = check_environment_natural_spawn_rules(
            &self.environment,
            position,
            entity_type,
            entity_facts,
        ) else {
            self.entities.discard_fresh(tree);
            return None;
        };
        let Ok(rules_ok) = rules else {
            self.entities.discard_fresh(tree);
            self.fatal = true;
            return None;
        };
        if !rules_ok {
            self.entities.discard_fresh(tree);
            return None;
        }
        let Ok(obstruction_ok) = check_environment_natural_spawn_obstruction(
            &self.environment,
            Some(self.entities),
            &tree,
        ) else {
            self.entities.discard_fresh(tree);
            self.fatal = true;
            return None;
        };
        if !obstruction_ok.unwrap_or(false) {
            self.entities.discard_fresh(tree);
            return None;
        }
        let cost = match biome_spawn_cost(
            self.catalog,
            self.environment.registries,
            self.environment.chunks,
            position,
            entity_type,
            self.start_chunk,
        ) {
            Ok(cost) => cost,
            Err(()) => {
                self.entities.discard_fresh(tree);
                self.fatal = true;
                return None;
            }
        };
        let Some(published) = finalize_and_publish_natural_spawn(
            &mut NaturalSpawnLevel {
                environment: &self.environment,
                store: self.entities,
            },
            tree,
            SpawnReason::Natural,
            group_data.cloned(),
            random,
        ) else {
            self.fatal = true;
            return None;
        };
        let nearby = player_ids_close_to_chunk(
            self.environment.players,
            position.x.div_euclid(16),
            position.z.div_euclid(16),
        );
        self.state.after_spawn(category, position, cost, &nearby);
        Some(published)
    }
}

fn distance_squared_between(left: Vec3d, right: Vec3d) -> f64 {
    (left.x - right.x).powi(2) + (left.y - right.y).powi(2) + (left.z - right.z).powi(2)
}

fn nearest_player_distance_squared(players: &[SpawnPlayer], position: BlockPos) -> Option<f64> {
    let target = Vec3d {
        x: f64::from(position.x) + 0.5,
        y: f64::from(position.y),
        z: f64::from(position.z) + 0.5,
    };
    players
        .iter()
        .filter(|player| !player.spectator)
        .map(|player| distance_squared_between(player.position, target))
        .min_by(f64::total_cmp)
}

fn select_weighted_spawn<'a>(
    list: &'a MobSpawnList,
    random: &GameRandom,
) -> Option<&'a MobSpawnerData> {
    if list.total_weight <= 0 {
        return None;
    }
    let mut selection = random.next_int(list.total_weight);
    for entry in &list.entries {
        if selection < entry.weight {
            return Some(&entry.data);
        }
        selection -= entry.weight;
    }
    None
}

// `NaturalSpawner.java:293-340`.
fn mobs_at<'a>(
    catalog: &'a DatapackMobSpawnCatalog,
    registries: &crate::registry_sync::SynchronizedRegistries,
    chunks: &'a ChunkStore,
    category: VanillaEntityMobCategory,
    position: BlockPos,
    biome_zoom_seed: i64,
) -> Result<&'a MobSpawnList, ()> {
    if category == VanillaEntityMobCategory::Monster
        && block_state(chunks, below(position))
            .and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id)
            == Some(bbb_protocol::block_types::VANILLA_BLOCK_NETHER_BRICKS_ID)
        && chunks.natural_spawn_fortress_full_box_contains(position)
    {
        return Ok(fortress_natural_enemy_spawns());
    }
    if let Some(found) = chunks.natural_spawn_structure_override_at(category, position) {
        return Ok(&found.override_.spawns);
    }
    let (quart_x, quart_y, quart_z) =
        fuzzy_biome_quart_position(biome_zoom_seed, position.x, position.y, position.z);
    let biome_id = chunks
        .biome_id_at_quart(quart_x, quart_y, quart_z)
        .ok_or(())?;
    Ok(catalog_biome(catalog, registries, biome_id)
        .ok_or(())?
        .spawns(category))
}

fn biome_spawn_cost(
    catalog: &DatapackMobSpawnCatalog,
    registries: &crate::registry_sync::SynchronizedRegistries,
    chunks: &ChunkStore,
    position: BlockPos,
    entity_type_id: i32,
    source_chunk: (i32, i32),
) -> Result<Option<NaturalSpawnCost>, ()> {
    if !catalog.entity_type_has_any_spawn_cost(entity_type_id) {
        return Ok(None);
    }
    // `NaturalSpawner` retains the starting `ChunkAccess` for candidate costs
    // even after the candidate drifts across its edge (`NaturalSpawner.java:154-202,492-515`).
    let biome_id = chunks
        .biome_id_at_quart_in_chunk(
            source_chunk.0,
            source_chunk.1,
            position.x.div_euclid(4),
            position.y.div_euclid(4),
            position.z.div_euclid(4),
        )
        .ok_or(())?;
    let biome = catalog_biome(catalog, registries, biome_id).ok_or(())?;
    Ok(biome
        .spawn_cost_by_id(entity_type_id)
        .map(|cost| NaturalSpawnCost {
            energy_budget: cost.energy_budget,
            charge: cost.charge,
        }))
}

fn catalog_biome<'a>(
    catalog: &'a DatapackMobSpawnCatalog,
    registries: &crate::registry_sync::SynchronizedRegistries,
    biome_id: i32,
) -> Option<&'a bbb_server_datapack::mob_spawn::BiomeMobSpawnSettings> {
    let resource_id = registries.registry_element_id(BIOME_REGISTRY, biome_id)?;
    if let Some(fast) = usize::try_from(biome_id)
        .ok()
        .and_then(|index| catalog.biome_by_index(index))
        .filter(|biome| biome.biome_id == resource_id)
    {
        return Some(fast);
    }
    catalog.biome(resource_id)
}

pub(super) fn is_halloween_today() -> bool {
    use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static LAST_CHECK_SECS: AtomicI64 = AtomicI64::new(-1);
    static CACHED: AtomicU8 = AtomicU8::new(0);

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let last = LAST_CHECK_SECS.load(Ordering::Relaxed);
    if last >= 0 && now_secs >= last && now_secs - last < 60 {
        return CACHED.load(Ordering::Relaxed) != 0;
    }
    let today = Local::now();
    let halloween = today.month() == 10 && today.day() == 31;
    CACHED.store(u8::from(halloween), Ordering::Relaxed);
    LAST_CHECK_SECS.store(now_secs, Ordering::Relaxed);
    halloween
}

pub(crate) struct NaturalEnvironment<'a> {
    dimension_id: LevelId,
    spawn_environment: super::environment_attributes::SpawnEnvironmentContext<'a>,
    chunks: &'a ChunkStore,
    simulation_levels: &'a SimulationChunkLevels,
    world_border: WorldBorder,
    players: &'a [SpawnPlayer],
    /// Feet positions of `players` that are alive and not spectating.
    alive_players: &'a [Vec3d],
    /// A chunk-generation `WorldGenRegion` rather than the live level.
    generation_region: bool,
    difficulty: Difficulty,
    dimension_resource_id: &'a str,
    registries: &'a crate::registry_sync::SynchronizedRegistries,
    datapacks: Option<&'a DatapackStartup>,
    world_seed: i64,
    biome_zoom_seed: i64,
    game_time: i64,
    overworld_clock_time: i64,
    halloween: bool,
    spawn_dimension: Option<SpawnDimensionFacts>,
    items: crate::entity::ItemPublication,
}

impl NaturalEnvironment<'_> {
    fn predicate_level(&self) -> SpawnPredicateLevel<'_> {
        SpawnPredicateLevel {
            chunks: self.chunks,
            registries: self.registries,
            biome_zoom_seed: self.biome_zoom_seed,
            world_seed: self.world_seed,
            difficulty: self.difficulty,
            spawn_dimension: self.spawn_dimension.as_ref(),
            spawn_environment: &self.spawn_environment,
            players: self.alive_players,
            generation_region: self.generation_region,
            missing: Cell::new(false),
        }
    }
}

#[cfg(test)]
pub(crate) fn environment_for_test<'a>(
    world: &'a ServerWorld,
    dimension: &'a Java26LevelKey,
) -> NaturalEnvironment<'a> {
    let level = world.level_by_id(&world.require_level_id(dimension));
    NaturalEnvironment {
        dimension_id: world.require_level_id(dimension),
        spawn_environment: world.cold.dimension_sky_lights.spawn_context(
            world
                .cold
                .level_dimension_types
                .get(&world.require_level_id(dimension))
                .map(String::as_str),
            &world.registries,
            &world.world_clocks,
        ),
        chunks: &level.chunks,
        simulation_levels: &level.simulation_levels,
        world_border: level.world_border,
        players: &[],
        alive_players: &[],
        generation_region: false,
        difficulty: Difficulty::Easy,
        dimension_resource_id: dimension.as_str(),
        registries: &world.registries,
        datapacks: None,
        world_seed: world.cold.seed,
        biome_zoom_seed: biome_zoom_seed(world.cold.seed),
        game_time: world.game_time,
        overworld_clock_time: 0,
        halloween: false,
        spawn_dimension: spawn_dimension_facts(None, Some("minecraft:overworld"), false),
        items: level.entities.item_publication(),
    }
}

fn alive_player_positions(players: &[SpawnPlayer]) -> Vec<Vec3d> {
    // `EntitySelector.NO_CREATIVE_OR_SPECTATOR` (`Silverfish.java:123`, `Endermite.java:142`).
    players
        .iter()
        .filter(|player| !player.spectator && !player.creative && !player.dead_or_dying)
        .map(|player| player.position)
        .collect()
}

impl<'a> NaturalEnvironment<'a> {
    fn spawn_context(&self, resource_id: &str, position: BlockPos) -> Option<SpawnContext<'a>> {
        let moon = self
            .spawn_environment
            .moon_brightness(self.chunks, position)?;
        let (moon_brightness, special_multiplier) = if self.generation_region {
            // WorldGenRegion.java:347-353: difficulty retains the level's
            // positional moon; WorldGenRegion.java:459-460 and
            // EnvironmentAttributes.java:64-65: attributes read the default.
            let special = crate::entity::difficulty_special_multiplier(
                self.difficulty,
                self.overworld_clock_time,
                0,
                moon,
            );
            (1.0, special)
        } else {
            let special = self.spawn_environment.special_multiplier(
                self.chunks,
                position,
                self.difficulty,
                self.overworld_clock_time,
            )?;
            (moon, special)
        };
        let biome =
            fuzzy_biome_resource_id(self.registries, self.chunks, self.biome_zoom_seed, position)?;
        let mut variants =
            variant_registry_for_entity(resource_id).map_or_else(Vec::new, |registry| {
                select_animal_variants(self.datapacks, self.registries, registry, |condition| {
                    spawn_condition_matches(self, position, biome, moon_brightness, condition)
                })
            });
        if let Some(registry) = sound_variant_registry_for_entity(resource_id) {
            variants
                .extend(sound_variant_candidates(self.datapacks, self.registries, registry).ok()?);
        }
        Some(SpawnContext {
            game_time: self.game_time,
            difficulty: self.difficulty,
            special_multiplier,
            dimension: self.dimension_resource_id.to_owned(),
            biome: Some(biome.to_owned()),
            biome_tags: animal_biome_tags(self.chunks.registry_tags(), biome),
            variants,
            halloween: self.halloween,
        })
    }

    /// A generation region sees no previously spawned vehicles.
    fn first_unridden_vehicle(
        &self,
        store: &EntityStore,
        resource_id: &str,
        search_box: Aabb,
    ) -> Option<i32> {
        if self.generation_region {
            return None;
        }
        super::spawner_minecarts::first_unridden_alive(store, resource_id, search_box)
    }

    fn no_collision(&self, store: &EntityStore, bounds: Aabb) -> bool {
        if self.generation_region {
            return no_block_collisions(self.chunks, bounds).unwrap_or(false);
        }
        // Husk.java:106: the camel-husk probe uses a source-null type box.
        collision_free(self.chunks, store, self.players, bounds, false)
    }
}

/// A [`NaturalEnvironment`] over the store its spawns publish into.
pub(crate) struct NaturalSpawnLevel<'a, 'b> {
    pub(crate) environment: &'a NaturalEnvironment<'b>,
    pub(crate) store: &'a mut EntityStore,
}

impl<'registry> SpawnLevel<'registry> for NaturalSpawnLevel<'_, 'registry> {
    fn entities(&mut self) -> &mut EntityStore {
        self.store
    }

    fn spawn_context(&self, resource_id: &str, pos: BlockPos) -> Option<SpawnContext<'registry>> {
        self.environment.spawn_context(resource_id, pos)
    }

    fn enchant(
        &self,
        provider_id: &str,
        stack: &RuntimeItemStack,
        special_multiplier: f32,
        random: &GameRandom,
    ) -> Option<RuntimeItemStack> {
        apply_enchantment_provider_to_stack(
            self.environment,
            provider_id,
            stack,
            special_multiplier,
            random,
        )
    }

    fn first_unridden(&self, resource_id: &str, search_box: Aabb) -> Option<i32> {
        self.environment
            .first_unridden_vehicle(self.store, resource_id, search_box)
    }

    fn no_collision(&self, bounds: Aabb) -> bool {
        self.environment.no_collision(self.store, bounds)
    }
}

impl SpawnEnchantmentProviderEnvironment for NaturalEnvironment<'_> {
    fn registry_tags(&self) -> Option<&crate::registry_tags::RegistryTagSnapshot> {
        self.chunks.registry_tags()
    }

    fn datapacks(&self) -> Option<&DatapackStartup> {
        self.datapacks
    }

    fn item_holders(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self.items.resolver()
    }
}

pub(super) fn check_environment_natural_spawn_rules(
    environment: &NaturalEnvironment<'_>,
    position: BlockPos,
    entity_type_id: i32,
    entity: crate::entity::natural_spawn::NaturalSpawnPreFinalizeMobFacts,
) -> Option<Result<bool, crate::entity::natural_spawn::mob_eligibility::NaturalSpawnEligibilityError>>
{
    let chunks = environment.chunks;
    let blocks = [
        block_state(chunks, position)?,
        block_state(chunks, below(position))?,
    ];
    let air_at_and_below =
        if entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            [
                blocks[0],
                blocks[1],
                block_state(
                    chunks,
                    BlockPos {
                        y: position.y - 2,
                        ..position
                    },
                )?,
            ]
            .map(bbb_server_types::fluid::block_state_is_air)
        } else {
            [false; 3]
        };
    let result = check_natural_spawn_rules(
        entity_type_id,
        NaturalSpawnRulesProbe {
            pathfinding_cost_from_light_levels: move || {
                pathfinding_light_cost(environment, position)
            },
            block_at: move |depth| blocks[depth],
            below_is_sand: move |state| tag_member(chunks, "minecraft:sand", state),
            air_at_and_below,
            turtle_going_home: entity.turtle_going_home,
            strider_is_in_lava: entity.strider_is_in_lava,
            // A fresh mob has no sensed repellent memory (`HoglinAi.java:165-168`).
            hoglin_near_repellent: false,
        },
    );
    Some(match result {
        Err(crate::entity::natural_spawn::mob_eligibility::NaturalSpawnEligibilityError::UnknownEntityType(_))
            if environment
                .registries
                .content_registry()
                .entity_by_id(entity_type_id)
                .is_some_and(|spec| spec.is_mob) =>
        {
            Ok(true)
        }
        other => other,
    })
}

pub(super) fn check_environment_natural_spawn_obstruction(
    environment: &NaturalEnvironment<'_>,
    entities: Option<&EntityStore>,
    entity: &crate::entity::Entity,
) -> Result<Option<bool>, crate::entity::natural_spawn::mob_eligibility::NaturalSpawnEligibilityError>
{
    let entity_type_id = entity.entity_type_id();
    let rule = match natural_spawn_obstruction_rule(entity_type_id) {
        Ok(rule) => rule,
        Err(
            crate::entity::natural_spawn::mob_eligibility::NaturalSpawnEligibilityError::UnknownEntityType(_),
        ) if matches!(
            entity.data(),
            crate::entity::EntityData::GeneralLiving(data)
                if data.plugin_mob().is_some() && data.is_mob()
        ) =>
        {
            SpawnObstructionRule::Standard
        }
        Err(error) => return Err(error),
    };
    let position = entity.block_position();
    let bounds = entity.bounding_box();
    let chunks = environment.chunks;
    let is_unobstructed = || match entities {
        Some(store) => collision_free(chunks, store, environment.players, bounds, true),
        None => true,
    };
    let ok = match rule {
        SpawnObstructionRule::Standard => !box_contains_liquid(chunks, bounds) && is_unobstructed(),
        SpawnObstructionRule::WaterOrLava => is_unobstructed(),
        SpawnObstructionRule::Ocelot => {
            let below = below(position);
            position.y >= chunks.sea_level()
                && (is_block(chunks, below, "minecraft:grass_block")
                    || block_state(chunks, below)
                        .is_some_and(|state| tag_member(chunks, "minecraft:leaves", state)))
                && !box_contains_liquid(chunks, bounds)
                && is_unobstructed()
        }
        SpawnObstructionRule::IronGolem => {
            let below = below(position);
            block_state(chunks, below)
                .and_then(vanilla_block_state_support_facts)
                .is_some_and(|facts| facts.collision_face_full(VanillaBlockSupportDirection::Up))
                && valid_empty_spawn_block_inner(chunks, position, entity_type_id, false)
                && valid_empty_spawn_block(chunks, above(position), entity_type_id)
                && valid_empty_spawn_block(
                    chunks,
                    BlockPos {
                        y: position.y + 2,
                        ..position
                    },
                    entity_type_id,
                )
                && is_unobstructed()
        }
        SpawnObstructionRule::Ravager => !box_contains_liquid(chunks, bounds),
        SpawnObstructionRule::Warden => {
            if box_contains_liquid(chunks, bounds) || !is_unobstructed() {
                false
            } else {
                let Some(facts) = vanilla_entity_type(entity_type_id) else {
                    return Ok(Some(false));
                };
                match entities {
                    Some(store) => collision_free(
                        chunks,
                        store,
                        environment.players,
                        spawn_aabb(facts.width, facts.height, position),
                        false,
                    ),
                    None => {
                        let Some(clear) = no_block_collisions(
                            chunks,
                            Aabb::of_entity(entity.position(), facts.width, facts.height),
                        ) else {
                            return Ok(None);
                        };
                        clear
                    }
                }
            }
        }
    };
    Ok(Some(ok))
}

#[cfg(test)]
fn natural_obstruction_probe(
    environment: &NaturalEnvironment<'_>,
    entities: &EntityStore,
    entity: &crate::entity::Entity,
) -> NaturalSpawnObstructionProbe {
    let position = entity.block_position();
    let bounds = entity.bounding_box();
    let below = below(position);
    let entity_type_id = entity.entity_type_id();
    let type_bounds = vanilla_entity_type(entity_type_id)
        .map(|facts| spawn_aabb(facts.width, facts.height, position));
    NaturalSpawnObstructionProbe {
        position,
        sea_level: environment.chunks.sea_level(),
        is_unobstructed: collision_free(
            environment.chunks,
            entities,
            environment.players,
            bounds,
            true,
        ),
        contains_any_liquid: box_contains_liquid(environment.chunks, bounds),
        default_dimensions_no_collision: type_bounds.is_some_and(|bounds| {
            collision_free(
                environment.chunks,
                entities,
                environment.players,
                bounds,
                false,
            )
        }),
        below_is_grass_or_leaves: is_block(environment.chunks, below, "minecraft:grass_block")
            || block_state(environment.chunks, below)
                .is_some_and(|state| tag_member(environment.chunks, "minecraft:leaves", state)),
        below_entity_can_stand: block_state(environment.chunks, below)
            .and_then(vanilla_block_state_support_facts)
            .is_some_and(|facts| facts.collision_face_full(VanillaBlockSupportDirection::Up)),
        at_is_valid_empty_spawn_block: valid_empty_spawn_block_inner(
            environment.chunks,
            position,
            entity_type_id,
            false,
        ),
        above_one_is_valid_empty_spawn_block: valid_empty_spawn_block(
            environment.chunks,
            above(position),
            entity_type_id,
        ),
        above_two_is_valid_empty_spawn_block: valid_empty_spawn_block(
            environment.chunks,
            BlockPos {
                y: position.y + 2,
                ..position
            },
            entity_type_id,
        ),
    }
}

fn spawn_aabb(width: f32, height: f32, position: BlockPos) -> Aabb {
    let half = f64::from(width) / 2.0;
    let x = f64::from(position.x) + 0.5;
    let y = f64::from(position.y);
    let z = f64::from(position.z) + 0.5;
    Aabb {
        min_x: x - half,
        min_y: y,
        min_z: z - half,
        max_x: x + half,
        max_y: y + f64::from(height),
        max_z: z + half,
    }
}

fn collision_free(
    chunks: &ChunkStore,
    entities: &EntityStore,
    players: &[SpawnPlayer],
    bounds: Aabb,
    blocks_building: bool,
) -> bool {
    no_block_collisions(chunks, bounds).unwrap_or(false)
        && entities
            .first_entity_in_box(bounds, |entity| {
                if blocks_building {
                    entity.blocks_building()
                } else {
                    entity.can_be_collided_with()
                }
            })
            .is_none()
        // EntityGetter.java:54-62: source-null collision excludes Player;
        // instance obstruction still reads its blocksBuilding box (:33-48).
        && (!blocks_building
            || !players
                .iter()
                .filter(|player| !player.spectator)
                .any(|player| player.bounds.intersects(bounds)))
}

fn spawn_environment(
    environment: &NaturalEnvironment<'_>,
    probe: SpawnPlacementProbe,
) -> Option<SpawnPlacementEnvironment> {
    use crate::entity::SpawnPlacementType;

    let position = probe.position;
    let chunks = environment.chunks;
    if matches!(probe.entry.placement, SpawnPlacementType::NoRestrictions) {
        return Some(SpawnPlacementEnvironment::default());
    }
    let inside_world_border = environment
        .world_border
        .contains_block_at(position, environment.game_time);
    let mut env = SpawnPlacementEnvironment {
        inside_world_border,
        ..SpawnPlacementEnvironment::default()
    };
    if !inside_world_border {
        return Some(env);
    }
    match probe.entry.placement {
        SpawnPlacementType::NoRestrictions => unreachable!(),
        SpawnPlacementType::InWater => {
            env.fluid_at = spawn_fluid(chunks, position)?;
            if matches!(env.fluid_at, SpawnFluid::Water) {
                env.above_is_redstone_conductor =
                    crate::redstone::is_redstone_conductor_with_registry(
                        block_state(chunks, above(position))?,
                        chunks.content_registry(),
                    );
            }
        }
        SpawnPlacementType::InLava => {
            env.fluid_at = spawn_fluid(chunks, position)?;
        }
        SpawnPlacementType::OnGround => {
            let at_state = block_state(chunks, position)?;
            let above_state = block_state(chunks, above(position))?;
            env.below_is_valid_spawn =
                valid_spawn_floor(chunks, below(position), probe.entity_type_id);
            env.at_is_valid_empty_spawn_block = env.below_is_valid_spawn
                && valid_empty_spawn_state(chunks, at_state, probe.entity_type_id, true);
            env.above_is_valid_empty_spawn_block = env.at_is_valid_empty_spawn_block
                && valid_empty_spawn_state(chunks, above_state, probe.entity_type_id, true);
        }
    }
    Some(env)
}

pub(super) fn valid_spawn_floor(
    chunks: &ChunkStore,
    position: BlockPos,
    entity_type_id: i32,
) -> bool {
    block_state(chunks, position).is_some_and(|state| {
        super::spawner_minecarts::valid_spawn_floor_state(chunks, state, entity_type_id)
    })
}

fn valid_empty_spawn_block(chunks: &ChunkStore, position: BlockPos, entity_type_id: i32) -> bool {
    valid_empty_spawn_block_inner(chunks, position, entity_type_id, true)
}

fn valid_empty_spawn_block_inner(
    chunks: &ChunkStore,
    position: BlockPos,
    entity_type_id: i32,
    check_fluid: bool,
) -> bool {
    let Some(state) = block_state(chunks, position) else {
        return false;
    };
    valid_empty_spawn_state(chunks, state, entity_type_id, check_fluid)
}

pub(super) fn valid_empty_spawn_state(
    chunks: &ChunkStore,
    state: i32,
    entity_type_id: i32,
    check_fluid: bool,
) -> bool {
    let full_collision = static_block_collision_box_iter(state).is_some_and(|mut boxes| {
        boxes.len() == 1 && boxes.next() == Some(Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 }))
    });
    if full_collision
        || is_signal_source(state, WireSignal::Emitting)
        || check_fluid && !block_state_fluid_state(state).is_empty()
        || tag_member(chunks, PREVENT_MOB_SPAWNING_INSIDE, state)
    {
        return false;
    }
    !is_block_dangerous(chunks, state, entity_type_id)
}

pub(super) fn is_block_dangerous(chunks: &ChunkStore, state: i32, entity_type_id: i32) -> bool {
    let resource_id = block_resource_id(state).unwrap_or_default();
    let Some(entity) = vanilla_entity_type(entity_type_id) else {
        return true;
    };
    // `EntityType.isBlockDangerous` (`EntityType.java:1462-1470`) applies the
    // builder immunity set before burning and the four unconditional hazards.
    let immune = matches!(
        (entity.resource_id, resource_id),
        ("minecraft:fox", "minecraft:sweet_berry_bush")
            | (
                "minecraft:polar_bear" | "minecraft:snow_golem" | "minecraft:stray",
                "minecraft:powder_snow"
            )
            | (
                "minecraft:wither" | "minecraft:wither_skeleton",
                "minecraft:wither_rose"
            )
    );
    if immune {
        return false;
    }
    let burning = tag_member(chunks, "minecraft:fire", state)
        || matches!(
            resource_id,
            "minecraft:lava" | "minecraft:magma_block" | "minecraft:lava_cauldron"
        )
        || (matches!(
            resource_id,
            "minecraft:campfire" | "minecraft:soul_campfire"
        ) && bool_property(state, "lit") == Some(true));
    (!entity.fire_immune && burning)
        || matches!(
            resource_id,
            "minecraft:wither_rose"
                | "minecraft:sweet_berry_bush"
                | "minecraft:cactus"
                | "minecraft:powder_snow"
        )
}

pub(super) fn block_state(chunks: &ChunkStore, position: BlockPos) -> Option<i32> {
    let max_y = chunks.min_y() + i32::try_from(chunks.section_count()).ok()? * 16;
    if position.y < chunks.min_y() || position.y >= max_y {
        return Some(chunks.air_state_id());
    }
    chunks.get_block_state(position.x, position.y, position.z)
}

pub(super) fn block_resource_id(state: i32) -> Option<&'static str> {
    vanilla_block_state_block_type_id(state)
        .and_then(vanilla_block_type)
        .map(|block| block.resource_id)
}

fn is_block(chunks: &ChunkStore, position: BlockPos, resource_id: &str) -> bool {
    block_state(chunks, position).and_then(block_resource_id) == Some(resource_id)
}

pub(super) fn tag_member(chunks: &ChunkStore, tag: &str, state: i32) -> bool {
    let Some(resource_id) = block_resource_id(state) else {
        return false;
    };
    chunks
        .registry_tags()
        .is_some_and(|tags| tags.contains(BLOCK_REGISTRY, tag, resource_id))
}

fn spawn_fluid(chunks: &ChunkStore, position: BlockPos) -> Option<SpawnFluid> {
    Some(
        match block_state_fluid_state(block_state(chunks, position)?).family() {
            Some(FluidFamily::Water) => SpawnFluid::Water,
            Some(FluidFamily::Lava) => SpawnFluid::Lava,
            None => SpawnFluid::Empty,
        },
    )
}

fn below(position: BlockPos) -> BlockPos {
    BlockPos {
        y: position.y - 1,
        ..position
    }
}

fn above(position: BlockPos) -> BlockPos {
    BlockPos {
        y: position.y + 1,
        ..position
    }
}

// `LevelReader.java:108-116,162-170`.
fn pathfinding_light_cost(environment: &NaturalEnvironment<'_>, position: BlockPos) -> f32 {
    let brightness = f32::from(
        environment
            .chunks
            .block_light_level(position.x, position.y, position.z)
            .max(
                environment
                    .chunks
                    .sky_light_level(position.x, position.y, position.z)
                    .saturating_sub(
                        environment
                            .spawn_dimension
                            .as_ref()
                            .map_or(0, |facts| facts.sky_darken),
                    ),
            ),
    ) / 15.0;
    let curved = brightness / (4.0 - 3.0 * brightness);
    let ambient = environment
        .spawn_dimension
        .as_ref()
        .map_or(0.0, |facts| facts.ambient_light);
    curved + ambient * (1.0 - curved) - 0.5
}

pub(super) fn world_surface_height(chunks: &ChunkStore, position: BlockPos) -> Option<i32> {
    chunks
        .final_heightmap_first_available(
            position.x.div_euclid(16),
            position.z.div_euclid(16),
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            position.x.rem_euclid(16),
            position.z.rem_euclid(16),
        )
        .or_else(|| {
            bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps::scan_first_available(
                chunks.column(position.x.div_euclid(16), position.z.div_euclid(16))?,
                VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                position.x.rem_euclid(16),
                position.z.rem_euclid(16),
            )
        })
}

fn no_block_collisions(chunks: &ChunkStore, bounds: Aabb) -> Option<bool> {
    no_block_collision_with_scratch(
        bounds,
        |position| {
            block_state(chunks, position)?;
            super::movement_collision::moving_piston_at(chunks, position)
        },
        |position, out| {
            let state = block_state(chunks, position)?;
            super::movement_collision::push_static_collision_shape_with_live_piston(
                chunks, position, state, out,
            );
            Some(())
        },
    )
}

pub(super) fn box_contains_liquid(chunks: &ChunkStore, bounds: Aabb) -> bool {
    let min_x = bounds.min_x.floor() as i32;
    let min_y = bounds.min_y.floor() as i32;
    let min_z = bounds.min_z.floor() as i32;
    let max_x = bounds.max_x.ceil() as i32;
    let max_y = bounds.max_y.ceil() as i32;
    let max_z = bounds.max_z.ceil() as i32;
    (min_x..max_x).any(|x| {
        (min_y..max_y).any(|y| {
            (min_z..max_z).any(|z| {
                block_state(chunks, BlockPos { x, y, z })
                    .is_none_or(|state| !block_state_fluid_state(state).is_empty())
            })
        })
    })
}

#[cfg(test)]
fn rough_biome_resource_id<'a>(
    registries: &'a crate::registry_sync::SynchronizedRegistries,
    chunks: &ChunkStore,
    source_chunk: (i32, i32),
    position: BlockPos,
) -> Option<&'a str> {
    let biome_id = chunks.biome_id_at_quart_in_chunk(
        source_chunk.0,
        source_chunk.1,
        position.x.div_euclid(4),
        position.y.div_euclid(4),
        position.z.div_euclid(4),
    )?;
    registries.registry_element_id(BIOME_REGISTRY, biome_id)
}

pub(super) fn fuzzy_biome_resource_id<'a>(
    registries: &'a crate::registry_sync::SynchronizedRegistries,
    chunks: &ChunkStore,
    biome_zoom_seed: i64,
    position: BlockPos,
) -> Option<&'a str> {
    let (quart_x, quart_y, quart_z) =
        fuzzy_biome_quart_position(biome_zoom_seed, position.x, position.y, position.z);
    let biome_id = chunks.biome_id_at_quart(quart_x, quart_y, quart_z)?;
    registries.registry_element_id(BIOME_REGISTRY, biome_id)
}

fn biome_matches_tag(
    registries: &crate::registry_sync::SynchronizedRegistries,
    chunks: &ChunkStore,
    position: BlockPos,
    biome_zoom_seed: i64,
    tag: &str,
) -> bool {
    fuzzy_biome_resource_id(registries, chunks, biome_zoom_seed, position).is_some_and(
        |resource_id| {
            chunks
                .registry_tags()
                .is_some_and(|tags| tags.contains(BIOME_REGISTRY, tag, resource_id))
        },
    )
}

fn spawn_condition_matches(
    environment: &NaturalEnvironment<'_>,
    position: BlockPos,
    biome: &str,
    moon_brightness: f32,
    condition: &SpawnCondition,
) -> bool {
    match condition {
        SpawnCondition::Biome { biomes } => holder_set_contains(
            environment.chunks.registry_tags(),
            BIOME_REGISTRY,
            biomes,
            biome,
        ),
        SpawnCondition::Structure { structures } => {
            structure_at_spawn(environment, position, structures)
        }
        SpawnCondition::MoonBrightness { range } => {
            let value = f64::from(moon_brightness);
            range.min.is_none_or(|min| value >= min.get())
                && range.max.is_none_or(|max| value <= max.get())
        }
    }
}

fn structure_at_spawn(
    environment: &NaturalEnvironment<'_>,
    position: BlockPos,
    structures: &HolderSetReference,
) -> bool {
    environment
        .chunks
        .natural_spawn_structure_target_at(position)
        .is_some_and(|target| {
            target.referenced_piece_contains(
                position,
                |id| {
                    holder_set_contains(
                        environment.chunks.registry_tags(),
                        STRUCTURE_REGISTRY,
                        structures,
                        id,
                    )
                },
                |x, z| environment.chunks.structure_component(x, z),
            )
        })
}

fn shuffle<T>(values: &mut [T], random: &GameRandom) {
    for index in (1..values.len()).rev() {
        let swap = random.next_int(i32::try_from(index + 1).unwrap()) as usize;
        values.swap(index, swap);
    }
}

// `WorldBorder.java:56-58`; `ServerLevel.java:1814-1816`.
fn world_border_contains_chunk(border: WorldBorder, chunk: (i32, i32), game_time: i64) -> bool {
    let min = BlockPos {
        x: chunk.0 * 16,
        y: 0,
        z: chunk.1 * 16,
    };
    let max = BlockPos {
        x: min.x + 15,
        y: 0,
        z: min.z + 15,
    };
    border.contains_block_at(min, game_time) && border.contains_block_at(max, game_time)
}

fn candidate_chunk_allows_spawning(
    levels: &SimulationChunkLevels,
    border: WorldBorder,
    game_time: i64,
    start_chunk: (i32, i32),
    candidate_chunk: (i32, i32),
) -> bool {
    candidate_chunk == start_chunk
        || levels.is_entity_ticking(candidate_chunk)
            && world_border_contains_chunk(border, candidate_chunk, game_time)
}

// `NaturalSpawner.java:342-349`.
fn random_position_within(
    chunks: &ChunkStore,
    chunk: (i32, i32),
    random: &GameRandom,
) -> Option<BlockPos> {
    let x = chunk.0.checked_mul(16)?.checked_add(random.next_int(16))?;
    let z = chunk.1.checked_mul(16)?.checked_add(random.next_int(16))?;
    let top = chunks.final_heightmap_first_available(
        chunk.0,
        chunk.1,
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        x.rem_euclid(16),
        z.rem_euclid(16),
    )?;
    let min_y = chunks.min_y();
    let bound = top.checked_sub(min_y)?.checked_add(1)?;
    let y = min_y.checked_add(random.next_int(bound))?;
    Some(BlockPos { x, y, z })
}

fn players_close_to_chunk(players: &[SpawnPlayer], chunk_x: i32, chunk_z: i32) -> bool {
    let center_x = f64::from(chunk_x * 16 + 8);
    let center_z = f64::from(chunk_z * 16 + 8);
    players.iter().any(|player| {
        !player.spectator
            && (player.position.x - center_x).powi(2) + (player.position.z - center_z).powi(2)
                < PLAYER_SPAWN_DISTANCE_SQUARED
    })
}

fn fill_player_ids_close_to_chunk(
    players: &[SpawnPlayer],
    chunk_x: i32,
    chunk_z: i32,
    out: &mut Vec<Uuid>,
) {
    out.clear();
    let center_x = f64::from(chunk_x * 16 + 8);
    let center_z = f64::from(chunk_z * 16 + 8);
    out.extend(
        players
            .iter()
            .filter(|player| {
                !player.spectator
                    && (player.position.x - center_x).powi(2)
                        + (player.position.z - center_z).powi(2)
                        < PLAYER_SPAWN_DISTANCE_SQUARED
            })
            .map(|player| player.uuid),
    );
}

fn player_ids_close_to_chunk(players: &[SpawnPlayer], chunk_x: i32, chunk_z: i32) -> Vec<Uuid> {
    let mut out = Vec::with_capacity(players.len());
    fill_player_ids_close_to_chunk(players, chunk_x, chunk_z, &mut out);
    out
}

#[cfg(test)]
mod tests;

pub(super) mod chunk_generation;
