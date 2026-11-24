//! Autonomous Panda back resting.

use crate::entity::EntityData;

use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(super) struct PandaLieOnBackGoal {
    cooldown: i32,
}

impl Goal for PandaLieOnBackGoal {
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
        self.cooldown < context.tick_count
            && panda.is_lazy()
            && panda.can_perform_action(targets.level().is_thundering())
            && context.next_int(reduced_tick_delay(400)) == 1
    }

    fn can_continue(
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
        !targets.body_entity().is_in_water()
            && (panda.is_lazy() || context.next_int(reduced_tick_delay(600)) != 1)
            && context.next_int(reduced_tick_delay(2000)) != 1
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::panda::set_on_back(entity, true);
        }
        self.cooldown = 0;
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::panda::set_on_back(entity, false);
        }
        self.cooldown = context.tick_count.saturating_add(200);
    }
}
