//! `Croak` and the bare `BehaviorBuilder.triggerIf` predicate, the two frog
//! children `FrogAi`'s `RunOne` packages weight against the strolls

use super::super::effect::BrainWorldEffect;
use super::super::memory::{MemoryModuleType, MemoryStatus};
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::POSE_STANDING;

/// `Croak.CROAK_TICKS` (`Croak.java:11`).
const CROAK_TICKS: i32 = 60;
/// `Croak.TIME_OUT_DURATION` (`Croak.java:12`).
const CROAK_TIME_OUT: i32 = 100;
/// `Pose.CROAKING.getId()`, which `Croak.start` writes
/// (`Croak.java:29`).
const POSE_CROAKING: i32 = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROAKING_ID;

/// `Croak`, which IDLE and LAY_SPAWN both register
/// through their own `RunOne`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Croak {
    /// `Croak.croakCounter`.
    croak_counter: i32,
}

impl BehaviorHooks for Croak {
    // `Croak.checkExtraStartConditions`.
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        tick.world().body().pose_id == POSE_STANDING
    }

    // `Croak.canStillUse`.
    fn can_still_use(
        &self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        self.croak_counter < CROAK_TICKS
    }

    // `Croak.start`.
    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        // `body.isInLiquid()`.
        if tick.world().body().in_water || tick.world().body().in_lava {
            return;
        }
        tick.world_mut().queue_effect(BrainWorldEffect::SetPose {
            pose_id: POSE_CROAKING,
        });
        self.croak_counter = 0;
    }

    // `Croak.stop`.
    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.world_mut().queue_effect(BrainWorldEffect::SetPose {
            pose_id: POSE_STANDING,
        });
    }

    // `Croak.tick`.
    fn tick(
        &mut self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        self.croak_counter += 1;
    }

    fn debug_string(&self) -> &'static str {
        "Croak"
    }
}

/// `new Croak()`, whose group asks for
/// an absent `WALK_TARGET` and whose duration is the constructor's
/// single `100`.
pub fn croak() -> Behavior {
    Behavior::with_duration(
        vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
        CROAK_TIME_OUT,
        CROAK_TIME_OUT,
        Croak::default(),
    )
}

/// `BehaviorBuilder.triggerIf(Entity::onGround)`,
/// the `OneShot` whose whole body is that predicate
/// and whose group asks for no memory at all.
///
/// Vanilla's `debugString` for a bare `triggerIf` is the lambda's own
/// generated name, so this publishes the
/// predicate instead.
#[derive(Debug, Clone, Copy, Default)]
pub struct TriggerIfOnGround;

impl BehaviorHooks for TriggerIfOnGround {
    // `triggerIf`'s `i.point((level, body, timestamp) ->
    // predicate.test(body))`.
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        tick.world().body().on_ground
    }

    fn debug_string(&self) -> &'static str {
        "TriggerIfOnGround"
    }
}

/// `BehaviorBuilder.triggerIf(Entity::onGround)`.
pub fn trigger_if_on_ground() -> Behavior {
    Behavior::one_shot(Vec::new(), TriggerIfOnGround)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TriggerIfInWater;

impl BehaviorHooks for TriggerIfInWater {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        tick.world().body().in_water
    }

    fn debug_string(&self) -> &'static str {
        "TriggerIfInWater"
    }
}

/// `BehaviorBuilder.triggerIf(Entity::isInWater)`.
pub fn trigger_if_in_water() -> Behavior {
    Behavior::one_shot(Vec::new(), TriggerIfInWater)
}

#[cfg(test)]
mod tests;
