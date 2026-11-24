use super::{Goal, MobAiContext};

const AMBIENT_STAND_INTERVAL: i32 = 400;

#[derive(Debug, Clone)]
pub(crate) struct RandomStandGoal {
    next_stand: i32,
}

impl RandomStandGoal {
    pub(super) const fn new() -> Self {
        Self {
            next_stand: -AMBIENT_STAND_INTERVAL,
        }
    }

    fn reset_stand_interval(&mut self) {
        self.next_stand = -AMBIENT_STAND_INTERVAL;
    }
}

impl Goal for RandomStandGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        self.next_stand += 1;
        if self.next_stand <= 0 || context.next_int(1000) >= self.next_stand {
            return false;
        }
        self.reset_stand_interval();
        !context.is_immobile() && context.next_int(10) == 0
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        false
    }

    fn start(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        context.request_horse_stand();
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
