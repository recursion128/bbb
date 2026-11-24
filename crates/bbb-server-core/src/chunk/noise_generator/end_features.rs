use std::sync::Arc;

use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::schema_support::invalid_schema;
use bbb_server_worldgen::noise::biome::NoiseBiomeSource;
use bbb_server_worldgen::noise::carver::BuiltInCarverProgram;
use bbb_server_worldgen::noise::density::NoiseDensitySource;
use bbb_server_worldgen::noise::features::end::EndFeaturesProgram;

use super::features::GenericFeatureResources;
use super::nether_features::NetherFeatureResources;
use super::SaveResult;

pub(super) fn compile_feature_resources(
    startup: &Arc<DatapackStartup>,
    source: &NoiseDensitySource,
    biome_source: Option<&NoiseBiomeSource>,
    carvers: Option<&BuiltInCarverProgram>,
) -> SaveResult<(
    Option<GenericFeatureResources>,
    Option<NetherFeatureResources>,
)> {
    let Some(biome_source) = biome_source.filter(|source| source.end_biome_order().is_some())
    else {
        return super::nether_features::compile_feature_resources(
            startup,
            source,
            biome_source,
            carvers,
        );
    };
    if matches!(carvers, Some(BuiltInCarverProgram::Nether(_))) {
        return Err(invalid_schema(
            &format!("noise_settings.{}.features", source.settings_id()),
            "The End FEATURES cannot share a Nether CARVERS owner",
        ));
    }
    let program =
        EndFeaturesProgram::from_registry(&startup.registries, &startup.tags, biome_source)
            .map_err(|error| {
                invalid_schema(
                    &format!("noise_settings.{}.features", source.settings_id()),
                    error.to_string(),
                )
            })?;
    Ok((
        Some(GenericFeatureResources::from_plan(startup, program.plan())),
        None,
    ))
}
