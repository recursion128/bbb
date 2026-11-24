//! Vanilla `ImprovedNoise` / `PerlinNoise` / `NormalNoise` sampling, seeded
//! from [`WorldgenRandom`] streams keyed by the noise name.

pub use crate::random::{position_seed, WorldgenRandom};

use super::mth_floor_f64;

pub mod aquifer;
pub mod beardifier;
pub mod biome;
pub mod buried_treasure;
pub mod carver;
pub mod density;
pub mod end_city;
pub mod features;
pub mod generated_structure_piece;
pub mod igloo;
pub mod mineshaft;
pub mod nether_fortress;
pub mod nether_fossil;
pub mod ocean_monument;
pub mod ocean_ruin;
pub mod ore_vein;
pub mod proto_chunk_entity;
pub use proto_chunk_entity::{
    EndCityItemFrameRecord, ProtoChunkEntityRecord, StructureMobKind, StructureMobRecord,
    TemplateMobRecord,
};
pub mod ruined_portal;
pub mod scattered;
pub mod shipwreck;
pub mod stronghold;
pub mod structure_nbt;
pub mod structure_piece_placement;
pub mod structure_pieces;
pub mod structure_starts;
pub mod structures;
pub mod surface;
pub mod template_entities;
pub mod woodland_mansion;

/// The private random of a `WorldGenRegion`, keyed by its center chunk.
pub fn worldgen_region_random(world_seed: i64, chunk_x: i32, chunk_z: i32) -> WorldgenRandom {
    WorldgenRandom::from_parts(&[
        world_seed,
        crate::random::name_seed("worldgen_region_random"),
        i64::from(chunk_x),
        i64::from(chunk_z),
    ])
}

/// Vanilla `ImprovedNoise` (`ImprovedNoise.java:7-30`).
#[derive(Clone)]
pub struct ImprovedNoise {
    p: [u8; 256],
    xo: f64,
    yo: f64,
    zo: f64,
}

impl ImprovedNoise {
    fn new(random: &mut WorldgenRandom) -> Self {
        let xo = random.next_double() * 256.0;
        let yo = random.next_double() * 256.0;
        let zo = random.next_double() * 256.0;
        let mut p = [0_u8; 256];
        for (i, slot) in p.iter_mut().enumerate() {
            *slot = i as u8;
        }
        for i in 0..256 {
            let offset = random.next_int(256 - i as i32) as usize;
            p.swap(i, i + offset);
        }
        Self { p, xo, yo, zo }
    }

    /// `ImprovedNoise.noise(x, y, z)` (`ImprovedNoise.java:32-34`) through the
    /// `yScale == 0` branch of `noise(x, y, z, yScale, yFudge)` (`:37-62`).
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        self.noise_with_y_scale(x, y, z, 0.0, 0.0)
    }

    /// The legacy five-argument sampler used by `BlendedNoise`
    /// (`ImprovedNoise.java:37-62`).
    pub fn noise_with_y_scale(&self, x: f64, y: f64, z: f64, y_scale: f64, y_fudge: f64) -> f64 {
        let x = x + self.xo;
        let y = y + self.yo;
        let z = z + self.zo;
        let xf = mth_floor_f64(x);
        let yf = mth_floor_f64(y);
        let zf = mth_floor_f64(z);
        let xr = x - f64::from(xf);
        let yr = y - f64::from(yf);
        let zr = z - f64::from(zf);
        let yr_fudge = if y_scale != 0.0 {
            let limit = if y_fudge >= 0.0 && y_fudge < yr {
                y_fudge
            } else {
                yr
            };
            (limit / y_scale + f64::from(1.0e-7_f32)).floor() * y_scale
        } else {
            0.0
        };
        self.sample_and_lerp(xf, yf, zf, xr, yr - yr_fudge, zr, yr)
    }

    fn p(&self, x: i32) -> i32 {
        i32::from(self.p[(x & 0xFF) as usize])
    }

    /// `ImprovedNoise.sampleAndLerp` (`ImprovedNoise.java:85-104`).
    #[allow(clippy::too_many_arguments)]
    fn sample_and_lerp(
        &self,
        x: i32,
        y: i32,
        z: i32,
        xr: f64,
        yr: f64,
        zr: f64,
        yr_original: f64,
    ) -> f64 {
        let x0 = self.p(x);
        let x1 = self.p(x + 1);
        let xy00 = self.p(x0 + y);
        let xy01 = self.p(x0 + y + 1);
        let xy10 = self.p(x1 + y);
        let xy11 = self.p(x1 + y + 1);
        let d000 = grad_dot(self.p(xy00 + z), xr, yr, zr);
        let d100 = grad_dot(self.p(xy10 + z), xr - 1.0, yr, zr);
        let d010 = grad_dot(self.p(xy01 + z), xr, yr - 1.0, zr);
        let d110 = grad_dot(self.p(xy11 + z), xr - 1.0, yr - 1.0, zr);
        let d001 = grad_dot(self.p(xy00 + z + 1), xr, yr, zr - 1.0);
        let d101 = grad_dot(self.p(xy10 + z + 1), xr - 1.0, yr, zr - 1.0);
        let d011 = grad_dot(self.p(xy01 + z + 1), xr, yr - 1.0, zr - 1.0);
        let d111 = grad_dot(self.p(xy11 + z + 1), xr - 1.0, yr - 1.0, zr - 1.0);
        let x_alpha = smoothstep(xr);
        let y_alpha = smoothstep(yr_original);
        let z_alpha = smoothstep(zr);
        lerp3(
            x_alpha, y_alpha, z_alpha, d000, d100, d010, d110, d001, d101, d011, d111,
        )
    }
}

/// `ImprovedNoise.gradDot` (`ImprovedNoise.java:77-79`) with `SimplexNoise.dot` (`:55-57`),
/// matching `SimplexNoise.GRADIENT` (`SimplexNoise.java:7-24`) without table loads or int casts.
#[inline(always)]
fn grad_dot(hash: i32, x: f64, y: f64, z: f64) -> f64 {
    match hash & 15 {
        0 | 12 => x + y,
        1 | 14 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 | 13 => -y + z,
        10 => y - z,
        11 | 15 => -y - z,
        _ => unreachable!(),
    }
}

/// `Mth.smoothstep` (`Mth.java:574-576`).
fn smoothstep(x: f64) -> f64 {
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

/// `Mth.lerp` (`Mth.java:540-542`).
fn lerp(alpha: f64, p0: f64, p1: f64) -> f64 {
    p0 + alpha * (p1 - p0)
}

/// `Mth.lerp2` (`Mth.java:544-546`).
fn lerp2(alpha1: f64, alpha2: f64, x00: f64, x10: f64, x01: f64, x11: f64) -> f64 {
    lerp(alpha2, lerp(alpha1, x00, x10), lerp(alpha1, x01, x11))
}

/// `Mth.lerp3` (`Mth.java:548-561`).
#[allow(clippy::too_many_arguments)]
fn lerp3(
    alpha1: f64,
    alpha2: f64,
    alpha3: f64,
    x000: f64,
    x100: f64,
    x010: f64,
    x110: f64,
    x001: f64,
    x101: f64,
    x011: f64,
    x111: f64,
) -> f64 {
    lerp(
        alpha3,
        lerp2(alpha1, alpha2, x000, x100, x010, x110),
        lerp2(alpha1, alpha2, x001, x101, x011, x111),
    )
}

#[derive(Clone)]
struct ActiveOctave {
    noise: ImprovedNoise,
    input_factor: f64,
    amplitude: f64,
    value_factor: f64,
}

/// Vanilla `PerlinNoise` built by `PerlinNoise.create(random, firstOctave, amplitudes)`
/// (`PerlinNoise.java:54-56`, constructor `:81-130` on the new-initialization branch).
#[derive(Clone)]
pub struct PerlinNoise {
    active_octaves: Vec<ActiveOctave>,
}

impl PerlinNoise {
    /// Each octave draws from its own stream keyed by `seed` and its octave.
    fn create(seed: i64, first_octave: i32, amplitudes: &[f64]) -> Option<Self> {
        if amplitudes.is_empty() {
            return None;
        }
        let octaves = amplitudes.len() as i32;
        let zero_octave_index = -first_octave;
        let mut factor = 2.0_f64.powi(-zero_octave_index);
        let mut value_factor = 2.0_f64.powi(octaves - 1) / (2.0_f64.powi(octaves) - 1.0);
        let mut active_octaves = Vec::with_capacity(amplitudes.len());
        for (i, &amplitude) in amplitudes.iter().enumerate() {
            if amplitude != 0.0 {
                let octave = first_octave + i as i32;
                let mut octave_random = WorldgenRandom::from_parts(&[seed, i64::from(octave)]);
                active_octaves.push(ActiveOctave {
                    noise: ImprovedNoise::new(&mut octave_random),
                    input_factor: factor,
                    amplitude,
                    value_factor,
                });
            }
            factor *= 2.0;
            value_factor /= 2.0;
        }
        Some(Self { active_octaves })
    }

    /// `PerlinNoise.createLegacyForBlendedNoise`: every octave present, drawn
    /// from one stream.
    pub fn create_for_blended_noise(
        random: &mut WorldgenRandom,
        first_octave: i32,
        octave_count: usize,
    ) -> Self {
        let zero_octave_index = usize::try_from(-first_octave).expect("legacy octaves end at zero");
        debug_assert_eq!(zero_octave_index + 1, octave_count);
        let mut noise_levels = vec![None; octave_count];
        noise_levels[zero_octave_index] = Some(ImprovedNoise::new(random));
        for index in (0..zero_octave_index).rev() {
            noise_levels[index] = Some(ImprovedNoise::new(random));
        }
        let mut factor = 2.0_f64.powi(first_octave);
        let mut value_factor =
            2.0_f64.powi(octave_count as i32 - 1) / (2.0_f64.powi(octave_count as i32) - 1.0);
        let mut active_octaves = Vec::with_capacity(octave_count);
        for noise in noise_levels {
            active_octaves.push(ActiveOctave {
                noise: noise.expect("legacy blended noise fills every octave"),
                input_factor: factor,
                amplitude: 1.0,
                value_factor,
            });
            factor *= 2.0;
            value_factor /= 2.0;
        }
        Self { active_octaves }
    }

    fn octave_noise(&self, index: usize) -> Option<&ImprovedNoise> {
        self.active_octaves
            .get(self.active_octaves.len().checked_sub(index + 1)?)
            .map(|octave| &octave.noise)
    }

    /// `PerlinNoise.getValue(x, y, z)` (`PerlinNoise.java:140-162`, `yScale`/`yFudge` 0).
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut value = 0.0;
        for octave in &self.active_octaves {
            let noise_val = octave.noise.noise(
                wrap(x * octave.input_factor),
                wrap(y * octave.input_factor),
                wrap(z * octave.input_factor),
            );
            value += octave.amplitude * noise_val * octave.value_factor;
        }
        value
    }
}

/// `PerlinNoise.wrap` (`PerlinNoise.java:188-190`).
fn wrap(x: f64) -> f64 {
    x - ((x / 3.3554432E7 + 0.5).floor() as i64) as f64 * 3.3554432E7
}

/// Vanilla `NormalNoise.create(random, firstOctave, amplitudes...)`
/// (`NormalNoise.java:30-36`, constructor `:38-65` on the new branch).
#[derive(Clone)]
pub struct NormalNoise {
    value_factor: f64,
    first: PerlinNoise,
    second: PerlinNoise,
}

/// `BlendedNoise`, the `old_blended_noise` density function used by the
/// built-in Nether and End settings (`BlendedNoise.java:15-145`).
#[derive(Clone)]
pub struct BlendedNoise {
    min_limit_noise: PerlinNoise,
    max_limit_noise: PerlinNoise,
    main_noise: PerlinNoise,
    xz_multiplier: f64,
    y_multiplier: f64,
    xz_factor: f64,
    y_factor: f64,
    smear_scale_multiplier: f64,
}

impl BlendedNoise {
    pub fn new(
        seed: i64,
        xz_scale: f64,
        y_scale: f64,
        xz_factor: f64,
        y_factor: f64,
        smear_scale_multiplier: f64,
    ) -> Self {
        let mut random = WorldgenRandom::named(seed, "minecraft:terrain");
        Self {
            min_limit_noise: PerlinNoise::create_for_blended_noise(&mut random, -15, 16),
            max_limit_noise: PerlinNoise::create_for_blended_noise(&mut random, -15, 16),
            main_noise: PerlinNoise::create_for_blended_noise(&mut random, -7, 8),
            xz_multiplier: 684.412 * xz_scale,
            y_multiplier: 684.412 * y_scale,
            xz_factor,
            y_factor,
            smear_scale_multiplier,
        }
    }

    pub fn compute(&self, block_x: i32, block_y: i32, block_z: i32) -> f64 {
        let limit_x = f64::from(block_x) * self.xz_multiplier;
        let limit_y = f64::from(block_y) * self.y_multiplier;
        let limit_z = f64::from(block_z) * self.xz_multiplier;
        let main_x = limit_x / self.xz_factor;
        let main_y = limit_y / self.y_factor;
        let main_z = limit_z / self.xz_factor;
        let limit_smear = self.y_multiplier * self.smear_scale_multiplier;
        let main_smear = limit_smear / self.y_factor;
        let mut main_noise_value = 0.0;
        let mut pow = 1.0;
        for index in 0..8 {
            if let Some(noise) = self.main_noise.octave_noise(index) {
                main_noise_value += noise.noise_with_y_scale(
                    wrap(main_x * pow),
                    wrap(main_y * pow),
                    wrap(main_z * pow),
                    main_smear * pow,
                    main_y * pow,
                ) / pow;
            }
            pow /= 2.0;
        }

        let factor = (main_noise_value / 10.0 + 1.0) / 2.0;
        let is_max = factor >= 1.0;
        let is_min = factor <= 0.0;
        let mut blend_min = 0.0;
        let mut blend_max = 0.0;
        pow = 1.0;
        for index in 0..16 {
            let wx = wrap(limit_x * pow);
            let wy = wrap(limit_y * pow);
            let wz = wrap(limit_z * pow);
            let y_scale_pow = limit_smear * pow;
            if !is_max {
                if let Some(noise) = self.min_limit_noise.octave_noise(index) {
                    blend_min +=
                        noise.noise_with_y_scale(wx, wy, wz, y_scale_pow, limit_y * pow) / pow;
                }
            }
            if !is_min {
                if let Some(noise) = self.max_limit_noise.octave_noise(index) {
                    blend_max +=
                        noise.noise_with_y_scale(wx, wy, wz, y_scale_pow, limit_y * pow) / pow;
                }
            }
            pow /= 2.0;
        }
        clamped_lerp(factor, blend_min / 512.0, blend_max / 512.0) / 128.0
    }
}

fn clamped_lerp(factor: f64, min: f64, max: f64) -> f64 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        lerp(factor, min, max)
    }
}

impl NormalNoise {
    const INPUT_FACTOR: f64 = 1.0181268882175227;

    /// `NormalNoise.create`: two Perlin noises from streams forked off `random`.
    pub fn create(
        random: &mut WorldgenRandom,
        first_octave: i32,
        amplitudes: &[f64],
    ) -> Option<Self> {
        let first = PerlinNoise::create(random.next_long(), first_octave, amplitudes)?;
        let second = PerlinNoise::create(random.next_long(), first_octave, amplitudes)?;
        let mut min_octave = i32::MAX;
        let mut max_octave = i32::MIN;
        for (i, amplitude) in amplitudes.iter().enumerate() {
            if *amplitude != 0.0 {
                min_octave = min_octave.min(i as i32);
                max_octave = max_octave.max(i as i32);
            }
        }
        let value_factor = if min_octave > max_octave {
            0.0
        } else {
            0.16666666666666666 / expected_deviation(max_octave - min_octave)
        };
        Some(Self {
            value_factor,
            first,
            second,
        })
    }

    /// `NormalNoise.getValue` (`NormalNoise.java:75-80`).
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64 {
        let x2 = x * Self::INPUT_FACTOR;
        let y2 = y * Self::INPUT_FACTOR;
        let z2 = z * Self::INPUT_FACTOR;
        (self.first.get_value(x, y, z) + self.second.get_value(x2, y2, z2)) * self.value_factor
    }
}

/// The named noise of a world (`RandomState.getOrCreateNoise`).
pub fn create_normal_noise(
    seed: i64,
    name: &str,
    first_octave: i32,
    amplitudes: &[f64],
) -> Option<NormalNoise> {
    NormalNoise::create(
        &mut WorldgenRandom::named(seed, name),
        first_octave,
        amplitudes,
    )
}

/// `NormalNoise.expectedDeviation` (`NormalNoise.java:71-73`).
fn expected_deviation(octave_span: i32) -> f64 {
    0.1 * (1.0 + 1.0 / f64::from(octave_span + 1))
}

/// `SimplexNoise` 2-arg `getValue` (`SimplexNoise.java:36-104`).
pub(crate) struct SimplexNoise {
    permutation: [i32; 256],
}

impl SimplexNoise {
    const GRADIENT: [[i32; 3]; 16] = [
        [1, 1, 0],
        [-1, 1, 0],
        [1, -1, 0],
        [-1, -1, 0],
        [1, 0, 1],
        [-1, 0, 1],
        [1, 0, -1],
        [-1, 0, -1],
        [0, 1, 1],
        [0, -1, 1],
        [0, 1, -1],
        [0, -1, -1],
        [1, 1, 0],
        [0, -1, 1],
        [-1, 1, 0],
        [0, -1, -1],
    ];
    const F2: f64 = 0.366_025_403_784_438_6;
    const G2: f64 = 0.211_324_865_405_187_13;

    pub(crate) fn new(random: &mut WorldgenRandom) -> Self {
        let _xo = random.next_double() * 256.0;
        let _yo = random.next_double() * 256.0;
        let _zo = random.next_double() * 256.0;
        let mut permutation = [0; 256];
        for (index, value) in permutation.iter_mut().enumerate() {
            *value = index as i32;
        }
        for index in 0..256 {
            let offset = random.next_int((256 - index) as i32) as usize;
            permutation.swap(index, index + offset);
        }
        Self { permutation }
    }

    fn permutation(&self, value: i32) -> i32 {
        self.permutation[(value & 0xff) as usize]
    }

    pub(crate) fn get_value(&self, xin: f64, yin: f64) -> f64 {
        let skew = (xin + yin) * Self::F2;
        let i = (xin + skew).floor() as i32;
        let j = (yin + skew).floor() as i32;
        let unskew = f64::from(i + j) * Self::G2;
        let x0 = xin - (f64::from(i) - unskew);
        let y0 = yin - (f64::from(j) - unskew);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - f64::from(i1) + Self::G2;
        let y1 = y0 - f64::from(j1) + Self::G2;
        let x2 = x0 - 1.0 + 2.0 * Self::G2;
        let y2 = y0 - 1.0 + 2.0 * Self::G2;
        let ii = i & 0xff;
        let jj = j & 0xff;
        let gi0 = (self.permutation(ii + self.permutation(jj)) % 12) as usize;
        let gi1 = (self.permutation(ii + i1 + self.permutation(jj + j1)) % 12) as usize;
        let gi2 = (self.permutation(ii + 1 + self.permutation(jj + 1)) % 12) as usize;
        70.0 * (Self::corner_noise(gi0, x0, y0)
            + Self::corner_noise(gi1, x1, y1)
            + Self::corner_noise(gi2, x2, y2))
    }

    fn corner_noise(index: usize, x: f64, y: f64) -> f64 {
        let mut attenuation = 0.5 - x * x - y * y;
        if attenuation < 0.0 {
            return 0.0;
        }
        attenuation *= attenuation;
        attenuation
            * attenuation
            * (f64::from(Self::GRADIENT[index][0]) * x + f64::from(Self::GRADIENT[index][1]) * y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_noise_with_all_zero_amplitudes_does_not_overflow() {
        let noise = NormalNoise::create(&mut WorldgenRandom::with_seed(123), -4, &[0.0, 0.0])
            .expect("all-zero non-empty amplitudes should construct a zero noise");
        assert_eq!(noise.get_value(12.5, -3.0, 7.25), 0.0);
    }
}
