//! Raid approach and nearby raider recruitment.

use bbb_protocol::packets::Vec3d;

use crate::entity::raider::{can_join_raid, PendingRaiderRecruit, RaiderRaid};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData};

use super::navigation::containing_block;
use super::random_pos::{default_random_pos_towards, RandomPosMob};
use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct RaiderTravelGoal {
    recruitment_tick: i32,
}

fn current_raid(body: &Entity) -> Option<i32> {
    let EntityData::GeneralLiving(data) = body.data() else {
        return None;
    };
    match data.raider_retained()?.raid() {
        RaiderRaid::Current(id) => Some(id),
        RaiderRaid::None | RaiderRaid::Unresolved(_) => None,
    }
}

fn can_approach(targets: &TargetView<'_, '_, '_>) -> bool {
    let Some(raid) =
        current_raid(targets.body_entity()).and_then(|id| targets.level().raider_raid(id))
    else {
        return false;
    };
    if !raid.active || raid.is_over {
        return false;
    }
    let pos = containing_block(targets.body_entity().position());
    targets
        .level()
        .chunks()
        .sections_to_village([pos.x >> 4, pos.y >> 4, pos.z >> 4])
        .is_some_and(|distance| distance > 1)
}

impl Goal for RaiderTravelGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.target.target.is_none()
            && matches!(
                targets
                    .store()
                    .controlling_mob_passenger(targets.body_entity(), targets.level()),
                Some(None)
            )
            && can_approach(targets)
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        can_approach(targets)
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(raid_id) = current_raid(targets.body_entity()) else {
            return;
        };
        let Some(raid_center) = targets
            .level()
            .raider_raid(raid_id)
            .filter(|raid| raid.active)
            .map(|raid| raid.center)
        else {
            return;
        };
        if targets.body_entity().tick_count() > self.recruitment_tick {
            self.recruitment_tick = targets.body_entity().tick_count().saturating_add(20);
            let mut recruits = Vec::new();
            {
                let body = targets.body_entity();
                let bodies = EntityBodyReadView::new(
                    targets.store(),
                    Some(body),
                    targets.level().detached_bodies(),
                );
                let bounds = body.bounding_box().inflate(16.0, 16.0, 16.0);
                for id in bodies.ids_in_box_sections(bounds) {
                    let Some(candidate) = bodies.get(id) else {
                        continue;
                    };
                    if !candidate.bounding_box().intersects(bounds) || !can_join_raid(candidate) {
                        continue;
                    }
                    if current_raid(candidate)
                        .and_then(|id| targets.level().raider_raid(id))
                        .is_some_and(|raid| raid.active)
                    {
                        continue;
                    }
                    recruits.push(TargetRef {
                        entity_id: candidate.id(),
                        uuid: candidate.uuid(),
                    });
                }
            }
            for recruit in recruits {
                targets.queue_command(crate::entity::EntityCommand::RaiderRecruit(
                    PendingRaiderRecruit { raid_id, recruit },
                ));
            }
        }
        if context.navigation_done() {
            if let Some(pos) = default_random_pos_towards(
                targets.level(),
                &context.random,
                RandomPosMob::of(targets.body_entity(), context.nav.malus),
                15,
                4,
                Vec3d {
                    x: f64::from(raid_center.x) + 0.5,
                    y: f64::from(raid_center.y),
                    z: f64::from(raid_center.z) + 0.5,
                },
                f64::from(std::f32::consts::FRAC_PI_2),
            )
            .flatten()
            {
                context.navigate_to_position(pos, 1.0);
            }
        }
    }
}
