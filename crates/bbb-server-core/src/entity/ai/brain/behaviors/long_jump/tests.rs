use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};

use super::*;
use crate::entity::ai::brain::tests::{test_world, TestWorld};
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{
    Behavior, Brain, BrainBlockFacts, BrainWorldEffect, MemoryModuleType, MemoryValue,
};
use crate::entity::GroundPathType;

/// The body's own position and block position, and the first cell
/// `BlockPos.betweenClosed` walks over `(mobX - 4, mobY - 2, mobZ - 4)`
/// which is the candidate the weighted pick takes
/// when the fixture draws zero.
const MOB: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};
const BODY_BLOCK: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const CANDIDATE: BlockPos = BlockPos {
    x: -4,
    y: 62,
    z: -4,
};

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

/// The air state's own facts.
fn air() -> BrainBlockFacts {
    BrainBlockFacts {
        is_air: true,
        is_solid_render: false,
        has_fluid: false,
        fluid_is_water: false,
        fluid_supports_frogspawn: false,
        supports_frogspawn: false,
        prefers_frog_jump: false,
        is_water_block: false,
        collision_shape_empty: true,
        collision_face_up_empty: true,
        face_sturdy_up: false,
    }
}

/// The block under a landing spot: no fluid, no shape of its own, and a solid
/// render.
fn landing_block() -> BrainBlockFacts {
    BrainBlockFacts {
        is_solid_render: true,
        ..air()
    }
}

fn brain_with(behavior: Behavior) -> Brain {
    let mut brain = Brain::new();
    for memory_type in [
        MemoryModuleType::LookTarget,
        MemoryModuleType::WalkTarget,
        MemoryModuleType::LongJumpCooldownTicks,
        MemoryModuleType::LongJumpMidJump,
    ] {
        brain.register_memory(memory_type);
    }
    brain.add_activity(crate::entity::ai::Activity::IDLE, [(0, behavior)], &[], &[]);
    brain
}

fn mid_jump_brain() -> Brain {
    let mut brain = brain_with(long_jump_mid_jump(100, 140, "minecraft:entity.frog.step"));
    brain.set_memory(
        MemoryModuleType::LongJumpMidJump,
        Some(MemoryValue::Bool(true)),
    );
    brain
}

fn long_jump_block() -> Behavior {
    long_jump_to_preferred_block(
        100,
        140,
        2,
        4,
        3.571_428_8,
        "minecraft:entity.frog.long_jump",
        0.5,
    )
}

/// A world whose first jump candidate is acceptable: the body stands on the
/// ground, dry and off honey, and the target and the block under it answer
/// every read `FrogAi.isAcceptableLandingSpot` makes.
fn jump_world() -> TestWorld {
    let mut world = test_world();
    world.body.position = MOB;
    world.body.block_position = BODY_BLOCK;
    world.body.bounding_box = bbb_server_types::geometry::Aabb::of_entity(MOB, 0.5, 0.5);
    world.body.y_body_rot = 90.0;
    world.body.on_ground = true;
    world.body.jump_strength = 0.42;
    world.body.gravity = 0.08;
    world.pathfinding_malus = 0.0;
    // `!level.getBlockState(body.blockPosition()).is(Blocks.HONEY_BLOCK)`
    // is the state's own `STICKY_HONEY` type
    world.path_types = vec![(BODY_BLOCK, GroundPathType::Open)];
    world.block_facts = vec![
        (CANDIDATE, air()),
        (
            pos(CANDIDATE.x, CANDIDATE.y - 1, CANDIDATE.z),
            landing_block(),
        ),
        (pos(CANDIDATE.x, CANDIDATE.y + 1, CANDIDATE.z), air()),
    ];
    world.path_types.push((CANDIDATE, GroundPathType::Open));
    world.path_types.push((
        pos(CANDIDATE.x, CANDIDATE.y - 1, CANDIDATE.z),
        GroundPathType::Open,
    ));
    world
}

fn delta_movement(world: &TestWorld) -> Option<Vec3d> {
    world.effects.iter().find_map(|effect| match effect {
        BrainWorldEffect::SetBodyDeltaMovement { movement } => Some(*movement),
        _ => None,
    })
}

#[test]
fn mid_jump_holds_the_body_off_the_ground_and_lands_it() {
    let mut brain = mid_jump_brain();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.on_ground = false;

    brain.tick(&mut context, &mut world);
    // `LongJumpMidJump.start` (`:28-31`): the friction flag, then the pose.
    assert_eq!(
        world.effect_names(),
        vec!["SetBodyDiscardFriction", "SetPose"]
    );
    assert_eq!(
        world.effects[1],
        BrainWorldEffect::SetPose {
            pose_id: POSE_LONG_JUMPING
        }
    );
    assert_eq!(brain.running_behaviors(), vec!["LongJumpMidJump"]);

    // `!body.onGround()` (`:24-26`) still holds, so the landing is not yet.
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(world.effect_names().len(), 2);

    // The tick after touchdown: `body.onGround()` is true, so `stop` damps the
    // horizontal delta and plays the landing sound before it releases the
    // friction and the pose (`:33-43`).
    world.body.on_ground = true;
    world.body.delta_movement = Vec3d {
        x: 0.25,
        y: -0.4,
        z: -0.5,
    };
    world.effects.clear();
    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.effect_names(),
        vec![
            "SetBodyDeltaMovement",
            "PlayBodySound",
            "SetBodyDiscardFriction",
            "SetPose",
        ]
    );
    // `body.getDeltaMovement().multiply(0.1F, 1.0, 0.1F)` (`:35`).
    let damped = delta_movement(&world).expect("the landing damps the body");
    assert!((damped.x - 0.025).abs() < 1e-9);
    assert_eq!(damped.y, -0.4);
    assert!((damped.z + 0.05).abs() < 1e-9);
    assert_eq!(
        world.sounds(),
        vec![("minecraft:entity.frog.step", 2.0, 1.0)]
    );
    // `eraseMemory(LONG_JUMP_MID_JUMP)` then the full cooldown sample
    // (`:41-42`), which this fixture draws at the low end.
    assert!(!brain.has_memory_value(MemoryModuleType::LongJumpMidJump));
    assert_eq!(
        brain.memory(MemoryModuleType::LongJumpCooldownTicks),
        Some(&MemoryValue::Int(100))
    );
    assert!(brain.running_behaviors().is_empty());
}

#[test]
fn mid_jump_releases_the_body_without_the_landing_sound_when_it_times_out() {
    let mut brain = mid_jump_brain();
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.on_ground = false;

    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["LongJumpMidJump"]);
    // `Behavior.timedOut` closes the hundred-tick window on the first tick
    // past its end timestamp (`Behavior`)
    world.effects.clear();
    while !brain.running_behaviors().is_empty() {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
        assert!(
            context.game_time <= 200,
            "the timeout must close the window"
        );
    }
    // `if (body.onGround())` (`:34`) is the only arm the sound and the damping
    // are in.
    assert_eq!(
        world.effect_names(),
        vec!["SetBodyDiscardFriction", "SetPose"]
    );
    assert_eq!(
        brain.memory(MemoryModuleType::LongJumpCooldownTicks),
        Some(&MemoryValue::Int(100))
    );
}

#[test]
fn the_jump_aborts_at_half_the_cooldown_when_the_body_cannot_start() {
    // `body.onGround() && !body.isInWater() && !body.isInLava() && !honey`
    // one refusal per case.
    for case in 0..4 {
        let mut brain = brain_with(long_jump_block());
        let mut context = BrainTickContext::default();
        let mut world = jump_world();
        match case {
            0 => world.body.on_ground = false,
            1 => world.body.in_water = true,
            2 => world.body.in_lava = true,
            _ => world.path_types = vec![(BODY_BLOCK, GroundPathType::StickyHoney)],
        }

        brain.tick(&mut context, &mut world);
        assert!(
            brain.running_behaviors().is_empty(),
            "case {case} must not start the jump"
        );
        // `timeBetweenLongJumps.sample(level.getRandom()) / 2` (`:96`).
        assert_eq!(
            brain.memory(MemoryModuleType::LongJumpCooldownTicks),
            Some(&MemoryValue::Int(50)),
            "case {case} writes the halved cooldown"
        );
        assert_eq!(brain.memory(MemoryModuleType::LookTarget), None);
    }
}

#[test]
fn the_jump_launches_the_vector_it_solved_forty_ticks_later() {
    let mut brain = brain_with(long_jump_block());
    let mut context = BrainTickContext::default();
    let mut world = jump_world();

    // `checkExtraStartConditions`, `start`'s candidate sweep and the first
    // `tick`'s `pickCandidate` all run inside this one tick
    brain.tick(&mut context, &mut world);
    assert_eq!(brain.running_behaviors(), vec!["LongJumpToPreferredBlock"]);
    // `body.getBrain().setMemory(LOOK_TARGET, new BlockPosTracker(targetPos))`
    // (`:164`).
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::BlockPos(CANDIDATE))
    );
    assert!(!brain.has_memory_value(MemoryModuleType::LongJumpMidJump));

    // `prepareJumpStart` is this tick's timestamp, so the next thirty-nine
    // ticks change nothing (`:138`).
    world.effects.clear();
    for _ in 0..39 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert!(world.effects.is_empty(), "{:?}", world.effect_names());

    context.game_time += 1;
    brain.tick(&mut context, &mut world);
    // `setYRot(body.yBodyRot)`, `setDiscardFriction(true)`,
    // `setDeltaMovement(chosenJump)`, the mid-jump memory, then the sound
    // (`:140-146`).
    assert_eq!(
        world.effect_names(),
        vec![
            "SetBodyYRot",
            "SetBodyDiscardFriction",
            "SetBodyDeltaMovement",
            "PlayBodySound",
        ]
    );
    assert_eq!(
        world.effects[0],
        BrainWorldEffect::SetBodyYRot { y_rot: 90.0 }
    );
    assert_eq!(
        world.effects[1],
        BrainWorldEffect::SetBodyDiscardFriction { discard: true }
    );
    // The solved vector points at the candidate, which is north and west of
    // the body and below it: the arc has to climb and travel that way.
    let launch = delta_movement(&world).expect("the launch writes the body's delta");
    assert!(launch.x < 0.0 && launch.z < 0.0, "{launch:?}");
    assert!(launch.y > 0.0, "{launch:?}");
    assert_eq!(
        world.effects[3],
        BrainWorldEffect::PlayBodySound {
            resource_id: "minecraft:entity.frog.long_jump",
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        }
    );
    assert_eq!(
        brain.memory(MemoryModuleType::LongJumpMidJump),
        Some(&MemoryValue::Bool(true))
    );
}

#[test]
fn a_body_that_moved_off_its_start_aborts_the_chosen_jump() {
    let mut brain = brain_with(long_jump_block());
    let mut context = BrainTickContext::default();
    let mut world = jump_world();
    brain.tick(&mut context, &mut world);
    assert!(!brain.has_memory_value(MemoryModuleType::LongJumpMidJump));
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::BlockPos(CANDIDATE))
    );

    // `initialPosition.get().equals(body.position())` (`:103`) fails once the
    // body has moved, and the mid-jump memory is still absent, so the invalid
    // arm writes the halved cooldown and erases the look target (`:108-111`).
    world.body.position = Vec3d {
        x: MOB.x + 1.0,
        ..MOB
    };
    context.game_time += 1;
    brain.tick(&mut context, &mut world);

    assert!(brain.running_behaviors().is_empty());
    assert_eq!(
        brain.memory(MemoryModuleType::LongJumpCooldownTicks),
        Some(&MemoryValue::Int(50))
    );
    assert_eq!(brain.memory(MemoryModuleType::LookTarget), None);
}

#[test]
fn the_jump_is_not_taken_when_the_navigation_can_already_reach_the_target() {
    let mut brain = brain_with(long_jump_block());
    let mut context = BrainTickContext::default();
    let mut world = jump_world();
    // `if (path == null || !path.canReach())` (`:167`): a path that reaches
    // leaves the candidate behind and no jump is made.
    world.path_reaches = Some(true);

    brain.tick(&mut context, &mut world);
    // The probe carries the third argument the provider wrote down
    // (`MIN_PATHFIND_DISTANCE_TO_VALID_JUMP`, `:166`).
    assert_eq!(
        world.path_reaches_calls.borrow().as_slice(),
        &[(CANDIDATE, 0, 8.0)]
    );
    world.effects.clear();
    for _ in 0..45 {
        context.game_time += 1;
        brain.tick(&mut context, &mut world);
    }
    assert!(!brain.has_memory_value(MemoryModuleType::LongJumpMidJump));
    assert!(world.effects.is_empty(), "{:?}", world.effect_names());
}

/// The candidate the jump would otherwise take, refused by one read.
fn refused_jump(tweak: impl FnOnce(&mut TestWorld)) {
    let mut brain = brain_with(long_jump_block());
    let mut context = BrainTickContext::default();
    let mut world = jump_world();
    tweak(&mut world);
    for _ in 0..45 {
        brain.tick(&mut context, &mut world);
        context.game_time += 1;
    }
    assert!(
        !brain.has_memory_value(MemoryModuleType::LongJumpMidJump),
        "the jump must not be taken"
    );
    assert_eq!(brain.memory(MemoryModuleType::LookTarget), None);
}

#[test]
fn a_target_with_fluid_around_it_is_not_a_landing_spot() {
    // `level.getFluidState(targetPos).isEmpty() &&
    // level.getFluidState(below).isEmpty() &&
    // level.getFluidState(targetPos.above()).isEmpty()`.
    for fluid_cell in [
        CANDIDATE,
        pos(CANDIDATE.x, CANDIDATE.y - 1, CANDIDATE.z),
        pos(CANDIDATE.x, CANDIDATE.y + 1, CANDIDATE.z),
    ] {
        refused_jump(|world| {
            let entry = world
                .block_facts
                .iter_mut()
                .find(|(staged, _)| *staged == fluid_cell)
                .expect("the staged cell");
            entry.1.has_fluid = true;
        });
    }
}

#[test]
fn a_target_over_a_block_that_is_not_solid_render_is_refused() {
    // `LongJumpToRandomPos.defaultAcceptableLandingSpot` (`:60-64`).
    refused_jump(|world| {
        let below = pos(CANDIDATE.x, CANDIDATE.y - 1, CANDIDATE.z);
        let entry = world
            .block_facts
            .iter_mut()
            .find(|(staged, _)| *staged == below)
            .expect("the staged cell");
        entry.1.is_solid_render = false;
    });
}

#[test]
fn a_target_the_bodys_own_malus_refuses_is_refused() {
    let mut brain = brain_with(long_jump_block());
    let mut context = BrainTickContext::default();
    let mut world = jump_world();
    // `body.getPathfindingMalus(WalkNodeEvaluator.getPathTypeStatic(body,
    // targetPos)) == 0.0F` (`:63`).
    world.pathfinding_malus = 8.0;

    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.pathfinding_malus_calls.borrow().as_slice(),
        &[GroundPathType::Open]
    );
    assert!(!brain.has_memory_value(MemoryModuleType::LongJumpMidJump));
}

#[test]
fn a_frog_preferred_block_and_a_trapdoor_are_accepted_outright() {
    // `:212-213` answers true for a preferred block below and `:216-218` for a
    // trapdoor target, both without asking the malus at all.
    for case in 0..2 {
        let mut brain = brain_with(long_jump_block());
        let mut context = BrainTickContext::default();
        let mut world = jump_world();
        // A malus that would refuse the cell if the spot ever asked for it.
        world.pathfinding_malus = 8.0;
        let below = pos(CANDIDATE.x, CANDIDATE.y - 1, CANDIDATE.z);
        match case {
            0 => {
                let entry = world
                    .block_facts
                    .iter_mut()
                    .find(|(staged, _)| *staged == below)
                    .expect("the staged cell");
                entry.1.prefers_frog_jump = true;
            }
            _ => {
                let entry = world
                    .path_types
                    .iter_mut()
                    .find(|(staged, _)| *staged == CANDIDATE)
                    .expect("the staged path type");
                entry.1 = GroundPathType::Trapdoor;
            }
        }

        brain.tick(&mut context, &mut world);
        assert!(
            world.pathfinding_malus_calls.borrow().is_empty(),
            "case {case} answers before the malus is read"
        );
        assert_eq!(
            brain.memory(MemoryModuleType::LookTarget),
            Some(&MemoryValue::BlockPos(CANDIDATE)),
            "case {case} accepts the candidate"
        );
    }
}
