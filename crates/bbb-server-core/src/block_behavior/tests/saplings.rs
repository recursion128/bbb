use super::native_world::NativeWorld;
use super::*;
use bbb_server_types::save::Java26BuiltinDimension;

fn sapling_state(name: &str, stage: i32) -> i32 {
    state_of(name, &[("stage", &stage.to_string())])
}

fn mangrove_state(stage: i32, age: i32, hanging: bool) -> i32 {
    state_of(
        "minecraft:mangrove_propagule",
        &[
            ("age", &age.to_string()),
            ("hanging", if hanging { "true" } else { "false" }),
            ("stage", &stage.to_string()),
            ("waterlogged", "false"),
        ],
    )
}

#[test]
fn saplings_age_in_the_light_and_stay_young_in_the_dark() {
    let pos = at(0, 71, 0);
    let stage_zero = sapling_state("minecraft:oak_sapling", 0);
    let mut dark = NativeWorld::new();
    dark.set(below(pos), dirt());
    dark.set(pos, stage_zero);
    dark.illuminate(above(pos), 8);
    for seed in 0..30 {
        dark.random_tick(pos, seed);
    }
    assert_eq!(dark.state(pos), stage_zero);

    let mut bright = NativeWorld::new();
    bright.set(below(pos), dirt());
    bright.set(pos, stage_zero);
    for seed in 0..30 {
        bright.random_tick(pos, seed);
        if bright.state(pos) != stage_zero {
            break;
        }
    }
    assert_eq!(bright.state(pos), sapling_state("minecraft:oak_sapling", 1));
}

#[test]
fn a_grown_sapling_in_the_light_becomes_its_tree_and_saves() {
    let pos = at(0, 71, 0);
    let mut world = NativeWorld::new();
    world.set(below(pos), dirt());
    world.set(pos, sapling_state("minecraft:birch_sapling", 1));
    world.random_tick(pos, 7);
    assert_eq!(name(world.state(pos)), "minecraft:birch_log");
    assert!(leaves_near(&mut world, pos, "minecraft:birch_leaves"));
    assert_eq!(world.saved_block_name(pos), "minecraft:birch_log");
}

#[test]
fn each_sapling_grows_its_own_species() {
    for (sapling, log, leaves) in [
        ("oak_sapling", "oak_log", "oak_leaves"),
        ("spruce_sapling", "spruce_log", "spruce_leaves"),
        ("jungle_sapling", "jungle_log", "jungle_leaves"),
        ("acacia_sapling", "acacia_log", "acacia_leaves"),
        ("cherry_sapling", "cherry_log", "cherry_leaves"),
    ] {
        let pos = at(0, 71, 0);
        let mut world = NativeWorld::new();
        world.set(below(pos), dirt());
        world.set(pos, sapling_state(&format!("minecraft:{sapling}"), 1));
        advance(&mut world, pos);
        assert_eq!(
            name(world.state(pos)),
            format!("minecraft:{log}"),
            "{sapling}"
        );
        assert!(
            leaves_near(&mut world, pos, &format!("minecraft:{leaves}")),
            "{sapling}"
        );
    }
}

#[test]
fn a_mangrove_propagule_grows_a_mangrove() {
    let pos = at(0, 71, 0);
    let mut world = NativeWorld::new();
    // Mangrove roots reach down until they meet ground, so the floor is wide.
    for x in -4..=4 {
        for z in -4..=4 {
            world.set(offset(pos, x, -1, z), dirt());
        }
    }
    world.set(pos, mangrove_state(1, 4, false));
    advance(&mut world, pos);
    assert!((0..8).any(|dy| name(world.state(offset(pos, 0, dy, 0))) == "minecraft:mangrove_log"));
    assert!(leaves_near(&mut world, pos, "minecraft:mangrove_leaves"));
}

#[test]
fn four_saplings_in_a_square_grow_one_big_tree() {
    for (sapling, log) in [
        ("minecraft:spruce_sapling", "minecraft:spruce_log"),
        ("minecraft:dark_oak_sapling", "minecraft:dark_oak_log"),
    ] {
        let pos = at(0, 71, 0);
        let mut world = NativeWorld::new();
        let state = sapling_state(sapling, 1);
        for (x, z) in [(0, 0), (-1, 0), (0, -1), (-1, -1)] {
            world.set(offset(below(pos), x, 0, z), dirt());
            world.set(offset(pos, x, 0, z), state);
        }
        advance(&mut world, pos);
        for (x, z) in [(0, 0), (-1, 0), (0, -1), (-1, -1)] {
            assert_eq!(name(world.state(offset(pos, x, 1, z))), log, "{sapling}");
        }
    }
}

#[test]
fn a_lone_dark_oak_sapling_and_a_crowded_sapling_stay_saplings() {
    let pos = at(0, 71, 0);
    let mut lone = NativeWorld::new();
    let dark_oak = sapling_state("minecraft:dark_oak_sapling", 1);
    lone.set(below(pos), dirt());
    lone.set(pos, dark_oak);
    advance(&mut lone, pos);
    assert_eq!(lone.state(pos), dark_oak);

    let mut crowded = NativeWorld::new();
    let oak = sapling_state("minecraft:oak_sapling", 1);
    crowded.set(below(pos), dirt());
    crowded.set(pos, oak);
    crowded.set(offset(pos, 0, 2, 0), stone());
    for _ in 0..20 {
        advance(&mut crowded, pos);
    }
    assert_eq!(crowded.state(pos), oak);
    assert_eq!(crowded.state(above(pos)), air());
}

#[test]
fn a_sapling_grown_beside_flowers_can_grow_a_bee_nest_with_bees() {
    let mut world = NativeWorld::new();
    let poppy = state_of("minecraft:poppy", &[]);
    let sapling = sapling_state("minecraft:oak_sapling", 1);
    let mut bees = None;
    // `oak_bees_005` places a nest 5% of the time.
    'search: for seed in 0..400 {
        let pos = at((seed % 20) * 16, 71, (seed / 20) * 16);
        world.set(below(pos), dirt());
        world.set(offset(below(pos), 2, 0, 0), dirt());
        world.set(offset(pos, 2, 0, 0), poppy);
        world.set(pos, sapling);
        world.with_level(|level| {
            level.level_random().reseed(seed.into());
            crate::block_behavior::sapling::advance_tree(level, pos, sapling);
        });
        for x in -2..=2 {
            for y in 1..=10 {
                for z in -2..=2 {
                    let nest = offset(pos, x, y, z);
                    if name(world.state(nest)) == "minecraft:bee_nest" {
                        bees = world
                            .world
                            .get_block_entity(Java26BuiltinDimension::Overworld, nest)
                            .and_then(|entity| entity.beehive())
                            .map(|hive| hive.bees().len());
                        break 'search;
                    }
                }
            }
        }
    }
    let bees = bees.expect("some oak grew a bee nest");
    assert!((2..=3).contains(&bees), "{bees} bees");
}

fn advance(world: &mut NativeWorld, pos: BlockPos) {
    let state = world.state(pos);
    world.with_level(|level| crate::block_behavior::sapling::advance_tree(level, pos, state));
}

fn name(state: i32) -> &'static str {
    &bbb_protocol::block_state_types::vanilla_block_state_identity(state)
        .unwrap()
        .name
}

fn leaves_near(world: &mut NativeWorld, pos: BlockPos, leaves: &str) -> bool {
    (-3..=3).any(|x| {
        (1..=12).any(|y| (-3..=3).any(|z| name(world.state(offset(pos, x, y, z))) == leaves))
    })
}

#[test]
fn mangrove_ground_growth_ignores_light_and_hanging_propagules_age() {
    let pos = at(0, 71, 0);
    let initial = mangrove_state(0, 4, false);
    let mut ground = NativeWorld::new();
    ground.set(below(pos), dirt());
    ground.set(pos, initial);
    ground.illuminate(above(pos), 0);
    for seed in 0..30 {
        ground.random_tick(pos, seed);
        if ground.state(pos) != initial {
            break;
        }
    }
    assert_eq!(ground.state(pos), mangrove_state(1, 4, false));

    let mut hanging = NativeWorld::new();
    hanging.set(above(pos), default_state("minecraft:mangrove_leaves"));
    hanging.set(pos, mangrove_state(0, 3, true));
    hanging.random_tick(pos, 91);
    assert_eq!(hanging.state(pos), mangrove_state(0, 4, true));
}
