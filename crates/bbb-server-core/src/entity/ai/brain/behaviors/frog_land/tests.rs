use bbb_protocol::packets::{BlockPos, SoundSource};

use super::*;
use crate::entity::ai::brain::tests::test_world;
use crate::entity::ai::BrainTickContext;
use crate::entity::ai::{
    Behavior, Brain, BrainBlockFacts, BrainWorldEffect, MemoryPositionTracker, MemoryWalkTarget,
};

/// The air state's own facts, which is what every position a test does not
/// stage answers.
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

fn water() -> BrainBlockFacts {
    BrainBlockFacts {
        is_air: false,
        has_fluid: true,
        fluid_is_water: true,
        is_water_block: true,
        ..air()
    }
}

/// A cell no fluid occupies, whose own shape is empty: the two terms
/// `TryFindLand` adds to a candidate.
fn dry() -> BrainBlockFacts {
    air()
}

/// A block a body can stand on: a collision shape of its own, a solid render,
/// and a sturdy face upwards.
fn solid() -> BrainBlockFacts {
    BrainBlockFacts {
        is_air: false,
        is_solid_render: true,
        collision_shape_empty: false,
        collision_face_up_empty: false,
        face_sturdy_up: true,
        ..air()
    }
}

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

/// The overrides first, then one entry per cell of the box they do not name,
/// so every position a scan reaches has an answer and the engine cannot report
/// a cell as unanswerable by accident. `crate::entity::ai::brain::tests::TestWorld`
/// resolves a staged position by its first match.
fn staged(
    overrides: Vec<(BlockPos, BrainBlockFacts)>,
    min: BlockPos,
    max: BlockPos,
    base: BrainBlockFacts,
) -> Vec<(BlockPos, BrainBlockFacts)> {
    let mut staged = overrides;
    for z in min.z..=max.z {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let candidate = pos(x, y, z);
                if !staged.iter().any(|(staged, _)| *staged == candidate) {
                    staged.push((candidate, base));
                }
            }
        }
    }
    staged
}

/// One brain whose IDLE activity is exactly `behavior`, with the memories the
/// three packages' entry conditions name registered.
fn brain_with(behavior: Behavior) -> Brain {
    let mut brain = Brain::new();
    for memory_type in [
        MemoryModuleType::WalkTarget,
        MemoryModuleType::LookTarget,
        MemoryModuleType::AttackTarget,
        MemoryModuleType::IsPregnant,
    ] {
        brain.register_memory(memory_type);
    }
    brain.add_activity(crate::entity::ai::Activity::IDLE, [(0, behavior)], &[], &[]);
    brain
}

fn walk_target(brain: &Brain) -> Option<MemoryWalkTarget> {
    match brain.memory(MemoryModuleType::WalkTarget) {
        Some(MemoryValue::WalkTarget(target)) => Some(*target),
        _ => None,
    }
}

#[test]
fn try_find_land_walks_the_first_cell_every_term_accepts() {
    let mut brain = brain_with(try_find_land(1, 1.5));
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    // `level.getFluidState(body.blockPosition()).is(FluidTags.WATER)` (`:25`).
    world.body.block_position = pos(0, 64, 0);
    // Exactly one cell of the reach qualifies, so the assertion holds whatever
    // order `BlockPos.withinManhattan` walks the box in. Every other cell is
    // dry but has no sturdy block below (`:45`), and the body's own column is
    // out of reach outright (`:39`).
    world.block_facts = staged(
        vec![
            (pos(0, 64, 0), water()),
            (pos(0, 64, 1), dry()),
            (pos(0, 63, 1), solid()),
        ],
        pos(-1, 62, -1),
        pos(1, 65, 1),
        dry(),
    );

    brain.tick(&mut context, &mut world);

    // A `OneShot` is already `Stopped` when the tick that started it ends
    // so its run is what the world was asked for.
    assert!(!world.block_facts_calls.borrow().is_empty());
    // `lookTarget.set(new BlockPosTracker(targetPos))` (`:47`).
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::BlockPos(pos(0, 64, 1)))
    );
    // `walkTarget.set(new WalkTarget(new BlockPosTracker(targetPos),
    // speedModifier, 1))` (`:48`).
    assert_eq!(
        walk_target(&brain),
        Some(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(pos(0, 64, 1)),
            speed_modifier: 1.5,
            close_enough_dist: 1,
        })
    );
}

#[test]
fn try_find_land_skips_a_water_block_and_a_cell_without_a_sturdy_below() {
    let mut brain = brain_with(try_find_land(1, 1.0));
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.block_position = pos(0, 64, 0);
    world.block_facts = staged(
        vec![
            (pos(0, 64, 0), water()),
            // `!state.is(Blocks.WATER) && level.getFluidState(pos).isEmpty()`
            // (`:42-43`): a water block is refused even with a sturdy block
            // below it.
            (pos(0, 64, 1), water()),
            (pos(0, 63, 1), solid()),
            // `belowState.isFaceSturdy(level, belowPos, Direction.UP)` (`:45`):
            // a dry cell over air is refused.
            (pos(0, 64, -1), dry()),
            // The one cell all four terms accept.
            (pos(1, 64, 0), dry()),
            (pos(1, 63, 0), solid()),
        ],
        pos(-1, 62, -1),
        pos(1, 65, 1),
        dry(),
    );

    brain.tick(&mut context, &mut world);

    assert_eq!(
        walk_target(&brain),
        Some(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(pos(1, 64, 0)),
            speed_modifier: 1.0,
            close_enough_dist: 1,
        })
    );
}

#[test]
fn try_find_land_refuses_a_body_that_is_not_in_water() {
    let mut brain = brain_with(try_find_land(2, 1.0));
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.block_facts = vec![(pos(0, 0, 0), air())];

    brain.tick(&mut context, &mut world);
    assert_eq!(walk_target(&brain), None);
}

#[test]
fn try_find_land_near_water_walks_the_cell_a_horizontal_side_reaches_water_from() {
    let mut brain = brain_with(try_find_land_near_water(1, 1.0));
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    // `level.getFluidState(body.blockPosition()).is(FluidTags.WATER)` (`:22`)
    // refuses a body that is already swimming.
    world.body.block_position = pos(0, 64, 0);
    // `state.getCollisionShape(...).isEmpty() &&
    // !belowState.getCollisionShape(...).isEmpty()` (`:38-39`) leaves
    // (0,64,-1) as the only candidate, and of its four horizontal sides
    // (`Direction.Plane.HORIZONTAL`) only the north one
    // is air over a water block.
    world.block_facts = staged(
        vec![
            (pos(0, 64, -1), dry()),
            (pos(0, 63, -1), solid()),
            (pos(0, 64, -2), air()),
            (pos(0, 63, -2), water()),
        ],
        pos(-2, 62, -2),
        pos(2, 65, 2),
        air(),
    );

    brain.tick(&mut context, &mut world);

    assert!(!world.block_facts_calls.borrow().is_empty());
    // `walkTarget.set(new WalkTarget(new BlockPosTracker(pos), speedModifier,
    // 0))` (`:44`).
    assert_eq!(
        walk_target(&brain),
        Some(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(pos(0, 64, -1)),
            speed_modifier: 1.0,
            close_enough_dist: 0,
        })
    );
    assert_eq!(
        brain.memory(MemoryModuleType::LookTarget),
        Some(&MemoryValue::BlockPos(pos(0, 64, -1)))
    );
}

#[test]
fn try_find_land_near_water_refuses_a_candidate_whose_below_block_is_empty() {
    let mut brain = brain_with(try_find_land_near_water(1, 1.0));
    let mut context = BrainTickContext::default();
    let mut world = test_world();
    world.body.block_position = pos(0, 64, 0);
    // The only candidate with a solid block below is (0,64,-1), and every side
    // of it is air over air (`:42`).
    world.block_facts = staged(
        vec![(pos(0, 63, -1), solid())],
        pos(-2, 62, -2),
        pos(2, 65, 2),
        air(),
    );

    brain.tick(&mut context, &mut world);
    assert!(!world.block_facts_calls.borrow().is_empty());
    assert_eq!(walk_target(&brain), None);
}

fn pregnant_brain(behavior: Behavior) -> (Brain, BrainTickContext) {
    let mut brain = brain_with(behavior);
    brain.set_memory(
        MemoryModuleType::WalkTarget,
        Some(MemoryValue::WalkTarget(MemoryWalkTarget {
            target: MemoryPositionTracker::BlockPos(pos(0, 64, 1)),
            speed_modifier: 1.0,
            close_enough_dist: 0,
        })),
    );
    brain.set_memory(MemoryModuleType::IsPregnant, Some(MemoryValue::Unit));
    (brain, BrainTickContext::default())
}

#[test]
fn try_lay_spawn_places_the_block_then_plays_the_sound_and_clears_the_pregnancy() {
    let (mut brain, mut context) = pregnant_brain(try_lay_spawn_on_fluid_near_land(4242));
    let mut world = test_world();
    world.body.block_position = pos(0, 64, 0);
    world.body.on_ground = true;
    world.block_facts = vec![
        // `body.blockPosition().below()` (`:24`) and its NORTH side, whose
        // fluid supports frogspawn (`:30`) and whose own face is empty (`:28`).
        (pos(0, 63, 0), solid()),
        (
            pos(0, 63, -1),
            BrainBlockFacts {
                fluid_supports_frogspawn: true,
                ..air()
            },
        ),
        // `level.getBlockState(spawnPos).isAir()` (`:34`).
        (pos(0, 64, -1), air()),
    ];

    brain.tick(&mut context, &mut world);

    // `level.setBlock(spawnPos, newState, 3)` with its
    // `GameEvent.BLOCK_PLACE` precedes
    // `level.playSound(null, body, FROG_LAY_SPAWN, BLOCKS, 1.0F, 1.0F)`
    // (`:36-38`).
    assert_eq!(
        world.effect_names(),
        vec!["PlaceBlock", "PlayBodySound", "SetMobBrainMemory"],
        "the block write precedes the sound"
    );
    assert_eq!(
        world.effects[0],
        BrainWorldEffect::PlaceBlock {
            pos: pos(0, 64, -1),
            block_state_id: 4242,
        }
    );
    assert_eq!(
        world.effects[1],
        BrainWorldEffect::PlayBodySound {
            resource_id: "minecraft:entity.frog.lay_spawn",
            source: SoundSource::Blocks,
            volume: 1.0,
            pitch: 1.0,
        }
    );
    assert!(brain.has_memory_value(MemoryModuleType::IsPregnant));
    assert_eq!(
        world.effects[2],
        BrainWorldEffect::SetMobBrainMemory {
            entity_id: world.body.entity_id,
            memory_type: MemoryModuleType::IsPregnant,
            value: None,
        }
    );
    brain.erase_memory(MemoryModuleType::IsPregnant);
    assert!(!brain.has_memory_value(MemoryModuleType::IsPregnant));
}

#[test]
fn try_lay_spawn_accepts_a_block_tag_where_the_fluid_tag_does_not() {
    let (mut brain, mut context) = pregnant_brain(try_lay_spawn_on_fluid_near_land(4242));
    let mut world = test_world();
    world.body.block_position = pos(0, 64, 0);
    world.body.on_ground = true;
    world.block_facts = vec![
        (pos(0, 63, 0), solid()),
        // `level.getBlockState(relativePos).is(BlockTags.SUPPORTS_FROGSPAWN)`
        // (`:31`), the other half of the disjunction.
        (
            pos(0, 63, -1),
            BrainBlockFacts {
                supports_frogspawn: true,
                ..air()
            },
        ),
        (pos(0, 64, -1), air()),
    ];

    brain.tick(&mut context, &mut world);
    assert_eq!(
        world.effects[0],
        BrainWorldEffect::PlaceBlock {
            pos: pos(0, 64, -1),
            block_state_id: 4242,
        }
    );
}

#[test]
fn try_lay_spawn_refuses_a_body_in_water_or_off_the_ground() {
    for (in_water, on_ground) in [(true, false), (false, false), (true, true)] {
        let (mut brain, mut context) = pregnant_brain(try_lay_spawn_on_fluid_near_land(4242));
        let mut world = test_world();
        world.body.in_water = in_water;
        world.body.on_ground = on_ground;

        brain.tick(&mut context, &mut world);
        // `!body.isInWater() && body.onGround()` (`:23`) refuses before any
        // read of the level.
        assert!(
            world.block_facts_calls.borrow().is_empty(),
            "in_water {in_water} on_ground {on_ground}"
        );
        assert!(world.effects.is_empty());
        assert!(brain.has_memory_value(MemoryModuleType::IsPregnant));
    }
}

#[test]
fn try_lay_spawn_keeps_the_pregnancy_when_no_side_supports_a_spawn() {
    let (mut brain, mut context) = pregnant_brain(try_lay_spawn_on_fluid_near_land(4242));
    let mut world = test_world();
    world.body.block_position = pos(0, 64, 0);
    world.body.on_ground = true;
    // Ground the body stands on, with nothing beside it.
    world.block_facts = staged(
        vec![(pos(0, 63, 0), solid())],
        pos(-1, 62, -1),
        pos(1, 65, 1),
        air(),
    );

    brain.tick(&mut context, &mut world);

    // The fall-through arm still answers true (`:45`): the four sides were
    // read and none of them qualified.
    assert!(world.block_facts_calls.borrow().len() > 1);
    assert!(world.effects.is_empty());
    assert!(brain.has_memory_value(MemoryModuleType::IsPregnant));
}

#[test]
fn the_frog_lands_the_spawn_block_at_lay_spawns_priority_three() {
    let brain: Brain =
        crate::entity::ai::mob::frog::frog_brain(&crate::entity::ai::BrainPacked::empty());
    assert_eq!(
        brain.behavior_priorities(crate::entity::ai::Activity::LAY_SPAWN),
        vec![
            (0, "SetEntityLookTargetSometimes"),
            (1, "StartAttacking"),
            (2, "TryFindLandNearWater"),
            (3, "TryLaySpawnOnFluidNearLand"),
            (4, "RunOne"),
        ]
    );
    assert_eq!(
        brain
            .behavior_priorities(crate::entity::ai::Activity::IDLE)
            .into_iter()
            .find(|(_, name)| *name == "TryFindLand"),
        Some((3, "TryFindLand"))
    );
}
