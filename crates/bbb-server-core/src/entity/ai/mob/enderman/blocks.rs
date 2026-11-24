//! Enderman carried-block goal admission and candidate sampling.

use bbb_protocol::packets::BlockPos;

use crate::entity::enderman::blocks::{BlockAction, PendingBlockMove};
use crate::entity::{EntityCommand, EntityData};

use super::super::target::{TargetRef, TargetView};
use super::super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(crate) struct EndermanLeaveBlockGoal;

#[derive(Debug, Clone)]
pub(crate) struct EndermanTakeBlockGoal;

impl Goal for EndermanLeaveBlockGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.enderman_retained().is_some_and(|state| state.carried_block_state_id().is_some()))
            && targets.level().mob_griefing()
            && context.next_int(reduced_tick_delay(2000)) == 0
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let position = targets.body_entity().position();
        let pos = BlockPos {
            x: (position.x - 1.0 + context.next_double() * 2.0).floor() as i32,
            y: (position.y + context.next_double() * 2.0).floor() as i32,
            z: (position.z - 1.0 + context.next_double() * 2.0).floor() as i32,
        };
        queue_move(targets, BlockAction::Place(pos));
    }
}

impl Goal for EndermanTakeBlockGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.enderman_retained().is_some_and(|state| state.carried_block_state_id().is_none()))
            && targets.level().mob_griefing()
            && context.next_int(reduced_tick_delay(20)) == 0
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let position = targets.body_entity().position();
        let pos = BlockPos {
            x: (position.x - 2.0 + context.next_double() * 4.0).floor() as i32,
            y: (position.y + context.next_double() * 3.0).floor() as i32,
            z: (position.z - 2.0 + context.next_double() * 4.0).floor() as i32,
        };
        queue_move(targets, BlockAction::Take(pos));
    }
}

fn queue_move(targets: &mut TargetView<'_, '_, '_>, action: BlockAction) {
    let enderman = TargetRef {
        entity_id: targets.body_entity().id(),
        uuid: targets.body_entity().uuid(),
    };
    targets.queue_command(EntityCommand::EndermanBlockMove(PendingBlockMove {
        enderman,
        action,
    }));
}
