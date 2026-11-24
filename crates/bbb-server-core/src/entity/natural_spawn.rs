mod eligibility;
mod existing;
pub mod mob_eligibility;
mod resolve;

pub use eligibility::{natural_spawn_pre_finalize_mob_facts, NaturalSpawnPreFinalizeMobFacts};
pub use existing::{natural_spawn_existing_facts, NaturalSpawnExistingFacts};
pub(crate) use resolve::finalize_and_publish_natural_spawn;
pub use resolve::{create_natural_spawn_root, NaturalSpawnPublishResult};

use std::collections::BTreeMap;

use bbb_protocol::entity_types::VanillaEntityMobCategory;
use bbb_protocol::packets::BlockPos;
use uuid::Uuid;

use crate::entity::natural_spawn_facts_26_1::{
    vanilla_mob_category_spawn_facts_26_1, VANILLA_MOB_CATEGORY_SPAWN_FACTS_26_1,
};

const SPAWN_CAP_AREA: i32 = 17 * 17;
const CATEGORY_COUNT: usize = VANILLA_MOB_CATEGORY_SPAWN_FACTS_26_1.len();

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NaturalSpawnCost {
    pub energy_budget: f64,
    pub charge: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct ExistingNaturalSpawnEntity<'a> {
    pub category: VanillaEntityMobCategory,
    pub position: BlockPos,
    pub is_mob: bool,
    pub persistence_required: bool,
    pub requires_custom_persistence: bool,
    pub cost_charge: Option<f64>,
    pub nearby_players: &'a [Uuid],
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PointCharge {
    position: BlockPos,
    charge: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NaturalSpawnPotential {
    charges: Vec<PointCharge>,
}

impl NaturalSpawnPotential {
    // `PotentialCalculator.java:10-28`.
    pub fn add_charge(&mut self, position: BlockPos, charge: f64) {
        if charge != 0.0 {
            self.charges.push(PointCharge { position, charge });
        }
    }

    pub fn energy_change(&self, position: BlockPos, charge: f64) -> f64 {
        if charge == 0.0 {
            return 0.0;
        }

        self.charges
            .iter()
            .map(|point| point.potential_change(position))
            .sum::<f64>()
            * charge
    }

    pub fn charge_count(&self) -> usize {
        self.charges.len()
    }
}

impl PointCharge {
    // `PotentialCalculator.java:34-42`; `Vec3i.java:201-220`.
    fn potential_change(self, position: BlockPos) -> f64 {
        let dx = f64::from(self.position.x) - f64::from(position.x);
        let dy = f64::from(self.position.y) - f64::from(position.y);
        let dz = f64::from(self.position.z) - f64::from(position.z);
        let distance_squared = dx * dx + dy * dy + dz * dz;
        if distance_squared == 0.0 {
            f64::INFINITY
        } else {
            self.charge / distance_squared.sqrt()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NaturalSpawnState {
    spawnable_chunk_count: i32,
    category_counts: [i32; CATEGORY_COUNT],
    local_category_counts: BTreeMap<Uuid, [i32; CATEGORY_COUNT]>,
    potential: NaturalSpawnPotential,
}

impl NaturalSpawnState {
    pub fn new(spawnable_chunk_count: i32) -> Self {
        Self {
            spawnable_chunk_count,
            category_counts: [0; CATEGORY_COUNT],
            local_category_counts: BTreeMap::new(),
            potential: NaturalSpawnPotential::default(),
        }
    }

    pub fn spawnable_chunk_count(&self) -> i32 {
        self.spawnable_chunk_count
    }

    pub fn category_count(&self, category: VanillaEntityMobCategory) -> i32 {
        self.category_counts[category as usize]
    }

    pub fn local_category_count(&self, player: Uuid, category: VanillaEntityMobCategory) -> i32 {
        self.local_category_counts
            .get(&player)
            .map_or(0, |counts| counts[category as usize])
    }

    pub fn potential(&self) -> &NaturalSpawnPotential {
        &self.potential
    }

    // `NaturalSpawner.java:67-98`.
    pub fn observe_existing(&mut self, entity: ExistingNaturalSpawnEntity<'_>) {
        if entity.is_mob && (entity.persistence_required || entity.requires_custom_persistence) {
            return;
        }
        if entity.category == VanillaEntityMobCategory::Misc {
            return;
        }

        if let Some(charge) = entity.cost_charge {
            self.potential.add_charge(entity.position, charge);
        }
        if entity.is_mob {
            self.add_local(entity.category, entity.nearby_players);
        }
        self.add_global(entity.category);
    }

    // `NaturalSpawner.java:104-118`.
    pub fn filtered_spawning_categories(
        &self,
        spawn_friendlies: bool,
        spawn_enemies: bool,
        spawn_persistent: bool,
    ) -> Vec<VanillaEntityMobCategory> {
        VANILLA_MOB_CATEGORY_SPAWN_FACTS_26_1
            .iter()
            .filter(|facts| facts.category != VanillaEntityMobCategory::Misc)
            .filter(|facts| spawn_friendlies || !facts.friendly)
            .filter(|facts| spawn_enemies || facts.friendly)
            .filter(|facts| spawn_persistent || !facts.persistent)
            .filter(|facts| self.can_spawn_for_category_global(facts.category))
            .map(|facts| facts.category)
            .collect()
    }

    // `NaturalSpawner.java:536-539`.
    pub fn can_spawn_for_category_global(&self, category: VanillaEntityMobCategory) -> bool {
        let facts = vanilla_mob_category_spawn_facts_26_1(category)
            .expect("the generated catalog covers every Vanilla mob category");
        let max_count = facts
            .max_instances_per_chunk
            .wrapping_mul(self.spawnable_chunk_count)
            / SPAWN_CAP_AREA;
        self.category_count(category) < max_count
    }

    // `LocalMobCapCalculator.java:27-52`.
    pub fn can_spawn_for_category_local(
        &self,
        category: VanillaEntityMobCategory,
        nearby_players: &[Uuid],
    ) -> bool {
        let max_count = vanilla_mob_category_spawn_facts_26_1(category)
            .expect("the generated catalog covers every Vanilla mob category")
            .max_instances_per_chunk;
        nearby_players.iter().any(|player| {
            self.local_category_counts
                .get(player)
                .is_none_or(|counts| counts[category as usize] < max_count)
        })
    }

    pub fn can_spawn(&self, position: BlockPos, cost: Option<NaturalSpawnCost>) -> bool {
        cost.is_none_or(|cost| {
            self.potential.energy_change(position, cost.charge) <= cost.energy_budget
        })
    }

    pub fn after_spawn(
        &mut self,
        category: VanillaEntityMobCategory,
        position: BlockPos,
        cost: Option<NaturalSpawnCost>,
        nearby_players: &[Uuid],
    ) {
        let charge = cost.map_or(0.0, |cost| cost.charge);
        self.potential.add_charge(position, charge);
        self.add_global(category);
        self.add_local(category, nearby_players);
    }

    fn add_global(&mut self, category: VanillaEntityMobCategory) {
        let count = &mut self.category_counts[category as usize];
        *count = count.wrapping_add(1);
    }

    fn add_local(&mut self, category: VanillaEntityMobCategory, nearby_players: &[Uuid]) {
        let index = category as usize;
        for player in nearby_players {
            let count = &mut self
                .local_category_counts
                .entry(*player)
                .or_insert([0; CATEGORY_COUNT])[index];
            *count = count.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests;
