use super::*;
use crate::entity::ai::brain::tests::test_world;
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{BrainPacked, MemoryValue};

fn brain() -> Brain {
    creaking_brain(&BrainPacked::empty())
}

#[test]
fn the_creaking_provider_registers_vanillas_memories() {
    let brain = brain();
    let mut registered = Vec::new();
    brain.for_each_memory(|memory_type, _, _| registered.push(memory_type.resource_id()));
    registered.sort_unstable();
    let mut expected: Vec<&str> = CREAKING_MEMORY_MODULES
        .iter()
        .map(|memory_type| memory_type.resource_id())
        .collect();
    expected.sort_unstable();
    assert_eq!(registered, expected);
    assert_eq!(registered.len(), 12);
}

#[test]
fn a_creaking_brain_starts_on_core_and_idle() {
    let brain = brain();
    // The constructor's `useDefaultActivity`.
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::IDLE]
    );
}

#[test]
fn a_frozen_creaking_takes_the_default_activity() {
    let mut brain = brain();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    brain.set_memory(
        MemoryModuleType::AttackTarget,
        Some(MemoryValue::EntityRef(7)),
    );
    update_activity(&mut brain, true);
    // `CreakingAi.updateActivity`: FIGHT carries the
    // `ATTACK_TARGET` condition and outranks IDLE.
    assert!(brain.is_active(Activity::FIGHT));

    // `:80-81`: a creaking that cannot move takes the brain's default.
    update_activity(&mut brain, false);
    assert!(!brain.is_active(Activity::FIGHT));
    assert_eq!(
        brain.active_activities(),
        vec![Activity::CORE, Activity::IDLE],
        "`useDefaultActivity` selects the default activity (`Brain.java:410-412`)"
    );
    let _ = (&mut context, &mut world);
}

#[test]
fn the_creaking_provider_registers_vanillas_behaviors_at_their_priorities() {
    let brain = brain();
    // `CreakingAi`: CORE's three, IDLE's three and FIGHT's three,
    // with each activity's first behavior at the priority its
    // `ActivityData.create` names (`ActivityData` sums the package
    // base onto the given one).
    assert_eq!(
        brain.behavior_priorities(Activity::CORE),
        vec![
            (0, "Swim"),
            (1, "LookAtTargetSink"),
            (2, "MoveToTargetSink"),
        ]
    );
    assert_eq!(
        brain.behavior_priorities(Activity::IDLE),
        vec![
            (10, "StartAttacking"),
            (11, "SetEntityLookTargetSometimes"),
            (12, "RunOne"),
        ]
    );
    assert_eq!(
        brain.behavior_priorities(Activity::FIGHT),
        vec![
            (10, "SetWalkTargetFromAttackTargetIfTargetOutOfReach"),
            (11, "MeleeAttack"),
            (12, "StopAttackingIfTargetInvalid"),
        ]
    );
}

#[test]
fn the_creaking_idle_activity_walks_and_attacks_through_the_shared_memories() {
    let mut brain = brain();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.distance_to_sqr = Some(4.0);
    world.entity_facts = Some(crate::entity::ai::brain::tests::entity_facts(
        7,
        "minecraft:player",
    ));
    // What the provider's own `PlayerSensor` reads each tick: a player in
    // range, targetable and attackable.
    world.nearest_players = vec![7];
    world.targeting = true;
    world.combat_targeting = true;
    world.line_of_sight = true;

    // `CreakingAi`: an inactive protector takes no target even with
    // the player sensor holding one.
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::NearestVisibleAttackablePlayer),
        Some(&MemoryValue::EntityRef(7)),
        "the sensor the provider registered wrote the player"
    );
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));

    // `:44` `Creaking::isActive` answers yes, and the IDLE gate then starts one
    // of its three children (`:48-52`).
    world.body.is_active = true;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::AttackTarget),
        Some(&MemoryValue::EntityRef(7))
    );
    // The IDLE gate's own activity is still the one that runs: `IS_ACTIVE`
    // alone does not take FIGHT, which the `ATTACK_TARGET` condition decides.
    assert!(brain.is_active(Activity::IDLE));

    update_activity(&mut brain, true);
    assert!(brain.is_active(Activity::FIGHT));
}
