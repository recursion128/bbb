//! Warden anger selects a target; roar then delegates pursuit and hits to melee.

use bbb_protocol::entity_poses::{VANILLA_ENTITY_POSE_ROARING_ID, VANILLA_ENTITY_POSE_STANDING_ID};
use bbb_protocol::packets::SoundSource;

use super::melee::{MeleeAttackGoal, MeleeKind};
use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext, MobPostStepAction};
use crate::entity::warden::anger::WardenState;
use crate::entity::warden::sonic_boom;
use crate::entity::EntityData;

/// `WardenAi.ROAR_DURATION` (`WardenAi.java:50`).
const ROAR_DURATION: i64 = 84;
/// `Roar.TICKS_BEFORE_PLAYING_ROAR_SOUND` (`Roar.java:18`).
const ROAR_SOUND_DELAY: i64 = 25;

#[derive(Debug, Clone, Copy, Default)]
enum Phase {
    #[default]
    Ready,
    Roaring {
        started_at: i64,
        sound_played: bool,
    },
    Fighting,
    Sonic {
        started_at: i64,
    },
}

#[derive(Debug, Clone)]
pub(super) struct WardenCombatGoal {
    phase: Phase,
    melee: MeleeAttackGoal,
}

impl Default for WardenCombatGoal {
    fn default() -> Self {
        Self {
            phase: Phase::Ready,
            // `WardenAi.SPEED_MULTIPLIER_WHEN_FIGHTING` (`WardenAi.java:46`).
            melee: MeleeAttackGoal::new(1.2, true, MeleeKind::Warden),
        }
    }
}

fn desired_target(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    fighting: bool,
) -> Option<TargetRef> {
    let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
        return None;
    };
    let state = data.warden_state()?;
    let current = context
        .valid_target(targets)
        .filter(|target| state.anger(target.uuid()) >= WardenState::ANGRY_THRESHOLD);
    if let Some(target) = current.filter(|target| fighting || target.is_player()) {
        return Some(target.target_ref());
    }
    let selected = state.angry_target(targets);
    if let Some(target) = selected.filter(|target| {
        targets
            .facts_of(*target)
            .is_some_and(|target| target.is_player())
    }) {
        return Some(target);
    }
    current.map(|target| target.target_ref()).or(selected)
}

impl WardenCombatGoal {
    fn begin_roar(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        target: TargetRef,
    ) {
        if matches!(self.phase, Phase::Sonic { .. }) {
            if let Some((entity, _, _)) = targets.world_mut() {
                sonic_boom::recover(entity);
            }
        }
        if matches!(self.phase, Phase::Fighting | Phase::Sonic { .. }) {
            self.melee.stop(context, targets);
        }
        context.set_target(Some(target), targets);
        context.stop_navigation();
        targets.set_pose(VANILLA_ENTITY_POSE_ROARING_ID);
        self.phase = Phase::Roaring {
            started_at: targets.game_time(),
            sound_played: false,
        };
        look_at_target(context, targets);
    }
}

impl Goal for WardenCombatGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        desired_target(
            context,
            targets,
            targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_ROARING_ID,
        )
        .is_some()
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(target) = desired_target(
            context,
            targets,
            targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_ROARING_ID,
        ) {
            if context.target.target == Some(target)
                && targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_ROARING_ID
            {
                self.phase = Phase::Fighting;
                self.melee.start(context, targets);
            } else {
                self.begin_roar(context, targets, target);
            }
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if matches!(self.phase, Phase::Sonic { .. }) {
            if let Some((entity, _, _)) = targets.world_mut() {
                sonic_boom::recover(entity);
            }
        }
        if matches!(self.phase, Phase::Fighting | Phase::Sonic { .. }) {
            self.melee.stop(context, targets);
        }
        context.stop_navigation();
        if matches!(self.phase, Phase::Roaring { .. }) {
            targets.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
        }
        context.set_target(None, targets);
        self.phase = Phase::Ready;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(target) = desired_target(
            context,
            targets,
            targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_ROARING_ID,
        ) else {
            self.stop(context, targets);
            return;
        };
        if matches!(self.phase, Phase::Ready) {
            self.start(context, targets);
        } else if context.target.target != Some(target) {
            self.begin_roar(context, targets, target);
        } else if matches!(self.phase, Phase::Roaring { started_at, .. } if started_at < targets.game_time())
            && targets.body_entity().pose_id() != VANILLA_ENTITY_POSE_ROARING_ID
        {
            self.phase = Phase::Fighting;
            self.melee.start(context, targets);
        }
        if let Phase::Roaring {
            started_at,
            sound_played,
        } = &mut self.phase
        {
            context.stop_navigation();
            look_at_target(context, targets);
            let elapsed = targets.game_time() - *started_at;
            if elapsed >= ROAR_SOUND_DELAY && !*sound_played {
                *sound_played = true;
                if !targets.body_entity().is_silent() {
                    // `Roar.tick`.
                    targets.play_body_sound(
                        "minecraft:entity.warden.roar",
                        SoundSource::Hostile,
                        3.0,
                        1.0,
                    );
                }
            }
            if elapsed < ROAR_DURATION {
                return;
            }
            targets.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
            if let Some((entity, _, _)) = targets.world_mut() {
                sonic_boom::target_set(entity);
            }
            self.phase = Phase::Fighting;
            self.melee.start(context, targets);
            self.melee.tick(context, targets);
            return;
        }
        if let Phase::Sonic { started_at } = self.phase {
            look_at_target(context, targets);
            if targets.game_time() - started_at >= sonic_boom::DURATION {
                if let Some((entity, _, _)) = targets.world_mut() {
                    sonic_boom::recover(entity);
                }
                self.phase = Phase::Fighting;
                self.melee.start(context, targets);
                return;
            }
            if matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                if data.warden_state().is_some_and(|state| state.sonic.waiting_for_boom()))
            {
                context.post_step.push(MobPostStepAction::WardenSonicFire {
                    target_id: target.entity_id,
                });
            }
            return;
        }
        if matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.warden_state().is_some_and(|state| state.sonic.ready()))
            && context.valid_target(targets).is_some_and(|target| {
                sonic_boom::within_range(context.position(), target.position())
            })
        {
            look_at_target(context, targets);
            self.phase = Phase::Sonic {
                started_at: targets.game_time(),
            };
            if let Some((entity, store, _)) = targets.world_mut() {
                sonic_boom::charge(entity, store);
            }
            return;
        }
        self.melee.tick(context, targets);
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

fn look_at_target(context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) {
    if let Some(target) = context.valid_target(targets) {
        let position = target.position();
        context.set_look_at_with_limits(position.x, target.eye_y(), position.z, 30.0, 30.0);
    }
}
