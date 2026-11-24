//! Guardian laser goal.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID;
use bbb_server_types::geometry::Vec3Ops;

use super::target::TargetView;
use super::{Goal, MobAiContext, MobPostStepAction};

#[derive(Debug, Clone, Default)]
pub(super) struct GuardianAttackGoal {
    charge: i32,
}

fn elder(targets: &TargetView<'_, '_, '_>) -> bool {
    targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
}

impl Goal for GuardianAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.valid_target(targets).is_some()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.valid_target(targets).is_some_and(|target| {
            elder(targets) || context.position().distance_to_sqr(target.position()) > 9.0
        })
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.charge = -10;
        context.stop_navigation();
        look_at_target(context, targets);
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::guardian::set_attack_target(entity, 0);
        }
        context.set_target(None, targets);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((target_id, position, eye_y, los)) = context.valid_target(targets).map(|target| {
            (
                target.entity_id(),
                target.position(),
                target.eye_y(),
                target.line_of_sight(),
            )
        }) else {
            return;
        };
        context.stop_navigation();
        context.set_look_at_with_limits(position.x, eye_y, position.z, 90.0, 90.0);
        if !los {
            context.set_target(None, targets);
            return;
        }
        self.charge += 1;
        if self.charge == 0 {
            if let Some((entity, _, _)) = targets.world_mut() {
                crate::entity::guardian::set_attack_target(entity, target_id);
            }
            if !targets.body_entity().is_silent() {
                targets.body_entity_event(21);
            }
        } else if self.charge >= if elder(targets) { 60 } else { 80 } {
            context
                .post_step
                .push(MobPostStepAction::GuardianAttack { target_id });
            context.set_target(None, targets);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

fn look_at_target(context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) {
    if let Some(target) = context.valid_target(targets) {
        let pos = target.position();
        context.set_look_at_with_limits(pos.x, target.eye_y(), pos.z, 90.0, 90.0);
    }
}
