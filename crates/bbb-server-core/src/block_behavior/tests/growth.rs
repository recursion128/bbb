use super::native_world::NativeWorld;

use super::*;

fn sugar_cane_state(age: i32) -> i32 {
    state_of("minecraft:sugar_cane", &[("age", &age.to_string())])
}

fn cactus_state(age: i32) -> i32 {
    state_of("minecraft:cactus", &[("age", &age.to_string())])
}

fn nether_wart_state(age: i32) -> i32 {
    state_of("minecraft:nether_wart", &[("age", &age.to_string())])
}

fn cocoa_state(age: i32, facing: &str) -> i32 {
    state_of(
        "minecraft:cocoa",
        &[("age", &age.to_string()), ("facing", facing)],
    )
}

fn pumpkin_stem(age: i32) -> i32 {
    state_of("minecraft:pumpkin_stem", &[("age", &age.to_string())])
}

fn melon_stem(age: i32) -> i32 {
    state_of("minecraft:melon_stem", &[("age", &age.to_string())])
}

fn attached_stem(resource_id: &str, facing: &str) -> i32 {
    state_of(resource_id, &[("facing", facing)])
}

fn bamboo_state(age: i32, leaves: &str, stage: i32) -> i32 {
    state_of(
        "minecraft:bamboo",
        &[
            ("age", &age.to_string()),
            ("leaves", leaves),
            ("stage", &stage.to_string()),
        ],
    )
}

fn sand() -> i32 {
    state_of("minecraft:sand", &[])
}

fn soul_sand() -> i32 {
    state_of("minecraft:soul_sand", &[])
}

fn jungle_log() -> i32 {
    state_of("minecraft:jungle_log", &[("axis", "y")])
}

#[test]
fn the_two_vegetation_subclasses_in_the_family_keep_the_vegetation_survival_body() {
    assert_eq!(
        can_survive(
            nether_wart_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                if pos == at(0, 70, 0) {
                    soul_sand()
                } else {
                    air()
                }
            }
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            nether_wart_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                if pos == at(0, 70, 0) {
                    sand()
                } else {
                    air()
                }
            }
        ),
        Some(false)
    );
    assert_eq!(
        can_survive(pumpkin_stem(0), at(0, 71, 0), vanilla_tags(), &mut |pos| {
            if pos == at(0, 70, 0) {
                farmland_state(0)
            } else {
                air()
            }
        }),
        Some(true)
    );
    assert_eq!(
        can_survive(pumpkin_stem(0), at(0, 71, 0), vanilla_tags(), &mut |pos| {
            if pos == at(0, 70, 0) {
                dirt()
            } else {
                air()
            }
        }),
        Some(false)
    );
}

fn one_cane(age: i32) -> NativeWorld {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), sand());
    world.set(at(1, 70, 0), water(0));
    world.set(at(0, 71, 0), sugar_cane_state(age));
    world
}

#[test]
fn sugar_cane_ages_until_it_reaches_fifteen() {
    let mut world = one_cane(0);

    world.random_tick(at(0, 71, 0), 7);

    assert_eq!(world.state(at(0, 71, 0)), sugar_cane_state(1));
    assert_eq!(world.state(at(0, 72, 0)), air());
}

#[test]
fn a_cane_at_fifteen_grows_a_fresh_one_above_and_resets_itself_to_zero() {
    let mut world = one_cane(sugar_cane::MAX_AGE);

    world.random_tick(at(0, 71, 0), 0);

    assert_eq!(world.state(at(0, 71, 0)), sugar_cane_state(0));
    assert_eq!(world.state(at(0, 72, 0)), sugar_cane_state(0));
}

#[test]
fn a_cane_with_anything_above_it_does_not_even_age() {
    let mut world = one_cane(0);
    world.set(at(0, 72, 0), stone());

    world.random_tick(at(0, 71, 0), 0);

    assert_eq!(world.state(at(0, 71, 0)), sugar_cane_state(0));
}

#[test]
fn the_third_cane_of_a_column_stops_growing_but_keeps_ageing() {
    let mut world = one_cane(0);
    world.set(at(0, 72, 0), sugar_cane_state(0));
    world.set(at(0, 73, 0), sugar_cane_state(4));

    world.random_tick(at(0, 73, 0), 0);

    assert_eq!(world.state(at(0, 73, 0)), sugar_cane_state(4));
    assert_eq!(world.state(at(0, 74, 0)), air());

    world.random_tick(at(0, 72, 0), 0);
    assert_eq!(world.state(at(0, 72, 0)), sugar_cane_state(0));

    let mut two_tall = one_cane(0);
    two_tall.set(at(0, 72, 0), sugar_cane_state(0));
    two_tall.random_tick(at(0, 72, 0), 0);
    assert_eq!(two_tall.state(at(0, 72, 0)), sugar_cane_state(1));
}

#[test]
fn sugar_cane_needs_water_beside_the_block_below_it_not_beside_itself() {
    let sand_at_origin = |pos: BlockPos| if pos == at(0, 70, 0) { sand() } else { air() };
    assert_eq!(
        can_survive(
            sugar_cane_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut sand_at_origin.clone()
        ),
        Some(false)
    );

    assert_eq!(
        can_survive(
            sugar_cane_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                match pos {
                    p if p == at(0, 70, 0) => sand(),
                    p if p == at(1, 71, 0) => water(0),
                    _ => air(),
                }
            }
        ),
        Some(false)
    );
    assert_eq!(
        can_survive(
            sugar_cane_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                match pos {
                    p if p == at(0, 70, 0) => sand(),
                    p if p == at(1, 70, 0) => water(0),
                    _ => air(),
                }
            }
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            sugar_cane_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                if pos == at(0, 70, 0) {
                    sugar_cane_state(3)
                } else {
                    air()
                }
            }
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            sugar_cane_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                match pos {
                    p if p == at(0, 70, 0) => stone(),
                    p if p == at(1, 70, 0) => water(0),
                    _ => air(),
                }
            }
        ),
        Some(false)
    );
}

#[test]
fn losing_its_support_removes_the_cane_on_a_world_tick() {
    let mut world = one_cane(0);
    world.set(at(1, 70, 0), air());

    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), sugar_cane_state(0));
    assert!(world.level_events().is_empty());

    world.run_tick();
    assert_eq!(world.state(at(0, 71, 0)), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:sugar_cane" && *count > 0));
}

#[test]
fn a_cactus_whose_support_went_away_is_destroyed_by_its_own_tick() {
    let mut world = one_cactus(0);
    world.set(at(0, 70, 0), air());

    world.shape_update(at(0, 71, 0), Direction::Down);
    world.run_tick();

    assert_eq!(world.state(at(0, 71, 0)), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:cactus" && *count > 0));
}

#[test]
fn a_bamboo_stalk_whose_support_went_away_is_destroyed_by_its_own_tick() {
    let stalk = bamboo_state(0, "none", 0);
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), stalk);
    world.set(at(0, 70, 0), air());

    world.shape_update(at(0, 71, 0), Direction::Down);
    world.run_tick();

    assert_eq!(world.state(at(0, 71, 0)), air());
}

fn one_cactus(age: i32) -> NativeWorld {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), sand());
    world.set(at(0, 71, 0), cactus_state(age));
    world
}

#[test]
fn a_cactus_at_fifteen_grows_a_new_segment() {
    let mut world = one_cactus(cactus::MAX_AGE);

    world.random_tick(at(0, 71, 0), 0);

    assert_eq!(world.state(at(0, 72, 0)), cactus_state(0));
    assert_eq!(world.state(at(0, 71, 0)), cactus_state(0));
}

#[test]
fn a_three_tall_cactus_at_fifteen_returns_before_it_can_do_anything() {
    let mut world = one_cactus(0);
    world.set(at(0, 72, 0), cactus_state(0));
    world.set(at(0, 73, 0), cactus_state(cactus::MAX_AGE));

    world.random_tick(at(0, 73, 0), 4096);

    assert_eq!(world.state(at(0, 73, 0)), cactus_state(cactus::MAX_AGE));
    assert_eq!(world.state(at(0, 74, 0)), air());
}

#[test]
fn the_cactus_flower_chance_is_one_tenth_short_and_one_quarter_tall() {
    // How many of 4 000 ticks of an age-8 top at `pos` put a flower over it;
    // every tick also ages the top.
    let flowers = |world: &mut NativeWorld, pos: BlockPos| {
        let top = cactus_state(cactus::ATTEMPT_GROW_CACTUS_FLOWER_AGE);
        let flower = default_state("minecraft:cactus_flower");
        world.with_level(|level| {
            (0..4_000)
                .filter(|_| {
                    level.set_block(
                        above(pos),
                        air(),
                        2 | 16,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                    level.set_block(pos, top, 2 | 16, crate::neighbor_updater::UPDATE_LIMIT);
                    crate::block_behavior::random_tick_block(level, pos, top);
                    assert_eq!(
                        level.block_state(pos),
                        cactus_state(cactus::ATTEMPT_GROW_CACTUS_FLOWER_AGE + 1)
                    );
                    level.block_state(above(pos)) == flower
                })
                .count()
        })
    };

    // A tenth is about 400 (sd ~19), a quarter about 1 000 (sd ~27).
    let mut short = one_cactus(0);
    let short = flowers(&mut short, at(0, 71, 0));
    assert!((300..=500).contains(&short), "{short}");

    let mut tall = one_cactus(0);
    tall.set(at(0, 72, 0), cactus_state(0));
    let tall = flowers(&mut tall, at(0, 73, 0));
    assert!((870..=1_130).contains(&tall), "{tall}");
}

#[test]
fn a_cactus_refuses_a_solid_neighbour_lava_and_a_liquid_above_it() {
    let bare = |pos: BlockPos| if pos == at(0, 70, 0) { sand() } else { air() };
    assert_eq!(
        can_survive(
            cactus_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut bare.clone()
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            cactus_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| match pos {
                p if p == at(0, 70, 0) => sand(),
                p if p == at(1, 71, 0) => stone(),
                _ => air(),
            }
        ),
        Some(false)
    );
    assert_eq!(
        can_survive(
            cactus_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| match pos {
                p if p == at(0, 70, 0) => sand(),
                p if p == at(1, 71, 0) => water(0),
                _ => air(),
            }
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            cactus_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| match pos {
                p if p == at(0, 70, 0) => sand(),
                p if p == at(1, 71, 0) => default_state("minecraft:lava"),
                _ => air(),
            }
        ),
        Some(false)
    );
    assert_eq!(
        can_survive(
            cactus_state(0),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| match pos {
                p if p == at(0, 70, 0) => sand(),
                p if p == at(0, 72, 0) => water(0),
                _ => air(),
            }
        ),
        Some(false)
    );
    assert_eq!(
        can_survive(cactus_state(0), at(0, 71, 0), vanilla_tags(), &mut |pos| {
            if pos == at(0, 70, 0) {
                dirt()
            } else {
                air()
            }
        }),
        Some(false)
    );
}

/// How many of 2 000 single random ticks change `state` at `pos`, which is put
/// back before each.
fn changes(world: &mut NativeWorld, pos: BlockPos, state: i32) -> usize {
    world.with_level(|level| {
        (0..2_000)
            .filter(|_| {
                level.set_block(pos, state, 2 | 16, crate::neighbor_updater::UPDATE_LIMIT);
                crate::block_behavior::random_tick_block(level, pos, state);
                level.block_state(pos) != state
            })
            .count()
    })
}

#[test]
fn nether_wart_can_age_or_stay_on_a_random_tick() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), soul_sand());
    world.set(at(0, 71, 0), nether_wart_state(0));

    // One in ten: about 200 of 2 000 (sd ~13).
    let aged = changes(&mut world, at(0, 71, 0), nether_wart_state(0));
    assert!((140..=260).contains(&aged), "{aged}");
    world.random_tick(at(0, 71, 0), 0);
    assert_eq!(world.state(at(0, 71, 0)), nether_wart_state(1));
}

fn one_cocoa(age: i32) -> NativeWorld {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 71, -1), jungle_log());
    world.set(at(0, 71, 0), cocoa_state(age, "north"));
    world
}

#[test]
fn cocoa_can_ripen_or_stay_on_a_random_tick() {
    let mut world = one_cocoa(0);
    // One in five: about 400 of 2 000 (sd ~18).
    let ripened = changes(&mut world, at(0, 71, 0), cocoa_state(0, "north"));
    assert!((310..=490).contains(&ripened), "{ripened}");

    world.random_tick(at(0, 71, 0), 0);
    assert_eq!(world.state(at(0, 71, 0)), cocoa_state(1, "north"));
}

#[test]
fn a_pod_only_notices_the_log_going_away_from_the_side_it_faces() {
    let mut world = one_cocoa(0);
    world.set(at(0, 71, -1), air());

    world.shape_update(at(0, 71, 0), Direction::Up);
    assert_eq!(world.state(at(0, 71, 0)), cocoa_state(0, "north"));

    world.shape_update(at(0, 71, 0), Direction::North);
    assert_eq!(world.state(at(0, 71, 0)), air());
}

#[test]
fn cocoa_survives_on_every_jungle_log_variant_and_on_nothing_else() {
    for name in ["minecraft:jungle_log", "minecraft:stripped_jungle_wood"] {
        assert_eq!(
            can_survive(
                cocoa_state(0, "north"),
                at(0, 71, 0),
                vanilla_tags(),
                &mut |pos| {
                    if pos == at(0, 71, -1) {
                        default_state(name)
                    } else {
                        air()
                    }
                }
            ),
            Some(true),
            "{name}"
        );
    }
    assert_eq!(
        can_survive(
            cocoa_state(0, "north"),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                if pos == at(0, 71, -1) {
                    default_state("minecraft:oak_log")
                } else {
                    air()
                }
            }
        ),
        Some(false)
    );
    assert_eq!(
        can_survive(
            cocoa_state(0, "south"),
            at(0, 71, 0),
            vanilla_tags(),
            &mut |pos| {
                if pos == at(0, 71, -1) {
                    jungle_log()
                } else {
                    air()
                }
            }
        ),
        Some(false)
    );
}

fn one_stem(stem: i32) -> NativeWorld {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    for x in -2..=2 {
        for z in -2..=2 {
            world.set(at(x, 70, z), dirt());
        }
    }
    world.set(at(0, 70, 0), farmland_state(0));
    world.set(at(0, 71, 0), stem);
    world
}

#[test]
fn a_stem_below_max_age_just_ages_on_the_crop_growth_draw() {
    let mut world = one_stem(pumpkin_stem(3));

    world.random_tick(at(0, 71, 0), 42);

    assert_eq!(world.state(at(0, 71, 0)), pumpkin_stem(4));
}

#[test]
fn a_grown_pumpkin_stem_plants_pumpkin_and_faces_its_fruit() {
    let mut world = one_stem(pumpkin_stem(stem::MAX_AGE));
    for seed in 0..64 {
        world.random_tick(at(0, 71, 0), seed);
        if block_type_of(world.state(at(0, 71, 0))) != block_type_of(pumpkin_stem(stem::MAX_AGE)) {
            break;
        }
    }
    let attached = world.state(at(0, 71, 0));
    assert_eq!(
        block_type_of(attached),
        block_type_of(default_state("minecraft:attached_pumpkin_stem"))
    );
    let direction = match string_property(attached, "facing").unwrap().as_str() {
        "north" => Direction::North,
        "east" => Direction::East,
        "south" => Direction::South,
        "west" => Direction::West,
        _ => unreachable!(),
    };
    assert_eq!(
        world.state(relative(at(0, 71, 0), direction)),
        default_state("minecraft:pumpkin")
    );
}
#[test]
fn a_grown_stem_with_all_fruit_targets_blocked_or_unsupported_stays_a_stem() {
    for blocked in [true, false] {
        let mut world = one_stem(pumpkin_stem(stem::MAX_AGE));
        for direction in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ] {
            let target = relative(at(0, 71, 0), direction);
            world.set(if blocked { target } else { below(target) }, stone());
        }
        for seed in 0..64 {
            world.random_tick(at(0, 71, 0), seed);
        }
        assert_eq!(world.state(at(0, 71, 0)), pumpkin_stem(stem::MAX_AGE));
    }
}

#[test]
fn a_stem_in_the_dark_never_grows() {
    let mut world = one_stem(pumpkin_stem(3));
    world.illuminate(at(0, 71, 0), crop::GROWTH_LIGHT_THRESHOLD - 1);

    world.random_tick(at(0, 71, 0), 42);

    assert_eq!(world.state(at(0, 71, 0)), pumpkin_stem(3));
}

#[test]
fn attached_stems_become_their_registered_grown_stem_when_fruit_is_removed() {
    {
        let (attached, fruit, grown) = (
            "minecraft:attached_melon_stem",
            "minecraft:melon",
            melon_stem(stem::MAX_AGE),
        );
        let mut world = one_stem(attached_stem(attached, "east"));
        world.set(at(1, 71, 0), default_state(fruit));
        world.set(at(1, 71, 0), air());

        world.shape_update(at(0, 71, 0), Direction::East);

        assert_eq!(world.state(at(0, 71, 0)), grown, "{attached}");
    }
}

#[test]
fn attached_stem_keeps_pointing_at_its_fruit_and_ignores_other_sides() {
    let attached = attached_stem("minecraft:attached_pumpkin_stem", "east");
    let mut world = one_stem(attached);
    world.set(at(1, 71, 0), default_state("minecraft:pumpkin"));

    world.shape_update(at(0, 71, 0), Direction::East);
    world.shape_update(at(0, 71, 0), Direction::North);

    assert_eq!(world.state(at(0, 71, 0)), attached);
}

#[test]
fn attached_stem_still_uses_vegetation_survival_when_support_disappears() {
    let attached = attached_stem("minecraft:attached_melon_stem", "east");
    let mut world = one_stem(attached);
    world.set(at(0, 70, 0), air());

    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), air());
}

#[test]
fn a_bamboo_sapling_grows_the_first_stalk_with_small_leaves() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), default_state("minecraft:bamboo_sapling"));

    world.random_tick(at(0, 71, 0), 0);

    assert_eq!(world.state(at(0, 72, 0)), bamboo_state(0, "small", 0));
    assert_eq!(world.state(at(0, 71, 0)), default_state("minecraft:bamboo"));
}

#[test]
fn a_bamboo_sapling_becomes_a_stalk_when_bamboo_appears_above_it() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), default_state("minecraft:bamboo_sapling"));
    world.set(at(0, 72, 0), bamboo_state(0, "small", 0));

    world.shape_update(at(0, 71, 0), Direction::Up);

    assert_eq!(world.state(at(0, 71, 0)), default_state("minecraft:bamboo"));
}

#[test]
fn a_bamboo_sapling_without_support_becomes_air_rather_than_scheduling_a_tick() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), stone());
    world.set(at(0, 71, 0), default_state("minecraft:bamboo_sapling"));

    world.shape_update(at(0, 71, 0), Direction::Down);

    assert_eq!(world.state(at(0, 71, 0)), air());
}

#[test]
fn a_growing_stalk_takes_small_leaves_over_a_bare_segment() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), bamboo_state(0, "none", 0));

    world.random_tick(at(0, 71, 0), 0);

    assert_eq!(world.state(at(0, 72, 0)), bamboo_state(0, "small", 0));
}

#[test]
fn a_growing_stalk_takes_large_leaves_and_pushes_the_ones_below_it_down() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), bamboo_state(0, "small", 0));
    world.set(at(0, 72, 0), bamboo_state(0, "small", 0));
    world.set(at(0, 73, 0), bamboo_state(0, "small", 0));

    world.random_tick(at(0, 73, 0), 0);

    let leaves_at = |world: &mut NativeWorld, pos| string_property(world.state(pos), "leaves");
    assert_eq!(
        leaves_at(&mut world, at(0, 74, 0)).as_deref(),
        Some("large")
    );
    assert_eq!(
        leaves_at(&mut world, at(0, 73, 0)).as_deref(),
        Some("small")
    );
    assert_eq!(
        leaves_at(&mut world, at(0, 72, 0)).as_deref(),
        Some("small")
    );
    assert_eq!(leaves_at(&mut world, at(0, 71, 0)).as_deref(), Some("none"));
}

#[test]
fn a_thick_new_segment_thickens_the_whole_column_under_it() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    for y in 1..=3 {
        world.set(at(0, y + 70, 0), bamboo_state(0, "small", 0));
    }

    world.random_tick(at(0, 73, 0), 0);

    for y in 1..=4 {
        assert_eq!(
            int_property(world.state(at(0, y + 70, 0)), "age"),
            Some(bamboo::AGE_THICK_BAMBOO),
            "y={y}"
        );
    }
}

#[test]
fn a_stalk_stops_at_sixteen_segments() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    for y in 1..=16 {
        world.set(at(0, y + 70, 0), bamboo_state(0, "none", 0));
    }

    world.random_tick(at(0, 86, 0), 0);

    assert_eq!(world.state(at(0, 87, 0)), air());
}

#[test]
fn a_stalk_at_fifteen_segments_latches_stage() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    for y in 1..=15 {
        world.set(at(0, y + 70, 0), bamboo_state(0, "none", 0));
    }

    world.random_tick(at(0, 85, 0), 0);

    assert_eq!(
        int_property(world.state(at(0, 86, 0)), "stage"),
        Some(bamboo::STAGE_DONE_GROWING)
    );
}

#[test]
fn a_latched_segment_never_ticks_again() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    world.set(
        at(0, 71, 0),
        bamboo_state(0, "none", bamboo::STAGE_DONE_GROWING),
    );

    world.random_tick(at(0, 71, 0), 0);

    assert_eq!(world.state(at(0, 72, 0)), air());
}

#[test]
fn a_thin_segment_thickens_from_the_thick_one_above_it() {
    let mut world = NativeWorld::ticking(at(0, 71, 0));
    world.set(at(0, 70, 0), dirt());
    world.set(at(0, 71, 0), bamboo_state(0, "none", 0));
    world.set(at(0, 72, 0), bamboo_state(1, "none", 0));

    world.shape_update(at(0, 71, 0), Direction::Up);

    assert_eq!(world.state(at(0, 71, 0)), bamboo_state(1, "none", 0));

    world.shape_update(at(0, 71, 0), Direction::Up);
    assert_eq!(world.state(at(0, 71, 0)), bamboo_state(1, "none", 0));
}

#[test]
fn both_bamboo_classes_share_one_survival_tag() {
    for name in ["minecraft:gravel", "minecraft:dirt"] {
        assert!(
            bamboo::can_survive(
                at(0, 71, 0),
                vanilla_tags(),
                &mut |pos| if pos == at(0, 70, 0) {
                    default_state(name)
                } else {
                    air()
                }
            ),
            "{name}"
        );
    }
    assert!(!bamboo::can_survive(
        at(0, 71, 0),
        vanilla_tags(),
        &mut |pos| if pos == at(0, 70, 0) { stone() } else { air() }
    ));
}

/// Bone meal near the build limit needs room for the whole tree or mushroom:
/// the trunk base height (SaplingBlock.java:60-67, TreeGrower.java:199-209),
/// that plus two for azalea (AzaleaBlock.java:43-50), and `4 + foliageRadius`
/// for a huge mushroom (MushroomBlock.java:105-125).
#[test]
fn bone_meal_targets_need_room_below_the_build_limit() {
    // oak base_height 4, azalea_tree base_height 4, huge_brown_mushroom foliage_radius 3.
    for (block, room) in [
        ("minecraft:oak_sapling", 4),
        ("minecraft:azalea", 6),
        ("minecraft:brown_mushroom", 7),
    ] {
        for (y_below_limit, valid) in [(room, true), (room - 1, false)] {
            let mut level = NativeWorld::new();
            let pos = BlockPos {
                x: 0,
                y: 319 - y_below_limit,
                z: 0,
            };
            level.set(pos, default_state(block));
            assert_eq!(
                level.with_level(|ctx| crate::block_behavior::bonemeal::grow_crop(ctx, pos)),
                valid,
                "{block} {y_below_limit} below the limit"
            );
        }
    }
}
