//! Warden emergence and departure on its retained lifecycle memories.

use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_EMERGING_ID, VANILLA_ENTITY_POSE_ROARING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::packets::SoundSource;

use super::target::TargetView;
use super::{Goal, MobAiContext};
use crate::entity::warden::anger::WardenState;
use crate::entity::EntityData;

/// `WardenAi.DIGGING_DURATION` (`WardenAi.java:48`).
const DIG_DURATION: i64 = 100;

#[derive(Debug, Clone, Copy, Default)]
enum Phase {
    #[default]
    Ready,
    Emerging,
    WaitingLanding {
        entered_at: i64,
    },
    Digging {
        started_at: i64,
    },
}

#[derive(Debug, Clone, Default)]
pub(super) struct WardenLifecycleGoal {
    phase: Phase,
}

fn state<'a>(targets: &'a TargetView<'_, '_, '_>) -> Option<&'a WardenState> {
    let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
        return None;
    };
    data.warden_state()
}

fn emerging(targets: &TargetView<'_, '_, '_>) -> bool {
    state(targets).is_some_and(|state| state.lifecycle.is_emerging())
}

fn finish_emergence(targets: &mut TargetView<'_, '_, '_>) {
    if !emerging(targets) && targets.body_entity().pose_id() == VANILLA_ENTITY_POSE_EMERGING_ID {
        targets.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
    }
}

impl WardenLifecycleGoal {
    fn begin_emergence(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) {
        self.phase = Phase::Emerging;
        context.stop_navigation();
        if !emerging(targets) {
            return;
        }
        targets.set_pose(VANILLA_ENTITY_POSE_EMERGING_ID);
        if !targets.body_entity().is_silent() {
            // `Emerging.start`.
            targets.play_body_sound(
                "minecraft:entity.warden.emerge",
                SoundSource::Hostile,
                5.0,
                1.0,
            );
        }
    }
}

impl Goal for WardenLifecycleGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(state) = state(targets) else {
            return false;
        };
        state.lifecycle.is_emerging()
            || targets.body_entity().pose_id() == VANILLA_ENTITY_POSE_EMERGING_ID
            || (state.lifecycle.can_dig()
                && context.navigation_done()
                && context.valid_target(targets).is_none()
                && targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_ROARING_ID
                && state.angry_target(targets).is_none())
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        match self.phase {
            Phase::Ready => false,
            // Digging: an attack target prevents starting the dig.
            Phase::WaitingLanding { .. } => self.can_use(context, targets),
            Phase::Emerging | Phase::Digging { .. } => true,
        }
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if emerging(targets) || targets.body_entity().pose_id() == VANILLA_ENTITY_POSE_EMERGING_ID {
            self.begin_emergence(context, targets);
        } else {
            context.stop_navigation();
            // `WardenAi.initDiggingActivity`.
            if let Some((entity, store, _)) = targets.world_mut() {
                if store.stop_invalid_riding(entity.id()) {
                    entity.set_boarding_cooldown(60);
                }
            }
            self.phase = Phase::WaitingLanding {
                entered_at: targets.game_time(),
            };
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        if matches!(self.phase, Phase::Emerging) {
            finish_emergence(targets);
        }
        self.phase = Phase::Ready;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        if emerging(targets) && !matches!(self.phase, Phase::Emerging) {
            self.begin_emergence(context, targets);
        }
        match self.phase {
            Phase::Ready => {}
            Phase::Emerging => {
                if !emerging(targets) {
                    finish_emergence(targets);
                    self.phase = Phase::Ready;
                }
            }
            Phase::WaitingLanding { entered_at } => {
                // ForceUnmount must reach the real body before its landing is read.
                if targets.game_time() > entered_at
                    && (targets.body_entity().on_ground()
                        || targets.body_entity().is_in_water()
                        || targets.body_entity().is_in_lava())
                {
                    let started_at = targets.game_time();
                    if let Some((entity, store, level)) = targets.world_mut() {
                        if !crate::entity::warden::lifecycle::begin_dig(entity, store, level) {
                            targets.abort();
                        }
                    }
                    self.phase = Phase::Digging { started_at };
                }
            }
            Phase::Digging { started_at } => {
                if targets.game_time() - started_at >= DIG_DURATION {
                    if let Some((entity, _, _)) = targets.world_mut() {
                        entity.discard();
                        targets.abort();
                    }
                }
            }
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
