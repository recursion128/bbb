//! Shared typed structure set order and dispatch.

use super::*;

impl StructureGenerationContext<'_> {
    pub(crate) fn generate_registered_structure_starts_status_task(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        rings: &NoiseStructureRingPositions,
    ) -> SaveResult<NoiseStructureChunkData> {
        let path = format!(
            "noise_settings.{}.structure_status.starts[{chunk_x},{chunk_z}]",
            self.label
        );
        let plan = self.plan;
        self.preflight_registered_structure_start_owners(plan, &path)?;
        let mut starts = NoiseStructureChunkData::from_supplied_starts(chunk_x, chunk_z, [], []);
        // `ChunkGenerator.createStructures` applies every placement before the
        // same per-set weighted retry (`ChunkGenerator.java:476-548`).
        for set in plan.sets() {
            if set
                .structures
                .iter()
                .any(|selection| starts.has_valid_start(&selection.structure))
            {
                continue;
            }
            if matches!(
                set.placement,
                NoiseStructurePlacement::ConcentricRings { .. }
            ) && !rings.contains_key(&set.id)
            {
                return Err(invalid_schema(
                    &path,
                    format!("concentric-ring set {} has no frozen positions", set.id),
                ));
            }
            if !plan
                .is_structure_chunk(&set.id, self.world_seed, chunk_x, chunk_z, rings)
                .ok_or_else(|| {
                    invalid_schema(&path, "structure placement dependency is unavailable")
                })?
            {
                continue;
            }
            let attempts = plan
                .weighted_attempt_order(&set.id, self.world_seed, chunk_x, chunk_z)
                .ok_or_else(|| {
                    invalid_schema(&path, "registered set lost its weighted selections")
                })?;
            for structure_id in attempts {
                let projected = plan
                    .projected_selection(&set.id, &structure_id, self.world_seed, chunk_x, chunk_z)
                    .ok_or_else(|| invalid_schema(&path, "selected structure is absent"))?;
                let projected_set = projected
                    .set(&set.id)
                    .ok_or_else(|| invalid_schema(&path, "projected set is absent"))?;
                let selected_owner = projected.structure_start_owner(projected_set);
                let candidate = self.dispatch_registered_structure_start_set(
                    selected_owner,
                    &projected,
                    starts.clone(),
                    [chunk_x, chunk_z],
                    rings,
                    &path,
                )?;
                if candidate.has_valid_start(&structure_id) {
                    starts = candidate;
                    break;
                }
            }
        }
        Ok(starts)
    }

    pub(crate) fn preflight_registered_structure_start_owners(
        &self,
        plan: &NoiseStructurePlan,
        path: &str,
    ) -> SaveResult<()> {
        for set in plan.sets() {
            let mut ids = BTreeSet::new();
            if set
                .structures
                .iter()
                .any(|selection| !ids.insert(selection.structure.as_str()))
            {
                return Err(invalid_schema(
                    path,
                    format!("set {} repeats a structure holder", set.id),
                ));
            }
            for selection in &set.structures {
                plan.single_selection_owner(set, selection)
                    .filter(|owner| {
                        !matches!(
                            owner,
                            StructureStartSetOwner::RegisteredRandomSpread
                                | StructureStartSetOwner::Unowned
                        )
                    })
                    .ok_or_else(|| {
                        invalid_schema(
                            path,
                            format!(
                                "set {} entry {} lost its typed owner",
                                set.id, selection.structure
                            ),
                        )
                    })?;
            }
        }
        Ok(())
    }

    pub(crate) fn dispatch_registered_structure_start_set(
        &self,
        owner: StructureStartSetOwner,
        plan: &NoiseStructurePlan,
        starts: NoiseStructureChunkData,
        [chunk_x, chunk_z]: [i32; 2],
        rings: &NoiseStructureRingPositions,
        path: &str,
    ) -> SaveResult<NoiseStructureChunkData> {
        Ok(match owner {
            StructureStartSetOwner::Jigsaw => {
                let generated = self.generate_jigsaw_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_jigsaw_starts(generated.starts().iter().cloned())
            }
            StructureStartSetOwner::BuriedTreasure => {
                let generated = self.generate_buried_treasure_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_buried_treasure_starts(generated)
            }
            StructureStartSetOwner::Mineshaft => {
                let generated = self.generate_mineshaft_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_mineshaft_starts(generated)
            }
            StructureStartSetOwner::Shipwreck => {
                let generated = self.generate_shipwreck_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_shipwreck_starts(generated)
            }
            StructureStartSetOwner::OceanRuin => {
                let generated = self.generate_ocean_ruin_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_ocean_ruin_starts(generated)
            }
            StructureStartSetOwner::Igloo => {
                let generated = self.generate_igloo_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_igloo_starts(generated)
            }
            StructureStartSetOwner::DesertPyramid
            | StructureStartSetOwner::JungleTemple
            | StructureStartSetOwner::SwampHut => {
                let generated = self
                    .generate_scattered_feature_structure_starts_for_plan_component(
                        plan,
                        chunk_x,
                        chunk_z,
                        &[],
                    );
                starts.with_scattered_starts(generated)
            }
            StructureStartSetOwner::RuinedPortal => {
                let generated = self.generate_ruined_portal_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_ruined_portal_starts(generated)
            }
            StructureStartSetOwner::OceanMonument => {
                let generated = self.generate_ocean_monument_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_ocean_monument_starts(generated)
            }
            StructureStartSetOwner::WoodlandMansion => {
                let generated = self.generate_woodland_mansion_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_woodland_mansion_starts(generated)
            }
            StructureStartSetOwner::Stronghold => {
                let generated = self.generate_stronghold_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                    rings,
                );
                starts.with_stronghold_starts(generated)
            }
            StructureStartSetOwner::NetherComplex => {
                let (jigsaw, fortress) = self
                    .generate_nether_complex_structure_starts_for_plan_component(
                        plan,
                        chunk_x,
                        chunk_z,
                        &[],
                        &[],
                    );
                starts
                    .with_jigsaw_starts(jigsaw)
                    .with_fortress_starts(fortress)
            }
            StructureStartSetOwner::NetherFossil => {
                let generated = self.generate_nether_fossil_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_nether_fossil_starts(generated)
            }
            StructureStartSetOwner::EndCity => {
                let generated = self.generate_end_city_structure_starts_for_plan_component(
                    plan,
                    chunk_x,
                    chunk_z,
                    &[],
                );
                starts.with_end_city_starts(generated)
            }
            StructureStartSetOwner::RegisteredRandomSpread | StructureStartSetOwner::Unowned => {
                return Err(invalid_schema(
                    path,
                    "registered start dispatch lost its typed owner",
                ));
            }
        })
    }
}
