//! `SitWhenOrderedToGoal`: holds a tame
//! animal's movement while it is ordered to sit. The sitting pose bit itself
//! follows the order (`TamableRetainedState::set_ordered_to_sit`).

use super::{target::TargetView, Goal, MobAiContext};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SitWhenOrderedToGoal;

impl Goal for SitWhenOrderedToGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        targets.body().ordered_to_sit() && !context.body.in_water && context.body.on_ground
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        targets.body().ordered_to_sit()
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
    }
}
