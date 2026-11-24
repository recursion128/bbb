use super::goals::{HurtByTargetGoal, HurtByVariant, NearestAttackableTargetGoal, NearestVariant};
use super::selector::TargetSelector;
use super::{TargetEffect, TargetView};
use crate::entity::ai::mob::spec::HurtBySpec;
use crate::entity::ai::mob::MobAiContext;
use crate::entity::ai::Goal;

#[derive(Debug, Clone)]
pub(crate) struct State {
    hurt_by: HurtByTargetGoal,
    nearest: NearestAttackableTargetGoal,
}

impl Default for State {
    fn default() -> Self {
        Self {
            hurt_by: HurtByTargetGoal::from_spec(HurtBySpec {
                variant: HurtByVariant::Bee,
                ignore_damage: &[],
                alert_others: Some(&[]),
            }),
            nearest: NearestAttackableTargetGoal::new(
                NearestVariant::BeeBecomeAngry,
                "Player",
                10,
                true,
                TargetSelector::AngryAt,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BeeTargetKind {
    HurtBy,
    Nearest,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BeeTargetGoal {
    kind: BeeTargetKind,
}

impl BeeTargetGoal {
    pub(crate) const fn new(kind: BeeTargetKind) -> Self {
        Self { kind }
    }
}

impl Goal for BeeTargetGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let mut state = std::mem::take(&mut context.target.bee);
        let result = match self.kind {
            BeeTargetKind::HurtBy => state.hurt_by.can_use(context, targets),
            BeeTargetKind::Nearest => state
                .nearest
                .bee_can_use_checked(context, |target| target.checked_line_of_sight(), targets)
                .is_some_and(|answer| answer),
        };
        context.target.bee = state;
        result
    }
    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let mut state = std::mem::take(&mut context.target.bee);
        let result = match self.kind {
            BeeTargetKind::HurtBy => state.hurt_by.bee_can_continue_checked(
                context,
                |target| target.checked_line_of_sight(),
                targets,
            ),
            BeeTargetKind::Nearest => state.nearest.bee_can_continue_checked(
                context,
                |target| target.checked_line_of_sight(),
                targets,
            ),
        }
        .is_some_and(|answer| answer);
        context.target.bee = state;
        result
    }
    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let mut state = std::mem::take(&mut context.target.bee);
        match self.kind {
            BeeTargetKind::HurtBy => state.hurt_by.start(context, targets),
            BeeTargetKind::Nearest => state.nearest.start(context, targets),
        }
        context.target.bee = state;
        for effect in std::mem::take(&mut context.target.bee_alert_requests) {
            let TargetEffect::SetMobTarget {
                target: Some(target),
                ..
            } = effect
            else {
                continue;
            };
            if targets
                .facts_of(target)
                .is_some_and(|target| target.direct_line_of_sight() == Some(true))
            {
                context.target.effects.push(effect);
            }
        }
    }
    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let mut state = std::mem::take(&mut context.target.bee);
        match self.kind {
            BeeTargetKind::HurtBy => state.hurt_by.stop(context, targets),
            BeeTargetKind::Nearest => state.nearest.stop(context, targets),
        }
        context.target.bee = state;
    }
}

pub(super) fn is_angry(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
    context
        .target
        .anger
        .is_some_and(|anger| anger.is_angry(targets.game_time()))
}
pub(in crate::entity::ai::mob) fn can_target(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
) -> bool {
    is_angry(context, targets) && !targets.bee_has_stung()
}
