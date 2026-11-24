//! Silverfish delayed friend release and host-block merging.

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::relative;

use crate::block_behavior::infested::{infested_state_by_host, is_compatible_host};
use crate::entity::EntityCommand;

use super::stroll::{RandomStrollGoal, StrollPosition};
use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(crate) struct SilverfishWakeFriendsGoal {
    deadline: Option<i64>,
}

impl SilverfishWakeFriendsGoal {
    pub(super) fn notify_hurt(&mut self, now: i64) {
        if self.deadline.is_none() {
            self.deadline = Some(now + 20);
        }
    }
}

impl Goal for SilverfishWakeFriendsGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.deadline.is_some()
    }

    fn tick(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self
            .deadline
            .is_some_and(|deadline| targets.game_time() >= deadline)
        {
            self.deadline = None;
            let entity_id = targets.body_entity().id();
            let uuid = targets.body_entity().uuid();
            targets.queue_command(EntityCommand::SilverfishWakeFriends { entity_id, uuid });
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SilverfishMergeGoal {
    stroll: RandomStrollGoal,
    merge_pos: Option<BlockPos>,
}

impl Default for SilverfishMergeGoal {
    fn default() -> Self {
        Self {
            stroll: RandomStrollGoal::new(1.0, StrollPosition::Default, 10),
            merge_pos: None,
        }
    }
}

impl Goal for SilverfishMergeGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.merge_pos = None;
        if context.target.target.is_some() || !context.nav.navigation_done {
            return false;
        }
        if targets.level().mob_griefing() && context.next_int(reduced_tick_delay(10)) == 0 {
            const DIRECTIONS: [Direction; 6] = [
                Direction::Down,
                Direction::Up,
                Direction::North,
                Direction::South,
                Direction::West,
                Direction::East,
            ];
            let direction = DIRECTIONS[context.next_int(6) as usize];
            let position = targets.body_entity().position();
            let pos = relative(
                BlockPos {
                    x: position.x.floor() as i32,
                    y: (position.y + 0.5).floor() as i32,
                    z: position.z.floor() as i32,
                },
                direction,
            );
            if targets.level().has_ticking_column_at(pos)
                && is_compatible_host(targets.level().block_state(pos))
            {
                self.merge_pos = Some(pos);
                return true;
            }
        }
        self.stroll.can_use(context, targets)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.merge_pos.is_none() && self.stroll.can_continue(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(pos) = self.merge_pos {
            if infested_state_by_host(targets.level().block_state(pos)).is_some() {
                let entity_id = targets.body_entity().id();
                let uuid = targets.body_entity().uuid();
                targets.queue_command(EntityCommand::SilverfishMerge {
                    entity_id,
                    uuid,
                    pos,
                });
            }
        } else {
            self.stroll.start(context, targets);
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.merge_pos.take().is_none() {
            self.stroll.stop(context, targets);
        }
    }
}
