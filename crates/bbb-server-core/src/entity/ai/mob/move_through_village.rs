//! `MoveThroughVillageGoal`: zombies walk toward the nearest unvisited
//! occupied village point of interest at night.

use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::chunk::poi::PoiOccupancy;

use super::target::TargetView;
use super::{Goal, MobAiContext};

// `MoveThroughVillageGoal(this, 1.0, true, 4, ...)` (`Zombie.java:121`).
const SPEED: f64 = 1.0;
const DISTANCE_TO_POI: f64 = 4.0;
// `LandRandomPos.getPos(mob, 15, 7, ...)` scored by a POI within 10 of the try.
const SEARCH_RADIUS: i32 = 25;

#[derive(Debug, Clone, Default)]
pub(super) struct MoveThroughVillageGoal {
    poi: Option<BlockPos>,
    visited: Vec<BlockPos>,
}

fn closer_than(pos: BlockPos, body: Vec3d, distance: f64) -> bool {
    (f64::from(pos.x) + 0.5 - body.x).powi(2)
        + (f64::from(pos.y) + 0.5 - body.y).powi(2)
        + (f64::from(pos.z) + 0.5 - body.z).powi(2)
        < distance * distance
}

impl Goal for MoveThroughVillageGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.visited.len() > 15 {
            self.visited.remove(0);
        }
        if targets.bright_outside() != Some(false) {
            return false;
        }
        let origin = targets.body_entity().block_position();
        if targets
            .level()
            .chunks()
            .sections_to_village([origin.x >> 4, origin.y >> 4, origin.z >> 4])
            .is_none_or(|distance| distance > 6)
        {
            return false;
        }
        let mut nearest: Option<(i64, BlockPos)> = None;
        targets.level().chunks().poi().visit_records_in_range(
            [origin.x, origin.y, origin.z],
            SEARCH_RADIUS,
            PoiOccupancy::IsOccupied,
            |record| {
                let [x, y, z] = record.pos;
                let pos = BlockPos { x, y, z };
                if self.visited.contains(&pos)
                    || !targets.level().registry_tag_membership(
                        "minecraft:point_of_interest_type",
                        "minecraft:village",
                        record.type_id,
                    )
                {
                    return;
                }
                let distance = i64::from(x - origin.x).pow(2)
                    + i64::from(y - origin.y).pow(2)
                    + i64::from(z - origin.z).pow(2);
                if nearest.is_none_or(|(best, _)| distance < best) {
                    nearest = Some((distance, pos));
                }
            },
        );
        self.poi = nearest.map(|(_, pos)| pos);
        self.poi.is_some()
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(target) = self.poi {
            let can_break_doors = matches!(targets.body_entity().data(),
                crate::entity::EntityData::GeneralLiving(data)
                    if data.zombie_retained().is_some_and(|zombie| zombie.can_break_doors()));
            context.nav.navigation_request = Some(super::NavigationRequest::ThroughVillage {
                target,
                speed_modifier: SPEED,
                can_break_doors,
            });
        }
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !context.navigation_done()
            && self.poi.is_some_and(|poi| {
                !closer_than(
                    poi,
                    targets.body_entity().position(),
                    f64::from(targets.body_entity().bb_width()) + DISTANCE_TO_POI,
                )
            })
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(poi) = self.poi.take() {
            if context.navigation_done()
                || closer_than(poi, targets.body_entity().position(), DISTANCE_TO_POI)
            {
                self.visited.push(poi);
            }
        }
    }
}

#[cfg(test)]
mod tests;
