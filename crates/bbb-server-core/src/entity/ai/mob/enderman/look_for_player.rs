use super::super::target::conditions::TargetingConditions;
use super::super::target::goals::TargetGoalState;
use super::super::target::{Target, TargetRef, TargetView};
use super::super::{reduced_tick_delay, MobAiContext};
use crate::entity::ai::Goal;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

#[derive(Debug, Clone)]
pub(crate) struct State {
    pending: Option<TargetRef>,
    target: Option<TargetRef>,
    aggro_time: i32,
    teleport_time: i32,
    base: TargetGoalState,
}
impl Default for State {
    fn default() -> Self {
        Self {
            pending: None,
            target: None,
            aggro_time: 0,
            teleport_time: 0,
            base: TargetGoalState::new(false),
        }
    }
}
impl State {
    pub(in crate::entity::ai::mob) fn teleport_succeeded(&mut self) {
        self.teleport_time = 0;
    }
}
#[derive(Debug, Clone)]
pub(crate) struct LookForPlayerGoal;
impl Goal for LookForPlayerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let conditions = TargetingConditions::for_combat()
            .range(context.body.follow_range)
            .ignore_line_of_sight();
        let mut nearest = None;
        for target in targets.local_players() {
            if !conditions
                .test_checked(targets, &target, |target| {
                    anger_inducing(context, targets, target)
                })
                .is_some_and(|answer| answer)
                || target.checked_line_of_sight() != Some(true)
            {
                continue;
            }
            let distance = context.body.position.distance_to_sqr(target.position());
            if nearest.is_none_or(|(best, _)| distance < best) {
                nearest = Some((distance, target.target_ref()));
            }
        }
        context.target.enderman_look.pending = nearest.map(|(_, target)| target);
        context.target.enderman_look.pending.is_some()
    }
    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let mut state = std::mem::take(&mut context.target.enderman_look);
        let answer = if let Some(pending) = state.pending {
            targets.facts_of(pending).is_some_and(|target| {
                if anger_inducing(context, targets, &target) != Some(true) {
                    return false;
                }
                look_at(context, target.position(), target.eye_y());
                true
            })
        } else {
            let continuing = state
                .target
                .and_then(|target| targets.facts_of(target))
                .is_some_and(|target| {
                    target.rides_body() != Some(true)
                        && TargetingConditions::for_combat()
                            .ignore_line_of_sight()
                            .test(targets, &target, |_| true)
                });
            continuing || state.base.can_continue_to_use(context, targets)
        };
        context.target.enderman_look = state;
        answer
    }
    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.target.enderman_look.aggro_time = reduced_tick_delay(5);
        context.target.enderman_look.teleport_time = 0;
        context.target.writes.enderman_stared_at = Some(true);
    }
    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let mut state = std::mem::take(&mut context.target.enderman_look);
        state.pending = None;
        state.target = None;
        state.base.stop(context, targets);
        context.target.enderman_look = state;
    }
    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let mut state = std::mem::take(&mut context.target.enderman_look);
        if context.valid_target(targets).is_none() {
            state.target = None;
        }
        if let Some(pending) = state.pending {
            state.aggro_time -= 1;
            if state.aggro_time <= 0 {
                state.target = Some(pending);
                state.pending = None;
                context.set_target(state.target, targets);
                state.base.start();
            }
        } else if !context.body.is_passenger {
            if let Some(target) = state.target.and_then(|target| targets.facts_of(target)) {
                let distance = target.position().distance_to_sqr(context.body.position);
                if stared(context, targets, &target) == Some(true) {
                    if distance < 16.0 && context.body.health > 0.0 {
                        let destination = crate::entity::enderman::random_destination(
                            &context.random,
                            context.body.position,
                        );
                        context
                            .species
                            .enderman_teleports
                            .push(super::teleport::Request {
                                destination,
                                reset_timer_on_success: false,
                            });
                    }
                    state.teleport_time = 0;
                } else if distance > 256.0 {
                    let before = state.teleport_time;
                    state.teleport_time += 1;
                    if before >= reduced_tick_delay(30) {
                        let direction = Vec3d {
                            x: context.body.position.x - target.position().x,
                            y: context.body.position.y
                                + f64::from(targets.body_entity().bb_height()) * 0.5
                                - target.eye_y(),
                            z: context.body.position.z - target.position().z,
                        }
                        .normalize();
                        let destination = Vec3d {
                            x: context.body.position.x + (context.random.next_double() - 0.5) * 8.0
                                - direction.x * 16.0,
                            y: context.body.position.y + f64::from(context.random.next_int(16) - 8)
                                - direction.y * 16.0,
                            z: context.body.position.z + (context.random.next_double() - 0.5) * 8.0
                                - direction.z * 16.0,
                        };
                        context
                            .species
                            .enderman_teleports
                            .push(super::teleport::Request {
                                destination,
                                reset_timer_on_success: true,
                            });
                    }
                }
            }
        }
        context.target.enderman_look = state;
    }
}
fn stared<'w>(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, 'w>,
    target: &Target<'_, '_, 'w>,
) -> Option<bool> {
    let player = target.player()?;
    if !crate::entity::living::gaze::not_wearing_disguise(targets.level(), player) {
        return Some(false);
    }
    crate::entity::living::gaze::is_looking_at_me(
        targets.level(),
        context.body.position,
        &[context.body.eye_y],
        player,
        0.025,
        true,
        false,
    )
}
fn anger_inducing<'w>(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, 'w>,
    target: &Target<'_, '_, 'w>,
) -> Option<bool> {
    Some(
        (stared(context, targets, target)?
            || context
                .target
                .anger
                .is_some_and(|anger| anger.is_angry_at(targets, target)))
            && target.rides_body() != Some(true),
    )
}
fn look_at(context: &mut MobAiContext, target: Vec3d, eye_y: f64) {
    let dx = target.x - context.body.position.x;
    let dz = target.z - context.body.position.z;
    let dy = eye_y - context.body.eye_y;
    let degrees = |radians: f64| radians * 180.0 / f64::from(std::f32::consts::PI);
    let yaw = degrees(crate::entity::projectile_hit::mth_atan2(dz, dx)) as f32 - 90.0;
    let pitch = -degrees(crate::entity::projectile_hit::mth_atan2(
        dy,
        (dx * dx + dz * dz).sqrt(),
    )) as f32;
    context.body.x_rot +=
        crate::entity::living::wrap_degrees(pitch - context.body.x_rot).clamp(-10.0, 10.0);
    context.body.y_rot +=
        crate::entity::living::wrap_degrees(yaw - context.body.y_rot).clamp(-10.0, 10.0);
}
