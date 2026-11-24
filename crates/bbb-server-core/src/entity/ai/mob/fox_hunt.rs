//! Fox stalking, charged hunting jumps and snow recovery.

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::VANILLA_BLOCK_SNOW_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::fox::{FoxHuntAction, FoxRetainedState};
use crate::entity::EntityData;
use crate::world::simulation_level::LevelCtx;

use super::target::TargetView;
use super::{reduced_tick_delay, spec, Goal, MobAiContext, MobPostStepAction};

fn fox<'a>(targets: &'a TargetView<'_, '_, '_>) -> Option<&'a FoxRetainedState> {
    match targets.body_entity().data() {
        EntityData::GeneralLiving(data) => data.fox_retained(),
        _ => None,
    }
}

fn pose(context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>, action: FoxHuntAction) {
    context.species.fox_reset_pitch = matches!(
        action,
        FoxHuntAction::ClearCharge | FoxHuntAction::Recover | FoxHuntAction::StopPounce
    );
    if let Some((entity, _, _)) = targets.world_mut() {
        crate::entity::fox::apply_hunt_action(entity, action);
    }
}

fn clear_path(level: &LevelCtx<'_, '_>, from: Vec3d, to: Vec3d) -> bool {
    (0..6).all(|i| {
        let t = f64::from(i) / 6.0;
        (1..4).all(|j| {
            let pos = BlockPos {
                x: (from.x + (to.x - from.x) * t).floor() as i32,
                y: (from.y + f64::from(j)).floor() as i32,
                z: (from.z + (to.z - from.z) * t).floor() as i32,
            };
            vanilla_block_state_support_facts(level.block_state(pos))
                .is_some_and(|facts| facts.replaceable())
        })
    })
}

#[derive(Debug, Clone)]
pub(super) struct FoxStalkGoal;

impl Goal for FoxStalkGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        fox(targets)
            .is_some_and(|fox| !fox.is_sleeping() && !fox.is_crouching() && !fox.is_interested())
            && !context.controls.jumping
            && context.valid_target(targets).is_some_and(|target| {
                target.alive()
                    && (spec::is_instance_of(target.entity_type_id(), "Chicken")
                        || spec::is_instance_of(target.entity_type_id(), "Rabbit"))
                    && target.distance_to_sqr(
                        context.body.position.x,
                        context.body.position.y,
                        context.body.position.z,
                    ) > 36.0
            })
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        pose(context, targets, FoxHuntAction::Stalk);
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let charge_at = context
            .valid_target(targets)
            .filter(|target| clear_path(targets.level(), context.body.position, target.position()))
            .map(|target| (target.position(), target.eye_y()));
        if let Some((at, eye_y)) = charge_at {
            pose(context, targets, FoxHuntAction::Charge);
            context.stop_navigation();
            context.set_look_at(at.x, eye_y, at.z);
        } else {
            pose(context, targets, FoxHuntAction::ClearCharge);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((at, eye_y, dist_sqr)) = context.valid_target(targets).map(|target| {
            (
                target.position(),
                target.eye_y(),
                target.distance_to_sqr(
                    context.body.position.x,
                    context.body.position.y,
                    context.body.position.z,
                ),
            )
        }) else {
            return;
        };
        context.set_look_at(at.x, eye_y, at.z);
        if dist_sqr <= 36.0 {
            pose(context, targets, FoxHuntAction::Charge);
            context.stop_navigation();
        } else {
            context.navigate_to(at, 1.5);
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct FoxPounceGoal;

impl Goal for FoxPounceGoal {
    // `FoxPounceGoal.isInterruptable` (`Fox.java:1161-1163`).
    fn is_interruptable(&self) -> bool {
        false
    }

    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !fox(targets).is_some_and(FoxRetainedState::is_fully_crouched) {
            return false;
        }
        let Some(target_pos) = context
            .valid_target(targets)
            .filter(|target| target.alive())
            .map(|target| target.position())
        else {
            return false;
        };
        // Normal chicken and rabbit prey inherit Entity.getMotionDirection == getDirection.
        if !clear_path(targets.level(), context.body.position, target_pos) {
            context.try_navigate_to_position(target_pos, 0, 1.5);
            pose(context, targets, FoxHuntAction::ClearCharge);
            return false;
        }
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let dy = targets.body_entity().delta_movement().y;
        context
            .valid_target(targets)
            .is_some_and(|target| target.alive())
            && (dy * dy >= f64::from(0.05_f32)
                || context.body.x_rot.abs() >= 15.0
                || !context.body.on_ground)
            && !fox(targets).is_some_and(FoxRetainedState::is_faceplanted)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.controls.jumping = true;
        pose(context, targets, FoxHuntAction::Pounce);
        let pounce_target = context
            .valid_target(targets)
            .map(|target| (target.position(), target.eye_y()));
        if let Some((at, eye_y)) = pounce_target {
            context.set_look_at_with_limits(at.x, eye_y, at.z, 60.0, 30.0);
            let dx = at.x - context.body.position.x;
            let dy = at.y - context.body.position.y;
            let dz = at.z - context.body.position.z;
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            let mut movement = targets.body_entity().delta_movement();
            if len >= 1.0e-4 {
                movement.x += dx / len * 0.8;
                movement.z += dz / len * 0.8;
            }
            movement.y += 0.9;
            targets.set_body_delta_movement(movement);
        }
        context.stop_navigation();
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        pose(context, targets, FoxHuntAction::StopPounce);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let target_info = context.valid_target(targets).map(|target| {
            (
                target.entity_id(),
                target.position(),
                target.eye_y(),
                target.distance_to_sqr(
                    context.body.position.x,
                    context.body.position.y,
                    context.body.position.z,
                ),
            )
        });
        if let Some((_, at, eye_y, _)) = target_info {
            context.set_look_at_with_limits(at.x, eye_y, at.z, 60.0, 30.0);
        }
        let movement = targets.body_entity().delta_movement();
        if !fox(targets).is_some_and(FoxRetainedState::is_faceplanted) {
            if movement.y * movement.y < f64::from(0.03_f32) && context.body.x_rot != 0.0 {
                context.body.x_rot +=
                    crate::entity::living::wrap_degrees(-context.body.x_rot) * 0.2;
            } else {
                let horizontal = movement.x.hypot(movement.z);
                let biased_y = movement.y
                    * if context.controls.jumping && movement.y > 0.0 {
                        6.5
                    } else {
                        1.0
                    };
                let len = horizontal.hypot(biased_y);
                if len > f64::from(1.0e-5_f32) {
                    context.body.x_rot =
                        ((-biased_y).signum() * (horizontal / len).acos()).to_degrees() as f32;
                }
            }
        }
        if let Some((target_id, _, _, dist_sqr)) = target_info.filter(|(_, _, _, d)| *d <= 4.0) {
            let _ = dist_sqr;
            context
                .post_step
                .push(MobPostStepAction::MeleeAttack { target_id });
        } else if context.body.x_rot > 0.0
            && context.body.on_ground
            && movement.y as f32 != 0.0
            && vanilla_block_state_block_type_id(
                targets
                    .level()
                    .block_state(targets.body_entity().block_position()),
            ) == Some(VANILLA_BLOCK_SNOW_ID)
        {
            context.body.x_rot = 60.0;
            context.set_target(None, targets);
            pose(context, targets, FoxHuntAction::Faceplant);
        }
        context.set_body_x_rot(context.body.x_rot);
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct FoxFaceplantGoal {
    countdown: i32,
}

impl Goal for FoxFaceplantGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        fox(targets).is_some_and(FoxRetainedState::is_faceplanted)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.countdown > 0 && self.can_use(context, targets)
    }

    fn start(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.countdown = reduced_tick_delay(40);
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        pose(context, targets, FoxHuntAction::Recover);
    }

    fn tick(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.countdown -= 1;
    }
}
