//! `LeavesBlock` (`LeavesBlock.java`).

use super::*;

#[test]
fn a_leaf_beside_scaffolding_reaches_the_decay_distance_and_disappears() {
    let mut world = NativeWorld::ticking(at(0, 70, 0));
    world.set(at(0, 69, 0), stone());
    world.set(at(0, 70, 0), scaffolding(0));
    world.set(at(1, 70, 0), oak_leaves(1, false, false));

    world.shape_update(at(1, 70, 0), Direction::West);
    world.run_tick();
    assert_eq!(world.state(at(1, 70, 0)), oak_leaves(7, false, false));

    let seed = 0;
    world.random_tick(at(1, 70, 0), seed);
    assert_eq!(world.state(at(1, 70, 0)), air());
}

#[test]
fn update_distance_takes_the_nearest_neighbour_plus_one() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), oak_log());
    world.set(at(1, 70, 0), oak_leaves(7, false, false));
    world.set(at(2, 70, 0), oak_leaves(7, false, false));
    world.set(at(3, 70, 0), oak_leaves(7, false, false));
    world.set(at(2, 70, 1), oak_leaves(3, false, false));

    // Next to the log: 0 + 1.
    assert_eq!(
        bbb_server_types::block_queries::distance(world.with_level(|level| {
            leaves::update_distance(level, oak_leaves(7, false, false), at(1, 70, 0))
        })),
        Some(1)
    );
    // Next to a leaf that already claims 3: 3 + 1, because the two leaves at
    // distance 7 would only offer 8 and the cap is 7 anyway.
    assert_eq!(
        bbb_server_types::block_queries::distance(world.with_level(|level| {
            leaves::update_distance(level, oak_leaves(7, false, false), at(2, 70, 0))
        })),
        Some(4)
    );
    // Surrounded by nothing helpful: the initial DECAY_DISTANCE stands.
    assert_eq!(
        bbb_server_types::block_queries::distance(world.with_level(|level| {
            leaves::update_distance(level, oak_leaves(1, false, false), at(5, 70, 5))
        })),
        Some(leaves::DECAY_DISTANCE)
    );
}

#[test]
fn update_distance_keeps_the_persistent_and_waterlogged_flags() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), oak_log());
    let updated = world.with_level(|level| {
        leaves::update_distance(level, oak_leaves(7, true, true), at(1, 70, 0))
    });
    assert_eq!(updated, oak_leaves(1, true, true));
}

#[test]
fn a_leaf_whose_distance_disagrees_with_an_adjacent_log_still_arms_a_tick() {
    let mut world = NativeWorld::ticking(at(0, 70, 0));
    world.set(at(0, 70, 0), oak_log());
    world.set(at(1, 70, 0), oak_leaves(5, false, false));

    world.shape_update(at(1, 70, 0), Direction::West);
    world.run_tick();
    assert_eq!(world.state(at(1, 70, 0)), oak_leaves(1, false, false));
}

#[test]
fn breaking_the_log_walks_every_leaf_out_to_the_decay_distance() {
    // A six-block arm of leaves hanging off one log, each already carrying the
    // distance the log gives it.
    let mut world = NativeWorld::ticking(at(0, 70, 0));
    world.set(at(0, 70, 0), oak_log());
    for x in 1..=6 {
        world.set(at(x, 70, 0), oak_leaves(x, false, false));
    }

    world.set(at(0, 70, 0), air());
    world.shape_update(at(1, 70, 0), Direction::West);

    for _ in 0..6 {
        world.run_tick();
    }
    let seen: Vec<i32> = (1..=6)
        .map(|x| bbb_server_types::block_queries::distance(world.state(at(x, 70, 0))).unwrap_or(0))
        .collect();
    assert_eq!(seen, vec![leaves::DECAY_DISTANCE; 6]);
}

#[test]
fn a_leaf_at_the_decay_distance_disappears_on_a_random_tick() {
    let mut world = NativeWorld::new();
    world.set(
        at(0, 70, 0),
        oak_leaves(leaves::DECAY_DISTANCE, false, false),
    );
    let seed = 0;

    world.random_tick(at(0, 70, 0), seed);

    assert_eq!(world.state(at(0, 70, 0)), air());
}

#[test]
fn a_persistent_leaf_never_decays_however_far_it_is_from_a_log() {
    let mut world = NativeWorld::new();
    let persistent = oak_leaves(leaves::DECAY_DISTANCE, true, false);
    world.set(at(0, 70, 0), persistent);
    let seed = 0;

    world.random_tick(at(0, 70, 0), seed);

    assert_eq!(world.state(at(0, 70, 0)), persistent);
    assert!(world.items().is_empty());
}

#[test]
fn a_waterlogged_leaf_that_decays_leaves_its_water_behind() {
    let mut world = NativeWorld::new();
    world.set(
        at(0, 70, 0),
        oak_leaves(leaves::DECAY_DISTANCE, false, true),
    );
    let seed = 0;

    world.random_tick(at(0, 70, 0), seed);

    assert_eq!(world.state(at(0, 70, 0)), water(0));
}
