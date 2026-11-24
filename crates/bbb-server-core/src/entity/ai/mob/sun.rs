//! Sun avoidance and shared shade navigation for undead mobs and foxes.

use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::EntityData;

use super::random_pos::{walk_target_value, RandomPosMob};
use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

fn bright_and_bare_headed(targets: &TargetView<'_, '_, '_>) -> bool {
    targets.bright_outside() == Some(true) && targets.body().head_item_id().is_none()
}

/// `RestrictSunGoal` (`RestrictSunGoal.java`): the ground path is cut where it
/// first meets the sky while the day is bright.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RestrictSunGoal;

impl Goal for RestrictSunGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        bright_and_bare_headed(targets)
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.set_avoid_sun(true);
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.set_avoid_sun(false);
    }
}

/// Shared `FleeSunGoal` navigation, including the fox shelter conditions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FleeSunGoal {
    speed_modifier: f64,
    wanted: Option<Vec3d>,
    fox_interval: Option<i32>,
}

impl FleeSunGoal {
    pub(super) const fn new(speed_modifier: f64, fox: bool) -> Self {
        Self {
            speed_modifier,
            wanted: None,
            fox_interval: if fox {
                Some(reduced_tick_delay(100))
            } else {
                None
            },
        }
    }
}

impl Goal for FleeSunGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        if let Some(interval) = &mut self.fox_interval {
            let sleeping = context.species.fox_sleep.unwrap_or_else(|| {
                matches!(body.data(), EntityData::GeneralLiving(data)
                    if data.fox_retained().is_some_and(|fox| fox.is_sleeping()))
            });
            if sleeping || context.target.target.is_some() {
                return false;
            }
            let pos = body.block_position();
            if !(targets.level().is_thundering() && targets.level().can_see_sky(pos)) {
                if *interval > 0 {
                    *interval -= 1;
                    return false;
                }
                *interval = 100;
                if targets.bright_outside() != Some(true)
                    || !targets.level().can_see_sky(pos)
                    || targets
                        .level()
                        .chunks()
                        .sections_to_village([pos.x >> 4, pos.y >> 4, pos.z >> 4])
                        .is_some_and(|distance| distance <= 1)
                {
                    return false;
                }
            }
        } else if context.target.target.is_some()
            || !body.is_on_fire()
            || !bright_and_bare_headed(targets)
            || !targets.level().can_see_sky(body.block_position())
        {
            return false;
        }
        self.wanted = hide_pos(context, targets);
        self.wanted.is_some()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !context.nav.navigation_done
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        if self.fox_interval.is_some() {
            context.species.fox_sleep = Some(false);
        }
        if let Some(wanted) = self.wanted {
            context.navigate_to_position(wanted, self.speed_modifier);
        }
    }
}

// `FleeSunGoal.getHidePos`.
fn hide_pos(context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) -> Option<Vec3d> {
    let body = targets.body_entity();
    let level = targets.level();
    let mob = RandomPosMob::of(body, context.nav.malus);
    let origin = body.block_position();
    for _ in 0..10 {
        let pos = BlockPos {
            x: origin.x + context.next_int(20) - 10,
            y: origin.y + context.next_int(6) - 3,
            z: origin.z + context.next_int(20) - 10,
        };
        if !level.can_see_sky(pos) && walk_target_value(level, mob, pos)? < 0.0 {
            return Some(Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            });
        }
    }
    None
}
