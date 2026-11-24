//! Fox rest under shelter during daylight.

use bbb_protocol::packets::BlockPos;

use crate::entity::EntityData;

use super::navigation::NavigationMove;
use super::random_pos::{walk_target_value, RandomPosMob};
use super::target::conditions::TargetingConditions;
use super::target::{Target, TargetView};
use super::{reduced_tick_delay, spec, Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct FoxSleepGoal {
    countdown: Option<i32>,
}

impl FoxSleepGoal {
    fn can_sleep(&mut self, context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
        let countdown = self
            .countdown
            .get_or_insert_with(|| context.next_int(reduced_tick_delay(140)));
        if *countdown > 0 {
            *countdown -= 1;
            return false;
        }
        let body = targets.body_entity();
        if targets.bright_outside() != Some(true) || body.is_in_powder_snow() {
            return false;
        }
        let pos = BlockPos {
            x: body.position().x.floor() as i32,
            y: body.bounding_box().max_y.floor() as i32,
            z: body.position().z.floor() as i32,
        };
        if targets.level().can_see_sky(pos)
            || !walk_target_value(
                targets.level(),
                RandomPosMob::of(body, context.nav.malus),
                pos,
            )
            .is_some_and(|value| value >= 0.0)
        {
            return false;
        }
        !has_alertable(targets)
    }
}

pub(super) fn has_alertable(targets: &TargetView<'_, '_, '_>) -> bool {
    let conditions = TargetingConditions::for_combat()
        .range(12.0)
        .ignore_line_of_sight();
    let bounds = targets
        .body_entity()
        .bounding_box()
        .inflate(12.0, 6.0, 12.0);
    targets
        .first_living_in_box(bounds, |target| {
            conditions.test(targets, target, |target| alertable(targets, target))
        })
        .is_some()
}

fn alertable(targets: &TargetView<'_, '_, '_>, target: &Target<'_, '_, '_>) -> bool {
    for class in spec::classes(target.entity_type_id()) {
        match class {
            "Fox" => return false,
            "Chicken" | "Rabbit" | "Monster" => return true,
            "TamableAnimal" => return !target.tame(),
            _ => {}
        }
    }
    if target.is_player() && (target.creative() || target.spectator()) {
        false
    } else {
        let trusted = matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.fox_retained().is_some_and(|fox| fox.trusts_uuid(target.uuid())));
        let sleeping = target.player().map_or_else(
            || {
                targets
                    .store()
                    .get(target.entity_id())
                    .is_some_and(|body| body.sleeping_pos().is_some())
            },
            |player| player.sleeping,
        );
        !trusted && !sleeping && !target.discrete()
    }
}

impl Goal for FoxSleepGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let sleeping = context.species.fox_sleep.unwrap_or_else(|| {
            matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                if data.fox_retained().is_some_and(|fox| fox.is_sleeping()))
        });
        context.controls.xxa == 0.0
            && context.controls.yya == 0.0
            && context.controls.zza == 0.0
            && (self.can_sleep(context, targets) || sleeping)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.can_sleep(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.controls.jumping = false;
        context.stop_navigation();
        context.controls.move_control_request = Some(NavigationMove {
            wanted: context.body.position,
            speed_modifier: 0.0,
        });
        context.species.fox_sleep = Some(true);
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.countdown = Some(context.next_int(reduced_tick_delay(140)));
        context.species.fox_sleep = Some(false);
    }
}
