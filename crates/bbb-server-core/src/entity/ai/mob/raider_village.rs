//! Raid members moving through a village toward unvisited homes.

use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::chunk::poi::PoiOccupancy;
use crate::entity::raider::RaiderRaid;
use crate::entity::villager::HOME_POI;
use crate::entity::EntityData;

use super::random_pos::{default_random_pos_towards, RandomPosMob};
use super::target::TargetView;
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct RaiderVillageGoal {
    selected: Option<BlockPos>,
    visited: Vec<BlockPos>,
    stuck: bool,
}

fn center_distance_squared(pos: BlockPos, body: Vec3d) -> f64 {
    (f64::from(pos.x) + 0.5 - body.x).powi(2)
        + (f64::from(pos.y) + 0.5 - body.y).powi(2)
        + (f64::from(pos.z) + 0.5 - body.z).powi(2)
}

impl Goal for RaiderVillageGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.visited.len() > 2 {
            self.visited.remove(0);
        }
        let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
            return false;
        };
        let Some(RaiderRaid::Current(id)) = data.raider_retained().map(|raider| raider.raid())
        else {
            return false;
        };
        if !targets
            .level()
            .raider_raid(id)
            .is_some_and(|raid| raid.active && !raid.is_over)
        {
            return false;
        }
        let pos = targets.body_entity().block_position();
        let mut selected = None;
        let mut count = 0;
        targets.level().chunks().poi().visit_records_in_range(
            [pos.x, pos.y, pos.z],
            48,
            PoiOccupancy::Any,
            |record| {
                let [x, y, z] = record.pos;
                let pos = BlockPos { x, y, z };
                if record.type_id == HOME_POI && !self.visited.contains(&pos) {
                    count += 1;
                    if context.random.next_int(count) == 0 {
                        selected = Some(pos);
                    }
                }
            },
        );
        self.selected = selected;
        self.selected.is_some() && context.target.target.is_none()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !context.navigation_done()
            && context.target.target.is_none()
            && !self.stuck
            && self.selected.is_some_and(|pos| {
                center_distance_squared(pos, targets.body_entity().position())
                    >= f64::from(targets.body_entity().bb_width() + 1.0).powi(2)
            })
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        let Some(pos) = self.selected else {
            return;
        };
        context.body.no_action_time = 0;
        context.target.reset_no_action_time = true;
        context.navigate_to_position(
            Vec3d {
                x: f64::from(pos.x),
                y: f64::from(pos.y),
                z: f64::from(pos.z),
            },
            f64::from(1.05_f32),
        );
        self.stuck = false;
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(pos) = self.selected {
            if center_distance_squared(pos, targets.body_entity().position()) < 1.0 {
                self.visited.push(pos);
            }
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if !context.navigation_done() {
            return;
        }
        let Some(pos) = self.selected else {
            return;
        };
        let towards = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        };
        let mob = RandomPosMob::of(targets.body_entity(), context.nav.malus);
        let Some(next_pos) = default_random_pos_towards(
            targets.level(),
            &context.random,
            mob,
            16,
            7,
            towards,
            f64::from((std::f64::consts::PI / 10.0) as f32),
        ) else {
            self.stuck = true;
            return;
        };
        let next_pos = next_pos.or_else(|| {
            default_random_pos_towards(
                targets.level(),
                &context.random,
                mob,
                8,
                7,
                towards,
                f64::from(std::f32::consts::FRAC_PI_2),
            )
            .flatten()
        });
        if let Some(next_pos) = next_pos {
            context.navigate_to_position(next_pos, f64::from(1.05_f32));
        } else {
            self.stuck = true;
        }
    }
}
