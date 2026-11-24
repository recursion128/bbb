//! `BuddingAmethystBlock` and `AmethystClusterBlock`
//! (`BuddingAmethystBlock.java`, `AmethystClusterBlock.java`): growth and support.

use super::*;

use crate::block_behavior::can_survive_at;

/// The budding block. Deliberately not the origin so a read at the wrong
/// position cannot pass by coincidence.
const GEODE: BlockPos = BlockPos { x: 4, y: 70, z: -9 };

fn budding_amethyst() -> i32 {
    default_state("minecraft:budding_amethyst")
}

fn cluster(name: &str, facing: &str, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[
            ("facing", facing),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

/// One budding amethyst with `above` over it and stone on its other faces, so
/// only the upward face can grow.
fn geode_with(above_state: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    world.set(GEODE, budding_amethyst());
    for direction in DIRECTION_VALUES {
        if direction != Direction::Up {
            world.set(relative(GEODE, direction), stone());
        }
    }
    world.set(above(GEODE), above_state);
    world
}

fn grow_upwards(world: &mut NativeWorld) {
    world.random_tick(GEODE, 55);
}

#[test]
fn an_empty_face_grows_a_small_bud_turned_towards_it() {
    let mut world = geode_with(air());
    grow_upwards(&mut world);

    assert_eq!(
        world.state(above(GEODE)),
        cluster("minecraft:small_amethyst_bud", "up", false)
    );
}

#[test]
fn each_stage_advances_the_bud_already_facing_that_way() {
    for (from, to) in [
        (
            "minecraft:small_amethyst_bud",
            "minecraft:medium_amethyst_bud",
        ),
        (
            "minecraft:medium_amethyst_bud",
            "minecraft:large_amethyst_bud",
        ),
        ("minecraft:large_amethyst_bud", "minecraft:amethyst_cluster"),
    ] {
        let mut world = geode_with(cluster(from, "up", false));
        grow_upwards(&mut world);
        assert_eq!(
            world.state(above(GEODE)),
            cluster(to, "up", false),
            "{from}"
        );
    }
}

#[test]
fn a_bud_turned_the_other_way_is_not_advanced() {
    // Every stage test in `randomTick` compares `FACING` against the direction
    // the draw picked (`BuddingAmethystBlock.java:35`), so a sideways bud in
    // the cell above is neither a growable cell nor the next stage.
    let sideways = cluster("minecraft:small_amethyst_bud", "north", false);
    let mut world = geode_with(sideways);
    grow_upwards(&mut world);
    assert_eq!(world.state(above(GEODE)), sideways);
}

#[test]
fn a_full_water_source_grows_a_waterlogged_bud_and_flowing_water_grows_nothing() {
    let mut world = geode_with(water(0));
    grow_upwards(&mut world);
    assert_eq!(
        world.state(above(GEODE)),
        cluster("minecraft:small_amethyst_bud", "up", true)
    );

    // `canClusterGrowAtState` demands `getFluidState().isFull()`
    // (`BuddingAmethystBlock.java:53`), which a flowing cell is not.
    let mut world = geode_with(water(1));
    grow_upwards(&mut world);
    assert_eq!(world.state(above(GEODE)), water(1));
}

#[test]
fn a_cluster_needs_a_full_sturdy_face_on_the_block_it_grew_from() {
    let up = cluster("minecraft:amethyst_cluster", "up", false);
    let read_below = |support: i32| {
        let mut world = NativeWorld::new();
        world.set(GEODE, support);
        world.set(above(GEODE), up);
        world.with_level(|level| can_survive_at(up, above(GEODE), level))
    };
    assert!(read_below(budding_amethyst()));
    assert!(read_below(stone()));
    assert!(!read_below(air()));
    assert!(!read_below(default_state("minecraft:oak_fence")));

    // The face is the one it points away from, so a sideways cluster reads a
    // sideways neighbour instead.
    let north = cluster("minecraft:amethyst_cluster", "north", false);
    let mut world = NativeWorld::new();
    world.set(relative(GEODE, Direction::South), stone());
    world.set(GEODE, north);
    assert!(world.with_level(|level| can_survive_at(north, GEODE, level)));
}

#[test]
fn only_an_update_from_the_face_it_grew_from_can_take_a_cluster_away() {
    let up = cluster("minecraft:amethyst_cluster", "up", false);
    let mut world = NativeWorld::new();
    world.set(GEODE, stone());
    world.set(above(GEODE), up);

    world.shape_update(above(GEODE), Direction::Up);
    world.shape_update(above(GEODE), Direction::North);
    assert_eq!(world.state(above(GEODE)), up);

    world.set(GEODE, air());
    world.shape_update(above(GEODE), Direction::North);
    assert_eq!(world.state(above(GEODE)), up);
    world.shape_update(above(GEODE), Direction::Down);
    assert_eq!(world.state(above(GEODE)), air());
}
