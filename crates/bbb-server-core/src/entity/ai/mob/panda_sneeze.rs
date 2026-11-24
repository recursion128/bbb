//! Autonomous Panda sneezing eligibility.

use crate::entity::EntityData;

use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(super) struct PandaSneezeGoal;

impl Goal for PandaSneezeGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
            return false;
        };
        let Some(panda) = data.panda_retained() else {
            return false;
        };
        data.is_baby()
            && panda.can_perform_action(targets.level().is_thundering())
            && ((panda.is_weak() && context.next_int(reduced_tick_delay(500)) == 1)
                || context.next_int(reduced_tick_delay(6000)) == 1)
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        false
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::panda::start_sneezing(entity);
        }
    }
}
