use super::spec::{GoalSpec, MobGoal};
use super::target::TargetView;
use super::MobAiContext;
use crate::entity::ai::{Goal, GoalControl, GoalControls};
use bbb_protocol::packets::{DebugGoal, DebugGoalInfo};

#[derive(Clone)]
struct GoalEntry {
    priority: i32,
    controls: GoalControls,
    running: bool,
    goal: MobGoal,
}

/// `GoalSelector`: goals in insertion order and which one holds each control.
#[derive(Clone, Default)]
pub(super) struct GoalSelector {
    goals: Vec<GoalEntry>,
    locked_controls: [Option<usize>; 4],
    disabled_controls: GoalControls,
    vindicator_breaking: bool,
}

impl GoalSelector {
    pub(super) fn reserve(&mut self, additional: usize) {
        self.goals.reserve(additional);
    }

    pub(super) fn can_vindicator_break(&self) -> bool {
        !self.disabled_controls.contains(GoalControl::Move)
            && !self.goals.iter().any(|entry| {
                entry.running && entry.priority <= 2 && entry.controls.contains(GoalControl::Move)
            })
    }

    pub(super) fn set_vindicator_breaking(&mut self, breaking: bool) {
        self.vindicator_breaking = breaking;
    }

    #[inline]
    pub(super) fn add(&mut self, priority: i32, spec: GoalSpec, goal: MobGoal) {
        self.goals.push(GoalEntry {
            priority,
            controls: spec.controls(),
            running: false,
            goal,
        });
    }

    /// Removes the first goal `matches` names, answering whether it was running.
    pub(super) fn remove(&mut self, matches: impl Fn(&MobGoal) -> bool) -> Option<bool> {
        let index = self.goals.iter().position(|entry| matches(&entry.goal))?;
        let running = self.goals.remove(index).running;
        for owner in &mut self.locked_controls {
            *owner = match *owner {
                Some(owner) if owner == index => None,
                Some(owner) if owner > index => Some(owner - 1),
                other => other,
            };
        }
        Some(running)
    }

    pub(super) fn is_running(&self, matches: impl Fn(&MobGoal) -> bool) -> bool {
        self.goals
            .iter()
            .any(|entry| entry.running && matches(&entry.goal))
    }

    pub(super) fn goals(&self) -> impl Iterator<Item = &MobGoal> {
        self.goals.iter().map(|entry| &entry.goal)
    }

    pub(super) fn notify_silverfish_hurt(&mut self, now: i64) {
        for entry in &mut self.goals {
            if let MobGoal::SilverfishWakeFriends(goal) = &mut entry.goal {
                goal.notify_hurt(now);
            }
        }
    }

    pub(super) fn observe_nautilus_damage(
        &mut self,
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
    ) {
        for entry in &mut self.goals {
            if let MobGoal::Nautilus(goal) = &mut entry.goal {
                goal.observe_damage(context, targets);
                return;
            }
        }
    }

    pub(super) fn len(&self) -> usize {
        self.goals.len()
    }

    pub(super) fn set_control_enabled(&mut self, control: GoalControl, enabled: bool) {
        if enabled {
            self.disabled_controls.remove(control);
        } else {
            self.disabled_controls.insert(control);
        }
    }

    pub(super) fn debug_info(&self) -> DebugGoalInfo {
        DebugGoalInfo {
            goals: self
                .goals
                .iter()
                .map(|entry| DebugGoal {
                    priority: entry.priority,
                    is_running: entry.running,
                    name: entry.goal.debug_name().to_owned(),
                })
                .collect(),
        }
    }

    pub(super) fn tick_with_effects(
        &mut self,
        context: &mut MobAiContext,
        after_effect: &mut impl FnMut(&mut MobAiContext, &mut TargetView<'_, '_, '_>) -> bool,
        targets: &mut TargetView<'_, '_, '_>,
    ) {
        self.cleanup(context, after_effect, targets);
        self.update(context, after_effect, targets);
        self.tick_running(context, true, after_effect, targets);
    }

    pub(super) fn tick_running(
        &mut self,
        context: &mut MobAiContext,
        force: bool,
        after_tick: &mut impl FnMut(&mut MobAiContext, &mut TargetView<'_, '_, '_>) -> bool,
        targets: &mut TargetView<'_, '_, '_>,
    ) {
        for entry in &mut self.goals {
            if entry.running
                && !(self.vindicator_breaking
                    && entry.priority > 2
                    && entry.controls.contains(GoalControl::Move))
                && (force || entry.goal.requires_update_every_tick())
            {
                entry.goal.tick(context, targets);
                let _ = after_tick(context, targets);
            }
        }
    }

    fn cleanup(
        &mut self,
        context: &mut MobAiContext,
        after_effect: &mut impl FnMut(&mut MobAiContext, &mut TargetView<'_, '_, '_>) -> bool,
        targets: &mut TargetView<'_, '_, '_>,
    ) {
        for entry in &mut self.goals {
            if !entry.running {
                continue;
            }
            let should_stop = entry.controls.intersects(self.disabled_controls)
                || (self.vindicator_breaking
                    && entry.priority > 2
                    && entry.controls.contains(GoalControl::Move))
                || {
                    let can_continue = entry.goal.can_continue(context, targets);
                    let allowed = after_effect(context, targets);
                    !(can_continue && allowed)
                };
            if should_stop {
                entry.running = false;
                entry.goal.stop(context, targets);
                let _ = after_effect(context, targets);
            }
        }
        for owner in &mut self.locked_controls {
            if owner.is_some_and(|index| !self.goals[index].running) {
                *owner = None;
            }
        }
    }

    fn update(
        &mut self,
        context: &mut MobAiContext,
        after_effect: &mut impl FnMut(&mut MobAiContext, &mut TargetView<'_, '_, '_>) -> bool,
        targets: &mut TargetView<'_, '_, '_>,
    ) {
        for index in 0..self.goals.len() {
            let candidate = &self.goals[index];
            if candidate.running
                || (self.vindicator_breaking
                    && candidate.priority > 2
                    && candidate.controls.contains(GoalControl::Move))
                || candidate.controls.intersects(self.disabled_controls)
                || !self.can_replace_all_controls(index)
            {
                continue;
            }
            let can_use = self.goals[index].goal.can_use(context, targets);
            if !after_effect(context, targets) || !can_use {
                continue;
            }
            for control in self.goals[index].controls.iter() {
                if let Some(owner) = self.locked_controls[control.index()] {
                    if self.goals[owner].running {
                        self.goals[owner].running = false;
                        self.goals[owner].goal.stop(context, targets);
                        let _ = after_effect(context, targets);
                    }
                    for lock in &mut self.locked_controls {
                        if *lock == Some(owner) {
                            *lock = None;
                        }
                    }
                }
                self.locked_controls[control.index()] = Some(index);
            }
            self.goals[index].running = true;
            self.goals[index].goal.start(context, targets);
            let _ = after_effect(context, targets);
        }
    }

    /// `WrappedGoal.canBeReplacedBy` (`WrappedGoal.java:16-18`).
    fn can_replace_all_controls(&self, index: usize) -> bool {
        let candidate = &self.goals[index];
        candidate.controls.iter().all(|control| {
            self.locked_controls[control.index()].is_none_or(|owner| {
                self.goals[owner].goal.is_interruptable()
                    && candidate.priority < self.goals[owner].priority
            })
        })
    }
}
