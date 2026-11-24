//! Built-in End City start selection and decoration ordering.

use bbb_server_worldgen::noise::end_city::{EndCityStructureStart, END_CITY_ID, END_CITY_TYPE_ID};
use bbb_server_worldgen::noise::structures::{
    NoiseStructureDecorationInvocation, StructureDecorationStep, StructureStartSetOwner,
};

use super::*;

pub(super) fn compile_builtin_structure_plan(
    startup: &DatapackStartup,
    biome_source: Option<&NoiseBiomeSource>,
    path: &str,
) -> SaveResult<Option<NoiseStructurePlan>> {
    let Some(biome_source) = biome_source else {
        return Ok(None);
    };
    let possible_biomes = biome_source.possible_biome_order();
    NoiseStructurePlan::from_startup(startup, &possible_biomes)
        .map(Some)
        .map_err(|error| invalid_schema(path, error.to_string()))
}

impl NoiseFinalDensityProgram {
    pub(super) fn validate_builtin_end_city_plan(
        &self,
        path: &str,
    ) -> SaveResult<&NoiseStructurePlan> {
        if self.biome_source.is_none() {
            return Err(invalid_schema(
                path,
                "End City requires an owned biome source",
            ));
        }
        let plan = self
            .structures
            .as_ref()
            .ok_or_else(|| invalid_schema(path, "End City requires the frozen structure plan"))?;
        if !plan
            .sets()
            .iter()
            .any(|set| plan.structure_start_owner(set) == StructureStartSetOwner::EndCity)
        {
            return Err(invalid_schema(
                path,
                "the End City definition, biome gate, or placement has no frozen owner",
            ));
        }
        Ok(plan)
    }

    pub(crate) fn owns_end_city_structure(&self, structure_id: &str) -> bool {
        self.structures.as_ref().is_some_and(|plan| {
            plan.structures()
                .get(structure_id)
                .is_some_and(|definition| {
                    definition.type_id == END_CITY_TYPE_ID
                        && definition.step == StructureDecorationStep::SurfaceStructures
                })
                && plan.sets().iter().any(|set| {
                    plan.structure_start_owner(set) == StructureStartSetOwner::EndCity
                        && set
                            .structures
                            .iter()
                            .any(|selection| selection.structure == structure_id)
                })
        })
    }

    #[cfg(test)]
    pub(crate) fn owns_end_city_set_for_test(&self, set_id: &str) -> bool {
        self.structures.as_ref().is_some_and(|plan| {
            plan.set(set_id).is_some_and(|set| {
                plan.structure_start_owner(set) == StructureStartSetOwner::EndCity
            })
        })
    }

    #[cfg(test)]
    pub(crate) fn end_city_set_candidate_for_test(
        &self,
        set_id: &str,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        self.structures.as_ref().is_some_and(|plan| {
            plan.random_spread_attempt_order(set_id, self.world_seed, chunk_x, chunk_z)
                .is_some()
        })
    }

    pub fn end_city_structure_decoration_invocation(
        &self,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        self.end_city_structure_decoration_invocation_for(END_CITY_ID, target_chunk)
    }

    pub(crate) fn end_city_structure_decoration_invocation_for(
        &self,
        structure_id: &str,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "noise_settings.{}.end_city_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        );
        let plan = self.validate_builtin_end_city_plan(&path)?;
        if !self.owns_end_city_structure(structure_id) {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the End City placement family",
            ));
        }
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| invalid_schema(&path, "structure is absent from decoration order"))?;
        let expected_step = StructureDecorationStep::SurfaceStructures.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "End City decoration step is {}, expected SURFACE_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }

    /// Generates the typed start before STRUCTURE_STARTS publication.
    pub fn generate_end_city_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing: &[EndCityStructureStart],
    ) -> SaveResult<Vec<EndCityStructureStart>> {
        let path = format!(
            "noise_settings.{}.end_city_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.validate_builtin_end_city_plan(&path)?;
        self.generate_end_city_structure_starts_for_plan_component(plan, chunk_x, chunk_z, existing)
    }

    pub(super) fn generate_end_city_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing: &[EndCityStructureStart],
    ) -> SaveResult<Vec<EndCityStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_end_city_structure_starts_for_plan_component(
                plan, chunk_x, chunk_z, existing,
            ))
    }

    #[cfg(test)]
    pub fn first_end_city_start_in_window(
        &self,
        radius: i32,
        require_ship: bool,
    ) -> SaveResult<Option<([i32; 2], Vec<EndCityStructureStart>)>> {
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                "noise_settings.end_city_structure_invocation.search",
                "End City search requires the frozen structure plan",
            )
        })?;
        for ring in 0..=radius {
            let chunks = if ring == 0 {
                vec![[0, 0]]
            } else {
                let mut chunks = Vec::with_capacity((ring as usize) * 8);
                for chunk_x in -ring..=ring {
                    chunks.push([chunk_x, -ring]);
                    chunks.push([chunk_x, ring]);
                }
                for chunk_z in (-ring + 1)..ring {
                    chunks.push([-ring, chunk_z]);
                    chunks.push([ring, chunk_z]);
                }
                chunks
            };
            for [chunk_x, chunk_z] in chunks {
                if plan
                    .random_spread_attempt_order(
                        bbb_server_worldgen::noise::end_city::END_CITY_SET_ID,
                        self.world_seed,
                        chunk_x,
                        chunk_z,
                    )
                    .is_none_or(|attempts| attempts.is_empty())
                {
                    continue;
                }
                let starts =
                    self.generate_end_city_structure_starts_component(chunk_x, chunk_z, &[])?;
                if !starts.is_empty()
                    && (!require_ship
                        || starts.iter().any(|start| {
                            start
                                .pieces()
                                .iter()
                                .any(|piece| piece.template() == "minecraft:end_city/ship")
                        }))
                {
                    return Ok(Some(([chunk_x, chunk_z], starts)));
                }
            }
        }
        Ok(None)
    }
}
