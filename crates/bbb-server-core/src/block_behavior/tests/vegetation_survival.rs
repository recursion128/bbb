use super::native_world::NativeWorld;
use super::*;

use crate::block_behavior::vegetation::{can_survive, small_dripleaf_can_survive};

fn survives_on(name: &str, below_name: &str) -> Option<bool> {
    let below_state = default_state(below_name);
    can_survive(default_state(name), PLANT, vanilla_tags(), &mut |at| {
        if at == below(PLANT) {
            below_state
        } else {
            air()
        }
    })
}

const PLANT: BlockPos = BlockPos { x: 3, y: 70, z: -7 };

fn small_dripleaf(half: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:small_dripleaf",
        &[
            ("facing", "north"),
            ("half", half),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn update_small_dripleaf(
    world: &mut NativeWorld,
    state: i32,
    pos: BlockPos,
    direction: Direction,
    neighbour_state: i32,
) -> i32 {
    world.set(pos, state);
    world.set(relative(pos, direction), neighbour_state);
    world.shape_update(pos, direction);
    world.state(pos)
}

#[test]
fn small_dripleaf_dry_support_is_clay_or_moss() {
    let lower = small_dripleaf("lower", false);
    for support in ["minecraft:clay", "minecraft:moss_block"] {
        let support = default_state(support);
        assert_eq!(
            small_dripleaf_can_survive(lower, PLANT, vanilla_tags(), &mut |pos| {
                if pos == below(PLANT) {
                    support
                } else {
                    air()
                }
            }),
            Some(true)
        );
    }
    assert_eq!(
        small_dripleaf_can_survive(lower, PLANT, vanilla_tags(), &mut |pos| {
            if pos == below(PLANT) {
                dirt()
            } else {
                air()
            }
        }),
        Some(false)
    );
}

#[test]
fn small_dripleaf_on_dirt_requires_source_water_in_its_own_cell() {
    let lower = small_dripleaf("lower", false);
    for (own_state, expected) in [(water(0), true), (water(1), false), (air(), false)] {
        assert_eq!(
            small_dripleaf_can_survive(lower, PLANT, vanilla_tags(), &mut |pos| {
                if pos == below(PLANT) {
                    dirt()
                } else {
                    own_state
                }
            }),
            Some(expected)
        );
    }
}

#[test]
fn small_dripleaf_upper_half_requires_its_lower_half() {
    let upper = small_dripleaf("upper", false);
    let lower = small_dripleaf("lower", false);
    assert_eq!(
        small_dripleaf_can_survive(upper, PLANT, vanilla_tags(), &mut |_| lower),
        Some(true)
    );
    for support in [upper, dirt(), air()] {
        assert_eq!(
            small_dripleaf_can_survive(upper, PLANT, vanilla_tags(), &mut |_| support),
            Some(false)
        );
    }
}

#[test]
fn small_dripleaf_shape_updates_require_the_partner_and_recheck_support() {
    let lower = small_dripleaf("lower", false);
    let upper = small_dripleaf("upper", false);
    let mut world = NativeWorld::new();
    world.set(below(PLANT), default_state("minecraft:moss_block"));
    world.set(PLANT, lower);
    world.set(above(PLANT), upper);

    assert_eq!(
        update_small_dripleaf(&mut world, lower, PLANT, Direction::Up, upper),
        lower
    );
    assert_eq!(
        update_small_dripleaf(&mut world, upper, above(PLANT), Direction::Down, lower),
        upper
    );
    assert_eq!(
        update_small_dripleaf(&mut world, lower, PLANT, Direction::North, air()),
        lower
    );
    assert_eq!(
        update_small_dripleaf(&mut world, lower, PLANT, Direction::Up, air()),
        air()
    );
    assert_eq!(
        update_small_dripleaf(&mut world, upper, above(PLANT), Direction::Down, air()),
        air()
    );

    world.set(below(PLANT), dirt());
    assert_eq!(
        update_small_dripleaf(&mut world, lower, PLANT, Direction::North, air()),
        air()
    );
    assert_eq!(
        update_small_dripleaf(&mut world, lower, PLANT, Direction::Down, dirt()),
        air()
    );
}

#[test]
fn the_base_body_is_the_supports_vegetation_tag_and_nothing_else() {
    for below in [
        "minecraft:dirt",
        "minecraft:grass_block",
        "minecraft:farmland",
    ] {
        assert_eq!(survives_on("minecraft:poppy", below), Some(true), "{below}");
    }
    for below in ["minecraft:stone", "minecraft:air", "minecraft:sand"] {
        assert_eq!(
            survives_on("minecraft:poppy", below),
            Some(false),
            "{below}"
        );
    }
}

#[test]
fn plants_without_their_own_support_rule_share_the_base_body() {
    for name in ["minecraft:oak_sapling", "minecraft:short_grass"] {
        assert_eq!(survives_on(name, "minecraft:dirt"), Some(true), "{name}");
        assert_eq!(survives_on(name, "minecraft:stone"), Some(false), "{name}");
    }
}

#[test]
fn azalea_adds_clay_to_the_base_tag() {
    let name = "minecraft:azalea";
    assert_eq!(survives_on(name, "minecraft:clay"), Some(true));
    assert_eq!(survives_on(name, "minecraft:dirt"), Some(true));
    assert_eq!(survives_on(name, "minecraft:stone"), Some(false));
    assert_eq!(
        survives_on("minecraft:poppy", "minecraft:clay"),
        Some(false)
    );
}

#[test]
fn dry_vegetation_adds_sand_and_terracotta_to_the_base_tag() {
    let name = "minecraft:dead_bush";
    assert_eq!(survives_on(name, "minecraft:sand"), Some(true));
    assert_eq!(survives_on(name, "minecraft:white_terracotta"), Some(true));
    assert_eq!(survives_on(name, "minecraft:dirt"), Some(true));
    assert_eq!(survives_on(name, "minecraft:stone"), Some(false));
}

#[test]
fn nether_sprouts_add_the_nyliums_and_soul_soil_to_the_base_tag() {
    let name = "minecraft:nether_sprouts";
    for below in [
        "minecraft:crimson_nylium",
        "minecraft:soul_soil",
        "minecraft:dirt",
    ] {
        assert_eq!(survives_on(name, below), Some(true), "{below}");
    }
    for below in ["minecraft:netherrack", "minecraft:soul_sand"] {
        assert_eq!(survives_on(name, below), Some(false), "{below}");
    }
}

#[test]
fn nether_wart_replaces_the_base_tag_rather_than_extending_it() {
    let name = "minecraft:nether_wart";
    assert_eq!(survives_on(name, "minecraft:soul_sand"), Some(true));
    for below in ["minecraft:dirt", "minecraft:soul_soil"] {
        assert_eq!(survives_on(name, below), Some(false), "{below}");
    }
}

#[test]
fn the_wither_rose_adds_the_three_nether_grounds_to_the_base_tag() {
    let name = "minecraft:wither_rose";
    for below in [
        "minecraft:netherrack",
        "minecraft:soul_sand",
        "minecraft:dirt",
    ] {
        assert_eq!(survives_on(name, below), Some(true), "{below}");
    }
    assert_eq!(survives_on(name, "minecraft:stone"), Some(false));
}

#[test]
fn stems_need_farmland() {
    for name in ["minecraft:pumpkin_stem", "minecraft:attached_melon_stem"] {
        assert_eq!(
            survives_on(name, "minecraft:farmland"),
            Some(true),
            "{name}"
        );
        assert_eq!(survives_on(name, "minecraft:dirt"), Some(false), "{name}");
    }
}

#[test]
fn nether_fungi_and_roots_take_nylium_soul_soil_and_the_base_tag() {
    let name = "minecraft:crimson_fungus";
    for below in [
        "minecraft:warped_nylium",
        "minecraft:soul_soil",
        "minecraft:dirt",
    ] {
        assert_eq!(survives_on(name, below), Some(true), "{below}");
    }
    for below in ["minecraft:netherrack", "minecraft:soul_sand"] {
        assert_eq!(survives_on(name, below), Some(false), "{below}");
    }
}

#[test]
fn a_cactus_flower_takes_a_centre_sturdy_face_or_the_override_tag() {
    let name = "minecraft:cactus_flower";
    for below in [
        "minecraft:cactus",
        "minecraft:farmland",
        "minecraft:stone",
        "minecraft:oak_fence",
    ] {
        assert_eq!(survives_on(name, below), Some(true), "{below}");
    }
    for below in ["minecraft:air", "minecraft:water", "minecraft:torch"] {
        assert_eq!(survives_on(name, below), Some(false), "{below}");
    }
}

#[test]
fn seagrass_needs_a_full_sturdy_face_that_is_not_a_magma_block() {
    let name = "minecraft:seagrass";
    assert_eq!(survives_on(name, "minecraft:sand"), Some(true));
    assert_eq!(survives_on(name, "minecraft:magma_block"), Some(false));
    assert_eq!(survives_on(name, "minecraft:oak_fence"), Some(false));
    assert_eq!(survives_on(name, "minecraft:air"), Some(false));
}

#[test]
fn a_lily_pad_needs_still_water_or_ice_below_and_nothing_in_its_own_cell() {
    let name = "minecraft:lily_pad";
    assert_eq!(survives_on(name, "minecraft:water"), Some(true));
    assert_eq!(survives_on(name, "minecraft:ice"), Some(true));
    assert_eq!(
        can_survive(
            default_state(name),
            PLANT,
            vanilla_tags(),
            &mut |at| if at == below(PLANT) { water(1) } else { air() }
        ),
        Some(false)
    );
    for below in ["minecraft:stone", "minecraft:lava"] {
        assert_eq!(survives_on(name, below), Some(false), "{below}");
    }
    assert_eq!(
        can_survive(default_state(name), PLANT, vanilla_tags(), &mut |_| water(
            0
        )),
        Some(false)
    );
}

#[test]
fn live_dynamic_support_removes_plants_the_shape_cannot_hold() {
    for name in ["minecraft:cactus_flower", "minecraft:seagrass"] {
        let mut world = NativeWorld::new();
        world.set(at(0, 70, 0), default_state("minecraft:pointed_dripstone"));
        world.set(at(0, 71, 0), default_state(name));
        world.shape_update(at(0, 71, 0), Direction::Down);
        assert_eq!(
            world.state(at(0, 71, 0)),
            block_state_fluid_state(default_state(name)).create_legacy_block(),
            "{name}"
        );
    }
}

#[test]
fn a_stem_pops_off_farmland_that_turned_back_into_dirt() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), farmland_state(0));
    world.set(at(0, 71, 0), default_state("minecraft:pumpkin_stem"));

    world.set(at(0, 70, 0), dirt());
    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), air());
}

#[test]
fn a_flower_becomes_air_when_the_block_under_it_stops_supporting_it() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), default_state("minecraft:poppy"));

    world.set(at(0, 70, 0), stone());
    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:poppy" && *count > 0));
}

#[test]
fn a_supported_flower_is_left_exactly_as_it_was() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), default_state("minecraft:poppy"));

    for direction in [Direction::Down, Direction::North] {
        world.shape_update(at(0, 71, 0), direction);
        assert_eq!(
            world.state(at(0, 71, 0)),
            default_state("minecraft:poppy"),
            "{direction:?}"
        );
    }
    assert!(world.items().is_empty());
}

#[test]
fn the_update_comes_from_any_direction_not_only_from_below() {
    let mut world = NativeWorld::new();
    world.set(at(0, 71, 0), default_state("minecraft:poppy"));

    world.shape_update(at(0, 71, 0), Direction::North);

    assert_eq!(world.state(at(0, 71, 0)), air());
}

#[test]
fn a_plant_losing_support_leaves_its_own_fluid() {
    for (name, support) in [
        ("minecraft:lily_pad", water(0)),
        ("minecraft:seagrass", stone()),
        ("minecraft:warped_fungus", dirt()),
    ] {
        let mut world = NativeWorld::new();
        world.set(at(0, 70, 0), support);
        world.set(at(0, 71, 0), default_state(name));
        world.shape_update(at(0, 71, 0), Direction::Down);
        assert_eq!(world.state(at(0, 71, 0)), default_state(name), "{name}");

        world.set(at(0, 70, 0), air());
        world.shape_update(at(0, 71, 0), Direction::Down);
        assert_eq!(
            world.state(at(0, 71, 0)),
            block_state_fluid_state(default_state(name)).create_legacy_block(),
            "{name}"
        );
    }
}
