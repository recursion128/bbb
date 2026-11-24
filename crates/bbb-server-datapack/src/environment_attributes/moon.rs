use bbb_server_types::environment::MoonPhase;

use crate::frozen_lookup::FrozenDatapackRegistryLookup;

use super::discrete::{DiscreteValue, Layer};
use super::{error, string, EnvironmentAttributeError, EnvironmentAttributeSources, Nbt, Result};

const ATTRIBUTE: &str = "minecraft:visual/moon_phase";

#[derive(Debug, Clone)]
pub struct DimensionMoonEnvironment {
    layer: Layer<MoonPhase>,
}

impl DiscreteValue for MoonPhase {
    fn default_value() -> Self {
        Self::FULL_MOON
    }

    fn from_canonical(
        value: &Nbt,
        _lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        let name = string(value)?;
        Self::from_serialized_name(&name).ok_or_else(|| error(format!("unknown moon phase {name}")))
    }
}

impl DimensionMoonEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            layer: Layer::compile(sources, dimension_type_id, ATTRIBUTE)?,
        })
    }

    // EnvironmentAttributeSystem.java:62-96.
    pub fn sample_phase(
        &self,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<MoonPhase> {
        self.layer.sample(biome_id, &clock_ticks).copied()
    }

    pub fn sample(&self, biome_id: &str, clock_ticks: impl Fn(&str) -> Option<i64>) -> Option<f32> {
        self.sample_phase(biome_id, clock_ticks)
            .map(MoonPhase::brightness)
    }
}

#[cfg(test)]
mod tests;
