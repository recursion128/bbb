//! Warden disturbance pursuit and sniffing on its retained awareness state.

use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DIGGING_ID, VANILLA_ENTITY_POSE_EMERGING_ID,
    VANILLA_ENTITY_POSE_ROARING_ID, VANILLA_ENTITY_POSE_SNIFFING_ID,
};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use super::target::TargetView;
use super::{Goal, MobAiContext, MobPostStepAction};
use crate::entity::warden::anger::WardenState;
use crate::entity::warden::awareness::nearest_attackable;
use crate::entity::EntityData;

/// `WardenAi.SNIFFING_DURATION` (`WardenAi.java:51`).
const SNIFF_DURATION: i64 = 84;
/// `Sensor.DEFAULT_SCAN_RATE` (`Sensor.java:14`).
const SCAN_INTERVAL: i64 = 20;

#[derive(Debug, Clone, Copy, Default)]
enum Phase {
    #[default]
    Ready,
    Investigating {
        location: BlockPos,
    },
    Sniffing {
        started_at: i64,
    },
}

#[derive(Debug, Clone, Default)]
pub(super) struct WardenAwarenessGoal {
    phase: Phase,
    next_scan: i64,
    next_path: i64,
}

fn state<'a>(targets: &'a TargetView<'_, '_, '_>) -> Option<&'a WardenState> {
    let EntityData::GeneralLiving(data) = targets.body_entity().data() else {
        return None;
    };
    data.warden_state()
}

fn eligible(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
    !matches!(
        targets.body_entity().pose_id(),
        VANILLA_ENTITY_POSE_ROARING_ID
            | VANILLA_ENTITY_POSE_EMERGING_ID
            | VANILLA_ENTITY_POSE_DIGGING_ID
    ) && context.valid_target(targets).is_none()
        && state(targets).is_some_and(|state| state.angry_target(targets).is_none())
}

fn center(location: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(location.x) + 0.5,
        y: f64::from(location.y) + 0.5,
        z: f64::from(location.z) + 0.5,
    }
}

impl WardenAwarenessGoal {
    fn redirect(&mut self, context: &mut MobAiContext, location: BlockPos, now: i64) {
        context.stop_navigation();
        self.phase = Phase::Investigating { location };
        // TryMoveToPosition retains the old path on failure, so stop it first.
        self.next_path = now + 1;
    }

    fn investigate(
        &mut self,
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        location: BlockPos,
    ) {
        let destination = center(location);
        context.set_look_at(destination.x, destination.y, destination.z);
        if context.position().distance_to_sqr(destination) <= 4.0 {
            context.stop_navigation();
        } else if context.navigation_done() && targets.game_time() >= self.next_path {
            // `WardenAi.initInvestigateActivity`.
            context.try_navigate_to_position(destination, 2, 0.7);
            self.next_path = targets.game_time() + SCAN_INTERVAL;
        }
    }
}

impl Goal for WardenAwarenessGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !eligible(context, targets) {
            return false;
        }
        let awareness = &state(targets).expect("eligible warden").awareness;
        if awareness.disturbance().is_some() {
            return true;
        }
        if targets.game_time() < self.next_scan {
            return false;
        }
        if awareness.is_sniffing() {
            return true;
        }
        if !awareness.can_sniff() {
            return false;
        }
        self.next_scan = targets.game_time() + SCAN_INTERVAL;
        nearest_attackable(targets).is_some()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !eligible(context, targets) {
            return false;
        }
        let awareness = &state(targets).expect("eligible warden").awareness;
        match self.phase {
            Phase::Ready => false,
            Phase::Investigating { .. } => awareness.disturbance().is_some(),
            Phase::Sniffing { started_at } => {
                awareness.disturbance().is_none()
                    && targets.game_time() - started_at < SNIFF_DURATION
                    && (awareness.is_sniffing() || targets.game_time() == started_at)
            }
        }
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((disturbance, is_sniffing)) = state(targets)
            .map(|state| (state.awareness.disturbance(), state.awareness.is_sniffing()))
        else {
            return;
        };
        if let Some(location) = disturbance {
            self.redirect(context, location, targets.game_time());
            self.investigate(context, targets, location);
            return;
        }
        if !is_sniffing {
            // `TryToSniff.SNIFF_COOLDOWN` (`TryToSniff.java:13`).
            let cooldown = i64::from(100 + context.next_int(101));
            if let Some((entity, _, _)) = targets.world_mut() {
                crate::entity::warden::awareness::begin_sniff(entity, cooldown);
            }
        }
        context.stop_navigation();
        targets.set_pose(VANILLA_ENTITY_POSE_SNIFFING_ID);
        if !targets.body_entity().is_silent() {
            // `Sniffing.start`.
            targets.play_body_sound(
                "minecraft:entity.warden.sniff",
                SoundSource::Hostile,
                5.0,
                1.0,
            );
        }
        self.phase = Phase::Sniffing {
            started_at: targets.game_time(),
        };
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if matches!(self.phase, Phase::Sniffing { .. }) {
            context.post_step.push(MobPostStepAction::WardenSniffEnd);
            self.next_scan = targets.game_time() + SCAN_INTERVAL;
        }
        context.stop_navigation();
        self.phase = Phase::Ready;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if !eligible(context, targets) {
            self.stop(context, targets);
            return;
        }
        let (disturbance, is_sniffing) = {
            let awareness = &state(targets).expect("eligible warden").awareness;
            (awareness.disturbance(), awareness.is_sniffing())
        };
        if let Some(location) = disturbance {
            if !matches!(self.phase, Phase::Investigating { location: previous } if previous == location)
            {
                self.stop(context, targets);
                self.redirect(context, location, targets.game_time());
            }
            self.investigate(context, targets, location);
        } else if let Phase::Sniffing { started_at } = self.phase {
            context.stop_navigation();
            if targets.game_time() - started_at >= SNIFF_DURATION
                || (!is_sniffing && targets.game_time() != started_at)
            {
                self.stop(context, targets);
            }
        } else {
            self.stop(context, targets);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
