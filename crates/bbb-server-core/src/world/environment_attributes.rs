use crate::world::LevelId;
use bbb_facts::environment_attribute_types::{
    VanillaEnvironmentAttribute, VanillaEnvironmentAttributeType, VanillaEnvironmentScalarValue,
    VANILLA_ENVIRONMENT_ATTRIBUTES,
};
use std::collections::BTreeMap;
use std::sync::OnceLock;

use bbb_protocol::nbt_summary::value_field as field;
use bbb_protocol::NbtSummaryValue as Nbt;

use bbb_server_datapack::dimension_type::DIMENSION_TYPE_REGISTRY;
use bbb_server_datapack::environment_attributes::color::DimensionColorEnvironment;
use bbb_server_datapack::environment_attributes::discrete::{
    DimensionAmbientParticlesEnvironment, DimensionAmbientSoundsEnvironment,
    DimensionBackgroundMusicEnvironment, DimensionParticleEnvironment,
};
use bbb_server_datapack::environment_attributes::{
    DimensionBedEnvironment, DimensionBooleanEnvironment, DimensionEyeblossomEnvironment,
    DimensionFloatAttribute, DimensionMoonEnvironment, DimensionVillagerActivityEnvironment,
    EnvironmentAttributeSources,
};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::environment::BedRule;
use bbb_server_types::{Result, ServerWorldError};

use super::ServerWorld;

#[derive(Debug, Clone)]
pub(super) struct DimensionSkyLights {
    scalars: BTreeMap<String, DimensionScalarAttributes>,
    colors: BTreeMap<String, DimensionColorAttributes>,
    bed: BTreeMap<String, DimensionBedEnvironment>,
    background_music: BTreeMap<String, DimensionBackgroundMusicEnvironment>,
    ambient_sounds: BTreeMap<String, DimensionAmbientSoundsEnvironment>,
    particle: BTreeMap<String, DimensionParticleEnvironment>,
    ambient_particles: BTreeMap<String, DimensionAmbientParticlesEnvironment>,
    moon: BTreeMap<String, DimensionMoonEnvironment>,
    eyeblossom: BTreeMap<String, DimensionEyeblossomEnvironment>,
    villager_activity: BTreeMap<String, DimensionVillagerActivityEnvironment>,
    dimensions: BTreeMap<String, DimensionClock>,
    markers: BTreeMap<String, BTreeMap<String, TimeMarker>>,
}

/// A boolean environment attribute with every layer but the biome one already applied.
#[derive(Debug, Clone)]
pub(in crate::world) enum BiomeBoolean {
    Uniform(bool),
    /// Indexed by biome registry id.
    ByBiome(Vec<bool>),
}

impl BiomeBoolean {
    pub(in crate::world) fn sample(&self, biome_id: impl FnOnce() -> Option<i32>) -> Option<bool> {
        match self {
            Self::Uniform(value) => Some(*value),
            Self::ByBiome(values) => values.get(usize::try_from(biome_id()?).ok()?).copied(),
        }
    }
}

type DimensionScalarAttributes = Vec<Option<CompiledScalarAttribute>>;
type DimensionColorAttributes = Vec<Option<DimensionColorEnvironment>>;

const fn str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn env_attr_index(resource_id: &str) -> Option<usize> {
    let mut i = 0;
    while i < VANILLA_ENVIRONMENT_ATTRIBUTES.len() {
        if str_eq(VANILLA_ENVIRONMENT_ATTRIBUTES[i].resource_id, resource_id) {
            return Some(i);
        }
        i += 1;
    }
    None
}

const fn require_env_attr_index(resource_id: &str) -> usize {
    match env_attr_index(resource_id) {
        Some(index) => index,
        None => panic!("unknown vanilla environment attribute"),
    }
}

const ATTR_BEES_STAY_IN_HIVE: usize =
    require_env_attr_index("minecraft:gameplay/bees_stay_in_hive");
const ATTR_SKY_LIGHT_LEVEL: usize = require_env_attr_index("minecraft:gameplay/sky_light_level");
const ATTR_SKY_LIGHT_FACTOR: usize = require_env_attr_index("minecraft:visual/sky_light_factor");
const ATTR_STAR_BRIGHTNESS: usize = require_env_attr_index("minecraft:visual/star_brightness");
const ATTR_SKY_COLOR: usize = require_env_attr_index("minecraft:visual/sky_color");
const ATTR_CLOUD_COLOR: usize = require_env_attr_index("minecraft:visual/cloud_color");
const ATTR_FOG_COLOR: usize = require_env_attr_index("minecraft:visual/fog_color");
const ATTR_SUNRISE_SUNSET_COLOR: usize =
    require_env_attr_index("minecraft:visual/sunrise_sunset_color");
const ATTR_SKY_LIGHT_COLOR: usize = require_env_attr_index("minecraft:visual/sky_light_color");
const ATTR_SURFACE_SLIME_SPAWN_CHANCE: usize =
    require_env_attr_index("minecraft:gameplay/surface_slime_spawn_chance");
const ATTR_CAT_WAKING_UP_GIFT_CHANCE: usize =
    require_env_attr_index("minecraft:gameplay/cat_waking_up_gift_chance");
const ATTR_SNOW_GOLEM_MELTS: usize = require_env_attr_index("minecraft:gameplay/snow_golem_melts");
const ATTR_TURTLE_EGG_HATCH_CHANCE: usize =
    require_env_attr_index("minecraft:gameplay/turtle_egg_hatch_chance");
const ATTR_CREAKING_ACTIVE: usize = require_env_attr_index("minecraft:gameplay/creaking_active");
const ATTR_CAN_PILLAGER_PATROL_SPAWN: usize =
    require_env_attr_index("minecraft:gameplay/can_pillager_patrol_spawn");
const ATTR_RESPAWN_ANCHOR_WORKS: usize =
    require_env_attr_index("minecraft:gameplay/respawn_anchor_works");
const ATTR_FAST_LAVA: usize = require_env_attr_index("minecraft:gameplay/fast_lava");
const ATTR_MONSTERS_BURN: usize = require_env_attr_index("minecraft:gameplay/monsters_burn");
const ATTR_PIGLINS_ZOMBIFY: usize = require_env_attr_index("minecraft:gameplay/piglins_zombify");
const ATTR_WATER_EVAPORATES: usize = require_env_attr_index("minecraft:gameplay/water_evaporates");
const ATTR_CAN_START_RAID: usize = require_env_attr_index("minecraft:gameplay/can_start_raid");
pub(in crate::world) const ATTR_SUN_ANGLE: usize =
    require_env_attr_index("minecraft:visual/sun_angle");
pub(in crate::world) const ATTR_INCREASED_FIRE_BURNOUT: usize =
    require_env_attr_index("minecraft:gameplay/increased_fire_burnout");
pub(in crate::world) const ATTR_NETHER_PORTAL_SPAWNS_PIGLIN: usize =
    require_env_attr_index("minecraft:gameplay/nether_portal_spawns_piglin");

#[derive(Debug, Clone)]
enum CompiledScalarAttribute {
    Boolean(DimensionBooleanEnvironment),
    Number(DimensionFloatAttribute),
}

impl CompiledScalarAttribute {
    fn varies_by_biome(&self) -> bool {
        match self {
            Self::Boolean(value) => value.varies_by_biome(),
            Self::Number(value) => value.varies_by_biome(),
        }
    }

    fn sample(
        &self,
        attribute_index: usize,
        biome: Option<&str>,
        clock: impl Fn(&str) -> Option<i64>,
        weather: ScalarWeather,
    ) -> Option<VanillaEnvironmentScalarValue> {
        use VanillaEnvironmentScalarValue as V;
        match self {
            Self::Boolean(environment) => {
                let value = match biome {
                    Some(biome) => environment.sample(biome, &clock)?,
                    None => environment.sample_dimension(&clock)?,
                };
                Some(weather.apply(attribute_index, V::Boolean(value)))
            }
            Self::Number(environment) => {
                let value = environment.sample_unclamped(biome, clock)?;
                let V::FloatBits(bits) =
                    weather.apply(attribute_index, V::FloatBits(value.to_bits()))
                else {
                    return None;
                };
                Some(V::FloatBits(
                    environment.sanitize(f32::from_bits(bits)).to_bits(),
                ))
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(in crate::world) struct ScalarWeather {
    rain: f32,
    thunder: f32,
}

impl ScalarWeather {
    pub(in crate::world) fn from_level(weather: &super::weather::LevelWeather) -> Self {
        if weather.can_have_weather() {
            Self {
                rain: weather.rain_level(),
                thunder: weather.thunder_level(),
            }
        } else {
            Self::default()
        }
    }

    // WeatherAttributes.java:12-63; thunder_level already includes rain.
    fn apply(
        self,
        attribute_index: usize,
        mut value: VanillaEnvironmentScalarValue,
    ) -> VanillaEnvironmentScalarValue {
        use VanillaEnvironmentScalarValue as V;
        for (level, alpha) in [
            (self.rain - self.thunder, 0.3125),
            (self.thunder, 0.52734375),
        ] {
            if level <= 0.0 {
                continue;
            }
            value = match (attribute_index, value) {
                (ATTR_BEES_STAY_IN_HIVE, V::Boolean(_)) => V::Boolean(true),
                (ATTR_SKY_LIGHT_LEVEL, V::FloatBits(bits)) => {
                    let from = f32::from_bits(bits);
                    let modified = from + alpha * (4.0 - from);
                    V::FloatBits((from + level * (modified - from)).to_bits())
                }
                (ATTR_SKY_LIGHT_FACTOR, V::FloatBits(bits)) => {
                    let from = f32::from_bits(bits);
                    let modified = from + alpha * (0.24 - from);
                    V::FloatBits((from + level * (modified - from)).to_bits())
                }
                (ATTR_STAR_BRIGHTNESS, V::FloatBits(bits)) => {
                    let from = f32::from_bits(bits);
                    V::FloatBits((from + level * (0.0 - from)).to_bits())
                }
                (_, value) => value,
            };
        }
        value
    }

    // WeatherAttributes.java:13-32,46-63; ARGB.java:225-227; Timelines.java:30.
    fn apply_color(self, attribute_index: usize, mut value: u32) -> u32 {
        use bbb_facts::color::{alpha_blend, blend_to_gray, multiply, srgb_lerp};
        for (level, thunder) in [(self.rain - self.thunder, false), (self.thunder, true)] {
            if level <= 0.0 {
                continue;
            }
            let modified = match attribute_index {
                ATTR_SKY_COLOR => {
                    let (brightness, factor) = if thunder { (0.24, 0.94) } else { (0.6, 0.75) };
                    blend_to_gray(value, brightness, factor)
                }
                ATTR_CLOUD_COLOR => {
                    let (brightness, factor) = if thunder { (0.095, 0.94) } else { (0.24, 0.5) };
                    blend_to_gray(value, brightness, factor)
                }
                ATTR_FOG_COLOR | ATTR_SUNRISE_SUNSET_COLOR => {
                    multiply(value, if thunder { 0xff3f_3f4c } else { 0xff7f_7f99 })
                }
                ATTR_SKY_LIGHT_COLOR => {
                    alpha_blend(value, if thunder { 0x867a_7aff } else { 0x4f7a_7aff })
                }
                _ => return value,
            };
            value = srgb_lerp(level, value, modified);
        }
        value
    }
}

fn boolean(value: VanillaEnvironmentScalarValue) -> Option<bool> {
    match value {
        VanillaEnvironmentScalarValue::Boolean(value) => Some(value),
        VanillaEnvironmentScalarValue::FloatBits(_) => None,
    }
}

fn number(value: VanillaEnvironmentScalarValue) -> Option<f32> {
    match value {
        VanillaEnvironmentScalarValue::FloatBits(bits) => Some(f32::from_bits(bits)),
        VanillaEnvironmentScalarValue::Boolean(_) => None,
    }
}

#[derive(Clone, Copy)]
pub(in crate::world) struct LootEnvironmentContext<'a> {
    bed: Option<&'a DimensionBedEnvironment>,
    background_music: Option<&'a DimensionBackgroundMusicEnvironment>,
    ambient_sounds: Option<&'a DimensionAmbientSoundsEnvironment>,
    particle: Option<&'a DimensionParticleEnvironment>,
    ambient_particles: Option<&'a DimensionAmbientParticlesEnvironment>,
    scalars: Option<&'a DimensionScalarAttributes>,
    colors: Option<&'a DimensionColorAttributes>,
    villager_activity: Option<&'a DimensionVillagerActivityEnvironment>,
    spawn: SpawnEnvironmentContext<'a>,
    weather: ScalarWeather,
}

impl<'a> LootEnvironmentContext<'a> {
    pub(in crate::world) fn block_scalar(
        &self,
        attribute_index: usize,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<VanillaEnvironmentScalarValue> {
        let environment = self.scalars?.get(attribute_index)?.as_ref()?;
        let biome = if environment.varies_by_biome() {
            self.spawn.biome_id_at(chunks, position)
        } else {
            ""
        };
        environment.sample(
            attribute_index,
            Some(biome),
            |clock| self.spawn.clock_ticks(clock),
            self.weather,
        )
    }

    pub(in crate::world) fn sample(
        &self,
        attribute: &'static VanillaEnvironmentAttribute,
        chunks: &crate::chunk::ChunkStore,
        origin: Option<crate::loot_runtime::LootOrigin>,
    ) -> Option<crate::loot_runtime::LootEnvironmentAttributeValue<'a>> {
        use crate::loot_runtime::LootEnvironmentAttributeValue;
        let attribute_index = env_attr_index(attribute.resource_id)?;
        if matches!(
            attribute.value_type,
            VanillaEnvironmentAttributeType::RgbColor | VanillaEnvironmentAttributeType::ArgbColor
        ) {
            let position =
                origin?.offset_containing(bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 });
            let environment = self.colors?.get(attribute_index)?.as_ref()?;
            let biome = environment
                .varies_by_biome()
                .then(|| self.spawn.biome_id_at(chunks, position));
            let value = environment.sample(biome, |clock| self.spawn.clock_ticks(clock))?;
            return Some(LootEnvironmentAttributeValue::Color(
                self.weather.apply_color(attribute_index, value),
            ));
        }
        if matches!(
            attribute.value_type,
            VanillaEnvironmentAttributeType::BedRule
                | VanillaEnvironmentAttributeType::BackgroundMusic
                | VanillaEnvironmentAttributeType::AmbientSounds
                | VanillaEnvironmentAttributeType::Particle
                | VanillaEnvironmentAttributeType::AmbientParticles
        ) {
            let position =
                origin?.offset_containing(bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 });
            let biome = self.spawn.biome_id_at(chunks, position);
            return match attribute.value_type {
                VanillaEnvironmentAttributeType::BedRule => self
                    .bed?
                    .sample_ref(biome, |clock| self.spawn.clock_ticks(clock))
                    .map(LootEnvironmentAttributeValue::BedRule),
                VanillaEnvironmentAttributeType::BackgroundMusic => self
                    .background_music?
                    .sample(biome, |clock| self.spawn.clock_ticks(clock))
                    .map(LootEnvironmentAttributeValue::BackgroundMusic),
                VanillaEnvironmentAttributeType::AmbientSounds => self
                    .ambient_sounds?
                    .sample(biome, |clock| self.spawn.clock_ticks(clock))
                    .map(LootEnvironmentAttributeValue::AmbientSounds),
                VanillaEnvironmentAttributeType::Particle => self
                    .particle?
                    .sample(biome, &|clock| self.spawn.clock_ticks(clock))
                    .map(LootEnvironmentAttributeValue::Particle),
                VanillaEnvironmentAttributeType::AmbientParticles => self
                    .ambient_particles?
                    .sample(biome, &|clock| self.spawn.clock_ticks(clock))
                    .map(LootEnvironmentAttributeValue::AmbientParticles),
                _ => unreachable!(),
            };
        }
        if matches!(
            attribute.value_type,
            VanillaEnvironmentAttributeType::TriState
        ) {
            let position =
                origin?.offset_containing(bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 });
            return self
                .spawn
                .eyeblossom_open(chunks, position)
                .map(LootEnvironmentAttributeValue::TriState);
        }
        if matches!(
            attribute.value_type,
            VanillaEnvironmentAttributeType::MoonPhase
        ) {
            let position =
                origin?.offset_containing(bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 });
            return self
                .spawn
                .environment?
                .sample_phase(self.spawn.biome_id_at(chunks, position), |clock| {
                    self.spawn.clock_ticks(clock)
                })
                .map(LootEnvironmentAttributeValue::MoonPhase);
        }
        if matches!(
            attribute.value_type,
            VanillaEnvironmentAttributeType::Activity
        ) {
            let position =
                origin?.offset_containing(bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 });
            return self
                .villager_activity?
                .sample_attribute(
                    attribute,
                    self.spawn.biome_id_at(chunks, position),
                    |clock| self.spawn.clock_ticks(clock),
                )
                .map(LootEnvironmentAttributeValue::Activity);
        }
        if attribute.positional {
            let position =
                origin?.offset_containing(bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 });
            return self
                .block_scalar(attribute_index, chunks, position)
                .map(LootEnvironmentAttributeValue::Scalar);
        }
        self.scalars?
            .get(attribute_index)?
            .as_ref()?
            .sample(
                attribute_index,
                None,
                |clock| self.spawn.clock_ticks(clock),
                self.weather,
            )
            .map(LootEnvironmentAttributeValue::Scalar)
    }
}

#[derive(Debug, Clone)]
struct DimensionClock {
    default_clock: Option<String>,
    has_fixed_time: bool,
}

#[derive(Debug, Clone)]
struct TimeMarker {
    ticks: i32,
    period: Option<i32>,
}

#[derive(Clone, Copy)]
pub(super) struct SpawnEnvironmentContext<'a> {
    environment: Option<&'a DimensionMoonEnvironment>,
    eyeblossom: Option<&'a DimensionEyeblossomEnvironment>,
    scalars: Option<&'a DimensionScalarAttributes>,
    registries: &'a crate::registry_sync::SynchronizedRegistries,
    clocks: &'a super::world_clock::ServerWorldClocks,
}

impl SpawnEnvironmentContext<'_> {
    fn clock_ticks(&self, clock: &str) -> Option<i64> {
        let index = self
            .registries
            .registry_element_index("minecraft:world_clock", clock)?;
        self.clocks.total_ticks(usize::try_from(index).ok()?)
    }

    pub(super) fn overworld_clock_time(&self) -> Option<i64> {
        self.clock_ticks("minecraft:overworld")
    }

    fn biome_id_at(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> &str {
        chunks
            .biome_id_at_quart_or_source(position.x >> 2, position.y >> 2, position.z >> 2)
            .and_then(|id| {
                self.registries
                    .registry_element_id("minecraft:worldgen/biome", id)
            })
            .unwrap_or("")
    }

    pub(super) fn moon_brightness(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<f32> {
        self.environment?
            .sample(self.biome_id_at(chunks, position), |clock| {
                self.clock_ticks(clock)
            })
    }

    pub(super) fn eyeblossom_open(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<bbb_server_types::environment::TriState> {
        self.eyeblossom?
            .sample(self.biome_id_at(chunks, position), |clock| {
                self.clock_ticks(clock)
            })
    }

    pub(super) fn surface_slime_spawn_chance(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<f32> {
        number(self.sample_scalar(ATTR_SURFACE_SLIME_SPAWN_CHANCE, chunks, position)?)
    }

    pub(super) fn cat_waking_up_gift_chance(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<f32> {
        number(self.sample_scalar(ATTR_CAT_WAKING_UP_GIFT_CHANCE, chunks, position)?)
    }

    pub(super) fn snow_golem_melts(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<bool> {
        boolean(self.sample_scalar(ATTR_SNOW_GOLEM_MELTS, chunks, position)?)
    }

    pub(super) fn turtle_egg_hatch_chance(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<f32> {
        number(self.sample_scalar(ATTR_TURTLE_EGG_HATCH_CHANCE, chunks, position)?)
    }

    pub(super) fn creaking_active(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<bool> {
        boolean(self.sample_scalar(ATTR_CREAKING_ACTIVE, chunks, position)?)
    }

    fn sample_scalar(
        &self,
        attribute_index: usize,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<VanillaEnvironmentScalarValue> {
        self.scalars?.get(attribute_index)?.as_ref()?.sample(
            attribute_index,
            Some(self.biome_id_at(chunks, position)),
            |clock| self.clock_ticks(clock),
            ScalarWeather::default(),
        )
    }

    fn difficulty_inputs(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<(i64, f32)> {
        if chunks.is_ticking_full(position.x >> 4, position.z >> 4) {
            Some((
                chunks.inhabited_time(position.x >> 4, position.z >> 4)?,
                self.moon_brightness(chunks, position)?,
            ))
        } else {
            Some((0, 0.0))
        }
    }

    pub(super) fn effective_difficulty(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
        difficulty: bbb_protocol::difficulties::Difficulty,
        overworld_clock: i64,
    ) -> Option<f32> {
        let (local_time, moon) = self.difficulty_inputs(chunks, position)?;
        Some(crate::entity::effective_difficulty(
            difficulty,
            overworld_clock,
            local_time,
            moon,
        ))
    }

    pub(super) fn special_multiplier(
        &self,
        chunks: &crate::chunk::ChunkStore,
        position: bbb_protocol::packets::BlockPos,
        difficulty: bbb_protocol::difficulties::Difficulty,
        overworld_clock: i64,
    ) -> Option<f32> {
        let (local_time, moon) = self.difficulty_inputs(chunks, position)?;
        Some(crate::entity::difficulty_special_multiplier(
            difficulty,
            overworld_clock,
            local_time,
            moon,
        ))
    }
}

impl DimensionSkyLights {
    // Level.java:380-381.
    pub(in crate::world) fn is_bright_outside(
        &self,
        dimension_type: &str,
        sky_darkening: Option<i32>,
    ) -> Option<bool> {
        if self.dimensions.get(dimension_type)?.has_fixed_time {
            return Some(false);
        }
        Some(sky_darkening? < 4)
    }

    // Level.java:380-385.
    pub(in crate::world) fn is_dark_outside(
        &self,
        dimension_type: &str,
        sky_darkening: i32,
    ) -> Option<bool> {
        Some(!self.dimensions.get(dimension_type)?.has_fixed_time && sky_darkening >= 4)
    }

    pub(in crate::world) fn loot_environment_context<'a>(
        &'a self,
        dimension_type: Option<&str>,
        registries: &'a crate::registry_sync::SynchronizedRegistries,
        clocks: &'a super::world_clock::ServerWorldClocks,
        weather: ScalarWeather,
    ) -> LootEnvironmentContext<'a> {
        LootEnvironmentContext {
            bed: dimension_type.and_then(|id| self.bed.get(id)),
            background_music: dimension_type.and_then(|id| self.background_music.get(id)),
            ambient_sounds: dimension_type.and_then(|id| self.ambient_sounds.get(id)),
            particle: dimension_type.and_then(|id| self.particle.get(id)),
            ambient_particles: dimension_type.and_then(|id| self.ambient_particles.get(id)),
            scalars: dimension_type.and_then(|id| self.scalars.get(id)),
            colors: dimension_type.and_then(|id| self.colors.get(id)),
            villager_activity: dimension_type.and_then(|id| self.villager_activity.get(id)),
            spawn: self.spawn_context(dimension_type, registries, clocks),
            weather,
        }
    }

    pub(super) fn spawn_context<'a>(
        &'a self,
        dimension_type: Option<&str>,
        registries: &'a crate::registry_sync::SynchronizedRegistries,
        clocks: &'a super::world_clock::ServerWorldClocks,
    ) -> SpawnEnvironmentContext<'a> {
        SpawnEnvironmentContext {
            environment: dimension_type.and_then(|id| self.moon.get(id)),
            eyeblossom: dimension_type.and_then(|id| self.eyeblossom.get(id)),
            scalars: dimension_type.and_then(|id| self.scalars.get(id)),
            registries,
            clocks,
        }
    }

    pub(super) fn sleep_clock_marker(
        &self,
        dimension_type: &str,
    ) -> Option<(&str, i32, Option<i32>)> {
        let clock = self
            .dimensions
            .get(dimension_type)?
            .default_clock
            .as_deref()?;
        let marker = self
            .markers
            .get(clock)?
            .get("minecraft:wake_up_from_sleep")?;
        Some((clock, marker.ticks, marker.period))
    }

    pub(super) fn time_marker_matches(
        &self,
        dimension_type: &str,
        marker: &str,
        total_ticks: impl FnOnce(&str) -> Option<i64>,
    ) -> Option<bool> {
        let dimension = self.dimensions.get(dimension_type)?;
        let Some(clock) = dimension.default_clock.as_deref() else {
            return Some(false);
        };
        let Some(marker) = self
            .markers
            .get(clock)
            .and_then(|markers| markers.get(marker))
        else {
            return Some(false);
        };
        let Some(ticks) = total_ticks(clock) else {
            return Some(false);
        };
        Some(
            i64::from(marker.ticks)
                == marker
                    .period
                    .map_or(ticks, |period| ticks % i64::from(period)),
        )
    }
}

fn configuration_error(detail: impl Into<String>) -> ServerWorldError {
    ServerWorldError::InvalidConfiguration(detail.into())
}

pub(super) fn load_sky_lights(startup: Option<&DatapackStartup>) -> Result<DimensionSkyLights> {
    let Some(startup) = startup else {
        static BUILTIN: OnceLock<std::result::Result<DimensionSkyLights, String>> = OnceLock::new();
        return BUILTIN.get_or_init(|| {
            let startup = DatapackStartup::load(&bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
            }).map_err(|error| error.to_string())?;
            load_sky_lights(Some(&startup)).map_err(|error| error.to_string())
        }).clone().map_err(configuration_error);
    };
    let registry = startup
        .registries
        .registry(DIMENSION_TYPE_REGISTRY)
        .ok_or_else(|| configuration_error("missing dimension types for sky light"))?;
    let sources = EnvironmentAttributeSources::new(startup)
        .map_err(|error| configuration_error(error.to_string()))?;
    let mut result = DimensionSkyLights {
        scalars: BTreeMap::new(),
        colors: BTreeMap::new(),
        bed: BTreeMap::new(),
        background_music: BTreeMap::new(),
        ambient_sounds: BTreeMap::new(),
        particle: BTreeMap::new(),
        ambient_particles: BTreeMap::new(),
        moon: BTreeMap::new(),
        eyeblossom: BTreeMap::new(),
        villager_activity: BTreeMap::new(),
        dimensions: BTreeMap::new(),
        markers: BTreeMap::new(),
    };
    for element in &registry.elements {
        let mut scalars = vec![None; VANILLA_ENVIRONMENT_ATTRIBUTES.len()];
        for (index, attribute) in VANILLA_ENVIRONMENT_ATTRIBUTES.iter().enumerate() {
            let Some(default) = attribute.scalar_default else {
                continue;
            };
            let value = match default {
                VanillaEnvironmentScalarValue::Boolean(_) => {
                    DimensionBooleanEnvironment::from_attribute(&sources, &element.id, attribute)
                        .map(CompiledScalarAttribute::Boolean)
                }
                VanillaEnvironmentScalarValue::FloatBits(_) => {
                    DimensionFloatAttribute::from_attribute(&sources, &element.id, attribute)
                        .map(CompiledScalarAttribute::Number)
                }
            }
            .map_err(|error| configuration_error(error.to_string()))?;
            scalars[index] = Some(value);
        }
        result.scalars.insert(element.id.clone(), scalars);
        let mut colors = vec![None; VANILLA_ENVIRONMENT_ATTRIBUTES.len()];
        for (index, attribute) in VANILLA_ENVIRONMENT_ATTRIBUTES.iter().enumerate() {
            if attribute.color_default.is_none() {
                continue;
            }
            let value = DimensionColorEnvironment::from_attribute(&sources, &element.id, attribute)
                .map_err(|error| configuration_error(error.to_string()))?;
            colors[index] = Some(value);
        }
        result.colors.insert(element.id.clone(), colors);
        result.bed.insert(
            element.id.clone(),
            DimensionBedEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        result.background_music.insert(
            element.id.clone(),
            DimensionBackgroundMusicEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        result.ambient_sounds.insert(
            element.id.clone(),
            DimensionAmbientSoundsEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        result.particle.insert(
            element.id.clone(),
            DimensionParticleEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        result.ambient_particles.insert(
            element.id.clone(),
            DimensionAmbientParticlesEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        result.moon.insert(
            element.id.clone(),
            DimensionMoonEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        result.villager_activity.insert(
            element.id.clone(),
            DimensionVillagerActivityEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        result.eyeblossom.insert(
            element.id.clone(),
            DimensionEyeblossomEnvironment::from_sources(&sources, &element.id)
                .map_err(|error| configuration_error(error.to_string()))?,
        );
        let dimension = sources
            .dimension(&element.id)
            .map_err(|error| configuration_error(error.to_string()))?;
        let default_clock =
            match field(dimension, "default_clock") {
                Some(Nbt::String(clock)) => Some(clock.try_to_string().map_err(|_| {
                    configuration_error("invalid canonical dimension default_clock")
                })?),
                None => None,
                _ => {
                    return Err(configuration_error(
                        "invalid canonical dimension default_clock",
                    ))
                }
            };
        let has_fixed_time = match field(dimension, "has_fixed_time") {
            Some(Nbt::Byte(value)) => *value != 0,
            None => false,
            _ => {
                return Err(configuration_error(
                    "invalid canonical dimension has_fixed_time",
                ))
            }
        };
        result.dimensions.insert(
            element.id.clone(),
            DimensionClock {
                default_clock,
                has_fixed_time,
            },
        );
    }
    let timelines = startup
        .registries
        .registry("minecraft:timeline")
        .ok_or_else(|| configuration_error("missing timeline registry"))?;
    for element in &timelines.elements {
        let timeline = &element
            .codec_value
            .as_ref()
            .ok_or_else(|| {
                configuration_error(format!("missing canonical timeline {}", element.id))
            })?
            .network_nbt;
        let Some(Nbt::String(clock)) = field(timeline, "clock") else {
            return Err(configuration_error(format!(
                "invalid canonical timeline clock {}",
                element.id
            )));
        };
        let period = match field(timeline, "period_ticks") {
            Some(Nbt::Int(period)) if *period > 0 => Some(*period),
            None => None,
            _ => {
                return Err(configuration_error(format!(
                    "invalid canonical timeline period {}",
                    element.id
                )))
            }
        };
        let entries = match field(timeline, "time_markers") {
            Some(Nbt::Compound(entries)) => entries,
            None => continue,
            _ => {
                return Err(configuration_error(format!(
                    "invalid canonical timeline markers {}",
                    element.id
                )))
            }
        };
        for entry in entries {
            let ticks = match &entry.value {
                Nbt::Int(ticks) => *ticks,
                value => match field(value, "ticks") {
                    Some(Nbt::Int(ticks)) => *ticks,
                    _ => {
                        return Err(configuration_error(format!(
                            "invalid canonical time marker {}",
                            entry.name.to_display_string()
                        )))
                    }
                },
            };
            if result
                .markers
                .entry(
                    clock
                        .try_to_string()
                        .map_err(|_| configuration_error("invalid canonical timeline clock"))?,
                )
                .or_default()
                .insert(
                    entry
                        .name
                        .try_to_string()
                        .map_err(|_| configuration_error("invalid canonical time marker id"))?,
                    TimeMarker { ticks, period },
                )
                .is_some()
            {
                return Err(configuration_error(format!(
                    "time marker {} was defined multiple times in {clock}",
                    entry.name.to_display_string()
                )));
            }
        }
    }
    Ok(result)
}

impl ServerWorld {
    pub(super) fn effective_difficulty_at(
        &self,
        dimension: &LevelId,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<f32> {
        let moon = self.cold.dimension_sky_lights.spawn_context(
            self.cold
                .level_dimension_types
                .get(dimension)
                .map(String::as_str),
            &self.registries,
            &self.world_clocks,
        );
        moon.effective_difficulty(
            &self.levels.get(dimension)?.chunks,
            position,
            self.admin.settings().difficulty,
            moon.overworld_clock_time()?,
        )
    }

    pub(super) fn can_pillager_patrol_spawn(
        &mut self,
        dimension: &LevelId,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Option<bool> {
        boolean(self.sample_scalar_at(dimension, ATTR_CAN_PILLAGER_PATROL_SPAWN, Some(pos))?)
    }

    pub(super) fn bees_stay_in_hive(
        &self,
        dimension: &LevelId,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Option<bool> {
        self.bees_stay_in_hive_values(dimension)?.sample(|| {
            self.levels
                .get(dimension)?
                .chunks
                .biome_id_at_quart_or_source(pos.x >> 2, pos.y >> 2, pos.z >> 2)
        })
    }

    pub(in crate::world) fn bees_stay_in_hive_values(
        &self,
        dimension: &LevelId,
    ) -> Option<BiomeBoolean> {
        self.biome_boolean(dimension, ATTR_BEES_STAY_IN_HIVE)
    }

    pub(super) fn villager_schedule_activity(
        &self,
        dimension: &LevelId,
        pos: bbb_protocol::packets::BlockPos,
        baby: bool,
    ) -> Option<&str> {
        let biome = self
            .levels
            .get(dimension)?
            .chunks
            .biome_id_at_quart_or_source(pos.x >> 2, pos.y >> 2, pos.z >> 2)
            .and_then(|id| {
                self.registries
                    .registry_element_id("minecraft:worldgen/biome", id)
            })
            .unwrap_or("");
        self.villager_schedule_activity_in(dimension, biome, baby)
    }

    pub(super) fn villager_schedule_activity_in(
        &self,
        dimension: &LevelId,
        biome: &str,
        baby: bool,
    ) -> Option<&str> {
        let dimension_type = self.cold.level_dimension_types.get(dimension)?;
        let environment = self
            .cold
            .dimension_sky_lights
            .villager_activity
            .get(dimension_type)?;
        environment.sample(biome, baby, |clock| {
            let index = self
                .registries
                .registry_element_index("minecraft:world_clock", clock)?;
            self.world_clock_total_ticks(usize::try_from(index).ok()?)
        })
    }

    pub(super) fn is_bright_outside(&self, dimension: &LevelId) -> Option<bool> {
        let dimension_type = self.cold.level_dimension_types.get(dimension)?;
        self.cold.dimension_sky_lights.is_bright_outside(
            dimension_type,
            self.levels.get(dimension)?.sky_darkening.map(i32::from),
        )
    }

    pub(super) fn is_dark_outside(&self, dimension: &LevelId) -> Option<bool> {
        let dimension_type = self.cold.level_dimension_types.get(dimension)?;
        self.cold.dimension_sky_lights.is_dark_outside(
            dimension_type,
            i32::from(self.levels.get(dimension)?.sky_darkening?),
        )
    }

    fn sleep_environment_biome(
        &self,
        dimension: &LevelId,
        pos: bbb_protocol::packets::BlockPos,
    ) -> &str {
        self.level_by_id(dimension)
            .chunks
            .biome_id_at_quart_or_source(pos.x >> 2, pos.y >> 2, pos.z >> 2)
            .and_then(|id| {
                self.registries
                    .registry_element_id("minecraft:worldgen/biome", id)
            })
            .unwrap_or("")
    }

    pub(super) fn bed_rule_at(
        &self,
        dimension: &LevelId,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Option<BedRule> {
        let dimension_type = self.cold.level_dimension_types.get(dimension)?;
        self.cold
            .dimension_sky_lights
            .bed
            .get(dimension_type)?
            .sample(self.sleep_environment_biome(dimension, pos), |clock| {
                let index = self
                    .registries
                    .registry_element_index("minecraft:world_clock", clock)?;
                self.world_clock_total_ticks(usize::try_from(index).ok()?)
            })
    }

    pub(super) fn anchor_works_at(
        &self,
        dimension: &LevelId,
        pos: bbb_protocol::packets::BlockPos,
    ) -> Option<bool> {
        boolean(self.sample_scalar_at(dimension, ATTR_RESPAWN_ANCHOR_WORKS, Some(pos))?)
    }

    pub(super) fn refresh_all_sky_darkening(&mut self) {
        for dimension in self.level_ids() {
            self.refresh_level_sky_darkening(&dimension);
        }
    }

    pub(super) fn refresh_level_sky_darkening(&mut self, dimension: &LevelId) {
        let darkening = self.dimension_sky_darken(dimension);
        let level = self.level_mut_by_id(dimension);
        level.sky_darkening = darkening;
    }

    pub(in crate::world) fn fast_lava_dimension_value(&self, dimension: &LevelId) -> Option<bool> {
        boolean(self.sample_scalar_at(dimension, ATTR_FAST_LAVA, None)?)
    }

    pub(in crate::world) fn monsters_burn(&self, dimension: &LevelId) -> Option<BiomeBoolean> {
        self.biome_boolean(dimension, ATTR_MONSTERS_BURN)
    }

    pub(in crate::world) fn piglins_zombify(&self, dimension: &LevelId) -> Option<BiomeBoolean> {
        self.biome_boolean(dimension, ATTR_PIGLINS_ZOMBIFY)
    }

    pub(in crate::world) fn water_evaporates(&self, dimension: &LevelId) -> Option<BiomeBoolean> {
        self.biome_boolean(dimension, ATTR_WATER_EVAPORATES)
    }

    fn biome_boolean(&self, dimension: &LevelId, attribute_index: usize) -> Option<BiomeBoolean> {
        const BIOME: &str = "minecraft:worldgen/biome";
        let dimension_type = self.cold.level_dimension_types.get(dimension)?;
        let environment = self
            .cold
            .dimension_sky_lights
            .scalars
            .get(dimension_type)?
            .get(attribute_index)?
            .as_ref()?;
        let weather = ScalarWeather::from_level(&self.levels.get(dimension)?.weather);
        let clock = |clock: &str| {
            let index = self
                .registries
                .registry_element_index("minecraft:world_clock", clock)?;
            self.world_clock_total_ticks(usize::try_from(index).ok()?)
        };
        let sample =
            |biome| boolean(environment.sample(attribute_index, Some(biome), clock, weather)?);
        if !environment.varies_by_biome() {
            return sample("").map(BiomeBoolean::Uniform);
        }
        (0..self.registries.registry_len(BIOME)?)
            .map(|index| {
                let biome = self
                    .registries
                    .registry_element_id(BIOME, i32::try_from(index).ok()?)?;
                sample(biome)
            })
            .collect::<Option<Vec<_>>>()
            .map(BiomeBoolean::ByBiome)
    }

    fn sample_scalar_at(
        &self,
        dimension: &LevelId,
        attribute_index: usize,
        position: Option<bbb_protocol::packets::BlockPos>,
    ) -> Option<VanillaEnvironmentScalarValue> {
        let dimension_type = self.cold.level_dimension_types.get(dimension)?;
        let environment = self
            .cold
            .dimension_sky_lights
            .scalars
            .get(dimension_type)?
            .get(attribute_index)?
            .as_ref()?;
        let biome = position.map(|pos| self.sleep_environment_biome(dimension, pos));
        environment.sample(
            attribute_index,
            biome,
            |clock| {
                let index = self
                    .registries
                    .registry_element_index("minecraft:world_clock", clock)?;
                self.world_clock_total_ticks(usize::try_from(index).ok()?)
            },
            ScalarWeather::from_level(&self.levels.get(dimension)?.weather),
        )
    }

    pub(super) fn can_start_raid(
        &self,
        dimension: &LevelId,
        position: bbb_protocol::packets::BlockPos,
    ) -> Option<bool> {
        boolean(self.sample_scalar_at(dimension, ATTR_CAN_START_RAID, Some(position))?)
    }

    pub(super) fn dimension_sky_darken(&self, dimension: &LevelId) -> Option<u8> {
        let light = number(self.sample_scalar_at(dimension, ATTR_SKY_LIGHT_LEVEL, None)?)?;
        Some((15.0 - light) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::DatapackStartupOptions;
    use bbb_server_persistence::Java26WeatherData;
    use bbb_server_types::save::Java26BuiltinDimension;
    use std::sync::Arc;

    #[test]
    fn the_raid_attribute_is_on_wherever_the_vanilla_data_does_not_turn_it_off() {
        let world = ServerWorld::new(crate::world::ServerWorldConfig::default()).unwrap();
        assert_eq!(
            world.can_start_raid(
                &Java26BuiltinDimension::Overworld.into(),
                bbb_protocol::packets::BlockPos { x: 0, y: 64, z: 0 }
            ),
            Some(true)
        );
        assert_eq!(
            world.fast_lava_dimension_value(&Java26BuiltinDimension::Overworld.into()),
            Some(false)
        );
        assert_eq!(
            world.fast_lava_dimension_value(&Java26BuiltinDimension::Nether.into()),
            Some(true)
        );
        for (dimension, expected) in [
            (Java26BuiltinDimension::Nether, false),
            (Java26BuiltinDimension::End, true),
        ] {
            assert_eq!(
                world.can_start_raid(
                    &dimension.into(),
                    bbb_protocol::packets::BlockPos { x: 0, y: 64, z: 0 }
                ),
                Some(expected)
            );
        }
    }

    #[test]
    fn default_world_retains_builtin_sky_context_for_custom_spawners() {
        let mut world = ServerWorld::new(crate::world::ServerWorldConfig::default()).unwrap();
        let key = Java26BuiltinDimension::Overworld.into();
        assert_eq!(world.dimension_sky_darken(&key), Some(0));
        assert_eq!(world.is_bright_outside(&key), Some(true));
        let clock = world
            .registries
            .registry_element_index("minecraft:world_clock", "minecraft:overworld")
            .unwrap() as usize;
        let mut state = world.world_clock_states_for_test()[clock];
        state.total_ticks = 18_000;
        state.paused = true;
        world.set_world_clock_state_for_test(clock, state);
        world.refresh_level_sky_darkening(&key);
        assert_eq!(world.dimension_sky_darken(&key), Some(11));
        assert_eq!(world.is_bright_outside(&key), Some(false));
    }

    #[test]
    fn sky_projection_uses_live_clock_and_dimension_weather_eligibility() {
        let startup = Arc::new(
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            })
            .unwrap(),
        );
        let mut world = ServerWorld::new(super::super::ServerWorldConfig {
            datapacks: Some(startup),
            ..super::super::ServerWorldConfig::default()
        })
        .unwrap();
        let overworld = LevelId::from(Java26BuiltinDimension::Overworld);
        let clock = usize::try_from(
            world
                .registries
                .registry_element_index("minecraft:world_clock", "minecraft:overworld")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            world
                .level_by_id(&Java26BuiltinDimension::Nether.into())
                .sky_darkening,
            Some(11)
        );
        assert_eq!(
            world
                .level_by_id(&Java26BuiltinDimension::End.into())
                .sky_darkening,
            Some(0)
        );
        let mut state = world.world_clock_states_for_test()[clock];
        state.total_ticks = 17_999;
        world.set_world_clock_state_for_test(clock, state);
        world.tick(Vec::new());
        assert_eq!(world.game_time(), 1);
        assert_eq!(world.world_clock_total_ticks(clock), Some(18_000));
        assert_eq!(world.dimension_sky_darken(&overworld), Some(11));
        let consumer_darkening = |world: &mut ServerWorld| {
            world.with_level(overworld, |level| {
                crate::world::simulation_level::LevelCtx::sky_darken(level)
            })
        };
        assert_eq!(consumer_darkening(&mut world), 11);

        state.total_ticks = 5_999;
        world.set_world_clock_state_for_test(clock, state);
        world.tick(Vec::new());
        assert_eq!(world.dimension_sky_darken(&overworld), Some(0));
        assert_eq!(consumer_darkening(&mut world), 0);
        state.total_ticks = 18_000;
        world.set_world_clock_state_for_test(clock, state);
        assert_eq!(consumer_darkening(&mut world), 0);
        world.tick_rate_mut().set_frozen(true);
        world.tick(Vec::new());
        assert_eq!(consumer_darkening(&mut world), 11);
        world.tick_rate_mut().set_frozen(false);
        state.total_ticks = 5_999;
        world.set_world_clock_state_for_test(clock, state);
        world.weather = Java26WeatherData {
            raining: true,
            thundering: true,
            rain_time: 10_000,
            thunder_time: 10_000,
            ..Java26WeatherData::default()
        };
        let weather = world.weather;
        for level in world.levels.values_mut() {
            level.weather.prepare(weather);
        }
        world.tick(Vec::new());
        assert_eq!(world.dimension_sky_darken(&overworld), Some(5));
        assert_eq!(consumer_darkening(&mut world), 5);
        for (dimension, darken) in [
            (Java26BuiltinDimension::Nether, 11),
            (Java26BuiltinDimension::End, 0),
        ] {
            assert_eq!(world.dimension_sky_darken(&dimension.into()), Some(darken));
        }
    }
}

#[cfg(test)]
mod time_marker_tests;

#[cfg(test)]
mod patrol_tests;

#[cfg(test)]
mod moon_tests;

#[cfg(test)]
mod block_consumer_tests;
