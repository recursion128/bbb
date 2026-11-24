//! `Frog`'s brain as its provider builds it.

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_FROGSPAWN_ID;

use crate::entity::ai::brain::{ordered_try_all, random_stroll_swim, trigger_if_in_water};

use crate::entity::ai::{
    animal_make_love, animal_panic, count_down_cooldown_ticks, croak, follow_temptation,
    long_jump_mid_jump, long_jump_to_preferred_block, look_at_target_sink, move_to_target_sink,
    random_stroll, run_one, set_entity_look_target_sometimes_of, set_walk_target_from_look_target,
    shoot_tongue, start_attacking, stop_attacking_if_target_invalid, trigger_if_on_ground,
    try_find_land, try_find_land_near_water, try_lay_spawn_on_fluid_near_land, Activity, Brain,
    BrainEntityFacts, BrainPacked, BrainWorld, MemoryModuleType, MemoryStatus, SensorType,
};

/// `FrogAi.SPEED_MULTIPLIER_WHEN_PANICKING` (`FrogAi.java:53`), the
/// `new AnimalPanic<>(2.0F)` the CORE activity opens with (`:76`).
const SPEED_MULTIPLIER_WHEN_PANICKING: f32 = 2.0;

/// `FrogAi.SPEED_MULTIPLIER_WHEN_TEMPTED` (`FrogAi.java:61`), the
/// `new FollowTemptation(s -> 1.25F)` both tempting activities register
/// (`:91`, `:118`).
const SPEED_MULTIPLIER_WHEN_TEMPTED: f32 = 1.25;

/// `FrogAi.SPEED_MULTIPLIER_WHEN_IDLING` (`FrogAi.java:54`), the speed every
/// `RandomStroll.stroll` and `SetWalkTargetFromLookTarget.create` in the two
/// `RunOne` packages is built with (`:99-100`, `:155-156`).
const SPEED_MULTIPLIER_WHEN_IDLING: f32 = 1.0;

/// `FrogAi.TIME_BETWEEN_LONG_JUMPS` (`FrogAi.java:57`), the `UniformInt` both
/// long-jump behaviours sample their cooldown from (`:173`, `:177`, `:64`).
const TIME_BETWEEN_LONG_JUMPS_MIN: i32 = 100;
const TIME_BETWEEN_LONG_JUMPS_MAX: i32 = 140;

/// `FrogAi.MAX_LONG_JUMP_HEIGHT`/`MAX_LONG_JUMP_WIDTH`/`MAX_JUMP_VELOCITY_MULTIPLIER`
/// (`FrogAi.java:58-60`), the three bounds
/// `LongJumpToPreferredBlock`'s constructor takes (`:178-180`).
const MAX_LONG_JUMP_HEIGHT: i32 = 2;
const MAX_LONG_JUMP_WIDTH: i32 = 4;
const MAX_JUMP_VELOCITY_MULTIPLIER: f32 = 3.571_428_8;

/// `LongJumpToPreferredBlock`'s `preferredBlocksChance` (`FrogAi.java:183`).
const PREFERRED_BLOCKS_CHANCE: f32 = 0.5;

/// `SetEntityLookTargetSometimes.create(EntityType.PLAYER, 6.0F,
/// UniformInt.of(30, 60))` (`FrogAi.java:89`, `:117`, `:147`).
const LOOK_TARGET_MAX_DIST: f32 = 6.0;
const LOOK_TARGET_INTERVAL_MIN: i32 = 30;
const LOOK_TARGET_INTERVAL_MAX: i32 = 60;

/// `EntityType.PLAYER`'s registry path, the type the three look-target
/// behaviours filter the visible list with (`FrogAi.java:89`).
const PLAYER_TYPE: &str = "minecraft:player";

/// `EntityType.FROG`'s registry path, the `partnerType`
/// `new AnimalMakeLove(EntityType.FROG)` is built with (`FrogAi.java:90`).
const FROG_TYPE: &str = "minecraft:frog";

/// `SetWalkTargetFromLookTarget.create(1.0F, 3)` (`FrogAi.java:100`, `:131`,
/// `:156`).
const LOOK_TARGET_CLOSE_ENOUGH: i32 = 3;

/// The four `RunOne` children's weights (`FrogAi.java:99-102`, `:155-158`):
/// IDLE weights the croak above the two strolls, LAY_SPAWN the two strolls
/// above them.
const IDLE_STROLL_WEIGHT: i32 = 1;
const IDLE_CROAK_WEIGHT: i32 = 3;
const IDLE_ON_GROUND_WEIGHT: i32 = 2;
const LAY_SPAWN_STROLL_WEIGHT: i32 = 2;
const LAY_SPAWN_LOOK_WEIGHT: i32 = 1;
const LAY_SPAWN_CROAK_WEIGHT: i32 = 2;
const LAY_SPAWN_ON_GROUND_WEIGHT: i32 = 1;

/// `SoundEvents.FROG_STEP` (`FrogAi.java:173`), the mid-jump behaviour's
/// landing sound (`LongJumpMidJump.java:36`).
const FROG_STEP_SOUND: &str = "minecraft:entity.frog.step";

/// `SoundEvents.FROG_LONG_JUMP` (`FrogAi.java:181`), the sound the launching
/// tick plays (`LongJumpToRandomPos.java:146`).
const FROG_LONG_JUMP_SOUND: &str = "minecraft:entity.frog.long_jump";

/// `Blocks.FROGSPAWN.defaultBlockState()`, which
/// `TryLaySpawnOnFluidNearLand` writes.
fn frogspawn_state_id() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_FROGSPAWN_ID)
        .expect("the built-in 26.1 frogspawn block has a default state")
}

/// The `#minecraft:frog_food` tag, which `Frog.canEat` tests entity types
/// against (`Frog.java:319`) and `FrogAi.getTemptations` tests the item in a
/// player's hand against (`FrogAi.java:235-237`).
pub const FROG_FOOD_TAG: &str = "minecraft:frog_food";

/// The memory slots `Frog.BRAIN_PROVIDER` registers.
///
/// Vanilla reaches them through the five sensors' `requires()` and the
/// `FrogAi` packages' `getRequiredMemories()` (`Frog`,
/// `FrogAi`). `AnimalMakeLove` is the one frog package still
/// unwired, so `BREED_TARGET` is registered and left empty; the world's
/// `brain_debug::FROG_MEMORY_MODULES` names the same set and a test pins the
/// two equal.
pub(crate) const FROG_MEMORY_MODULES: &[MemoryModuleType] = &[
    MemoryModuleType::NearestLivingEntities,
    MemoryModuleType::NearestVisibleLivingEntities,
    MemoryModuleType::HurtBy,
    MemoryModuleType::HurtByEntity,
    MemoryModuleType::NearestAttackable,
    MemoryModuleType::TemptingPlayer,
    MemoryModuleType::IsTempted,
    MemoryModuleType::IsInWater,
    MemoryModuleType::WalkTarget,
    MemoryModuleType::LookTarget,
    MemoryModuleType::AttackTarget,
    MemoryModuleType::BreedTarget,
    MemoryModuleType::Path,
    MemoryModuleType::CantReachWalkTargetSince,
    MemoryModuleType::UnreachableTongueTargets,
    MemoryModuleType::TemptationCooldownTicks,
    MemoryModuleType::LongJumpCooldownTicks,
    MemoryModuleType::LongJumpMidJump,
    MemoryModuleType::IsPanicking,
    MemoryModuleType::IsPregnant,
];

/// `Frog.canEat`: slime food must have actual size one.
pub fn can_eat(world: &dyn BrainWorld, facts: &BrainEntityFacts) -> bool {
    if facts.slime_size.is_some_and(|size| size != 1) {
        return false;
    }
    world.entity_type_in_tag(facts.entity_resource_id, FROG_FOOD_TAG)
}

/// `Frog.BRAIN_PROVIDER.makeBrain(body, packedBrain)`
///
/// The sensor order is the provider's `List.of` order:
/// `NEAREST_LIVING_ENTITIES`, `HURT_BY`, `FROG_ATTACKABLES`,
/// `FROG_TEMPTATIONS`, `IS_IN_WATER`.
pub(crate) fn frog_brain(packed: &BrainPacked) -> Brain {
    let mut brain = Brain::new();
    for memory_type in FROG_MEMORY_MODULES {
        brain.register_memory(*memory_type);
    }
    brain.register_sensor(SensorType::NearestLivingEntities);
    brain.register_sensor(SensorType::HurtBy);
    brain.register_sensor(SensorType::FrogAttackables);
    brain.register_sensor(SensorType::FrogTemptations);
    brain.register_sensor(SensorType::IsInWater);
    add_activities(&mut brain);
    // `Brain`'s constructor applies the packed memories after the activities
    brain.apply_packed(packed);
    brain
}

/// `FrogAi.getActivities` with the behaviours this
/// runtime has.
fn add_activities(brain: &mut Brain) {
    // `FrogAi.initCoreActivity`, whose
    // `ActivityData.create` numbers the five behaviours 0..4
    brain.add_activity(
        Activity::CORE,
        [
            (0, animal_panic(SPEED_MULTIPLIER_WHEN_PANICKING)),
            (1, look_at_target_sink()),
            (2, move_to_target_sink()),
            (
                3,
                count_down_cooldown_ticks(MemoryModuleType::TemptationCooldownTicks),
            ),
            (
                4,
                count_down_cooldown_ticks(MemoryModuleType::LongJumpCooldownTicks),
            ),
        ],
        &[],
        &[],
    );
    // `FrogAi.initIdleActivity`: priority 0 holds the
    // look target and the breeding pair in that order.
    brain.add_activity(
        Activity::IDLE,
        [
            (0, look_target()),
            (0, animal_make_love(FROG_TYPE)),
            (1, follow_temptation(SPEED_MULTIPLIER_WHEN_TEMPTED)),
            (2, start_attacking()),
            (3, try_find_land(6, SPEED_MULTIPLIER_WHEN_IDLING)),
            (
                4,
                run_one(
                    vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
                    [
                        (
                            IDLE_STROLL_WEIGHT,
                            random_stroll(SPEED_MULTIPLIER_WHEN_IDLING),
                        ),
                        (
                            IDLE_STROLL_WEIGHT,
                            set_walk_target_from_look_target(
                                SPEED_MULTIPLIER_WHEN_IDLING,
                                LOOK_TARGET_CLOSE_ENOUGH,
                            ),
                        ),
                        (IDLE_CROAK_WEIGHT, croak()),
                        (IDLE_ON_GROUND_WEIGHT, trigger_if_on_ground()),
                    ],
                ),
            ),
        ],
        &[
            (MemoryModuleType::LongJumpMidJump, MemoryStatus::ValueAbsent),
            (MemoryModuleType::IsInWater, MemoryStatus::ValueAbsent),
        ],
        &[],
    );
    // `FrogAi.initSwimActivity`.
    brain.add_activity(
        Activity::SWIM,
        [
            (0, look_target()),
            (1, follow_temptation(SPEED_MULTIPLIER_WHEN_TEMPTED)),
            (2, start_attacking()),
            (3, try_find_land(8, 1.5)),
            (
                5,
                ordered_try_all(
                    vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
                    [
                        (1, random_stroll_swim(0.75)),
                        (1, random_stroll(1.0)),
                        (
                            1,
                            set_walk_target_from_look_target(1.0, LOOK_TARGET_CLOSE_ENOUGH),
                        ),
                        (5, trigger_if_in_water()),
                    ],
                ),
            ),
        ],
        &[
            (MemoryModuleType::LongJumpMidJump, MemoryStatus::ValueAbsent),
            (MemoryModuleType::IsInWater, MemoryStatus::ValuePresent),
        ],
        &[],
    );
    // `FrogAi.initLaySpawnActivity`.
    brain.add_activity(
        Activity::LAY_SPAWN,
        [
            (0, look_target()),
            (1, start_attacking()),
            (2, try_find_land_near_water(8, SPEED_MULTIPLIER_WHEN_IDLING)),
            (3, try_lay_spawn_on_fluid_near_land(frogspawn_state_id())),
            (
                4,
                run_one(
                    Vec::new(),
                    [
                        (
                            LAY_SPAWN_STROLL_WEIGHT,
                            random_stroll(SPEED_MULTIPLIER_WHEN_IDLING),
                        ),
                        (
                            LAY_SPAWN_LOOK_WEIGHT,
                            set_walk_target_from_look_target(
                                SPEED_MULTIPLIER_WHEN_IDLING,
                                LOOK_TARGET_CLOSE_ENOUGH,
                            ),
                        ),
                        (LAY_SPAWN_CROAK_WEIGHT, croak()),
                        (LAY_SPAWN_ON_GROUND_WEIGHT, trigger_if_on_ground()),
                    ],
                ),
            ),
        ],
        &[
            (MemoryModuleType::LongJumpMidJump, MemoryStatus::ValueAbsent),
            (MemoryModuleType::IsPregnant, MemoryStatus::ValuePresent),
        ],
        &[],
    );
    // `FrogAi.initJumpActivity`, whose two pairs are
    // priorities 0 and 1 of the same activity.
    brain.add_activity(
        Activity::LONG_JUMP,
        [
            (
                0,
                long_jump_mid_jump(
                    TIME_BETWEEN_LONG_JUMPS_MIN,
                    TIME_BETWEEN_LONG_JUMPS_MAX,
                    FROG_STEP_SOUND,
                ),
            ),
            (
                1,
                long_jump_to_preferred_block(
                    TIME_BETWEEN_LONG_JUMPS_MIN,
                    TIME_BETWEEN_LONG_JUMPS_MAX,
                    MAX_LONG_JUMP_HEIGHT,
                    MAX_LONG_JUMP_WIDTH,
                    MAX_JUMP_VELOCITY_MULTIPLIER,
                    FROG_LONG_JUMP_SOUND,
                    PREFERRED_BLOCKS_CHANCE,
                ),
            ),
        ],
        &[
            (MemoryModuleType::TemptingPlayer, MemoryStatus::ValueAbsent),
            (MemoryModuleType::BreedTarget, MemoryStatus::ValueAbsent),
            (
                MemoryModuleType::LongJumpCooldownTicks,
                MemoryStatus::ValueAbsent,
            ),
            (MemoryModuleType::IsInWater, MemoryStatus::ValueAbsent),
        ],
        &[],
    );
    // `FrogAi.initTongueActivity`:
    // `ActivityData.create(Activity.TONGUE, 0, list, ATTACK_TARGET)` numbers
    // the two behaviours 0 and 1 and makes the memory both the condition and
    // the one erased when the activity stops.
    brain.add_activity(
        Activity::TONGUE,
        [(0, stop_attacking_if_target_invalid()), (1, shoot_tongue())],
        &[(MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent)],
        &[MemoryModuleType::AttackTarget],
    );
}

/// `SetEntityLookTargetSometimes.create(EntityType.PLAYER, 6.0F,
/// UniformInt.of(30, 60))`.
fn look_target() -> crate::entity::ai::Behavior {
    set_entity_look_target_sometimes_of(
        Some(PLAYER_TYPE),
        LOOK_TARGET_MAX_DIST,
        LOOK_TARGET_INTERVAL_MIN,
        LOOK_TARGET_INTERVAL_MAX,
    )
}

/// `FrogAi.updateActivity`, which
/// `Frog.customServerAiStep` runs right after the brain tick
pub(crate) fn update_activity(brain: &mut Brain) {
    brain.set_active_activity_to_first_valid(&[
        Activity::TONGUE,
        Activity::LAY_SPAWN,
        Activity::LONG_JUMP,
        Activity::SWIM,
        Activity::IDLE,
    ]);
}

#[cfg(test)]
mod tests;
