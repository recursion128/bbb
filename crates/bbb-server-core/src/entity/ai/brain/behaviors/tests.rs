use bbb_protocol::packets::{BlockPos, Vec3d};
use uuid::Uuid;

use super::*;
use crate::entity::ai::brain::tests::{entity_facts, test_world};
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{
    animal_panic, count_down_cooldown_ticks, look_at_target_sink, move_to_target_sink,
    shoot_tongue, start_attacking, stop_attacking_if_target_invalid,
    stop_attacking_if_target_invalid_unless_in, BehaviorStatus, Brain, BrainDamageSource,
    BrainEntityFacts, BrainWorldEffect, MemoryPath, MemoryPositionTracker, MemoryWalkTarget,
    ShootTongue,
};

fn brain_with_panic() -> Brain {
    let mut brain = Brain::new();
    brain.register_sensor(crate::entity::ai::SensorType::HurtBy);
    // `AnimalPanic`'s own entry condition names only `IS_PANICKING` and
    // `HURT_BY`; the frog's provider registers the walk target through its
    // other behaviours.
    brain.register_memory(MemoryModuleType::WalkTarget);
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(0, animal_panic(2.0))],
        &[],
        &[],
    );
    brain
}

fn hurt(damage_type: &str) -> BrainDamageSource {
    BrainDamageSource {
        damage_type: damage_type.to_string(),
        source_entity: None,
    }
}

#[test]
fn animal_panic_starts_on_a_panic_causing_source_and_stops_the_navigation() {
    let mut brain = brain_with_panic();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(hurt("minecraft:mob_attack"));
    world.panic_causes = true;

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["AnimalPanic"]);
    assert_eq!(
        brain.memory(MemoryModuleType::IsPanicking),
        Some(&MemoryValue::Bool(true))
    );
    // `body.getNavigation().stop()`.
    assert_eq!(world.navigation_stops, 1);

    // The duration is drawn from the context between 100 and 120
    // `BrainTickContext` draws the low end, so the
    // behaviour times out on the first tick past its timestamp
    for _ in 0..100 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert_eq!(brain.running_behaviors(), vec!["AnimalPanic"]);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    // `AnimalPanic.stop`.
    assert!(!brain.has_memory_value(MemoryModuleType::IsPanicking));

    // `HURT_BY` outlives the behaviour: only the source's own window closes
    // it, and a scan carries the next twenty ticks
    world.body.last_damage = None;
    for _ in 0..20 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert_eq!(brain.memory(MemoryModuleType::HurtBy), None);
}

#[test]
fn animal_panic_refuses_a_source_the_panic_tag_does_not_name() {
    let mut brain = brain_with_panic();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(hurt("minecraft:generic"));
    world.panic_causes = false;

    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert!(!brain.has_memory_value(MemoryModuleType::IsPanicking));
    assert_eq!(world.navigation_stops, 0);
}

/// A brain whose CORE is the frog's `new AnimalPanic<>(2.0F)`
/// so `tick` writes the walk target with that speed
/// multiplier.
#[test]
fn animal_panic_walks_the_position_its_getter_samples_once_the_navigation_is_done() {
    let mut brain = brain_with_panic();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(hurt("minecraft:mob_attack"));
    world.panic_causes = true;
    // `body.getNavigation().isDone()` holds after the
    // stop `start` made, so the first tick of the behaviour writes.
    world.navigation_done = true;
    world.land_random_pos = Some(Some(Vec3d {
        x: 4.5,
        y: 64.0,
        z: -2.5,
    }));

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["AnimalPanic"]);
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(BlockPos { x: 4, y: 64, z: -3 }),
            speed_modifier: 2.0,
            close_enough_dist: 0,
        })),
        "`new WalkTarget(panicToPos, speedMultiplier, 0)` (`AnimalPanic.java:84`)"
    );
    // `LandRandomPos.getPos(body, 5, 4)`, drawn from
    // the body's own random.
    assert_eq!(world.land_random_pos_calls, vec![(5, 4)]);
    // The body is not burning, so the water search is not even reached
    assert!(world.closest_water_calls.is_empty());
}

#[test]
fn animal_panic_leaves_the_walk_target_alone_while_its_navigation_runs() {
    let mut brain = brain_with_panic();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(hurt("minecraft:mob_attack"));
    world.panic_causes = true;
    world.navigation_done = false;
    world.land_random_pos = Some(Some(Vec3d {
        x: 4.5,
        y: 64.0,
        z: -2.5,
    }));

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["AnimalPanic"]);
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
    assert!(world.land_random_pos_calls.is_empty());
}

#[test]
fn animal_panic_runs_to_the_water_it_finds_while_burning() {
    let mut brain = brain_with_panic();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(hurt("minecraft:mob_attack"));
    world.panic_causes = true;
    world.body.on_fire = true;
    world.closest_water = Some(Some(BlockPos { x: 1, y: 64, z: 0 }));
    // A body `Mth.ceil(getBbWidth()) == 2` wide takes the four-cell predicate
    world.body.bounding_box =
        bbb_server_types::geometry::Aabb::of_entity(Vec3d::default(), 2.0, 2.0);

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(BlockPos { x: 1, y: 64, z: 0 }),
            speed_modifier: 2.0,
            close_enough_dist: 0,
        })),
        "`lookForWater(level, body).map(Vec3::atBottomCenterOf)` (`AnimalPanic.java:91`)"
    );
    assert_eq!(world.closest_water_calls, vec![(5, 1, true)]);
    // The position getter is not reached at all: `:92` returns the water.
    assert!(world.land_random_pos_calls.is_empty());
}

#[test]
fn animal_panic_falls_through_to_its_position_getter_without_water() {
    let mut brain = brain_with_panic();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.last_damage = Some(hurt("minecraft:mob_attack"));
    world.panic_causes = true;
    world.body.on_fire = true;
    world.closest_water = Some(None);
    world.land_random_pos = Some(Some(Vec3d {
        x: 0.5,
        y: 60.0,
        z: 4.5,
    }));

    brain.tick(&mut context, &mut world);
    assert_eq!(world.closest_water_calls, vec![(5, 1, false)]);
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(BlockPos { x: 0, y: 60, z: 4 }),
            speed_modifier: 2.0,
            close_enough_dist: 0,
        })),
        "the empty `Optional` falls through to `positionGetter.apply(body)` (`:97`)"
    );

    // A null position writes nothing, even though the behaviour keeps running
    brain.erase_memory(MemoryModuleType::WalkTarget);
    world.land_random_pos = Some(None);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["AnimalPanic"]);
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
}

#[test]
fn count_down_cooldown_ticks_erases_the_memory_it_reaches_zero_on() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::LongJumpCooldownTicks);
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [
            (
                0,
                count_down_cooldown_ticks(MemoryModuleType::LongJumpCooldownTicks),
            ),
            (1, crate::entity::ai::Behavior::new(Vec::new(), ())),
        ],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::LongJumpCooldownTicks,
        Some(MemoryValue::Int(2)),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::LongJumpCooldownTicks),
        Some(&MemoryValue::Int(1))
    );
    assert_eq!(brain.running_behaviors(), vec!["CountDownCooldownTicks"]);

    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::LongJumpCooldownTicks),
        Some(&MemoryValue::Int(0))
    );

    // `canStillUse` refuses a zero and `stop` erases the memory
    // even though the behaviour's own
    // timeout is refused.
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::LongJumpCooldownTicks));
    assert!(brain.running_behaviors().is_empty());
}

#[test]
fn count_down_cooldown_ticks_never_times_out_on_its_own() {
    let hooks = CountDownCooldownTicks {
        cooldown_ticks: MemoryModuleType::TemptationCooldownTicks,
    };
    // `CountDownCooldownTicks.timedOut`
    // refuses the duration, so a behaviour that is still usable at twice its
    // span keeps running.
    assert!(!BehaviorHooks::timed_out(&hooks, 1_000, 60));
    let behavior = count_down_cooldown_ticks(MemoryModuleType::TemptationCooldownTicks);
    assert_eq!(behavior.status(), BehaviorStatus::Stopped);
}

/// The frog's TONGUE activity as `FrogAi.initTongueActivity` builds it
/// `StopAttackingIfTargetInvalid` at the
/// `ActivityData`-numbered priority 0 and `ShootTongue` at 1.
fn brain_with_tongue() -> Brain {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestVisibleLivingEntities);
    brain.register_memory(MemoryModuleType::UnreachableTongueTargets);
    brain.add_activity(
        crate::entity::ai::Activity::TONGUE,
        [(0, stop_attacking_if_target_invalid()), (1, shoot_tongue())],
        &[(MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent)],
        &[MemoryModuleType::AttackTarget],
    );
    brain.set_memory(
        MemoryModuleType::AttackTarget,
        Some(MemoryValue::EntityRef(9)),
    );
    // `FrogAi.updateActivity` picks TONGUE first while
    // `ATTACK_TARGET` answers.
    brain.set_active_activity_if_possible(crate::entity::ai::Activity::TONGUE);
    brain
}

fn staged_food() -> crate::entity::ai::brain::tests::TestWorld {
    let mut world = test_world();
    world.entity_facts = Some(entity_facts(9, "minecraft:slime"));
    world.entity_type_tags = true;
    world.distance_to_sqr = Some(100.0);
    world.path_distance = Some(0.0);
    world.body.attack_damage = 10.0;
    world.body.position = Vec3d {
        x: 0.0,
        y: 64.0,
        z: 0.0,
    };
    world.body.block_position = BlockPos { x: 0, y: 64, z: 0 };
    world
}

#[test]
fn start_attacking_takes_the_nearest_attackable_and_erases_the_cant_reach_stamp() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestAttackable);
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(2, start_attacking())],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::NearestAttackable,
        Some(MemoryValue::EntityRef(9)),
    );
    brain.set_memory(
        MemoryModuleType::CantReachWalkTargetSince,
        Some(MemoryValue::Long(41)),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::AttackTarget),
        Some(&MemoryValue::EntityRef(9))
    );
    // `cantReachSince.erase()`.
    assert!(!brain.has_memory_value(MemoryModuleType::CantReachWalkTargetSince));
    // A `OneShot` stops the moment it started, so the
    // next tick starts it again rather than ticking it.
    assert!(brain.running_behaviors().is_empty());
}

#[test]
fn start_attacking_refuses_a_breeding_body_and_a_target_it_cannot_attack() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestAttackable);
    brain.register_memory(MemoryModuleType::BreedTarget);
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(2, start_attacking())],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::NearestAttackable,
        Some(MemoryValue::EntityRef(9)),
    );
    brain.set_memory(
        MemoryModuleType::BreedTarget,
        Some(MemoryValue::EntityRef(3)),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));

    // `!BehaviorUtils.isBreeding(mob)` answers yes and
    // `body.canAttack(target)` answers no.
    brain.erase_memory(MemoryModuleType::BreedTarget);
    world.body_can_attack = false;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));
}

#[test]
fn stop_attacking_erases_a_target_this_world_no_longer_holds() {
    let mut brain = brain_with_tongue();
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    // The `ATTACK_TARGET` entry condition holds for the two TONGUE behaviours,
    // so the stop behaviour alone runs while the target answers.
    world.entity_facts = None;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));
}

#[test]
fn stop_attacking_erases_a_target_the_body_became_tired_of() {
    let mut brain = brain_with_tongue();
    brain.set_memory(
        MemoryModuleType::CantReachWalkTargetSince,
        Some(MemoryValue::Long(0)),
    );
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    // `isTiredOfTryingToReachTarget`:
    // the level's time is more than two hundred ticks past the stamp.
    world.game_time = 201;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));
}

/// `CreakingAi.FIGHT`'s third entry is the
/// `stopAttackingWhen` overload: the callback reads
/// `NEAREST_VISIBLE_ATTACKABLE_PLAYERS`, so a target the sensor scan no longer
/// lists is erased even while it stays attackable and alive.
#[test]
fn stop_attacking_erases_a_target_the_scan_no_longer_lists() {
    let mut brain = brain_with_reach_condition();
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    brain.set_memory(
        MemoryModuleType::NearestVisibleAttackablePlayers,
        Some(MemoryValue::EntityList(vec![4])),
    );
    brain.tick(&mut context, &mut world);
    assert!(
        !brain.has_memory_value(MemoryModuleType::AttackTarget),
        "`isAttackTargetStillReachable` (`CreakingAi.java:70-73`) answers false for 9"
    );

    // An empty memory is the `orElse(false)` arm of the same predicate.
    let mut brain = brain_with_reach_condition();
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));
}

fn brain_with_reach_condition() -> Brain {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestVisibleAttackablePlayers);
    brain.add_activity(
        crate::entity::ai::Activity::FIGHT,
        [(
            0,
            stop_attacking_if_target_invalid_unless_in(Some(
                MemoryModuleType::NearestVisibleAttackablePlayers,
            )),
        )],
        &[(MemoryModuleType::AttackTarget, MemoryStatus::ValuePresent)],
        &[MemoryModuleType::AttackTarget],
    );
    brain.set_memory(
        MemoryModuleType::AttackTarget,
        Some(MemoryValue::EntityRef(9)),
    );
    // `CreakingAi.updateActivity` picks FIGHT first
    // while the condition holds.
    brain.set_active_activity_if_possible(crate::entity::ai::Activity::FIGHT);
    brain
}

#[test]
fn shoot_tongue_catches_on_the_tick_it_starts_and_writes_vanillas_sequence() {
    let mut brain = brain_with_tongue();
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    world.distance_to_sqr = Some(1.0);
    world.entity_facts = Some(BrainEntityFacts {
        position: Vec3d {
            x: 9.0,
            y: 64.0,
            z: 0.0,
        },
        ..entity_facts(9, "minecraft:slime")
    });

    // `ShootTongue.start` and then `tick`'s
    // MOVE_TO_TARGET arm at `target.distanceTo(body) < 1.75F` run
    // in the same tick, because `Brain.tick` starts before it ticks.
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::EntityTracker(9))
    );
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(BlockPos { x: 9, y: 64, z: 0 }),
            speed_modifier: 2.0,
            close_enough_dist: 0,
        }))
    );
    assert_eq!(world.tongue_target, Some(9));
    assert_eq!(
        world.effect_names(),
        vec!["PlayBodySound", "SetPose", "SetDeltaMovement"]
    );
    assert_eq!(world.sounds(), vec![(ShootTongue::TONGUE_SOUND, 2.0, 1.0)]);
    assert_eq!(world.poses(), vec![POSE_USING_TONGUE]);
    // `target.setDeltaMovement(...scale(0.75))` (`:107`) from the target's own
    // position to the body's: nine blocks apart, so the unit vector scaled by
    // three quarters.
    assert_eq!(
        world.effects[2],
        BrainWorldEffect::SetDeltaMovement {
            entity_id: 9,
            movement: Vec3d {
                x: -0.75,
                y: 0.0,
                z: 0.0,
            },
        }
    );

    // `CATCH_ANIMATION` counts six ticks before `eatEntity`
    // whose `eatAnimationTimer++` compares the
    // value from before the increment, so the eat lands on the seventh tick.
    for tick in 1..=6 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert_eq!(world.tongue_target, Some(9), "catch tick {tick}");
        assert_eq!(world.sounds().len(), 1, "no eat on catch tick {tick}");
        assert_eq!(
            brain.running_behaviors(),
            vec!["ShootTongue"],
            "tick {tick}"
        );
    }
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(world.sounds().len(), 2, "the eat sound follows the catch");
    assert_eq!(world.sounds()[1], (ShootTongue::EAT_SOUND, 2.0, 1.0));
    // `Mob.doHurtTarget` with the frog's own attack damage.
    assert_eq!(
        world.effects.last(),
        Some(&BrainWorldEffect::HurtEntity {
            target_id: 9,
            damage_type: "minecraft:mob_attack",
            amount: 10.0,
            remove_if_killed: true,
        })
    );
}

#[test]
fn shoot_tongue_asks_for_the_bite_and_the_removal_together() {
    let mut brain = brain_with_tongue();
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    world.distance_to_sqr = Some(1.0);
    brain.tick(&mut context, &mut world);

    // Seven ticks later the tongue bites: `body.doHurtTarget(level, target)`
    // and the `if (!target.isAlive()) target.remove(KILLED)` behind it are one
    // call, because the store applies both after this tick
    for _ in 0..7 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert_eq!(
        world.effects.last(),
        Some(&BrainWorldEffect::HurtEntity {
            target_id: 9,
            damage_type: "minecraft:mob_attack",
            amount: 10.0,
            remove_if_killed: true,
        })
    );
}

#[test]
fn shoot_tongue_refuses_a_target_it_cannot_path_to_and_remembers_it() {
    let mut brain = brain_with_tongue();
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    // `canPathfindToTarget` needs a path whose
    // end is within `EATING_DISTANCE` of the target.
    world.path_distance = Some(1.75);

    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::AttackTarget));
    assert_eq!(
        brain.memory(MemoryModuleType::UnreachableTongueTargets),
        Some(&MemoryValue::UuidList(vec![Uuid::from_u128(10)]))
    );
    // `setMemoryWithExpiry(..., 100L)`.
    assert_eq!(
        brain.time_until_expiry(MemoryModuleType::UnreachableTongueTargets),
        ShootTongue::UNREACHABLE_COOLDOWN
    );
    assert!(world.effects.is_empty());
}

#[test]
fn shoot_tongue_refuses_a_food_it_must_not_eat_and_a_croaking_body() {
    let mut brain = brain_with_tongue();
    let mut context = BrainTickContext::default();
    let mut world = staged_food();
    // `Frog.canEat` refuses slimes larger than size one.
    world.entity_facts = Some(BrainEntityFacts {
        slime_size: Some(2),
        ..entity_facts(9, "minecraft:slime")
    });
    brain.tick(&mut context, &mut world);
    assert!(world.effects.is_empty());

    // A croaking frog does not shoot.
    world.entity_facts = Some(entity_facts(9, "minecraft:slime"));
    world.body.pose_id = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROAKING_ID;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(world.effects.is_empty());
}

#[test]
fn move_to_target_sink_starts_the_navigation_and_erases_the_walk_target_it_reached() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.block_position = BlockPos { x: 0, y: 64, z: 0 };
    world.navigation_done = false;
    // `tryComputePath` hands the navigation the path it just built
    // so the sink's `start` finds one.
    world.path = Some(MemoryPath {
        target: BlockPos { x: 4, y: 64, z: 0 },
        length: 4,
        reached: true,
    });
    world.move_to_result = Some(MemoryPath {
        target: BlockPos { x: 4, y: 64, z: 0 },
        length: 4,
        reached: true,
    });
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(BlockPos { x: 4, y: 64, z: 0 }),
            speed_modifier: 2.0,
            close_enough_dist: 0,
        })),
    );

    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.move_to_calls,
        vec![(BlockPos { x: 4, y: 64, z: 0 }, 0, 2.0)]
    );
    assert_eq!(brain.running_behaviors(), vec!["MoveToTargetSink"]);
    assert_eq!(
        brain.memory(MemoryModuleType::Path),
        Some(&MemoryValue::Path(MemoryPath {
            target: BlockPos { x: 4, y: 64, z: 0 },
            length: 4,
            reached: true,
        }))
    );

    // The body arrives: `canStillUse` refuses and `stop` erases both memories
    // and stops the navigation.
    world.body.block_position = BlockPos { x: 4, y: 64, z: 0 };
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
    assert!(!brain.has_memory_value(MemoryModuleType::Path));
    assert_eq!(world.navigation_stops, 1);
}

/// `WalkTarget.getTarget()` is a `PositionTracker`, and the `EntityTracker`
/// arm answers the tracked entity's block as of the call.
/// The sink reads it four times per tick
/// so a target that
/// walked away is repathed without anybody rewriting the memory.
#[test]
fn move_to_target_sink_repaths_to_where_its_tracked_target_walked() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.block_position = BlockPos { x: 0, y: 64, z: 0 };
    world.navigation_done = false;
    world.path = Some(MemoryPath {
        target: BlockPos { x: 4, y: 64, z: 0 },
        length: 4,
        reached: false,
    });
    world.move_to_result = world.path;
    world.entity_facts = Some(BrainEntityFacts {
        position: Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
        ..entity_facts(9, "minecraft:player")
    });
    // The memory keeps the block the tracker was built over, which is stale the
    // moment the entity moves.
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::Entity {
                entity_id: 9,
                last_seen: BlockPos { x: 4, y: 64, z: 0 },
            },
            speed_modifier: 2.0,
            close_enough_dist: 0,
        })),
    );

    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.move_to_calls,
        vec![(BlockPos { x: 4, y: 64, z: 0 }, 0, 2.0)]
    );

    // The target walks five blocks east and nothing rewrites the memory. The
    // running sink's own `tick` sees `distSqr > 4.0` against the block it
    // pathed to and repaths.
    world.entity_facts = Some(BrainEntityFacts {
        position: Vec3d {
            x: 9.5,
            y: 64.0,
            z: 0.5,
        },
        ..entity_facts(9, "minecraft:player")
    });
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.move_to_calls,
        vec![
            (BlockPos { x: 4, y: 64, z: 0 }, 0, 2.0),
            (BlockPos { x: 9, y: 64, z: 0 }, 0, 2.0)
        ]
    );
    assert_eq!(brain.running_behaviors(), vec!["MoveToTargetSink"]);
}

/// `MoveToTargetSink.canStillUse` gives up on
/// a tracked target that turned spectator.
#[test]
fn move_to_target_sink_stops_walking_to_a_spectator() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.block_position = BlockPos { x: 0, y: 64, z: 0 };
    world.navigation_done = false;
    let target = BlockPos { x: 4, y: 64, z: 0 };
    world.move_to_result = Some(MemoryPath {
        target,
        length: 4,
        reached: true,
    });
    let facts = BrainEntityFacts {
        position: Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
        ..entity_facts(9, "minecraft:player")
    };
    world.entity_facts = Some(facts);
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::Entity {
                entity_id: 9,
                last_seen: target,
            },
            speed_modifier: 1.0,
            close_enough_dist: 0,
        })),
    );
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["MoveToTargetSink"]);

    world.entity_facts = Some(BrainEntityFacts {
        spectator: true,
        ..facts
    });
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert_eq!(world.navigation_stops, 1);
}

#[test]
fn move_to_target_sink_stops_after_a_moved_target_has_no_new_path() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.navigation_done = false;
    let original = BlockPos { x: 4, y: 64, z: 0 };
    world.move_to_result = Some(MemoryPath {
        target: original,
        length: 4,
        reached: true,
    });
    world.entity_facts = Some(BrainEntityFacts {
        position: Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
        ..entity_facts(9, "minecraft:player")
    });
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::Entity {
                entity_id: 9,
                last_seen: original,
            },
            speed_modifier: 1.0,
            close_enough_dist: 0,
        })),
    );
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["MoveToTargetSink"]);

    world.entity_facts = Some(BrainEntityFacts {
        position: Vec3d {
            x: 9.5,
            y: 64.0,
            z: 0.5,
        },
        ..entity_facts(9, "minecraft:player")
    });
    world.move_to_result = None;
    world.towards_result = Some(None);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["MoveToTargetSink"]);
    assert_eq!(world.move_to_calls.len(), 1);

    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
    assert_eq!(world.navigation_stops, 1);
}

#[test]
fn move_to_target_sink_rejects_a_path_when_the_target_is_already_reached() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::CantReachWalkTargetSince);
    brain.set_memory(
        MemoryModuleType::CantReachWalkTargetSince,
        Some(MemoryValue::Long(11)),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    let target = world.body.block_position;
    world.move_to_result = Some(MemoryPath {
        target,
        length: 1,
        reached: true,
    });
    let mut tick = BrainTick::new(&mut brain.memories, brain.activity, &mut world);

    assert_eq!(
        try_compute_path(
            &mut tick,
            &mut context,
            WalkTargetValue {
                pos: target,
                speed_modifier: 1.0,
                close_enough_dist: 0
            },
        ),
        None
    );
    assert!(!tick.has_memory_value(MemoryModuleType::CantReachWalkTargetSince));
    assert!(world.towards_calls.is_empty());
}

#[test]
fn move_to_target_sink_samples_only_after_null_path_then_starts_the_prepared_route() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.game_time = 37;
    world.navigation_done = false;
    let target = BlockPos { x: 8, y: 64, z: 0 };
    let partial = BlockPos { x: 2, y: 64, z: 0 };
    world.prepare_path_results.extend([
        None,
        Some(MemoryPath {
            target: partial,
            length: 2,
            reached: false,
        }),
    ]);
    world.towards_result = Some(Some(Vec3d {
        x: 2.5,
        y: 64.0,
        z: 0.5,
    }));
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget::at(
            target, 1.25, 0,
        ))),
    );

    brain.tick(&mut context, &mut world);
    assert_eq!(world.prepare_path_calls, [(target, 0), (partial, 0)]);
    assert_eq!(world.move_to_calls, [(partial, 0, 1.25)]);
    assert_eq!(
        world.towards_calls,
        [(
            10,
            7,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 0.5
            },
            f64::from(std::f32::consts::FRAC_PI_2)
        )]
    );
    assert_eq!(
        brain.memory(MemoryModuleType::CantReachWalkTargetSince),
        Some(&MemoryValue::Long(37))
    );
    assert_eq!(
        brain.memory(MemoryModuleType::Path),
        Some(&MemoryValue::Path(MemoryPath {
            target: partial,
            length: 2,
            reached: false
        }))
    );
}

#[test]
fn move_to_target_sink_keeps_a_nonreaching_path_without_sampling() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.game_time = 11;
    world.navigation_done = false;
    let target = BlockPos { x: 8, y: 64, z: 0 };
    world.move_to_result = Some(MemoryPath {
        target,
        length: 3,
        reached: false,
    });
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget::at(
            target, 1.0, 0,
        ))),
    );

    brain.tick(&mut context, &mut world);
    assert!(world.towards_calls.is_empty());
    assert_eq!(
        brain.memory(MemoryModuleType::CantReachWalkTargetSince),
        Some(&MemoryValue::Long(11))
    );
}

#[test]
fn move_to_target_sink_clears_the_cant_reach_stamp_after_a_reachable_retry() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.game_time = 11;
    world.navigation_done = false;
    let target = BlockPos { x: 8, y: 64, z: 0 };
    world.move_to_result = Some(MemoryPath {
        target,
        length: 3,
        reached: false,
    });
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget::at(
            target, 1.0, 0,
        ))),
    );
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::CantReachWalkTargetSince),
        Some(&MemoryValue::Long(11))
    );

    world.navigation_done = true;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));

    world.navigation_done = false;
    world.move_to_result = Some(MemoryPath {
        target,
        length: 8,
        reached: true,
    });
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget::at(
            target, 1.0, 0,
        ))),
    );
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::CantReachWalkTargetSince));
    assert_eq!(world.move_to_calls.len(), 2);
}

#[test]
fn move_to_target_sink_erases_walk_target_after_both_path_attempts_fail() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.game_time = 23;
    world.towards_result = Some(Some(Vec3d {
        x: 2.5,
        y: 64.0,
        z: 0.5,
    }));
    let target = BlockPos { x: 8, y: 64, z: 0 };
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget::at(
            target, 1.0, 0,
        ))),
    );

    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
    assert_eq!(
        brain.memory(MemoryModuleType::CantReachWalkTargetSince),
        Some(&MemoryValue::Long(23))
    );
    assert!(world.move_to_calls.is_empty());
}

/// `MoveToTargetSink.stop`'s stuck retry holds
/// the sink off for the drawn number of ticks.
#[test]
fn move_to_target_sink_holds_off_for_its_stuck_retry_delay() {
    let walk_target_pos = BlockPos { x: 4, y: 64, z: 0 };
    let walk_target = MemoryWalkTarget::at(walk_target_pos, 2.0, 0);
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(2, move_to_target_sink())],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.block_position = BlockPos { x: 0, y: 64, z: 0 };
    world.navigation_done = false;
    world.path = Some(MemoryPath {
        target: walk_target_pos,
        length: 4,
        reached: false,
    });
    world.move_to_result = Some(MemoryPath {
        target: walk_target_pos,
        length: 4,
        reached: false,
    });
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(walk_target)),
    );
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["MoveToTargetSink"]);

    // The body is stuck short of the target and the navigation is done, so
    // `canStillUse` refuses and `stop` draws the retry (`:78-89`). The level's
    // seam answers seven, which is the delay the sink must serve.
    world.navigation_done = true;
    world.navigation_stuck = true;
    world.level_random_int = 7;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty(), "`stop` ends the run");
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));

    // The cooldown the level drew is what keeps the sink off: the same walk
    // target for the next seven ticks is refused by the decrement alone
    // (`:45-49`), and the eighth starts the navigation again.
    world.navigation_done = false;
    world.navigation_stuck = false;
    for tick in 1..=7 {
        brain.set_memory(
            MemoryModuleType::WalkTarget,
            Some(MemoryValue::WalkTarget(walk_target)),
        );
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert!(
            brain.running_behaviors().is_empty(),
            "the retry has {tick} ticks behind it"
        );
    }
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(walk_target)),
    );
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["MoveToTargetSink"]);
}

#[test]
fn look_at_target_sink_aims_the_body_and_erases_the_tracker_it_lost() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestVisibleLivingEntities);
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(1, look_at_target_sink())],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::LookTarget,
        Some(MemoryValue::EntityTracker(9)),
    );
    brain.set_memory(
        MemoryModuleType::NearestVisibleLivingEntities,
        Some(MemoryValue::VisibleEntities(vec![9])),
    );
    let mut context = BrainTickContext::default();
    let mut world = staged_food();

    brain.tick(&mut context, &mut world);
    // `EntityTracker.currentPosition` with `trackEyeHeight`.
    assert_eq!(
        world.look_at,
        Some(Vec3d {
            x: 9.0,
            y: 1.0,
            z: 0.0,
        })
    );

    // `EntityTracker.isVisibleBy` refuses an entity that is not in the visible
    // list.
    world.entity_facts = None;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::LookTarget));
}

#[test]
fn swim_jumps_on_a_draw_under_its_chance_and_only_where_the_body_can_move() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::IsInWater);
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(0, crate::entity::ai::swim(0.8, true))],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    // `body.isInWater() && body.getFluidHeight(WATER) >
    // body.getFluidJumpThreshold()`, which reads the body
    // rather than a memory.
    world.body.in_water = true;
    world.body.water_fluid_height = 1.0;

    // With seed 0 the body's stream draws `0.73096776` first, which is under
    // the creaking's 0.8.
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["Swim"]);
    assert_eq!(world.jump_requests, 1);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["Swim"]);
    assert_eq!(
        world.jump_requests, 1,
        "the stream's second float is `0.83144075`, over the 0.8 chance"
    );

    // Lava is water enough for `Swim.shouldSwim`, and the
    // creaking's own override is the only term that refuses a start
    world.body.in_lava = true;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["Swim"]);
    world.body.can_move = false;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(
        brain.running_behaviors().is_empty(),
        "a frozen body neither starts nor keeps swimming"
    );
    world.body.can_move = true;
    world.body.in_lava = false;

    // Shallow water refuses it as well, and so does a body out of water.
    world.body.water_fluid_height = 0.2;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    world.body.water_fluid_height = 1.0;
    world.body.in_water = false;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
}

#[test]
fn set_entity_look_target_sometimes_arms_its_ticker_before_it_looks() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestVisibleLivingEntities);
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(11, set_entity_look_target_sometimes(8.0, 30, 60))],
        &[],
        &[],
    );
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(1, look_at_target_sink())],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::NearestVisibleLivingEntities,
        Some(MemoryValue::VisibleEntities(vec![9])),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.entity_facts = Some(entity_facts(9, "minecraft:player"));
    world.body.position = Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };

    world.entity_facts = Some(entity_facts(9, "minecraft:player"));
    world.body.position = Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    // The `distanceToSqr` the visible-list filter reads (`:31-32`).
    world.distance_to_sqr = Some(4.0);

    // `Ticker.tickDownAndCheck` arms the first call and answers false, so the
    // behaviour does not start and writes nothing
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert!(!brain.has_memory_value(MemoryModuleType::LookTarget));
    assert_eq!(world.look_at, None);
    // The armed interval is `nextInt(31)` off the drawn span, which the level's
    // seam answers at the low end: twenty-nine ticks later the counter reaches
    // zero and the behaviour looks (`:60-67`).
    for _ in 0..28 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert!(!brain.has_memory_value(MemoryModuleType::LookTarget));
    }
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::EntityTracker(9)),
        "the entity the visible list holds"
    );
    // A `OneShot` is done inside the tick it starts, and
    // the aim comes from CORE's sink, which ran before IDLE on the tick the
    // target was written.
    assert!(brain.running_behaviors().is_empty());
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.look_at,
        Some(Vec3d {
            x: 9.0,
            y: 1.0,
            z: 0.0,
        }),
        "`LookAtTargetSink` aims at the tracker the written target names"
    );
}

#[test]
fn do_nothing_holds_its_drawn_duration_and_writes_nothing() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(12, do_nothing(30, 60))],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["DoNothing"]);
    // The level's seam draws the low end of the span, so the duration is thirty
    // and the generic driver ends the behaviour on the first tick past its
    // timestamp.
    for _ in 0..29 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert_eq!(brain.running_behaviors(), vec!["DoNothing"]);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.running_behaviors(),
        vec!["DoNothing"],
        "the thirtieth tick"
    );
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
}

#[test]
fn random_stroll_walks_the_sampled_position_and_erases_it_where_none_was_found() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(12, random_stroll(0.3))],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.land_random_pos = Some(Some(Vec3d {
        x: 4.5,
        y: 64.0,
        z: -2.5,
    }));

    brain.tick(&mut context, &mut world);
    assert!(
        brain.running_behaviors().is_empty(),
        "a `OneShot` ends as it starts"
    );
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(BlockPos { x: 4, y: 64, z: -3 }),
            speed_modifier: 0.3,
            close_enough_dist: 0,
        })),
        "`RandomStroll` floors the sampled position to a block and walks it \\
         (`RandomStroll.java:43-54`)"
    );
    // The stroll is the other caller of the one `LandRandomPos.getPos` seam,
    // and it samples at `body, 10, 7`.
    assert_eq!(world.land_random_pos_calls, vec![(10, 7)]);

    // A sample that found nothing erases the target and refuses the behaviour
    brain.erase_memory(MemoryModuleType::WalkTarget);
    world.land_random_pos = Some(None);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
}

#[test]
fn set_walk_target_from_look_target_walks_the_entity_the_look_target_names() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestVisibleLivingEntities);
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(12, set_walk_target_from_look_target(0.3, 3))],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::LookTarget,
        Some(MemoryValue::EntityTracker(9)),
    );
    brain.set_memory(
        MemoryModuleType::NearestVisibleLivingEntities,
        Some(MemoryValue::VisibleEntities(vec![9])),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.entity_facts = Some(entity_facts(9, "minecraft:player"));

    brain.tick(&mut context, &mut world);
    assert!(
        brain.running_behaviors().is_empty(),
        "a `OneShot` ends as it starts"
    );
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::Entity {
                entity_id: 9,
                last_seen: BlockPos { x: 9, y: 0, z: 0 },
            },
            speed_modifier: 0.3,
            close_enough_dist: 3,
        })),
        "the walk target takes the look target's own tracker \
         (`SetWalkTargetFromLookTarget.java:25`)"
    );
}

#[test]
fn run_one_starts_one_weighted_child_and_ends_with_it() {
    // The shuffle puts the stroll or the idle wait first (the look-target
    // child has no `LOOK_TARGET` and never starts); 200 brains all picking
    // the wait have odds far below 1e-20.
    let mut context = BrainTickContext::default();
    let mut strolled = 0;
    for _ in 0..200 {
        let mut brain = Brain::new();
        brain.add_activity(
            crate::entity::ai::Activity::IDLE,
            [(
                12,
                run_one(
                    Vec::new(),
                    [
                        (2, random_stroll(0.3)),
                        (2, set_walk_target_from_look_target(0.3, 3)),
                        (1, do_nothing(30, 60)),
                    ],
                ),
            )],
            &[],
            &[],
        );
        let mut world = test_world();
        world.land_random_pos = Some(Some(Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        }));
        brain.tick(&mut context, &mut world);
        match brain.memory(MemoryModuleType::WalkTarget) {
            Some(walk_target) => {
                assert_eq!(
                    walk_target,
                    &MemoryValue::WalkTarget(MemoryWalkTarget {
                        target: MemoryPositionTracker::BlockPos(BlockPos { x: 1, y: 64, z: 1 }),
                        speed_modifier: 0.3,
                        close_enough_dist: 0,
                    }),
                    "the stroll's own sampled position"
                );
                // The stroll is a `OneShot`, so the gate ends inside the tick
                // its child ends.
                assert!(brain.running_behaviors().is_empty());
                strolled += 1;
            }
            None => assert!(!brain.running_behaviors().is_empty(), "the wait runs on"),
        }
    }
    assert!(strolled > 0);
}

#[test]
fn run_one_checks_child_memory_and_stops_an_empty_gate_in_its_first_tick() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(12, run_one(Vec::new(), [(1, random_stroll(0.3))]))],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.land_random_pos = Some(Some(Vec3d {
        x: 4.5,
        y: 64.0,
        z: 2.5,
    }));
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget::at(
            BlockPos { x: 1, y: 64, z: 1 },
            0.3,
            0,
        ))),
    );

    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert!(world.land_random_pos_calls.is_empty());

    brain.erase_memory(MemoryModuleType::WalkTarget);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(world.land_random_pos_calls, [(10, 7)]);
    assert!(brain.has_memory_value(MemoryModuleType::WalkTarget));
    assert!(brain.running_behaviors().is_empty());
}

#[test]
fn nested_gate_registers_child_memories_without_strengthening_its_entry_condition() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(
            12,
            run_one(
                vec![(MemoryModuleType::LookTarget, MemoryStatus::ValuePresent)],
                [(1, run_one(Vec::new(), [(1, random_stroll(0.3))]))],
            ),
        )],
        &[],
        &[],
    );
    assert!(brain.check_memory(MemoryModuleType::LookTarget, MemoryStatus::Registered));
    assert!(brain.check_memory(MemoryModuleType::WalkTarget, MemoryStatus::Registered));
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.land_random_pos = Some(Some(Vec3d {
        x: 1.5,
        y: 64.0,
        z: 1.5,
    }));
    brain.tick(&mut context, &mut world);
    assert!(world.land_random_pos_calls.is_empty());
    brain.set_memory(
        MemoryModuleType::LookTarget,
        Some(MemoryValue::BlockPos(BlockPos { x: 1, y: 64, z: 1 })),
    );
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(world.land_random_pos_calls, [(10, 7)]);
}

#[test]
fn ordered_swim_gate_rechecks_walk_target_before_the_next_child() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::SWIM,
        [(
            5,
            ordered_try_all(
                vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
                [
                    (1, random_stroll_swim(0.75)),
                    (1, random_stroll(1.0)),
                    (5, trigger_if_in_water()),
                ],
            ),
        )],
        &[],
        &[],
    );
    brain.set_active_activity_if_possible(crate::entity::ai::Activity::SWIM);
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.in_water = true;
    let swim_target = Vec3d {
        x: 2.5,
        y: 64.0,
        z: 1.5,
    };
    world.swim_target_pos = Some(Some(swim_target));
    world.land_random_pos = Some(Some(Vec3d {
        x: 9.5,
        y: 64.0,
        z: 9.5,
    }));

    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert_eq!(world.swim_target_pos_calls, 1);
    assert!(world.land_random_pos_calls.is_empty());
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(make_walk_target(
            swim_target,
            0.75,
            0
        )))
    );
}

#[test]
fn ordered_swim_gate_tries_the_land_child_after_a_null_swim_sample() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::SWIM,
        [(
            5,
            ordered_try_all(
                vec![(MemoryModuleType::WalkTarget, MemoryStatus::ValueAbsent)],
                [(1, random_stroll_swim(0.75)), (1, random_stroll(1.0))],
            ),
        )],
        &[],
        &[],
    );
    brain.set_active_activity_if_possible(crate::entity::ai::Activity::SWIM);
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.in_water = true;
    world.swim_target_pos = Some(None);
    let land_target = Vec3d {
        x: 3.5,
        y: 64.0,
        z: 2.5,
    };
    world.land_random_pos = Some(Some(land_target));

    brain.tick(&mut context, &mut world);
    assert_eq!(world.swim_target_pos_calls, 1);
    assert_eq!(world.land_random_pos_calls, [(10, 7)]);
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(make_walk_target(
            land_target,
            1.0,
            0
        )))
    );
}

#[test]
fn run_one_holds_while_its_child_runs_and_ends_with_it() {
    let mut brain = Brain::new();
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(12, run_one(Vec::new(), [(1, do_nothing(30, 60))]))],
        &[],
        &[],
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["RunOne"]);
    for _ in 0..30 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert_eq!(
            brain.running_behaviors(),
            vec!["RunOne"],
            "the gate holds while its one child runs"
        );
    }
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(
        brain.running_behaviors().is_empty(),
        "the gate ends inside the tick its last child ends"
    );
}

#[test]
fn melee_attack_hits_a_target_in_reach_once_per_cooldown() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestVisibleLivingEntities);
    // `CreakingAi` registers this in FIGHT; a bare brain
    // activates IDLE and CORE, so the behaviour runs there.
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(11, melee_attack(40, true))],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::AttackTarget,
        Some(MemoryValue::EntityRef(9)),
    );
    brain.register_memory(MemoryModuleType::AttackCoolingDown);
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    // `entity_facts` puts the target at x = 9; the body stands inside its
    // reach, which is the bounding boxes `isWithinMeleeAttackRange` inflates
    world.body.position = Vec3d {
        x: 8.0,
        y: 0.0,
        z: 0.0,
    };
    world.body.bounding_box = bbb_server_types::geometry::Aabb::of_entity(
        Vec3d {
            x: 8.0,
            y: 0.0,
            z: 0.0,
        },
        0.6,
        1.8,
    );
    world.entity_facts = Some(entity_facts(9, "minecraft:player"));
    brain.set_memory(
        MemoryModuleType::NearestVisibleLivingEntities,
        Some(MemoryValue::VisibleEntities(vec![9])),
    );
    world.dies_on_hurt = false;

    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.effects,
        vec![
            BrainWorldEffect::SwingMainHand,
            BrainWorldEffect::MeleeAttack { target_id: 9 },
        ],
        "the swing comes before the hurt (`MeleeAttack.java:33-36`)"
    );
    assert_eq!(
        brain.memory(MemoryModuleType::AttackCoolingDown),
        Some(&MemoryValue::Bool(true)),
        "`attackCoolingDown.setWithExpiry(true, 40)` (`MeleeAttack.java:35`)"
    );

    // The cooldown slot is a `MemorySlot` with an expiry of forty ticks, so the
    // behaviour's entry condition refuses every tick until it clears
    for _ in 0..40 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert_eq!(world.effects.len(), 2, "no second hit inside the cooldown");
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(world.effects.len(), 4, "the forty-first tick attacks again");
}

#[test]
fn start_attacking_when_active_refuses_a_body_that_is_not_active() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestVisibleAttackablePlayer);
    brain.register_memory(MemoryModuleType::AttackTarget);
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(10, crate::entity::ai::start_attacking_when_active())],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::NearestVisibleAttackablePlayer,
        Some(MemoryValue::EntityRef(7)),
    );
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    brain.tick(&mut context, &mut world);
    assert!(
        !brain.has_memory_value(MemoryModuleType::AttackTarget),
        "`Creaking::isActive` (`CreakingAi.java:44`) answers false"
    );

    world.body.is_active = true;
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::AttackTarget),
        Some(&MemoryValue::EntityRef(7))
    );
}

/// A brain with the frog's IDLE package: `FollowTemptation` at priority 1 and
/// `StartAttacking` at 2, plus CORE's
/// `CountDownCooldownTicks` over the cooldown the first one writes on stop
fn brain_with_temptation() -> Brain {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::NearestAttackable);
    brain.add_activity(
        crate::entity::ai::Activity::CORE,
        [(
            3,
            count_down_cooldown_ticks(MemoryModuleType::TemptationCooldownTicks),
        )],
        &[],
        &[],
    );
    brain.add_activity(
        crate::entity::ai::Activity::IDLE,
        [(1, follow_temptation(1.25)), (2, start_attacking())],
        &[],
        &[],
    );
    brain.set_memory(
        MemoryModuleType::TemptingPlayer,
        Some(MemoryValue::EntityRef(9)),
    );
    brain
}

/// The player `TemptingSensor` would have written sits four blocks away, which
/// is outside `DEFAULT_CLOSE_ENOUGH_DIST`.
fn tempting_player() -> crate::entity::ai::brain::tests::TestWorld {
    let mut world = test_world();
    world.body.position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    world.body.block_position = BlockPos { x: 0, y: 64, z: 0 };
    world.entity_facts = Some(BrainEntityFacts {
        position: Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
        ..entity_facts(9, "minecraft:player")
    });
    world.distance_to_sqr = Some(16.0);
    world
}

#[test]
fn follow_temptation_writes_the_player_tracker_and_walks_to_it() {
    let mut brain = brain_with_temptation();
    let mut context = BrainTickContext::default();
    let mut world = tempting_player();

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["FollowTemptation"]);
    // `start`.
    assert_eq!(
        brain.memory(MemoryModuleType::IsTempted),
        Some(&MemoryValue::Bool(true))
    );
    // `brain.setMemory(LOOK_TARGET, new EntityTracker(player, true))` (`:87`),
    // which CORE's `LookAtTargetSink` aims the body with.
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::EntityTracker(9))
    );
    // `new WalkTarget(new EntityTracker(player, false, false), 1.25F, 2)`
    // (`:92-94`): the player's own tracker, the frog's speed multiplier and
    // `closeEnoughDist` 2.
    assert_eq!(
        brain.memory(MemoryModuleType::WalkTarget),
        Some(&MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::Entity {
                entity_id: 9,
                last_seen: BlockPos { x: 4, y: 64, z: 0 },
            },
            speed_modifier: 1.25,
            close_enough_dist: 2,
        }))
    );
}

#[test]
fn follow_temptation_erases_the_walk_target_inside_its_close_enough_distance() {
    let mut brain = brain_with_temptation();
    let mut context = BrainTickContext::default();
    let mut world = tempting_player();
    // `DEFAULT_CLOSE_ENOUGH_DIST` is 2.5, so the comparison is against 6.25
    world.distance_to_sqr = Some(6.0);
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(BlockPos { x: 4, y: 64, z: 0 }),
            speed_modifier: 1.25,
            close_enough_dist: 2,
        })),
    );

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["FollowTemptation"]);
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
    // The look target is written before the walk target is judged (`:87-90`).
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::EntityTracker(9))
    );

    // Exactly the squared distance is not inside it.
    world.distance_to_sqr = Some(6.25);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.has_memory_value(MemoryModuleType::WalkTarget));
}

#[test]
fn follow_temptation_stops_on_the_cooldown_the_tempting_player_ends() {
    let mut brain = brain_with_temptation();
    let mut context = BrainTickContext::default();
    let mut world = tempting_player();

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["FollowTemptation"]);

    // The sensor erased `TEMPTING_PLAYER`, so `canStillUse` refuses and the
    // stop writes the cooldown CORE counts down (`:66-70`, `:76-82`).
    brain.erase_memory(MemoryModuleType::TemptingPlayer);
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::TemptationCooldownTicks),
        Some(&MemoryValue::Int(100))
    );
    assert!(!brain.has_memory_value(MemoryModuleType::IsTempted));
    assert!(!brain.has_memory_value(MemoryModuleType::WalkTarget));
    assert!(!brain.has_memory_value(MemoryModuleType::LookTarget));
    // CORE's `CountDownCooldownTicks` is the other half of the gate: it runs
    // over the cooldown the stop left behind. Its start
    // conditions were read before the stop wrote the memory, so it begins on
    // the next tick.
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["CountDownCooldownTicks"]);
    assert_eq!(
        brain.memory(MemoryModuleType::TemptationCooldownTicks),
        Some(&MemoryValue::Int(99))
    );

    // While the cooldown is set the entry condition refuses a fresh start even
    // with a tempting player back in the memory (`:41`).
    brain.set_memory(
        MemoryModuleType::TemptingPlayer,
        Some(MemoryValue::EntityRef(9)),
    );
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.running_behaviors(),
        vec!["CountDownCooldownTicks"],
        "`FollowTemptation` stays refused while the cooldown answers"
    );
}

#[test]
fn follow_temptation_refuses_a_breeding_or_panicking_body() {
    for memory_type in [MemoryModuleType::BreedTarget, MemoryModuleType::IsPanicking] {
        let mut brain = brain_with_temptation();
        let mut context = BrainTickContext::default();
        let mut world = tempting_player();
        brain.set_memory(memory_type, Some(MemoryValue::EntityRef(3)));

        // The entry condition asks for both memories absent (`:45-46`).
        brain.tick(&mut context, &mut world);
        assert!(
            brain.running_behaviors().is_empty(),
            "{memory_type} refuses the start"
        );

        // `canStillUse` asks the same of a body that already follows (`:66-70`).
        brain.erase_memory(memory_type);
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert_eq!(brain.running_behaviors(), vec!["FollowTemptation"]);
        brain.set_memory(memory_type, Some(MemoryValue::EntityRef(3)));
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert!(
            brain.running_behaviors().is_empty(),
            "{memory_type} stops the run"
        );
    }
}
