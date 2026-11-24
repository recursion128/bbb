use super::*;
use crate::noise::features::{decoration_seed, expected_feature_seed};

impl NoiseStructurePlan {
    /// Structure half of `ChunkGenerator.applyBiomeDecoration`: the dynamic
    /// registry order is retained within each generation step and the index
    /// resets to zero at every step (`ChunkGenerator.java:313-359`).
    pub fn decoration_invocations_for_chunk(
        &self,
        world_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Vec<NoiseStructureDecorationInvocation> {
        let decoration_seed = decoration_seed(world_seed, chunk_x, chunk_z);
        let mut invocations = Vec::with_capacity(self.structure_order.len());
        for step in StructureDecorationStep::ALL {
            let mut step_index = 0;
            for structure in self.structure_order.iter() {
                let definition = self
                    .structures
                    .get(structure)
                    .expect("registry-ordered structure was decoded");
                if definition.step != step {
                    continue;
                }
                invocations.push(NoiseStructureDecorationInvocation {
                    step: step.ordinal(),
                    step_index,
                    structure: structure.clone(),
                    feature_seed: expected_feature_seed(
                        decoration_seed,
                        i32::try_from(step_index).expect("a registry step index fits a Java int"),
                        i32::try_from(step.ordinal())
                            .expect("the fixed decoration ordinal fits a Java int"),
                    ),
                });
                step_index += 1;
            }
        }
        invocations
    }
}
