//! Vanilla biome climate sampling shared by world simulation and rendering.

use bbb_registry_codec::biome_climate::{
    biome_info_noise as shared_biome_info_noise, height_adjusted_temperature, TemperatureModifier,
};

use crate::registries::BiomeTemperatureModifier;
use crate::BlockPos;

pub fn biome_info_noise(x: f64, z: f64) -> f64 {
    shared_biome_info_noise(x, z)
}

pub fn biome_height_adjusted_temperature(
    base_temperature: f32,
    temperature_modifier: BiomeTemperatureModifier,
    pos: BlockPos,
    sea_level: i32,
) -> f32 {
    let modifier = match temperature_modifier {
        BiomeTemperatureModifier::None => TemperatureModifier::None,
        BiomeTemperatureModifier::Frozen => TemperatureModifier::Frozen,
    };
    height_adjusted_temperature(base_temperature, modifier, pos.x, pos.y, pos.z, sea_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_biome_info_noise_matches_vanilla_seed_samples() {
        assert_eq!(biome_info_noise(0.0, 0.0), 0.0);
        assert!((biome_info_noise(-496.0, -512.0) + 0.102_904_227_905_454_12).abs() < 1.0e-12);
    }

    #[test]
    fn shared_height_adjusted_temperature_preserves_world_api() {
        assert!(
            biome_height_adjusted_temperature(
                0.149,
                BiomeTemperatureModifier::None,
                BlockPos {
                    x: -512,
                    y: 81,
                    z: -511,
                },
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
}
