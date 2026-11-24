//! Frozen structure placement queries.

use super::super::{invalid_schema, NoiseFinalDensityProgram, SaveResult};
use bbb_server_worldgen::noise::scattered::ScatteredFeatureKind;
use bbb_server_worldgen::noise::structures::{
    NoiseStructureDecorationInvocation, StructureDecorationStep,
};

impl NoiseFinalDensityProgram {
    pub fn leftover_scattered_structure_decoration_invocation(
        &self,
        kind: ScatteredFeatureKind,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        self.leftover_scattered_structure_decoration_invocation_for(
            kind,
            kind.structure_id(),
            target_chunk,
        )
    }

    pub(crate) fn leftover_scattered_structure_decoration_invocation_for(
        &self,
        kind: ScatteredFeatureKind,
        structure_id: &str,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let family = leftover_family(kind)?;
        let path = self.leftover_invocation_path(kind, structure_id, target_chunk)?;
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                format!("{family} placement requires the frozen structure plan"),
            )
        })?;
        if !plan
            .structures()
            .get(structure_id)
            .is_some_and(|definition| {
                definition.type_id == kind.type_id()
                    && definition.step == StructureDecorationStep::SurfaceStructures
            })
        {
            return Err(invalid_schema(
                &path,
                format!("structure is not owned by the {family} placement family"),
            ));
        }
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from the decoration order")
            })?;
        let expected_step = StructureDecorationStep::SurfaceStructures.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "{family} decoration step is {}, expected SURFACE_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }

    fn leftover_invocation_path(
        &self,
        kind: ScatteredFeatureKind,
        structure_id: &str,
        target_chunk: [i32; 2],
    ) -> SaveResult<String> {
        let family = leftover_family(kind)?;
        Ok(format!(
            "noise_settings.{}.{family}_structure_invocation[{},{}].{}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1],
            structure_id
        ))
    }
}

fn leftover_family(kind: ScatteredFeatureKind) -> SaveResult<&'static str> {
    kind.leftover_features_label().ok_or_else(|| {
        invalid_schema(
            "noise_settings.leftover_scattered_structure_invocation",
            "structure is not owned by the leftover scattered placement family",
        )
    })
}
