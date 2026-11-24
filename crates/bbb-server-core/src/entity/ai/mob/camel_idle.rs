//! Idle camel pose changes.

use super::target::TargetView;
use super::{Goal, MobAiContext};
use crate::entity::camel::{idle_pose_change, CamelPoseChange};

#[derive(Debug, Clone, Default)]
pub(crate) struct CamelIdlePoseGoal {
    next_attempt: i64,
    change: Option<CamelPoseChange>,
}

impl Goal for CamelIdlePoseGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.change = None;
        let now = targets.game_time();
        if now < self.next_attempt || !context.navigation_done() {
            return false;
        }
        self.next_attempt = now.saturating_add(20);
        // CamelAi: pose changes share four equally weighted idle choices.
        if context.next_int(4) != 0 {
            return false;
        }
        self.change = idle_pose_change(targets.body_entity(), targets.store(), targets.level());
        self.change.is_some()
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        false
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(change) = self.change.take() {
            if let Some((entity, store, level)) = targets.world_mut() {
                if crate::entity::camel::apply_pose_change(entity, store, level, change).is_none() {
                    targets.abort();
                }
            }
        }
    }
}
