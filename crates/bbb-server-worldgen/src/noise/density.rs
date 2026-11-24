//! Executable base-density semantics for `minecraft:noise` level sources.
//!
//! This resolves one `NoiseGeneratorSettings` holder and computes the density
//! functions that `NoiseChunk` consumes for fresh base terrain. Biomes,
//! surface rules and carvers remain outside this owner, so a source validated
//! here is not yet a served level source.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::worldgen::density::{
    BinaryKind, MappedKind, MarkerKind, DENSITY_FUNCTION_REGISTRY, MAX_DENSITY_DEPTH,
};
use bbb_server_datapack::worldgen::noise::{NOISE_REGISTRY, NOISE_SETTINGS_REGISTRY};
use bbb_server_datapack::worldgen::{density as seed, noise::NoiseHolder, Holder};

use super::{create_normal_noise, BlendedNoise, NormalNoise};

pub mod chunk;
pub mod end;
pub mod overworld;

pub use chunk::{NoiseBlender, NoiseChunkSampler};

use end::EndIslandDensity;
use overworld::{
    apply_mapped, NoiseSampler, ShiftSampler, ShiftedNoiseSampler, Spline, SplinePoint,
    WeirdScaledSampler,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseDensityError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for NoiseDensityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for NoiseDensityError {}

pub fn refuse(path: impl Into<String>, message: impl Into<String>) -> NoiseDensityError {
    NoiseDensityError {
        path: path.into(),
        message: message.into(),
    }
}

/// Index of a node in [`NoiseDensitySource`]'s flat density graph.
pub type NodeId = u32;

/// `NoiseRouter` fields in codec declaration order (`NoiseRouter.java:24-42`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Router {
    Barrier,
    FluidLevelFloodedness,
    FluidLevelSpread,
    Lava,
    Temperature,
    Vegetation,
    Continents,
    Erosion,
    Depth,
    Ridges,
    PreliminarySurfaceLevel,
    FinalDensity,
    VeinToggle,
    VeinRidged,
    VeinGap,
}

const CLIMATE: [Router; 6] = [
    Router::Temperature,
    Router::Vegetation,
    Router::Continents,
    Router::Erosion,
    Router::Depth,
    Router::Ridges,
];

/// The part of `NoiseGeneratorSettings` needed to evaluate its base density.
///
/// The density graph is one flat node array in post order (children before
/// parents); shared holders are one node.
pub struct NoiseDensitySource {
    settings_id: String,
    min_y: i32,
    height: i32,
    cell_width: i32,
    cell_height: i32,
    default_block_state_id: i32,
    default_fluid_state_id: i32,
    air_block_state_id: i32,
    water_block_state_id: i32,
    lava_block_state_id: i32,
    sea_level: i32,
    disable_mob_generation: bool,
    aquifers_enabled: bool,
    ore_veins_enabled: bool,
    climate_enabled: bool,
    world_seed: i64,
    /// Only surface rules still read this: legacy settings look the surface
    /// biome up at y = 0.
    legacy_random_source: bool,
    nodes: Vec<DensityFunction>,
    router: [NodeId; 15],
    /// Marker kinds by cache slot; `NoiseChunkSampler` owns one cache per slot.
    marker_kinds: Vec<MarkerKind>,
    /// `interpolated` markers reached by the cell fill, mapped to their
    /// `NoiseChunkSampler::scratch.interpolators` index in `NoiseChunk.wrap` order.
    interpolators: Vec<(usize, NodeId)>,
    /// Subset of `interpolators` indices read during `select_cell_yz`
    /// (`FinalDensity` and explicit `cache_all_in_cell` nodes).
    cell_fill_interpolators: Vec<usize>,
    /// Subset of `interpolators` indices read during the per-block loop
    /// (`VeinToggle`, `VeinRidged`, `VeinGap` when `ore_veins_enabled`).
    block_interpolators: Vec<usize>,
    /// Explicit `cache_all_in_cell` markers reached by the same cell fill,
    /// after their nested children and in graph argument order.
    cell_caches: Vec<(usize, NodeId)>,
    /// `NoiseGeneratorSettings.spawnTarget`, read by `Climate.Sampler.findSpawnPosition`.
    spawn_target: Vec<crate::noise::biome::climate::ParameterPoint>,
}

impl NoiseDensitySource {
    pub fn from_registry(
        generation: &DatapackGeneration,
        settings_id: &str,
        world_seed: i64,
    ) -> Result<Self, NoiseDensityError> {
        Self::from_registry_mode(generation, settings_id, world_seed, false)
    }

    pub fn from_registry_with_climate(
        generation: &DatapackGeneration,
        settings_id: &str,
        world_seed: i64,
    ) -> Result<Self, NoiseDensityError> {
        Self::from_registry_mode(generation, settings_id, world_seed, true)
    }

    pub fn from_registry_mode(
        generation: &DatapackGeneration,
        settings_id: &str,
        world_seed: i64,
        climate_enabled: bool,
    ) -> Result<Self, NoiseDensityError> {
        let settings_id = normalize_resource_id(settings_id, "noise settings holder")?;
        let path = format!("{NOISE_SETTINGS_REGISTRY}.{settings_id}");
        let settings = generation
            .worldgen
            .noise_settings
            .get(&settings_id)
            .ok_or_else(|| refuse(&path, "noise settings holder is absent"))?;
        let min_y = settings.noise.min_y;
        let height = settings.noise.height;
        let size_horizontal = settings.noise.size_horizontal;
        let size_vertical = settings.noise.size_vertical;
        let default_block_state_id = settings.default_block;
        let default_fluid_state_id = settings.default_fluid;
        // `NoiseGeneratorSettings.java:20-45` retains this exact BlockState.
        let default_state = |name| {
            vanilla_block_type_for_resource_id(name)
                .and_then(|block| vanilla_block_default_state_id(block.id))
                .ok_or_else(|| refuse(&path, format!("Vanilla 26.1 {name} is unavailable")))
        };
        let air_block_state_id = default_state("minecraft:air")?;
        let water_block_state_id = default_state("minecraft:water")?;
        let lava_block_state_id = default_state("minecraft:lava")?;
        let sea_level = settings.sea_level;
        let disable_mob_generation = settings.disable_mob_generation;
        let aquifers_enabled = settings.aquifers_enabled;
        let ore_veins_enabled = settings.ore_veins_enabled;
        let legacy_random_source = settings.legacy_random_source;
        let mut resolver = DensityResolver {
            generation,
            world_seed,
            nodes: Vec::new(),
            cache: BTreeMap::new(),
            noise_cache: BTreeMap::new(),
            resolving: BTreeSet::new(),
            resolution_stack: Vec::new(),
            marker_kinds: Vec::new(),
        };
        // `NoiseRouter.CODEC` requires all 15 fields, independent of enabled consumers.
        let mut router = [0; 15];
        for (slot, holder) in router.iter_mut().zip(&settings.noise_router) {
            *slot = resolver.compile_holder(holder, &path, 0)?;
        }

        let mut source = Self {
            settings_id,
            min_y,
            height,
            cell_width: size_horizontal * 4,
            cell_height: size_vertical * 4,
            default_block_state_id,
            default_fluid_state_id,
            air_block_state_id,
            water_block_state_id,
            lava_block_state_id,
            sea_level,
            disable_mob_generation,
            aquifers_enabled,
            ore_veins_enabled,
            climate_enabled,
            world_seed,
            legacy_random_source,
            nodes: resolver.nodes,
            router,
            marker_kinds: resolver.marker_kinds,
            interpolators: Vec::new(),
            cell_fill_interpolators: Vec::new(),
            block_interpolators: Vec::new(),
            cell_caches: Vec::new(),
            spawn_target: crate::noise::biome::climate::spawn_targets(&settings.spawn_target),
        };
        // Router fields are wrapped in declaration order before cell caches
        // fill (`NoiseRouter.java:49-67`, `NoiseChunk.java:299-315`).
        let mut wrapped = Vec::new();
        if aquifers_enabled {
            wrapped.extend([
                Router::Barrier,
                Router::FluidLevelFloodedness,
                Router::FluidLevelSpread,
                Router::Lava,
            ]);
        }
        if climate_enabled {
            wrapped.extend(CLIMATE);
        } else if aquifers_enabled {
            wrapped.extend([Router::Erosion, Router::Depth]);
        }
        wrapped.extend([Router::PreliminarySurfaceLevel, Router::FinalDensity]);
        if ore_veins_enabled {
            wrapped.extend([Router::VeinToggle, Router::VeinRidged, Router::VeinGap]);
        }
        let (mut interpolator_slots, mut cell_caches) = (Vec::new(), Vec::new());
        for field in wrapped {
            source.collect_chunk_markers(
                source.router(field),
                &mut interpolator_slots,
                &mut cell_caches,
            );
        }
        let mut slot_to_interpolator = vec![usize::MAX; source.marker_kinds.len()];
        let mut interpolator_count = 0usize;
        for (slot, kind) in source.marker_kinds.iter().enumerate() {
            if matches!(kind, MarkerKind::Interpolated) {
                slot_to_interpolator[slot] = interpolator_count;
                interpolator_count += 1;
            }
        }
        let mut cell_fill_slots = Vec::new();
        let mut ignored_cell_caches = Vec::new();
        source.collect_chunk_markers(
            source.router(Router::FinalDensity),
            &mut cell_fill_slots,
            &mut ignored_cell_caches,
        );
        for (_, input) in &cell_caches {
            source.collect_chunk_markers(*input, &mut cell_fill_slots, &mut ignored_cell_caches);
        }
        let mut block_slots = Vec::new();
        if ore_veins_enabled {
            for field in [Router::VeinToggle, Router::VeinRidged, Router::VeinGap] {
                source.collect_chunk_markers(
                    source.router(field),
                    &mut block_slots,
                    &mut ignored_cell_caches,
                );
            }
        }
        source.interpolators = interpolator_slots
            .into_iter()
            .map(|(slot, input)| (slot_to_interpolator[slot], input))
            .collect();
        source.cell_fill_interpolators = cell_fill_slots
            .into_iter()
            .map(|(slot, _)| slot_to_interpolator[slot])
            .collect();
        source.block_interpolators = block_slots
            .into_iter()
            .map(|(slot, _)| slot_to_interpolator[slot])
            .collect();
        source.cell_caches = cell_caches;
        Ok(source)
    }

    pub fn settings_id(&self) -> &str {
        &self.settings_id
    }

    pub const fn min_y(&self) -> i32 {
        self.min_y
    }

    pub const fn height(&self) -> i32 {
        self.height
    }

    pub const fn cell_width(&self) -> i32 {
        self.cell_width
    }

    pub const fn cell_height(&self) -> i32 {
        self.cell_height
    }

    pub const fn default_block_state_id(&self) -> i32 {
        self.default_block_state_id
    }

    pub const fn default_fluid_state_id(&self) -> i32 {
        self.default_fluid_state_id
    }

    pub const fn air_block_state_id(&self) -> i32 {
        self.air_block_state_id
    }

    pub const fn water_block_state_id(&self) -> i32 {
        self.water_block_state_id
    }

    pub const fn lava_block_state_id(&self) -> i32 {
        self.lava_block_state_id
    }

    pub const fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub const fn disable_mob_generation(&self) -> bool {
        self.disable_mob_generation
    }

    pub const fn aquifers_enabled(&self) -> bool {
        self.aquifers_enabled
    }

    pub const fn ore_veins_enabled(&self) -> bool {
        self.ore_veins_enabled
    }

    pub const fn world_seed(&self) -> i64 {
        self.world_seed
    }

    pub const fn legacy_random_source(&self) -> bool {
        self.legacy_random_source
    }

    pub const fn router(&self, field: Router) -> NodeId {
        self.router[field as usize]
    }

    fn node(&self, id: NodeId) -> &DensityFunction {
        &self.nodes[id as usize]
    }

    const fn interpolated_mode(&self) -> SampleMode {
        SampleMode::Interpolated {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
        }
    }

    /// A router function outside any `NoiseChunk`, with `flat_cache` and
    /// `interpolated` markers applied but no memoisation.
    pub fn sample_router(&self, field: Router, x: i32, y: i32, z: i32) -> f64 {
        self.sample_mode(self.router(field), x, y, z, self.interpolated_mode())
    }

    /// `NoiseChunk.preliminarySurfaceLevel` (`NoiseChunk.java:218-228`) on a
    /// one-column `NoiseChunk`, so `cache_2d` markers survive the
    /// `find_top_surface` probes.
    pub fn preliminary_surface_level(&self, x: i32, z: i32) -> i32 {
        NoiseChunkSampler::for_point(self, x, z).preliminary_surface_level(x, z)
    }

    /// `DensityFunction.compute` before `NoiseChunk` applies marker caches and
    /// cell interpolation (`DensityFunction.java:12-29`,
    /// `NoiseBasedChunkGenerator.java:130-160`).
    pub fn sample_uninterpolated(&self, x: i32, y: i32, z: i32) -> f64 {
        self.sample_mode(self.router(Router::FinalDensity), x, y, z, SampleMode::Raw)
    }

    /// `CacheAllInCell` samples `NoiseInterpolator` through `Mth.lerp3`, whose
    /// rounding order is X, Y, then Z (`NoiseChunk.java:766-779`).
    pub fn sample_interpolated(&self, x: i32, y: i32, z: i32) -> f64 {
        self.sample_router(Router::FinalDensity, x, y, z)
    }

    /// `Climate.Sampler` converts quart to block coordinates and quantizes
    /// six f32 climate values (`Climate.java:474-496`).
    ///
    /// Matches `RandomState.java:85-109`: the uncached sampler strips chunk
    /// markers (`SampleMode::Raw`) while memoising shared `cache_2d`,
    /// `cache_once`, and `flat_cache` subgraphs across the six fields at one point.
    pub fn climate_target(&self, quart_x: i32, quart_y: i32, quart_z: i32) -> Option<[i64; 7]> {
        if !self.climate_enabled {
            return None;
        }
        let (x, y, z) = (
            quart_x.wrapping_mul(4),
            quart_y.wrapping_mul(4),
            quart_z.wrapping_mul(4),
        );
        let marker_count = self.marker_kinds.len();
        if marker_count <= 64 {
            let mut memo = [None; 64];
            Some(self.sample_climate_point(x, y, z, &mut memo[..marker_count]))
        } else {
            let mut memo = vec![None; marker_count];
            Some(self.sample_climate_point(x, y, z, &mut memo))
        }
    }

    /// `Climate.Sampler.findSpawnPosition` (`Climate.java:498-500`): the block
    /// column `[x, z]` a new world's spawn search starts from.
    pub fn find_spawn_position(&self) -> [i32; 2] {
        let mut memo = vec![None; self.marker_kinds.len()];
        crate::noise::biome::climate::find_spawn_position(&self.spawn_target, |x, z| {
            memo.fill(None);
            self.sample_climate_point((x >> 2) * 4, 0, (z >> 2) * 4, &mut memo)
        })
    }

    pub fn sample_erosion(&self, x: i32, y: i32, z: i32) -> Option<f64> {
        if !self.climate_enabled {
            return None;
        }
        let marker_count = self.marker_kinds.len();
        if marker_count <= 64 {
            let mut memo = [None; 64];
            Some(self.sample_raw_memo(
                self.router(Router::Erosion),
                x,
                y,
                z,
                &mut memo[..marker_count],
            ))
        } else {
            let mut memo = vec![None; marker_count];
            Some(self.sample_raw_memo(self.router(Router::Erosion), x, y, z, &mut memo))
        }
    }

    fn sample_climate_point(&self, x: i32, y: i32, z: i32, memo: &mut [Option<f64>]) -> [i64; 7] {
        let mut target = [0; 7];
        for (value, field) in target.iter_mut().zip(CLIMATE) {
            *value = quantize_climate(self.sample_raw_memo(self.router(field), x, y, z, memo));
        }
        target
    }

    fn sample_raw_memo(&self, id: NodeId, x: i32, y: i32, z: i32, memo: &mut [Option<f64>]) -> f64 {
        match self.node(id) {
            DensityFunction::Constant(value) => *value,
            DensityFunction::BeardifierMarker => 0.0,
            DensityFunction::YClampedGradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => {
                let factor =
                    (f64::from(y) - f64::from(*from_y)) / (f64::from(*to_y) - f64::from(*from_y));
                clamped_lerp(factor, *from_value, *to_value)
            }
            DensityFunction::Binary {
                kind,
                first,
                second,
            } => {
                let first = self.sample_raw_memo(*first, x, y, z, memo);
                match kind {
                    BinaryKind::Add => first + self.sample_raw_memo(*second, x, y, z, memo),
                    BinaryKind::Mul if first == 0.0 => 0.0,
                    BinaryKind::Mul => first * self.sample_raw_memo(*second, x, y, z, memo),
                    BinaryKind::Min => {
                        java_min(first, self.sample_raw_memo(*second, x, y, z, memo))
                    }
                    BinaryKind::Max => {
                        java_max(first, self.sample_raw_memo(*second, x, y, z, memo))
                    }
                }
            }
            DensityFunction::Mapped { kind, input } => {
                apply_mapped(*kind, self.sample_raw_memo(*input, x, y, z, memo))
            }
            DensityFunction::Marker { kind, input, slot } => match kind {
                MarkerKind::Cache2d | MarkerKind::CacheOnce | MarkerKind::FlatCache => {
                    if let Some(cached) = memo[*slot] {
                        return cached;
                    }
                    let value = self.sample_raw_memo(*input, x, y, z, memo);
                    memo[*slot] = Some(value);
                    value
                }
                MarkerKind::CacheAllInCell | MarkerKind::Interpolated => {
                    self.sample_raw_memo(*input, x, y, z, memo)
                }
            },
            DensityFunction::Clamp { input, min, max } => {
                java_clamp(self.sample_raw_memo(*input, x, y, z, memo), *min, *max)
            }
            DensityFunction::RangeChoice {
                input,
                min_inclusive,
                max_exclusive,
                when_in_range,
                when_out_of_range,
            } => {
                let value = self.sample_raw_memo(*input, x, y, z, memo);
                if value >= *min_inclusive && value < *max_exclusive {
                    self.sample_raw_memo(*when_in_range, x, y, z, memo)
                } else {
                    self.sample_raw_memo(*when_out_of_range, x, y, z, memo)
                }
            }
            DensityFunction::FindTopSurface {
                density,
                upper_bound,
                lower_bound,
                cell_height,
            } => {
                const MAX_SURFACE_Y: i32 = 2032;
                let step = (*cell_height).max(1);
                let raw_upper = self.sample_raw_memo(*upper_bound, x, y, z, memo);
                if !raw_upper.is_finite() {
                    return f64::from(*lower_bound);
                }
                let clamped_upper = raw_upper.clamp(
                    f64::from(*lower_bound),
                    f64::from(MAX_SURFACE_Y.max(*lower_bound)),
                );
                let top_y = (clamped_upper / f64::from(step)).floor() as i32 * step;
                if top_y <= *lower_bound {
                    return f64::from(*lower_bound);
                }
                let mut block_y = top_y;
                while block_y >= *lower_bound {
                    if self.sample_mode(*density, x, block_y, z, SampleMode::Raw) > 0.0 {
                        return f64::from(block_y);
                    }
                    let Some(next_y) = block_y.checked_sub(step) else {
                        break;
                    };
                    block_y = next_y;
                }
                f64::from(*lower_bound)
            }
            DensityFunction::BlendDensity(input) => self.sample_raw_memo(*input, x, y, z, memo),
            DensityFunction::BlendAlpha => 1.0,
            DensityFunction::BlendOffset => 0.0,
            DensityFunction::Noise(noise) => noise.sample(x, y, z),
            DensityFunction::Shift(noise) => noise.sample(x, y, z),
            DensityFunction::ShiftedNoise(noise) => {
                noise.sample(x, y, z, |id| self.sample_raw_memo(id, x, y, z, memo))
            }
            DensityFunction::Spline(spline) => {
                f64::from(spline.evaluate(&mut |id| self.sample_raw_memo(id, x, y, z, memo)))
            }
            DensityFunction::WeirdScaled(sampler) => {
                sampler.sample(x, y, z, self.sample_raw_memo(sampler.input, x, y, z, memo))
            }
            DensityFunction::OldBlendedNoise(noise) => noise.compute(x, y, z),
            DensityFunction::EndIslands(islands) => islands.sample(x, z),
        }
    }

    fn sample_mode(&self, id: NodeId, x: i32, y: i32, z: i32, mode: SampleMode) -> f64 {
        match self.node(id) {
            DensityFunction::Constant(value) => *value,
            // The unit marker is zero until `NoiseChunk.wrap` binds the
            // target Beardifier (`DensityFunctions.java:340-370`).
            DensityFunction::BeardifierMarker => 0.0,
            DensityFunction::YClampedGradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => {
                let factor =
                    (f64::from(y) - f64::from(*from_y)) / (f64::from(*to_y) - f64::from(*from_y));
                clamped_lerp(factor, *from_value, *to_value)
            }
            DensityFunction::Binary {
                kind,
                first,
                second,
            } => {
                let first = self.sample_mode(*first, x, y, z, mode);
                match kind {
                    BinaryKind::Add => first + self.sample_mode(*second, x, y, z, mode),
                    BinaryKind::Mul if first == 0.0 => 0.0,
                    BinaryKind::Mul => first * self.sample_mode(*second, x, y, z, mode),
                    BinaryKind::Min => java_min(first, self.sample_mode(*second, x, y, z, mode)),
                    BinaryKind::Max => java_max(first, self.sample_mode(*second, x, y, z, mode)),
                }
            }
            DensityFunction::Mapped { kind, input } => {
                apply_mapped(*kind, self.sample_mode(*input, x, y, z, mode))
            }
            DensityFunction::Marker { kind, input, .. } => match (kind, mode) {
                (MarkerKind::Cache2d | MarkerKind::CacheAllInCell | MarkerKind::CacheOnce, _)
                | (_, SampleMode::Raw) => self.sample_mode(*input, x, y, z, mode),
                (MarkerKind::FlatCache, SampleMode::Interpolated { .. }) => {
                    // `NoiseChunk.FlatCache` (`NoiseChunk.java:631-660`).
                    let quantized_x = x - x.rem_euclid(4);
                    let quantized_z = z - z.rem_euclid(4);
                    self.sample_mode(*input, quantized_x, 0, quantized_z, SampleMode::Raw)
                }
                (
                    MarkerKind::Interpolated,
                    SampleMode::Interpolated {
                        cell_width,
                        cell_height,
                    },
                ) => self.interpolate(*input, x, y, z, cell_width, cell_height),
            },
            DensityFunction::Clamp { input, min, max } => {
                java_clamp(self.sample_mode(*input, x, y, z, mode), *min, *max)
            }
            DensityFunction::RangeChoice {
                input,
                min_inclusive,
                max_exclusive,
                when_in_range,
                when_out_of_range,
            } => {
                let value = self.sample_mode(*input, x, y, z, mode);
                if value >= *min_inclusive && value < *max_exclusive {
                    self.sample_mode(*when_in_range, x, y, z, mode)
                } else {
                    self.sample_mode(*when_out_of_range, x, y, z, mode)
                }
            }
            DensityFunction::FindTopSurface {
                density,
                upper_bound,
                lower_bound,
                cell_height,
            } => {
                // Minecraft DimensionType.MAX_Y = 2031; bound probes against crafted datapacks.
                const MAX_SURFACE_Y: i32 = 2032;
                let step = (*cell_height).max(1);
                let raw_upper = self.sample_mode(*upper_bound, x, y, z, mode);
                if !raw_upper.is_finite() {
                    return f64::from(*lower_bound);
                }
                let clamped_upper = raw_upper.clamp(
                    f64::from(*lower_bound),
                    f64::from(MAX_SURFACE_Y.max(*lower_bound)),
                );
                let top_y = (clamped_upper / f64::from(step)).floor() as i32 * step;
                if top_y <= *lower_bound {
                    return f64::from(*lower_bound);
                }
                let mut block_y = top_y;
                while block_y >= *lower_bound {
                    if self.sample_mode(*density, x, block_y, z, mode) > 0.0 {
                        return f64::from(block_y);
                    }
                    let Some(next_y) = block_y.checked_sub(step) else {
                        break;
                    };
                    block_y = next_y;
                }
                f64::from(*lower_bound)
            }
            DensityFunction::BlendDensity(input) => self.sample_mode(*input, x, y, z, mode),
            DensityFunction::BlendAlpha => 1.0,
            DensityFunction::BlendOffset => 0.0,
            DensityFunction::Noise(noise) => noise.sample(x, y, z),
            DensityFunction::Shift(noise) => noise.sample(x, y, z),
            DensityFunction::ShiftedNoise(noise) => {
                noise.sample(x, y, z, |id| self.sample_mode(id, x, y, z, mode))
            }
            DensityFunction::Spline(spline) => {
                f64::from(spline.evaluate(&mut |id| self.sample_mode(id, x, y, z, mode)))
            }
            DensityFunction::WeirdScaled(sampler) => {
                sampler.sample(x, y, z, self.sample_mode(sampler.input, x, y, z, mode))
            }
            DensityFunction::OldBlendedNoise(noise) => noise.compute(x, y, z),
            DensityFunction::EndIslands(islands) => islands.sample(x, z),
        }
    }

    fn interpolate(
        &self,
        input: NodeId,
        x: i32,
        y: i32,
        z: i32,
        cell_width: i32,
        cell_height: i32,
    ) -> f64 {
        let x0 = x - x.rem_euclid(cell_width);
        let y0 = y - y.rem_euclid(cell_height);
        let z0 = z - z.rem_euclid(cell_width);
        let x1 = x0.wrapping_add(cell_width);
        let y1 = y0.wrapping_add(cell_height);
        let z1 = z0.wrapping_add(cell_width);
        let mode = SampleMode::Interpolated {
            cell_width,
            cell_height,
        };
        let sample = |sample_x, sample_y, sample_z| {
            self.sample_mode(input, sample_x, sample_y, sample_z, mode)
        };
        let factor_x = f64::from(x.rem_euclid(cell_width)) / f64::from(cell_width);
        let factor_y = f64::from(y.rem_euclid(cell_height)) / f64::from(cell_height);
        let factor_z = f64::from(z.rem_euclid(cell_width)) / f64::from(cell_width);
        lerp(
            factor_z,
            lerp(
                factor_y,
                lerp(factor_x, sample(x0, y0, z0), sample(x1, y0, z0)),
                lerp(factor_x, sample(x0, y1, z0), sample(x1, y1, z0)),
            ),
            lerp(
                factor_y,
                lerp(factor_x, sample(x0, y0, z1), sample(x1, y0, z1)),
                lerp(factor_x, sample(x0, y1, z1), sample(x1, y1, z1)),
            ),
        )
    }

    /// Collects cell marker slots in `NoiseChunk.wrap` order (children before
    /// parents, first argument before second).
    fn collect_chunk_markers(
        &self,
        id: NodeId,
        interpolators: &mut Vec<(usize, NodeId)>,
        cell_caches: &mut Vec<(usize, NodeId)>,
    ) {
        let mut visit =
            |child: NodeId| self.collect_chunk_markers(child, interpolators, cell_caches);
        match self.node(id) {
            DensityFunction::Binary { first, second, .. } => {
                visit(*first);
                visit(*second);
            }
            DensityFunction::Mapped { input, .. }
            | DensityFunction::Clamp { input, .. }
            | DensityFunction::BlendDensity(input) => visit(*input),
            DensityFunction::Marker { kind, input, slot } => {
                visit(*input);
                let list = match kind {
                    MarkerKind::Interpolated => &mut *interpolators,
                    MarkerKind::CacheAllInCell => &mut *cell_caches,
                    _ => return,
                };
                if !list.iter().any(|(existing, _)| existing == slot) {
                    list.push((*slot, *input));
                }
            }
            DensityFunction::RangeChoice {
                input,
                when_in_range,
                when_out_of_range,
                ..
            } => {
                visit(*input);
                visit(*when_in_range);
                visit(*when_out_of_range);
            }
            DensityFunction::FindTopSurface {
                density,
                upper_bound,
                ..
            } => {
                visit(*density);
                visit(*upper_bound);
            }
            DensityFunction::ShiftedNoise(noise) => {
                visit(noise.shift_x);
                visit(noise.shift_y);
                visit(noise.shift_z);
            }
            DensityFunction::Spline(spline) => spline.visit_coordinates(&mut visit),
            DensityFunction::WeirdScaled(sampler) => visit(sampler.input),
            DensityFunction::Constant(_)
            | DensityFunction::BeardifierMarker
            | DensityFunction::YClampedGradient { .. }
            | DensityFunction::BlendAlpha
            | DensityFunction::BlendOffset
            | DensityFunction::Noise(_)
            | DensityFunction::Shift(_)
            | DensityFunction::OldBlendedNoise(_)
            | DensityFunction::EndIslands(_) => {}
        }
    }
}

fn quantize_climate(value: f64) -> i64 {
    (value as f32 * 10_000.0_f32) as i64
}

/// The climate side of `Climate.Sampler`, either uncached
/// (`RandomState.sampler`) or through a chunk's caches
/// (`NoiseChunk.cachedClimateSampler`).
pub trait ClimateSampler {
    fn climate_target(&mut self, quart_x: i32, quart_y: i32, quart_z: i32) -> Option<[i64; 7]>;

    fn sample_erosion(&mut self, x: i32, y: i32, z: i32) -> Option<f64>;
}

impl ClimateSampler for &NoiseDensitySource {
    fn climate_target(&mut self, quart_x: i32, quart_y: i32, quart_z: i32) -> Option<[i64; 7]> {
        NoiseDensitySource::climate_target(self, quart_x, quart_y, quart_z)
    }

    fn sample_erosion(&mut self, x: i32, y: i32, z: i32) -> Option<f64> {
        NoiseDensitySource::sample_erosion(self, x, y, z)
    }
}

/// The ore-vein router functions as `OreVeinifier.create` computes them.
pub trait VeinRouter {
    fn vein(&mut self, field: Router, x: i32, y: i32, z: i32) -> f64;
}

impl VeinRouter for &NoiseDensitySource {
    fn vein(&mut self, field: Router, x: i32, y: i32, z: i32) -> f64 {
        self.sample_router(field, x, y, z)
    }
}

/// One node of the flat density graph; children are earlier [`NodeId`]s.
pub enum DensityFunction {
    Constant(f64),
    BeardifierMarker,
    YClampedGradient {
        from_y: i32,
        to_y: i32,
        from_value: f64,
        to_value: f64,
    },
    Binary {
        kind: BinaryKind,
        first: NodeId,
        second: NodeId,
    },
    Mapped {
        kind: MappedKind,
        input: NodeId,
    },
    Marker {
        kind: MarkerKind,
        input: NodeId,
        slot: usize,
    },
    Clamp {
        input: NodeId,
        min: f64,
        max: f64,
    },
    RangeChoice {
        input: NodeId,
        min_inclusive: f64,
        max_exclusive: f64,
        when_in_range: NodeId,
        when_out_of_range: NodeId,
    },
    FindTopSurface {
        density: NodeId,
        upper_bound: NodeId,
        lower_bound: i32,
        cell_height: i32,
    },
    BlendDensity(NodeId),
    BlendAlpha,
    BlendOffset,
    Noise(NoiseSampler),
    Shift(ShiftSampler),
    ShiftedNoise(ShiftedNoiseSampler),
    Spline(Spline),
    WeirdScaled(WeirdScaledSampler),
    OldBlendedNoise(Box<BlendedNoise>),
    EndIslands(Box<EndIslandDensity>),
}

#[derive(Clone, Copy)]
pub enum SampleMode {
    Raw,
    Interpolated { cell_width: i32, cell_height: i32 },
}

struct DensityResolver<'a> {
    generation: &'a DatapackGeneration,
    world_seed: i64,
    nodes: Vec<DensityFunction>,
    cache: BTreeMap<String, NodeId>,
    noise_cache: BTreeMap<String, Arc<NormalNoise>>,
    resolving: BTreeSet<String>,
    resolution_stack: Vec<String>,
    marker_kinds: Vec<MarkerKind>,
}

impl DensityResolver<'_> {
    fn compile_holder(
        &mut self,
        holder: &seed::DensityHolder,
        path: &str,
        depth: usize,
    ) -> Result<NodeId, NoiseDensityError> {
        match holder {
            Holder::Named(reference) => self.compile_reference(reference, path, depth),
            Holder::Inline(value) => self.compile_direct(value, path, depth),
        }
    }
    fn compile_reference(
        &mut self,
        reference: &str,
        path: &str,
        depth: usize,
    ) -> Result<NodeId, NoiseDensityError> {
        self.check_depth(path, depth)?;
        if let Some(cached) = self.cache.get(reference) {
            return Ok(*cached);
        }
        if !self.resolving.insert(reference.to_string()) {
            let cycle_start = self
                .resolution_stack
                .iter()
                .position(|item| item == reference)
                .unwrap_or(0);
            let cycle = self.resolution_stack[cycle_start..]
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(reference))
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(refuse(
                path,
                format!("density-function holder cycle: {cycle}"),
            ));
        }
        self.resolution_stack.push(reference.to_string());
        let result = (|| {
            let value = self
                .generation
                .worldgen
                .density_functions
                .get(reference)
                .ok_or_else(|| {
                    refuse(
                        DENSITY_FUNCTION_REGISTRY,
                        format!("has no element {reference}"),
                    )
                })?;
            self.compile_direct(
                value,
                &format!("{DENSITY_FUNCTION_REGISTRY}.{reference}"),
                depth + 1,
            )
        })();
        self.resolution_stack.pop();
        self.resolving.remove(reference);
        let node = result?;
        self.cache.insert(reference.to_string(), node);
        Ok(node)
    }
    fn compile_direct(
        &mut self,
        value: &seed::DensityFunction,
        path: &str,
        depth: usize,
    ) -> Result<NodeId, NoiseDensityError> {
        self.check_depth(path, depth)?;
        use seed::DensityFunction as Seed;
        let function = match value {
            Seed::Constant(value) => DensityFunction::Constant(*value),
            Seed::Beardifier => DensityFunction::BeardifierMarker,
            Seed::BlendAlpha => DensityFunction::BlendAlpha,
            Seed::BlendOffset => DensityFunction::BlendOffset,
            Seed::EndIslands => {
                DensityFunction::EndIslands(Box::new(EndIslandDensity::new(self.world_seed)))
            }
            Seed::YClampedGradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => DensityFunction::YClampedGradient {
                from_y: *from_y,
                to_y: *to_y,
                from_value: *from_value,
                to_value: *to_value,
            },
            Seed::Binary {
                kind,
                first,
                second,
            } => DensityFunction::Binary {
                kind: *kind,
                first: self.compile_holder(first, path, depth + 1)?,
                second: self.compile_holder(second, path, depth + 1)?,
            },
            Seed::Mapped { kind, input } => DensityFunction::Mapped {
                kind: *kind,
                input: self.compile_holder(input, path, depth + 1)?,
            },
            Seed::Marker { kind, input } => {
                let kind = *kind;
                let input = self.compile_holder(input, path, depth + 1)?;
                let slot = self.marker_kinds.len();
                self.marker_kinds.push(kind);
                DensityFunction::Marker { kind, input, slot }
            }
            Seed::Clamp { input, min, max } => DensityFunction::Clamp {
                input: self.compile_direct(input, path, depth + 1)?,
                min: *min,
                max: *max,
            },
            Seed::RangeChoice {
                input,
                min_inclusive,
                max_exclusive,
                when_in_range,
                when_out_of_range,
            } => DensityFunction::RangeChoice {
                input: self.compile_holder(input, path, depth + 1)?,
                min_inclusive: *min_inclusive,
                max_exclusive: *max_exclusive,
                when_in_range: self.compile_holder(when_in_range, path, depth + 1)?,
                when_out_of_range: self.compile_holder(when_out_of_range, path, depth + 1)?,
            },
            Seed::FindTopSurface {
                density,
                upper_bound,
                lower_bound,
                cell_height,
            } => DensityFunction::FindTopSurface {
                density: self.compile_holder(density, path, depth + 1)?,
                upper_bound: self.compile_holder(upper_bound, path, depth + 1)?,
                lower_bound: *lower_bound,
                cell_height: *cell_height,
            },
            Seed::BlendDensity(input) => {
                DensityFunction::BlendDensity(self.compile_holder(input, path, depth + 1)?)
            }
            Seed::Noise {
                noise,
                xz_scale,
                y_scale,
            } => DensityFunction::Noise(NoiseSampler {
                noise: self.compile_noise(noise, path)?,
                xz_scale: *xz_scale,
                y_scale: *y_scale,
            }),
            Seed::Shift { kind, noise } => DensityFunction::Shift(ShiftSampler {
                kind: *kind,
                noise: self.compile_noise(noise, path)?,
            }),
            Seed::ShiftedNoise {
                shift_x,
                shift_y,
                shift_z,
                xz_scale,
                y_scale,
                noise,
            } => DensityFunction::ShiftedNoise(ShiftedNoiseSampler {
                shift_x: self.compile_holder(shift_x, path, depth + 1)?,
                shift_y: self.compile_holder(shift_y, path, depth + 1)?,
                shift_z: self.compile_holder(shift_z, path, depth + 1)?,
                xz_scale: *xz_scale,
                y_scale: *y_scale,
                noise: self.compile_noise(noise, path)?,
            }),
            Seed::Spline(spline) => {
                DensityFunction::Spline(self.compile_spline(spline, path, depth + 1)?)
            }
            Seed::WeirdScaled {
                input,
                noise,
                rarity,
            } => DensityFunction::WeirdScaled(WeirdScaledSampler {
                input: self.compile_holder(input, path, depth + 1)?,
                noise: self.compile_noise(noise, path)?,
                rarity: *rarity,
            }),
            Seed::OldBlendedNoise {
                xz_scale,
                y_scale,
                xz_factor,
                y_factor,
                smear,
            } => DensityFunction::OldBlendedNoise(Box::new(BlendedNoise::new(
                self.world_seed,
                *xz_scale,
                *y_scale,
                *xz_factor,
                *y_factor,
                *smear,
            ))),
        };
        self.nodes.push(function);
        Ok(NodeId::try_from(self.nodes.len() - 1).expect("density graphs stay below 2^32 nodes"))
    }
    fn compile_noise(
        &mut self,
        holder: &NoiseHolder,
        path: &str,
    ) -> Result<Arc<NormalNoise>, NoiseDensityError> {
        // `RandomState.java:61-63` requires a key before general instantiation.
        let Holder::Named(reference) = holder else {
            return Err(refuse(
                path,
                "inline noise parameters have no runtime owner",
            ));
        };
        if let Some(noise) = self.noise_cache.get(reference) {
            return Ok(Arc::clone(noise));
        }
        let noise = Arc::new(normal_noise_from_registry(
            self.generation,
            reference,
            self.world_seed,
        )?);
        self.noise_cache
            .insert(reference.clone(), Arc::clone(&noise));
        Ok(noise)
    }
    fn compile_spline(
        &mut self,
        spline: &seed::Spline,
        path: &str,
        depth: usize,
    ) -> Result<Spline, NoiseDensityError> {
        self.check_depth(path, depth)?;
        Ok(match spline {
            seed::Spline::Constant(value) => Spline::Constant(*value),
            seed::Spline::Multipoint { coordinate, points } => Spline::Multipoint {
                coordinate: self.compile_holder(coordinate, path, depth + 1)?,
                points: points
                    .iter()
                    .map(|point| {
                        Ok(SplinePoint {
                            location: point.location,
                            value: self.compile_spline(&point.value, path, depth + 1)?,
                            derivative: point.derivative,
                        })
                    })
                    .collect::<Result<_, NoiseDensityError>>()?,
            },
        })
    }
    fn check_depth(&self, path: &str, depth: usize) -> Result<(), NoiseDensityError> {
        if depth > MAX_DENSITY_DEPTH {
            Err(refuse(path, "density graph exceeds 128 nested nodes"))
        } else {
            Ok(())
        }
    }
}

pub fn normal_noise_from_registry(
    generation: &DatapackGeneration,
    reference: &str,
    world_seed: i64,
) -> Result<NormalNoise, NoiseDensityError> {
    let reference = normalize_resource_id(reference, "noise holder")?;
    let path = format!("{NOISE_REGISTRY}.{reference}");
    let parameters = generation
        .worldgen
        .noises
        .get(&reference)
        .ok_or_else(|| refuse(&path, "noise holder is absent"))?;
    if !parameters
        .amplitudes
        .iter()
        .any(|amplitude| *amplitude != 0.0)
    {
        return Err(refuse(
            &path,
            "noise amplitudes must include a non-zero value",
        ));
    }
    create_normal_noise(
        world_seed,
        &reference,
        parameters.first_octave,
        &parameters.amplitudes,
    )
    .ok_or_else(|| refuse(path, "invalid noise parameters"))
}

pub fn normalize_resource_id(raw: &str, what: &str) -> Result<String, NoiseDensityError> {
    let (namespace, path) = raw.split_once(':').unwrap_or(("minecraft", raw));
    let valid_namespace = !namespace.is_empty()
        && namespace.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        });
    let valid_path = !path.is_empty()
        && path.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b'/')
        });
    if valid_namespace && valid_path && !path.contains(':') {
        Ok(format!("{namespace}:{path}"))
    } else {
        Err(refuse(what, format!("malformed resource id {raw:?}")))
    }
}

fn clamped_lerp(factor: f64, min: f64, max: f64) -> f64 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        min + factor * (max - min)
    }
}

fn lerp(factor: f64, min: f64, max: f64) -> f64 {
    min + factor * (max - min)
}

fn java_min(first: f64, second: f64) -> f64 {
    if first.is_nan() || second.is_nan() {
        f64::NAN
    } else if first == 0.0 && second == 0.0 {
        if first.is_sign_negative() || second.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else if first <= second {
        first
    } else {
        second
    }
}

fn java_max(first: f64, second: f64) -> f64 {
    if first.is_nan() || second.is_nan() {
        f64::NAN
    } else if first == 0.0 && second == 0.0 {
        if first.is_sign_positive() || second.is_sign_positive() {
            0.0
        } else {
            -0.0
        }
    } else if first >= second {
        first
    } else {
        second
    }
}

fn java_clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else {
        java_min(value, max)
    }
}

#[cfg(test)]
pub mod tests;
