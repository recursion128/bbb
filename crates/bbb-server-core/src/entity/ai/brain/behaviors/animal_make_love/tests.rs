use crate::random::GameRandom;
use bbb_protocol::packets::BlockPos;

use super::*;
use crate::entity::ai::brain::tests::{entity_facts, test_world, TestWorld};
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{Activity, Brain, BrainBreedFacts, MemoryPositionTracker};

const FROG: &str = "minecraft:frog";
const BODY_ID: i32 = 1;
const PARTNER_ID: i32 = 7;

/// The `60 + body.getRandom().nextInt(50)` draw `start` takes
/// off the body's own random, which
/// `BrainTickContext::body_random` seeds the same way.
fn spawn_child_delay() -> i64 {
    i64::from(SPAWN_CHILD_MIN_DELAY + GameRandom::with_seed(0).next_int(50))
}

fn breed_facts(in_love: bool, panicking: bool) -> BrainBreedFacts {
    BrainBreedFacts {
        same_class: true,
        in_love,
        panicking,
    }
}

/// One brain whose IDLE activity is exactly the breeding pair, with the entry
/// conditions' memories registered.
fn brain_with(behavior: Behavior) -> Brain {
    let mut brain = Brain::new();
    for memory_type in [
        MemoryModuleType::NearestVisibleLivingEntities,
        MemoryModuleType::BreedTarget,
        MemoryModuleType::WalkTarget,
        MemoryModuleType::LookTarget,
        MemoryModuleType::IsPanicking,
        MemoryModuleType::AttackTarget,
    ] {
        brain.register_memory(memory_type);
    }
    brain.add_activity(Activity::IDLE, [(0, behavior)], &[], &[]);
    brain
}

fn visible(brain: &mut Brain, ids: Vec<i32>) {
    brain.set_memory(
        MemoryModuleType::NearestVisibleLivingEntities,
        Some(MemoryValue::VisibleEntities(ids)),
    );
}

/// A world holding one in-love frog body and one visible in-love frog partner.
fn breedable_world() -> TestWorld {
    let mut world = test_world();
    world.body.entity_id = BODY_ID;
    world.body.in_love = 1;
    world.distance_to_sqr = Some(1.0);
    world.entity_facts = Some(entity_facts(PARTNER_ID, FROG));
    world.breed_facts = vec![(PARTNER_ID, breed_facts(true, false))];
    world
}

fn walk_target(brain: &Brain) -> Option<MemoryWalkTarget> {
    match brain.memory(MemoryModuleType::WalkTarget) {
        Some(MemoryValue::WalkTarget(target)) => Some(*target),
        _ => None,
    }
}

/// The walk target queued for the partner, which tracks the body itself
/// the body stands at the origin here.
fn partner_walk_target() -> MemoryWalkTarget {
    MemoryWalkTarget {
        target: MemoryPositionTracker::Entity {
            entity_id: BODY_ID,
            last_seen: BlockPos { x: 0, y: 0, z: 0 },
        },
        speed_modifier: 1.0,
        close_enough_dist: 2,
    }
}

/// The `SetMobBrainMemory` writes the tick queued for the partner, in order.
fn partner_writes(world: &TestWorld) -> Vec<(i32, MemoryModuleType, Option<MemoryValue>)> {
    world
        .effects
        .iter()
        .filter_map(|effect| match effect {
            BrainWorldEffect::SetMobBrainMemory {
                entity_id,
                memory_type,
                value,
            } => Some((*entity_id, *memory_type, value.clone())),
            _ => None,
        })
        .collect()
}

fn mated(world: &TestWorld) -> bool {
    world.effects.iter().any(|effect| {
        matches!(effect, BrainWorldEffect::SpawnChildFromBreeding { partner_id } if *partner_id == PARTNER_ID)
    })
}

#[test]
fn animal_make_love_refuses_a_body_that_is_not_in_love() {
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    world.body.in_love = 0;
    visible(&mut brain, vec![PARTNER_ID]);

    brain.tick(&mut context, &mut world);

    assert_eq!(brain.memory(MemoryModuleType::BreedTarget), None);
    assert_eq!(walk_target(&brain), None);
    assert!(world.effects.is_empty());
}

#[test]
fn animal_make_love_refuses_a_body_without_a_visible_partner() {
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    visible(&mut brain, Vec::new());

    brain.tick(&mut context, &mut world);

    assert_eq!(brain.memory(MemoryModuleType::BreedTarget), None);
    assert!(world.effects.is_empty());
}

#[test]
fn animal_make_love_starts_on_the_first_eligible_partner_and_writes_both_halves() {
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    let other = PARTNER_ID + 1;
    // The first entry is a frog that is not in love, the second one is
    // eligible: `findClosest` walks the visible list in order and answers the
    // first candidate every term accepts (`AnimalMakeLove`)
    visible(&mut brain, vec![other, PARTNER_ID]);
    world.breed_facts = vec![
        (other, breed_facts(false, false)),
        (PARTNER_ID, breed_facts(true, false)),
    ];

    brain.tick(&mut context, &mut world);

    // `body.getBrain().setMemory(BREED_TARGET, partner)` (`:53`).
    assert_eq!(
        brain.memory(MemoryModuleType::BreedTarget),
        Some(&MemoryValue::EntityRef(PARTNER_ID))
    );
    // `BehaviorUtils.lockGazeAndWalkToEachOther` (`:55`): the body's own two
    // memories hold the partner, and the partner's two are queued, in the
    // order `BehaviorUtils` writes them.
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::EntityTracker(PARTNER_ID))
    );
    assert_eq!(
        walk_target(&brain),
        Some(MemoryWalkTarget {
            target: MemoryPositionTracker::Entity {
                entity_id: PARTNER_ID,
                last_seen: BlockPos {
                    x: PARTNER_ID,
                    y: 0,
                    z: 0,
                },
            },
            speed_modifier: 1.0,
            close_enough_dist: 2,
        })
    );
    // `Brain.tick` starts each non-running behaviour and then ticks the
    // running ones, so the same tick writes the pair's
    // gaze twice: once from `start` and once from `tick`.
    assert_eq!(
        partner_writes(&world),
        vec![
            (
                PARTNER_ID,
                MemoryModuleType::BreedTarget,
                Some(MemoryValue::EntityRef(BODY_ID))
            ),
            (
                PARTNER_ID,
                MemoryModuleType::LookTarget,
                Some(MemoryValue::EntityTracker(BODY_ID))
            ),
            (
                PARTNER_ID,
                MemoryModuleType::WalkTarget,
                Some(MemoryValue::WalkTarget(partner_walk_target()))
            ),
            (
                PARTNER_ID,
                MemoryModuleType::LookTarget,
                Some(MemoryValue::EntityTracker(BODY_ID))
            ),
            (
                PARTNER_ID,
                MemoryModuleType::WalkTarget,
                Some(MemoryValue::WalkTarget(partner_walk_target()))
            ),
        ]
    );
}

#[test]
fn animal_make_love_mates_only_at_the_deadline_and_in_range() {
    let delay = spawn_child_delay();
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    visible(&mut brain, vec![PARTNER_ID]);

    // `spawnChildAtTime = timestamp + 60 + nextInt(50)` (`:56-57`), taken on
    // the tick the behaviour started on.
    brain.tick(&mut context, &mut world);
    context.game_time = delay - 1;
    world.effects.clear();
    brain.tick(&mut context, &mut world);
    // Before the deadline the body only keeps walking toward the partner
    // (`:77-78`).
    assert!(!mated(&world));
    assert_eq!(
        brain.memory(MemoryModuleType::BreedTarget),
        Some(&MemoryValue::EntityRef(PARTNER_ID))
    );

    // `body.closerThan(partner, 3.0)` (`:77`) refuses at exactly the range.
    context.game_time = delay;
    world.distance_to_sqr = Some(BREED_RANGE * BREED_RANGE);
    brain.tick(&mut context, &mut world);
    assert!(!mated(&world));

    world.distance_to_sqr = Some(BREED_RANGE * BREED_RANGE - 1.0);
    brain.tick(&mut context, &mut world);

    // `body.spawnChildFromBreeding(level, partner)` (`:79`) and the two
    // `BREED_TARGET` erases (`:80-81`), after the pair's gaze writes.
    assert!(mated(&world));
    assert_eq!(brain.memory(MemoryModuleType::BreedTarget), None);
    assert_eq!(
        partner_writes(&world).last().cloned(),
        Some((PARTNER_ID, MemoryModuleType::BreedTarget, None))
    );
}

#[test]
fn animal_make_love_stops_when_the_partner_panics() {
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    visible(&mut brain, vec![PARTNER_ID]);
    brain.tick(&mut context, &mut world);
    world.effects.clear();

    // `!partner.isPanicking()`.
    world.breed_facts = vec![(PARTNER_ID, breed_facts(true, true))];
    brain.tick(&mut context, &mut world);

    // `stop` (`:86-91`) erases the body's three memories and leaves the
    // partner's own `BREED_TARGET` where it is.
    assert_eq!(brain.memory(MemoryModuleType::BreedTarget), None);
    assert_eq!(brain.memory(MemoryModuleType::LookTarget), None);
    assert_eq!(walk_target(&brain), None);
    assert_eq!(partner_writes(&world), Vec::new());
}

#[test]
fn animal_make_love_stops_when_the_body_panics() {
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    visible(&mut brain, vec![PARTNER_ID]);
    brain.tick(&mut context, &mut world);
    world.effects.clear();

    // `!body.isPanicking()`.
    brain.set_memory(MemoryModuleType::IsPanicking, Some(MemoryValue::Bool(true)));
    brain.tick(&mut context, &mut world);

    assert_eq!(brain.memory(MemoryModuleType::BreedTarget), None);
    assert_eq!(partner_writes(&world), Vec::new());
}

#[test]
fn animal_make_love_stops_when_the_partner_leaves_the_visible_list() {
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    visible(&mut brain, vec![PARTNER_ID]);
    brain.tick(&mut context, &mut world);
    world.effects.clear();

    // `BehaviorUtils.entityIsVisible(body.getBrain(), partner)` (`:68`).
    world.targeting = false;
    brain.tick(&mut context, &mut world);

    assert_eq!(brain.memory(MemoryModuleType::BreedTarget), None);
    assert_eq!(partner_writes(&world), Vec::new());
}

#[test]
fn animal_make_love_does_not_start_while_the_body_already_holds_a_target() {
    let mut brain = brain_with(animal_make_love(FROG));
    let mut context = BrainTickContext::default();
    let mut world = breedable_world();
    visible(&mut brain, vec![PARTNER_ID]);
    // The entry condition asks for an absent `BREED_TARGET`
    brain.set_memory(
        MemoryModuleType::BreedTarget,
        Some(MemoryValue::EntityRef(PARTNER_ID)),
    );

    brain.tick(&mut context, &mut world);

    assert!(world.effects.is_empty());
    assert_eq!(
        brain.memory(MemoryModuleType::BreedTarget),
        Some(&MemoryValue::EntityRef(PARTNER_ID))
    );
}
