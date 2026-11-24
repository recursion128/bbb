use super::look_at::LookAtPlayerGoal;
use super::random_pos::{land_random_pos_away, RandomPosMob};
use super::stroll::{RandomStrollGoal, StrollPosition};
use super::target::TargetView;
use super::{Goal, MobAiContext};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

#[derive(Debug, Clone, Copy)]
enum IdleAction {
    Stroll,
    Look(i64),
    Away(Vec3d),
    Wait(i64),
}

#[derive(Debug, Clone)]
pub(super) struct HoglinIdleGoal {
    stroll: RandomStrollGoal,
    look: LookAtPlayerGoal,
    action: IdleAction,
}

impl Default for HoglinIdleGoal {
    fn default() -> Self {
        Self {
            stroll: RandomStrollGoal::new(0.4, StrollPosition::Land, 0),
            look: LookAtPlayerGoal::hoglin(),
            action: IdleAction::Wait(0),
        }
    }
}

impl Goal for HoglinIdleGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if context.target.target.is_some() {
            return false;
        }
        if !targets.body().baby()
            && context.navigation_done()
            && crate::entity::hoglin::retreat_target(targets.body_entity()).is_none()
        {
            if let Some(piglin) = crate::entity::hoglin::nearest_adult_piglin(targets.body_entity())
                .and_then(|target| targets.facts_of(target))
                .filter(|target| target.alive())
            {
                if context.position().distance_to_sqr(piglin.position()) < 64.0 {
                    let mob = RandomPosMob::of(targets.body_entity(), context.nav.malus);
                    if let Some(Some(destination)) = land_random_pos_away(
                        targets.level(),
                        &context.random,
                        mob,
                        0.0,
                        16.0,
                        7,
                        piglin.position(),
                    ) {
                        self.action = IdleAction::Away(destination);
                        return true;
                    }
                }
            }
        }
        // HoglinAi: stroll/look-walk/wait have weights 2/2/1.
        match context.next_int(5) {
            0 | 1 if context.navigation_done() && self.stroll.can_use(context, targets) => {
                self.action = IdleAction::Stroll;
            }
            2 | 3 => {
                self.action =
                    IdleAction::Look(targets.game_time() + i64::from(30 + context.next_int(31)));
            }
            _ => {
                self.action =
                    IdleAction::Wait(targets.game_time() + i64::from(30 + context.next_int(31)));
            }
        }
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if context.target.target.is_some() {
            return false;
        }
        match self.action {
            IdleAction::Stroll => self.stroll.can_continue(context, targets),
            IdleAction::Away(_) => !context.navigation_done(),
            IdleAction::Look(until) | IdleAction::Wait(until) => targets.game_time() < until,
        }
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        match self.action {
            IdleAction::Stroll => self.stroll.start(context, targets),
            IdleAction::Away(destination) => {
                context.try_navigate_to_position(destination, 0, 0.4);
            }
            IdleAction::Look(_) | IdleAction::Wait(_) => {}
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        match self.action {
            IdleAction::Stroll => self.stroll.stop(context, targets),
            IdleAction::Away(_) => context.stop_navigation(),
            IdleAction::Look(_) | IdleAction::Wait(_) => {}
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.look
            .tick_idle_attention(context, targets, matches!(self.action, IdleAction::Look(_)));
        match self.action {
            IdleAction::Stroll => self.stroll.tick(context, targets),
            IdleAction::Look(_) | IdleAction::Away(_) | IdleAction::Wait(_) => {}
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

pub(super) fn avoid_threat(context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
    let retreat = crate::entity::hoglin::retreat_target(targets.body_entity());
    if retreat.is_some()
        && !crate::entity::hoglin::is_fleeing(targets.body_entity(), context.target.target)
    {
        return false;
    }
    let (avoid, distance_sqr, speed) = if let Some(target) = retreat {
        let Some(target) = targets.facts_of(target) else {
            return false;
        };
        (target.position(), 15.0 * 15.0, 1.3)
    } else {
        let Some(pos) = crate::entity::hoglin::nearest_repellent(targets.body_entity()) else {
            return false;
        };
        (
            Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            },
            64.0,
            1.0,
        )
    };
    let dx = avoid.x - context.body.position.x;
    let dy = avoid.y - context.body.position.y;
    let dz = avoid.z - context.body.position.z;
    if dx * dx + dy * dy + dz * dz >= distance_sqr {
        return false;
    }
    if !context.nav.navigation_done
        && context.nav.navigation_target.is_some_and(|target| {
            (f64::from(target.x) + 0.5 - context.body.position.x) * dx
                + (f64::from(target.y) - context.body.position.y) * dy
                + (f64::from(target.z) + 0.5 - context.body.position.z) * dz
                < 0.0
        })
    {
        if retreat.is_some() {
            context.set_navigation_speed(speed);
            return true;
        }
        return false;
    }
    let mob = RandomPosMob::of(targets.body_entity(), context.nav.malus);
    if let Some(Some(destination)) =
        land_random_pos_away(targets.level(), &context.random, mob, 0.0, 16.0, 7, avoid)
    {
        context.navigate_to_position(destination, speed);
        return true;
    }
    false
}
