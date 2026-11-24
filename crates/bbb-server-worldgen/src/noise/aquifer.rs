//! Chunk-scoped enabled and disabled noise aquifers
//! (`Aquifer.java:58-513`).

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;

use super::density::chunk::NoiseChunkSampler;
use super::density::{NoiseDensitySource, Router};
use crate::fast_hash::FastHashMap;
use crate::random::PositionalRandom;

enum DensityRouter<'sampler, 'source> {
    Unbound(&'source NoiseDensitySource),
    Bound(&'sampler mut NoiseChunkSampler<'source>),
}

impl DensityRouter<'_, '_> {
    fn source(&self) -> &NoiseDensitySource {
        match self {
            Self::Unbound(source) => source,
            Self::Bound(sampler) => sampler.source(),
        }
    }

    fn preliminary_surface_level(&mut self, x: i32, z: i32) -> i32 {
        match self {
            Self::Unbound(source) => source.preliminary_surface_level(x, z),
            Self::Bound(sampler) => sampler.preliminary_surface_level(x, z),
        }
    }

    fn sample(&mut self, field: Router, x: i32, y: i32, z: i32) -> f64 {
        match self {
            Self::Unbound(source) => source.sample_router(field, x, y, z),
            Self::Bound(sampler) => sampler.aquifer(field, x, y, z),
        }
    }

    fn barrier(&mut self, x: i32, y: i32, z: i32) -> f64 {
        self.sample(Router::Barrier, x, y, z)
    }

    fn fluid_level_floodedness(&mut self, x: i32, y: i32, z: i32) -> f64 {
        self.sample(Router::FluidLevelFloodedness, x, y, z)
    }

    fn fluid_level_spread(&mut self, x: i32, y: i32, z: i32) -> f64 {
        self.sample(Router::FluidLevelSpread, x, y, z)
    }

    fn lava(&mut self, x: i32, y: i32, z: i32) -> f64 {
        self.sample(Router::Lava, x, y, z)
    }

    fn erosion(&mut self, x: i32, y: i32, z: i32) -> f64 {
        self.sample(Router::Erosion, x, y, z)
    }

    fn depth(&mut self, x: i32, y: i32, z: i32) -> f64 {
        self.sample(Router::Depth, x, y, z)
    }
}

pub const WAY_BELOW_MIN_Y: i32 = -32_512;
pub const FLOWING_UPDATE_SIMILARITY: f64 = 1.0 - (144.0 - 100.0) / 25.0;
pub const SURFACE_OFFSETS: [(i32, i32); 13] = [
    (0, 0),
    (-2, -1),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (1, 0),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluidStatus {
    level: i32,
    state: i32,
}

impl FluidStatus {
    pub fn at(self, y: i32, air: i32) -> i32 {
        if y < self.level {
            self.state
        } else {
            air
        }
    }
}

pub struct AquiferDecision {
    pub state: Option<i32>,
    pub schedule_fluid_update: bool,
}

pub enum Aquifer {
    Disabled,
    Enabled(NoiseAquifer),
}

impl Aquifer {
    pub fn for_chunk(
        source: &NoiseDensitySource,
        chunk_start_x: i32,
        chunk_start_z: i32,
        min_y: i32,
        height: i32,
    ) -> Self {
        if source.aquifers_enabled() {
            Self::Enabled(NoiseAquifer::new(
                source,
                chunk_start_x,
                chunk_start_z,
                min_y,
                height,
            ))
        } else {
            Self::Disabled
        }
    }

    pub fn for_noise_chunk(
        sampler: &mut NoiseChunkSampler<'_>,
        chunk_start_x: i32,
        chunk_start_z: i32,
        min_y: i32,
        height: i32,
    ) -> Self {
        if sampler.source().aquifers_enabled() {
            let mut router = DensityRouter::Bound(sampler);
            Self::Enabled(NoiseAquifer::new_with_router(
                &mut router,
                chunk_start_x,
                chunk_start_z,
                min_y,
                height,
            ))
        } else {
            Self::Disabled
        }
    }

    pub fn compute(
        &mut self,
        source: &NoiseDensitySource,
        x: i32,
        y: i32,
        z: i32,
        density: f64,
    ) -> AquiferDecision {
        let mut router = DensityRouter::Unbound(source);
        self.compute_with_router(&mut router, x, y, z, density)
    }

    pub fn compute_with_sampler(
        &mut self,
        sampler: &mut NoiseChunkSampler<'_>,
        x: i32,
        y: i32,
        z: i32,
        density: f64,
    ) -> AquiferDecision {
        let mut router = DensityRouter::Bound(sampler);
        self.compute_with_router(&mut router, x, y, z, density)
    }

    fn compute_with_router(
        &mut self,
        router: &mut DensityRouter<'_, '_>,
        x: i32,
        y: i32,
        z: i32,
        density: f64,
    ) -> AquiferDecision {
        match self {
            Self::Disabled => {
                if density > 0.0 {
                    AquiferDecision {
                        state: None,
                        schedule_fluid_update: false,
                    }
                } else {
                    AquiferDecision {
                        state: Some(
                            global_fluid(router.source(), y)
                                .at(y, router.source().air_block_state_id()),
                        ),
                        schedule_fluid_update: false,
                    }
                }
            }
            Self::Enabled(aquifer) => aquifer.compute_with_router(router, x, y, z, density),
        }
    }
}

pub struct NoiseAquifer {
    random: PositionalRandom,
    min_grid_x: i32,
    min_grid_y: i32,
    min_grid_z: i32,
    grid_size_x: i32,
    grid_size_z: i32,
    statuses: Vec<Option<FluidStatus>>,
    locations: Vec<Option<i64>>,
    preliminary_surfaces: FastHashMap<(i32, i32), i32>,
    skip_sampling_above_y: i32,
}

impl NoiseAquifer {
    pub fn new(
        source: &NoiseDensitySource,
        chunk_start_x: i32,
        chunk_start_z: i32,
        min_y: i32,
        height: i32,
    ) -> Self {
        let mut router = DensityRouter::Unbound(source);
        Self::new_with_router(&mut router, chunk_start_x, chunk_start_z, min_y, height)
    }

    fn new_with_router(
        router: &mut DensityRouter<'_, '_>,
        chunk_start_x: i32,
        chunk_start_z: i32,
        min_y: i32,
        height: i32,
    ) -> Self {
        let min_grid_x = grid_x(chunk_start_x.wrapping_sub(5));
        let max_grid_x = grid_x(chunk_start_x.wrapping_add(10)).wrapping_add(1);
        let grid_size_x = max_grid_x - min_grid_x + 1;
        let min_grid_y = grid_y(min_y.wrapping_add(1)).wrapping_sub(1);
        let max_grid_y = grid_y(min_y.wrapping_add(height).wrapping_add(1)).wrapping_add(1);
        let grid_size_y = max_grid_y - min_grid_y + 1;
        let min_grid_z = grid_z(chunk_start_z.wrapping_sub(5));
        let max_grid_z = grid_z(chunk_start_z.wrapping_add(10)).wrapping_add(1);
        let grid_size_z = max_grid_z - min_grid_z + 1;
        let cache_len = usize::try_from(grid_size_x * grid_size_y * grid_size_z)
            .expect("validated noise dimensions produce a bounded aquifer cache");
        let mut aquifer = Self {
            random: PositionalRandom::new(router.source().world_seed(), "minecraft:aquifer"),
            min_grid_x,
            min_grid_y,
            min_grid_z,
            grid_size_x,
            grid_size_z,
            statuses: vec![None; cache_len],
            locations: vec![None; cache_len],
            preliminary_surfaces: FastHashMap::default(),
            skip_sampling_above_y: 0,
        };
        let max_surface = aquifer.max_preliminary_surface_with_router(
            router,
            from_grid_x(min_grid_x, 0),
            from_grid_z(min_grid_z, 0),
            from_grid_x(max_grid_x, 9),
            from_grid_z(max_grid_z, 9),
        );
        let skip_grid_y = grid_y(max_surface.wrapping_add(8).wrapping_add(12)).wrapping_add(1);
        aquifer.skip_sampling_above_y = from_grid_y(skip_grid_y, 11).wrapping_sub(1);
        aquifer
    }

    pub fn compute(
        &mut self,
        source: &NoiseDensitySource,
        x: i32,
        y: i32,
        z: i32,
        density: f64,
    ) -> AquiferDecision {
        let mut router = DensityRouter::Unbound(source);
        self.compute_with_router(&mut router, x, y, z, density)
    }

    fn compute_with_router(
        &mut self,
        router: &mut DensityRouter<'_, '_>,
        x: i32,
        y: i32,
        z: i32,
        density: f64,
    ) -> AquiferDecision {
        if density > 0.0 {
            return decision(None, false);
        }
        let global = global_fluid(router.source(), y);
        if y > self.skip_sampling_above_y {
            return decision(
                Some(global.at(y, router.source().air_block_state_id())),
                false,
            );
        }
        if block_is(
            global.at(y, router.source().air_block_state_id()),
            router.source().lava_block_state_id(),
        ) {
            return decision(Some(router.source().lava_block_state_id()), false);
        }

        let x_anchor = grid_x(x.wrapping_sub(5));
        let y_anchor = grid_y(y.wrapping_add(1));
        let z_anchor = grid_z(z.wrapping_sub(5));
        let mut closest = [(i32::MAX, 0_usize); 4];
        for dx in 0..=1 {
            for dy in -1..=1 {
                for dz in 0..=1 {
                    let grid_x = x_anchor + dx;
                    let grid_y = y_anchor + dy;
                    let grid_z = z_anchor + dz;
                    let index = self.index(grid_x, grid_y, grid_z);
                    let packed = self.location(grid_x, grid_y, grid_z, index);
                    let (center_x, center_y, center_z) = unpack_block_pos(packed);
                    let distance = center_x
                        .wrapping_sub(x)
                        .wrapping_mul(center_x.wrapping_sub(x))
                        .wrapping_add(
                            center_y
                                .wrapping_sub(y)
                                .wrapping_mul(center_y.wrapping_sub(y)),
                        )
                        .wrapping_add(
                            center_z
                                .wrapping_sub(z)
                                .wrapping_mul(center_z.wrapping_sub(z)),
                        );
                    insert_closest(&mut closest, distance, index);
                }
            }
        }

        let status1 = self.status_with_router(router, closest[0].1);
        let similarity12 = similarity(closest[0].0, closest[1].0);
        let fluid_state = status1.at(y, router.source().air_block_state_id());
        if similarity12 <= 0.0 {
            let schedule = similarity12 >= FLOWING_UPDATE_SIMILARITY
                && status1 != self.status_with_router(router, closest[1].1);
            return decision(Some(fluid_state), schedule);
        }
        // `Aquifer.java:233-252` matches water and lava by block identity.
        if block_is(fluid_state, router.source().water_block_state_id())
            && block_is(
                global_fluid(router.source(), y.wrapping_sub(1))
                    .at(y.wrapping_sub(1), router.source().air_block_state_id()),
                router.source().lava_block_state_id(),
            )
        {
            return decision(Some(fluid_state), true);
        }

        let status2 = self.status_with_router(router, closest[1].1);
        let mut barrier_noise = None;
        let barrier12 = similarity12
            * pressure_with_router(router, x, y, z, status1, status2, &mut barrier_noise);
        if density + barrier12 > 0.0 {
            return decision(None, false);
        }
        let status3 = self.status_with_router(router, closest[2].1);
        let similarity13 = similarity(closest[0].0, closest[2].0);
        if similarity13 > 0.0 {
            let barrier13 = similarity12
                * similarity13
                * pressure_with_router(router, x, y, z, status1, status3, &mut barrier_noise);
            if density + barrier13 > 0.0 {
                return decision(None, false);
            }
        }
        let similarity23 = similarity(closest[1].0, closest[2].0);
        if similarity23 > 0.0 {
            let barrier23 = similarity12
                * similarity23
                * pressure_with_router(router, x, y, z, status2, status3, &mut barrier_noise);
            if density + barrier23 > 0.0 {
                return decision(None, false);
            }
        }
        let may_flow12 = status1 != status2;
        let may_flow23 = similarity23 >= FLOWING_UPDATE_SIMILARITY && status2 != status3;
        let may_flow13 = similarity13 >= FLOWING_UPDATE_SIMILARITY && status1 != status3;
        let schedule = if may_flow12 || may_flow23 || may_flow13 {
            true
        } else {
            similarity13 >= FLOWING_UPDATE_SIMILARITY
                && similarity(closest[0].0, closest[3].0) >= FLOWING_UPDATE_SIMILARITY
                && status1 != self.status_with_router(router, closest[3].1)
        };
        decision(Some(fluid_state), schedule)
    }

    pub fn index(&self, grid_x: i32, grid_y: i32, grid_z: i32) -> usize {
        let x = grid_x - self.min_grid_x;
        let y = grid_y - self.min_grid_y;
        let z = grid_z - self.min_grid_z;
        usize::try_from((y * self.grid_size_z + z) * self.grid_size_x + x)
            .expect("aquifer sample stays inside its chunk-scoped cache")
    }

    pub fn location(&mut self, grid_x: i32, grid_y: i32, grid_z: i32, index: usize) -> i64 {
        if let Some(location) = self.locations[index] {
            return location;
        }
        let mut random = self.random.at(grid_x, grid_y, grid_z);
        let location = pack_block_pos(
            from_grid_x(grid_x, random.next_int(10)),
            from_grid_y(grid_y, random.next_int(9)),
            from_grid_z(grid_z, random.next_int(10)),
        );
        self.locations[index] = Some(location);
        location
    }

    pub fn status(&mut self, source: &NoiseDensitySource, index: usize) -> FluidStatus {
        let mut router = DensityRouter::Unbound(source);
        self.status_with_router(&mut router, index)
    }

    fn status_with_router(
        &mut self,
        router: &mut DensityRouter<'_, '_>,
        index: usize,
    ) -> FluidStatus {
        if let Some(status) = self.statuses[index] {
            return status;
        }
        let (x, y, z) = unpack_block_pos(
            self.locations[index].expect("a nearest aquifer location was initialized"),
        );
        let status = self.compute_fluid_with_router(router, x, y, z);
        self.statuses[index] = Some(status);
        status
    }

    pub fn compute_fluid(
        &mut self,
        source: &NoiseDensitySource,
        x: i32,
        y: i32,
        z: i32,
    ) -> FluidStatus {
        let mut router = DensityRouter::Unbound(source);
        self.compute_fluid_with_router(&mut router, x, y, z)
    }

    fn compute_fluid_with_router(
        &mut self,
        router: &mut DensityRouter<'_, '_>,
        x: i32,
        y: i32,
        z: i32,
    ) -> FluidStatus {
        let global = global_fluid(router.source(), y);
        let mut lowest_surface = i32::MAX;
        let top = y.wrapping_add(12);
        let bottom = y.wrapping_sub(12);
        let mut center_under_global = false;
        for (offset_x, offset_z) in SURFACE_OFFSETS {
            let sample_x = x.wrapping_add(offset_x.wrapping_mul(16));
            let sample_z = z.wrapping_add(offset_z.wrapping_mul(16));
            let surface = self.preliminary_surface_with_router(router, sample_x, sample_z);
            let adjusted = surface.wrapping_add(8);
            let center = offset_x == 0 && offset_z == 0;
            if center && bottom > adjusted {
                return global;
            }
            let top_above_surface = top > adjusted;
            if top_above_surface || center {
                let at_surface = global_fluid(router.source(), adjusted);
                if at_surface.at(adjusted, router.source().air_block_state_id())
                    != router.source().air_block_state_id()
                {
                    if center {
                        center_under_global = true;
                    }
                    if top_above_surface {
                        return at_surface;
                    }
                }
            }
            lowest_surface = lowest_surface.min(surface);
        }
        let level =
            surface_level_with_router(router, x, y, z, global, lowest_surface, center_under_global);
        FluidStatus {
            level,
            state: fluid_type_with_router(router, x, y, z, global, level),
        }
    }

    pub fn preliminary_surface(&mut self, source: &NoiseDensitySource, x: i32, z: i32) -> i32 {
        let mut router = DensityRouter::Unbound(source);
        self.preliminary_surface_with_router(&mut router, x, z)
    }

    fn preliminary_surface_with_router(
        &mut self,
        router: &mut DensityRouter<'_, '_>,
        x: i32,
        z: i32,
    ) -> i32 {
        let quantized = (x - x.rem_euclid(4), z - z.rem_euclid(4));
        if let Some(surface) = self.preliminary_surfaces.get(&quantized) {
            return *surface;
        }
        let surface = router.preliminary_surface_level(quantized.0, quantized.1);
        self.preliminary_surfaces.insert(quantized, surface);
        surface
    }

    pub fn max_preliminary_surface(
        &mut self,
        source: &NoiseDensitySource,
        min_x: i32,
        min_z: i32,
        max_x: i32,
        max_z: i32,
    ) -> i32 {
        let mut router = DensityRouter::Unbound(source);
        self.max_preliminary_surface_with_router(&mut router, min_x, min_z, max_x, max_z)
    }

    fn max_preliminary_surface_with_router(
        &mut self,
        router: &mut DensityRouter<'_, '_>,
        min_x: i32,
        min_z: i32,
        max_x: i32,
        max_z: i32,
    ) -> i32 {
        let mut max_y = i32::MIN;
        let mut z = min_z;
        while z <= max_z {
            let mut x = min_x;
            while x <= max_x {
                max_y = max_y.max(self.preliminary_surface_with_router(router, x, z));
                x = x.wrapping_add(4);
            }
            z = z.wrapping_add(4);
        }
        max_y
    }
}

pub fn surface_level(
    source: &NoiseDensitySource,
    x: i32,
    y: i32,
    z: i32,
    global: FluidStatus,
    lowest_surface: i32,
    center_under_global: bool,
) -> i32 {
    let mut router = DensityRouter::Unbound(source);
    surface_level_with_router(
        &mut router,
        x,
        y,
        z,
        global,
        lowest_surface,
        center_under_global,
    )
}

fn surface_level_with_router(
    router: &mut DensityRouter<'_, '_>,
    x: i32,
    y: i32,
    z: i32,
    global: FluidStatus,
    lowest_surface: i32,
    center_under_global: bool,
) -> i32 {
    let (partially_flooded, fully_flooded) = if router.erosion(x, y, z) < f64::from(-0.225_f32)
        && router.depth(x, y, z) > f64::from(0.9_f32)
    {
        (-1.0, -1.0)
    } else {
        let distance_below_surface = lowest_surface.wrapping_add(8).wrapping_sub(y);
        let floodedness_factor = if center_under_global {
            clamped_map(f64::from(distance_below_surface), 0.0, 64.0, 1.0, 0.0)
        } else {
            0.0
        };
        let floodedness = router.fluid_level_floodedness(x, y, z).clamp(-1.0, 1.0);
        let fully_threshold = map(floodedness_factor, 1.0, 0.0, -0.3, 0.8);
        let partially_threshold = map(floodedness_factor, 1.0, 0.0, -0.8, 0.4);
        (
            floodedness - partially_threshold,
            floodedness - fully_threshold,
        )
    };
    if fully_flooded > 0.0 {
        global.level
    } else if partially_flooded > 0.0 {
        randomized_surface_level_with_router(router, x, y, z, lowest_surface)
    } else {
        WAY_BELOW_MIN_Y
    }
}

pub fn randomized_surface_level(
    source: &NoiseDensitySource,
    x: i32,
    y: i32,
    z: i32,
    lowest_surface: i32,
) -> i32 {
    let mut router = DensityRouter::Unbound(source);
    randomized_surface_level_with_router(&mut router, x, y, z, lowest_surface)
}

fn randomized_surface_level_with_router(
    router: &mut DensityRouter<'_, '_>,
    x: i32,
    y: i32,
    z: i32,
    lowest_surface: i32,
) -> i32 {
    let cell_x = x.div_euclid(16);
    let cell_y = y.div_euclid(40);
    let cell_z = z.div_euclid(16);
    let middle_y = cell_y.wrapping_mul(40).wrapping_add(20);
    let spread = router.fluid_level_spread(cell_x, cell_y, cell_z) * 10.0;
    lowest_surface.min(middle_y.wrapping_add(quantize(spread, 3)))
}

pub fn fluid_type(
    source: &NoiseDensitySource,
    x: i32,
    y: i32,
    z: i32,
    global: FluidStatus,
    level: i32,
) -> i32 {
    let mut router = DensityRouter::Unbound(source);
    fluid_type_with_router(&mut router, x, y, z, global, level)
}

fn fluid_type_with_router(
    router: &mut DensityRouter<'_, '_>,
    x: i32,
    y: i32,
    z: i32,
    global: FluidStatus,
    level: i32,
) -> i32 {
    if level <= -10
        && level != WAY_BELOW_MIN_Y
        && !block_is(global.state, router.source().lava_block_state_id())
        && router
            .lava(x.div_euclid(64), y.div_euclid(40), z.div_euclid(64))
            .abs()
            > 0.3
    {
        router.source().lava_block_state_id()
    } else {
        global.state
    }
}

pub fn pressure(
    source: &NoiseDensitySource,
    x: i32,
    y: i32,
    z: i32,
    first: FluidStatus,
    second: FluidStatus,
    barrier_noise: &mut Option<f64>,
) -> f64 {
    let mut router = DensityRouter::Unbound(source);
    pressure_with_router(&mut router, x, y, z, first, second, barrier_noise)
}

fn pressure_with_router(
    router: &mut DensityRouter<'_, '_>,
    x: i32,
    y: i32,
    z: i32,
    first: FluidStatus,
    second: FluidStatus,
    barrier_noise: &mut Option<f64>,
) -> f64 {
    let first_state = first.at(y, router.source().air_block_state_id());
    let second_state = second.at(y, router.source().air_block_state_id());
    let water_lava = (block_is(first_state, router.source().lava_block_state_id())
        && block_is(second_state, router.source().water_block_state_id()))
        || (block_is(first_state, router.source().water_block_state_id())
            && block_is(second_state, router.source().lava_block_state_id()));
    if water_lava {
        return 2.0;
    }
    let level_difference = first.level.abs_diff(second.level) as f64;
    if level_difference == 0.0 {
        return 0.0;
    }
    let average_level = 0.5 * f64::from(first.level.wrapping_add(second.level));
    let above_average = f64::from(y) + 0.5 - average_level;
    let middle_distance = level_difference / 2.0 - above_average.abs();
    let gradient = if above_average > 0.0 {
        if middle_distance > 0.0 {
            middle_distance / 1.5
        } else {
            middle_distance / 2.5
        }
    } else {
        let center = 3.0 + middle_distance;
        if center > 0.0 {
            center / 3.0
        } else {
            center / 10.0
        }
    };
    let noise = if (-2.0..=2.0).contains(&gradient) {
        if let Some(noise) = *barrier_noise {
            noise
        } else {
            let noise = router.barrier(x, y, z);
            *barrier_noise = Some(noise);
            noise
        }
    } else {
        0.0
    };
    2.0 * (noise + gradient)
}

pub fn global_fluid(source: &NoiseDensitySource, y: i32) -> FluidStatus {
    // `NoiseBasedChunkGenerator.java:69-80` fixes deep lava, then uses the
    // configured default state at sea level.
    if y < source.sea_level().min(-54) {
        FluidStatus {
            level: -54,
            state: source.lava_block_state_id(),
        }
    } else {
        FluidStatus {
            level: source.sea_level(),
            state: source.default_fluid_state_id(),
        }
    }
}

fn block_is(state: i32, canonical_state: i32) -> bool {
    vanilla_block_state_block_type_id(state) == vanilla_block_state_block_type_id(canonical_state)
}

pub fn decision(state: Option<i32>, schedule_fluid_update: bool) -> AquiferDecision {
    AquiferDecision {
        state,
        schedule_fluid_update,
    }
}

pub fn insert_closest(closest: &mut [(i32, usize); 4], distance: i32, index: usize) {
    for slot in 0..4 {
        if closest[slot].0 >= distance {
            for shift in (slot + 1..4).rev() {
                closest[shift] = closest[shift - 1];
            }
            closest[slot] = (distance, index);
            break;
        }
    }
}

pub fn similarity(first: i32, second: i32) -> f64 {
    1.0 - f64::from(second - first) / 25.0
}

pub fn grid_x(block: i32) -> i32 {
    block >> 4
}

pub fn grid_y(block: i32) -> i32 {
    block.div_euclid(12)
}

pub fn grid_z(block: i32) -> i32 {
    block >> 4
}

pub fn from_grid_x(grid: i32, offset: i32) -> i32 {
    (grid << 4).wrapping_add(offset)
}

pub fn from_grid_y(grid: i32, offset: i32) -> i32 {
    grid.wrapping_mul(12).wrapping_add(offset)
}

pub fn from_grid_z(grid: i32, offset: i32) -> i32 {
    (grid << 4).wrapping_add(offset)
}

pub fn pack_block_pos(x: i32, y: i32, z: i32) -> i64 {
    let x = i64::from(x) & ((1_i64 << 26) - 1);
    let y = i64::from(y) & ((1_i64 << 12) - 1);
    let z = i64::from(z) & ((1_i64 << 26) - 1);
    (x << 38) | y | (z << 12)
}

pub fn unpack_block_pos(packed: i64) -> (i32, i32, i32) {
    (
        (packed >> 38) as i32,
        ((packed << 52) >> 52) as i32,
        ((packed << 26) >> 38) as i32,
    )
}

pub fn quantize(value: f64, resolution: i32) -> i32 {
    (value / f64::from(resolution)).floor() as i32 * resolution
}

pub fn map(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64 {
    to_min + (value - from_min) / (from_max - from_min) * (to_max - to_min)
}

pub fn clamped_map(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64 {
    let factor = ((value - from_min) / (from_max - from_min)).clamp(0.0, 1.0);
    to_min + factor * (to_max - to_min)
}

#[cfg(test)]
mod tests {
    use super::{grid_x, grid_y, grid_z, insert_closest, pack_block_pos, unpack_block_pos};

    #[test]
    fn packed_aquifer_locations_match_block_pos_signed_fields() {
        for position in [
            (0, 0, 0),
            (-5, -64, 17),
            (30_000_000, 319, -30_000_000),
            (-33_554_432, -2_048, 33_554_431),
            (33_554_431, 2_047, -33_554_432),
        ] {
            assert_eq!(
                unpack_block_pos(pack_block_pos(position.0, position.1, position.2)),
                position
            );
        }
    }

    #[test]
    fn negative_grid_boundaries_and_equal_distance_order_match_vanilla() {
        assert_eq!([-17, -16, -1, 0, 15, 16].map(grid_x), [-2, -1, -1, 0, 0, 1]);
        assert_eq!([-13, -12, -1, 0, 11, 12].map(grid_y), [-2, -1, -1, 0, 0, 1]);
        assert_eq!([-17, -16, -1, 0, 15, 16].map(grid_z), [-2, -1, -1, 0, 0, 1]);
        let mut closest = [(i32::MAX, 0); 4];
        insert_closest(&mut closest, 7, 1);
        insert_closest(&mut closest, 7, 2);
        insert_closest(&mut closest, 9, 3);
        assert_eq!(closest[..3], [(7, 2), (7, 1), (9, 3)]);
    }
}
