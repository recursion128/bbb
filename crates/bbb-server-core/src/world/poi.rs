use super::ServerWorld;
use crate::chunk::poi::{PoiOccupancy, PoiRecord};
use crate::world::LevelId;
use bbb_server_types::save::Java26LevelKey;

impl ServerWorld {
    pub fn poi_record(&mut self, dimension: &Java26LevelKey, pos: [i32; 3]) -> Option<PoiRecord> {
        let dimension = self.level_id(dimension)?;
        self.levels
            .get_mut(&dimension)?
            .chunks
            .poi_mut()
            .record(pos)
            .cloned()
    }

    pub fn acquire_poi_ticket(&mut self, dimension: &Java26LevelKey, pos: [i32; 3]) -> bool {
        let Some(dimension) = self.level_id(dimension) else {
            return false;
        };
        self.levels
            .get_mut(&dimension)
            .is_some_and(|level| level.chunks.poi_mut().acquire_ticket(pos))
    }

    pub fn release_poi_ticket(
        &mut self,
        dimension: &Java26LevelKey,
        pos: [i32; 3],
    ) -> Option<bool> {
        let dimension = self.level_id(dimension)?;
        self.levels
            .get_mut(&dimension)?
            .chunks
            .poi_mut()
            .release_ticket(pos)
    }

    pub fn poi_in_range(
        &mut self,
        dimension: &Java26LevelKey,
        center: [i32; 3],
        radius: i32,
        occupancy: PoiOccupancy,
        predicate: impl Fn(&PoiRecord) -> bool,
    ) -> Option<Vec<PoiRecord>> {
        self.poi_in_range_id(
            &self.level_id(dimension)?,
            center,
            radius,
            occupancy,
            predicate,
        )
    }
    pub(super) fn poi_in_range_id(
        &mut self,
        dimension: &LevelId,
        center: [i32; 3],
        radius: i32,
        occupancy: PoiOccupancy,
        predicate: impl Fn(&PoiRecord) -> bool,
    ) -> Option<Vec<PoiRecord>> {
        Some(
            self.levels
                .get_mut(dimension)?
                .chunks
                .poi_mut()
                .records_in_range(center, radius, occupancy, predicate),
        )
    }

    pub fn sections_to_village(
        &self,
        dimension: &Java26LevelKey,
        section: [i32; 3],
    ) -> Option<i32> {
        self.sections_to_village_id(&self.level_id(dimension)?, section)
    }
    pub fn is_close_to_village(
        &self,
        dimension: &Java26LevelKey,
        pos: [i32; 3],
        distance: i32,
    ) -> Option<bool> {
        self.is_close_to_village_id(&self.level_id(dimension)?, pos, distance)
    }
    fn sections_to_village_id(&self, dimension: &LevelId, section: [i32; 3]) -> Option<i32> {
        self.levels
            .get(dimension)?
            .chunks
            .sections_to_village(section)
    }

    pub(super) fn is_close_to_village_id(
        &self,
        dimension: &LevelId,
        pos: [i32; 3],
        distance: i32,
    ) -> Option<bool> {
        // ServerLevel.java:1543-1544: distances above six never count as close.
        Some(
            distance <= 6
                && self.sections_to_village_id(dimension, pos.map(|value| value >> 4))? <= distance,
        )
    }
}
