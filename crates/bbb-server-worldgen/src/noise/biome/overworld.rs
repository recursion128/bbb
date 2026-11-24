//! Built-in Overworld parameter catalog and climate lookup.

use std::sync::OnceLock;

use crate::noise::density::ClimateSampler;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::dynamic_registry::DatapackGeneration;

use super::climate::{ParameterPoint, ParameterTree};
use super::overworld_builder::overworld_preset;
use super::{resolve_multi_noise_preset, MultiNoisePreset, NoiseBiomeError, BIOME_REGISTRY};

pub struct OverworldBiomeSearch {
    last_result: Option<usize>,
}

impl OverworldBiomeSearch {
    pub const fn new() -> Self {
        Self { last_result: None }
    }
}

impl Default for OverworldBiomeSearch {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OverworldBiomeSource {
    points: Vec<ParameterPoint>,
    tree: &'static ParameterTree,
    registry_len: usize,
}

pub static OVERWORLD_PARAMETER_TREE: OnceLock<ParameterTree> = OnceLock::new();

/// The five Overworld-preset settings registered together by
/// `NoiseGeneratorSettings.java:49-84`.
pub const OVERWORLD_NOISE_SETTINGS: [&str; 5] = [
    "minecraft:overworld",
    "minecraft:large_biomes",
    "minecraft:amplified",
    "minecraft:caves",
    "minecraft:floating_islands",
];

impl OverworldBiomeSource {
    pub fn biome_order() -> &'static [&'static str] {
        &overworld_preset().biome_order
    }

    pub fn from_registry(
        generation: &DatapackGeneration,
        settings_id: &str,
        preset_id: &str,
    ) -> Result<Self, NoiseBiomeError> {
        let preset = resolve_multi_noise_preset(
            generation,
            &NbtSummaryValue::string(preset_id),
            "biome_source.preset",
        )?;
        Self::from_preset(generation, settings_id, preset)
    }

    pub(super) fn from_preset(
        generation: &DatapackGeneration,
        _settings_id: &str,
        preset: MultiNoisePreset,
    ) -> Result<Self, NoiseBiomeError> {
        if preset != MultiNoisePreset::Overworld {
            return Err(super::refuse(
                "biome_source.preset",
                format!(
                    "preset {} has no Overworld biome runtime owner",
                    preset.id()
                ),
            ));
        }

        let biomes = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| super::refuse(BIOME_REGISTRY, "registry is absent"))?;
        let resolve = |id: &str| {
            biomes
                .elements
                .iter()
                .position(|element| element.id == id)
                .and_then(|index| i32::try_from(index).ok())
                .ok_or_else(|| {
                    super::refuse(BIOME_REGISTRY, format!("has no published element {id}"))
                })
        };
        let points = overworld_preset()
            .points
            .iter()
            .map(|point| Ok(ParameterPoint::new(point.ranges, 0, resolve(point.biome)?)))
            .collect::<Result<Vec<_>, NoiseBiomeError>>()?;
        let tree = OVERWORLD_PARAMETER_TREE.get_or_init(|| ParameterTree::new(&points));
        Ok(Self {
            points,
            tree,
            registry_len: biomes.elements.len(),
        })
    }

    pub const fn registry_len(&self) -> usize {
        self.registry_len
    }

    pub fn sample(
        &self,
        climate: &mut impl ClimateSampler,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
        search: &mut OverworldBiomeSearch,
    ) -> i32 {
        let target = climate
            .climate_target(quart_x, quart_y, quart_z)
            .expect("Overworld source is compiled with six climate functions");
        let winner = self
            .tree
            .search(&self.points, &target, &mut search.last_result);
        self.points[winner].biome
    }
}

#[cfg(test)]
mod tests {
    use super::super::overworld_builder::overworld_preset;

    #[test]
    fn builder_emits_the_overworld_preset_census() {
        let preset = overworld_preset();
        assert_eq!(preset.points.len(), 7593);
        assert_eq!(preset.biome_order.len(), 54);
        assert_eq!(preset.biome_order[0], "minecraft:mushroom_fields");
        assert_eq!(preset.points.last().unwrap().biome, "minecraft:deep_dark");
    }
}
