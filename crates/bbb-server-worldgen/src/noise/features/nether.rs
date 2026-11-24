use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::TagLoadReport;

use crate::noise::biome::NoiseBiomeSource;
use crate::noise::density::NoiseDensitySource;

use super::{refuse, NoiseFeaturesError, NoiseFeaturesPlan};

pub const NETHER_FEATURE_BIOMES: [&str; 5] = [
    "minecraft:nether_wastes",
    "minecraft:soul_sand_valley",
    "minecraft:crimson_forest",
    "minecraft:warped_forest",
    "minecraft:basalt_deltas",
];

pub struct NetherFeaturesProgram {
    plan: NoiseFeaturesPlan,
}

impl NetherFeaturesProgram {
    pub fn from_registry(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        source: &NoiseDensitySource,
        biome_source: &NoiseBiomeSource,
    ) -> Result<Self, NoiseFeaturesError> {
        if source.min_y() != 0 || source.height() != 128 {
            return Err(refuse(
                "noise_settings.minecraft:nether.features",
                "requires a compatible Nether density source",
            ));
        }
        if biome_source.nether_biome_order() != Some(NETHER_FEATURE_BIOMES.as_slice()) {
            return Err(refuse(
                "noise_settings.minecraft:nether.features",
                "requires the exact built-in Nether biome source",
            ));
        }
        Ok(Self {
            plan: NoiseFeaturesPlan::from_registry_with_tags(
                generation,
                tags,
                &NETHER_FEATURE_BIOMES,
            )?,
        })
    }

    pub const fn plan(&self) -> &NoiseFeaturesPlan {
        &self.plan
    }

    pub fn validate_target(&self, chunk_x: i32, chunk_z: i32) -> Result<(), NoiseFeaturesError> {
        for coordinate in [chunk_x, chunk_z] {
            coordinate
                .checked_sub(1)
                .and_then(|value| value.checked_mul(16))
                .ok_or_else(|| {
                    refuse(
                        "noise_settings.minecraft:nether.features.apply",
                        "feature-region block-coordinate range underflows",
                    )
                })?;
            coordinate
                .checked_add(1)
                .and_then(|value| value.checked_mul(16))
                .and_then(|value| value.checked_add(15))
                .ok_or_else(|| {
                    refuse(
                        "noise_settings.minecraft:nether.features.apply",
                        "feature-region block-coordinate range overflows",
                    )
                })?;
        }
        Ok(())
    }
}
