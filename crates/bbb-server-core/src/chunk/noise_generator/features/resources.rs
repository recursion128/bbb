use std::ops::Deref;
use std::sync::Arc;

use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_worldgen::noise::biome::NoiseBiomeSource;
use bbb_server_worldgen::noise::features::NoiseFeaturesPlan;

use super::super::{invalid_schema, SaveResult};
use crate::registry_tags::RegistryTagSnapshot;

pub(crate) struct GenericFeatureResources {
    startup: Arc<DatapackStartup>,
    plan: NoiseFeaturesPlan,
    tags: Arc<RegistryTagSnapshot>,
}

impl GenericFeatureResources {
    pub(crate) fn compile(
        startup: &Arc<DatapackStartup>,
        biome_source: &NoiseBiomeSource,
        path: &str,
    ) -> SaveResult<Self> {
        let possible_biomes = biome_source.possible_biome_order();
        let plan = NoiseFeaturesPlan::from_registry_with_tags(
            &startup.registries,
            &startup.tags,
            &possible_biomes,
        )
        .map_err(|error| invalid_schema(path, error.to_string()))?;
        let tags = Arc::new(
            RegistryTagSnapshot::from_tag_load_report_with_content_registry(
                &startup.tags,
                Arc::new(startup.content_registry().clone()),
            ),
        );
        Ok(Self {
            startup: Arc::clone(startup),
            plan,
            tags,
        })
    }

    pub(crate) fn from_plan(startup: &Arc<DatapackStartup>, plan: &NoiseFeaturesPlan) -> Self {
        Self {
            startup: Arc::clone(startup),
            plan: plan.clone(),
            tags: Arc::new(
                RegistryTagSnapshot::from_tag_load_report_with_content_registry(
                    &startup.tags,
                    Arc::new(startup.content_registry().clone()),
                ),
            ),
        }
    }

    pub(crate) fn startup(&self) -> &DatapackStartup {
        &self.startup
    }

    pub(crate) const fn plan(&self) -> &NoiseFeaturesPlan {
        &self.plan
    }

    pub(crate) fn tags(&self) -> &RegistryTagSnapshot {
        &self.tags
    }
}

impl Deref for GenericFeatureResources {
    type Target = DatapackStartup;

    fn deref(&self) -> &Self::Target {
        self.startup()
    }
}
