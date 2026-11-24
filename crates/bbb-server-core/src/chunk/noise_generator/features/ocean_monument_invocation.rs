//! Frozen structure placement queries.

use super::super::{invalid_schema, NoiseFinalDensityProgram, SaveResult};
use bbb_server_worldgen::noise::ocean_monument::OCEAN_MONUMENT_ID;
#[cfg(test)]
use bbb_server_worldgen::noise::ocean_monument::{
    OceanMonumentStructureStart, OCEAN_MONUMENT_SET_ID,
};
use bbb_server_worldgen::noise::structures::{
    NoiseStructureDecorationInvocation, StructureDecorationStep,
};

impl NoiseFinalDensityProgram {
    pub fn leftover_ocean_monument_structure_decoration_invocation(
        &self,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        self.leftover_ocean_monument_structure_decoration_invocation_for(
            target_chunk,
            OCEAN_MONUMENT_ID,
        )
    }

    pub fn leftover_ocean_monument_structure_decoration_invocation_for(
        &self,
        target_chunk: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = self.ocean_monument_invocation_path(target_chunk, structure_id);
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "ocean-monument placement requires the frozen structure plan",
            )
        })?;
        if !self.owns_ocean_monument_structure(structure_id) {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the ocean-monument placement family",
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
                    "ocean-monument decoration step is {}, expected SURFACE_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }

    #[cfg(test)]
    pub fn first_overworld_ocean_monument_start_in_window(
        &self,
        radius: i32,
    ) -> SaveResult<Option<([i32; 2], Vec<OceanMonumentStructureStart>)>> {
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                "noise_settings.ocean_monument_structure_invocation.search",
                "ocean-monument search requires the frozen structure plan",
            )
        })?;
        const PINNED: [i32; 2] = [47, -48];
        if PINNED[0].abs() <= radius && PINNED[1].abs() <= radius {
            if let Some(attempts) = plan.random_spread_attempt_order(
                OCEAN_MONUMENT_SET_ID,
                self.world_seed,
                PINNED[0],
                PINNED[1],
            ) {
                if !attempts.is_empty() {
                    let starts = self.generate_ocean_monument_structure_starts_component(
                        PINNED[0],
                        PINNED[1],
                        &[],
                    )?;
                    if !starts.is_empty() {
                        return Ok(Some((PINNED, starts)));
                    }
                }
            }
        }
        for ring in 0..=radius {
            let chunks = if ring == 0 {
                vec![[0, 0]]
            } else {
                let mut ring_chunks = Vec::with_capacity((ring as usize) * 8);
                for chunk_x in -ring..=ring {
                    ring_chunks.push([chunk_x, -ring]);
                    ring_chunks.push([chunk_x, ring]);
                }
                for chunk_z in (-ring + 1)..ring {
                    ring_chunks.push([-ring, chunk_z]);
                    ring_chunks.push([ring, chunk_z]);
                }
                ring_chunks
            };
            for [chunk_x, chunk_z] in chunks {
                if [chunk_x, chunk_z] == PINNED {
                    continue;
                }
                let Some(attempts) = plan.random_spread_attempt_order(
                    OCEAN_MONUMENT_SET_ID,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                ) else {
                    continue;
                };
                if attempts.is_empty() {
                    continue;
                }
                let starts =
                    self.generate_ocean_monument_structure_starts_component(chunk_x, chunk_z, &[])?;
                if !starts.is_empty() {
                    return Ok(Some(([chunk_x, chunk_z], starts)));
                }
            }
        }
        Ok(None)
    }

    fn ocean_monument_invocation_path(&self, target_chunk: [i32; 2], structure_id: &str) -> String {
        format!(
            "noise_settings.{}.ocean_monument_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        )
    }
}
