//! Tick-boundary replacement of reloadable worldgen resources.

use std::sync::Arc;

use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::tag_loader::TagLoadReport;
use bbb_server_worldgen::noise::biome::NoiseBiomeSource;

use super::features::GenericFeatureResources;
use super::{
    compile_builtin_structure_plan, compile_feature_resources, invalid_schema,
    BuiltInCarverProgram, NetherCarverProgram, NetherFeatureResources, NoiseFinalDensityProgram,
    NoiseStructurePlan, OverworldCarverProgram, SaveResult,
};

pub(super) fn compile_end_carvers(
    generation: &DatapackGeneration,
    tags: &TagLoadReport,
    world_seed: i64,
    end_biomes: &[&str],
    path: &str,
) -> SaveResult<(Option<BuiltInCarverProgram>, bool)> {
    let program = OverworldCarverProgram::from_registry(generation, tags, world_seed, end_biomes)
        .map_err(|error| invalid_schema(path, error.to_string()))?;
    if program.is_empty() {
        Ok((None, true))
    } else {
        Ok((Some(BuiltInCarverProgram::Overworld(program)), false))
    }
}

/// A fully compiled replacement which cannot fail once its program owner is
/// exclusively held at the server tick boundary.
pub struct NoiseReloadableResources {
    carvers: Option<BuiltInCarverProgram>,
    carvers_are_verified_empty: bool,
    structures: Option<NoiseStructurePlan>,
    feature_resources: Option<GenericFeatureResources>,
    nether_features: Option<NetherFeatureResources>,
}

impl NoiseFinalDensityProgram {
    /// `MinecraftServer.reloadResources` keeps the registry layers and rebuilds
    /// reloadable resources before installing them on the server thread
    /// (`MinecraftServer.java:1504-1544`, `ReloadableServerResources.java:76-108`).
    pub(crate) fn prepare_reloadable_resources(
        &self,
        startup: &Arc<DatapackStartup>,
    ) -> SaveResult<NoiseReloadableResources> {
        let path = format!(
            "noise_settings.{}.reloadable_resources",
            self.source.settings_id()
        );
        let frozen = self
            .structure_startup()
            .ok_or_else(|| invalid_schema(&path, "frozen reloadable resources are absent"))?;
        if !frozen
            .registries
            .same_frozen_registries(&startup.registries)
        {
            return Err(invalid_schema(
                &path,
                "reload attempted to replace the frozen dynamic registry generation",
            ));
        }
        if !startup.structures.warnings.is_empty() {
            return Err(invalid_schema(
                &path,
                "reload contains malformed structure-template winners",
            ));
        }
        let (carvers, carvers_are_verified_empty) = if let Some(end_biomes) = self
            .biome_source
            .as_deref()
            .and_then(NoiseBiomeSource::end_biome_order)
            .filter(|_| self.carvers.is_some() || self.carvers_are_verified_empty)
        {
            compile_end_carvers(
                &startup.registries,
                &startup.tags,
                self.world_seed,
                end_biomes,
                &path,
            )?
        } else {
            let carvers = match self.carvers.as_ref() {
                Some(BuiltInCarverProgram::Overworld(_)) => {
                    let biome_source = self.biome_source.as_deref().ok_or_else(|| {
                        invalid_schema(&path, "Overworld CARVERS lost its frozen biome source")
                    })?;
                    let possible_biomes = biome_source.possible_biome_order();
                    Some(BuiltInCarverProgram::Overworld(
                        OverworldCarverProgram::from_registry(
                            &startup.registries,
                            &startup.tags,
                            self.world_seed,
                            &possible_biomes,
                        )
                        .map_err(|error| invalid_schema(&path, error.to_string()))?,
                    ))
                }
                Some(BuiltInCarverProgram::Nether(_)) => {
                    let biome_source = self.biome_source.as_deref().ok_or_else(|| {
                        invalid_schema(&path, "Nether CARVERS lost its frozen biome source")
                    })?;
                    Some(BuiltInCarverProgram::Nether(
                        NetherCarverProgram::from_registry(
                            &startup.registries,
                            &startup.tags,
                            self.world_seed,
                            &self.source,
                            biome_source,
                        )
                        .map_err(|error| invalid_schema(&path, error.to_string()))?,
                    ))
                }
                None => None,
            };
            (carvers, false)
        };
        let structures =
            compile_builtin_structure_plan(startup, self.biome_source.as_deref(), &path)?;
        if structures.is_some() != self.structures.is_some() {
            return Err(invalid_schema(
                &path,
                "reload changed the frozen structure-plan owner family",
            ));
        }
        let (feature_resources, nether_features) = compile_feature_resources(
            startup,
            &self.source,
            self.biome_source.as_deref(),
            carvers.as_ref(),
        )?;
        if feature_resources.is_some() != self.feature_resources.is_some()
            || nether_features.is_some() != self.nether_features.is_some()
        {
            return Err(invalid_schema(
                &path,
                "reload changed the frozen feature-plan owner family",
            ));
        }
        Ok(NoiseReloadableResources {
            carvers,
            carvers_are_verified_empty,
            structures,
            feature_resources,
            nether_features,
        })
    }

    pub(crate) fn install_reloadable_resources(&mut self, replacement: NoiseReloadableResources) {
        self.carvers = replacement.carvers;
        self.carvers_are_verified_empty = replacement.carvers_are_verified_empty;
        self.structures = replacement.structures;
        self.structure_rings = std::sync::OnceLock::new();
        self.feature_resources = replacement.feature_resources;
        self.nether_features = replacement.nether_features;
    }

    pub(crate) fn clone_with_reloadable_resources(
        &self,
        replacement: NoiseReloadableResources,
    ) -> Self {
        let item_publication = std::sync::OnceLock::new();
        if let Some(publication) = self.item_publication.get() {
            let _ = item_publication.set(publication.clone());
        }
        Self {
            source: Arc::clone(&self.source),
            world_seed: self.world_seed,
            biome_source: self.biome_source.as_ref().map(Arc::clone),
            biome_names: self.biome_names.clone(),
            surface: self.surface.as_ref().map(Arc::clone),
            carvers: replacement.carvers,
            carvers_are_verified_empty: replacement.carvers_are_verified_empty,
            structures: replacement.structures,
            feature_resources: replacement.feature_resources,
            nether_features: replacement.nether_features,
            dimension_type_id: self.dimension_type_id.clone(),
            dimension_shape: self.dimension_shape,
            structure_rings: std::sync::OnceLock::new(),
            item_publication,
        }
    }

    #[cfg(test)]
    pub fn reloadable_resources_startup_is_for_test(&self, startup: &Arc<DatapackStartup>) -> bool {
        self.structure_startup()
            .is_some_and(|current| std::ptr::eq(current, startup.as_ref()))
    }

    #[cfg(test)]
    pub fn reloadable_feature_contains_for_test(
        &self,
        biome_id: &str,
        placed_feature_id: &str,
    ) -> bool {
        self.feature_resources
            .as_ref()
            .map(GenericFeatureResources::plan)
            .or_else(|| {
                self.nether_features
                    .as_ref()
                    .map(|resources| resources.program().plan())
            })
            .is_some_and(|plan| plan.biome_contains_placed_feature(biome_id, placed_feature_id))
    }

    #[cfg(test)]
    pub fn reloadable_structure_template_source_for_test(&self, template_id: &str) -> Option<&str> {
        self.structure_startup()?
            .structures
            .template(template_id)
            .map(|template| template.source_pack.as_str())
    }
}
