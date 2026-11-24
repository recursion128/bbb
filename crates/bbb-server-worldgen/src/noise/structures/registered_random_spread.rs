use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnownedStructureStartCandidate {
    pub set_id: String,
    pub attempts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SelectedAttempt {
    set_id: String,
    structure_id: String,
    level_seed: i64,
    source_x: i32,
    source_z: i32,
}

pub(super) fn owner(plan: &NoiseStructurePlan, set: &NoiseStructureSet) -> StructureStartSetOwner {
    if matches!(set.placement, NoiseStructurePlacement::RandomSpread { .. })
        && set.structures.len() > 1
        && set.structures.iter().all(|selection| {
            single_selection_owner(plan, set, selection)
                .is_some_and(|owner| owner != StructureStartSetOwner::Unowned)
        })
    {
        // StructureSet retains an ordered heterogeneous Holder<Structure>
        // list; type dispatch happens only after weighted selection
        // (`StructureSet.java:12-41`, `ChunkGenerator.java:464-588`).
        StructureStartSetOwner::RegisteredRandomSpread
    } else {
        StructureStartSetOwner::Unowned
    }
}

fn single_selection_owner(
    plan: &NoiseStructurePlan,
    set: &NoiseStructureSet,
    selection: &NoiseStructureSelection,
) -> Option<StructureStartSetOwner> {
    set.structures
        .iter()
        .any(|candidate| candidate.structure == selection.structure)
        .then(|| NoiseStructureSet {
            id: set.id.clone(),
            structures: vec![selection.clone()],
            placement: set.placement.clone(),
        })
        .map(|single| plan.structure_start_owner(&single))
}

impl NoiseStructurePlan {
    pub fn single_selection_owner(
        &self,
        set: &NoiseStructureSet,
        selection: &NoiseStructureSelection,
    ) -> Option<StructureStartSetOwner> {
        single_selection_owner(self, set, selection)
    }

    pub fn projected_set(&self, set_id: &str) -> Option<Self> {
        let set = self.set(set_id)?.clone();
        Some(Self {
            possible_biomes: std::sync::Arc::clone(&self.possible_biomes),
            pools: std::sync::Arc::clone(&self.pools),
            structures: std::sync::Arc::clone(&self.structures),
            structure_order: std::sync::Arc::clone(&self.structure_order),
            sets: vec![set],
            set_index: BTreeMap::from([(set_id.to_string(), 0)]),
            registered_sets: std::sync::Arc::clone(&self.registered_sets),
            registered_selected_attempt: None,
        })
    }

    pub fn projected_selection(
        &self,
        set_id: &str,
        structure_id: &str,
        level_seed: i64,
        source_x: i32,
        source_z: i32,
    ) -> Option<Self> {
        let mut projected = self.projected_set(set_id)?;
        let set = projected.sets.first_mut()?;
        let selection = set
            .structures
            .iter()
            .find(|selection| selection.structure == structure_id)?
            .clone();
        set.structures = vec![selection];
        projected.registered_selected_attempt = Some(SelectedAttempt {
            set_id: set_id.to_string(),
            structure_id: structure_id.to_string(),
            level_seed,
            source_x,
            source_z,
        });
        Some(projected)
    }

    /// The candidate chunk of `set_id` nearest the origin, for tests that need
    /// a chunk where the set attempts a start.
    #[cfg(test)]
    pub(crate) fn first_candidate(&self, set_id: &str, level_seed: i64) -> [i32; 2] {
        const RADIUS: i32 = 64;
        (0..=RADIUS)
            .flat_map(|r| (-r..=r).flat_map(move |x| (-r..=r).map(move |z| (x, z))))
            .find(|&(x, z)| {
                self.random_spread_attempt_order(set_id, level_seed, x, z)
                    .is_some_and(|attempts| !attempts.is_empty())
            })
            .map(|(x, z)| [x, z])
            .unwrap_or_else(|| panic!("no {set_id} candidate within {RADIUS} chunks"))
    }

    /// Candidate gating and weighted retry order from
    /// `ChunkGenerator.createStructures` (`ChunkGenerator.java:464-551`).
    pub fn random_spread_attempt_order(
        &self,
        set_id: &str,
        level_seed: i64,
        source_x: i32,
        source_z: i32,
    ) -> Option<Vec<String>> {
        if let Some(selected) = &self.registered_selected_attempt {
            return (selected.set_id == set_id
                && selected.level_seed == level_seed
                && selected.source_x == source_x
                && selected.source_z == source_z)
                .then(|| vec![selected.structure_id.clone()]);
        }
        let candidate =
            self.is_random_spread_structure_chunk(set_id, level_seed, source_x, source_z)?;
        if !candidate {
            return Some(Vec::new());
        }
        self.weighted_attempt_order(set_id, level_seed, source_x, source_z)
    }
}
