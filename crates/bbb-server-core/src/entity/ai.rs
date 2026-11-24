mod brain;
mod goal;
pub(crate) mod mob;

pub use brain::{
    animal_make_love, animal_panic, count_down_cooldown_ticks, croak, do_nothing,
    follow_temptation, long_jump_mid_jump, long_jump_to_preferred_block, look_at_target_sink,
    melee_attack, move_to_target_sink, random_stroll, run_one, set_entity_look_target_sometimes,
    set_entity_look_target_sometimes_of, set_walk_target_from_attack_target_if_target_out_of_reach,
    set_walk_target_from_look_target, shoot_tongue, start_attacking, start_attacking_when_active,
    stop_attacking_if_target_invalid, stop_attacking_if_target_invalid_unless_in, swim,
    trigger_if_on_ground, try_find_land, try_find_land_near_water,
    try_lay_spawn_on_fluid_near_land, Activity, AnimalMakeLove, AnimalPanic, Behavior,
    BehaviorHooks, BehaviorStatus, Brain, BrainBlockFacts, BrainBreedFacts, BrainCodecError,
    BrainDamageSource, BrainEntityFacts, BrainPacked, BrainPlayer, BrainTick, BrainTickContext,
    BrainWorld, BrainWorldEffect, CountDownCooldownTicks, Croak, DoNothing, EntityTargetingTest,
    FollowTemptation, LongJumpMidJump, LongJumpToPreferredBlock, LookAtTargetSink, MeleeAttack,
    MemoryGlobalPos, MemoryModuleType, MemoryPath, MemoryPositionTracker, MemoryStatus,
    MemoryValue, MemoryValueKind, MemoryWalkTarget, MoveToTargetSink, PackedMemory, RandomStroll,
    RunOne, SensorType, SetEntityLookTargetSometimes,
    SetWalkTargetFromAttackTargetIfTargetOutOfReach, SetWalkTargetFromLookTarget, ShootTongue,
    StartAttacking, StartAttackingPredicate, StopAttackingIfTargetInvalid, Swim, TriggerIfOnGround,
    TryFindLand, TryFindLandNearWater, TryLaySpawnOnFluidNearLand, VillagerHeldItem,
    DEFAULT_SCAN_RATE, MOB_ATTACK_DAMAGE_TYPE,
};
pub use brain::{FarmCell, VillagerJobFacts, VillagerPlayFacts};
pub(crate) use goal::{Goal, GoalControl, GoalControls};

/// `instanceof Enemy`, shared with the canonical target predicate owner.
#[must_use]
pub fn entity_type_is_enemy(entity_type_id: i32) -> bool {
    mob::target::selector::is_enemy(entity_type_id)
}
