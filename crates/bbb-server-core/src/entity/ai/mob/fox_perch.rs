//! Idle fox sitting and surveying through the shared look control.

use crate::entity::fox::FoxHuntAction;
use crate::entity::EntityData;

use super::fox_sleep::has_alertable;
use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct FoxPerchGoal {
    relative_x: f64,
    relative_z: f64,
    look_time: i32,
    looks_remaining: i32,
}

impl FoxPerchGoal {
    fn reset_look(&mut self, context: &mut MobAiContext) {
        let angle = std::f64::consts::TAU * context.next_double();
        self.relative_x = angle.cos();
        self.relative_z = angle.sin();
        self.look_time = reduced_tick_delay(80 + context.next_int(20));
    }
}

impl Goal for FoxPerchGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        targets.last_hurt_by_mob().is_none()
            && context.next_float() < 0.02
            && matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                if data.fox_retained().is_some_and(|fox|
                    !fox.is_sleeping() && !fox.is_pouncing() && !fox.is_crouching()))
            && context.target.target.is_none()
            && context.navigation_done()
            && !has_alertable(targets)
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.looks_remaining > 0
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.reset_look(context);
        self.looks_remaining = 2 + context.next_int(3);
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::fox::apply_hunt_action(entity, FoxHuntAction::Sit);
        }
        context.stop_navigation();
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::fox::apply_hunt_action(entity, FoxHuntAction::Stand);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.look_time -= 1;
        if self.look_time <= 0 {
            self.looks_remaining -= 1;
            self.reset_look(context);
        }
        context.set_look_at_with_limits(
            context.body.position.x + self.relative_x,
            context.body.eye_y,
            context.body.position.z + self.relative_z,
            75.0,
            40.0,
        );
    }
}
