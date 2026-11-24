use super::{Goal, MobAiContext};
use crate::entity::EntityData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GrazingAction {
    Start,
    Eat,
}

#[derive(Clone, Default)]
pub(crate) struct SheepEatBlockGoal {
    eat_animation_tick: i32,
}

impl Goal for SheepEatBlockGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
            unreachable!("sheep has general living state");
        };
        let delay = if data.is_baby() { 50 } else { 1000 };
        if context.next_int((delay + 1) / 2) != 0 {
            return false;
        }
        crate::entity::sheep::can_eat_at(targets.body_entity().block_position(), targets.level())
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        self.eat_animation_tick > 0
    }

    fn start(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.eat_animation_tick = 20;
        context.species.sheep_action = Some(GrazingAction::Start);
    }

    fn stop(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.eat_animation_tick = 0;
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.eat_animation_tick = self.eat_animation_tick.saturating_sub(1);
        if self.eat_animation_tick == 2 {
            context.species.sheep_action = Some(GrazingAction::Eat);
        }
    }
}
