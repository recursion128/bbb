use super::*;
use crate::entity::ai::brain::tests::test_world;
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{Behavior, Brain, BrainWorldEffect, MemoryModuleType};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROAKING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};

/// One brain whose IDLE activity is exactly `behavior`.
fn brain_with(behavior: Behavior) -> Brain {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::WalkTarget);
    brain.add_activity(crate::entity::ai::Activity::IDLE, [(0, behavior)], &[], &[]);
    brain
}

#[test]
fn croak_holds_the_croaking_pose_for_sixty_ticks_and_restores_it() {
    let mut brain = brain_with(croak());
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.pose_id = VANILLA_ENTITY_POSE_STANDING_ID;

    brain.tick(&mut context, &mut world);
    // `Croak.start` (`:27-32`): a dry body starts the croak counter and puts
    // on the pose.
    assert_eq!(
        world.effects,
        vec![BrainWorldEffect::SetPose {
            pose_id: VANILLA_ENTITY_POSE_CROAKING_ID
        }]
    );
    assert_eq!(brain.running_behaviors(), vec!["Croak"]);

    // `canStillUse` (`:23-25`) keeps it for the first sixty counter ticks.
    for _ in 0..59 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert_eq!(brain.running_behaviors(), vec!["Croak"]);
    }
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    // `Croak.stop` (`:34-36`).
    assert_eq!(
        world.effects.last(),
        Some(&BrainWorldEffect::SetPose {
            pose_id: VANILLA_ENTITY_POSE_STANDING_ID
        })
    );
}

#[test]
fn croak_leaves_a_body_in_liquid_standing() {
    let mut brain = brain_with(croak());
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.pose_id = VANILLA_ENTITY_POSE_STANDING_ID;
    // `body.isInLiquid()`, over water here.
    world.body.in_water = true;

    brain.tick(&mut context, &mut world);
    // The croak counter is never reset, so the behaviour still runs and stops
    // on it alone (`:28-31`).
    assert!(world.effects.is_empty());
    assert_eq!(brain.running_behaviors(), vec!["Croak"]);
    for _ in 0..61 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    // No tick of this body ever put the croaking pose on: the only pose it
    // wrote is the standing one `stop` writes.
    assert!(
        world.effects.iter().all(|effect| *effect
            == BrainWorldEffect::SetPose {
                pose_id: VANILLA_ENTITY_POSE_STANDING_ID
            }),
        "{:?}",
        world.effects
    );
    assert!(!world.effects.is_empty(), "the behaviour did run and stop");
}

#[test]
fn croak_refuses_a_body_that_is_not_standing() {
    let mut brain = brain_with(croak());
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    // `body.getPose() == Pose.STANDING` (`:19-21`).
    world.body.pose_id = VANILLA_ENTITY_POSE_CROAKING_ID;

    brain.tick(&mut context, &mut world);
    assert!(brain.running_behaviors().is_empty());
    assert!(world.effects.is_empty());
}
