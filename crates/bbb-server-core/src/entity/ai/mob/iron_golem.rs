//! Iron golem target approach and village patrol destinations.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use super::random_pos::{land_random_pos, land_random_pos_towards, RandomPosMob};
use super::stroll::StrollPosition;
use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};
use crate::chunk::poi::PoiOccupancy;
use crate::random::GameRandom;

#[derive(Debug, Clone, Default)]
pub(super) struct MoveTowardsTargetGoal {
    target: Option<TargetRef>,
    wanted: Vec3d,
}

impl Goal for MoveTowardsTargetGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(target) = context.unchecked_target(targets) else {
            return false;
        };
        if context.position().distance_to_sqr(target.position()) > 32.0 * 32.0 {
            return false;
        }
        if !context.nav.stroll_allowed {
            return false;
        }
        let Some(wanted) = super::stroll::get_position(
            targets,
            &context.random,
            RandomPosMob::of(targets.body_entity(), context.nav.malus),
            StrollPosition::TowardsTarget {
                target: target.position(),
            },
        )
        .flatten() else {
            return false;
        };
        self.wanted = wanted;
        self.target = Some(target.target_ref());
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !context.navigation_done()
            && self
                .target
                .and_then(|target| targets.facts_of(target))
                .is_some_and(|target| {
                    target.alive()
                        && context.position().distance_to_sqr(target.position()) < 32.0 * 32.0
                })
    }

    fn stop(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.target = None;
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.navigate_to_position(self.wanted, 0.9);
    }
}

pub(super) fn patrol_position(
    targets: &TargetView<'_, '_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
) -> Option<Option<Vec3d>> {
    if random.next_float() < 0.3 {
        return land_random_pos(targets.level(), random, mob, 10, 7);
    }
    let villager_first = random.next_float() < 0.7;
    let first = if villager_first {
        towards_villager(targets, random, mob)?
    } else {
        towards_poi(targets, random, mob)?
    };
    if first.is_some() {
        return Some(first);
    }
    let second = if villager_first {
        towards_poi(targets, random, mob)?
    } else {
        towards_villager(targets, random, mob)?
    };
    if second.is_some() {
        Some(second)
    } else {
        land_random_pos(targets.level(), random, mob, 10, 7)
    }
}

fn towards_villager(
    targets: &TargetView<'_, '_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
) -> Option<Option<Vec3d>> {
    let bounds = targets
        .body_entity()
        .bounding_box()
        .inflate(32.0, 32.0, 32.0);
    let mut selected = None;
    let mut count = 0;
    for id in targets.store().ids_in_box_sections(bounds) {
        let Some(villager) = targets.store().get(id) else {
            continue;
        };
        if villager.entity_type_id() != VANILLA_ENTITY_TYPE_VILLAGER_ID
            || !villager.bounding_box().intersects(bounds)
            || !crate::entity::villager::wants_to_spawn_golem(
                targets.store(),
                id,
                targets.game_time(),
            )
        {
            continue;
        }
        count += 1;
        if random.next_int(count) == 0 {
            selected = Some(villager.position());
        }
    }
    match selected {
        Some(position) => land_random_pos_towards(targets.level(), random, mob, 10, 7, position),
        None => Some(None),
    }
}

fn towards_poi(
    targets: &TargetView<'_, '_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
) -> Option<Option<Vec3d>> {
    let chunks = targets.level().chunks();
    let pos = targets.body_entity().block_position();
    let center = [pos.x >> 4, pos.y >> 4, pos.z >> 4];
    let mut selected = None;
    let mut count = 0;
    for x in center[0] - 2..=center[0] + 2 {
        for y in center[1] - 2..=center[1] + 2 {
            for z in center[2] - 2..=center[2] + 2 {
                if chunks.sections_to_village([x, y, z])? == 0 {
                    count += 1;
                    if random.next_int(count) == 0 {
                        selected = Some([x * 16 + 8, y * 16 + 8, z * 16 + 8]);
                    }
                }
            }
        }
    }
    let Some(center) = selected else {
        return Some(None);
    };
    let mut selected = None;
    let mut count = 0;
    chunks
        .poi()
        .visit_records_in_range(center, 8, PoiOccupancy::IsOccupied, |record| {
            count += 1;
            if random.next_int(count) == 0 {
                selected = Some(record.pos);
            }
        });
    match selected {
        Some([x, y, z]) => land_random_pos_towards(
            targets.level(),
            random,
            mob,
            10,
            7,
            Vec3d {
                x: f64::from(x) + 0.5,
                y: f64::from(y),
                z: f64::from(z) + 0.5,
            },
        ),
        None => Some(None),
    }
}
