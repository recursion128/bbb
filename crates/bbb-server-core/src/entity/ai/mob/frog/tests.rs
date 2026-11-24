use super::*;
use crate::entity::ai::brain::tests::test_world;
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{BrainDamageSource, MemoryValue};

fn brain() -> Brain {
    frog_brain(&BrainPacked::empty())
}

fn brain_with(memories: Vec<crate::entity::ai::PackedMemory>) -> Brain {
    let packed = BrainPacked { memories };
    frog_brain(&packed)
}

#[test]
fn the_frog_provider_registers_vanillas_memories() {
    let brain = brain();
    let mut registered = Vec::new();
    brain.for_each_memory(|memory_type, _, _| registered.push(memory_type.resource_id()));
    registered.sort_unstable();
    let mut expected: Vec<&str> = FROG_MEMORY_MODULES
        .iter()
        .map(|memory_type| memory_type.resource_id())
        .collect();
    expected.sort_unstable();
    assert_eq!(registered, expected);
    assert_eq!(registered.len(), 20);
}

#[test]
fn the_frog_provider_registers_vanillas_behaviors_at_their_priorities() {
    let brain = brain();
    // `FrogAi`. `ActivityData.create` numbers the `Pair.of`
    // priorities the provider writes down, and
    // `FrogAi` numbers each package's pairs from its own base: CORE and TONGUE
    // from 0, and IDLE, SWIM and LAY_SPAWN from the pairs they name.
    //
    // Only the behaviours this runtime has are registered, so a package's
    // empty slots are absent rather than filled.
    assert_eq!(
        brain.behavior_priorities(Activity::CORE),
        vec![
            (0, "AnimalPanic"),
            (1, "LookAtTargetSink"),
            (2, "MoveToTargetSink"),
            (3, "CountDownCooldownTicks"),
            (4, "CountDownCooldownTicks"),
        ]
    );
    assert_eq!(
        brain.behavior_priorities(Activity::IDLE),
        vec![
            (0, "SetEntityLookTargetSometimes"),
            (0, "AnimalMakeLove"),
            (1, "FollowTemptation"),
            (2, "StartAttacking"),
            (3, "TryFindLand"),
            (4, "RunOne"),
        ]
    );
    // `FrogAi`: SWIM's ordered, try-all `GateBehavior`.
    assert_eq!(
        brain.behavior_priorities(Activity::SWIM),
        vec![
            (0, "SetEntityLookTargetSometimes"),
            (1, "FollowTemptation"),
            (2, "StartAttacking"),
            (3, "TryFindLand"),
            (5, "GateBehavior"),
        ]
    );
    assert_eq!(
        brain.behavior_priorities(Activity::LAY_SPAWN),
        vec![
            (0, "SetEntityLookTargetSometimes"),
            (1, "StartAttacking"),
            (2, "TryFindLandNearWater"),
            (3, "TryLaySpawnOnFluidNearLand"),
            (4, "RunOne"),
        ]
    );
    assert_eq!(
        brain.behavior_priorities(Activity::TONGUE),
        vec![(0, "StopAttackingIfTargetInvalid"), (1, "ShootTongue")]
    );
    // `FrogAi.initJumpActivity`: the mid-jump half at
    // priority 0 and the landing-search half at priority 1.
    assert_eq!(
        brain.behavior_priorities(Activity::LONG_JUMP),
        vec![(0, "LongJumpMidJump"), (1, "LongJumpToPreferredBlock")]
    );
    // The other provider family keeps the priorities it had: the frog's two
    // new slots are its own IDLE/SWIM packages, not a shift of the shared
    // numbers.
    let creaking = crate::entity::ai::mob::creaking::creaking_brain(&BrainPacked::empty());
    assert_eq!(
        creaking.behavior_priorities(Activity::IDLE),
        vec![
            (10, "StartAttacking"),
            (11, "SetEntityLookTargetSometimes"),
            (12, "RunOne"),
        ]
    );
}

#[test]
fn a_frog_brain_starts_on_the_activities_frog_ai_registers() {
    let brain = brain();
    // The constructor's `useDefaultActivity`.
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::IDLE]
    );
}

#[test]
fn the_frog_activity_selection_reads_the_in_water_memory() {
    // A spawned frog carries the long-jump cooldown `FrogAi.initMemories`
    // writes, which is what keeps LONG_JUMP out of the running while it is dry
    let mut brain = brain_with(vec![crate::entity::ai::PackedMemory {
        memory_type: MemoryModuleType::LongJumpCooldownTicks,
        value: MemoryValue::Int(100),
        time_to_live: None,
    }]);
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    world.body.in_water = true;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::IsInWater),
        Some(&MemoryValue::Unit)
    );
    // `Frog.updateActivity` -> `FrogAi.updateActivity` (`Frog`,
    // `FrogAi`): SWIM requires IS_IN_WATER present and IDLE requires
    // it absent.
    update_activity(&mut brain);
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::SWIM]
    );

    // The delay the body's random drew was 0, so the sensor scanned on the
    // first tick and scans again `DEFAULT_SCAN_RATE` ticks later
    world.body.in_water = false;
    for _ in 0..usize::try_from(crate::entity::ai::DEFAULT_SCAN_RATE).unwrap() {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert!(!brain.has_memory_value(MemoryModuleType::IsInWater));
    update_activity(&mut brain);
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::IDLE]
    );
}

#[test]
fn the_frog_long_jump_cooldown_a_spawn_wrote_counts_down_to_the_long_jump() {
    // `FrogAi.initMemories` writes this at spawn, which
    // is `crate::entity::spawner_brain_memory`'s frog row.
    let mut brain = brain_with(vec![crate::entity::ai::PackedMemory {
        memory_type: MemoryModuleType::LongJumpCooldownTicks,
        value: MemoryValue::Int(2),
        time_to_live: None,
    }]);
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    // The LONG_JUMP activity requires the cooldown to be absent
    update_activity(&mut brain);
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::IDLE]
    );

    brain.tick(&mut context, &mut world);
    // IDLE's `RunOne` gate holds a child of its own here: the shuffle that
    // picks it draws on `ShufflingList`'s own source,
    // so which child it takes is not staged by this fixture.
    assert_eq!(
        brain.running_behaviors(),
        vec!["CountDownCooldownTicks", "RunOne"]
    );
    assert_eq!(
        brain.memory(MemoryModuleType::LongJumpCooldownTicks),
        Some(&MemoryValue::Int(1))
    );
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::LongJumpCooldownTicks));
    update_activity(&mut brain);
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::LONG_JUMP]
    );
}

#[test]
fn the_frog_tongue_activity_takes_its_memory_from_the_target() {
    let mut brain = brain();
    brain.register_memory(MemoryModuleType::AttackTarget);
    brain.set_memory(
        MemoryModuleType::AttackTarget,
        Some(MemoryValue::EntityRef(3)),
    );
    // TONGUE is first in `FrogAi.updateActivity`'s list and
    // is the only activity that erases its own memory when it stops
    update_activity(&mut brain);
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::TONGUE]
    );
    brain.set_active_activity_to_first_valid(&[Activity::IDLE]);
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));
}

#[test]
fn the_frog_panic_activity_reads_a_panic_causing_source() {
    let mut brain = brain();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(BrainDamageSource {
        damage_type: "minecraft:mob_attack".to_string(),
        source_entity: None,
    });
    world.panic_causes = true;

    // `FrogAi.initCoreActivity`.
    brain.tick(&mut context, &mut world);
    // The panic and IDLE's `RunOne` gate, whose child the shuffle on
    // `ShufflingList`'s own source picks.
    assert_eq!(brain.running_behaviors(), vec!["AnimalPanic", "RunOne"]);
    assert_eq!(
        brain.memory(MemoryModuleType::IsPanicking),
        Some(&MemoryValue::Bool(true))
    );

    // The panic outranks nothing: CORE is active the whole time.
    update_activity(&mut brain);
    assert!(brain.is_active(Activity::CORE));
}

/// The `FrogAi` behaviours this runtime does not run yet, which is what the
/// ledger's remaining work is measured against.
const UNWIRED_BEHAVIORS: &[&str] = &[
    "SetEntityLookTargetSometimes",
    "AnimalMakeLove",
    "TryFindLand",
    "RunOne",
    "TryFindLandNearWater",
    "TryLaySpawnOnFluidNearLand",
    "LongJumpMidJump",
    "LongJumpToPreferredBlock",
];

#[test]
fn the_wired_core_behaviours_are_the_ones_this_runtime_has() {
    let mut brain = brain();
    brain.set_memory(
        MemoryModuleType::TemptationCooldownTicks,
        Some(MemoryValue::Int(1)),
    );
    brain.set_memory(
        MemoryModuleType::LongJumpCooldownTicks,
        Some(MemoryValue::Int(1)),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    brain.tick(&mut context, &mut world);
    // CORE's two counters, and IDLE's `RunOne` gate holding the child its own
    // shuffle drew.
    assert_eq!(
        brain.running_behaviors(),
        vec!["CountDownCooldownTicks", "CountDownCooldownTicks", "RunOne"]
    );
    assert!(!UNWIRED_BEHAVIORS.contains(&"CountDownCooldownTicks"));
}

#[test]
fn the_tongue_activity_runs_the_two_behaviours_frog_ai_numbers_it_with() {
    let mut brain = brain();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.entity_facts = Some(crate::entity::ai::brain::tests::entity_facts(
        9,
        "minecraft:slime",
    ));
    world.entity_type_tags = true;
    world.distance_to_sqr = Some(1.0);
    world.path_distance = Some(0.0);
    // The two sensors the food path reads scan on this tick: the living-entity
    // scan and the frog's own
    // attackable filter over it.
    world.nearest_living = vec![9];

    // IDLE's `StartAttacking` at priority 2 fills the
    // memory the TONGUE activity is gated on.
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::NearestAttackable),
        Some(&MemoryValue::EntityRef(9))
    );
    assert_eq!(
        brain.memory(MemoryModuleType::AttackTarget),
        Some(&MemoryValue::EntityRef(9))
    );
    update_activity(&mut brain);
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::TONGUE]
    );
    assert!(!UNWIRED_BEHAVIORS.contains(&"ShootTongue"));
    assert!(!UNWIRED_BEHAVIORS.contains(&"StopAttackingIfTargetInvalid"));

    // The TONGUE activity catches and starts its six-tick animation on the
    // tick it becomes active. CORE's
    // `LookAtTargetSink` may join it in the same tick or the next, since
    // whether it sees the `LOOK_TARGET` the catch wrote depends on which
    // activity the priority's own map hands over first -- Vanilla's is a
    // `HashMap` too.
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().contains(&"ShootTongue"));
}

#[test]
fn a_fresh_frog_brain_holds_no_running_behaviour_and_no_memory_value() {
    let brain = brain();
    assert_eq!(brain.running_behaviors(), Vec::<&str>::new());
    let mut written = Vec::new();
    brain.for_each_memory(|memory_type, value, _| {
        if value.is_some() {
            written.push(memory_type.resource_id());
        }
    });
    // The only memory a spawned frog carries is the long-jump cooldown
    // `FrogAi.initMemories` writes, which an empty pack
    // does not hold.
    assert!(written.is_empty(), "{written:?}");
}
