//! Idle shell peeking.

use super::target::TargetView;
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(crate) struct ShulkerPeekGoal {
    until: i32,
}

// Shulker: peeking ends without closing an attacking shell.
impl Goal for ShulkerPeekGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.target.target.is_none()
            && context.next_int(20) == 0
            && crate::entity::shulker::can_stay_at(
                targets.body_entity(),
                targets.store(),
                targets.level(),
            ) == Some(true)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.target.target.is_none() && context.tick_count < self.until
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.until = context
            .tick_count
            .saturating_add(20 * (1 + context.next_int(3)));
        if let Some((entity, store, level)) = targets.world_mut() {
            crate::entity::shulker::set_peek(entity, store, level, 30);
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if context.target.target.is_none() {
            if let Some((entity, store, level)) = targets.world_mut() {
                crate::entity::shulker::set_peek(entity, store, level, 0);
            }
        }
    }
}
