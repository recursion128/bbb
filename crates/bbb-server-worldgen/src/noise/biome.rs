//! Built-in multi-noise biome selection.

use std::collections::BTreeSet;
use std::fmt;

use crate::random::WorldgenRandom;
use bbb_protocol::{normalize_resource_location, packets::BlockPos, NbtSummaryValue};
use sha2::{Digest, Sha256};

use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::worldgen::{
    density::{DensityFunction, DensityHolder},
    structure::DecodedMultiNoiseParameterList,
    Holder,
};

use super::density::{ClimateSampler, NoiseDensitySource, Router};
use super::NormalNoise;

pub mod climate;
pub mod direct;
pub mod end;
pub mod overworld;
mod overworld_builder;
mod registry_backed;

use direct::{DirectBiomeSearch, DirectBiomeSource};
use end::EndBiomeSource;
use overworld::{OverworldBiomeSearch, OverworldBiomeSource};
use registry_backed::RegistryBackedBiomeSource;

pub const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
pub const PARAMETER_LIST_REGISTRY: &str =
    "minecraft:worldgen/multi_noise_biome_source_parameter_list";
pub const NOISE_SETTINGS_REGISTRY: &str = "minecraft:worldgen/noise_settings";
pub const NOISE_REGISTRY: &str = "minecraft:worldgen/noise";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiNoisePreset {
    Nether,
    Overworld,
}

impl MultiNoisePreset {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Nether => "minecraft:nether",
            Self::Overworld => "minecraft:overworld",
        }
    }
}

/// Resolved `minecraft:worldgen/multi_noise_biome_source_parameter_list` holder:
/// either one of Vanilla's two built-in preset providers or a datapack-defined
/// climate `biomes` list.
#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedMultiNoisePreset {
    Builtin(MultiNoisePreset),
    Custom(NbtSummaryValue),
}

const NETHER_BIOME_POINTS: [(f32, f32, f32, &str); 5] = [
    (0.0, 0.0, 0.0, "minecraft:nether_wastes"),
    (0.0, -0.5, 0.0, "minecraft:soul_sand_valley"),
    (0.4, 0.0, 0.0, "minecraft:crimson_forest"),
    (0.0, 0.5, 0.375, "minecraft:warped_forest"),
    (-0.5, 0.0, 0.175, "minecraft:basalt_deltas"),
];

/// Resolves the persistent-or-inline holder used by the preset branch of
/// `MultiNoiseBiomeSource.CODEC`; `MultiNoiseBiomeSource.java:18-40` and
/// `MultiNoiseBiomeSourceParameterList.java:23-36,50-82`.
pub fn resolve_multi_noise_preset(
    generation: &DatapackGeneration,
    holder: &NbtSummaryValue,
    path: &str,
) -> Result<MultiNoisePreset, NoiseBiomeError> {
    match resolve_multi_noise_parameter_list(generation, holder, path)? {
        ResolvedMultiNoisePreset::Builtin(preset) => Ok(preset),
        ResolvedMultiNoisePreset::Custom(_) => Err(refuse(
            path,
            "custom multi-noise parameter list has no built-in preset",
        )),
    }
}

/// Resolves either a built-in `overworld`/`nether` preset or a custom datapack
/// `multi_noise_biome_source_parameter_list` (`preset` alias or `biomes` list).
pub fn resolve_multi_noise_parameter_list(
    generation: &DatapackGeneration,
    holder: &NbtSummaryValue,
    path: &str,
) -> Result<ResolvedMultiNoisePreset, NoiseBiomeError> {
    let mut visited = BTreeSet::new();
    match holder {
        NbtSummaryValue::String(raw) => {
            let id = normalize_resource_location(
                raw.try_to_string()
                    .map_err(|error| refuse(path, error.to_string()))?,
            )
            .map_err(|error| refuse(path, error.to_string()))?;
            resolve_registered_parameter_list(generation, &id, path, &mut visited)
        }
        NbtSummaryValue::Compound(fields) => {
            let has_preset = fields.iter().any(|entry| entry.name == "preset");
            let has_biomes = fields.iter().any(|entry| entry.name == "biomes");
            if has_preset == has_biomes {
                return Err(refuse(
                    path,
                    "multi-noise parameter list requires exactly one of preset or biomes",
                ));
            }
            if has_biomes {
                direct::reject_extra(fields, &["biomes"], path)?;
                let biomes = direct::required(fields, "biomes", path)?;
                let biomes_path = format!("{path}.biomes");
                DirectBiomeSource::from_nbt(generation, biomes, &biomes_path)?;
                return Ok(ResolvedMultiNoisePreset::Custom(biomes.clone()));
            }
            direct::reject_extra(fields, &["preset"], path)?;
            let NbtSummaryValue::String(raw) = direct::required(fields, "preset", path)? else {
                return Err(refuse(format!("{path}.preset"), "must be a resource id"));
            };
            resolve_multi_noise_preset_id(
                generation,
                &raw.try_to_string()
                    .map_err(|error| refuse(path, error.to_string()))?,
                &format!("{path}.preset"),
                &mut visited,
            )
        }
        _ => Err(refuse(
            path,
            "must be a named or inline multi-noise parameter-list holder",
        )),
    }
}

fn resolve_registered_parameter_list(
    generation: &DatapackGeneration,
    id: &str,
    path: &str,
    visited: &mut BTreeSet<String>,
) -> Result<ResolvedMultiNoisePreset, NoiseBiomeError> {
    if !visited.insert(id.to_string()) {
        return Err(refuse(path, format!("cyclic multi-noise preset {id}")));
    }
    let source = generation
        .element(PARAMETER_LIST_REGISTRY, id)
        .ok_or_else(|| refuse(path, format!("{PARAMETER_LIST_REGISTRY}.{id} is absent")))?;
    let elem_path = format!("{PARAMETER_LIST_REGISTRY}.{id}");
    let decoded_owned;
    let decoded = match generation
        .worldgen
        .cached_multi_noise_parameter_list(id, &source.source_json)
    {
        Some(Ok(decoded)) => decoded,
        Some(Err(err)) => {
            let err_path = if err.path.is_empty() {
                elem_path
            } else {
                format!("{elem_path}.{}", err.path)
            };
            return Err(refuse(err_path, err.message.clone()));
        }
        None => {
            let parsed = source.decoded_loaded_source(usize::MAX).map_err(|error| {
                refuse(
                    &elem_path,
                    format!("winning source JSON cannot be decoded: {error}"),
                )
            })?;
            let json = parsed
                .json()
                .ok_or_else(|| refuse(&elem_path, "parsed source JSON is absent"))?;
            decoded_owned = DecodedMultiNoiseParameterList::decode(json).map_err(|err| {
                let err_path = if err.path.is_empty() {
                    elem_path.clone()
                } else {
                    format!("{elem_path}.{}", err.path)
                };
                refuse(err_path, err.message)
            })?;
            &decoded_owned
        }
    };
    match decoded {
        DecodedMultiNoiseParameterList::Biomes(biomes_nbt) => {
            let biomes_path = format!("{elem_path}.biomes");
            DirectBiomeSource::from_nbt(generation, biomes_nbt, &biomes_path)?;
            Ok(ResolvedMultiNoisePreset::Custom(biomes_nbt.clone()))
        }
        DecodedMultiNoiseParameterList::Preset(preset) => resolve_multi_noise_preset_id(
            generation,
            preset,
            &format!("{elem_path}.preset"),
            visited,
        ),
    }
}

fn resolve_multi_noise_preset_id(
    generation: &DatapackGeneration,
    raw: &str,
    path: &str,
    visited: &mut BTreeSet<String>,
) -> Result<ResolvedMultiNoisePreset, NoiseBiomeError> {
    let id = normalize_resource_location(raw.to_string())
        .map_err(|error| refuse(path, error.to_string()))?;
    match id.as_str() {
        "minecraft:nether" => Ok(ResolvedMultiNoisePreset::Builtin(MultiNoisePreset::Nether)),
        "minecraft:overworld" => Ok(ResolvedMultiNoisePreset::Builtin(
            MultiNoisePreset::Overworld,
        )),
        _ if generation.element(PARAMETER_LIST_REGISTRY, &id).is_some() => {
            resolve_registered_parameter_list(generation, &id, path, visited)
        }
        _ => Err(refuse(path, format!("unknown multi-noise preset {id}"))),
    }
}

/// `BiomeManager.obfuscateSeed` (`BiomeManager.java:23-25`).
pub fn biome_zoom_seed(seed: i64) -> i64 {
    let digest = Sha256::digest(seed.to_le_bytes());
    i64::from_le_bytes(
        digest[..8]
            .try_into()
            .expect("SHA-256 has eight prefix bytes"),
    )
}

/// Selects the noise-biome quart sampled by `BiomeManager.getBiome`
/// (`BiomeManager.java:31-65`).
pub fn fuzzy_biome_quart_position(
    seed: i64,
    block_x: i32,
    block_y: i32,
    block_z: i32,
) -> (i32, i32, i32) {
    let adjusted_x = block_x.wrapping_sub(2);
    let adjusted_y = block_y.wrapping_sub(2);
    let adjusted_z = block_z.wrapping_sub(2);
    let parent = (adjusted_x >> 2, adjusted_y >> 2, adjusted_z >> 2);
    let fraction = (
        f64::from(adjusted_x & 3) / 4.0,
        f64::from(adjusted_y & 3) / 4.0,
        f64::from(adjusted_z & 3) / 4.0,
    );
    let mut winner = 0;
    let mut best = f64::INFINITY;
    for corner in 0..8 {
        let high_x = corner & 4 != 0;
        let high_y = corner & 2 != 0;
        let high_z = corner & 1 != 0;
        let quart_x = parent.0 + i32::from(high_x);
        let quart_y = parent.1 + i32::from(high_y);
        let quart_z = parent.2 + i32::from(high_z);
        let distance = fiddled_distance(
            seed,
            quart_x,
            quart_y,
            quart_z,
            fraction.0 - f64::from(high_x),
            fraction.1 - f64::from(high_y),
            fraction.2 - f64::from(high_z),
        );
        if best > distance {
            best = distance;
            winner = corner;
        }
    }
    (
        parent.0 + i32::from(winner & 4 != 0),
        parent.1 + i32::from(winner & 2 != 0),
        parent.2 + i32::from(winner & 1 != 0),
    )
}

pub fn lcg_next(value: i64, salt: i64) -> i64 {
    value
        .wrapping_mul(
            value
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407),
        )
        .wrapping_add(salt)
}

pub fn fiddled_distance(seed: i64, x: i32, y: i32, z: i32, dx: f64, dy: f64, dz: f64) -> f64 {
    let mut value = seed;
    for salt in [x, y, z, x, y, z] {
        value = lcg_next(value, i64::from(salt));
    }
    let fx = fiddle(value);
    value = lcg_next(value, seed);
    let fy = fiddle(value);
    value = lcg_next(value, seed);
    let fz = fiddle(value);
    (dz + fz).powi(2) + (dy + fy).powi(2) + (dx + fx).powi(2)
}

pub fn fiddle(value: i64) -> f64 {
    f64::from(((value >> 24).rem_euclid(1024)) as i32) / 1024.0 * 0.9 - 0.45
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseBiomeError {
    path: String,
    message: String,
}

impl fmt::Display for NoiseBiomeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for NoiseBiomeError {}

pub fn refuse(path: impl Into<String>, message: impl Into<String>) -> NoiseBiomeError {
    NoiseBiomeError {
        path: path.into(),
        message: message.into(),
    }
}

#[derive(Clone, Copy)]
pub struct ParameterPoint {
    temperature: i64,
    humidity: i64,
    offset: i64,
    biome: i32,
}

impl ParameterPoint {
    fn fitness(self, temperature: i64, humidity: i64) -> i64 {
        let temperature = self.temperature - temperature;
        let humidity = self.humidity - humidity;
        temperature * temperature + humidity * humidity + self.offset * self.offset
    }

    pub fn magnitude(self) -> i64 {
        self.temperature.abs() + self.humidity.abs() + self.offset.abs()
    }
}

/// Search state is local to one `createBiomes` task, matching the R-tree's
/// thread-local last-result cache without coupling concurrent chunks.
pub struct NetherBiomeSearch {
    last_result: Option<usize>,
}

impl NetherBiomeSearch {
    pub const fn new() -> Self {
        Self { last_result: None }
    }
}

impl Default for NetherBiomeSearch {
    fn default() -> Self {
        Self::new()
    }
}

pub enum NetherBiomeSource {
    Fast {
        temperature: Box<NormalNoise>,
        vegetation: Box<NormalNoise>,
        points: Vec<ParameterPoint>,
        search_order: Vec<usize>,
        registry_len: usize,
        biome_order: Vec<&'static str>,
    },
    Router {
        density: Box<NoiseDensitySource>,
        points: Vec<climate::ParameterPoint>,
        tree: climate::ParameterTree,
        registry_len: usize,
        biome_order: Vec<&'static str>,
    },
}

impl NetherBiomeSource {
    pub fn from_registry(
        generation: &DatapackGeneration,
        settings_id: &str,
        preset_id: &str,
        world_seed: i64,
    ) -> Result<Self, NoiseBiomeError> {
        let preset = resolve_multi_noise_preset(
            generation,
            &NbtSummaryValue::string(preset_id),
            "biome_source.preset",
        )?;
        Self::from_preset(generation, settings_id, preset, world_seed)
    }

    fn from_preset(
        generation: &DatapackGeneration,
        settings_id: &str,
        preset: MultiNoisePreset,
        world_seed: i64,
    ) -> Result<Self, NoiseBiomeError> {
        if preset != MultiNoisePreset::Nether {
            return Err(refuse(
                "biome_source.preset",
                format!("preset {} has no Nether biome runtime owner", preset.id()),
            ));
        }

        let settings_path = format!("{NOISE_SETTINGS_REGISTRY}.{settings_id}");
        let settings = generation
            .worldgen
            .noise_settings
            .get(settings_id)
            .ok_or_else(|| refuse(&settings_path, "noise settings holder is absent"))?;
        let router_path = format!("{settings_path}.noise_router");
        let router = &settings.noise_router;

        let biomes = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| refuse(BIOME_REGISTRY, "registry is absent"))?;
        let mut points = Vec::with_capacity(NETHER_BIOME_POINTS.len());
        let mut climate_points = Vec::with_capacity(NETHER_BIOME_POINTS.len());
        let mut biome_order = Vec::with_capacity(NETHER_BIOME_POINTS.len());
        for (temp, hum, offset, id) in NETHER_BIOME_POINTS {
            if let Some(index) = biomes
                .elements
                .iter()
                .position(|element| element.id == id)
                .and_then(|index| i32::try_from(index).ok())
            {
                points.push(point(temp, hum, offset, index));
                let q_temp = quantize(temp);
                let q_hum = quantize(hum);
                climate_points.push(climate::ParameterPoint::new(
                    [
                        [q_temp, q_temp],
                        [q_hum, q_hum],
                        [0, 0],
                        [0, 0],
                        [0, 0],
                        [0, 0],
                    ],
                    quantize(offset),
                    index,
                ));
                biome_order.push(id);
            }
        }
        if points.is_empty() {
            return Err(refuse(
                BIOME_REGISTRY,
                "has no published built-in Nether biomes",
            ));
        }

        let fast_dimensions_zero = [
            Router::Continents,
            Router::Erosion,
            Router::Depth,
            Router::Ridges,
        ]
        .into_iter()
        .all(|field| is_zero_density(&router[field as usize]));
        if fast_dimensions_zero {
            if let (Ok(temperature), Ok(vegetation)) = (
                compile_shifted_noise(
                    generation,
                    &router[Router::Temperature as usize],
                    "minecraft:nether/temperature",
                    world_seed,
                    &router_path,
                ),
                compile_shifted_noise(
                    generation,
                    &router[Router::Vegetation as usize],
                    "minecraft:nether/vegetation",
                    world_seed,
                    &router_path,
                ),
            ) {
                let mut search_order = (0..points.len()).collect::<Vec<_>>();
                search_order.sort_by_key(|index| points[*index].magnitude());
                return Ok(Self::Fast {
                    temperature: Box::new(temperature),
                    vegetation: Box::new(vegetation),
                    points,
                    search_order,
                    registry_len: biomes.elements.len(),
                    biome_order,
                });
            }
        }

        let density =
            NoiseDensitySource::from_registry_with_climate(generation, settings_id, world_seed)
                .map_err(|error| refuse(&router_path, error.to_string()))?;
        let tree = climate::ParameterTree::new(&climate_points);
        Ok(Self::Router {
            density: Box::new(density),
            points: climate_points,
            tree,
            registry_len: biomes.elements.len(),
            biome_order,
        })
    }

    pub const fn registry_len(&self) -> usize {
        match self {
            Self::Fast { registry_len, .. } | Self::Router { registry_len, .. } => *registry_len,
        }
    }

    fn biome_order(&self) -> &[&'static str] {
        match self {
            Self::Fast { biome_order, .. } | Self::Router { biome_order, .. } => biome_order,
        }
    }

    pub fn sample(
        &self,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
        search: &mut NetherBiomeSearch,
    ) -> i32 {
        match self {
            Self::Fast {
                temperature,
                vegetation,
                points,
                search_order,
                ..
            } => {
                let block_x = quart_x.wrapping_mul(4);
                let block_y = quart_y.wrapping_mul(4);
                let block_z = quart_z.wrapping_mul(4);
                let coordinates = (
                    f64::from(block_x) * 0.25,
                    f64::from(block_y) * 0.0,
                    f64::from(block_z) * 0.25,
                );
                let temperature =
                    quantize(
                        temperature.get_value(coordinates.0, coordinates.1, coordinates.2) as f32,
                    );
                let vegetation =
                    quantize(
                        vegetation.get_value(coordinates.0, coordinates.1, coordinates.2) as f32,
                    );
                let mut winner = search.last_result;
                let mut best = winner.map_or(i64::MAX, |index| {
                    points[index].fitness(temperature, vegetation)
                });
                for &index in search_order {
                    let fitness = points[index].fitness(temperature, vegetation);
                    if fitness < best {
                        best = fitness;
                        winner = Some(index);
                    }
                }
                let winner = winner.expect("the built-in Nether parameter list is non-empty");
                search.last_result = Some(winner);
                points[winner].biome
            }
            Self::Router {
                density,
                points,
                tree,
                ..
            } => {
                let target = density
                    .climate_target(quart_x, quart_y, quart_z)
                    .expect("Nether router was compiled with a climate sampler");
                let winner = tree.search(points, &target, &mut search.last_result);
                points[winner].biome
            }
        }
    }
}

pub enum NoiseBiomeSearch {
    Direct(DirectBiomeSearch),
    End,
    Nether(NetherBiomeSearch),
    Overworld(OverworldBiomeSearch),
    RegistryBacked,
}

pub enum NoiseBiomeSource {
    Direct(DirectBiomeSource),
    End(EndBiomeSource),
    Nether(NetherBiomeSource),
    Overworld(OverworldBiomeSource),
    RegistryBacked(RegistryBackedBiomeSource),
}

impl NoiseBiomeSource {
    pub fn direct_from_nbt(
        generation: &DatapackGeneration,
        value: &bbb_protocol::NbtSummaryValue,
        path: &str,
    ) -> Result<Self, NoiseBiomeError> {
        DirectBiomeSource::from_nbt(generation, value, path).map(Self::Direct)
    }

    pub fn from_registry(
        generation: &DatapackGeneration,
        settings_id: &str,
        preset_id: &str,
        world_seed: i64,
    ) -> Result<Self, NoiseBiomeError> {
        let preset = resolve_multi_noise_parameter_list(
            generation,
            &NbtSummaryValue::string(preset_id),
            "biome_source.preset",
        )?;
        Self::from_resolved_preset(
            generation,
            settings_id,
            &preset,
            world_seed,
            "biome_source.preset",
        )
    }

    pub fn from_resolved_preset(
        generation: &DatapackGeneration,
        settings_id: &str,
        preset: &ResolvedMultiNoisePreset,
        world_seed: i64,
        path: &str,
    ) -> Result<Self, NoiseBiomeError> {
        match preset {
            ResolvedMultiNoisePreset::Builtin(preset) => {
                Self::from_preset(generation, settings_id, *preset, world_seed)
            }
            ResolvedMultiNoisePreset::Custom(biomes) => {
                Self::direct_from_nbt(generation, biomes, &format!("{path}.biomes"))
            }
        }
    }

    pub fn from_preset(
        generation: &DatapackGeneration,
        settings_id: &str,
        preset: MultiNoisePreset,
        world_seed: i64,
    ) -> Result<Self, NoiseBiomeError> {
        match preset {
            MultiNoisePreset::Nether => {
                NetherBiomeSource::from_preset(generation, settings_id, preset, world_seed)
                    .map(Self::Nether)
            }
            MultiNoisePreset::Overworld => {
                OverworldBiomeSource::from_preset(generation, settings_id, preset)
                    .map(Self::Overworld)
            }
        }
    }

    pub fn end_from_registry(generation: &DatapackGeneration) -> Result<Self, NoiseBiomeError> {
        EndBiomeSource::from_registry(generation).map(Self::End)
    }

    pub fn registry_backed_from_nbt(
        generation: &DatapackGeneration,
        tags: &bbb_server_datapack::tag_loader::TagLoadReport,
        value: &bbb_protocol::NbtSummaryValue,
        path: &str,
    ) -> Result<Self, NoiseBiomeError> {
        RegistryBackedBiomeSource::from_nbt(generation, tags, value, path).map(Self::RegistryBacked)
    }

    pub const fn registry_len(&self) -> usize {
        match self {
            Self::Direct(source) => source.registry_len(),
            Self::End(source) => source.registry_len(),
            Self::Nether(source) => source.registry_len(),
            Self::Overworld(source) => source.registry_len(),
            Self::RegistryBacked(source) => source.registry_len(),
        }
    }

    pub fn overworld_biome_order(&self) -> Option<&'static [&'static str]> {
        match self {
            Self::Direct(_) | Self::End(_) | Self::Nether(_) | Self::RegistryBacked(_) => None,
            Self::Overworld(_) => Some(OverworldBiomeSource::biome_order()),
        }
    }

    pub const fn end_biome_order(&self) -> Option<&'static [&'static str]> {
        match self {
            Self::End(_) => Some(EndBiomeSource::biome_order()),
            Self::Direct(_) | Self::Nether(_) | Self::Overworld(_) | Self::RegistryBacked(_) => {
                None
            }
        }
    }

    pub fn nether_biome_order(&self) -> Option<&[&'static str]> {
        match self {
            Self::Nether(source) => Some(source.biome_order()),
            Self::Direct(_) | Self::End(_) | Self::Overworld(_) | Self::RegistryBacked(_) => None,
        }
    }

    /// `BiomeSource.java:29-41`; `MultiNoiseBiomeSource.java:18-45`.
    pub fn possible_biome_order(&self) -> Vec<&str> {
        match self {
            Self::Direct(source) => source
                .possible_biomes()
                .iter()
                .map(String::as_str)
                .collect(),
            Self::End(_) => EndBiomeSource::biome_order().to_vec(),
            Self::Nether(source) => source.biome_order().to_vec(),
            Self::Overworld(_) => OverworldBiomeSource::biome_order().to_vec(),
            Self::RegistryBacked(source) => source.possible_biomes(),
        }
    }

    pub fn create_search(&self) -> NoiseBiomeSearch {
        match self {
            Self::Direct(_) => NoiseBiomeSearch::Direct(DirectBiomeSearch::new()),
            Self::End(_) => NoiseBiomeSearch::End,
            Self::Nether(_) => NoiseBiomeSearch::Nether(NetherBiomeSearch::new()),
            Self::Overworld(_) => NoiseBiomeSearch::Overworld(OverworldBiomeSearch::new()),
            Self::RegistryBacked(_) => NoiseBiomeSearch::RegistryBacked,
        }
    }

    pub fn sample(
        &self,
        density: &NoiseDensitySource,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
        search: &mut NoiseBiomeSearch,
    ) -> i32 {
        let mut climate = density;
        self.sample_with(&mut climate, quart_x, quart_y, quart_z, search)
    }

    /// `BiomeSource.getNoiseBiome` with the caller's `Climate.Sampler`.
    pub fn sample_with(
        &self,
        climate: &mut impl ClimateSampler,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
        search: &mut NoiseBiomeSearch,
    ) -> i32 {
        match (self, search) {
            (Self::Direct(source), NoiseBiomeSearch::Direct(search)) => {
                source.sample(climate, quart_x, quart_y, quart_z, search)
            }
            (Self::End(source), NoiseBiomeSearch::End) => {
                source.sample(climate, quart_x, quart_y, quart_z)
            }
            (Self::Nether(source), NoiseBiomeSearch::Nether(search)) => {
                source.sample(quart_x, quart_y, quart_z, search)
            }
            (Self::Overworld(source), NoiseBiomeSearch::Overworld(search)) => {
                source.sample(climate, quart_x, quart_y, quart_z, search)
            }
            (Self::RegistryBacked(source), NoiseBiomeSearch::RegistryBacked) => {
                source.sample(quart_x, quart_z)
            }
            _ => unreachable!("search state is created by its biome source"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn find_closest_biome_3d(
        &self,
        density: &NoiseDensitySource,
        origin: BlockPos,
        allowed: &BTreeSet<i32>,
        search_radius: i32,
        horizontal_step: i32,
        vertical_step: i32,
        min_search_y: i32,
        max_search_y: i32,
    ) -> Option<(BlockPos, i32)> {
        if let Self::RegistryBacked(source) = self {
            if let Some(biome) = source.fixed_biome() {
                // `FixedBiomeSource.java:66-75`.
                return allowed.contains(&biome).then(|| {
                    (
                        BlockPos {
                            y: origin.y.clamp(min_search_y, max_search_y),
                            ..origin
                        },
                        biome,
                    )
                });
            }
        }
        let mut search = self.create_search();
        find_closest_biome_3d_by(
            origin,
            allowed,
            search_radius,
            horizontal_step,
            vertical_step,
            min_search_y,
            max_search_y,
            |quart_x, quart_y, quart_z| {
                self.sample(density, quart_x, quart_y, quart_z, &mut search)
            },
        )
    }

    /// The non-closest, unit-step path used by structure rings in
    /// `BiomeSource.findBiomeHorizontal` (`BiomeSource.java:117-174`).
    #[allow(clippy::too_many_arguments)]
    pub fn find_biome_horizontal(
        &self,
        density: &NoiseDensitySource,
        origin_x: i32,
        origin_y: i32,
        origin_z: i32,
        search_radius: i32,
        allowed: &BTreeSet<i32>,
        random: &mut WorldgenRandom,
    ) -> Option<(i32, i32, i32)> {
        if search_radius < 0 || allowed.is_empty() {
            return None;
        }
        if let Self::RegistryBacked(source) = self {
            if let Some(biome) = source.fixed_biome() {
                if !allowed.contains(&biome) {
                    return None;
                }
                return find_fixed_biome_horizontal(
                    origin_x,
                    origin_y,
                    origin_z,
                    search_radius,
                    random,
                );
            }
        }
        let center_x = origin_x >> 2;
        let center_z = origin_z >> 2;
        let radius = search_radius >> 2;
        let quart_y = origin_y >> 2;
        let mut search = self.create_search();
        let mut result = None;
        let mut found = 0;
        for offset_z in -radius..=radius {
            for offset_x in -radius..=radius {
                let quart_x = center_x.wrapping_add(offset_x);
                let quart_z = center_z.wrapping_add(offset_z);
                let biome = self.sample(density, quart_x, quart_y, quart_z, &mut search);
                if allowed.contains(&biome) {
                    if result.is_none() || random.next_int(found + 1) == 0 {
                        result = Some((quart_x.wrapping_mul(4), origin_y, quart_z.wrapping_mul(4)));
                    }
                    found += 1;
                }
            }
        }
        result
    }
}

// `BiomeSource.java:82-115`; `BlockPos.java:429-462`; `Mth.java:708-725`.
#[allow(clippy::too_many_arguments)]
fn find_closest_biome_3d_by(
    origin: BlockPos,
    allowed: &BTreeSet<i32>,
    search_radius: i32,
    horizontal_step: i32,
    vertical_step: i32,
    min_search_y: i32,
    max_search_y: i32,
    mut sample: impl FnMut(i32, i32, i32) -> i32,
) -> Option<(BlockPos, i32)> {
    if allowed.is_empty()
        || search_radius < 0
        || horizontal_step < 1
        || vertical_step < 1
        || min_search_y > max_search_y
    {
        return None;
    }
    let sample_radius = search_radius.div_euclid(horizontal_step);
    let sample_ys = out_from_origin(origin.y, min_search_y, max_search_y, vertical_step);
    for (offset_x, offset_z) in SpiralAround::new(sample_radius) {
        let block_x = origin
            .x
            .wrapping_add(offset_x.wrapping_mul(horizontal_step));
        let block_z = origin
            .z
            .wrapping_add(offset_z.wrapping_mul(horizontal_step));
        let quart_x = block_x.div_euclid(4);
        let quart_z = block_z.div_euclid(4);
        for &block_y in &sample_ys {
            let biome = sample(quart_x, block_y.div_euclid(4), quart_z);
            if allowed.contains(&biome) {
                return Some((
                    BlockPos {
                        x: block_x,
                        y: block_y,
                        z: block_z,
                    },
                    biome,
                ));
            }
        }
    }
    None
}

fn out_from_origin(origin: i32, lower: i32, upper: i32, step: i32) -> Vec<i32> {
    let center = origin.clamp(lower, upper);
    let mut values = vec![center];
    let mut distance = i64::from(step);
    loop {
        let mut added = false;
        let positive = i64::from(center) + distance;
        if positive <= i64::from(upper) {
            values.push(positive as i32);
            added = true;
        }
        let negative = i64::from(center) - distance;
        if negative >= i64::from(lower) {
            values.push(negative as i32);
            added = true;
        }
        if !added {
            break;
        }
        distance += i64::from(step);
    }
    values
}

struct SpiralAround {
    directions: [(i32, i32); 4],
    cursor: (i32, i32),
    legs: i64,
    leg: i64,
    leg_size: i64,
    leg_index: i64,
}

impl SpiralAround {
    fn new(radius: i32) -> Self {
        Self {
            directions: [(1, 0), (0, 1), (-1, 0), (0, -1)],
            cursor: (0, 1),
            legs: i64::from(radius) * 4,
            leg: -1,
            leg_size: 0,
            leg_index: 0,
        }
    }
}

impl Iterator for SpiralAround {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        let direction = self.directions[(self.leg + 4).rem_euclid(4) as usize];
        self.cursor.0 = self.cursor.0.wrapping_add(direction.0);
        self.cursor.1 = self.cursor.1.wrapping_add(direction.1);
        if self.leg_index >= self.leg_size {
            if self.leg >= self.legs {
                return None;
            }
            self.leg += 1;
            self.leg_index = 0;
            self.leg_size = self.leg / 2 + 1;
        }
        self.leg_index += 1;
        Some(self.cursor)
    }
}

pub fn point(temperature: f32, humidity: f32, offset: f32, biome: i32) -> ParameterPoint {
    ParameterPoint {
        temperature: quantize(temperature),
        humidity: quantize(humidity),
        offset: quantize(offset),
        biome,
    }
}

fn is_zero_density(holder: &DensityHolder) -> bool {
    matches!(holder, Holder::Inline(value) if matches!(value.as_ref(), DensityFunction::Constant(0.0)))
}

fn compile_shifted_noise(
    generation: &DatapackGeneration,
    value: &DensityHolder,
    expected_noise: &str,
    world_seed: i64,
    path: &str,
) -> Result<NormalNoise, NoiseBiomeError> {
    let Holder::Inline(value) = value else {
        return Err(refuse(path, "Nether climate requires shifted noise"));
    };
    let DensityFunction::ShiftedNoise {
        shift_x,
        shift_y,
        shift_z,
        xz_scale,
        y_scale,
        noise,
    } = value.as_ref()
    else {
        return Err(refuse(path, "Nether climate requires shifted noise"));
    };
    if ![shift_x, shift_y, shift_z].into_iter().all(is_zero_density)
        || *xz_scale != 0.25
        || *y_scale != 0.0
    {
        return Err(refuse(
            path,
            "Nether climate requires zero shifts and scales xz=0.25, y=0",
        ));
    }
    if !matches!(noise, Holder::Named(id) if id == expected_noise) {
        return Err(refuse(
            path,
            format!("expected noise holder {expected_noise}"),
        ));
    }
    super::density::normal_noise_from_registry(generation, expected_noise, world_seed)
        .map_err(|error| refuse(error.path, error.message))
}

fn quantize(value: f32) -> i64 {
    // Climate.quantizeCoord, Climate.java:75-77.
    (value * 10_000.0_f32) as i64
}

#[cfg(test)]
mod custom_parameter_list_tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use bbb_protocol::NbtSummaryValue;
    use bbb_server_datapack::dynamic_registry::RawRegistryElement;
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::{
        resolve_multi_noise_parameter_list, NoiseBiomeSearch, NoiseBiomeSource,
        ResolvedMultiNoisePreset, BIOME_REGISTRY, NOISE_SETTINGS_REGISTRY, PARAMETER_LIST_REGISTRY,
    };
    use crate::noise::density::NoiseDensitySource;

    fn builtin_startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    #[test]
    fn custom_multi_noise_parameter_list_resolves_and_samples_climates() {
        let mut startup = builtin_startup();
        let custom_json = serde_json::json!({
            "biomes": [
                {
                    "biome": "minecraft:plains",
                    "parameters": {
                        "temperature": [-2.0, 0.0],
                        "humidity": [-2.0, 2.0],
                        "continentalness": [-2.0, 2.0],
                        "erosion": [-2.0, 2.0],
                        "depth": [-2.0, 2.0],
                        "weirdness": [-2.0, 2.0],
                        "offset": 0.0
                    }
                },
                {
                    "biome": "minecraft:desert",
                    "parameters": {
                        "temperature": [0.0, 2.0],
                        "humidity": [-2.0, 2.0],
                        "continentalness": [-2.0, 2.0],
                        "erosion": [-2.0, 2.0],
                        "depth": [-2.0, 2.0],
                        "weirdness": [-2.0, 2.0],
                        "offset": 0.0
                    }
                }
            ]
        });
        let alias_json = serde_json::json!({
            "preset": "example:split_climates"
        });
        let registry = startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == PARAMETER_LIST_REGISTRY)
            .unwrap();
        for (id, raw_json) in [
            ("example:split_climates", custom_json),
            ("example:alias_climates", alias_json),
        ] {
            let bytes = Arc::<[u8]>::from(serde_json::to_vec(&raw_json).unwrap());
            let decoded = bbb_registry_codec::decode_loaded_registry_json(
                &bytes,
                DatapackLoadLimits::default().max_element_bytes,
            )
            .unwrap();
            registry.elements.push(RawRegistryElement {
                id: id.to_string(),
                source_pack: "file/custom_param_list".to_string(),
                source_json: bytes,
                source: Some(decoded),
                codec_value: None,
            });
        }

        let biomes = startup.registries.registry(BIOME_REGISTRY).unwrap();
        let plains_idx = biomes
            .elements
            .iter()
            .position(|element| element.id == "minecraft:plains")
            .unwrap() as i32;
        let desert_idx = biomes
            .elements
            .iter()
            .position(|element| element.id == "minecraft:desert")
            .unwrap() as i32;
        let allowed = BTreeSet::from([plains_idx, desert_idx]);

        let density = NoiseDensitySource::from_registry_with_climate(
            &startup.registries,
            "minecraft:overworld",
            1234,
        )
        .unwrap();

        for holder_id in ["example:split_climates", "example:alias_climates"] {
            let resolved = resolve_multi_noise_parameter_list(
                &startup.registries,
                &NbtSummaryValue::string(holder_id),
                "biome_source.preset",
            )
            .unwrap();
            assert!(matches!(resolved, ResolvedMultiNoisePreset::Custom(_)));

            let source = NoiseBiomeSource::from_resolved_preset(
                &startup.registries,
                "minecraft:overworld",
                &resolved,
                1234,
                "biome_source.preset",
            )
            .unwrap();
            assert_eq!(
                source.possible_biome_order(),
                vec!["minecraft:plains", "minecraft:desert"]
            );
            let mut search = source.create_search();
            assert!(matches!(search, NoiseBiomeSearch::Direct(_)));
            for qx in -8..=8 {
                let sampled = source.sample(&density, qx * 8, 16, qx * -5, &mut search);
                assert!(allowed.contains(&sampled));
            }
        }
    }

    #[test]
    fn custom_nether_noise_router_falls_back_to_climate_sampler() {
        let mut startup = builtin_startup();
        let settings = startup
            .registries
            .worldgen
            .noise_settings
            .get_mut("minecraft:nether")
            .unwrap();
        // Replace zero depth with a non-zero expression and point temperature at depth.
        let mut updated = (**settings).clone();
        updated.noise_router[super::Router::Depth as usize] =
            bbb_server_datapack::worldgen::density::DensityHolder::Inline(
                bbb_server_datapack::worldgen::density::DensityFunction::Constant(0.25).into(),
            );
        updated.noise_router[super::Router::Temperature as usize] =
            bbb_server_datapack::worldgen::density::DensityHolder::Inline(
                bbb_server_datapack::worldgen::density::DensityFunction::Constant(-0.5).into(),
            );
        *settings = Arc::new(updated);
        let _ = NOISE_SETTINGS_REGISTRY;

        let source = NoiseBiomeSource::from_registry(
            &startup.registries,
            "minecraft:nether",
            "minecraft:nether",
            0,
        )
        .unwrap();
        let mut search = source.create_search();
        assert!(matches!(search, NoiseBiomeSearch::Nether(_)));
        let density =
            NoiseDensitySource::from_registry(&startup.registries, "minecraft:nether", 0).unwrap();
        let basalt_idx = startup
            .registries
            .registry(BIOME_REGISTRY)
            .unwrap()
            .elements
            .iter()
            .position(|element| element.id == "minecraft:basalt_deltas")
            .unwrap() as i32;
        assert_eq!(source.sample(&density, 0, 8, 0, &mut search), basalt_idx);
    }
}

#[cfg(test)]
mod zoom_tests {
    use super::{biome_zoom_seed, fiddle, fuzzy_biome_quart_position};

    #[test]
    fn fuzzy_zoom_matches_vanilla_reference_coordinates() {
        let seed = biome_zoom_seed(42);
        assert_eq!(seed, -4_111_196_313_959_201_555);
        assert_eq!(fuzzy_biome_quart_position(seed, 0, 0, 0), (0, 0, 0));
        assert_eq!(fuzzy_biome_quart_position(seed, 16, -64, 32), (4, -17, 7));
        assert_eq!(
            fuzzy_biome_quart_position(seed, i32::MAX, i32::MIN, 0),
            (536_870_912, 536_870_911, 0)
        );
        assert!((fiddle(0) + 0.45).abs() < f64::EPSILON);
    }
}

#[cfg(test)]
mod locate_biome_tests {
    use std::collections::BTreeSet;

    use bbb_protocol::packets::BlockPos;

    use super::{find_closest_biome_3d_by, out_from_origin};

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    #[test]
    fn locate_biome_spiral_returns_first_scan_match_not_nearest_distance() {
        let found = find_closest_biome_3d_by(
            pos(0, 0, 0),
            &BTreeSet::from([9]),
            4,
            4,
            1,
            0,
            0,
            |quart_x, _, quart_z| i32::from(matches!((quart_x, quart_z), (1, 1) | (-1, 0))) * 9,
        );
        assert_eq!(found, Some((pos(4, 0, 4), 9)));
    }

    #[test]
    fn locate_biome_y_scan_prefers_positive_then_negative_offsets() {
        assert_eq!(out_from_origin(0, -128, 128, 64), [0, 64, -64, 128, -128]);
        let found = find_closest_biome_3d_by(
            pos(0, 0, 0),
            &BTreeSet::from([7]),
            0,
            32,
            64,
            -128,
            128,
            |_, quart_y, _| i32::from(matches!(quart_y, 16 | -16)) * 7,
        );
        assert_eq!(found, Some((pos(0, 64, 0), 7)));
    }

    #[test]
    fn locate_biome_clamps_y_and_floor_divides_negative_blocks() {
        let high = find_closest_biome_3d_by(
            pos(-1, 999, -1),
            &BTreeSet::from([5]),
            0,
            32,
            64,
            -63,
            320,
            |quart_x, quart_y, quart_z| i32::from((quart_x, quart_y, quart_z) == (-1, 80, -1)) * 5,
        );
        assert_eq!(high, Some((pos(-1, 320, -1), 5)));

        let low = find_closest_biome_3d_by(
            pos(-1, -999, -1),
            &BTreeSet::from([5]),
            0,
            32,
            64,
            -63,
            320,
            |quart_x, quart_y, quart_z| i32::from((quart_x, quart_y, quart_z) == (-1, -16, -1)) * 5,
        );
        assert_eq!(low, Some((pos(-1, -63, -1), 5)));
    }
}

// FixedBiomeSource.java:45-63: fixed-biome ring relocation consumes two block-coordinate draws.
pub fn find_fixed_biome_horizontal(
    x: i32,
    y: i32,
    z: i32,
    radius: i32,
    random: &mut WorldgenRandom,
) -> Option<(i32, i32, i32)> {
    let bound = radius.checked_mul(2)?.checked_add(1)?;
    Some((
        x.wrapping_sub(radius).wrapping_add(random.next_int(bound)),
        y,
        z.wrapping_sub(radius).wrapping_add(random.next_int(bound)),
    ))
}
