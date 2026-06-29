use std::sync::OnceLock;

use bbb_pack::{BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier};
use bbb_renderer::terrain::TerrainTint;
use bbb_world::BlockPos;

use crate::terrain_runtime::BlockRenderPosition;

const SWAMP_GRASS_DARK: [u8; 3] = [0x4c, 0x76, 0x3c];
const SWAMP_GRASS_LIGHT: [u8; 3] = [0x6a, 0x70, 0x39];

static BIOME_INFO_NOISE: OnceLock<SimplexNoise> = OnceLock::new();
static TEMPERATURE_NOISE: OnceLock<SimplexNoise> = OnceLock::new();
static FROZEN_TEMPERATURE_NOISE: OnceLock<PerlinSimplexNoise> = OnceLock::new();

pub(crate) fn terrain_tint_from_rgb(rgb: [u8; 3]) -> TerrainTint {
    TerrainTint::from_rgb_u8(rgb[0], rgb[1], rgb[2])
}

pub(crate) fn biome_colormap_climate(profile: Option<&BiomeColorProfile>) -> (f32, f32) {
    profile
        .map(|profile| {
            (
                profile.temperature.clamp(0.0, 1.0),
                profile.downfall.clamp(0.0, 1.0),
            )
        })
        .unwrap_or((0.5, 1.0))
}

pub(crate) fn apply_grass_color_modifier(
    modifier: GrassColorModifier,
    base: [u8; 3],
    position: Option<BlockRenderPosition>,
) -> [u8; 3] {
    match modifier {
        GrassColorModifier::None => base,
        GrassColorModifier::DarkForest => {
            let base = rgb_to_u24(base);
            u24_to_rgb(((base & 0xfe_fe_fe) + 0x28_34_0a) >> 1)
        }
        GrassColorModifier::Swamp => {
            let position = position.unwrap_or(BlockRenderPosition { x: 0, y: 0, z: 0 });
            if biome_info_noise(position.x as f64, position.z as f64) < -0.1 {
                SWAMP_GRASS_DARK
            } else {
                SWAMP_GRASS_LIGHT
            }
        }
    }
}

fn biome_info_noise(x: f64, z: f64) -> f64 {
    biome_info_noise_raw(x * 0.0225, z * 0.0225)
}

pub(crate) fn biome_height_adjusted_temperature(
    base_temperature: f32,
    temperature_modifier: BiomeTemperatureModifier,
    pos: BlockPos,
    sea_level: i32,
) -> f32 {
    let adjusted_temperature =
        biome_modified_temperature(base_temperature, temperature_modifier, pos);
    let snow_level = sea_level + 17;
    if pos.y > snow_level {
        let noise = (biome_temperature_noise(pos.x, pos.z) * 8.0) as f32;
        adjusted_temperature - (noise + (pos.y - snow_level) as f32) * 0.05 / 40.0
    } else {
        adjusted_temperature
    }
}

fn biome_modified_temperature(
    base_temperature: f32,
    temperature_modifier: BiomeTemperatureModifier,
    pos: BlockPos,
) -> f32 {
    match temperature_modifier {
        BiomeTemperatureModifier::None => base_temperature,
        BiomeTemperatureModifier::Frozen => {
            let ground_value_large_variation = frozen_temperature_noise(pos.x, pos.z) * 7.0;
            let ground_value_edge_variation =
                biome_info_noise_raw(pos.x as f64 * 0.2, pos.z as f64 * 0.2);
            let ice_patches = ground_value_large_variation + ground_value_edge_variation;
            if ice_patches < 0.3 {
                let ground_value_small_variation =
                    biome_info_noise_raw(pos.x as f64 * 0.09, pos.z as f64 * 0.09);
                if ground_value_small_variation < 0.8 {
                    return 0.2;
                }
            }
            base_temperature
        }
    }
}

fn biome_temperature_noise(x: i32, z: i32) -> f64 {
    TEMPERATURE_NOISE
        .get_or_init(|| SimplexNoise::new(LegacyRandomSource::new(1234)))
        .get_value(x as f64 / 8.0, z as f64 / 8.0)
}

fn frozen_temperature_noise(x: i32, z: i32) -> f64 {
    FROZEN_TEMPERATURE_NOISE
        .get_or_init(|| PerlinSimplexNoise::new(LegacyRandomSource::new(3456), &[-2, -1, 0]))
        .get_value(x as f64 * 0.05, z as f64 * 0.05)
}

fn biome_info_noise_raw(x: f64, z: f64) -> f64 {
    BIOME_INFO_NOISE
        .get_or_init(|| SimplexNoise::new(LegacyRandomSource::new(2345)))
        .get_value(x, z)
}

#[derive(Debug, Clone)]
struct LegacyRandomSource {
    seed: u64,
}

impl LegacyRandomSource {
    const MASK: u64 = (1u64 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;

    fn new(seed: i64) -> Self {
        let mut random = Self { seed: 0 };
        random.set_seed(seed);
        random
    }

    fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK;
    }

    fn next(&mut self, bits: u8) -> i32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return (((bound as i64) * i64::from(self.next(31))) >> 31) as i32;
        }

        loop {
            let sample = self.next(31);
            let modulo = sample % bound;
            if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
                return modulo;
            }
        }
    }

    fn next_double(&mut self) -> f64 {
        let upper = i64::from(self.next(26));
        let lower = i64::from(self.next(27));
        (((upper << 27) + lower) as f64) * 1.110223e-16
    }

    fn consume_count(&mut self, rounds: usize) {
        for _ in 0..rounds {
            self.next(32);
        }
    }
}

#[derive(Debug, Clone)]
struct PerlinSimplexNoise {
    noise_levels: Vec<Option<SimplexNoise>>,
    highest_freq_input_factor: f64,
    highest_freq_value_factor: f64,
}

impl PerlinSimplexNoise {
    fn new(mut random: LegacyRandomSource, octave_set: &[i32]) -> Self {
        assert!(!octave_set.is_empty(), "octave set must not be empty");
        let first_octave = *octave_set.iter().min().unwrap();
        let last_octave = *octave_set.iter().max().unwrap();
        assert_eq!(
            last_octave, 0,
            "biome temperature PerlinSimplexNoise expects zero as the highest octave"
        );
        let low_freq_octaves = -first_octave;
        let high_freq_octaves = last_octave;
        let octaves = usize::try_from(low_freq_octaves + high_freq_octaves + 1).unwrap();
        assert!(octaves >= 1, "octave count must be positive");

        let zero_octave = SimplexNoise::from_random(&mut random);
        let zero_octave_index = usize::try_from(high_freq_octaves).unwrap();
        let mut noise_levels = vec![None; octaves];
        if octave_set.contains(&0) {
            noise_levels[zero_octave_index] = Some(zero_octave);
        }

        for (i, slot) in noise_levels
            .iter_mut()
            .enumerate()
            .skip(zero_octave_index + 1)
        {
            let octave = high_freq_octaves - i as i32;
            if octave_set.contains(&octave) {
                *slot = Some(SimplexNoise::from_random(&mut random));
            } else {
                random.consume_count(262);
            }
        }

        Self {
            noise_levels,
            highest_freq_input_factor: 2.0_f64.powi(high_freq_octaves),
            highest_freq_value_factor: 1.0 / (2.0_f64.powi(octaves as i32) - 1.0),
        }
    }

    fn get_value(&self, x: f64, y: f64) -> f64 {
        let mut value = 0.0;
        let mut factor = self.highest_freq_input_factor;
        let mut value_factor = self.highest_freq_value_factor;
        for noise_level in &self.noise_levels {
            if let Some(noise_level) = noise_level {
                value += noise_level.get_value(x * factor, y * factor) * value_factor;
            }
            factor /= 2.0;
            value_factor *= 2.0;
        }
        value
    }
}

#[derive(Debug, Clone)]
struct SimplexNoise {
    p: [i32; 256],
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

    fn new(mut random: LegacyRandomSource) -> Self {
        Self::from_random(&mut random)
    }

    fn from_random(random: &mut LegacyRandomSource) -> Self {
        let _xo = random.next_double() * 256.0;
        let _yo = random.next_double() * 256.0;
        let _zo = random.next_double() * 256.0;
        let mut p = [0; 256];
        for (index, value) in p.iter_mut().enumerate() {
            *value = index as i32;
        }
        for ix in 0..256usize {
            let offset = random.next_int((256 - ix) as i32) as usize;
            p.swap(ix, ix + offset);
        }
        Self { p }
    }

    fn p(&self, x: i32) -> i32 {
        self.p[(x & 0xff) as usize]
    }

    fn get_value(&self, xin: f64, yin: f64) -> f64 {
        let s = (xin + yin) * Self::F2;
        let i = floor_i32(xin + s);
        let j = floor_i32(yin + s);
        let t = f64::from(i + j) * Self::G2;
        let x0 = xin - (f64::from(i) - t);
        let y0 = yin - (f64::from(j) - t);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - f64::from(i1) + Self::G2;
        let y1 = y0 - f64::from(j1) + Self::G2;
        let x2 = x0 - 1.0 + 2.0 * Self::G2;
        let y2 = y0 - 1.0 + 2.0 * Self::G2;
        let ii = i & 0xff;
        let jj = j & 0xff;
        let gi0 = self.p(ii + self.p(jj)) % 12;
        let gi1 = self.p(ii + i1 + self.p(jj + j1)) % 12;
        let gi2 = self.p(ii + 1 + self.p(jj + 1)) % 12;
        let n0 = simplex_corner_noise(gi0 as usize, x0, y0, 0.0, 0.5);
        let n1 = simplex_corner_noise(gi1 as usize, x1, y1, 0.0, 0.5);
        let n2 = simplex_corner_noise(gi2 as usize, x2, y2, 0.0, 0.5);
        70.0 * (n0 + n1 + n2)
    }
}

fn floor_i32(value: f64) -> i32 {
    value.floor() as i32
}

fn simplex_corner_noise(index: usize, x: f64, y: f64, z: f64, base: f64) -> f64 {
    let mut t0 = base - x * x - y * y - z * z;
    if t0 < 0.0 {
        0.0
    } else {
        t0 *= t0;
        t0 * t0 * simplex_dot(SimplexNoise::GRADIENT[index], x, y, z)
    }
}

fn simplex_dot(g: [i32; 3], x: f64, y: f64, z: f64) -> f64 {
    f64::from(g[0]) * x + f64::from(g[1]) * y + f64::from(g[2]) * z
}

fn rgb_to_u24(rgb: [u8; 3]) -> u32 {
    u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2])
}

fn u24_to_rgb(value: u32) -> [u8; 3] {
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

pub(crate) fn is_grass_tinted_block(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:grass_block"
            | "minecraft:short_grass"
            | "minecraft:tall_grass"
            | "minecraft:fern"
            | "minecraft:large_fern"
            | "minecraft:potted_fern"
            | "minecraft:bush"
            | "minecraft:sugar_cane"
    ) || block_name.contains("vine")
}

pub(crate) fn is_foliage_tinted_block(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:oak_leaves"
            | "minecraft:jungle_leaves"
            | "minecraft:acacia_leaves"
            | "minecraft:dark_oak_leaves"
            | "minecraft:mangrove_leaves"
            | "minecraft:vine"
    )
}

pub(crate) fn is_dry_foliage_tinted_block(block_name: &str) -> bool {
    matches!(block_name, "minecraft:leaf_litter")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biome_info_noise_matches_vanilla_seed_samples() {
        assert_eq!(biome_info_noise(0.0, 0.0), 0.0);
        let dark_sample = biome_info_noise(-496.0, -512.0);
        assert!((dark_sample - -0.102_904_227_905_454_12).abs() < 1.0e-12);
    }

    #[test]
    fn temperature_noise_matches_vanilla_seed_samples() {
        assert_eq!(biome_temperature_noise(0, 0), 0.0);
        let sample = biome_temperature_noise(16, 16);
        assert!((sample - -0.498_924_790_550_414_5).abs() < 1.0e-12);
    }

    #[test]
    fn height_adjusted_temperature_uses_noise_and_frozen_modifier() {
        let rain_position = BlockPos {
            x: -512,
            y: 81,
            z: -511,
        };
        assert!(
            biome_height_adjusted_temperature(
                0.149,
                BiomeTemperatureModifier::None,
                rain_position,
                63,
            ) >= 0.15
        );
        assert_eq!(
            biome_height_adjusted_temperature(
                0.0,
                BiomeTemperatureModifier::Frozen,
                BlockPos { x: 0, y: 64, z: 0 },
                63,
            ),
            0.2
        );
    }

    #[test]
    fn swamp_grass_modifier_uses_biome_info_noise() {
        assert_eq!(
            apply_grass_color_modifier(
                GrassColorModifier::Swamp,
                [1, 2, 3],
                Some(BlockRenderPosition { x: 0, y: 0, z: 0 })
            ),
            SWAMP_GRASS_LIGHT
        );
        assert_eq!(
            apply_grass_color_modifier(
                GrassColorModifier::Swamp,
                [1, 2, 3],
                Some(BlockRenderPosition {
                    x: -496,
                    y: 0,
                    z: -512,
                })
            ),
            SWAMP_GRASS_DARK
        );
    }
}
