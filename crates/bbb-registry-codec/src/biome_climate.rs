//! Vanilla biome temperature and shared biome-info simplex sampling.

use std::sync::OnceLock;

use crate::legacy_random::LegacyRandomSource;

static BIOME_INFO_NOISE: OnceLock<SimplexNoise> = OnceLock::new();
static TEMPERATURE_NOISE: OnceLock<SimplexNoise> = OnceLock::new();
static FROZEN_TEMPERATURE_NOISE: OnceLock<PerlinSimplexNoise> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureModifier {
    None,
    Frozen,
}

/// `Biome.BIOME_INFO_NOISE.getValue(x * 0.0225, z * 0.0225, false)`
pub fn biome_info_noise(x: f64, z: f64) -> f64 {
    biome_info_noise_raw(x * 0.0225, z * 0.0225)
}

/// Direct `BIOME_INFO_NOISE` sampling used by placement and the frozen modifier.
pub fn biome_info_noise_raw(x: f64, z: f64) -> f64 {
    BIOME_INFO_NOISE
        .get_or_init(|| SimplexNoise::new(LegacyRandomSource::with_seed(2345)))
        .get_value(x, z)
}

/// `Biome.getHeightAdjustedTemperature` and frozen
/// modifier.
pub fn height_adjusted_temperature(
    base_temperature: f32,
    modifier: TemperatureModifier,
    x: i32,
    y: i32,
    z: i32,
    sea_level: i32,
) -> f32 {
    let adjusted = match modifier {
        TemperatureModifier::None => base_temperature,
        TemperatureModifier::Frozen => {
            let large = FROZEN_TEMPERATURE_NOISE
                .get_or_init(|| {
                    PerlinSimplexNoise::new(LegacyRandomSource::with_seed(3456), &[-2, -1, 0])
                })
                .get_value(f64::from(x) * 0.05, f64::from(z) * 0.05)
                * 7.0;
            let edge = biome_info_noise_raw(f64::from(x) * 0.2, f64::from(z) * 0.2);
            if large + edge < 0.3
                && biome_info_noise_raw(f64::from(x) * 0.09, f64::from(z) * 0.09) < 0.8
            {
                0.2
            } else {
                base_temperature
            }
        }
    };
    let snow_level = sea_level + 17;
    if y > snow_level {
        let noise = (TEMPERATURE_NOISE
            .get_or_init(|| SimplexNoise::new(LegacyRandomSource::with_seed(1234)))
            .get_value(f64::from(x) / 8.0, f64::from(z) / 8.0)
            * 8.0) as f32;
        adjusted - (noise + (y - snow_level) as f32) * 0.05 / 40.0
    } else {
        adjusted
    }
}

#[derive(Debug, Clone)]
struct PerlinSimplexNoise {
    levels: Vec<Option<SimplexNoise>>,
    input_factor: f64,
    value_factor: f64,
}

impl PerlinSimplexNoise {
    fn new(mut random: LegacyRandomSource, octave_set: &[i32]) -> Self {
        let first = *octave_set.iter().min().expect("octave set is non-empty");
        let last = *octave_set.iter().max().expect("octave set is non-empty");
        assert_eq!(last, 0, "shared biome noise has zero as its highest octave");
        let count = usize::try_from(last - first + 1).expect("octave span fits usize");
        let zero = SimplexNoise::from_random(&mut random);
        let mut levels = vec![None; count];
        if octave_set.contains(&0) {
            levels[0] = Some(zero);
        }
        for (index, slot) in levels.iter_mut().enumerate().skip(1) {
            let octave = -(index as i32);
            if octave_set.contains(&octave) {
                *slot = Some(SimplexNoise::from_random(&mut random));
            } else {
                for _ in 0..262 {
                    random.next_bits(32);
                }
            }
        }
        Self {
            levels,
            input_factor: 2.0_f64.powi(last),
            value_factor: 1.0 / (2.0_f64.powi(count as i32) - 1.0),
        }
    }

    fn get_value(&self, x: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut input_factor = self.input_factor;
        let mut value_factor = self.value_factor;
        for level in &self.levels {
            if let Some(level) = level {
                value += level.get_value(x * input_factor, z * input_factor) * value_factor;
            }
            input_factor /= 2.0;
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
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let mut p = [0; 256];
        for (index, value) in p.iter_mut().enumerate() {
            *value = index as i32;
        }
        for index in 0..256 {
            let offset = random.next_int((256 - index) as i32) as usize;
            p.swap(index, index + offset);
        }
        Self { p }
    }

    fn p(&self, value: i32) -> i32 {
        self.p[(value & 255) as usize]
    }

    fn get_value(&self, x: f64, z: f64) -> f64 {
        let skew = (x + z) * Self::F2;
        let i = (x + skew).floor() as i32;
        let j = (z + skew).floor() as i32;
        let unskew = f64::from(i + j) * Self::G2;
        let x0 = x - (f64::from(i) - unskew);
        let z0 = z - (f64::from(j) - unskew);
        let (i1, j1) = if x0 > z0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - f64::from(i1) + Self::G2;
        let z1 = z0 - f64::from(j1) + Self::G2;
        let x2 = x0 - 1.0 + 2.0 * Self::G2;
        let z2 = z0 - 1.0 + 2.0 * Self::G2;
        let ii = i & 255;
        let jj = j & 255;
        let g0 = (self.p(ii + self.p(jj)) % 12) as usize;
        let g1 = (self.p(ii + i1 + self.p(jj + j1)) % 12) as usize;
        let g2 = (self.p(ii + 1 + self.p(jj + 1)) % 12) as usize;
        70.0 * (corner(g0, x0, z0) + corner(g1, x1, z1) + corner(g2, x2, z2))
    }
}

fn corner(index: usize, x: f64, z: f64) -> f64 {
    let mut attenuation = 0.5 - x * x - z * z;
    if attenuation < 0.0 {
        return 0.0;
    }
    attenuation *= attenuation;
    attenuation
        * attenuation
        * (f64::from(SimplexNoise::GRADIENT[index][0]) * x
            + f64::from(SimplexNoise::GRADIENT[index][1]) * z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_biome_noises_match_vanilla_samples() {
        assert_eq!(biome_info_noise(0.0, 0.0), 0.0);
        assert!((biome_info_noise(-496.0, -512.0) + 0.102_904_227_905_454_12).abs() < 1.0e-12);
        let temperature =
            height_adjusted_temperature(0.149, TemperatureModifier::None, -512, 81, -511, 63);
        assert!(temperature >= 0.15);
        assert_eq!(
            height_adjusted_temperature(0.0, TemperatureModifier::Frozen, 0, 64, 0, 63),
            0.2
        );
    }
}
