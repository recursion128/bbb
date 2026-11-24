//! Built-in CARVERS-stage runtimes.

use crate::random::large_feature_random;
use std::collections::BTreeSet;

use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_DIRT_ID,
    VANILLA_BLOCK_GRASS_BLOCK_ID, VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_MYCELIUM_ID,
    VANILLA_BLOCK_WATER_ID,
};
use bbb_server_datapack::worldgen::{CanyonShape, CarverKind, FloatProvider, VerticalAnchor};

use crate::configured_float_provider::sample_float_provider;
use crate::height_provider::sample_height_provider_with_bounds;
use crate::{mth_cos, mth_floor_f64, mth_sin};
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::TagLoadReport;
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::fluid::block_state_fluid_state;

use super::aquifer::Aquifer;
use super::biome::{NoiseBiomeSearch, NoiseBiomeSource};
use super::density::chunk::NoiseChunkSampler;
use super::density::{NoiseDensityError, NoiseDensitySource};
use super::surface::{set_generated_block, NoiseSurfaceProgram};

pub mod nether;
mod plan;

use nether::NetherCarverProgram;
use plan::{CompiledCarver, DebugStates, RegistryCarverPlan};

pub const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
pub const BLOCK_REGISTRY: &str = "minecraft:block";

pub enum BuiltInCarverProgram {
    Overworld(OverworldCarverProgram),
    Nether(NetherCarverProgram),
}

impl BuiltInCarverProgram {
    pub fn validate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Result<(), NoiseDensityError> {
        match self {
            Self::Overworld(program) => program.validate_chunk(chunk_x, chunk_z),
            Self::Nether(program) => program.validate_chunk(chunk_x, chunk_z),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        surface: &NoiseSurfaceProgram,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
    ) -> Result<Vec<i64>, NoiseDensityError> {
        match self {
            Self::Overworld(program) => program.apply(
                source,
                biome_source,
                surface,
                chunk_x,
                chunk_z,
                column,
                ocean_floor_wg,
                world_surface_wg,
                postprocessing,
            ),
            Self::Nether(program) => program.apply(
                source,
                biome_source,
                surface,
                chunk_x,
                chunk_z,
                column,
                ocean_floor_wg,
                world_surface_wg,
                postprocessing,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_retrogen(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        surface: &NoiseSurfaceProgram,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
    ) -> Result<Vec<i64>, NoiseDensityError> {
        match self {
            Self::Overworld(program) => program.apply_with_generation_shape(
                source,
                biome_source,
                surface,
                chunk_x,
                chunk_z,
                column,
                ocean_floor_wg,
                world_surface_wg,
                postprocessing,
                -64,
                64,
                true,
            ),
            Self::Nether(_) => Err(error(
                "noise_generator.carvers.retrogen",
                "below-zero retrogen requires an Overworld carver owner",
            )),
        }
    }

    pub const fn is_overworld(&self) -> bool {
        matches!(self, Self::Overworld(_))
    }
}

pub struct OverworldCarverProgram {
    world_seed: i64,
    plan: RegistryCarverPlan,
}

impl OverworldCarverProgram {
    pub fn from_registry(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        world_seed: i64,
        possible_biomes: &[&str],
    ) -> Result<Self, NoiseDensityError> {
        let path = "noise_generator.carvers";
        if possible_biomes.is_empty() {
            return Err(error(path, "carver runtime requires possible biomes"));
        }
        Ok(Self {
            world_seed,
            plan: RegistryCarverPlan::compile(generation, tags, possible_biomes)?,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.plan.is_empty()
    }

    pub fn validate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Result<(), NoiseDensityError> {
        let path = "noise_generator.carvers.apply";
        let min_source_x = chunk_x
            .checked_sub(8)
            .ok_or_else(|| error(path, "source chunk x range underflows"))?;
        let max_source_x = chunk_x
            .checked_add(8)
            .ok_or_else(|| error(path, "source chunk x range overflows"))?;
        let min_source_z = chunk_z
            .checked_sub(8)
            .ok_or_else(|| error(path, "source chunk z range underflows"))?;
        let max_source_z = chunk_z
            .checked_add(8)
            .ok_or_else(|| error(path, "source chunk z range overflows"))?;
        for coordinate in [min_source_x, max_source_x, min_source_z, max_source_z] {
            coordinate
                .checked_mul(16)
                .and_then(|start| start.checked_add(15))
                .ok_or_else(|| error(path, "source block-coordinate range overflows"))?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        surface: &NoiseSurfaceProgram,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
    ) -> Result<Vec<i64>, NoiseDensityError> {
        let column_min_y = column.min_y();
        let column_height = i32::try_from(column.section_count())
            .ok()
            .and_then(|sections| sections.checked_mul(16))
            .ok_or_else(|| {
                error(
                    "noise_generator.carvers.apply",
                    "target column height overflows",
                )
            })?;
        // `WorldGenerationContext` and `NoiseChunk.forChunk` clamp the noise vertical range to the
        // target chunk accessor (`WorldGenerationContext.java:10-13`, `NoiseSettings.java:54-58`).
        let generation_min_y = source.min_y().max(column_min_y);
        let generation_max_y = source
            .min_y()
            .saturating_add(source.height())
            .min(column_min_y.saturating_add(column_height));
        let generation_height = generation_max_y.saturating_sub(generation_min_y);
        self.apply_with_generation_shape(
            source,
            biome_source,
            surface,
            chunk_x,
            chunk_z,
            column,
            ocean_floor_wg,
            world_surface_wg,
            postprocessing,
            generation_min_y,
            generation_height,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_with_generation_shape(
        &self,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
        surface: &NoiseSurfaceProgram,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        ocean_floor_wg: &mut [i32; 256],
        world_surface_wg: &mut [i32; 256],
        postprocessing: &mut Vec<(i32, i32, i32)>,
        generation_min_y: i32,
        generation_height: i32,
        upgrading: bool,
    ) -> Result<Vec<i64>, NoiseDensityError> {
        let path = "noise_generator.carvers.apply";
        let column_min_y = column.min_y();
        let column_height = i32::try_from(column.section_count())
            .ok()
            .and_then(|sections| sections.checked_mul(16))
            .ok_or_else(|| error(path, "target column height overflows"))?;
        let clamped_min_y = source.min_y().max(column_min_y);
        let clamped_max_y = source
            .min_y()
            .saturating_add(source.height())
            .min(column_min_y.saturating_add(column_height));
        if generation_height <= 0
            || generation_min_y < clamped_min_y
            || generation_min_y.saturating_add(generation_height) > clamped_max_y
        {
            return Err(error(
                path,
                "carver generation accessor is outside the noise shape",
            ));
        }
        self.validate_chunk(chunk_x, chunk_z)?;
        let min_source_x = chunk_x
            .checked_sub(8)
            .ok_or_else(|| error(path, "source chunk x range underflows"))?;
        let max_source_x = chunk_x
            .checked_add(8)
            .ok_or_else(|| error(path, "source chunk x range overflows"))?;
        let min_source_z = chunk_z
            .checked_sub(8)
            .ok_or_else(|| error(path, "source chunk z range underflows"))?;
        let max_source_z = chunk_z
            .checked_add(8)
            .ok_or_else(|| error(path, "source chunk z range overflows"))?;
        let _chunk_start_x = chunk_x
            .checked_mul(16)
            .ok_or_else(|| error(path, "target block x overflows"))?;
        let _chunk_start_z = chunk_z
            .checked_mul(16)
            .ok_or_else(|| error(path, "target block z overflows"))?;

        let mut runtime = CarverRuntime {
            source,
            biome_source,
            surface,
            search: biome_source.create_search(),
            column,
            ocean_floor_wg,
            world_surface_wg,
            postprocessing,
            sampler: None,
            aquifer: None,
            preliminary_corners: None,
            cave_air: vanilla_block_default_state_id(
                vanilla_block_type_for_resource_id("minecraft:cave_air")
                    .ok_or_else(|| error(path, "Vanilla cave_air block is absent"))?
                    .id,
            )
            .ok_or_else(|| error(path, "Vanilla cave_air default state is absent"))?,
            last_schedule_fluid_update: false,
            // `ProtoChunk.getOrCreateCarvingMask` indexes relative to chunk minY/height (`ProtoChunk.java:292-298`).
            mask: CarvingMask::new(column_min_y, column_height),
            generation_min_y,
            generation_height,
            upgrading,
            chunk_x,
            chunk_z,
        };

        // `NoiseBasedChunkGenerator.applyCarvers` resets the holder index for each
        // source biome (`NoiseBasedChunkGenerator.java:299-345`).
        for source_x in min_source_x..=max_source_x {
            for source_z in min_source_z..=max_source_z {
                let quart_x = source_x
                    .checked_mul(4)
                    .ok_or_else(|| error(path, "source biome quart x coordinate overflows"))?;
                let quart_z = source_z
                    .checked_mul(4)
                    .ok_or_else(|| error(path, "source biome quart z coordinate overflows"))?;
                let source_biome = runtime.biome_source.sample(
                    runtime.source,
                    quart_x,
                    0,
                    quart_z,
                    &mut runtime.search,
                );
                let carvers = self.plan.for_biome(source_biome)?;
                for (index, carver) in carvers.iter().enumerate() {
                    let seed = self.world_seed.wrapping_add(index as i64);
                    let mut random = large_feature_random(seed, source_x, source_z);
                    if random.next_float() <= carver.config.probability {
                        runtime.carve(carver, &mut random, source_x, source_z);
                    }
                }
            }
        }
        Ok(runtime.mask.into_trimmed_longs())
    }
}

pub struct CarverRuntime<'a> {
    source: &'a NoiseDensitySource,
    biome_source: &'a NoiseBiomeSource,
    surface: &'a NoiseSurfaceProgram,
    search: NoiseBiomeSearch,
    column: &'a mut ChunkColumn,
    ocean_floor_wg: &'a mut [i32; 256],
    world_surface_wg: &'a mut [i32; 256],
    postprocessing: &'a mut Vec<(i32, i32, i32)>,
    sampler: Option<NoiseChunkSampler<'a>>,
    aquifer: Option<Aquifer>,
    preliminary_corners: Option<[i32; 4]>,
    cave_air: i32,
    last_schedule_fluid_update: bool,
    mask: CarvingMask,
    generation_min_y: i32,
    generation_height: i32,
    upgrading: bool,
    chunk_x: i32,
    chunk_z: i32,
}

impl CarverRuntime<'_> {
    fn carve(
        &mut self,
        carver: &CompiledCarver,
        random: &mut WorldgenRandom,
        source_x: i32,
        source_z: i32,
    ) {
        match &carver.config.kind {
            CarverKind::Cave {
                nether,
                horizontal_radius_multiplier,
                vertical_radius_multiplier,
                floor_level,
            } => self.carve_caves(
                carver,
                if *nether {
                    CaveRuntimeKind::Nether
                } else {
                    CaveRuntimeKind::Overworld
                },
                [
                    horizontal_radius_multiplier,
                    vertical_radius_multiplier,
                    floor_level,
                ],
                random,
                source_x,
                source_z,
            ),
            CarverKind::Canyon {
                vertical_rotation,
                shape,
            } => self.carve_canyon(carver, vertical_rotation, shape, random, source_x, source_z),
        }
    }

    fn carve_caves(
        &mut self,
        carver: &CompiledCarver,
        kind: CaveRuntimeKind,
        [horizontal_radius, vertical_radius, floor_level]: [&FloatProvider; 3],
        random: &mut WorldgenRandom,
        source_x: i32,
        source_z: i32,
    ) {
        // `CaveWorldCarver.java:21-235`.
        let max_distance = 112;
        let bound = random.next_int(kind.cave_bound()) + 1;
        let bound = random.next_int(bound) + 1;
        let cave_count = random.next_int(bound);
        for _ in 0..cave_count {
            let x = f64::from(source_x.wrapping_mul(16).wrapping_add(random.next_int(16)));
            let y = f64::from(
                sample_height_provider_with_bounds(
                    &carver.config.y,
                    self.generation_min_y,
                    self.generation_height,
                    random,
                )
                .expect("validated carver height provider samples"),
            );
            let z = f64::from(source_z.wrapping_mul(16).wrapping_add(random.next_int(16)));
            let horizontal_multiplier = f64::from(sample_float_provider(horizontal_radius, random));
            let vertical_multiplier = f64::from(sample_float_provider(vertical_radius, random));
            let floor = f64::from(sample_float_provider(floor_level, random));
            let mut tunnels = 1;
            if random.next_int(4) == 0 {
                let y_scale = f64::from(sample_float_provider(&carver.config.y_scale, random));
                let thickness = 1.0_f32 + random.next_float() * 6.0_f32;
                let horizontal_radius =
                    1.5 + f64::from(mth_sin(f64::from(std::f32::consts::FRAC_PI_2)) * thickness);
                self.carve_ellipsoid(
                    carver,
                    kind,
                    x + 1.0,
                    y,
                    z,
                    horizontal_radius,
                    horizontal_radius * y_scale,
                    SkipChecker::Cave { floor },
                );
                tunnels += random.next_int(4);
            }
            for _ in 0..tunnels {
                let horizontal_rotation = random.next_float() * std::f32::consts::TAU;
                let vertical_rotation = (random.next_float() - 0.5_f32) / 4.0_f32;
                let mut thickness =
                    (random.next_float() * 2.0_f32 + random.next_float()) * kind.thickness_scale();
                if kind == CaveRuntimeKind::Overworld && random.next_int(10) == 0 {
                    thickness *= random.next_float() * random.next_float() * 3.0_f32 + 1.0_f32;
                }
                let distance = max_distance - random.next_int(max_distance / 4);
                self.carve_cave_tunnel(
                    carver,
                    kind,
                    kind.top_level_tunnel_y_scale(),
                    random.next_long(),
                    x,
                    y,
                    z,
                    horizontal_multiplier,
                    vertical_multiplier,
                    thickness,
                    horizontal_rotation,
                    vertical_rotation,
                    0,
                    distance,
                    floor,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_cave_tunnel(
        &mut self,
        carver: &CompiledCarver,
        kind: CaveRuntimeKind,
        tunnel_y_scale: f64,
        tunnel_seed: i64,
        mut x: f64,
        mut y: f64,
        mut z: f64,
        horizontal_multiplier: f64,
        vertical_multiplier: f64,
        thickness: f32,
        mut horizontal_rotation: f32,
        mut vertical_rotation: f32,
        step: i32,
        distance: i32,
        floor: f64,
    ) {
        let mut random = WorldgenRandom::with_seed(tunnel_seed);
        let split_point = random.next_int(distance / 2) + distance / 4;
        let steep = random.next_int(6) == 0;
        let mut y_rotation_delta = 0.0_f32;
        let mut x_rotation_delta = 0.0_f32;
        for current_step in step..distance {
            let angle = std::f32::consts::PI * current_step as f32 / distance as f32;
            let horizontal_radius = 1.5 + f64::from(mth_sin(f64::from(angle)) * thickness);
            let vertical_radius = horizontal_radius * tunnel_y_scale;
            let cos_vertical = mth_cos(f64::from(vertical_rotation));
            x += f64::from(mth_cos(f64::from(horizontal_rotation)) * cos_vertical);
            y += f64::from(mth_sin(f64::from(vertical_rotation)));
            z += f64::from(mth_sin(f64::from(horizontal_rotation)) * cos_vertical);
            vertical_rotation *= if steep { 0.92_f32 } else { 0.7_f32 };
            vertical_rotation += x_rotation_delta * 0.1_f32;
            horizontal_rotation += y_rotation_delta * 0.1_f32;
            x_rotation_delta *= 0.9_f32;
            y_rotation_delta *= 0.75_f32;
            x_rotation_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 2.0_f32;
            y_rotation_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 4.0_f32;
            if current_step == split_point && thickness > 1.0_f32 {
                // Recursive branches reset yScale to 1.0 (`CaveWorldCarver.java:117-165`).
                let split_y_scale = 1.0;
                let left_seed = random.next_long();
                let left_thickness = random.next_float() * 0.5_f32 + 0.5_f32;
                self.carve_cave_tunnel(
                    carver,
                    kind,
                    split_y_scale,
                    left_seed,
                    x,
                    y,
                    z,
                    horizontal_multiplier,
                    vertical_multiplier,
                    left_thickness,
                    horizontal_rotation - std::f32::consts::FRAC_PI_2,
                    vertical_rotation / 3.0_f32,
                    current_step,
                    distance,
                    floor,
                );
                let right_seed = random.next_long();
                let right_thickness = random.next_float() * 0.5_f32 + 0.5_f32;
                self.carve_cave_tunnel(
                    carver,
                    kind,
                    split_y_scale,
                    right_seed,
                    x,
                    y,
                    z,
                    horizontal_multiplier,
                    vertical_multiplier,
                    right_thickness,
                    horizontal_rotation + std::f32::consts::FRAC_PI_2,
                    vertical_rotation / 3.0_f32,
                    current_step,
                    distance,
                    floor,
                );
                return;
            }
            if random.next_int(4) != 0 {
                if !self.can_reach(x, z, current_step, distance, thickness) {
                    return;
                }
                self.carve_ellipsoid(
                    carver,
                    kind,
                    x,
                    y,
                    z,
                    horizontal_radius * horizontal_multiplier,
                    vertical_radius * vertical_multiplier,
                    SkipChecker::Cave { floor },
                );
            }
        }
    }

    fn carve_canyon(
        &mut self,
        carver: &CompiledCarver,
        vertical_rotation: &FloatProvider,
        shape: &CanyonShape,
        random: &mut WorldgenRandom,
        source_x: i32,
        source_z: i32,
    ) {
        // `CanyonWorldCarver.java:20-155`.
        let x = f64::from(source_x.wrapping_mul(16).wrapping_add(random.next_int(16)));
        let y = f64::from(
            sample_height_provider_with_bounds(
                &carver.config.y,
                self.generation_min_y,
                self.generation_height,
                random,
            )
            .expect("validated carver height provider samples"),
        );
        let z = f64::from(source_z.wrapping_mul(16).wrapping_add(random.next_int(16)));
        let horizontal_rotation = random.next_float() * std::f32::consts::TAU;
        let vertical_rotation = sample_float_provider(vertical_rotation, random);
        let y_scale = f64::from(sample_float_provider(&carver.config.y_scale, random));
        let thickness = sample_float_provider(&shape.thickness, random);
        let distance = (112.0_f32 * sample_float_provider(&shape.distance_factor, random)) as i32;
        let tunnel_seed = random.next_long();
        let mut tunnel_random = WorldgenRandom::with_seed(tunnel_seed);
        let mut width_factors = vec![0.0_f32; self.generation_height as usize];
        let mut width_factor = 1.0_f32;
        for (index, value) in width_factors.iter_mut().enumerate() {
            if index == 0 || tunnel_random.next_int(shape.width_smoothness) == 0 {
                width_factor = 1.0_f32 + tunnel_random.next_float() * tunnel_random.next_float();
            }
            *value = width_factor * width_factor;
        }
        self.carve_canyon_tunnel(
            carver,
            shape,
            &mut tunnel_random,
            &width_factors,
            x,
            y,
            z,
            thickness,
            horizontal_rotation,
            vertical_rotation,
            distance,
            y_scale,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_canyon_tunnel(
        &mut self,
        carver: &CompiledCarver,
        shape: &CanyonShape,
        random: &mut WorldgenRandom,
        width_factors: &[f32],
        mut x: f64,
        mut y: f64,
        mut z: f64,
        thickness: f32,
        mut horizontal_rotation: f32,
        mut vertical_rotation: f32,
        distance: i32,
        y_scale: f64,
    ) {
        let mut y_rotation_delta = 0.0_f32;
        let mut x_rotation_delta = 0.0_f32;
        for current_step in 0..distance {
            let angle = current_step as f32 * std::f32::consts::PI / distance as f32;
            let mut horizontal_radius = 1.5 + f64::from(mth_sin(f64::from(angle)) * thickness);
            let mut vertical_radius = horizontal_radius * y_scale;
            horizontal_radius *= f64::from(sample_float_provider(
                &shape.horizontal_radius_factor,
                random,
            ));
            let vertical_multiplier =
                1.0_f32 - (0.5_f32 - current_step as f32 / distance as f32).abs() * 2.0_f32;
            let factor = shape.vertical_radius_default_factor
                + shape.vertical_radius_center_factor * vertical_multiplier;
            let random_factor = 0.75_f32 + random.next_float() * 0.25_f32;
            vertical_radius = f64::from(factor) * vertical_radius * f64::from(random_factor);
            let cos_vertical = mth_cos(f64::from(vertical_rotation));
            x += f64::from(mth_cos(f64::from(horizontal_rotation)) * cos_vertical);
            y += f64::from(mth_sin(f64::from(vertical_rotation)));
            z += f64::from(mth_sin(f64::from(horizontal_rotation)) * cos_vertical);
            vertical_rotation *= 0.7_f32;
            vertical_rotation += x_rotation_delta * 0.05_f32;
            horizontal_rotation += y_rotation_delta * 0.05_f32;
            x_rotation_delta *= 0.8_f32;
            y_rotation_delta *= 0.5_f32;
            x_rotation_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 2.0_f32;
            y_rotation_delta +=
                (random.next_float() - random.next_float()) * random.next_float() * 4.0_f32;
            if random.next_int(4) != 0 {
                if !self.can_reach(x, z, current_step, distance, thickness) {
                    return;
                }
                self.carve_ellipsoid(
                    carver,
                    CaveRuntimeKind::Overworld,
                    x,
                    y,
                    z,
                    horizontal_radius,
                    vertical_radius,
                    SkipChecker::Canyon { width_factors },
                );
            }
        }
    }

    pub fn can_reach(
        &self,
        x: f64,
        z: f64,
        current_step: i32,
        total_steps: i32,
        thickness: f32,
    ) -> bool {
        let center_x = f64::from(self.chunk_x.wrapping_mul(16).wrapping_add(8));
        let center_z = f64::from(self.chunk_z.wrapping_mul(16).wrapping_add(8));
        let dx = x - center_x;
        let dz = z - center_z;
        let remaining = f64::from(total_steps - current_step);
        let radius = f64::from(thickness + 2.0_f32 + 16.0_f32);
        dx * dx + dz * dz - remaining * remaining <= radius * radius
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_ellipsoid(
        &mut self,
        carver: &CompiledCarver,
        kind: CaveRuntimeKind,
        x: f64,
        y: f64,
        z: f64,
        horizontal_radius: f64,
        vertical_radius: f64,
        skip: SkipChecker<'_>,
    ) -> bool {
        // `WorldCarver.java:62-188`.
        let center_x = f64::from(self.chunk_x.wrapping_mul(16).wrapping_add(8));
        let center_z = f64::from(self.chunk_z.wrapping_mul(16).wrapping_add(8));
        let max_delta = 16.0 + horizontal_radius * 2.0;
        if (x - center_x).abs() > max_delta || (z - center_z).abs() > max_delta {
            return false;
        }
        let chunk_min_x = self.chunk_x.wrapping_mul(16);
        let chunk_min_z = self.chunk_z.wrapping_mul(16);
        let min_x = (mth_floor_f64(x - horizontal_radius) - chunk_min_x - 1).max(0);
        let max_x = (mth_floor_f64(x + horizontal_radius) - chunk_min_x).min(15);
        let min_y = (mth_floor_f64(y - vertical_radius) - 1).max(self.generation_min_y + 1);
        let protected_top = if self.upgrading { 0 } else { 7 };
        let max_y = (mth_floor_f64(y + vertical_radius) + 1)
            .min(self.generation_min_y + self.generation_height - 1 - protected_top);
        let min_z = (mth_floor_f64(z - horizontal_radius) - chunk_min_z - 1).max(0);
        let max_z = (mth_floor_f64(z + horizontal_radius) - chunk_min_z).min(15);
        let mut carved = false;
        for local_x in min_x..=max_x {
            let world_x = chunk_min_x + local_x;
            let dx = (f64::from(world_x) + 0.5 - x) / horizontal_radius;
            for local_z in min_z..=max_z {
                let world_z = chunk_min_z + local_z;
                let dz = (f64::from(world_z) + 0.5 - z) / horizontal_radius;
                if dx * dx + dz * dz >= 1.0 {
                    continue;
                }
                let mut has_grass = false;
                for world_y in (min_y + 1..=max_y).rev() {
                    let dy = (f64::from(world_y) - 0.5 - y) / vertical_radius;
                    if skip.should_skip(self.generation_min_y, dx, dy, dz, world_y) {
                        continue;
                    }
                    let first_visit = self.mask.set_if_absent(local_x, world_y, local_z);
                    if !first_visit && carver.debug.is_none() {
                        continue;
                    }
                    carved |= match kind {
                        CaveRuntimeKind::Overworld => self.carve_block(
                            carver,
                            local_x,
                            world_y,
                            local_z,
                            world_x,
                            world_z,
                            &mut has_grass,
                        ),
                        CaveRuntimeKind::Nether => self.carve_nether_block(
                            &carver.replaceable,
                            local_x,
                            world_y,
                            local_z,
                            world_x,
                            world_z,
                        ),
                    };
                }
            }
        }
        carved
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_block(
        &mut self,
        carver: &CompiledCarver,
        local_x: i32,
        y: i32,
        local_z: i32,
        world_x: i32,
        world_z: i32,
        has_grass: &mut bool,
    ) -> bool {
        let old = self
            .column
            .get_block_state(local_x, y, local_z)
            .unwrap_or(self.source.air_block_state_id());
        let old_type = vanilla_block_state_block_type_id(old);
        if matches!(
            old_type,
            Some(VANILLA_BLOCK_GRASS_BLOCK_ID | VANILLA_BLOCK_MYCELIUM_ID)
        ) {
            *has_grass = true;
        }
        if !old_type.is_some_and(|block| carver.replaceable.contains(&block))
            && carver.debug.is_none()
        {
            return false;
        }

        let lava_level = resolve_anchor(
            carver.config.lava_level,
            self.generation_min_y,
            self.generation_height,
        );
        let state = if y <= lava_level {
            self.source.lava_block_state_id()
        } else {
            let chunk_start_x = self.chunk_x.wrapping_mul(16);
            let chunk_start_z = self.chunk_z.wrapping_mul(16);
            let sampler = self.sampler.get_or_insert_with(|| {
                NoiseChunkSampler::for_chunk(
                    self.source,
                    chunk_start_x,
                    chunk_start_z,
                    self.generation_min_y,
                    self.generation_height,
                )
            });
            let aquifer = self.aquifer.get_or_insert_with(|| {
                Aquifer::for_noise_chunk(
                    sampler,
                    chunk_start_x,
                    chunk_start_z,
                    self.generation_min_y,
                    self.generation_height,
                )
            });
            let decision = aquifer.compute_with_sampler(sampler, world_x, y, world_z, 0.0);
            self.last_schedule_fluid_update = decision.schedule_fluid_update;
            let Some(state) = debug_carve_state(carver.debug, decision.state) else {
                return false;
            };
            state
        };
        let column_min_y = self.column.min_y();
        set_generated_block(
            self.column,
            self.ocean_floor_wg,
            self.world_surface_wg,
            self.postprocessing,
            local_x,
            y,
            local_z,
            world_x,
            world_z,
            state,
            column_min_y,
            self.last_schedule_fluid_update,
        );

        if *has_grass {
            let below_y = y - 1;
            let below = self
                .column
                .get_block_state(local_x, below_y, local_z)
                .unwrap_or(self.source.air_block_state_id());
            if vanilla_block_state_block_type_id(below) == Some(VANILLA_BLOCK_DIRT_ID) {
                let top = self.surface.top_material_cached(
                    self.source,
                    self.biome_source,
                    &mut self.search,
                    self.world_surface_wg,
                    &mut self.preliminary_corners,
                    world_x,
                    below_y,
                    world_z,
                    !block_state_fluid_state(state).is_empty(),
                    self.upgrading,
                );
                if let Some(top) = top {
                    set_generated_block(
                        self.column,
                        self.ocean_floor_wg,
                        self.world_surface_wg,
                        self.postprocessing,
                        local_x,
                        below_y,
                        local_z,
                        world_x,
                        world_z,
                        top,
                        column_min_y,
                        true,
                    );
                }
            }
        }
        true
    }

    fn carve_nether_block(
        &mut self,
        replaceable: &BTreeSet<i32>,
        local_x: i32,
        y: i32,
        local_z: i32,
        world_x: i32,
        world_z: i32,
    ) -> bool {
        // `NetherWorldCarver.java:17-62` bypasses the aquifer and surface repair.
        let old = self
            .column
            .get_block_state(local_x, y, local_z)
            .unwrap_or(self.source.air_block_state_id());
        if !vanilla_block_state_block_type_id(old).is_some_and(|block| replaceable.contains(&block))
        {
            return false;
        }
        let state = if y <= self.generation_min_y.saturating_add(31) {
            self.source.lava_block_state_id()
        } else {
            self.cave_air
        };
        let column_min_y = self.column.min_y();
        set_generated_block(
            self.column,
            self.ocean_floor_wg,
            self.world_surface_wg,
            self.postprocessing,
            local_x,
            y,
            local_z,
            world_x,
            world_z,
            state,
            column_min_y,
            false,
        );
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaveRuntimeKind {
    Overworld,
    Nether,
}

impl CaveRuntimeKind {
    const fn cave_bound(self) -> i32 {
        match self {
            Self::Overworld => 15,
            Self::Nether => 10,
        }
    }

    const fn thickness_scale(self) -> f32 {
        match self {
            Self::Overworld => 1.0,
            Self::Nether => 2.0,
        }
    }

    const fn top_level_tunnel_y_scale(self) -> f64 {
        match self {
            Self::Overworld => 1.0,
            Self::Nether => 5.0,
        }
    }
}

pub enum SkipChecker<'a> {
    Cave { floor: f64 },
    Canyon { width_factors: &'a [f32] },
}

impl SkipChecker<'_> {
    pub fn should_skip(&self, min_y: i32, dx: f64, dy: f64, dz: f64, y: i32) -> bool {
        match self {
            Self::Cave { floor } => dy <= *floor || dx * dx + dy * dy + dz * dz >= 1.0,
            Self::Canyon { width_factors } => {
                let index = usize::try_from(y - min_y - 1)
                    .expect("ellipsoid bounds keep canyon width-factor indices valid");
                (dx * dx + dz * dz) * f64::from(width_factors[index]) + dy * dy / 6.0 >= 1.0
            }
        }
    }
}

pub struct CarvingMask {
    min_y: i32,
    words: Vec<u64>,
}

impl CarvingMask {
    pub fn new(min_y: i32, height: i32) -> Self {
        let bit_count = usize::try_from(height * 256).expect("validated world height fits mask");
        Self {
            min_y,
            words: vec![0; bit_count.div_ceil(64)],
        }
    }

    pub fn set_if_absent(&mut self, x: i32, y: i32, z: i32) -> bool {
        let index = usize::try_from((x & 15) | ((z & 15) << 4) | ((y - self.min_y) << 8))
            .expect("carving coordinates stay in the target height");
        let word = index / 64;
        let bit = 1_u64 << (index % 64);
        if self.words[word] & bit != 0 {
            false
        } else {
            self.words[word] |= bit;
            true
        }
    }

    pub fn into_trimmed_longs(mut self) -> Vec<i64> {
        while self.words.last() == Some(&0) {
            self.words.pop();
        }
        self.words.into_iter().map(|word| word as i64).collect()
    }
}

/// `CarverDebugSettings` remaps aquifer results while debug mode is on
/// (`WorldCarver.java:167-189`).
fn debug_carve_state(debug: Option<DebugStates>, state: Option<i32>) -> Option<i32> {
    let Some(debug) = debug else {
        return state;
    };
    let Some(state) = state else {
        return Some(debug.barrier);
    };
    Some(match vanilla_block_state_block_type_id(state) {
        Some(VANILLA_BLOCK_AIR_ID) => debug.air,
        Some(VANILLA_BLOCK_WATER_ID) => debug.water,
        Some(VANILLA_BLOCK_LAVA_ID) => debug.lava,
        _ => state,
    })
}

fn resolve_anchor(anchor: VerticalAnchor, min_y: i32, height: i32) -> i32 {
    match anchor {
        VerticalAnchor::Absolute(y) => y,
        VerticalAnchor::AboveBottom(offset) => min_y + offset,
        VerticalAnchor::BelowTop(offset) => min_y + height - 1 - offset,
    }
}

fn normalize_id(raw: &str, path: &str) -> Result<String, NoiseDensityError> {
    bbb_protocol::normalize_resource_location(raw.to_string())
        .map_err(|source| error(path, format!("invalid resource location: {source}")))
}

pub fn error(path: impl Into<String>, message: impl Into<String>) -> NoiseDensityError {
    NoiseDensityError {
        path: path.into(),
        message: message.into(),
    }
}

#[cfg(test)]
pub mod tests;
