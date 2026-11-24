//! Frozen structure placement queries.

use super::super::NoiseFinalDensityProgram;
#[cfg(test)]
use super::super::{invalid_schema, SaveResult};
#[cfg(test)]
use bbb_server_worldgen::noise::structures::NoiseStructureDecorationInvocation;
use bbb_server_worldgen::noise::structures::StructureDecorationStep;

impl NoiseFinalDensityProgram {
    pub fn owned_jigsaw_features_structure<'a>(
        &self,
        exclusive: Option<&'a str>,
    ) -> Option<&'a str> {
        let exclusive = exclusive?;
        let definition = self.structures.as_ref()?.structures().get(exclusive)?;
        definition.jigsaw.as_ref()?;
        owned_jigsaw_features_step(definition.step).then_some(exclusive)
    }

    #[cfg(test)]
    pub(crate) fn jigsaw_structure_decoration_invocation(
        &self,
        target_chunk: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = self.jigsaw_invocation_path(target_chunk, structure_id);
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(&path, "jigsaw placement requires the frozen structure plan")
        })?;
        let definition = plan.structures().get(structure_id).ok_or_else(|| {
            invalid_schema(&path, "structure is absent from the frozen structure plan")
        })?;
        if definition.jigsaw.is_none() || !owned_jigsaw_features_step(definition.step) {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the jigsaw FEATURES placement family",
            ));
        }
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from the decoration order")
            })?;
        let expected_step = definition.step.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "jigsaw decoration step is {}, expected {:?} ordinal {expected_step}",
                    invocation.step, definition.step
                ),
            ));
        }
        Ok(invocation)
    }

    #[cfg(test)]
    fn jigsaw_invocation_path(&self, target_chunk: [i32; 2], structure_id: &str) -> String {
        format!(
            "noise_settings.{}.jigsaw_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        )
    }
}

fn owned_jigsaw_features_step(step: StructureDecorationStep) -> bool {
    matches!(
        step,
        StructureDecorationStep::SurfaceStructures
            | StructureDecorationStep::UndergroundStructures
            | StructureDecorationStep::UndergroundDecoration
    )
}
