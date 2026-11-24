use super::*;
use crate::registry_tags::RegistryTagSnapshot;

pub(crate) struct FeaturePlacementSource<'a> {
    pub(super) world_seed: i64,
    pub(super) sea_level: i32,
    pub(super) dimension_min_y: i32,
    pub(super) generation_min_y: i32,
    pub(super) generation_depth: i32,
    pub(super) publication: Option<&'a GenerationItemPublication>,
    pub(super) resources: &'a DatapackStartup,
    pub(super) tags: Option<&'a RegistryTagSnapshot>,
    pub(super) plan: &'a NoiseFeaturesPlan,
}

impl FeaturePlacementSource<'_> {
    pub(crate) fn step_count(&self) -> usize {
        self.plan.step_count().max(11)
    }
}

impl NoiseFinalDensityProgram {
    pub(crate) fn feature_placement_source(
        &self,
        target: [i32; 2],
    ) -> SaveResult<FeaturePlacementSource<'_>> {
        let (resources, tags) = if let Some(features) = self.feature_resources.as_ref() {
            (features.startup(), Some(features.tags()))
        } else if let Some(features) = self.nether_features.as_ref() {
            (features.startup(), Some(features.tags()))
        } else {
            return Err(invalid_schema(
                "FEATURES",
                "feature execution requires frozen resources",
            ));
        };
        let plan = if let Some(features) = self.nether_features.as_ref() {
            features
                .program()
                .validate_target(target[0], target[1])
                .map_err(|error| invalid_schema("FEATURES", error.to_string()))?;
            features.program().plan()
        } else {
            self.feature_resources.as_ref().unwrap().plan()
        };
        Ok(FeaturePlacementSource {
            world_seed: self.world_seed,
            sea_level: self.source.sea_level(),
            dimension_min_y: self.dimension_shape.min_y,
            generation_min_y: self.source.min_y(),
            generation_depth: self.source.height(),
            publication: self.item_publication(),
            resources,
            tags,
            plan,
        })
    }
}

impl<'a> crate::chunk::structure_generation::StructureGenerationContext<'a> {
    pub(crate) fn flat_feature_placement_source(
        &self,
        plan: &'a NoiseFeaturesPlan,
    ) -> FeaturePlacementSource<'a> {
        FeaturePlacementSource {
            world_seed: self.world_seed,
            sea_level: self.sea_level,
            dimension_min_y: self.dimension_shape.min_y,
            generation_min_y: 0,
            generation_depth: 384,
            publication: self.item_publication,
            resources: self.startup,
            tags: None,
            plan,
        }
    }
}
