use super::*;

impl NoiseStructureChunkData {
    pub fn fortress_starts(&self) -> &[FortressStructureStart] {
        &self.fortress_starts
    }

    pub fn nether_fossil_starts(&self) -> &[NetherFossilStructureStart] {
        &self.nether_fossil_starts
    }

    pub fn with_fortress_starts(
        mut self,
        starts: impl IntoIterator<Item = FortressStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.fortress_starts,
            starts,
            FortressStructureStart::structure_id,
        );
        self
    }

    pub fn with_nether_fossil_starts(
        mut self,
        starts: impl IntoIterator<Item = NetherFossilStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.nether_fossil_starts,
            starts,
            NetherFossilStructureStart::structure_id,
        );
        self
    }

    pub fn is_empty(&self) -> bool {
        self.starts.is_empty()
            && self.buried_treasure_start.is_none()
            && self.mineshaft_starts.is_empty()
            && self.shipwreck_starts.is_empty()
            && self.ocean_ruin_starts.is_empty()
            && self.igloo_starts.is_empty()
            && self.scattered_starts.is_empty()
            && self.ruined_portal_starts.is_empty()
            && self.ocean_monument_starts.is_empty()
            && self.woodland_mansion_starts.is_empty()
            && self.stronghold_starts.is_empty()
            && self.fortress_starts.is_empty()
            && self.nether_fossil_starts.is_empty()
            && self.end_city_starts.is_empty()
            && self.restored_bounds.is_empty()
            && self.references.values().all(Vec::is_empty)
    }
}
