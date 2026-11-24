#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum GoalControl {
    Move,
    Look,
    Jump,
    Target,
}

impl GoalControl {
    pub(crate) const ALL: [Self; 4] = [Self::Move, Self::Look, Self::Jump, Self::Target];

    pub(crate) const fn bit(self) -> u8 {
        1 << self as u8
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GoalControls(u8);

impl GoalControls {
    pub(crate) const NONE: Self = Self(0);
    pub(crate) const MOVE: Self = Self(GoalControl::Move.bit());
    pub(crate) const LOOK: Self = Self(GoalControl::Look.bit());
    pub(crate) const JUMP: Self = Self(GoalControl::Jump.bit());
    pub(crate) const TARGET: Self = Self(GoalControl::Target.bit());

    pub(crate) const fn contains(self, control: GoalControl) -> bool {
        self.0 & control.bit() != 0
    }

    pub(crate) const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub(crate) const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub(crate) fn insert(&mut self, control: GoalControl) {
        self.0 |= control.bit();
    }

    pub(crate) fn remove(&mut self, control: GoalControl) {
        self.0 &= !control.bit();
    }

    pub(crate) fn iter(self) -> impl Iterator<Item = GoalControl> {
        GoalControl::ALL
            .into_iter()
            .filter(move |control| self.contains(*control))
    }
}

use super::mob::target::TargetView;
use super::mob::MobAiContext;

/// One goal's `canUse`/`start`/`tick`/`stop` cycle, dispatched through
/// [`super::mob::spec::MobGoal`].
pub(crate) trait Goal {
    fn can_use(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>)
        -> bool;

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.can_use(context, targets)
    }

    fn start(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {}

    fn stop(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {}

    fn tick(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {}

    fn requires_update_every_tick(&self) -> bool {
        false
    }

    /// `Goal.isInterruptable` (`Goal.java:18-20`).
    fn is_interruptable(&self) -> bool {
        true
    }
}
