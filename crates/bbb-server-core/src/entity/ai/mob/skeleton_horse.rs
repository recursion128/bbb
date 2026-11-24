use crate::entity::{skeleton_horse, EntityCommand};

use super::target::TargetView;
use super::{Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(super) struct SkeletonHorseTrapGoal;

impl Goal for SkeletonHorseTrapGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        skeleton_horse::is_trap(targets.body_entity())
            && skeleton_horse::has_trigger_player(
                targets.body_entity().position(),
                targets.level().players(),
            )
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        false
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let entity_id = targets.body_entity().id();
        let uuid = targets.body_entity().uuid();
        targets.queue_command(EntityCommand::SkeletonHorseTrap { entity_id, uuid });
    }
}
