use super::*;
#[cfg(test)]
use bbb_server_worldgen::noise::buried_treasure::BuriedTreasureGenerationStub;
use bbb_server_worldgen::noise::buried_treasure::BuriedTreasureStructureStart;
#[cfg(test)]
use bbb_server_worldgen::noise::structures::StructureHeightmap;

impl NoiseFinalDensityProgram {
    pub(crate) fn owns_buried_treasure_structure(&self, structure_id: &str) -> bool {
        let Some(plan) = self.structures.as_ref() else {
            return false;
        };
        plan.sets().iter().any(|set| {
            plan.structure_start_owner(set) == StructureStartSetOwner::BuriedTreasure
                && set
                    .structures
                    .iter()
                    .any(|selection| selection.structure == structure_id)
        })
    }

    /// Selects every registered buried-treasure start without publishing the
    /// STRUCTURE_STARTS status (`ChunkGenerator.java:464-589`).
    pub fn generate_buried_treasure_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[BuriedTreasureStructureStart],
    ) -> SaveResult<Vec<BuriedTreasureStructureStart>> {
        let path = format!(
            "noise_settings.{}.buried_treasure_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "buried-treasure starts require the frozen structure plan",
            )
        })?;
        self.generate_buried_treasure_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_buried_treasure_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[BuriedTreasureStructureStart],
    ) -> SaveResult<Vec<BuriedTreasureStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_buried_treasure_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    #[cfg(test)]
    pub(crate) fn generate_buried_treasure_start_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        has_existing_valid_start: bool,
    ) -> SaveResult<Option<BuriedTreasureGenerationStub>> {
        let path = format!(
            "noise_settings.{}.buried_treasure_start[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "buried treasure starts require the frozen structure plan",
            )
        })?;
        let startup = self.structure_startup().ok_or_else(|| {
            invalid_schema(
                &path,
                "buried treasure starts require frozen structure resources",
            )
        })?;
        let biome_source = self.biome_source.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "buried treasure starts require a frozen biome source",
            )
        })?;
        let biome_registry = startup
            .registries
            .registry("minecraft:worldgen/biome")
            .ok_or_else(|| invalid_schema(&path, "the frozen biome registry is absent"))?;
        let mut biome_search = biome_source.create_search();
        Ok(plan.buried_treasure_generation_stub(
            self.world_seed,
            chunk_x,
            chunk_z,
            has_existing_valid_start,
            |x, z, heightmap| match heightmap {
                StructureHeightmap::OceanFloorWg => {
                    self.fresh_base_height(x, z, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID)
                }
                _ => None,
            },
            |position| {
                let biome = biome_source.sample(
                    &self.source,
                    position.x.div_euclid(4),
                    position.y.div_euclid(4),
                    position.z.div_euclid(4),
                    &mut biome_search,
                );
                usize::try_from(biome)
                    .ok()
                    .and_then(|biome| biome_registry.elements.get(biome))
                    .map(|element| element.id.clone())
            },
        ))
    }

    pub fn buried_treasure_structure_decoration_invocation_for(
        &self,
        target_chunk: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "noise_settings.{}.buried_treasure_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        );
        if !self.owns_buried_treasure_structure(structure_id) {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the buried-treasure placement family",
            ));
        }
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "buried-treasure placement requires the frozen structure plan",
            )
        })?;
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from the decoration order")
            })?;
        let expected_step = StructureDecorationStep::UndergroundStructures.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "buried-treasure decoration step is {}, expected UNDERGROUND_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }
}
