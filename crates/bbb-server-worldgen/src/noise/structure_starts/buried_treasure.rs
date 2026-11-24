use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct BuriedTreasureStarts(pub(super) Vec<BuriedTreasureStructureStart>);

impl BuriedTreasureStarts {
    pub(super) fn is_none(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &BuriedTreasureStructureStart> {
        self.0.iter()
    }

    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut BuriedTreasureStructureStart> {
        self.0.iter_mut()
    }

    pub(super) fn as_slice(&self) -> &[BuriedTreasureStructureStart] {
        &self.0
    }

    pub(super) fn push(&mut self, start: BuriedTreasureStructureStart) {
        self.0.push(start);
    }

    pub(super) fn sort(&mut self) {
        self.0
            .sort_by(|left, right| left.structure_id().cmp(right.structure_id()));
    }

    pub(super) fn retain(&mut self, predicate: impl FnMut(&BuriedTreasureStructureStart) -> bool) {
        self.0.retain(predicate);
    }
}

impl NoiseStructureChunkData {
    pub fn buried_treasure_starts(&self) -> &[BuriedTreasureStructureStart] {
        self.buried_treasure_start.as_slice()
    }

    pub fn buried_treasure_start(&self) -> Option<&BuriedTreasureStructureStart> {
        self.buried_treasure_start
            .iter()
            .find(|start| start.structure_id() == BURIED_TREASURE_ID)
    }

    pub fn with_buried_treasure_start(self, start: BuriedTreasureStructureStart) -> Self {
        self.with_buried_treasure_starts([start])
    }

    pub fn with_buried_treasure_starts(
        mut self,
        starts: impl IntoIterator<Item = BuriedTreasureStructureStart>,
    ) -> Self {
        insert_starts(
            &mut self.buried_treasure_start.0,
            starts,
            BuriedTreasureStructureStart::structure_id,
        );
        self
    }
}
