//! Per-chunk density evaluation with Vanilla's marker caches and cell
//! interpolation (`NoiseChunk.java`).
//!
//! `NoiseChunk.wrap` replaces every marker by a stateful cache; here the
//! function graph stays immutable and each marker owns a cache slot in the
//! sampler instead. The caches are pure memoisation of the same value the
//! uncached walk (`DensityFunction::sample_mode`) computes, except where
//! Vanilla's semantics differ observably: `flat_cache` samples the quart grid
//! at `y = 0` inside the chunk and falls back to the exact position outside
//! (`NoiseChunk.FlatCache`), and `interpolated` markers sampled outside a cell
//! fill lerp in Y, X, Z order (`NoiseChunk.NoiseInterpolator.updateForY/X/Z`).

use std::cell::RefCell;
use std::sync::Arc;

use super::{
    clamped_lerp, java_clamp, java_max, java_min, lerp, quantize_climate, DensityFunction, NodeId,
    NoiseDensitySource, Router, CLIMATE,
};
use crate::noise::beardifier::JigsawBeardifier;
use bbb_server_datapack::worldgen::density::{BinaryKind, MarkerKind};

const MAX_POOLED_SCRATCH: usize = 4;

thread_local! {
    static SAMPLER_SCRATCH_POOL: RefCell<Vec<SamplerScratch>> = const { RefCell::new(Vec::new()) };
}

/// The target-bound old-generation Blender owned by one `NoiseChunk`.
pub trait NoiseBlender: Send + Sync {
    fn blend_offset_and_factor(&self, block_x: i32, block_z: i32) -> (f64, f64);
    fn blend_density(&self, block_x: i32, block_y: i32, block_z: i32, value: f64) -> f64;
}

/// `DensityFunction.FunctionContext`: `Chunk` is the `NoiseChunk` itself and
/// `Point` is a `SinglePointContext`, which bypasses `cache_once`, cell caches
/// and interpolators (`NoiseChunk.CacheOnce.compute`,
/// `NoiseChunk.NoiseInterpolator.compute`).
#[derive(Clone, Copy)]
pub enum Context {
    Chunk,
    Point { x: i32, y: i32, z: i32 },
}

#[derive(Clone, Copy)]
pub enum Slot {
    Cache2d { last: Option<((i32, i32), f64)> },
    CacheAllInCell { index: usize },
    CacheOnce { last: Option<(u64, f64)> },
    FlatCache { index: usize, filled: bool },
    Interpolated { index: usize },
}

#[derive(Default)]
struct SamplerScratch {
    slots: Vec<Slot>,
    cell_caches: Vec<Vec<f64>>,
    flat_caches: Vec<Vec<f64>>,
    interpolators: Vec<Interpolator>,
    cell_values: Vec<f64>,
}

fn cell_cache_index(
    in_cell_x: i32,
    in_cell_y: i32,
    in_cell_z: i32,
    cell_width: i32,
    cell_height: i32,
) -> Option<usize> {
    if in_cell_x < 0
        || in_cell_y < 0
        || in_cell_z < 0
        || in_cell_x >= cell_width
        || in_cell_y >= cell_height
        || in_cell_z >= cell_width
    {
        return None;
    }
    Some(
        (((cell_height - 1 - in_cell_y) * cell_width + in_cell_x) * cell_width + in_cell_z)
            as usize,
    )
}

/// `NoiseChunk.NoiseInterpolator` corner slices and lerp state.
pub struct Interpolator {
    slice0: Vec<f64>,
    slice1: Vec<f64>,
    noise000: f64,
    noise001: f64,
    noise100: f64,
    noise101: f64,
    noise010: f64,
    noise011: f64,
    noise110: f64,
    noise111: f64,
    value_xz00: f64,
    value_xz10: f64,
    value_xz01: f64,
    value_xz11: f64,
    value_z0: f64,
    value_z1: f64,
    value: f64,
    fill_xy0: f64,
    fill_xy1: f64,
}

impl Interpolator {
    pub fn new(slice_len: usize) -> Self {
        Self {
            slice0: vec![0.0; slice_len],
            slice1: vec![0.0; slice_len],
            noise000: 0.0,
            noise001: 0.0,
            noise100: 0.0,
            noise101: 0.0,
            noise010: 0.0,
            noise011: 0.0,
            noise110: 0.0,
            noise111: 0.0,
            value_xz00: 0.0,
            value_xz10: 0.0,
            value_xz01: 0.0,
            value_xz11: 0.0,
            value_z0: 0.0,
            value_z1: 0.0,
            value: 0.0,
            fill_xy0: 0.0,
            fill_xy1: 0.0,
        }
    }

    pub fn reset(&mut self, slice_len: usize) {
        self.slice0.clear();
        self.slice0.resize(slice_len, 0.0);
        self.slice1.clear();
        self.slice1.resize(slice_len, 0.0);
        self.noise000 = 0.0;
        self.noise001 = 0.0;
        self.noise100 = 0.0;
        self.noise101 = 0.0;
        self.noise010 = 0.0;
        self.noise011 = 0.0;
        self.noise110 = 0.0;
        self.noise111 = 0.0;
        self.value_xz00 = 0.0;
        self.value_xz10 = 0.0;
        self.value_xz01 = 0.0;
        self.value_xz11 = 0.0;
        self.value_z0 = 0.0;
        self.value_z1 = 0.0;
        self.value = 0.0;
        self.fill_xy0 = 0.0;
        self.fill_xy1 = 0.0;
    }

    pub fn select_cell_yz(&mut self, cell_y_index: usize, cell_z_index: usize, size_y: usize) {
        let index = |z: usize, y: usize| z * size_y + y;
        self.noise000 = self.slice0[index(cell_z_index, cell_y_index)];
        self.noise001 = self.slice0[index(cell_z_index + 1, cell_y_index)];
        self.noise100 = self.slice1[index(cell_z_index, cell_y_index)];
        self.noise101 = self.slice1[index(cell_z_index + 1, cell_y_index)];
        self.noise010 = self.slice0[index(cell_z_index, cell_y_index + 1)];
        self.noise011 = self.slice0[index(cell_z_index + 1, cell_y_index + 1)];
        self.noise110 = self.slice1[index(cell_z_index, cell_y_index + 1)];
        self.noise111 = self.slice1[index(cell_z_index + 1, cell_y_index + 1)];
    }

    pub fn update_for_y(&mut self, factor_y: f64) {
        self.value_xz00 = lerp(factor_y, self.noise000, self.noise010);
        self.value_xz10 = lerp(factor_y, self.noise100, self.noise110);
        self.value_xz01 = lerp(factor_y, self.noise001, self.noise011);
        self.value_xz11 = lerp(factor_y, self.noise101, self.noise111);
    }

    pub fn update_for_x(&mut self, factor_x: f64) {
        self.value_z0 = lerp(factor_x, self.value_xz00, self.value_xz10);
        self.value_z1 = lerp(factor_x, self.value_xz01, self.value_xz11);
    }

    pub fn update_for_z(&mut self, factor_z: f64) {
        self.value = lerp(factor_z, self.value_z0, self.value_z1);
    }

    /// `Mth.lerp3`: X, then Y, then Z.
    pub fn lerp3(&self, factor_x: f64, factor_y: f64, factor_z: f64) -> f64 {
        lerp(
            factor_z,
            lerp(
                factor_y,
                lerp(factor_x, self.noise000, self.noise100),
                lerp(factor_x, self.noise010, self.noise110),
            ),
            lerp(
                factor_y,
                lerp(factor_x, self.noise001, self.noise101),
                lerp(factor_x, self.noise011, self.noise111),
            ),
        )
    }

    /// Precomputes the X-then-Y halves of `lerp3` once per `(x_in_cell, y_in_cell)`.
    #[inline]
    fn update_fill_xy(&mut self, factor_x: f64, factor_y: f64) {
        self.fill_xy0 = lerp(
            factor_y,
            lerp(factor_x, self.noise000, self.noise100),
            lerp(factor_x, self.noise010, self.noise110),
        );
        self.fill_xy1 = lerp(
            factor_y,
            lerp(factor_x, self.noise001, self.noise101),
            lerp(factor_x, self.noise011, self.noise111),
        );
    }

    /// Completes `lerp3` along Z using `update_fill_xy` (bit-for-bit identical).
    #[inline]
    fn lerp_fill_z(&self, factor_z: f64) -> f64 {
        lerp(factor_z, self.fill_xy0, self.fill_xy1)
    }
}

/// One `NoiseChunk`: the cache slots plus the interpolation cursor that
/// `NoiseBasedChunkGenerator.doFill` drives.
pub struct NoiseChunkSampler<'a> {
    source: &'a NoiseDensitySource,
    beardifier: JigsawBeardifier,
    blender: Option<Arc<dyn NoiseBlender>>,
    cell_width: i32,
    cell_height: i32,
    cell_count_xz: i32,
    cell_count_y: i32,
    cell_noise_min_y: i32,
    first_cell_x: i32,
    first_cell_z: i32,
    first_noise_x: i32,
    first_noise_z: i32,
    noise_size_xz: i32,
    scratch: SamplerScratch,
    interpolating: bool,
    filling_cell: bool,
    filling_slice: bool,
    cell_start_x: i32,
    cell_start_y: i32,
    cell_start_z: i32,
    in_cell_x: i32,
    in_cell_y: i32,
    in_cell_z: i32,
    fill_factor_z: f64,
    interpolation_counter: u64,
}

impl Drop for NoiseChunkSampler<'_> {
    fn drop(&mut self) {
        let scratch = std::mem::take(&mut self.scratch);
        SAMPLER_SCRATCH_POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            if pool.len() < MAX_POOLED_SCRATCH {
                pool.push(scratch);
            }
        });
    }
}

impl<'a> NoiseChunkSampler<'a> {
    /// `NoiseChunk.forChunk` with the noise shape already clamped to the
    /// dimension (`noise_min_y` and `noise_height` are cell multiples).
    pub fn for_chunk(
        source: &'a NoiseDensitySource,
        chunk_start_x: i32,
        chunk_start_z: i32,
        noise_min_y: i32,
        noise_height: i32,
    ) -> Self {
        let cell_count_xz = 16 / source.cell_width();
        Self::new(
            source,
            cell_count_xz,
            chunk_start_x,
            chunk_start_z,
            noise_min_y,
            noise_height,
        )
    }

    /// A target-bound `NoiseChunk`; explicit Beardifier markers and the
    /// synthetic final-density addition share this owned value.
    pub fn for_chunk_with_beardifier(
        source: &'a NoiseDensitySource,
        chunk_start_x: i32,
        chunk_start_z: i32,
        noise_min_y: i32,
        noise_height: i32,
        beardifier: JigsawBeardifier,
    ) -> Self {
        let cell_count_xz = 16 / source.cell_width();
        Self::new_with_beardifier(
            source,
            cell_count_xz,
            chunk_start_x,
            chunk_start_z,
            noise_min_y,
            noise_height,
            beardifier,
        )
    }

    /// The production `NoiseChunk` binding for both structure and
    /// old-generation density inputs.
    pub fn for_chunk_with_beardifier_and_blender(
        source: &'a NoiseDensitySource,
        chunk_start_x: i32,
        chunk_start_z: i32,
        noise_min_y: i32,
        noise_height: i32,
        beardifier: JigsawBeardifier,
        blender: Arc<dyn NoiseBlender>,
    ) -> Self {
        let cell_count_xz = 16 / source.cell_width();
        Self::new_with_beardifier_and_blender(
            source,
            cell_count_xz,
            chunk_start_x,
            chunk_start_z,
            noise_min_y,
            noise_height,
            beardifier,
            Some(blender),
        )
    }

    /// A sampler for `SinglePointContext` evaluation only: a one-quart flat
    /// cache around `(x, z)` and no interpolation state.
    pub fn for_point(source: &'a NoiseDensitySource, x: i32, z: i32) -> Self {
        Self::new(source, 0, x, z, source.min_y(), 0)
    }

    /// The one-cell `NoiseChunk` of `NoiseBasedChunkGenerator.iterateNoiseColumn`.
    pub fn for_column(
        source: &'a NoiseDensitySource,
        block_x: i32,
        block_z: i32,
        noise_min_y: i32,
        noise_height: i32,
    ) -> Self {
        let cell_width = source.cell_width();
        Self::new(
            source,
            1,
            block_x.div_euclid(cell_width) * cell_width,
            block_z.div_euclid(cell_width) * cell_width,
            noise_min_y,
            noise_height,
        )
    }

    /// `iterateNoiseColumn` from the top down; stops at the first `Some`.
    pub fn iterate_column<T>(
        &mut self,
        block_x: i32,
        block_z: i32,
        beardifier: &JigsawBeardifier,
        mut visit: impl FnMut(&mut Self, i32, f64) -> Option<T>,
    ) -> Option<T> {
        let (cell_width, cell_height) = (self.cell_width, self.cell_height);
        let factor_x = f64::from(block_x.rem_euclid(cell_width)) / f64::from(cell_width);
        let factor_z = f64::from(block_z.rem_euclid(cell_width)) / f64::from(cell_width);
        self.initialize_for_first_cell_x();
        self.advance_cell_x(0);
        let mut result = None;
        'cells: for cell_y_index in (0..self.cell_count_y).rev() {
            self.select_cell_yz(cell_y_index, 0, beardifier);
            for y_in_cell in (0..cell_height).rev() {
                let pos_y = (self.cell_noise_min_y + cell_y_index) * cell_height + y_in_cell;
                self.update_for_y(pos_y, f64::from(y_in_cell) / f64::from(cell_height));
                self.update_for_x(block_x, factor_x);
                self.update_for_z(block_z, factor_z);
                let density = self.interpolated_density();
                if let Some(value) = visit(self, pos_y, density) {
                    result = Some(value);
                    break 'cells;
                }
            }
        }
        self.stop_interpolation();
        result
    }

    pub fn new(
        source: &'a NoiseDensitySource,
        cell_count_xz: i32,
        min_block_x: i32,
        min_block_z: i32,
        noise_min_y: i32,
        noise_height: i32,
    ) -> Self {
        Self::new_with_beardifier(
            source,
            cell_count_xz,
            min_block_x,
            min_block_z,
            noise_min_y,
            noise_height,
            JigsawBeardifier::default(),
        )
    }

    fn new_with_beardifier(
        source: &'a NoiseDensitySource,
        cell_count_xz: i32,
        min_block_x: i32,
        min_block_z: i32,
        noise_min_y: i32,
        noise_height: i32,
        beardifier: JigsawBeardifier,
    ) -> Self {
        Self::new_with_beardifier_and_blender(
            source,
            cell_count_xz,
            min_block_x,
            min_block_z,
            noise_min_y,
            noise_height,
            beardifier,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_beardifier_and_blender(
        source: &'a NoiseDensitySource,
        cell_count_xz: i32,
        min_block_x: i32,
        min_block_z: i32,
        noise_min_y: i32,
        noise_height: i32,
        beardifier: JigsawBeardifier,
        blender: Option<Arc<dyn NoiseBlender>>,
    ) -> Self {
        let cell_width = source.cell_width();
        let cell_height = source.cell_height();
        let mut scratch = SAMPLER_SCRATCH_POOL
            .with(|pool| pool.borrow_mut().pop())
            .unwrap_or_default();
        scratch.slots.clear();
        scratch.slots.reserve(source.marker_kinds.len());
        let mut cell_cache_count = 0;
        let mut flat_cache_count = 0;
        let mut interpolator_count = 0;
        for kind in &source.marker_kinds {
            let slot = match kind {
                MarkerKind::Cache2d => Slot::Cache2d { last: None },
                MarkerKind::CacheAllInCell => {
                    let index = cell_cache_count;
                    cell_cache_count += 1;
                    Slot::CacheAllInCell { index }
                }
                MarkerKind::CacheOnce => Slot::CacheOnce { last: None },
                MarkerKind::FlatCache => {
                    let index = flat_cache_count;
                    flat_cache_count += 1;
                    Slot::FlatCache {
                        index,
                        filled: false,
                    }
                }
                MarkerKind::Interpolated => {
                    let index = interpolator_count;
                    interpolator_count += 1;
                    Slot::Interpolated { index }
                }
            };
            scratch.slots.push(slot);
        }
        if scratch.cell_caches.len() < cell_cache_count {
            scratch.cell_caches.resize_with(cell_cache_count, Vec::new);
        }
        for cache in &mut scratch.cell_caches[..cell_cache_count] {
            cache.clear();
        }
        if scratch.flat_caches.len() < flat_cache_count {
            scratch.flat_caches.resize_with(flat_cache_count, Vec::new);
        }
        if scratch.interpolators.len() < interpolator_count {
            scratch
                .interpolators
                .resize_with(interpolator_count, || Interpolator::new(0));
        }
        scratch.cell_values.clear();
        Self {
            source,
            beardifier,
            blender,
            cell_width,
            cell_height,
            cell_count_xz,
            cell_count_y: noise_height.div_euclid(cell_height),
            cell_noise_min_y: noise_min_y.div_euclid(cell_height),
            first_cell_x: min_block_x.div_euclid(cell_width),
            first_cell_z: min_block_z.div_euclid(cell_width),
            first_noise_x: min_block_x >> 2,
            first_noise_z: min_block_z >> 2,
            noise_size_xz: (cell_count_xz * cell_width) >> 2,
            scratch,
            interpolating: false,
            filling_cell: false,
            filling_slice: false,
            cell_start_x: 0,
            cell_start_y: 0,
            cell_start_z: 0,
            in_cell_x: 0,
            in_cell_y: 0,
            in_cell_z: 0,
            fill_factor_z: 0.0,
            interpolation_counter: 0,
        }
    }

    pub fn slice_size_y(&self) -> usize {
        (self.cell_count_y + 1) as usize
    }

    pub fn slice_len(&self) -> usize {
        (self.cell_count_xz + 1) as usize * self.slice_size_y()
    }

    /// `NoiseChunk.initializeForFirstCellX`.
    pub fn initialize_for_first_cell_x(&mut self) {
        assert!(!self.interpolating, "starting interpolation twice");
        self.interpolating = true;
        self.interpolation_counter = 0;
        let slice_len = self.slice_len();
        let cell_len = (self.cell_width * self.cell_width * self.cell_height) as usize;
        for &(index, _) in &self.source.interpolators {
            self.scratch.interpolators[index].reset(slice_len);
        }
        for (slot, _) in &self.source.cell_caches {
            let Slot::CacheAllInCell { index } = self.scratch.slots[*slot] else {
                unreachable!("cell-cache slots are allocated by kind");
            };
            let values = &mut self.scratch.cell_caches[index];
            values.clear();
            values.resize(cell_len, 0.0);
        }
        self.scratch.cell_values.clear();
        self.scratch.cell_values.resize(cell_len, 0.0);
        self.fill_slice(true, self.first_cell_x);
    }

    /// `NoiseChunk.advanceCellX`.
    pub fn advance_cell_x(&mut self, cell_x_index: i32) {
        self.fill_slice(false, self.first_cell_x + cell_x_index + 1);
        self.cell_start_x = (self.first_cell_x + cell_x_index) * self.cell_width;
    }

    /// `NoiseChunk.swapSlices`.
    pub fn swap_slices(&mut self) {
        for &(index, _) in &self.source.interpolators {
            let interpolator = &mut self.scratch.interpolators[index];
            std::mem::swap(&mut interpolator.slice0, &mut interpolator.slice1);
        }
    }

    /// `NoiseChunk.stopInterpolation`.
    pub fn stop_interpolation(&mut self) {
        assert!(self.interpolating, "stopping interpolation twice");
        self.interpolating = false;
    }

    /// `NoiseChunk.fillSlice`: every interpolator samples its corners at the
    /// cell column `(cell_x, cell_z)` for each cell Y through
    /// `sliceFillingContextProvider`.
    pub fn fill_slice(&mut self, slice0: bool, cell_x: i32) {
        self.cell_start_x = cell_x * self.cell_width;
        self.in_cell_x = 0;
        self.filling_slice = true;
        let size_y = self.slice_size_y();
        let source = self.source;
        for cell_z_index in 0..=self.cell_count_xz {
            let cell_z = self.first_cell_z + cell_z_index;
            self.cell_start_z = cell_z * self.cell_width;
            self.in_cell_z = 0;
            let z_offset = cell_z_index as usize * size_y;
            for cell_y_index in 0..=self.cell_count_y {
                self.cell_start_y = (cell_y_index + self.cell_noise_min_y) * self.cell_height;
                self.in_cell_y = 0;
                self.interpolation_counter += 1;
                for &(index, input) in &source.interpolators {
                    let value = self.compute(input, Context::Chunk);
                    let interpolator = &mut self.scratch.interpolators[index];
                    let slice = if slice0 {
                        &mut interpolator.slice0
                    } else {
                        &mut interpolator.slice1
                    };
                    slice[z_offset + cell_y_index as usize] = value;
                }
            }
        }
        self.filling_slice = false;
    }

    /// `NoiseChunk.selectCellYZ` followed by the `CacheAllInCell` fill of
    /// `final_density + beardifier` (`NoiseChunk.fillAllDirectly`).
    pub fn select_cell_yz(
        &mut self,
        cell_y_index: i32,
        cell_z_index: i32,
        beardifier: &JigsawBeardifier,
    ) {
        let size_y = self.slice_size_y();
        let source = self.source;
        let cell_len = (self.cell_width * self.cell_width * self.cell_height) as usize;
        for &(index, _) in &source.interpolators {
            self.scratch.interpolators[index].select_cell_yz(
                cell_y_index as usize,
                cell_z_index as usize,
                size_y,
            );
        }
        let mut factor_xz_table = [0.0_f64; 16];
        let mut factor_y_table = [0.0_f64; 16];
        let inv_w = f64::from(self.cell_width);
        let inv_h = f64::from(self.cell_height);
        for (i, slot) in factor_xz_table
            .iter_mut()
            .enumerate()
            .take(self.cell_width as usize)
        {
            *slot = f64::from(i as i32) / inv_w;
        }
        for (i, slot) in factor_y_table
            .iter_mut()
            .enumerate()
            .take(self.cell_height as usize)
        {
            *slot = f64::from(i as i32) / inv_h;
        }
        let factor_xz = |i: i32| {
            factor_xz_table
                .get(i as usize)
                .copied()
                .unwrap_or_else(|| f64::from(i) / inv_w)
        };
        let factor_y = |i: i32| {
            factor_y_table
                .get(i as usize)
                .copied()
                .unwrap_or_else(|| f64::from(i) / inv_h)
        };
        self.filling_cell = true;
        self.cell_start_y = (cell_y_index + self.cell_noise_min_y) * self.cell_height;
        self.cell_start_z = (self.first_cell_z + cell_z_index) * self.cell_width;
        // `NoiseChunk.selectCellYZ` fills explicit cell caches before the
        // implicit final-density cache (`NoiseChunk.java:285-315,529-571`).
        for (slot, input) in &source.cell_caches {
            let Slot::CacheAllInCell { index: cache_idx } = self.scratch.slots[*slot] else {
                unreachable!("cell-cache slots are allocated by kind");
            };
            if self.scratch.cell_caches[cache_idx].len() != cell_len {
                self.scratch.cell_caches[cache_idx].resize(cell_len, 0.0);
            }
            let mut pos = 0;
            for y_in_cell in (0..self.cell_height).rev() {
                self.in_cell_y = y_in_cell;
                let fy = factor_y(y_in_cell);
                for x_in_cell in 0..self.cell_width {
                    self.in_cell_x = x_in_cell;
                    let fx = factor_xz(x_in_cell);
                    for &index in &source.cell_fill_interpolators {
                        self.scratch.interpolators[index].update_fill_xy(fx, fy);
                    }
                    for z_in_cell in 0..self.cell_width {
                        self.in_cell_z = z_in_cell;
                        self.fill_factor_z = factor_xz(z_in_cell);
                        self.interpolation_counter += 1;
                        let value = self.compute(*input, Context::Chunk);
                        self.scratch.cell_caches[cache_idx][pos] = value;
                        pos += 1;
                    }
                }
            }
        }
        if self.scratch.cell_values.len() != cell_len {
            self.scratch.cell_values.resize(cell_len, 0.0);
        }
        let mut pos = 0;
        for y_in_cell in (0..self.cell_height).rev() {
            self.in_cell_y = y_in_cell;
            let fy = factor_y(y_in_cell);
            for x_in_cell in 0..self.cell_width {
                self.in_cell_x = x_in_cell;
                let fx = factor_xz(x_in_cell);
                for &index in &source.cell_fill_interpolators {
                    self.scratch.interpolators[index].update_fill_xy(fx, fy);
                }
                for z_in_cell in 0..self.cell_width {
                    self.in_cell_z = z_in_cell;
                    self.fill_factor_z = factor_xz(z_in_cell);
                    self.interpolation_counter += 1;
                    let density = self.compute(source.router(Router::FinalDensity), Context::Chunk);
                    let (x, y, z) = self.position(Context::Chunk);
                    self.scratch.cell_values[pos] = density + beardifier.sample(x, y, z);
                    pos += 1;
                }
            }
        }
        self.filling_cell = false;
    }

    /// `NoiseChunk.updateForY`.
    pub fn update_for_y(&mut self, pos_y: i32, factor_y: f64) {
        self.in_cell_y = pos_y - self.cell_start_y;
        for &index in &self.source.block_interpolators {
            self.scratch.interpolators[index].update_for_y(factor_y);
        }
    }

    /// `NoiseChunk.updateForX`.
    pub fn update_for_x(&mut self, pos_x: i32, factor_x: f64) {
        self.in_cell_x = pos_x - self.cell_start_x;
        for &index in &self.source.block_interpolators {
            self.scratch.interpolators[index].update_for_x(factor_x);
        }
    }

    /// `NoiseChunk.updateForZ`.
    pub fn update_for_z(&mut self, pos_z: i32, factor_z: f64) {
        self.in_cell_z = pos_z - self.cell_start_z;
        self.interpolation_counter += 1;
        for &index in &self.source.block_interpolators {
            self.scratch.interpolators[index].update_for_z(factor_z);
        }
    }

    /// `NoiseChunk.getInterpolatedDensity` through `CacheAllInCell.compute`.
    pub fn interpolated_density(&self) -> f64 {
        let index = cell_cache_index(
            self.in_cell_x,
            self.in_cell_y,
            self.in_cell_z,
            self.cell_width,
            self.cell_height,
        )
        .expect("the interpolation cursor stays inside its selected cell");
        self.scratch.cell_values[index]
    }

    /// `Climate.Sampler.sample` through `NoiseChunk.cachedClimateSampler`.
    pub fn climate_target(&mut self, quart_x: i32, quart_y: i32, quart_z: i32) -> Option<[i64; 7]> {
        if !self.source.climate_enabled {
            return None;
        }
        let context = Context::Point {
            x: quart_x.wrapping_mul(4),
            y: quart_y.wrapping_mul(4),
            z: quart_z.wrapping_mul(4),
        };
        let mut target = [0_i64; 7];
        for (value, field) in target.iter_mut().zip(CLIMATE) {
            *value = quantize_climate(self.compute(self.source.router(field), context));
        }
        Some(target)
    }

    /// `NoiseChunk.preliminarySurfaceLevel` without its column memo.
    pub fn preliminary_surface_level(&mut self, x: i32, z: i32) -> i32 {
        let source = self.source;
        let context = Context::Point {
            x: (x >> 2) << 2,
            y: 0,
            z: (z >> 2) << 2,
        };
        self.compute(source.router(Router::PreliminarySurfaceLevel), context)
            .floor() as i32
    }

    pub(crate) const fn source(&self) -> &NoiseDensitySource {
        self.source
    }

    /// An aquifer router function at a `SinglePointContext`.
    pub(crate) fn aquifer(&mut self, field: Router, x: i32, y: i32, z: i32) -> f64 {
        self.compute(self.source.router(field), Context::Point { x, y, z })
    }

    pub fn position(&self, context: Context) -> (i32, i32, i32) {
        match context {
            Context::Chunk => (
                self.cell_start_x + self.in_cell_x,
                self.cell_start_y + self.in_cell_y,
                self.cell_start_z + self.in_cell_z,
            ),
            Context::Point { x, y, z } => (x, y, z),
        }
    }

    fn blending_position(&self, context: Context) -> (i32, i32) {
        // Bound blend alpha/offset use their quart-grid FlatCache and only
        // fall back to the exact point outside it (`NoiseChunk.java:631-665`).
        let (x, _, z) = self.position(context);
        let quart_x = x >> 2;
        let quart_z = z >> 2;
        let local_x = quart_x.wrapping_sub(self.first_noise_x);
        let local_z = quart_z.wrapping_sub(self.first_noise_z);
        if local_x >= 0
            && local_z >= 0
            && local_x <= self.noise_size_xz
            && local_z <= self.noise_size_xz
        {
            (quart_x.wrapping_mul(4), quart_z.wrapping_mul(4))
        } else {
            (x, z)
        }
    }

    /// `DensityFunction.compute` with the `NoiseChunk` wrappers applied.
    pub fn compute(&mut self, id: NodeId, context: Context) -> f64 {
        let source = self.source;
        match source.node(id) {
            DensityFunction::Constant(value) => *value,
            DensityFunction::BeardifierMarker => {
                let (x, y, z) = self.position(context);
                self.beardifier.sample(x, y, z)
            }
            DensityFunction::YClampedGradient {
                from_y,
                to_y,
                from_value,
                to_value,
            } => {
                let (_, y, _) = self.position(context);
                let factor =
                    (f64::from(y) - f64::from(*from_y)) / (f64::from(*to_y) - f64::from(*from_y));
                clamped_lerp(factor, *from_value, *to_value)
            }
            DensityFunction::Binary {
                kind,
                first,
                second,
            } => {
                let first = self.compute(*first, context);
                match kind {
                    BinaryKind::Add => first + self.compute(*second, context),
                    BinaryKind::Mul if first == 0.0 => 0.0,
                    BinaryKind::Mul => first * self.compute(*second, context),
                    BinaryKind::Min => java_min(first, self.compute(*second, context)),
                    BinaryKind::Max => java_max(first, self.compute(*second, context)),
                }
            }
            DensityFunction::Mapped { kind, input } => {
                super::overworld::apply_mapped(*kind, self.compute(*input, context))
            }
            DensityFunction::Marker { kind, input, slot } => {
                self.marker(*kind, *input, *slot, context)
            }
            DensityFunction::Clamp { input, min, max } => {
                java_clamp(self.compute(*input, context), *min, *max)
            }
            DensityFunction::RangeChoice {
                input,
                min_inclusive,
                max_exclusive,
                when_in_range,
                when_out_of_range,
            } => {
                let value = self.compute(*input, context);
                if value >= *min_inclusive && value < *max_exclusive {
                    self.compute(*when_in_range, context)
                } else {
                    self.compute(*when_out_of_range, context)
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
                let (x, _, z) = self.position(context);
                let raw_upper = self.compute(*upper_bound, context);
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
                    // `FindTopSurface.compute` probes with a fresh `SinglePointContext`.
                    if self.compute(*density, Context::Point { x, y: block_y, z }) > 0.0 {
                        return f64::from(block_y);
                    }
                    let Some(next_y) = block_y.checked_sub(step) else {
                        break;
                    };
                    block_y = next_y;
                }
                f64::from(*lower_bound)
            }
            DensityFunction::BlendDensity(input) => {
                let value = self.compute(*input, context);
                if matches!(context, Context::Chunk) {
                    if let Some(blender) = &self.blender {
                        let (x, y, z) = self.position(context);
                        return blender.blend_density(x, y, z, value);
                    }
                }
                value
            }
            DensityFunction::BlendAlpha => self.blender.as_ref().map_or(1.0, |blender| {
                let (x, z) = self.blending_position(context);
                blender.blend_offset_and_factor(x, z).0
            }),
            DensityFunction::BlendOffset => self.blender.as_ref().map_or(0.0, |blender| {
                let (x, z) = self.blending_position(context);
                blender.blend_offset_and_factor(x, z).1
            }),
            DensityFunction::Noise(noise) => {
                let (x, y, z) = self.position(context);
                noise.sample(x, y, z)
            }
            DensityFunction::Shift(noise) => {
                let (x, y, z) = self.position(context);
                noise.sample(x, y, z)
            }
            DensityFunction::ShiftedNoise(noise) => {
                let (x, y, z) = self.position(context);
                noise.sample(x, y, z, |id| self.compute(id, context))
            }
            DensityFunction::Spline(spline) => {
                f64::from(spline.evaluate(&mut |coordinate| self.compute(coordinate, context)))
            }
            DensityFunction::WeirdScaled(sampler) => {
                let (x, y, z) = self.position(context);
                let input = self.compute(sampler.input, context);
                sampler.sample(x, y, z, input)
            }
            DensityFunction::OldBlendedNoise(noise) => {
                let (x, y, z) = self.position(context);
                noise.compute(x, y, z)
            }
            DensityFunction::EndIslands(islands) => {
                let (x, _, z) = self.position(context);
                islands.sample(x, z)
            }
        }
    }

    pub fn marker(
        &mut self,
        kind: MarkerKind,
        input: NodeId,
        slot: usize,
        context: Context,
    ) -> f64 {
        match kind {
            // `NoiseChunk.Cache2D.compute`.
            MarkerKind::Cache2d => {
                let (x, _, z) = self.position(context);
                if let Slot::Cache2d {
                    last: Some((position, value)),
                } = self.scratch.slots[slot]
                {
                    if position == (x, z) {
                        return value;
                    }
                }
                let value = self.compute(input, context);
                self.scratch.slots[slot] = Slot::Cache2d {
                    last: Some(((x, z), value)),
                };
                value
            }
            MarkerKind::CacheAllInCell => {
                if matches!(context, Context::Point { .. }) || self.filling_slice {
                    return self.compute(input, context);
                }
                debug_assert!(
                    self.interpolating,
                    "cell cache sampled outside the interpolation loop"
                );
                let Some(index) = cell_cache_index(
                    self.in_cell_x,
                    self.in_cell_y,
                    self.in_cell_z,
                    self.cell_width,
                    self.cell_height,
                ) else {
                    return self.compute(input, context);
                };
                let cached = match self.scratch.slots[slot] {
                    Slot::CacheAllInCell { index: cache_idx } => {
                        self.scratch.cell_caches[cache_idx].get(index).copied()
                    }
                    _ => unreachable!("cell-cache slots are allocated by kind"),
                };
                cached.unwrap_or_else(|| self.compute(input, context))
            }
            // `NoiseChunk.CacheOnce.compute`: one value per interpolation
            // counter step, i.e. per block in the fill loop, per point during
            // cell fill, and per cell Y while filling a slice.
            MarkerKind::CacheOnce => {
                if matches!(context, Context::Point { .. }) {
                    return self.compute(input, context);
                }
                let counter = self.interpolation_counter;
                if let Slot::CacheOnce {
                    last: Some((last_counter, value)),
                } = self.scratch.slots[slot]
                {
                    if last_counter == counter {
                        return value;
                    }
                }
                let value = self.compute(input, context);
                self.scratch.slots[slot] = Slot::CacheOnce {
                    last: Some((counter, value)),
                };
                value
            }
            // `NoiseChunk.FlatCache.compute`.
            MarkerKind::FlatCache => {
                let (x, _, z) = self.position(context);
                let size = self.noise_size_xz + 1;
                let index_x = (x >> 2).wrapping_sub(self.first_noise_x);
                let index_z = (z >> 2).wrapping_sub(self.first_noise_z);
                if index_x >= 0 && index_z >= 0 && index_x < size && index_z < size {
                    let index = (index_x + index_z * size) as usize;
                    let Slot::FlatCache {
                        index: cache_idx,
                        filled,
                    } = self.scratch.slots[slot]
                    else {
                        unreachable!("flat-cache slots are allocated by kind");
                    };
                    if !filled {
                        let len = (size * size) as usize;
                        let cache = &mut self.scratch.flat_caches[cache_idx];
                        cache.clear();
                        cache.resize(len, 0.0);
                        for fx in 0..size {
                            let block_x = (self.first_noise_x + fx) << 2;
                            for fz in 0..size {
                                let block_z = (self.first_noise_z + fz) << 2;
                                let sampled = self.compute(
                                    input,
                                    Context::Point {
                                        x: block_x,
                                        y: 0,
                                        z: block_z,
                                    },
                                );
                                self.scratch.flat_caches[cache_idx][(fx + fz * size) as usize] =
                                    sampled;
                            }
                        }
                        self.scratch.slots[slot] = Slot::FlatCache {
                            index: cache_idx,
                            filled: true,
                        };
                    }
                    self.scratch.flat_caches[cache_idx][index]
                } else {
                    self.compute(input, context)
                }
            }
            MarkerKind::Interpolated => {
                if matches!(context, Context::Point { .. }) {
                    return self.compute(input, context);
                }
                if self.filling_slice {
                    // Slice corners evaluate the wrapped function directly
                    // (`NoiseInterpolator.fillArray` outside a cell fill).
                    return self.compute(input, context);
                }
                debug_assert!(
                    self.interpolating,
                    "interpolator sampled outside the interpolation loop"
                );
                let Slot::Interpolated { index } = self.scratch.slots[slot] else {
                    unreachable!("interpolator slots are allocated by kind");
                };
                let interpolator = &self.scratch.interpolators[index];
                if self.filling_cell {
                    interpolator.lerp_fill_z(self.fill_factor_z)
                } else {
                    interpolator.value
                }
            }
        }
    }

    /// `NoiseChunk.FlatCache` construction: one `SinglePointContext(x, 0, z)`
    /// sample per quart position of the chunk plus its positive edge.
    pub fn fill_flat_cache(&mut self, input: NodeId) -> Vec<f64> {
        let size = self.noise_size_xz + 1;
        let mut values = vec![0.0; (size * size) as usize];
        for x in 0..size {
            let block_x = (self.first_noise_x + x) << 2;
            for z in 0..size {
                let block_z = (self.first_noise_z + z) << 2;
                values[(x + z * size) as usize] = self.compute(
                    input,
                    Context::Point {
                        x: block_x,
                        y: 0,
                        z: block_z,
                    },
                );
            }
        }
        values
    }
}

impl super::ClimateSampler for NoiseChunkSampler<'_> {
    fn climate_target(&mut self, quart_x: i32, quart_y: i32, quart_z: i32) -> Option<[i64; 7]> {
        NoiseChunkSampler::climate_target(self, quart_x, quart_y, quart_z)
    }

    fn sample_erosion(&mut self, x: i32, y: i32, z: i32) -> Option<f64> {
        self.source.climate_enabled.then(|| {
            self.compute(
                self.source.router(Router::Erosion),
                Context::Point { x, y, z },
            )
        })
    }
}

impl super::VeinRouter for NoiseChunkSampler<'_> {
    fn vein(&mut self, field: Router, x: i32, y: i32, z: i32) -> f64 {
        debug_assert_eq!(self.position(Context::Chunk), (x, y, z));
        self.compute(self.source.router(field), Context::Chunk)
    }
}

#[cfg(test)]
pub mod tests;
