//! Copper-chest oxidation and shape-update coverage.

use super::*;

const LEFT_HALF: BlockPos = BlockPos { x: 8, y: 70, z: 8 };
const RIGHT_HALF: BlockPos = BlockPos { x: 9, y: 70, z: 8 };

const SEED_OXIDIZES: i64 = 4610;

fn chest_state(name: &str, facing: &str, chest_type: &str, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[
            ("facing", facing),
            ("type", chest_type),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn copper_chest(chest_type: &str) -> i32 {
    chest_state("minecraft:copper_chest", "north", chest_type, false)
}

fn exposed_copper_chest(chest_type: &str) -> i32 {
    chest_state("minecraft:exposed_copper_chest", "north", chest_type, false)
}

fn waxed_copper_chest(chest_type: &str) -> i32 {
    chest_state("minecraft:waxed_copper_chest", "north", chest_type, false)
}

fn plain_chest(chest_type: &str) -> i32 {
    chest_state("minecraft:chest", "north", chest_type, false)
}

fn trapped_chest(chest_type: &str) -> i32 {
    chest_state("minecraft:trapped_chest", "north", chest_type, false)
}

fn chest_with_block_entity(pos: BlockPos, state: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    world.set(pos, state);
    world
}

#[test]
fn a_copper_chest_nobody_has_open_oxidizes() {
    let mut world = chest_with_block_entity(LEFT_HALF, copper_chest("single"));
    let seed = SEED_OXIDIZES;

    world.random_tick(LEFT_HALF, seed);

    assert_eq!(world.state(LEFT_HALF), exposed_copper_chest("single"));
}

#[test]
fn right_half_open_or_waxed_chests_do_not_oxidize() {
    let right_half = {
        let mut world = chest_with_block_entity(LEFT_HALF, copper_chest("right"));
        world.set(LEFT_HALF, copper_chest("right"));
        world
    };
    let someone_looking_inside = {
        let mut world = chest_with_block_entity(LEFT_HALF, copper_chest("single"));
        world
            .world
            .level_block_entity_for_test(
                bbb_server_types::save::Java26BuiltinDimension::Overworld,
                LEFT_HALF,
            )
            .unwrap()
            .chest_mut()
            .unwrap()
            .start_open();
        world
    };
    let waxed = chest_with_block_entity(LEFT_HALF, waxed_copper_chest("single"));

    for (label, mut world, expected_state) in [
        ("right half", right_half, copper_chest("right")),
        (
            "someone looking inside",
            someone_looking_inside,
            copper_chest("single"),
        ),
        ("waxed", waxed, waxed_copper_chest("single")),
    ] {
        let seed = SEED_OXIDIZES;
        world.random_tick(LEFT_HALF, seed);
        assert_eq!(world.state(LEFT_HALF), expected_state, "{label}");
    }
}

#[test]
fn the_right_half_of_a_double_chest_follows_the_half_that_oxidized() {
    let mut world = chest_with_block_entity(LEFT_HALF, copper_chest("left"));
    world.set(RIGHT_HALF, copper_chest("right"));
    let seed = SEED_OXIDIZES;

    world.random_tick(LEFT_HALF, seed);

    assert_eq!(world.state(LEFT_HALF), exposed_copper_chest("left"));
    assert_eq!(world.state(RIGHT_HALF), exposed_copper_chest("right"));
}

#[test]
fn a_single_chest_pairs_with_a_half_whose_partner_slot_points_back_at_it() {
    let mut world = NativeWorld::new();
    world.set(LEFT_HALF, plain_chest("single"));
    world.set(RIGHT_HALF, plain_chest("right"));

    world.shape_update(LEFT_HALF, Direction::East);

    assert_eq!(world.state(LEFT_HALF), plain_chest("left"));
}

#[test]
fn a_paired_chest_whose_partner_cell_changed_falls_back_to_single() {
    let mut world = NativeWorld::new();
    world.set(LEFT_HALF, plain_chest("left"));

    world.shape_update(LEFT_HALF, Direction::East);

    assert_eq!(world.state(LEFT_HALF), plain_chest("single"));
}

#[test]
fn an_update_from_a_face_that_is_not_the_partner_leaves_the_pair_alone() {
    let mut world = NativeWorld::new();
    world.set(LEFT_HALF, plain_chest("left"));
    world.set(RIGHT_HALF, plain_chest("right"));

    world.shape_update(LEFT_HALF, Direction::Up);
    world.shape_update(LEFT_HALF, Direction::West);

    assert_eq!(world.state(LEFT_HALF), plain_chest("left"));
    assert_eq!(world.state(RIGHT_HALF), plain_chest("right"));
}

#[test]
fn only_a_copper_chest_pairs_with_a_block_that_is_not_its_own() {
    let mut world = NativeWorld::new();
    world.set(LEFT_HALF, copper_chest("single"));
    world.set(RIGHT_HALF, exposed_copper_chest("right"));
    world.shape_update(LEFT_HALF, Direction::East);
    assert_eq!(world.state(LEFT_HALF), exposed_copper_chest("left"));

    let mut world = NativeWorld::new();
    world.set(LEFT_HALF, plain_chest("single"));
    world.set(RIGHT_HALF, trapped_chest("right"));
    world.shape_update(LEFT_HALF, Direction::East);
    assert_eq!(world.state(LEFT_HALF), plain_chest("single"));
}

#[test]
fn a_waterlogged_chest_re_arms_its_water_on_every_shape_update() {
    let mut world = NativeWorld::new();
    world.set(
        LEFT_HALF,
        chest_state("minecraft:chest", "north", "single", true),
    );

    world.shape_update(LEFT_HALF, Direction::North);

    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && (tick.x, tick.y, tick.z) == (LEFT_HALF.x, LEFT_HALF.y, LEFT_HALF.z)
            && tick.delay == 5));
}
