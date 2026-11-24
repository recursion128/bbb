//! Built-in Overworld and Nether `SurfaceSystem` and executable surface-rule tree.

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_registry_codec::biome_climate::{height_adjusted_temperature, TemperatureModifier};
use bbb_server_datapack::worldgen::surface::{SurfaceCondition, SurfaceRule};
use bbb_server_datapack::worldgen::VerticalAnchor as Anchor;

use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::fluid::{
    block_state_blocks_motion, block_state_fluid_state, block_state_is_air,
};

use super::biome::{
    biome_zoom_seed, fuzzy_biome_quart_position, NoiseBiomeSearch, NoiseBiomeSource,
};
use super::density::{normal_noise_from_registry, NoiseDensityError, NoiseDensitySource};
use super::NormalNoise;
use crate::random::{PositionalRandom, WorldgenRandom};
use bbb_server_datapack::worldgen::noise::NOISE_SETTINGS_REGISTRY;

pub const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

#[derive(Clone)]
pub enum Rule {
    Bandlands,
    Block(i32),
    Sequence(Vec<Self>),
    Condition(Condition, Box<Self>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BiomeSet {
    bits: [u64; 2],
    extra: Box<[i32]>,
}

impl BiomeSet {
    pub fn from_ids(ids: impl IntoIterator<Item = i32>) -> Self {
        let mut bits = [0_u64; 2];
        let mut extra = Vec::new();
        for id in ids {
            let idx = id as u32;
            if idx < 64 {
                bits[0] |= 1_u64 << idx;
            } else if idx < 128 {
                bits[1] |= 1_u64 << (idx - 64);
            } else if !extra.contains(&id) {
                extra.push(id);
            }
        }
        extra.sort_unstable();
        Self {
            bits,
            extra: extra.into_boxed_slice(),
        }
    }

    #[inline]
    pub fn contains(&self, biome: i32) -> bool {
        let idx = biome as u32;
        if idx < 64 {
            (self.bits[0] & (1_u64 << idx)) != 0
        } else if idx < 128 {
            (self.bits[1] & (1_u64 << (idx - 64))) != 0
        } else {
            self.extra.binary_search(&biome).is_ok()
        }
    }
}

#[derive(Clone)]
pub enum Condition {
    AbovePreliminarySurface,
    Biome(BiomeSet),
    Hole,
    NoiseThreshold {
        slot: usize,
        noise: Arc<NormalNoise>,
        min: f64,
        max: f64,
    },
    Not(Box<Self>),
    Steep,
    StoneDepth {
        offset: i32,
        add_surface_depth: bool,
        secondary_depth_range: i32,
        ceiling: bool,
    },
    Temperature,
    VerticalGradient {
        random: PositionalRandom,
        true_at_and_below: Anchor,
        false_at_and_above: Anchor,
    },
    Water {
        offset: i32,
        surface_depth_multiplier: i32,
        add_stone_depth: bool,
    },
    YAbove {
        anchor: Anchor,
        surface_depth_multiplier: i32,
        add_stone_depth: bool,
    },
}

#[derive(Clone, Copy)]
pub struct BiomeFacts {
    temperature: f32,
    modifier: TemperatureModifier,
}

pub struct NoiseSurfaceProgram {
    rule: Rule,
    noise_threshold_count: usize,
    root_random: PositionalRandom,
    clay_bands: [i32; 192],
    clay_bands_offset: NormalNoise,
    surface: NormalNoise,
    surface_secondary: NormalNoise,
    badlands_pillar: NormalNoise,
    badlands_pillar_roof: NormalNoise,
    badlands_surface: NormalNoise,
    iceberg_pillar: NormalNoise,
    iceberg_pillar_roof: NormalNoise,
    iceberg_surface: NormalNoise,
    biome_facts: Vec<BiomeFacts>,
    biome_zoom_seed: i64,
    eroded_badlands: Option<i32>,
    frozen_ocean: Option<i32>,
    deep_frozen_ocean: Option<i32>,
    default_block: i32,
    air: i32,
    water: i32,
    snow_block: i32,
    packed_ice: i32,
    sea_level: i32,
    min_y: i32,
    height: i32,
    legacy_random_source: bool,
}

impl NoiseSurfaceProgram {
    pub fn from_registry(
        generation: &DatapackGeneration,
        settings_id: &str,
        world_seed: i64,
        source: &NoiseDensitySource,
    ) -> Result<Self, NoiseDensityError> {
        let path = format!("{NOISE_SETTINGS_REGISTRY}.{settings_id}");
        let settings = generation
            .worldgen
            .noise_settings
            .get(settings_id)
            .ok_or_else(|| error(&path, "noise settings holder is absent"))?;
        // The resolved settings holder supplies the rule; its resource id does not gate SURFACE.
        // `NoiseBasedChunkGenerator.java:267-295`.
        let root_random = PositionalRandom::new(world_seed, "minecraft:surface");
        let (biome_facts, biome_ids) = parse_biome_facts(generation)?;
        // `SurfaceSystem.java:113,158` checks `Holder.is` against optional biome ResourceKeys.
        let eroded_badlands = biome_ids.get("minecraft:eroded_badlands").copied();
        let frozen_ocean = biome_ids.get("minecraft:frozen_ocean").copied();
        let deep_frozen_ocean = biome_ids.get("minecraft:deep_frozen_ocean").copied();
        let mut compiler = Compiler {
            generation,
            world_seed,
            biome_ids,
            noises: crate::fast_hash::FastHashMap::default(),
            noise_threshold_count: 0,
        };
        let rule = compiler.compile_rule(&settings.surface_rule)?;
        let noise_threshold_count = compiler.noise_threshold_count;
        let noise = |id: &str| normal_noise_from_registry(generation, id, world_seed);
        let state = |name: &str| {
            vanilla_block_type_for_resource_id(name)
                .and_then(|block| vanilla_block_default_state_id(block.id))
                .ok_or_else(|| error(&path, format!("default block state for {name} is absent")))
        };
        let mut band_random = WorldgenRandom::named(world_seed, "minecraft:clay_bands");
        let clay_bands = generate_bands(
            &mut band_random,
            state("minecraft:terracotta")?,
            state("minecraft:orange_terracotta")?,
            state("minecraft:yellow_terracotta")?,
            state("minecraft:brown_terracotta")?,
            state("minecraft:red_terracotta")?,
            state("minecraft:white_terracotta")?,
            state("minecraft:light_gray_terracotta")?,
        );
        Ok(Self {
            rule,
            noise_threshold_count,
            root_random,
            clay_bands,
            clay_bands_offset: noise("minecraft:clay_bands_offset")?,
            surface: noise("minecraft:surface")?,
            surface_secondary: noise("minecraft:surface_secondary")?,
            badlands_pillar: noise("minecraft:badlands_pillar")?,
            badlands_pillar_roof: noise("minecraft:badlands_pillar_roof")?,
            badlands_surface: noise("minecraft:badlands_surface")?,
            iceberg_pillar: noise("minecraft:iceberg_pillar")?,
            iceberg_pillar_roof: noise("minecraft:iceberg_pillar_roof")?,
            iceberg_surface: noise("minecraft:iceberg_surface")?,
            biome_facts,
            biome_zoom_seed: biome_zoom_seed(world_seed),
            eroded_badlands,
            frozen_ocean,
            deep_frozen_ocean,
            default_block: source.default_block_state_id(),
            air: source.air_block_state_id(),
            water: source.water_block_state_id(),
            snow_block: state("minecraft:snow_block")?,
            packed_ice: state("minecraft:packed_ice")?,
            sea_level: source.sea_level(),
            min_y: source.min_y(),
            height: source.height(),
            legacy_random_source: source.legacy_random_source(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
    ) {
        self.apply_with_biome_override(
            source,
            biome_source,
            chunk_x,
            chunk_z,
            column,
            ocean_floor_wg,
            world_surface_wg,
            postprocessing,
            |_, _, _| None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_with_biome_override(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
        mut biome_override: impl FnMut(i32, i32, i32) -> Option<i32>,
    ) {
        self.apply_with_biome_override_and_generation_shape(
            source,
            biome_source,
            chunk_x,
            chunk_z,
            column,
            ocean_floor_wg,
            world_surface_wg,
            postprocessing,
            &mut biome_override,
            self.min_y,
            self.height,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_retrogen(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
        mut biome_override: impl FnMut(i32, i32, i32) -> Option<i32>,
    ) {
        self.apply_with_biome_override_and_generation_shape(
            source,
            biome_source,
            chunk_x,
            chunk_z,
            column,
            ocean_floor_wg,
            world_surface_wg,
            postprocessing,
            &mut biome_override,
            -64,
            64,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_with_biome_override_and_generation_shape(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
        biome_override: &mut dyn FnMut(i32, i32, i32) -> Option<i32>,
        generation_min_y: i32,
        generation_height: i32,
    ) {
        let start_x = chunk_x * 16;
        let start_z = chunk_z * 16;
        let mut search = biome_source.create_search();
        let mut preliminary_corners: Option<[i32; 4]> = None;
        let mut noise_thresholds = vec![None; self.noise_threshold_count];
        let mut halo_biomes = crate::fast_hash::FastHashMap::default();
        let mut cached_biome_override = |quart_x: i32, quart_y: i32, quart_z: i32| -> Option<i32> {
            if let Some(biome) = biome_override(quart_x, quart_y, quart_z) {
                return Some(biome);
            }
            Some(
                *halo_biomes
                    .entry((quart_x, quart_y, quart_z))
                    .or_insert_with(|| {
                        biome_source.sample(source, quart_x, quart_y, quart_z, &mut search)
                    }),
            )
        };
        // `SurfaceSystem.buildSurface` (`SurfaceSystem.java:66-163`).
        for local_x in 0..16 {
            for local_z in 0..16 {
                let x = start_x + local_x;
                let z = start_z + local_z;
                let index = (local_x + local_z * 16) as usize;
                let starting_height = world_surface_wg[index];
                // `SurfaceSystem.buildSurface` looks the extension biome up at
                // y=0 under legacy settings (`SurfaceSystem.java:110-112`).
                let (surf_qx, surf_qy, surf_qz) = fuzzy_biome_quart_position(
                    self.biome_zoom_seed,
                    x,
                    if self.legacy_random_source {
                        0
                    } else {
                        starting_height
                    },
                    z,
                );
                let surface_biome = cached_biome_override(surf_qx, surf_qy, surf_qz)
                    .expect("cached biome lookup always produces a biome");
                if Some(surface_biome) == self.eroded_badlands {
                    self.eroded_badlands_extension(
                        column,
                        ocean_floor_wg,
                        world_surface_wg,
                        postprocessing,
                        local_x,
                        local_z,
                        x,
                        z,
                        starting_height,
                    );
                }
                let height = world_surface_wg[index];
                let surface_depth = self.surface_depth(x, z);
                noise_thresholds.fill(None);
                let mut context = Context {
                    program: self,
                    source,
                    biome_source,
                    search: None,
                    biome_override: &mut cached_biome_override,
                    world_surface_wg,
                    preliminary_corners: &mut preliminary_corners,
                    noise_thresholds: &mut noise_thresholds,
                    x,
                    z,
                    local_x,
                    local_z,
                    y: height,
                    surface_depth,
                    surface_secondary: None,
                    min_surface_level: None,
                    water_height: i32::MIN,
                    stone_depth_above: 0,
                    stone_depth_below: 0,
                    biome: None,
                    generation_min_y,
                    generation_height,
                };
                let mut next_ceiling_stone_y = i32::MAX;
                for y in (self.min_y..=height).rev() {
                    let old = column
                        .get_block_state(local_x, y, local_z)
                        .unwrap_or(self.air);
                    if block_state_is_air(old) {
                        context.stone_depth_above = 0;
                        context.water_height = i32::MIN;
                    } else if !block_state_fluid_state(old).is_empty() {
                        if context.water_height == i32::MIN {
                            context.water_height = y + 1;
                        }
                    } else {
                        if next_ceiling_stone_y >= y {
                            next_ceiling_stone_y = i32::MIN / 2;
                            for lookahead in ((self.min_y - 1)..y).rev() {
                                let next = column
                                    .get_block_state(local_x, lookahead, local_z)
                                    .unwrap_or(self.air);
                                if block_state_is_air(next)
                                    || !block_state_fluid_state(next).is_empty()
                                {
                                    next_ceiling_stone_y = lookahead + 1;
                                    break;
                                }
                            }
                        }
                        context.stone_depth_above += 1;
                        context.stone_depth_below = y - next_ceiling_stone_y + 1;
                        context.y = y;
                        context.biome = None;
                        if old == self.default_block {
                            if let Some(state) = self.rule.apply(&mut context) {
                                set_surface_block(
                                    column,
                                    ocean_floor_wg,
                                    context.world_surface_wg,
                                    postprocessing,
                                    local_x,
                                    y,
                                    local_z,
                                    x,
                                    z,
                                    state,
                                    self.min_y,
                                );
                            }
                        }
                    }
                }
                if Some(surface_biome) == self.frozen_ocean
                    || Some(surface_biome) == self.deep_frozen_ocean
                {
                    let min_surface_level = context.min_surface_level();
                    self.frozen_ocean_extension(
                        surface_biome,
                        column,
                        ocean_floor_wg,
                        world_surface_wg,
                        postprocessing,
                        local_x,
                        local_z,
                        x,
                        z,
                        starting_height,
                        min_surface_level,
                    );
                }
            }
        }
    }

    /// `SurfaceSystem.topMaterial` as used by carvers after removing a grass
    /// or mycelium surface (`SurfaceSystem.java:183-203`).
    #[allow(clippy::too_many_arguments)]
    pub fn top_material(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        search: &mut NoiseBiomeSearch,
        world_surface_wg: &[i32; 256],
        world_x: i32,
        y: i32,
        world_z: i32,
        under_fluid: bool,
    ) -> Option<i32> {
        let mut preliminary_corners = None;
        self.top_material_with_generation_shape(
            source,
            biome_source,
            search,
            world_surface_wg,
            &mut preliminary_corners,
            world_x,
            y,
            world_z,
            under_fluid,
            self.min_y,
            self.height,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn top_material_retrogen(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        search: &mut NoiseBiomeSearch,
        world_surface_wg: &[i32; 256],
        world_x: i32,
        y: i32,
        world_z: i32,
        under_fluid: bool,
    ) -> Option<i32> {
        let mut preliminary_corners = None;
        self.top_material_with_generation_shape(
            source,
            biome_source,
            search,
            world_surface_wg,
            &mut preliminary_corners,
            world_x,
            y,
            world_z,
            under_fluid,
            -64,
            64,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn top_material_cached(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        search: &mut NoiseBiomeSearch,
        world_surface_wg: &[i32; 256],
        preliminary_corners: &mut Option<[i32; 4]>,
        world_x: i32,
        y: i32,
        world_z: i32,
        under_fluid: bool,
        upgrading: bool,
    ) -> Option<i32> {
        let (generation_min_y, generation_height) = if upgrading {
            (-64, 64)
        } else {
            (self.min_y, self.height)
        };
        self.top_material_with_generation_shape(
            source,
            biome_source,
            search,
            world_surface_wg,
            preliminary_corners,
            world_x,
            y,
            world_z,
            under_fluid,
            generation_min_y,
            generation_height,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn top_material_with_generation_shape(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        search: &mut NoiseBiomeSearch,
        world_surface_wg: &[i32; 256],
        preliminary_corners: &mut Option<[i32; 4]>,
        world_x: i32,
        y: i32,
        world_z: i32,
        under_fluid: bool,
        generation_min_y: i32,
        generation_height: i32,
    ) -> Option<i32> {
        let mut heightmap = *world_surface_wg;
        let mut no_biome_override = |_, _, _| None;
        let mut stack_thresholds = [None; 32];
        let mut heap_thresholds = Vec::new();
        let noise_thresholds: &mut [Option<bool>] =
            if self.noise_threshold_count <= stack_thresholds.len() {
                &mut stack_thresholds[..self.noise_threshold_count]
            } else {
                heap_thresholds.resize(self.noise_threshold_count, None);
                &mut heap_thresholds
            };
        let mut context = Context {
            program: self,
            source,
            biome_source,
            search: Some(search),
            biome_override: &mut no_biome_override,
            world_surface_wg: &mut heightmap,
            preliminary_corners,
            noise_thresholds,
            x: world_x,
            z: world_z,
            local_x: world_x.rem_euclid(16),
            local_z: world_z.rem_euclid(16),
            y,
            surface_depth: self.surface_depth(world_x, world_z),
            surface_secondary: None,
            min_surface_level: None,
            water_height: if under_fluid {
                y.saturating_add(1)
            } else {
                i32::MIN
            },
            stone_depth_above: 1,
            stone_depth_below: 1,
            biome: None,
            generation_min_y,
            generation_height,
        };
        self.rule.apply(&mut context)
    }

    pub fn surface_depth(&self, x: i32, z: i32) -> i32 {
        let noise = self.surface.get_value(f64::from(x), 0.0, f64::from(z));
        (noise * 2.75 + 3.0 + self.root_random.at(x, 0, z).next_double() * 0.25) as i32
    }

    pub fn band(&self, x: i32, y: i32, z: i32) -> i32 {
        let sample = self
            .clay_bands_offset
            .get_value(f64::from(x), 0.0, f64::from(z))
            * 4.0;
        let offset = sample.floor() as i32 + if sample - sample.floor() >= 0.5 { 1 } else { 0 };
        self.clay_bands[(y + offset).rem_euclid(192) as usize]
    }

    #[allow(clippy::too_many_arguments)]
    pub fn eroded_badlands_extension(
        &self,
        column: &mut ChunkColumn,
        ocean: &mut [i32; 256],
        surface: &mut [i32; 256],
        post: &mut Vec<(i32, i32, i32)>,
        local_x: i32,
        local_z: i32,
        x: i32,
        z: i32,
        height: i32,
    ) {
        let buffer = (self
            .badlands_surface
            .get_value(f64::from(x), 0.0, f64::from(z))
            * 8.25)
            .abs()
            .min(
                self.badlands_pillar
                    .get_value(f64::from(x) * 0.2, 0.0, f64::from(z) * 0.2)
                    * 15.0,
            );
        if buffer <= 0.0 {
            return;
        }
        let roof =
            (self
                .badlands_pillar_roof
                .get_value(f64::from(x) * 0.75, 0.0, f64::from(z) * 0.75)
                * 1.5)
                .abs();
        let start_y =
            (64.0 + (buffer * buffer * 2.5).min((roof * 50.0).ceil() + 24.0)).floor() as i32;
        if height > start_y {
            return;
        }
        let default_block_type = vanilla_block_state_block_type_id(self.default_block);
        for y in (self.min_y..=start_y).rev() {
            let old = column
                .get_block_state(local_x, y, local_z)
                .unwrap_or(self.air);
            if vanilla_block_state_block_type_id(old) == default_block_type {
                break;
            }
            if old == self.water {
                return;
            }
        }
        for y in (self.min_y..=start_y).rev() {
            if !block_state_is_air(
                column
                    .get_block_state(local_x, y, local_z)
                    .unwrap_or(self.air),
            ) {
                break;
            }
            set_surface_block(
                column,
                ocean,
                surface,
                post,
                local_x,
                y,
                local_z,
                x,
                z,
                self.default_block,
                self.min_y,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn frozen_ocean_extension(
        &self,
        biome: i32,
        column: &mut ChunkColumn,
        ocean: &mut [i32; 256],
        surface: &mut [i32; 256],
        post: &mut Vec<(i32, i32, i32)>,
        local_x: i32,
        local_z: i32,
        x: i32,
        z: i32,
        height: i32,
        min_surface_level: i32,
    ) {
        let iceberg = (self
            .iceberg_surface
            .get_value(f64::from(x), 0.0, f64::from(z))
            * 8.25)
            .abs()
            .min(
                self.iceberg_pillar
                    .get_value(f64::from(x) * 1.28, 0.0, f64::from(z) * 1.28)
                    * 15.0,
            );
        if iceberg <= 1.8 {
            return;
        }
        let roof =
            (self
                .iceberg_pillar_roof
                .get_value(f64::from(x) * 1.17, 0.0, f64::from(z) * 1.17)
                * 1.5)
                .abs();
        let mut top = (iceberg * iceberg * 1.2).min((roof * 40.0).ceil() + 14.0);
        let facts = self.biome_facts[biome as usize];
        if height_adjusted_temperature(
            facts.temperature,
            facts.modifier,
            x,
            self.sea_level,
            z,
            self.sea_level,
        ) > 0.1
        {
            top -= 2.0;
        }
        let bottom;
        if top > 2.0 {
            bottom = f64::from(self.sea_level) - top - 7.0;
            top += f64::from(self.sea_level);
        } else {
            top = 0.0;
            bottom = 0.0;
        }
        let mut random = self.root_random.at(x, 0, z);
        let max_snow_depth = 2 + random.next_int(4);
        let min_snow_height = self.sea_level + 18 + random.next_int(10);
        let mut snow_depth = 0;
        let from_y = height.max(top as i32 + 1);
        for y in (min_surface_level..=from_y).rev() {
            let old = column
                .get_block_state(local_x, y, local_z)
                .unwrap_or(self.air);
            let replace =
                (block_state_is_air(old) && y < top as i32 && random.next_double() > 0.01)
                    || (old == self.water
                        && y > bottom as i32
                        && y < self.sea_level
                        && bottom != 0.0
                        && random.next_double() > 0.15);
            if replace {
                let state = if snow_depth <= max_snow_depth && y > min_snow_height {
                    snow_depth += 1;
                    self.snow_block
                } else {
                    self.packed_ice
                };
                set_surface_block(
                    column, ocean, surface, post, local_x, y, local_z, x, z, state, self.min_y,
                );
            }
        }
    }
}

pub struct Context<'a> {
    program: &'a NoiseSurfaceProgram,
    source: &'a NoiseDensitySource,
    biome_source: &'a NoiseBiomeSource,
    search: Option<&'a mut NoiseBiomeSearch>,
    biome_override: &'a mut dyn FnMut(i32, i32, i32) -> Option<i32>,
    world_surface_wg: &'a mut [i32; 256],
    preliminary_corners: &'a mut Option<[i32; 4]>,
    noise_thresholds: &'a mut [Option<bool>],
    x: i32,
    z: i32,
    local_x: i32,
    local_z: i32,
    y: i32,
    surface_depth: i32,
    surface_secondary: Option<f64>,
    min_surface_level: Option<i32>,
    water_height: i32,
    stone_depth_above: i32,
    stone_depth_below: i32,
    biome: Option<i32>,
    generation_min_y: i32,
    generation_height: i32,
}

impl Context<'_> {
    pub fn biome(&mut self) -> i32 {
        if let Some(biome) = self.biome {
            biome
        } else {
            let (quart_x, quart_y, quart_z) =
                fuzzy_biome_quart_position(self.program.biome_zoom_seed, self.x, self.y, self.z);
            let biome = (self.biome_override)(quart_x, quart_y, quart_z).unwrap_or_else(|| {
                let search = self
                    .search
                    .as_deref_mut()
                    .expect("uncached biome lookup requires a biome search");
                self.biome_source
                    .sample(self.source, quart_x, quart_y, quart_z, search)
            });
            self.biome = Some(biome);
            biome
        }
    }

    pub fn secondary(&mut self) -> f64 {
        *self.surface_secondary.get_or_insert_with(|| {
            self.program
                .surface_secondary
                .get_value(f64::from(self.x), 0.0, f64::from(self.z))
        })
    }

    pub fn min_surface_level(&mut self) -> i32 {
        if let Some(value) = self.min_surface_level {
            return value;
        }
        let corners = *self.preliminary_corners.get_or_insert_with(|| {
            let cell_x = self.x >> 4;
            let cell_z = self.z >> 4;
            [
                self.source
                    .preliminary_surface_level(cell_x << 4, cell_z << 4),
                self.source
                    .preliminary_surface_level((cell_x + 1) << 4, cell_z << 4),
                self.source
                    .preliminary_surface_level(cell_x << 4, (cell_z + 1) << 4),
                self.source
                    .preliminary_surface_level((cell_x + 1) << 4, (cell_z + 1) << 4),
            ]
        });
        let fx = f64::from((self.x & 15) as f32 / 16.0);
        let fz = f64::from((self.z & 15) as f32 / 16.0);
        let x0 = f64::from(corners[0]) + fx * f64::from(corners[1] - corners[0]);
        let x1 = f64::from(corners[2]) + fx * f64::from(corners[3] - corners[2]);
        let value = (x0 + fz * (x1 - x0)).floor() as i32 + self.surface_depth - 8;
        self.min_surface_level = Some(value);
        value
    }
}

impl Rule {
    pub fn apply(&self, context: &mut Context<'_>) -> Option<i32> {
        match self {
            Self::Bandlands => Some(context.program.band(context.x, context.y, context.z)),
            Self::Block(state) => Some(*state),
            Self::Sequence(rules) => rules.iter().find_map(|rule| rule.apply(context)),
            Self::Condition(condition, rule) => condition
                .test(context)
                .then(|| rule.apply(context))
                .flatten(),
        }
    }
}

impl Condition {
    pub fn test(&self, context: &mut Context<'_>) -> bool {
        match self {
            Self::AbovePreliminarySurface => context.y >= context.min_surface_level(),
            Self::Biome(biomes) => biomes.contains(context.biome()),
            Self::Hole => context.surface_depth <= 0,
            Self::NoiseThreshold {
                slot,
                noise,
                min,
                max,
            } => {
                if let Some(cached) = context.noise_thresholds[*slot] {
                    return cached;
                }
                let value = noise.get_value(f64::from(context.x), 0.0, f64::from(context.z));
                let matched = value >= *min && value <= *max;
                context.noise_thresholds[*slot] = Some(matched);
                matched
            }
            Self::Not(condition) => !condition.test(context),
            Self::Steep => {
                let x = context.local_x as usize;
                let z = context.local_z as usize;
                let north = context.world_surface_wg[x + z.saturating_sub(1) * 16];
                let south = context.world_surface_wg[x + (z + 1).min(15) * 16];
                let west = context.world_surface_wg[x.saturating_sub(1) + z * 16];
                let east = context.world_surface_wg[(x + 1).min(15) + z * 16];
                south >= north + 4 || west >= east + 4
            }
            Self::StoneDepth {
                offset,
                add_surface_depth,
                secondary_depth_range,
                ceiling,
            } => {
                let depth = if *ceiling {
                    context.stone_depth_below
                } else {
                    context.stone_depth_above
                };
                let surface = if *add_surface_depth {
                    context.surface_depth
                } else {
                    0
                };
                let secondary = if *secondary_depth_range == 0 {
                    0
                } else {
                    (((context.secondary() + 1.0) * 0.5) * f64::from(*secondary_depth_range)) as i32
                };
                depth <= 1 + offset + surface + secondary
            }
            Self::Temperature => {
                let facts = context.program.biome_facts[context.biome() as usize];
                height_adjusted_temperature(
                    facts.temperature,
                    facts.modifier,
                    context.x,
                    context.y,
                    context.z,
                    context.program.sea_level,
                ) < 0.15
            }
            Self::VerticalGradient {
                random,
                true_at_and_below,
                false_at_and_above,
            } => {
                let low =
                    true_at_and_below.resolve(context.generation_min_y, context.generation_height);
                let high =
                    false_at_and_above.resolve(context.generation_min_y, context.generation_height);
                if context.y <= low {
                    true
                } else if context.y >= high {
                    false
                } else {
                    // `Mth.map(blockY, low, high, 1.0, 0.0)` compared against the
                    // widened float draw (`SurfaceRules.java:764-776`).
                    let alpha = (f64::from(context.y) - f64::from(low))
                        / (f64::from(high) - f64::from(low));
                    let probability = 1.0 + alpha * (0.0 - 1.0);
                    f64::from(random.at(context.x, context.y, context.z).next_float()) < probability
                }
            }
            Self::Water {
                offset,
                surface_depth_multiplier,
                add_stone_depth,
            } => {
                context.water_height == i32::MIN
                    || context.y
                        + if *add_stone_depth {
                            context.stone_depth_above
                        } else {
                            0
                        }
                        >= context.water_height
                            + offset
                            + context.surface_depth * surface_depth_multiplier
            }
            Self::YAbove {
                anchor,
                surface_depth_multiplier,
                add_stone_depth,
            } => {
                context.y
                    + if *add_stone_depth {
                        context.stone_depth_above
                    } else {
                        0
                    }
                    >= anchor.resolve(context.generation_min_y, context.generation_height)
                        + context.surface_depth * surface_depth_multiplier
            }
        }
    }
}

struct Compiler<'a> {
    generation: &'a DatapackGeneration,
    world_seed: i64,
    biome_ids: BTreeMap<String, i32>,
    noises: crate::fast_hash::FastHashMap<String, Arc<NormalNoise>>,
    noise_threshold_count: usize,
}

impl Compiler<'_> {
    fn compile_rule(&mut self, rule: &SurfaceRule) -> Result<Rule, NoiseDensityError> {
        Ok(match rule {
            SurfaceRule::Bandlands => Rule::Bandlands,
            SurfaceRule::Block(state) => Rule::Block(*state),
            SurfaceRule::Sequence(rules) => Rule::Sequence(
                rules
                    .iter()
                    .map(|rule| self.compile_rule(rule))
                    .collect::<Result<_, _>>()?,
            ),
            SurfaceRule::Condition(condition, rule) => Rule::Condition(
                self.compile_condition(condition)?,
                Box::new(self.compile_rule(rule)?),
            ),
        })
    }
    fn compile_condition(
        &mut self,
        condition: &SurfaceCondition,
    ) -> Result<Condition, NoiseDensityError> {
        Ok(match condition {
            SurfaceCondition::AbovePreliminarySurface => Condition::AbovePreliminarySurface,
            // `SurfaceRules.BiomeConditionSource` tests `Holder.is` against a ResourceKey set
            // (`SurfaceRules.java:168-192`).
            SurfaceCondition::Biome(ids) => Condition::Biome(BiomeSet::from_ids(
                ids.iter().filter_map(|id| self.biome_ids.get(id).copied()),
            )),
            SurfaceCondition::Hole => Condition::Hole,
            SurfaceCondition::NoiseThreshold { noise, min, max } => {
                let compiled = if let Some(existing) = self.noises.get(noise) {
                    Arc::clone(existing)
                } else {
                    let created = Arc::new(normal_noise_from_registry(
                        self.generation,
                        noise,
                        self.world_seed,
                    )?);
                    self.noises.insert(noise.clone(), Arc::clone(&created));
                    created
                };
                let slot = self.noise_threshold_count;
                self.noise_threshold_count += 1;
                Condition::NoiseThreshold {
                    slot,
                    noise: compiled,
                    min: *min,
                    max: *max,
                }
            }
            SurfaceCondition::Not(condition) => {
                Condition::Not(Box::new(self.compile_condition(condition)?))
            }
            SurfaceCondition::Steep => Condition::Steep,
            SurfaceCondition::StoneDepth {
                offset,
                add_surface_depth,
                secondary_depth_range,
                ceiling,
            } => Condition::StoneDepth {
                offset: *offset,
                add_surface_depth: *add_surface_depth,
                secondary_depth_range: *secondary_depth_range,
                ceiling: *ceiling,
            },
            SurfaceCondition::Temperature => Condition::Temperature,
            SurfaceCondition::VerticalGradient {
                random_name,
                true_at_and_below,
                false_at_and_above,
            } => Condition::VerticalGradient {
                random: PositionalRandom::new(self.world_seed, random_name),
                true_at_and_below: *true_at_and_below,
                false_at_and_above: *false_at_and_above,
            },
            SurfaceCondition::Water {
                offset,
                surface_depth_multiplier,
                add_stone_depth,
            } => Condition::Water {
                offset: *offset,
                surface_depth_multiplier: *surface_depth_multiplier,
                add_stone_depth: *add_stone_depth,
            },
            SurfaceCondition::YAbove {
                anchor,
                surface_depth_multiplier,
                add_stone_depth,
            } => Condition::YAbove {
                anchor: *anchor,
                surface_depth_multiplier: *surface_depth_multiplier,
                add_stone_depth: *add_stone_depth,
            },
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn set_surface_block(
    column: &mut ChunkColumn,
    ocean: &mut [i32; 256],
    surface: &mut [i32; 256],
    post: &mut Vec<(i32, i32, i32)>,
    local_x: i32,
    y: i32,
    local_z: i32,
    world_x: i32,
    world_z: i32,
    state: i32,
    min_y: i32,
) {
    set_generated_block(
        column, ocean, surface, post, local_x, y, local_z, world_x, world_z, state, min_y, true,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn set_generated_block(
    column: &mut ChunkColumn,
    ocean: &mut [i32; 256],
    surface: &mut [i32; 256],
    post: &mut Vec<(i32, i32, i32)>,
    local_x: i32,
    y: i32,
    local_z: i32,
    world_x: i32,
    world_z: i32,
    state: i32,
    min_y: i32,
    mark_fluid_for_postprocessing: bool,
) {
    if column.set_block_state(local_x, y, local_z, state).is_none() {
        return;
    }
    update_heightmap(
        column,
        ocean,
        local_x,
        y,
        local_z,
        state,
        min_y,
        block_state_blocks_motion,
    );
    update_heightmap(
        column,
        surface,
        local_x,
        y,
        local_z,
        state,
        min_y,
        |state| !block_state_is_air(state),
    );
    if mark_fluid_for_postprocessing && !block_state_fluid_state(state).is_empty() {
        post.push((world_x, y, world_z));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_heightmap(
    column: &ChunkColumn,
    map: &mut [i32; 256],
    x: i32,
    y: i32,
    z: i32,
    state: i32,
    min_y: i32,
    opaque: impl Fn(i32) -> bool,
) {
    let index = (x + z * 16) as usize;
    let first_available = map[index];
    if y <= first_available - 2 {
        return;
    }
    if opaque(state) {
        if y >= first_available {
            map[index] = y + 1;
        }
    } else if first_available - 1 == y {
        map[index] = min_y;
        for scan_y in (min_y..y).rev() {
            if opaque(column.get_block_state(x, scan_y, z).unwrap_or(0)) {
                map[index] = scan_y + 1;
                break;
            }
        }
    }
}

pub fn fuzzy_biome(
    seed: i64,
    biome_source: &NoiseBiomeSource,
    density: &NoiseDensitySource,
    search: &mut NoiseBiomeSearch,
    x: i32,
    y: i32,
    z: i32,
) -> i32 {
    fuzzy_biome_with_override(
        seed,
        biome_source,
        density,
        search,
        &mut |_, _, _| None,
        x,
        y,
        z,
    )
}

#[allow(clippy::too_many_arguments)]
fn fuzzy_biome_with_override(
    seed: i64,
    biome_source: &NoiseBiomeSource,
    density: &NoiseDensitySource,
    search: &mut NoiseBiomeSearch,
    biome_override: &mut dyn FnMut(i32, i32, i32) -> Option<i32>,
    x: i32,
    y: i32,
    z: i32,
) -> i32 {
    let (quart_x, quart_y, quart_z) = fuzzy_biome_quart_position(seed, x, y, z);
    biome_override(quart_x, quart_y, quart_z)
        .unwrap_or_else(|| biome_source.sample(density, quart_x, quart_y, quart_z, search))
}

#[allow(clippy::too_many_arguments)]
pub fn generate_bands(
    random: &mut WorldgenRandom,
    terracotta: i32,
    orange: i32,
    yellow: i32,
    brown: i32,
    red: i32,
    white: i32,
    light_gray: i32,
) -> [i32; 192] {
    let mut bands = [terracotta; 192];
    let mut index = 0;
    while index < bands.len() {
        index += random.next_int(5) as usize + 1;
        if index < bands.len() {
            bands[index] = orange;
        }
        index += 1;
    }
    make_bands(random, &mut bands, 1, yellow);
    make_bands(random, &mut bands, 2, brown);
    make_bands(random, &mut bands, 1, red);
    let count = random.next_int_between_inclusive(9, 15);
    let mut made = 0;
    let mut start = 0;
    while made < count && start < bands.len() {
        bands[start] = white;
        if start > 1 && random.next_boolean() {
            bands[start - 1] = light_gray;
        }
        if start + 1 < bands.len() && random.next_boolean() {
            bands[start + 1] = light_gray;
        }
        made += 1;
        start += random.next_int(16) as usize + 4;
    }
    bands
}

pub fn make_bands(
    random: &mut WorldgenRandom,
    bands: &mut [i32; 192],
    base_width: usize,
    state: i32,
) {
    for _ in 0..random.next_int_between_inclusive(6, 15) {
        let width = base_width + random.next_int(3) as usize;
        let start = random.next_int(192) as usize;
        for slot in bands.iter_mut().skip(start).take(width) {
            *slot = state;
        }
    }
}

pub fn parse_biome_facts(
    generation: &DatapackGeneration,
) -> Result<(Vec<BiomeFacts>, BTreeMap<String, i32>), NoiseDensityError> {
    let registry = generation
        .registry(BIOME_REGISTRY)
        .ok_or_else(|| error(BIOME_REGISTRY, "registry is absent"))?;
    let mut facts = Vec::with_capacity(registry.elements.len());
    let mut ids = BTreeMap::new();
    for (index, element) in registry.elements.iter().enumerate() {
        let climate = generation
            .worldgen
            .biome_climates
            .get(index)
            .ok_or_else(|| error(BIOME_REGISTRY, "published biome climate is absent"))?;
        let temperature = climate.temperature;
        let modifier = climate.modifier;
        let index =
            i32::try_from(index).map_err(|_| error(BIOME_REGISTRY, "registry is too large"))?;
        ids.insert(element.id.clone(), index);
        facts.push(BiomeFacts {
            temperature,
            modifier,
        });
    }
    Ok((facts, ids))
}

pub fn error(path: impl Into<String>, message: impl Into<String>) -> NoiseDensityError {
    NoiseDensityError {
        path: path.into(),
        message: message.into(),
    }
}

#[cfg(test)]
pub mod tests;
