//! `Entity.applyMovementEmissionAndPlaySound`'s game-event half
//! (`Entity.java:816-822`, `:936-938`) on the real tick path: a walking mob's
//! `GameEvent.STEP`.

use super::*;

use crate::level_event::VanillaGameEvent;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const FLOOR_Y: i32 = 64;
const MOB_POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 65.0,
    z: 4.5,
};
/// Half a block of travel per tick with a descent the floor clips, which is
/// what makes `verticalCollisionBelow` -- and so `onGround()` -- true
/// (`Entity.java:757-760`).
const WALK_STEP: Vec3d = Vec3d {
    x: 0.5,
    y: -0.1,
    z: 0.0,
};

/// A stone floor with clear air above it, so the mob has ground to step on
/// whatever the generator left there.
fn lay_floor(world: &mut ServerWorld) {
    let stone = block_state_id("minecraft:stone");
    let air = block_state_id("minecraft:air");
    for x in -2..=8 {
        for z in 2..=8 {
            for (y, state) in [(FLOOR_Y, stone), (65, air), (66, air), (67, air)] {
                let _ =
                    world.set_block(DIMENSION, BlockPos { x, y, z }, state, SET_BLOCK_UPDATE_ALL);
            }
        }
    }
}

/// `GameEvent.STEP` raised by a mob the world moved on its own tick: the event
/// names the mob and the state it walked.
#[test]
fn a_walking_mob_raises_step_with_the_walked_state() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    lay_floor(&mut world);

    let (mob, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        MOB_POSITION,
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).expect("zombie is living"),
        ),
    );

    // Half a block of walk per tick, so `moveDist` crosses `nextStep`
    // (`Entity.java:815`) inside the loop whatever the mob's own brain asks
    // for; the travel that consumes it is the level's, not the test's.
    let mut step = None;
    for _ in 0..8 {
        world
            .stored_entity_mut_for_test(DIMENSION, mob)
            .expect("the mob is live")
            .set_delta_movement(WALK_STEP);
        world.tick(receiver.drain());
        step = world.game_events(DIMENSION).0.into_iter().find(|row| {
            row.game_event == VanillaGameEvent::STEP && row.source_entity_id == Some(mob)
        });
        if step.is_some() {
            break;
        }
    }

    let step = step.unwrap_or_else(|| {
        panic!(
            "the mob never raised a STEP: {:?}",
            world.game_events(DIMENSION).0
        )
    });
    assert_eq!(
        step.affected_state,
        Some(block_state_id("minecraft:stone")),
        "`GameEvent.Context.of(this, blockState)` names the walked state"
    );
    let moved = world
        .stored_entity_for_test(DIMENSION, mob)
        .expect("the mob is live");
    assert!(
        moved.position().x > MOB_POSITION.x,
        "the event came from a body the level actually moved"
    );
}
