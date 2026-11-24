use std::sync::Arc;

use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_worldgen::noise::biome::NoiseBiomeSource;
use bbb_server_worldgen::noise::carver::BuiltInCarverProgram;
use bbb_server_worldgen::noise::density::NoiseDensitySource;
use bbb_server_worldgen::noise::features::nether::NetherFeaturesProgram;

use super::features::GenericFeatureResources;
use super::{invalid_schema, SaveResult};
use crate::registry_tags::RegistryTagSnapshot;

pub(super) struct NetherFeatureResources {
    startup: Arc<DatapackStartup>,
    program: NetherFeaturesProgram,
    tags: Arc<RegistryTagSnapshot>,
}

impl NetherFeatureResources {
    pub(super) fn startup(&self) -> &DatapackStartup {
        &self.startup
    }

    pub(super) const fn program(&self) -> &NetherFeaturesProgram {
        &self.program
    }

    pub(super) fn tags(&self) -> &RegistryTagSnapshot {
        &self.tags
    }
}

pub(super) fn compile_feature_resources(
    startup: &Arc<DatapackStartup>,
    source: &NoiseDensitySource,
    biome_source: Option<&NoiseBiomeSource>,
    carvers: Option<&BuiltInCarverProgram>,
) -> SaveResult<(
    Option<GenericFeatureResources>,
    Option<NetherFeatureResources>,
)> {
    match carvers {
        Some(BuiltInCarverProgram::Overworld(_)) => {
            let biome_source = biome_source.ok_or_else(|| {
                invalid_schema(
                    "noise_settings.features",
                    "generic FEATURES require the frozen biome source",
                )
            })?;
            let path = format!("noise_settings.{}.features", source.settings_id());
            Ok((
                Some(GenericFeatureResources::compile(
                    startup,
                    biome_source,
                    &path,
                )?),
                None,
            ))
        }
        Some(BuiltInCarverProgram::Nether(_)) => {
            let biome_source = biome_source.ok_or_else(|| {
                invalid_schema(
                    "noise_settings.minecraft:nether.features",
                    "built-in Nether features require the biome source",
                )
            })?;
            let path = format!("noise_settings.{}.features", source.settings_id());
            let program = NetherFeaturesProgram::from_registry(
                &startup.registries,
                &startup.tags,
                source,
                biome_source,
            )
            .map_err(|error| invalid_schema(&path, error.to_string()))?;
            Ok((
                None,
                Some(NetherFeatureResources {
                    startup: Arc::clone(startup),
                    program,
                    tags: Arc::new(
                        RegistryTagSnapshot::from_tag_load_report_with_content_registry(
                            &startup.tags,
                            Arc::new(startup.content_registry().clone()),
                        ),
                    ),
                }),
            ))
        }
        None => Ok((None, None)),
    }
}
