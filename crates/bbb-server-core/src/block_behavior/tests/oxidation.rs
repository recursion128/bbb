use super::native_world::NativeWorld;
use super::*;

use crate::block_behavior::weathering;

const COPPER: BlockPos = BlockPos { x: 8, y: 70, z: 8 };

fn copper_block() -> i32 {
    default_state("minecraft:copper_block")
}

fn exposed_copper() -> i32 {
    default_state("minecraft:exposed_copper")
}

fn weathered_copper() -> i32 {
    default_state("minecraft:weathered_copper")
}

fn oxidized_copper() -> i32 {
    default_state("minecraft:oxidized_copper")
}

fn copper_with(origin_state: i32, neighbours: &[((i32, i32, i32), i32)]) -> NativeWorld {
    let mut world = NativeWorld::new();
    world.set(COPPER, origin_state);
    for ((dx, dy, dz), state) in neighbours {
        world.set(at(COPPER.x + dx, COPPER.y + dy, COPPER.z + dz), *state);
    }
    world
}

/// How many of 40 000 single random ticks age the block at `COPPER`, which is
/// put back to `from` before each.
fn oxidations(world: &mut NativeWorld, from: i32) -> usize {
    world.with_level(|level| {
        (0..40_000)
            .filter(|_| {
                level.set_block(COPPER, from, 2 | 16, crate::neighbor_updater::UPDATE_LIMIT);
                crate::block_behavior::random_tick_block(level, COPPER, from);
                level.block_state(COPPER) != from
            })
            .count()
    })
}

#[test]
fn the_chance_modifier_only_slows_the_first_step_of_a_chain() {
    // 0.0569 per tick is about 2 276 of 40 000 (sd ~46); the first step's
    // 0.75 is about 1 707 (sd ~40).
    let exposed = oxidations(&mut copper_with(exposed_copper(), &[]), exposed_copper());
    assert!((2_050..=2_500).contains(&exposed), "{exposed}");
    let unaffected = oxidations(&mut copper_with(copper_block(), &[]), copper_block());
    assert!((1_500..=1_920).contains(&unaffected), "{unaffected}");
}

#[test]
fn a_block_at_the_end_of_its_chain_never_moves() {
    let mut world = copper_with(oxidized_copper(), &[]);

    world.random_tick(COPPER, 4610);

    assert_eq!(world.state(COPPER), oxidized_copper());
}

#[test]
fn one_same_aged_neighbour_halves_the_chance_and_two_of_them_third_it() {
    // The fraction is squared: a quarter of 2 276 is about 569 (sd ~24) and a
    // ninth about 253 (sd ~16).
    let mut world = copper_with(exposed_copper(), &[((1, 0, 0), exposed_copper())]);
    let one = oxidations(&mut world, exposed_copper());
    assert!((450..=690).contains(&one), "{one}");

    let mut world = copper_with(
        exposed_copper(),
        &[
            ((1, 0, 0), exposed_copper()),
            ((-1, 0, 0), exposed_copper()),
        ],
    );
    let two = oxidations(&mut world, exposed_copper());
    assert!((175..=335).contains(&two), "{two}");
}

#[test]
fn an_older_neighbour_counts_on_the_other_side_of_the_fraction() {
    let mut world = copper_with(
        exposed_copper(),
        &[
            ((1, 0, 0), weathered_copper()),
            ((-1, 0, 0), weathered_copper()),
        ],
    );

    world.random_tick(COPPER, 4618);

    assert_eq!(world.state(COPPER), weathered_copper());
}

#[test]
fn a_neighbour_from_outside_the_chain_is_not_counted_at_all() {
    let waxed = default_state("minecraft:waxed_exposed_copper");
    let mut world = copper_with(
        exposed_copper(),
        &[
            ((1, 0, 0), waxed),
            ((-1, 0, 0), waxed),
            ((0, 1, 0), stone()),
        ],
    );

    world.random_tick(COPPER, 4618);

    assert_eq!(world.state(COPPER), weathered_copper());
}

#[test]
fn any_chain_of_the_same_age_counts_because_there_is_only_one_chain() {
    let bulb = default_state("minecraft:exposed_copper_bulb");
    let mut world = copper_with(exposed_copper(), &[((1, 0, 0), bulb), ((-1, 0, 0), bulb)]);
    // Two same-aged bulbs are a ninth, as two same-aged blocks are.
    let aged = oxidations(&mut world, exposed_copper());
    assert!((175..=335).contains(&aged), "{aged}");
}

#[test]
fn a_younger_neighbour_stops_the_oxidation() {
    let mut world = copper_with(weathered_copper(), &[((1, 0, 0), copper_block())]);

    world.random_tick(COPPER, 4610);

    assert_eq!(world.state(COPPER), weathered_copper());
}

#[test]
fn the_scan_sees_four_blocks_of_manhattan_distance_and_no_more() {
    for (offset, blocked) in [
        ((4, 0, 0), true),
        ((5, 0, 0), false),
        ((2, 1, 1), true),
        ((2, 2, 1), false),
        ((0, -4, 0), true),
        ((0, 0, -5), false),
    ] {
        let mut world = copper_with(weathered_copper(), &[(offset, copper_block())]);

        world.random_tick(COPPER, 4610);

        let expected = if blocked {
            weathered_copper()
        } else {
            oxidized_copper()
        };
        assert_eq!(world.state(COPPER), expected, "offset {offset:?}");
    }
}

#[test]
fn oxidizing_keeps_every_property_the_two_blocks_share() {
    let cut = state_of(
        "minecraft:cut_copper_slab",
        &[("type", "top"), ("waterlogged", "true")],
    );
    assert_eq!(
        weathering::next_state(cut),
        Some(state_of(
            "minecraft:exposed_cut_copper_slab",
            &[("type", "top"), ("waterlogged", "true")],
        ))
    );
    let stairs = state_of(
        "minecraft:weathered_cut_copper_stairs",
        &[
            ("facing", "west"),
            ("half", "top"),
            ("shape", "inner_left"),
            ("waterlogged", "false"),
        ],
    );
    assert_eq!(
        weathering::next_state(stairs),
        Some(state_of(
            "minecraft:oxidized_cut_copper_stairs",
            &[
                ("facing", "west"),
                ("half", "top"),
                ("shape", "inner_left"),
                ("waterlogged", "false"),
            ],
        ))
    );
}

fn copper_door(name: &str, half: &str) -> i32 {
    state_of(
        name,
        &[
            ("facing", "east"),
            ("half", half),
            ("hinge", "right"),
            ("open", "true"),
            ("powered", "false"),
        ],
    )
}

#[test]
fn only_the_lower_half_of_a_copper_door_oxidizes_and_it_carries_the_upper_one() {
    let mut world = NativeWorld::new();
    world.set(COPPER, copper_door("minecraft:copper_door", "lower"));
    world.set(
        at(COPPER.x, COPPER.y + 1, COPPER.z),
        copper_door("minecraft:copper_door", "upper"),
    );
    world.set(at(COPPER.x, COPPER.y - 1, COPPER.z), stone());

    world.random_tick(at(COPPER.x, COPPER.y + 1, COPPER.z), 4610);
    assert_eq!(
        world.state(at(COPPER.x, COPPER.y + 1, COPPER.z)),
        copper_door("minecraft:copper_door", "upper")
    );

    world.random_tick(COPPER, 4610);

    assert_eq!(
        world.state(COPPER),
        copper_door("minecraft:exposed_copper_door", "lower")
    );
    assert_eq!(
        world.state(at(COPPER.x, COPPER.y + 1, COPPER.z)),
        copper_door("minecraft:exposed_copper_door", "upper")
    );
}
