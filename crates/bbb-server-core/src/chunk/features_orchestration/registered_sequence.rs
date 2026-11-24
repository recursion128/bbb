//! Ordered FEATURES placement for every registered typed structure owner.

use bbb_server_worldgen::noise::structures::NoiseStructureDecorationInvocation;

use super::*;
use crate::chunk::noise_generator::features::FeatureWorldGenRegion;
use crate::chunk::structure_generation::StructureGenerationContext;
use crate::chunk::structure_status::NoiseStructureStatusCache;
use bbb_server_worldgen::noise::buried_treasure::BuriedTreasureStructureStart;
use bbb_server_worldgen::noise::end_city::EndCityStructureStart;
use bbb_server_worldgen::noise::igloo::IglooStructureStart;
use bbb_server_worldgen::noise::mineshaft::MineshaftStructureStart;
use bbb_server_worldgen::noise::nether_fortress::FortressStructureStart;
use bbb_server_worldgen::noise::nether_fossil::NetherFossilStructureStart;
use bbb_server_worldgen::noise::ocean_monument::OceanMonumentStructureStart;
use bbb_server_worldgen::noise::ocean_ruin::OceanRuinStructureStart;
use bbb_server_worldgen::noise::ruined_portal::RuinedPortalStructureStart;
use bbb_server_worldgen::noise::scattered::ScatteredFeatureStructureStart;
use bbb_server_worldgen::noise::shipwreck::ShipwreckStructureStart;
use bbb_server_worldgen::noise::stronghold::StrongholdStructureStart;
use bbb_server_worldgen::noise::structure_starts::JigsawStructureStart;
use bbb_server_worldgen::noise::woodland_mansion::WoodlandMansionStructureStart;

use crate::chunk::noise_generator::features::StructurePlacementFamily as RegisteredStructureFamily;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RegisteredStructureProbe {
    family: RegisteredStructureFamily,
    invocation: NoiseStructureDecorationInvocation,
}

impl RegisteredStructureProbe {
    fn invocation(&self) -> &NoiseStructureDecorationInvocation {
        &self.invocation
    }

    fn into_placement(self) -> RegisteredStructurePlacement {
        RegisteredStructurePlacement {
            family: self.family,
            invocation: self.invocation,
        }
    }
}

fn prepare_registered_probe(
    program: &StructureGenerationContext<'_>,
    target: [i32; 2],
    structure_id: &str,
) -> SaveResult<RegisteredStructureProbe> {
    let family = program.placement_family(structure_id).ok_or_else(|| {
        invalid_carvers_region(
            target,
            format!("referenced structure {structure_id} has no FEATURES owner"),
        )
    })?;
    Ok(RegisteredStructureProbe {
        family,
        invocation: program.structure_invocation(target, structure_id, family)?,
    })
}

fn prepare_registered_probes(
    cache: &NoiseStructureStatusCache,
    program: &StructureGenerationContext<'_>,
    target: [i32; 2],
) -> SaveResult<Vec<RegisteredStructureProbe>> {
    cache
        .target_reference_ids(target)?
        .iter()
        .map(|id| prepare_registered_probe(program, target, id))
        .collect()
}

#[derive(Default)]
pub(crate) struct RegisteredStructureSteps {
    by_step: BTreeMap<usize, Vec<RegisteredStructurePlacement>>,
}

impl RegisteredStructureSteps {
    fn from_probes(probes: Vec<RegisteredStructureProbe>) -> Self {
        let mut steps = Self::default();
        for probe in probes {
            steps
                .by_step
                .entry(probe.invocation().step)
                .or_default()
                .push(probe.into_placement());
        }
        for placements in steps.by_step.values_mut() {
            placements.sort_by_key(RegisteredStructurePlacement::step_index);
        }
        steps
    }

    #[cfg(test)]
    fn invocations(&self) -> Vec<NoiseStructureDecorationInvocation> {
        self.by_step
            .values()
            .flatten()
            .map(|placement| placement.invocation.clone())
            .collect()
    }
}

#[cfg(test)]
pub(super) fn registered_invocations_for_test(
    cache: &NoiseStructureStatusCache,
    program: &NoiseFinalDensityProgram,
    target: [i32; 2],
) -> SaveResult<Vec<NoiseStructureDecorationInvocation>> {
    Ok(
        RegisteredStructureSteps::from_probes(prepare_registered_probes(
            cache,
            &program.structure_generation_context()?,
            target,
        )?)
        .invocations(),
    )
}

#[cfg(test)]
pub(super) fn registered_owner_for_test(
    program: &NoiseFinalDensityProgram,
    structure_id: &str,
) -> bool {
    program
        .structure_generation_context()
        .is_ok_and(|context| context.placement_family(structure_id).is_some())
}

struct RegisteredStructurePlacement {
    family: RegisteredStructureFamily,
    invocation: NoiseStructureDecorationInvocation,
}

impl RegisteredStructurePlacement {
    fn step_index(&self) -> usize {
        self.invocation.step_index
    }

    fn place(
        self,
        program: &StructureGenerationContext<'_>,
        cache: &mut NoiseStructureStatusCache,
        region: &mut FeatureWorldGenRegion<'_>,
    ) -> SaveResult<()> {
        match self.family {
            RegisteredStructureFamily::Mineshaft => program
                .place_referenced_structure::<MineshaftStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::BuriedTreasure => program
                .place_referenced_structure::<BuriedTreasureStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::Shipwreck => program
                .place_referenced_structure::<ShipwreckStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::OceanRuin => program
                .place_referenced_structure::<OceanRuinStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::Igloo => program
                .place_referenced_structure::<IglooStructureStart>(cache, &self.invocation, region),
            RegisteredStructureFamily::EndCity => program
                .place_referenced_structure::<EndCityStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::WoodlandMansion => program
                .place_referenced_structure::<WoodlandMansionStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::RuinedPortal => program
                .place_referenced_structure::<RuinedPortalStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::SwampHut => program
                .place_referenced_structure::<ScatteredFeatureStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::DesertPyramid | RegisteredStructureFamily::JungleTemple => {
                program.place_referenced_structure::<ScatteredFeatureStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                )
            }
            RegisteredStructureFamily::OceanMonument => program
                .place_referenced_structure::<OceanMonumentStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::Stronghold => program
                .place_referenced_structure::<StrongholdStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::Jigsaw => program
                .place_referenced_structure::<JigsawStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::Fortress => program
                .place_referenced_structure::<FortressStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
            RegisteredStructureFamily::NetherFossil => program
                .place_referenced_structure::<NetherFossilStructureStart>(
                    cache,
                    &self.invocation,
                    region,
                ),
        }
    }
}

type PreparedRegisteredStructures = (
    BTreeMap<(i32, i32), RegisteredStructureSteps>,
    BTreeSet<(i32, i32)>,
);

pub(super) fn prepare_registered_structure_targets(
    store: &mut ChunkStore,
    program: &NoiseFinalDensityProgram,
    targets: &BTreeSet<(i32, i32)>,
    loading_starts_only: bool,
) -> SaveResult<PreparedRegisteredStructures> {
    if !store.structure_status.generate_structures() {
        return Ok((BTreeMap::new(), BTreeSet::new()));
    }
    let mut sources = BTreeSet::new();
    for &(x, z) in targets {
        sources.extend(
            store
                .structure_status
                .target_reference_source_chunks([x, z])?
                .into_iter()
                .map(|[x, z]| (x, z)),
        );
    }
    for &(x, z) in &sources {
        if loading_starts_only {
            store.ensure_structure_starts_column(program, [x, z])?;
        } else {
            store.try_load_column(x, z)?;
        }
    }
    let context = program.structure_generation_context()?;
    let mut steps = BTreeMap::new();
    for &(x, z) in targets {
        steps.insert(
            (x, z),
            RegisteredStructureSteps::from_probes(prepare_registered_probes(
                &store.structure_status,
                &context,
                [x, z],
            )?),
        );
    }
    Ok((steps, sources))
}

impl RegisteredStructureSteps {
    pub(crate) fn place_step(
        &mut self,
        program: &StructureGenerationContext<'_>,
        cache: &mut NoiseStructureStatusCache,
        step: usize,
        region: &mut FeatureWorldGenRegion<'_>,
    ) -> SaveResult<()> {
        for placement in self.by_step.remove(&step).unwrap_or_default() {
            placement.place(program, cache, region)?;
        }
        Ok(())
    }
}

pub(crate) fn prepare_flat_structure_steps(
    program: &StructureGenerationContext<'_>,
    cache: &NoiseStructureStatusCache,
    target: [i32; 2],
) -> SaveResult<RegisteredStructureSteps> {
    if cache.generate_structures() {
        Ok(RegisteredStructureSteps::from_probes(
            prepare_registered_probes(cache, program, target)?,
        ))
    } else {
        Ok(RegisteredStructureSteps::default())
    }
}
