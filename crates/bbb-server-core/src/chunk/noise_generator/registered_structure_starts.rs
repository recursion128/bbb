//! Registry-ordered STRUCTURE_STARTS dispatch across existing typed owners.

use super::*;

impl NoiseFinalDensityProgram {
    /// Generates biome-gated stronghold starts without publishing the status.
    pub fn generate_stronghold_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[StrongholdStructureStart],
        ring_positions: &NoiseStructureRingPositions,
    ) -> SaveResult<Vec<StrongholdStructureStart>> {
        let path = format!(
            "noise_settings.{}.stronghold_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(&path, "stronghold starts require the frozen structure plan")
        })?;
        self.generate_stronghold_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
            ring_positions,
        )
    }

    pub(super) fn generate_stronghold_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[StrongholdStructureStart],
        ring_positions: &NoiseStructureRingPositions,
    ) -> SaveResult<Vec<StrongholdStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_stronghold_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
                ring_positions,
            ))
    }

    /// Generates leftover scattered-feature starts without publishing the status.
    pub fn generate_scattered_feature_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ScatteredFeatureStructureStart],
    ) -> SaveResult<Vec<ScatteredFeatureStructureStart>> {
        let path = format!(
            "noise_settings.{}.scattered_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "scattered-feature starts require the built-in Overworld structure plan",
            )
        })?;
        self.generate_scattered_feature_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_scattered_feature_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ScatteredFeatureStructureStart],
    ) -> SaveResult<Vec<ScatteredFeatureStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_scattered_feature_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    /// Generates biome-gated ruined-portal starts without publishing the status.
    pub fn generate_ruined_portal_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[RuinedPortalStructureStart],
    ) -> SaveResult<Vec<RuinedPortalStructureStart>> {
        let path = format!(
            "noise_settings.{}.ruined_portal_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "ruined-portal starts require a built-in noise structure plan",
            )
        })?;
        self.generate_ruined_portal_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_ruined_portal_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[RuinedPortalStructureStart],
    ) -> SaveResult<Vec<RuinedPortalStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_ruined_portal_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    /// Generates biome-gated ocean-monument starts without publishing the status.
    pub fn generate_ocean_monument_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanMonumentStructureStart],
    ) -> SaveResult<Vec<OceanMonumentStructureStart>> {
        let path = format!(
            "noise_settings.{}.ocean_monument_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "ocean-monument starts require the frozen Overworld structure plan",
            )
        })?;
        self.generate_ocean_monument_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_ocean_monument_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanMonumentStructureStart],
    ) -> SaveResult<Vec<OceanMonumentStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_ocean_monument_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    /// Generates biome-gated woodland-mansion starts without publishing the status.
    pub fn generate_woodland_mansion_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[WoodlandMansionStructureStart],
    ) -> SaveResult<Vec<WoodlandMansionStructureStart>> {
        let path = format!(
            "noise_settings.{}.woodland_mansion_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "woodland-mansion starts require the built-in Overworld structure plan",
            )
        })?;
        self.generate_woodland_mansion_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_woodland_mansion_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[WoodlandMansionStructureStart],
    ) -> SaveResult<Vec<WoodlandMansionStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_woodland_mansion_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    pub(super) fn generate_registered_structure_starts_status_task(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        rings: &NoiseStructureRingPositions,
    ) -> SaveResult<NoiseStructureChunkData> {
        self.structure_generation_context()?
            .generate_registered_structure_starts_status_task(chunk_x, chunk_z, rings)
    }

    pub(super) fn preflight_registered_structure_start_owners(
        &self,
        plan: &NoiseStructurePlan,
        path: &str,
    ) -> SaveResult<()> {
        self.structure_generation_context()?
            .preflight_registered_structure_start_owners(plan, path)
    }
}
