use crate::frozen_lookup::FrozenDatapackRegistryLookup;

use super::discrete::{DiscreteValue, Layer};
use super::{string, EnvironmentAttributeError, EnvironmentAttributeSources, Nbt, Result};

/// `EnvironmentAttributes.VILLAGER_ACTIVITY` and its baby twin
/// (`EnvironmentAttributes.java:157-162`), which is the schedule
/// `Villager.registerBrainGoals` reads (`Villager.java:218-226`).
const ADULT_ATTRIBUTE: &str = "minecraft:gameplay/villager_activity";
const BABY_ATTRIBUTE: &str = "minecraft:gameplay/baby_villager_activity";

/// The registered default both attributes carry
/// (`EnvironmentAttributes.java:157-162`).
const DEFAULT_ACTIVITY: &str = "minecraft:idle";

#[derive(Debug, Clone)]
pub struct DimensionVillagerActivityEnvironment {
    adult: Layer<String>,
    baby: Layer<String>,
}

impl DiscreteValue for String {
    fn default_value() -> Self {
        DEFAULT_ACTIVITY.to_owned()
    }

    fn from_canonical(
        value: &Nbt,
        _lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self> {
        string(value)
    }
}

impl DimensionVillagerActivityEnvironment {
    pub fn from_sources(
        sources: &EnvironmentAttributeSources<'_>,
        dimension_type_id: &str,
    ) -> std::result::Result<Self, EnvironmentAttributeError> {
        Ok(Self {
            adult: Layer::compile(sources, dimension_type_id, ADULT_ATTRIBUTE)?,
            baby: Layer::compile(sources, dimension_type_id, BABY_ATTRIBUTE)?,
        })
    }

    pub fn sample(
        &self,
        biome_id: &str,
        baby: bool,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<&str> {
        let layer = if baby { &self.baby } else { &self.adult };
        layer.sample(biome_id, &clock_ticks).map(String::as_str)
    }

    pub fn sample_attribute(
        &self,
        attribute: &'static bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute,
        biome_id: &str,
        clock_ticks: impl Fn(&str) -> Option<i64>,
    ) -> Option<&str> {
        let baby = match attribute.resource_id {
            ADULT_ATTRIBUTE => false,
            BABY_ATTRIBUTE => true,
            _ => return None,
        };
        self.sample(biome_id, baby, clock_ticks)
    }
}
