//! `Creaking`'s brain as its provider builds it.

use crate::entity::ai::{
    do_nothing, look_at_target_sink, melee_attack, move_to_target_sink, random_stroll, run_one,
    set_entity_look_target_sometimes, set_walk_target_from_attack_target_if_target_out_of_reach,
    set_walk_target_from_look_target, start_attacking_when_active,
    stop_attacking_if_target_invalid_unless_in, swim, Activity, Brain, BrainPacked,
    MemoryModuleType, MemoryStatus, SensorType,
};

/// The memory slots `Creaking.BRAIN_PROVIDER` registers.
///
/// Vanilla reaches them through the two sensors' `requires()`
/// and the
/// `CreakingAi` packages' `getRequiredMemories()`.
/// Every memory a package names is listed even when this runtime does not run
/// the behaviour yet, so the registered set is the provider's; a test pins it
/// against the world's `brain_debug` projection for the same type.
pub(crate) const CREAKING_MEMORY_MODULES: &[MemoryModuleType] = &[
    MemoryModuleType::NearestLivingEntities,
    MemoryModuleType::NearestVisibleLivingEntities,
    MemoryModuleType::NearestPlayers,
    MemoryModuleType::NearestVisiblePlayer,
    MemoryModuleType::NearestVisibleAttackablePlayer,
    MemoryModuleType::NearestVisibleAttackablePlayers,
    MemoryModuleType::WalkTarget,
    MemoryModuleType::LookTarget,
    MemoryModuleType::AttackTarget,
    MemoryModuleType::AttackCoolingDown,
    MemoryModuleType::Path,
    MemoryModuleType::CantReachWalkTargetSince,
];

/// `CreakingAi`'s `ATTACK_INTERVAL`, which the creaking's
/// FIGHT hands `MeleeAttack.create`.
pub(crate) const CREAKING_ATTACK_INTERVAL: i32 = 40;
/// `Creaking.SPEED_MULTIPLIER_WHEN_IDLING` (`Creaking.java:70`), the speed
/// `CreakingAi`'s `RandomStroll` and `SetWalkTargetFromLookTarget` share
/// (`CreakingAi.java:50`).
const SPEED_MULTIPLIER_WHEN_IDLING: f32 = 0.3;
/// `CreakingAi`'s two `RandomStroll` weights and the `DoNothing` weight
/// (`CreakingAi.java:50`).
const STROLL_WEIGHT: i32 = 2;
const LOOK_TARGET_STROLL_WEIGHT: i32 = 2;
const DO_NOTHING_WEIGHT: i32 = 1;
/// `SetEntityLookTargetSometimes.create(8.0F, UniformInt.of(30, 60))`
/// (`CreakingAi.java:47`).
const LOOK_TARGET_MAX_DIST: f32 = 8.0;
const LOOK_TARGET_INTERVAL_MIN: i32 = 30;
const LOOK_TARGET_INTERVAL_MAX: i32 = 60;
/// `new DoNothing(30, 60)` (`CreakingAi.java:50`).
const DO_NOTHING_MIN_TICKS: i32 = 30;
const DO_NOTHING_MAX_TICKS: i32 = 60;
/// `new MoveToTargetSink()`'s `closeEnoughDistance` for the look-target stroll
/// (`SetWalkTargetFromLookTarget.create(0.3F, 3)`, `CreakingAi.java:50`).
const LOOK_TARGET_CLOSE_ENOUGH: i32 = 3;

/// `Creaking.BRAIN_PROVIDER.makeBrain(body, packedBrain)`
pub(crate) fn creaking_brain(packed: &BrainPacked) -> Brain {
    let mut brain = Brain::new();
    for memory_type in CREAKING_MEMORY_MODULES {
        brain.register_memory(*memory_type);
    }
    brain.register_sensor(SensorType::NearestLivingEntities);
    brain.register_sensor(SensorType::NearestPlayers);
    add_activities(&mut brain);
    // `Brain`'s constructor applies the packed memories after the activities
    brain.apply_packed(packed);
    brain
}

/// `CreakingAi.getActivities` with the behaviours
/// this runtime has.
///
/// `ActivityData.create` numbers a behaviour from the priority it is given
/// so a behaviour this runtime does not run leaves
/// its index empty rather than shifting the ones after it. CORE starts at 0,
/// IDLE and FIGHT both at 10, which is
/// what keeps the two sinks ahead of every activity behaviour.
fn add_activities(brain: &mut Brain) {
    // `CreakingAi.initCoreActivity`: the list is
    // `Swim(0.8F)` at index 0 -- with the anonymous override's `canMove` term
    // -- `LookAtTargetSink(45, 90)` at 1 and `MoveToTargetSink()` at
    // 2. The list is numbered from 0, so the two sinks keep their indices.
    brain.add_activity(
        Activity::CORE,
        [
            (0, swim(0.8, true)),
            (1, look_at_target_sink()),
            (2, move_to_target_sink()),
        ],
        &[],
        &[],
    );
    // `CreakingAi.initIdleActivity` (`CreakingAi`, numbered from
    // 10): `StartAttacking.create(Creaking::isActive, ...)` at 10,
    // `SetEntityLookTargetSometimes(8.0F, UniformInt.of(30, 60))` at 11 and the
    // weighted `RunOne` at 12.
    brain.add_activity(
        Activity::IDLE,
        [
            (10, start_attacking_when_active()),
            (
                11,
                set_entity_look_target_sometimes(
                    LOOK_TARGET_MAX_DIST,
                    LOOK_TARGET_INTERVAL_MIN,
                    LOOK_TARGET_INTERVAL_MAX,
                ),
            ),
            (
                12,
                run_one(
                    Vec::new(),
                    [
                        (STROLL_WEIGHT, random_stroll(SPEED_MULTIPLIER_WHEN_IDLING)),
                        (
                            LOOK_TARGET_STROLL_WEIGHT,
                            set_walk_target_from_look_target(
                                SPEED_MULTIPLIER_WHEN_IDLING,
                                LOOK_TARGET_CLOSE_ENOUGH,
                            ),
                        ),
                        (
                            DO_NOTHING_WEIGHT,
                            do_nothing(DO_NOTHING_MIN_TICKS, DO_NOTHING_MAX_TICKS),
                        ),
                    ],
                ),
            ),
        ],
        &[],
        &[],
    );
    // `CreakingAi.initFightActivity` (`CreakingAi`, numbered from
    // 10): `SetWalkTargetFromAttackTargetIfTargetOutOfReach(1.0F)` at 10,
    // `MeleeAttack.create(Creaking::canMove, 40)` at 11 and
    // `StopAttackingIfTargetInvalid.create(isAttackTargetStillReachable)` at
    // 12; the activity's condition is the `ATTACK_TARGET` `ActivityData.create`
    // turns into both the requirement and the memory erased when the activity
    // stops.
    brain.add_activity(
        Activity::FIGHT,
        [
            (
                10,
                set_walk_target_from_attack_target_if_target_out_of_reach(1.0),
            ),
            (11, melee_attack(CREAKING_ATTACK_INTERVAL, true)),
            // `CreakingAi.isAttackTargetStillReachable`
            // is the activity's `stopAttackingWhen`.
            (
                12,
                stop_attacking_if_target_invalid_unless_in(Some(
                    MemoryModuleType::NearestVisibleAttackablePlayers,
                )),
            ),
        ],
        &[(MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent)],
        &[MemoryModuleType::AttackTarget],
    );
}

/// `CreakingAi.updateActivity`, which
/// `Creaking.customServerAiStep` runs right after the brain tick
///
/// A creaking that cannot move takes the brain's default activity; one that can
/// picks the first of `FIGHT` and `IDLE` whose condition is met.
pub(crate) fn update_activity(brain: &mut Brain, can_move: bool) {
    if !can_move {
        brain.use_default_activity();
    } else {
        brain.set_active_activity_to_first_valid(&[Activity::FIGHT, Activity::IDLE]);
    }
}

#[cfg(test)]
mod tests;
