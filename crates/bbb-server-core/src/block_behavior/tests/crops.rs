use super::native_world::NativeWorld;
use super::*;

fn one_farmland(crop: i32, moisture: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    for x in -2..=2 {
        for z in -2..=2 {
            world.set(at(x, 70, z), dirt());
        }
    }
    world.set(at(0, 70, 0), farmland_state(moisture));
    world.set(at(0, 71, 0), crop);
    world
}

fn speed_at(world: &mut NativeWorld, block_type_id: i32, pos: BlockPos) -> f32 {
    world
        .with_level(|level| crop::growth_speed(level, block_type_id, pos))
        .expect("builtin block tags are available")
}

fn wheat_block_type() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(wheat(0))
        .expect("wheat has a registry id")
}

#[test]
fn growth_speed_is_one_plus_the_score_of_the_nine_blocks_underneath() {
    let mut world = one_farmland(wheat(0), 0);
    assert_eq!(speed_at(&mut world, wheat_block_type(), at(0, 71, 0)), 2.0);

    let mut world = one_farmland(wheat(0), 1);
    assert_eq!(speed_at(&mut world, wheat_block_type(), at(0, 71, 0)), 4.0);

    let mut world = one_farmland(wheat(0), farmland::MAX_MOISTURE);
    assert_eq!(speed_at(&mut world, wheat_block_type(), at(0, 71, 0)), 4.0);
}

#[test]
fn the_eight_outer_blocks_of_the_footprint_count_a_quarter_each() {
    let mut world = one_farmland(wheat(0), 0);
    for x in -1..=1 {
        for z in -1..=1 {
            world.set(at(x, 70, z), farmland_state(0));
        }
    }
    assert_eq!(speed_at(&mut world, wheat_block_type(), at(0, 71, 0)), 4.0);

    let mut world = one_farmland(wheat(0), 0);
    for x in -1..=1 {
        for z in -1..=1 {
            world.set(at(x, 70, z), farmland_state(farmland::MAX_MOISTURE));
        }
    }
    assert_eq!(speed_at(&mut world, wheat_block_type(), at(0, 71, 0)), 10.0);
}

#[test]
fn a_crop_flanked_on_both_horizontal_axes_is_halved() {
    let mut world = one_farmland(wheat(0), 0);
    world.set(at(-1, 71, 0), wheat(0));
    world.set(at(0, 71, -1), wheat(0));
    assert_eq!(speed_at(&mut world, wheat_block_type(), at(0, 71, 0)), 1.0);
}

#[test]
fn a_crop_with_only_a_diagonal_neighbour_is_halved_by_the_else_branch() {
    let mut world = one_farmland(wheat(0), 0);
    world.set(at(-1, 71, -1), wheat(0));
    assert_eq!(speed_at(&mut world, wheat_block_type(), at(0, 71, 0)), 1.0);
}

#[test]
fn a_crop_grows_by_one_age_when_the_draw_comes_up_zero() {
    let mut world = one_farmland(wheat(3), 0);

    world.random_tick(at(0, 71, 0), 9);

    assert_eq!(world.state(at(0, 71, 0)), wheat(4));
}

#[test]
fn a_crop_below_light_nine_does_not_grow() {
    let mut world = one_farmland(wheat(0), 0);
    world.illuminate(at(0, 71, 0), 8);

    world.random_tick(at(0, 71, 0), 9);

    assert_eq!(world.state(at(0, 71, 0)), wheat(0));
}

#[test]
fn a_crop_at_exactly_light_nine_grows() {
    let mut world = one_farmland(wheat(0), 0);
    world.illuminate(at(0, 71, 0), 9);

    world.random_tick(at(0, 71, 0), 9);

    assert_eq!(world.state(at(0, 71, 0)), wheat(1));
}

#[test]
fn a_fully_grown_crop_never_ages_further() {
    let mut world = one_farmland(wheat(7), farmland::MAX_MOISTURE);

    world.random_tick(at(0, 71, 0), 9);

    assert_eq!(world.state(at(0, 71, 0)), wheat(7));
    assert!(!crop::is_valid_bonemeal_target(wheat(7)));
    assert!(crop::is_valid_bonemeal_target(wheat(6)));
}

/// How many of 13 000 single random ticks age `crop` on dry farmland.
fn growths(crop: i32) -> usize {
    let pos = at(0, 71, 0);
    let mut world = one_farmland(crop, 0);
    world.with_level(|level| {
        (0..13_000)
            .filter(|_| {
                level.set_block(pos, crop, 2 | 16, crate::neighbor_updater::UPDATE_LIMIT);
                crate::block_behavior::random_tick_block(level, pos, crop);
                level.block_state(pos) != crop
            })
            .count()
    })
}

#[test]
fn beetroot_skips_the_inherited_body_one_random_tick_in_three() {
    // Speed 2 is a 1-in-13 roll: wheat ages about 1 000 times (sd ~30) and
    // beetroot two thirds of that (sd ~25).
    let wheat = growths(wheat(0));
    let beetroot = growths(beetroots(0));
    assert!((850..=1_150).contains(&wheat), "{wheat}");
    assert!((550..=790).contains(&beetroot), "{beetroot}");

    let mut world = one_farmland(beetroots(0), 0);
    world.random_tick(at(0, 71, 0), 5);
    assert_eq!(world.state(at(0, 71, 0)), beetroots(1));
}

#[test]
fn beetroot_stops_at_age_three_where_wheat_would_still_have_four_to_go() {
    let mut world = one_farmland(beetroots(3), farmland::MAX_MOISTURE);
    world.random_tick(at(0, 71, 0), 5);
    assert_eq!(world.state(at(0, 71, 0)), beetroots(3));
}

#[test]
fn a_torchflower_crop_at_its_last_age_becomes_a_torchflower() {
    let mut world = one_farmland(torchflower_crop(1), 0);
    world.random_tick(at(0, 71, 0), 5);

    assert_eq!(
        world.state(at(0, 71, 0)),
        state_of("minecraft:torchflower", &[])
    );
}

#[test]
fn a_crop_pops_off_when_the_farmland_under_it_is_gone() {
    let mut world = one_farmland(wheat(4), 0);
    world.set(at(0, 70, 0), dirt());

    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:wheat_seeds" && *count > 0));
}

#[test]
fn a_crop_pops_off_below_light_eight_but_survives_at_eight() {
    let mut world = one_farmland(wheat(4), 0);
    world.illuminate(at(0, 71, 0), 8);
    world.shape_update(at(0, 71, 0), Direction::Down);
    assert_eq!(world.state(at(0, 71, 0)), wheat(4));

    let mut world = one_farmland(wheat(4), 0);
    world.illuminate(at(0, 71, 0), 7);
    world.shape_update(at(0, 71, 0), Direction::Down);
    assert_eq!(world.state(at(0, 71, 0)), air());
}
