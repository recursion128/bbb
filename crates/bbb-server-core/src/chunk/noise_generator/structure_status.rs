//! Selective structure-status tasks for the currently owned start families.

use std::collections::{BTreeMap, BTreeSet};

use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
use bbb_server_worldgen::noise::structures::{NoiseStructurePlacement, NoiseStructurePlan};

use super::{NoiseFinalDensityProgram, SaveResult};
use bbb_server_types::schema_support::invalid_schema;

pub type NoiseStructureRingPositions = BTreeMap<String, BTreeSet<[i32; 2]>>;

impl NoiseFinalDensityProgram {
    pub(crate) fn structure_generation_context(
        &self,
    ) -> SaveResult<crate::chunk::structure_generation::StructureGenerationContext<'_>> {
        let path = format!(
            "noise_settings.{}.structure_status",
            self.source.settings_id()
        );
        let startup = self
            .structure_startup()
            .ok_or_else(|| invalid_schema(&path, "structure starts require frozen resources"))?;
        startup
            .registries
            .registry("minecraft:worldgen/biome")
            .ok_or_else(|| invalid_schema(&path, "the frozen biome registry is absent"))?;
        let plan = self
            .structures
            .as_ref()
            .ok_or_else(|| invalid_schema(&path, "structure starts require a frozen plan"))?;
        let biomes = self
            .biome_source
            .as_ref()
            .ok_or_else(|| invalid_schema(&path, "structure starts require a biome source"))?;
        Ok(
            crate::chunk::structure_generation::StructureGenerationContext {
                item_publication: self.item_publication(),
                startup,
                plan,
                world_seed: self.world_seed,
                dimension_shape: self.dimension_shape,
                label: self.source.settings_id(),
                sea_level: self.source.sea_level(),
                terrain: crate::chunk::structure_generation::StructureTerrain::Noise(self),
                biome_search: std::cell::RefCell::new(Some(biomes.create_search())),
            },
        )
    }

    pub(crate) fn structure_biome_at_quart(
        &self,
        x: i32,
        y: i32,
        z: i32,
        search: &mut bbb_server_worldgen::noise::biome::NoiseBiomeSearch,
    ) -> Option<i32> {
        Some(
            self.biome_source
                .as_ref()?
                .sample(&self.source, x, y, z, search),
        )
    }

    pub(crate) const fn structure_world_seed(&self) -> i64 {
        self.world_seed
    }

    pub(crate) fn natural_spawn_structure_plan(&self) -> Option<&NoiseStructurePlan> {
        self.structures.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn disable_natural_spawn_structures_for_test(&mut self) {
        self.structures = None;
    }

    /// Freezes every concentric-ring source once for the world-level status
    /// cache (`ChunkGeneratorStructureState.java:107-160`).
    pub fn structure_start_ring_positions(&self) -> SaveResult<NoiseStructureRingPositions> {
        if let Some(rings) = self.structure_rings.get() {
            return Ok(rings.clone());
        }
        let rings = self.compute_structure_start_ring_positions()?;
        Ok(self.structure_rings.get_or_init(|| rings).clone())
    }

    pub fn compute_structure_start_ring_positions(
        &self,
    ) -> SaveResult<NoiseStructureRingPositions> {
        let path = format!(
            "noise_settings.{}.structure_status.rings",
            self.source.settings_id()
        );

        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "structure status requires a frozen noise structure plan",
            )
        })?;
        let mut rings = BTreeMap::new();
        for set in plan.sets() {
            if !matches!(
                set.placement,
                NoiseStructurePlacement::ConcentricRings { .. }
            ) {
                continue;
            }
            let positions = self.structure_ring_positions_component(&set.id)?;
            rings.insert(
                set.id.clone(),
                positions
                    .into_iter()
                    .map(|(x, z)| [x, z])
                    .collect::<BTreeSet<_>>(),
            );
        }
        Ok(rings)
    }

    /// Completes the selective STRUCTURE_STARTS task only when every frozen
    /// set is either non-candidate or handled by a typed owner. The caller
    /// publishes status only after this returns (`ChunkStatusTasks.java:40-57`).
    pub fn generate_structure_starts_status_task(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        generate_structures: bool,
        rings: &NoiseStructureRingPositions,
    ) -> SaveResult<NoiseStructureChunkData> {
        if !generate_structures {
            return Ok(NoiseStructureChunkData::from_supplied_starts(
                chunk_x,
                chunk_z,
                [],
                [],
            ));
        }
        self.preflight_structure_starts_status_task(chunk_x, chunk_z)?;
        self.generate_registered_structure_starts_status_task(chunk_x, chunk_z, rings)
    }

    pub fn preflight_structure_starts_status_task(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<()> {
        let path = format!(
            "noise_settings.{}.structure_status.starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(&path, "structure status requires the frozen structure plan")
        })?;
        if let Some(candidate) =
            plan.unowned_random_spread_candidate(self.world_seed, chunk_x, chunk_z)
        {
            return Err(invalid_schema(
                &path,
                format!(
                    "set {} selected {:?}, but no typed start owner is available",
                    candidate.set_id, candidate.attempts
                ),
            ));
        }
        Ok(())
    }
}
