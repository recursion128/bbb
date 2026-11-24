//! Patrolling raiders hold position before engaging their target.

use crate::entity::raider::hold_ground::{HoldGroundAction, PendingHoldGround};
use crate::entity::{EntityCommand, EntityData};

use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(crate) struct RaiderHoldGroundGoal;

fn queue_action(targets: &mut TargetView<'_, '_, '_>, action: HoldGroundAction) {
    let raider = TargetRef {
        entity_id: targets.body_entity().id(),
        uuid: targets.body_entity().uuid(),
    };
    targets.queue_command(EntityCommand::RaiderHoldGround(PendingHoldGround {
        raider,
        action,
    }));
}

impl Goal for RaiderHoldGroundGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !targets.has_active_raid()
            && matches!(targets.body_entity().data(), EntityData::GeneralLiving(data) if data.raider_retained().is_some_and(|raider| raider.patrolling()))
            && context.valid_target(targets).is_some()
            && !targets.body().aggressive()
            && !targets
                .last_hurt_by_mob()
                .is_some_and(|target| target.is_player())
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        if let Some(target) = context.target.target {
            queue_action(
                targets,
                HoldGroundAction::Share {
                    target,
                    aggressive: false,
                },
            );
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(target) = context.target.target {
            queue_action(
                targets,
                HoldGroundAction::Share {
                    target,
                    aggressive: true,
                },
            );
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((dist_sqr, position, eye_y)) = context.valid_target(targets).map(|target| {
            (
                targets.body().distance_to_facts_sqr(&target),
                target.position(),
                target.eye_y(),
            )
        }) else {
            return;
        };
        if dist_sqr > 100.0 {
            context.set_look_at_with_limits(position.x, eye_y, position.z, 30.0, 30.0);
            if context.next_int(50) == 0 {
                queue_action(targets, HoldGroundAction::Ambient);
            }
        } else {
            queue_action(targets, HoldGroundAction::Aggressive);
        }
    }
}
