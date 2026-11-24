//! Nether structure starts and shared structure resource access.

use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_worldgen::noise::nether_fortress::FortressStructureStart;
use bbb_server_worldgen::noise::nether_fossil::NetherFossilStructureStart;
use bbb_server_worldgen::noise::structure_starts::JigsawStructureStart;
use bbb_server_worldgen::noise::structures::NoiseStructureDecorationInvocation;
#[cfg(test)]
use bbb_server_worldgen::noise::structures::StructureStartSetOwner;

use super::*;
use crate::chunk::blending::BlendingResolver;

impl NoiseFinalDensityProgram {
    #[cfg(test)]
    pub(crate) fn owns_fortress_structure(&self, structure_id: &str) -> bool {
        self.structure_generation_context().is_ok_and(|context| {
            context.placement_family(structure_id)
                == Some(crate::chunk::noise_generator::features::StructurePlacementFamily::Fortress)
        })
    }

    #[cfg(test)]
    pub(crate) fn owns_nether_fossil_structure(&self, structure_id: &str) -> bool {
        self.structure_generation_context().is_ok_and(|context| {
            context.placement_family(structure_id)
                == Some(
                    crate::chunk::noise_generator::features::StructurePlacementFamily::NetherFossil,
                )
        })
    }

    #[cfg(test)]
    pub(crate) fn nether_structure_set_owner_for_test(
        &self,
        set_id: &str,
    ) -> Option<StructureStartSetOwner> {
        let plan = self.structures.as_ref()?;
        plan.set(set_id).map(|set| plan.structure_start_owner(set))
    }

    pub(super) fn structure_startup(&self) -> Option<&DatapackStartup> {
        self.feature_resources.as_deref().or_else(|| {
            self.nether_features
                .as_ref()
                .map(|features| features.startup())
        })
    }

    pub fn structure_decoration_invocation(
        &self,
        target: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "noise_settings.{}.structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target[0],
            target[1]
        );
        self.structures
            .as_ref()
            .ok_or_else(|| invalid_schema(&path, "frozen structure plan is absent"))?
            .decoration_invocations_for_chunk(self.world_seed, target[0], target[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from frozen decoration order")
            })
    }

    pub fn generate_nether_complex_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_jigsaw: &[JigsawStructureStart],
        existing_fortress: &[FortressStructureStart],
    ) -> SaveResult<(Vec<JigsawStructureStart>, Vec<FortressStructureStart>)> {
        let path = format!(
            "noise_settings.{}.nether_complex_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self
            .structures
            .as_ref()
            .ok_or_else(|| invalid_schema(&path, "Nether complex starts require a frozen plan"))?;
        self.generate_nether_complex_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_jigsaw,
            existing_fortress,
        )
    }

    pub(super) fn generate_nether_complex_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_jigsaw: &[JigsawStructureStart],
        existing_fortress: &[FortressStructureStart],
    ) -> SaveResult<(Vec<JigsawStructureStart>, Vec<FortressStructureStart>)> {
        Ok(self
            .structure_generation_context()?
            .generate_nether_complex_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_jigsaw,
                existing_fortress,
            ))
    }

    pub fn generate_nether_fossil_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing: &[NetherFossilStructureStart],
    ) -> SaveResult<Vec<NetherFossilStructureStart>> {
        let path = format!(
            "noise_settings.{}.nether_fossil_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self
            .structures
            .as_ref()
            .ok_or_else(|| invalid_schema(&path, "Nether fossil starts require a frozen plan"))?;
        self.generate_nether_fossil_structure_starts_for_plan_component(
            plan, chunk_x, chunk_z, existing,
        )
    }

    pub(super) fn generate_nether_fossil_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing: &[NetherFossilStructureStart],
    ) -> SaveResult<Vec<NetherFossilStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_nether_fossil_structure_starts_for_plan_component(
                plan, chunk_x, chunk_z, existing,
            ))
    }

    /// Generates through Vanilla's CARVERS status for a built-in noise source.
    /// Features, persistence and serving remain outside this owner.
    pub fn generate_fresh_carved_chunk(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.generate_fresh_carved_chunk_with_beardifier(
            chunk_x,
            chunk_z,
            &JigsawBeardifier::default(),
        )
    }

    /// Resolves the target's stored references through typed source
    /// STRUCTURE_STARTS before entering NOISE. The provider must not treat an
    /// unavailable or lower-status source as an empty starts payload.
    #[cfg(test)]
    pub(crate) fn generate_fresh_carved_chunk_from_jigsaw_structure_references<E>(
        &self,
        target: &JigsawStructureChunkData,
        source_at: impl FnMut(i32, i32) -> Result<JigsawStructureChunkData, E>,
    ) -> SaveResult<NoiseGeneratedChunk>
    where
        E: std::fmt::Display,
    {
        self.generate_fresh_carved_chunk_from_jigsaw_structure_references_with_biome_blender(
            target, source_at, None,
        )
    }

    pub(crate) fn generate_fresh_carved_chunk_from_jigsaw_structure_references_borrowed<'a, E>(
        &self,
        target: &JigsawStructureChunkData,
        source_at: impl FnMut(i32, i32) -> Result<&'a JigsawStructureChunkData, E>,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<NoiseGeneratedChunk>
    where
        E: std::fmt::Display,
    {
        let chunk = target.chunk();
        let beardifier = self.beardifier_from_structure_references_borrowed(target, source_at)?;
        self.generate_fresh_carved_chunk_with_beardifier_and_biome_blender(
            chunk[0],
            chunk[1],
            &beardifier,
            biome_blender,
        )
    }

    pub(crate) fn beardifier_from_structure_references_borrowed<'a, E>(
        &self,
        target: &JigsawStructureChunkData,
        source_at: impl FnMut(i32, i32) -> Result<&'a JigsawStructureChunkData, E>,
    ) -> SaveResult<JigsawBeardifier>
    where
        E: std::fmt::Display,
    {
        let chunk = target.chunk();
        let path = format!(
            "noise_settings.{}.fresh_carvers.structure_references",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "terrain-adjusting references require a frozen structure plan",
            )
        })?;
        let referenced = target
            .resolve_referenced_terrain_starts_borrowed(plan, source_at)
            .map_err(|error| invalid_schema(&path, error.to_string()))?;
        Ok(JigsawBeardifier::from_ordered_terrain_starts(
            chunk[0],
            chunk[1],
            &referenced,
        ))
    }

    pub(crate) fn generate_fresh_carved_chunk_from_complete_jigsaw_source_window_borrowed<'a, E>(
        &self,
        target_starts: &JigsawStructureChunkData,
        mut source_at: impl FnMut(i32, i32) -> Result<&'a JigsawStructureChunkData, E>,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<(NoiseGeneratedChunk, JigsawStructureChunkData)>
    where
        E: std::fmt::Display,
    {
        let chunk = target_starts.chunk();
        let path = format!(
            "noise_settings.{}.fresh_carvers.structure_reference_window",
            self.source.settings_id()
        );
        let target_references =
            JigsawStructureChunkData::references_from_complete_source_window_borrowed(
                target_starts,
                &mut source_at,
            )
            .map_err(|error| invalid_schema(&path, error.to_string()))?;
        let generated = self
            .generate_fresh_carved_chunk_from_jigsaw_structure_references_borrowed(
                &target_references,
                source_at,
                biome_blender,
            )?;
        if generated.chunk_x() != chunk[0] || generated.chunk_z() != chunk[1] {
            return Err(invalid_schema(
                &path,
                "generated chunk does not match the target structure component",
            ));
        }
        Ok((generated, target_references))
    }

    #[cfg(test)]
    pub(crate) fn generate_fresh_carved_chunk_from_jigsaw_structure_references_with_biome_blender<
        E,
    >(
        &self,
        target: &JigsawStructureChunkData,
        source_at: impl FnMut(i32, i32) -> Result<JigsawStructureChunkData, E>,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<NoiseGeneratedChunk>
    where
        E: std::fmt::Display,
    {
        let chunk = target.chunk();
        let beardifier = self.beardifier_from_structure_references(target, source_at)?;
        self.generate_fresh_carved_chunk_with_beardifier_and_biome_blender(
            chunk[0],
            chunk[1],
            &beardifier,
            biome_blender,
        )
    }

    #[cfg(test)]
    pub(crate) fn beardifier_from_structure_references<E>(
        &self,
        target: &JigsawStructureChunkData,
        source_at: impl FnMut(i32, i32) -> Result<JigsawStructureChunkData, E>,
    ) -> SaveResult<JigsawBeardifier>
    where
        E: std::fmt::Display,
    {
        let chunk = target.chunk();
        let path = format!(
            "noise_settings.{}.fresh_carvers.structure_references",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "terrain-adjusting references require a frozen structure plan",
            )
        })?;
        let referenced = target
            .resolve_referenced_terrain_starts(plan, source_at)
            .map_err(|error| invalid_schema(&path, error.to_string()))?;
        Ok(JigsawBeardifier::from_ordered_terrain_starts(
            chunk[0],
            chunk[1],
            &referenced,
        ))
    }

    /// Requires the complete 17x17 STRUCTURE_STARTS dependency window, builds
    /// the target STRUCTURE_REFERENCES component, then enters Beardifier/NOISE.
    /// Status publication and source scheduling remain with the world owner
    /// (`ChunkPyramid.java:9-19`, `ChunkStatusTasks.java:67-73`).
    #[cfg(test)]
    pub(crate) fn generate_fresh_carved_chunk_from_complete_jigsaw_source_window<E>(
        &self,
        target_starts: &JigsawStructureChunkData,
        source_at: impl FnMut(i32, i32) -> Result<JigsawStructureChunkData, E>,
    ) -> SaveResult<(NoiseGeneratedChunk, JigsawStructureChunkData)>
    where
        E: std::fmt::Display,
    {
        self.generate_fresh_carved_chunk_from_complete_jigsaw_source_window_with_biome_blender(
            target_starts,
            source_at,
            None,
        )
    }

    #[cfg(test)]
    pub(crate) fn generate_fresh_carved_chunk_from_complete_jigsaw_source_window_with_biome_blender<
        E,
    >(
        &self,
        target_starts: &JigsawStructureChunkData,
        source_at: impl FnMut(i32, i32) -> Result<JigsawStructureChunkData, E>,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<(NoiseGeneratedChunk, JigsawStructureChunkData)>
    where
        E: std::fmt::Display,
    {
        let chunk = target_starts.chunk();
        let path = format!(
            "noise_settings.{}.fresh_carvers.structure_reference_window",
            self.source.settings_id()
        );
        let (target_references, sources) =
            JigsawStructureChunkData::references_from_complete_source_window(
                target_starts,
                source_at,
            )
            .map_err(|error| invalid_schema(&path, error.to_string()))?;
        let generated = self
            .generate_fresh_carved_chunk_from_jigsaw_structure_references_with_biome_blender(
                &target_references,
                |source_x, source_z| {
                    sources.get(&[source_x, source_z]).cloned().ok_or_else(|| {
                        format!(
                            "complete source window lost STRUCTURE_STARTS [{source_x}, {source_z}]"
                        )
                    })
                },
                biome_blender,
            )?;
        if generated.chunk_x() != chunk[0] || generated.chunk_z() != chunk[1] {
            return Err(invalid_schema(
                &path,
                "generated chunk does not match the target structure component",
            ));
        }
        Ok((generated, target_references))
    }

    pub(crate) fn fresh_column_state(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let noise_min_y = self.source.min_y().max(self.dimension_shape.min_y);
        let noise_max_y = self
            .source
            .min_y()
            .checked_add(self.source.height())?
            .min(self.dimension_shape.max_y_exclusive());
        if y < noise_min_y || y >= noise_max_y {
            return Some(self.source.air_block_state_id());
        }
        let noise_height = noise_max_y.checked_sub(noise_min_y)?;
        if noise_height <= 0
            || noise_min_y.rem_euclid(self.source.cell_height()) != 0
            || noise_height.rem_euclid(self.source.cell_height()) != 0
        {
            return None;
        }
        let mut aquifer = Aquifer::Disabled;
        let density = self.source.sample_interpolated(x, y, z);
        let decision = aquifer.compute(&self.source, x, y, z, density);
        Some(
            decision
                .state
                .unwrap_or(self.source.default_block_state_id()),
        )
    }
}
