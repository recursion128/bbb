use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::TagLoadReport;

use crate::noise::biome::NoiseBiomeSource;

use super::{refuse, NoiseFeaturesError, NoiseFeaturesPlan};

pub const END_FEATURE_BIOMES: [&str; 5] = [
    "minecraft:the_end",
    "minecraft:end_highlands",
    "minecraft:end_midlands",
    "minecraft:small_end_islands",
    "minecraft:end_barrens",
];

pub struct EndFeaturesProgram {
    plan: NoiseFeaturesPlan,
}

impl EndFeaturesProgram {
    pub fn from_registry(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        biome_source: &NoiseBiomeSource,
    ) -> Result<Self, NoiseFeaturesError> {
        if biome_source.end_biome_order() != Some(END_FEATURE_BIOMES.as_slice()) {
            return Err(refuse(
                "noise_generator.end.features",
                "requires the exact built-in End biome source",
            ));
        }
        Ok(Self {
            plan: NoiseFeaturesPlan::from_registry_with_tags(
                generation,
                tags,
                &END_FEATURE_BIOMES,
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
                        "noise_settings.minecraft:end.features.apply",
                        "feature-region block-coordinate range underflows",
                    )
                })?;
            coordinate
                .checked_add(1)
                .and_then(|value| value.checked_mul(16))
                .and_then(|value| value.checked_add(15))
                .ok_or_else(|| {
                    refuse(
                        "noise_settings.minecraft:end.features.apply",
                        "feature-region block-coordinate range overflows",
                    )
                })?;
        }
        Ok(())
    }
}
