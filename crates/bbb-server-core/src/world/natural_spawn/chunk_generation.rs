use super::*;
use crate::entity::{EntityIdCounter, SpawnHeightmap, SpawnPlacementType};
use crate::world::LevelId;
use bbb_protocol::block_state_types::vanilla_block_state_path_facts;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
};
use bbb_protocol::NbtSummaryEntry;
use bbb_server_types::save::{SaveError, SaveResult};

const MAX_ATTEMPTS: usize = 16_384;

fn refusal(detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("original chunk mobs: {}", detail.into()),
    }
}

#[derive(Clone, Copy)]
pub(in crate::world) struct OriginalMobWorldInputs<'a> {
    dimension_id: LevelId,
    catalog: Option<&'a DatapackMobSpawnCatalog>,
    datapacks: Option<&'a DatapackStartup>,
    plugins: &'a std::sync::Arc<crate::plugin::ServerPluginRegistry>,
    registries: &'a crate::registry_sync::SynchronizedRegistries,
    sky_lights: &'a super::super::environment_attributes::DimensionSkyLights,
    clocks: &'a super::super::world_clock::ServerWorldClocks,
    dimension_type: Option<&'a str>,
    difficulty: Difficulty,
    spawn_mobs: bool,
    seed: i64,
    game_time: i64,
    halloween: bool,
}

impl<'a> OriginalMobWorldInputs<'a> {
    pub(in crate::world) fn from_parts(
        config: &'a crate::world::ServerWorldConfig,
        catalog: &'a Option<super::MobSpawnCatalog>,
        (registries, sky_lights, clocks, dimension_types): (
            &'a crate::registry_sync::SynchronizedRegistries,
            &'a super::super::environment_attributes::DimensionSkyLights,
            &'a super::super::world_clock::ServerWorldClocks,
            &'a std::collections::BTreeMap<LevelId, String>,
        ),
        admin: &crate::admin::AdminState,
        (rules, seed, game_time, dimension): (
            super::super::simulation_level::SimulationGameRules,
            i64,
            i64,
            LevelId,
        ),
    ) -> Self {
        Self {
            dimension_id: dimension,
            catalog: catalog.as_deref(),
            datapacks: config.datapacks.as_deref(),
            plugins: &config.plugins,
            registries,
            sky_lights,
            clocks,
            dimension_type: dimension_types.get(&dimension).map(String::as_str),
            difficulty: admin.settings().difficulty,
            spawn_mobs: rules.spawn_mobs(),
            seed,
            game_time,
            halloween: super::is_halloween_today(),
        }
    }
}

#[derive(Clone)]
pub(in crate::world) struct OriginalMobInputs<'a> {
    world: OriginalMobWorldInputs<'a>,
    dimension: &'a Java26LevelKey,
    simulation_levels: &'a SimulationChunkLevels,
    world_border: WorldBorder,
    thundering: bool,
    items: crate::entity::ItemPublication,
}

impl ServerWorld {
    pub(in super::super) fn generate_original_mob_columns(
        &mut self,
        dimension: &LevelId,
        centers: &[(i32, i32)],
    ) -> SaveResult<()> {
        let input = self.entity_nbt_read_snapshot();
        let (chunks, inputs) = self.original_mob_inputs(dimension);
        inputs.prepare(chunks, centers)?;
        inputs.finalize_structure_mobs_in(chunks, centers, &input);
        Ok(())
    }

    pub(in crate::world) fn original_mob_inputs(
        &mut self,
        dimension: &LevelId,
    ) -> (&mut ChunkStore, OriginalMobInputs<'_>) {
        let world = OriginalMobWorldInputs::from_parts(
            &self.cold.config,
            &self.cold.mob_spawns,
            (
                &self.registries,
                &self.cold.dimension_sky_lights,
                &self.world_clocks,
                &self.cold.level_dimension_types,
            ),
            &self.admin,
            (self.game_rules, self.cold.seed, self.game_time, *dimension),
        );
        let level = self
            .levels
            .get_mut(dimension)
            .expect("served generation level");
        let inputs = OriginalMobInputs::new(
            world,
            &level.dimension,
            &level.simulation_levels,
            level.world_border,
            &level.weather,
            level.entities.item_publication(),
        );
        (&mut level.chunks, inputs)
    }
}

impl<'a> OriginalMobInputs<'a> {
    pub(in crate::world) fn is_bright_outside(&self, sky_darkening: i32) -> Option<bool> {
        self.world
            .sky_lights
            .is_bright_outside(self.world.dimension_type?, Some(sky_darkening))
    }

    pub(in crate::world) fn is_dark_outside(&self, sky_darkening: i32) -> Option<bool> {
        self.world
            .sky_lights
            .is_dark_outside(self.world.dimension_type?, sky_darkening)
    }

    pub(in crate::world) fn difficulty(&self) -> Difficulty {
        self.world.difficulty
    }

    fn environment(&self) -> super::super::environment_attributes::SpawnEnvironmentContext<'a> {
        let world = self.world;
        world
            .sky_lights
            .spawn_context(world.dimension_type, world.registries, world.clocks)
    }

    pub(in crate::world) fn local_special_multiplier(
        &self,
        chunks: &ChunkStore,
        position: BlockPos,
    ) -> Option<f32> {
        let environment = self.environment();
        environment.special_multiplier(
            chunks,
            position,
            self.world.difficulty,
            environment.overworld_clock_time()?,
        )
    }

    pub(in crate::world) fn local_effective_difficulty(
        &self,
        chunks: &ChunkStore,
        position: BlockPos,
    ) -> Option<f32> {
        let environment = self.environment();
        environment.effective_difficulty(
            chunks,
            position,
            self.world.difficulty,
            environment.overworld_clock_time()?,
        )
    }

    pub(in crate::world) fn cat_waking_up_gift_chance(
        &self,
        chunks: &ChunkStore,
        position: BlockPos,
    ) -> Option<f32> {
        self.environment()
            .cat_waking_up_gift_chance(chunks, position)
    }

    pub(in crate::world) fn snow_golem_melts(
        &self,
        chunks: &ChunkStore,
        position: BlockPos,
    ) -> Option<bool> {
        self.environment().snow_golem_melts(chunks, position)
    }

    pub(in crate::world) fn turtle_egg_hatch_chance(
        &self,
        chunks: &ChunkStore,
        position: BlockPos,
    ) -> Option<f32> {
        self.environment().turtle_egg_hatch_chance(chunks, position)
    }

    pub(in crate::world) fn creaking_active(
        &self,
        chunks: &ChunkStore,
        position: BlockPos,
    ) -> Option<bool> {
        self.environment().creaking_active(chunks, position)
    }

    pub(in crate::world) fn eyeblossom_open(
        &self,
        chunks: &ChunkStore,
        position: BlockPos,
    ) -> Option<bbb_server_types::environment::TriState> {
        self.environment().eyeblossom_open(chunks, position)
    }

    pub(in crate::world) fn new(
        world: OriginalMobWorldInputs<'a>,
        dimension: &'a Java26LevelKey,
        simulation_levels: &'a SimulationChunkLevels,
        world_border: WorldBorder,
        weather: &super::super::weather::LevelWeather,
        items: crate::entity::ItemPublication,
    ) -> Self {
        Self {
            world,
            dimension,
            simulation_levels,
            world_border,
            thundering: weather.is_thundering(),
            items,
        }
    }

    pub(in crate::world) fn prepare(
        &self,
        chunks: &mut ChunkStore,
        centers: &[(i32, i32)],
    ) -> SaveResult<()> {
        chunks.try_ensure_served_full_columns_with_population(centers, &mut |chunks, center| {
            self.original_chunk_records(chunks, center).map_err(refusal)
        })
    }

    fn original_chunk_records(
        &self,
        chunks: &ChunkStore,
        center: (i32, i32),
    ) -> Result<Vec<Vec<NbtSummaryEntry>>, String> {
        let catalog = self.world.catalog.ok_or("missing creature catalog")?;
        let position = BlockPos {
            x: center.0.wrapping_mul(16),
            y: chunks.min_y() + chunks.section_count() as i32 * 16 - 1,
            z: center.1.wrapping_mul(16),
        };
        let biome = fuzzy_biome_resource_id(
            self.world.registries,
            chunks,
            biome_zoom_seed(self.world.seed),
            position,
        )
        .ok_or("missing highest-Y creature biome")?;
        let settings = catalog
            .biome(biome)
            .ok_or("missing biome creature settings")?;
        let random = chunk_random(self.world.seed, self.dimension.as_str(), center);
        let mut runtime = LazyRuntime {
            inputs: self,
            chunks,
            center,
            random: random.clone(),
            inner: None,
        };
        if self.world.spawn_mobs {
            runtime.spawn_mobs(
                settings.spawns(VanillaEntityMobCategory::Creature),
                settings.creature_spawn_probability,
                &random,
            )?;
        }
        let Some(runtime) = runtime.inner else {
            return Ok(Vec::new());
        };
        if runtime.store.bodies().any(|entity| {
            runtime.store.vehicle(entity.id()).is_none() && {
                let pos = entity.block_position();
                (pos.x >> 4, pos.z >> 4) != center
            }
        }) {
            return Err("generated entity escaped its population column".into());
        }
        let saved = crate::entity::save::entity_chunk_records(
            &runtime.store,
            center,
            &rustc_hash::FxHashSet::default(),
        )
        .map_err(|error| error.to_string())?;
        Ok(saved
            .into_iter()
            .map(bbb_server_persistence::Java26EntityRecord::into_raw)
            .collect())
    }
    fn original_runtime<'r>(
        &'r self,
        chunks: &'r ChunkStore,
        random: GameRandom,
    ) -> Result<OriginalRuntime<'r>, String> {
        let datapacks = self.world.datapacks;
        let dimension_type = self.world.dimension_type.ok_or("missing dimension type")?;
        let facts = match datapacks {
            Some(packs) => dimension_type_runtime_facts(&packs.registries, dimension_type),
            None => builtin_dimension_type_runtime_facts(dimension_type),
        }
        .map_err(|error| error.to_string())?;
        let spawn_environment = self.world.sky_lights.spawn_context(
            Some(dimension_type),
            self.world.registries,
            self.world.clocks,
        );
        let clock = spawn_environment
            .overworld_clock_time()
            .ok_or("missing overworld clock")?;
        let mut spawn_dimension =
            spawn_dimension_facts(datapacks, Some(dimension_type), self.thundering)
                .ok_or("missing spawn dimension facts")?;
        // WorldGenRegion.java:165-167.
        spawn_dimension.sky_darken = 0;
        let environment = NaturalEnvironment {
            dimension_id: self.world.dimension_id,
            spawn_environment,
            chunks,
            simulation_levels: self.simulation_levels,
            world_border: self.world_border,
            players: &[],
            alive_players: &[],
            generation_region: true,
            difficulty: self.world.difficulty,
            dimension_resource_id: self.dimension.as_str(),
            registries: self.world.registries,
            datapacks,
            world_seed: self.world.seed,
            biome_zoom_seed: biome_zoom_seed(self.world.seed),
            game_time: self.world.game_time,
            overworld_clock_time: clock,
            halloween: self.world.halloween,
            spawn_dimension: Some(spawn_dimension),
            items: self.items.clone(),
        };
        let mut store = EntityStore::with_random(random.clone(), EntityIdCounter::new());
        store.set_plugins(std::sync::Arc::clone(self.world.plugins));
        store.set_item_publication(self.items.clone());
        let runtime = OriginalRuntime {
            environment,
            has_ceiling: facts.shape.has_ceiling,
            store,
            random,
        };
        Ok(runtime)
    }
}

/// One stream per generated chunk, keyed by the world seed, the dimension and
/// the chunk, so regenerating a chunk repeats its creatures.
fn chunk_random(seed: i64, dimension: &str, center: (i32, i32)) -> GameRandom {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in dimension
        .bytes()
        .chain(seed.to_be_bytes())
        .chain(center.0.to_be_bytes())
        .chain(center.1.to_be_bytes())
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    GameRandom::with_seed(hash as i64)
}

fn spend(remaining: &mut usize) -> Result<(), String> {
    *remaining = remaining
        .checked_sub(1)
        .ok_or("chunk-generation creature work bound exceeded")?;
    Ok(())
}

fn air_at(chunks: &ChunkStore, pos: BlockPos) -> Result<bool, String> {
    Ok(bbb_server_types::fluid::block_state_is_air(
        block_state(chunks, pos).ok_or("missing ceiling block")?,
    ))
}

fn placement(
    type_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<crate::entity::SpawnPlacementEntry, String> {
    if vanilla_entity_type(type_id).is_none() && content_registry.entity_by_id(type_id).is_none() {
        return Err("missing entity type".into());
    }
    Ok(crate::entity::minecart_spawner::spawn_placement::spawn_placement_by_id(type_id))
}

/// The creature pass of one chunk; the runtime is only built once an
/// attempt reaches the level.
struct LazyRuntime<'a> {
    inputs: &'a OriginalMobInputs<'a>,
    chunks: &'a ChunkStore,
    center: (i32, i32),
    random: GameRandom,
    inner: Option<OriginalRuntime<'a>>,
}
impl<'a> LazyRuntime<'a> {
    fn runtime(&mut self) -> Result<&mut OriginalRuntime<'a>, String> {
        if self.inner.is_none() {
            self.inner = Some(
                self.inputs
                    .original_runtime(self.chunks, self.random.clone())?,
            );
        }
        Ok(self.inner.as_mut().unwrap())
    }

    /// `NaturalSpawner.spawnMobsForChunkGeneration`.
    fn spawn_mobs(
        &mut self,
        list: &MobSpawnList,
        probability: f32,
        random: &GameRandom,
    ) -> Result<usize, String> {
        if list.entries.is_empty() {
            return Ok(0);
        }
        if list.total_weight < 0
            || list.entries.iter().any(|entry| entry.weight < 0)
            || list
                .entries
                .iter()
                .try_fold(0_i32, |sum, entry| sum.checked_add(entry.weight))
                != Some(list.total_weight)
        {
            return Err("invalid chunk-generation spawn weights".into());
        }
        // An all-zero `WeightedList` is empty.
        if list.total_weight == 0 {
            return Ok(0);
        }
        let center = self.center;
        let (xo, zo) = (center.0.wrapping_mul(16), center.1.wrapping_mul(16));
        let mut remaining = MAX_ATTEMPTS;
        let mut spawned = 0;
        loop {
            spend(&mut remaining)?;
            if random.next_float() >= probability || probability.is_nan() {
                break;
            }
            let mut selection = random.next_int(list.total_weight);
            let selected = list
                .entries
                .iter()
                .find(|entry| {
                    selection = selection.wrapping_sub(entry.weight);
                    selection < 0
                })
                .ok_or("chunk-generation weighted selection outside list")?;
            let data = &selected.data;
            let type_id = data.entity_type_id;
            let bound = 1_i32
                .wrapping_add(data.max_count)
                .wrapping_sub(data.min_count);
            if bound <= 0 {
                return Err("invalid chunk-generation group count range".into());
            }
            let count = data.min_count.wrapping_add(random.next_int(bound));
            let mut group = None;
            let mut x = xo.wrapping_add(random.next_int(16));
            let mut z = zo.wrapping_add(random.next_int(16));
            let (start_x, start_z) = (x, z);
            for _ in 0..count {
                for _ in 0..4 {
                    spend(&mut remaining)?;
                    let runtime = self.runtime()?;
                    let pos = runtime.top_position(type_id, x, z)?;
                    let mut success = false;
                    if runtime.placement_allows(type_id, pos)? {
                        let width = f64::from(
                            vanilla_entity_type(type_id).map_or(0.0, |facts| facts.width),
                        );
                        let clamp = |value: i32, origin: i32| {
                            let minimum = f64::from(origin) + width;
                            let maximum = f64::from(origin) + 16.0 - width;
                            let value = f64::from(value);
                            if value < minimum {
                                minimum
                            } else if value.is_nan() || maximum.is_nan() {
                                f64::NAN
                            } else {
                                value.min(maximum)
                            }
                        };
                        let position = Vec3d {
                            x: clamp(x, xo),
                            y: f64::from(pos.y),
                            z: clamp(z, zo),
                        };
                        if !runtime.collision_and_rules_allow(type_id, position)? {
                            continue;
                        }
                        let Some(mob) = runtime.create_mob(type_id) else {
                            continue;
                        };
                        let yaw = random.next_float() * 360.0;
                        if let Some(next) = runtime.finish_mob(mob, position, yaw, group.clone())? {
                            group = next;
                            spawned += 1;
                            success = true;
                        }
                    }
                    spend(&mut remaining)?;
                    x = x.wrapping_add(random.next_int(5).wrapping_sub(random.next_int(5)));
                    z = z.wrapping_add(random.next_int(5).wrapping_sub(random.next_int(5)));
                    while x < xo || x >= xo.wrapping_add(16) || z < zo || z >= zo.wrapping_add(16) {
                        spend(&mut remaining)?;
                        x = start_x
                            .wrapping_add(random.next_int(5).wrapping_sub(random.next_int(5)));
                        z = start_z
                            .wrapping_add(random.next_int(5).wrapping_sub(random.next_int(5)));
                    }
                    if success {
                        break;
                    }
                }
            }
        }
        Ok(spawned)
    }
}

struct OriginalRuntime<'a> {
    environment: NaturalEnvironment<'a>,
    has_ceiling: bool,
    store: EntityStore,
    random: GameRandom,
}
impl OriginalRuntime<'_> {
    fn top_position(&mut self, type_id: i32, x: i32, z: i32) -> Result<BlockPos, String> {
        let chunks = self.environment.chunks;
        let content_registry = self.environment.registries.content_registry();
        let placement = placement(type_id, content_registry)?;
        let heightmap = match placement.heightmap {
            SpawnHeightmap::MotionBlocking => VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            SpawnHeightmap::MotionBlockingNoLeaves => {
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID
            }
        };
        let column = chunks
            .column(x >> 4, z >> 4)
            .ok_or("missing population heightmap column")?;
        let y = bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps::scan_first_available(
            column,
            heightmap,
            x & 15,
            z & 15,
        )
        .ok_or("missing population heightmap")?;
        let mut pos = BlockPos { x, y, z };
        if self.has_ceiling {
            let mut remaining = chunks.section_count() * 16 + 2;
            loop {
                pos.y = pos.y.wrapping_sub(1);
                remaining = remaining
                    .checked_sub(1)
                    .ok_or("ceiling scan exhausted build height")?;
                if air_at(chunks, pos)? {
                    break;
                }
            }
            loop {
                pos.y = pos.y.wrapping_sub(1);
                if !air_at(chunks, pos)? || pos.y <= chunks.min_y() {
                    break;
                }
            }
        }
        if placement.placement == SpawnPlacementType::OnGround {
            let below = below(pos);
            let state = block_state(chunks, below).ok_or("missing adjusted placement block")?;
            let pathfindable = if let Some(facts) = vanilla_block_state_path_facts(state) {
                facts.land_pathfindable()
            } else if let Some((spec, _)) = content_registry.block_by_state_id(state) {
                !spec.full_cube_collision
            } else {
                return Err("missing land path facts".into());
            };
            if pathfindable {
                pos = below;
            }
        }
        Ok(pos)
    }
    fn placement_allows(&mut self, type_id: i32, pos: BlockPos) -> Result<bool, String> {
        let content_registry = self.environment.registries.content_registry();
        let can_summon = vanilla_entity_type(type_id)
            .map(|facts| facts.can_summon)
            .or_else(|| {
                content_registry
                    .entity_by_id(type_id)
                    .map(|spec| spec.can_summon)
            })
            .ok_or("missing entity type")?;
        if !can_summon {
            return Ok(false);
        }
        let placement = placement(type_id, content_registry)?;
        let probe = SpawnPlacementProbe {
            entity_type_id: type_id,
            position: pos,
            entry: placement,
        };
        let environment =
            spawn_environment(&self.environment, probe).ok_or("missing placement blocks")?;
        Ok(is_spawn_position_ok(placement.placement, environment))
    }
    fn collision_and_rules_allow(&mut self, type_id: i32, pos: Vec3d) -> Result<bool, String> {
        let content_registry = self.environment.registries.content_registry();
        let (width, height) = vanilla_entity_type(type_id)
            .map(|facts| (facts.width, facts.height))
            .or_else(|| {
                content_registry
                    .entity_by_id(type_id)
                    .map(|spec| (spec.width, spec.height))
            })
            .ok_or("missing entity dimensions")?;
        let scale =
            crate::entity::minecart_spawner::spawn_placement::spawn_dimensions_scale(type_id);
        let bounds = Aabb::of_entity(pos, width * scale, height * scale);
        if !no_block_collisions(self.environment.chunks, bounds)
            .ok_or("missing collision blocks")?
        {
            return Ok(false);
        }
        let environment = self.environment.predicate_level();
        let probe = SpawnPlacementProbe {
            entity_type_id: type_id,
            position: BlockPos {
                x: pos.x.floor() as i32,
                y: pos.y.floor() as i32,
                z: pos.z.floor() as i32,
            },
            entry: placement(type_id, content_registry)?,
        };
        let allowed = check_spawn_placement_predicate(
            &environment,
            probe,
            SpawnPredicateReason::ChunkGeneration,
            &self.random,
        );
        if environment.missing() {
            return Err("missing chunk-generation predicate context".into());
        }
        Ok(allowed)
    }
    fn create_mob(&mut self, type_id: i32) -> Option<Box<Entity>> {
        create_natural_spawn_root(&mut self.store, type_id, Vec3d::default())
    }
    /// `None` when the mob is rejected, otherwise the group data the next
    /// member of the group inherits.
    fn finish_mob(
        &mut self,
        mut mob: Box<Entity>,
        pos: Vec3d,
        yaw: f32,
        group: Option<SpawnGroupData>,
    ) -> Result<Option<Option<SpawnGroupData>>, String> {
        mob.set_pos(pos);
        mob.set_rot(yaw, 0.0);
        let facts = natural_spawn_pre_finalize_mob_facts(&mob, 0.0)
            .ok_or("missing pre-finalize mob facts")?;
        let rules = super::check_environment_natural_spawn_rules(
            &self.environment,
            mob.block_position(),
            mob.entity_type_id(),
            facts,
        )
        .ok_or("missing mob rule context")?
        .map_err(|error| format!("mob rules: {error:?}"))?;
        if !rules {
            self.store.discard_fresh(mob);
            return Ok(None);
        }
        // WorldGenRegion.java:429-435 has no entity collisions, including earlier generated mobs.
        let unobstructed =
            super::check_environment_natural_spawn_obstruction(&self.environment, None, &mob)
                .map_err(|error| format!("mob obstruction: {error:?}"))?
                .ok_or("missing mob collision context")?;
        if !unobstructed {
            self.store.discard_fresh(mob);
            return Ok(None);
        }
        let result = finalize_and_publish_natural_spawn(
            &mut NaturalSpawnLevel {
                environment: &self.environment,
                store: &mut self.store,
            },
            mob,
            SpawnReason::ChunkGeneration,
            group,
            &self.random,
        )
        .ok_or("original mob finalization failed")?;
        Ok(Some(result.group_data))
    }
}

mod structure_mobs;
#[cfg(test)]
mod tests;
