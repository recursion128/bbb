use super::*;
use crate::block_behavior::fire;
use bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID;
use bbb_server_types::save::Java26BuiltinDimension;
const FIRE: BlockPos = BlockPos { x: 2, y: 70, z: 3 };
fn frame(world: &mut NativeWorld, axis: &str) {
    let obsidian = default_state("minecraft:obsidian");
    let (left, right, interiors) = match axis {
        "x" => (
            at(FIRE.x + 1, FIRE.y, FIRE.z),
            at(FIRE.x - 2, FIRE.y, FIRE.z),
            vec![FIRE, at(FIRE.x - 1, FIRE.y, FIRE.z)],
        ),
        "z" => (
            at(FIRE.x, FIRE.y, FIRE.z - 1),
            at(FIRE.x, FIRE.y, FIRE.z + 2),
            vec![FIRE, at(FIRE.x, FIRE.y, FIRE.z + 1)],
        ),
        _ => unreachable!(),
    };
    for y in FIRE.y..FIRE.y + 3 {
        world.set(BlockPos { y, ..left }, obsidian);
        world.set(BlockPos { y, ..right }, obsidian);
    }
    for pos in interiors {
        world.set(below(pos), obsidian);
        world.set(offset(pos, 0, 3, 0), obsidian);
    }
}

fn axis(state: i32) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)?
        .properties
        .get("axis")
        .cloned()
}

#[test]
fn base_fire_on_place_lights_an_x_portal() {
    let mut world = NativeWorld::new();
    frame(&mut world, "x");
    let state = default_state("minecraft:fire");
    load_state(&mut world, FIRE, state);

    world.with_level(|level| on_place(level, FIRE, state, air(), false));

    for pos in [
        at(FIRE.x - 1, FIRE.y, FIRE.z),
        FIRE,
        at(FIRE.x - 1, FIRE.y + 1, FIRE.z),
        at(FIRE.x, FIRE.y + 1, FIRE.z),
        at(FIRE.x - 1, FIRE.y + 2, FIRE.z),
        at(FIRE.x, FIRE.y + 2, FIRE.z),
    ] {
        assert_eq!(
            world.state(pos),
            state_of("minecraft:nether_portal", &[("axis", "x")]),
            "{pos:?}"
        );
    }
}

#[test]
fn base_fire_on_place_falls_back_to_a_z_portal() {
    let mut world = NativeWorld::new();
    frame(&mut world, "z");
    let state = default_state("minecraft:soul_fire");
    load_state(&mut world, FIRE, state);

    world.with_level(|level| on_place(level, FIRE, state, air(), false));

    for dy in 0..3 {
        for dz in 0..2 {
            assert_eq!(
                axis(world.state(offset(FIRE, 0, dy, dz))).as_deref(),
                Some("z")
            );
        }
    }
}

fn portal_interior(axis: &str) -> Vec<BlockPos> {
    (0..3)
        .flat_map(|dy| {
            (0..2).map(move |column| match axis {
                "x" => offset(FIRE, -column, dy, 0),
                "z" => offset(FIRE, 0, dy, column),
                _ => unreachable!(),
            })
        })
        .collect()
}

fn lit_portal(axis: &str) -> NativeWorld {
    let mut world = NativeWorld::new();
    frame(&mut world, axis);
    for pos in portal_interior(axis) {
        load_state(
            &mut world,
            pos,
            state_of("minecraft:nether_portal", &[("axis", axis)]),
        );
    }
    world
}

#[test]
fn portal_shape_updates_preserve_complete_frames_and_ignore_the_wrong_horizontal_axis() {
    for (axis, wrong_direction, frame_direction) in [
        ("x", Direction::North, Direction::East),
        ("z", Direction::East, Direction::North),
    ] {
        let mut world = lit_portal(axis);
        world.shape_update(FIRE, frame_direction);
        world.shape_update(FIRE, Direction::Down);
        for pos in portal_interior(axis) {
            assert_eq!(self::axis(world.state(pos)).as_deref(), Some(axis));
        }

        world.set(below(FIRE), air());
        world.shape_update(FIRE, wrong_direction);
        for pos in portal_interior(axis) {
            assert_eq!(self::axis(world.state(pos)).as_deref(), Some(axis));
        }
        world.shape_update(FIRE, Direction::Down);
        for pos in portal_interior(axis) {
            assert_eq!(world.state(pos), air(), "{axis} {pos:?}");
        }
    }
}

#[test]
fn breaking_a_portal_frame_collapses_the_connected_portal() {
    for axis in ["x", "z"] {
        for broken_frame in [
            below(FIRE),
            offset(FIRE, 0, 3, 0),
            match axis {
                "x" => offset(FIRE, 1, 1, 0),
                "z" => offset(FIRE, 0, 1, -1),
                _ => unreachable!(),
            },
        ] {
            let mut world = lit_portal(axis);
            world.with_level(|level| {
                level.set_block(
                    broken_frame,
                    air(),
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            });
            for pos in portal_interior(axis) {
                assert_eq!(world.state(pos), air(), "{axis} {broken_frame:?} {pos:?}");
            }
        }
    }
}

#[test]
fn base_fire_on_place_removes_unsupported_fire() {
    let mut world = NativeWorld::new();
    world.set(below(FIRE), air());
    let state = default_state("minecraft:fire");
    load_state(&mut world, FIRE, state);

    world.with_level(|level| on_place(level, FIRE, state, air(), false));

    assert_eq!(world.state(FIRE), air());
}

#[test]
fn placement_survival_and_shape_are_directional() {
    let mut world = NativeWorld::new();
    world.set(below(FIRE), air());
    world.set(relative(FIRE, Direction::East), oak_log());
    let state =
        fire::state_for_placement(FIRE, vanilla_tags(), &mut |pos| world.state(pos)).unwrap();
    assert_eq!(block_type_of(state), Some(VANILLA_BLOCK_FIRE_ID));
    assert_eq!(bool_property(state, "east"), Some(true));
    assert_eq!(bool_property(state, "up"), Some(false));
    assert_eq!(
        fire::can_survive(state, FIRE, &mut |pos| world.state(pos)),
        Some(true)
    );

    load_state(&mut world, FIRE, state);
    world.set(relative(FIRE, Direction::East), air());
    world.shape_update(FIRE, Direction::East);
    assert_eq!(world.state(FIRE), air());
}

fn load_state(world: &mut NativeWorld, pos: BlockPos, state: i32) {
    // A saved fire or portal state has already passed its placement callback.
    world
        .world
        .set_raw_block_state_for_test(Java26BuiltinDimension::Overworld, pos, state);
}

fn set_random(world: &mut NativeWorld, seed: i64) {
    world.with_level(|level| level.level_random().reseed(seed));
}

#[test]
fn lava_ignites_flammable_material_and_respects_the_spread_gate() {
    let lava = offset(FIRE, 0, 2, 0);
    for (material, spread, blocked, ignites) in [
        ("minecraft:oak_log", true, false, true),
        ("minecraft:coal_block", true, false, false),
        ("minecraft:oak_log", false, false, false),
        ("minecraft:oak_log", true, true, false),
    ] {
        let mut world = NativeWorld::new();
        for dx in -1..=1 {
            for dz in -1..=1 {
                world.set(offset(lava, dx, 0, dz), default_state(material));
            }
        }
        if blocked {
            for dx in -2..=2 {
                for dz in -2..=2 {
                    for dy in 1..=3 {
                        world.set(offset(lava, dx, dy, dz), stone());
                    }
                }
            }
        }
        load_state(&mut world, lava, default_state("minecraft:lava"));
        if spread {
            world.command("gamerule fire_spread_radius_around_player -1");
        }
        set_random(&mut world, 1);
        for _ in 0..64 {
            world.world.run_live_random_tick_sample_for_test(
                Java26BuiltinDimension::Overworld,
                lava,
                default_state("minecraft:lava"),
            );
        }
        let found = (-1..=1)
            .any(|dx| (-1..=1).any(|dz| fire::is_fire(world.state(offset(lava, dx, 1, dz)))));
        assert_eq!(found, ignites, "{material} spread={spread}");
    }
}

#[test]
fn soul_fire_placement_uses_the_actual_support_material() {
    for (base, expected) in [
        ("minecraft:soul_sand", "minecraft:soul_fire"),
        ("minecraft:stone", "minecraft:fire"),
    ] {
        let mut world = NativeWorld::new();
        world.set(below(FIRE), default_state(base));
        let state =
            fire::state_for_placement(FIRE, vanilla_tags(), &mut |pos| world.state(pos)).unwrap();
        world.set(FIRE, state);
        assert_eq!(
            block_type_of(world.state(FIRE)),
            block_type_of(default_state(expected))
        );
    }
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_ignited_by_lava(default_state(
            "minecraft:chest"
        )),
        Some(true)
    );
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_ignited_by_lava(default_state(
            "minecraft:coal_block"
        )),
        Some(false)
    );
}

fn burning_world(raining: bool, covered: usize) -> NativeWorld {
    let mut world = NativeWorld::ticking(FIRE);
    world.command("gamerule fire_spread_radius_around_player -1");
    world.command("gamerule random_tick_speed 0");
    world.command("difficulty hard");
    if raining {
        world.command("weather rain");
        for _ in 0..30 {
            world.run_tick();
        }
    }
    world.set(below(FIRE), stone());
    for dy in 0..=3 {
        for dz in -1..=1 {
            for dx in [-1, 1] {
                world.set(
                    offset(FIRE, dx, dy, dz),
                    default_state("minecraft:hay_block"),
                );
            }
        }
    }
    for &(dx, dz) in &[(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)][..covered] {
        world.set(offset(FIRE, dx, 6, dz), stone());
    }
    load_state(&mut world, FIRE, default_state("minecraft:fire"));
    set_random(&mut world, 4);
    world
}

fn start_fire(world: &mut NativeWorld) {
    let state = world.state(FIRE);
    world.with_level(|level| {
        level.schedule_block_tick(
            FIRE,
            bbb_server_types::chunk_ticks::BlockTickType::of_block_state(state).unwrap(),
            1,
            bbb_server_types::chunk_ticks::TickPriority::Normal,
        );
    });
}

fn burn(world: &mut NativeWorld, ticks: usize) {
    start_fire(world);
    for _ in 0..ticks {
        if !fire::is_fire(world.state(FIRE)) {
            break;
        }
        world.run_tick();
    }
}

#[test]
fn a_dry_fire_tick_burns_its_neighbours_and_spreads() {
    let mut world = burning_world(false, 0);
    start_fire(&mut world);
    let mut fire = false;
    let mut consumed = false;
    for _ in 0..1600 {
        world.run_tick();
        for dy in 0..=3 {
            for dz in -1..=1 {
                for dx in [-1, 1] {
                    let state = world.state(offset(FIRE, dx, dy, dz));
                    fire |= fire::is_fire(state);
                    consumed |= state != default_state("minecraft:hay_block");
                }
            }
        }
        if fire && consumed {
            break;
        }
    }
    assert!(fire && consumed);
}

#[test]
fn actual_rain_extinguishes_exposed_fire_and_a_roof_protects_it() {
    for covered in [0, 4, 5] {
        let mut world = burning_world(true, covered);
        set_random(&mut world, 1);
        burn(&mut world, if covered == 5 { 40 } else { 1600 });
        assert_eq!(fire::is_fire(world.state(FIRE)), covered == 5);
    }
    for covered in [false, true] {
        let mut world = NativeWorld::ticking(FIRE);
        world.command("gamerule fire_spread_radius_around_player -1");
        world.command("gamerule random_tick_speed 0");
        world.command("difficulty hard");
        world.command("weather rain");
        for _ in 0..30 {
            world.run_tick();
        }
        let neighbour = relative(FIRE, Direction::East);
        world.set(below(FIRE), default_state("minecraft:netherrack"));
        world.set(below(neighbour), stone());
        world.set(relative(neighbour, Direction::East), stone());
        world.set(
            neighbour,
            state_of(
                "minecraft:vine",
                &[
                    ("east", "true"),
                    ("west", "false"),
                    ("north", "false"),
                    ("south", "false"),
                    ("up", "false"),
                ],
            ),
        );
        if covered {
            world.set(offset(neighbour, 0, 6, 0), stone());
        }
        assert_eq!(
            world.with_level(|level| level.is_raining_at(neighbour)),
            !covered
        );
        load_state(&mut world, FIRE, default_state("minecraft:fire"));
        set_random(&mut world, 1);
        start_fire(&mut world);
        let mut consumed = false;
        let mut ignited = false;
        for _ in 0..2400 {
            world.run_tick();
            let state = world.state(neighbour);
            consumed |= block_type_of(state) != block_type_of(default_state("minecraft:vine"));
            ignited |= fire::is_fire(state);
            if consumed && (!covered || ignited) {
                break;
            }
        }
        assert!(consumed);
        // Rain never lets the burning vine turn to fire; under the roof it
        // may burn away or catch.
        if !covered {
            assert!(!ignited);
        }
    }
}

#[test]
fn a_portal_sample_spawns_over_its_actual_column_floor() {
    use bbb_protocol::packets::Vec3d;
    for (column_height, monsters, piglins, nearby, difficulty, floor) in [
        (1, true, true, true, 3, "minecraft:obsidian"),
        (2, true, true, true, 3, "minecraft:obsidian"),
        (1, false, true, true, 3, "minecraft:obsidian"),
        (1, true, false, true, 3, "minecraft:obsidian"),
        (1, true, true, false, 3, "minecraft:obsidian"),
        (1, true, true, true, 0, "minecraft:obsidian"),
        (1, true, true, true, 3, "minecraft:ice"),
    ] {
        let mut world = NativeWorld::new();
        world.command("gamerule random_tick_speed 0");
        world.join_listener(Vec3d {
            x: if nearby { 2.5 } else { 1000.0 },
            y: 70.0,
            z: 6.5,
        });
        world.command(if difficulty == 0 {
            "difficulty peaceful"
        } else {
            "difficulty hard"
        });
        if !monsters {
            world.command("gamerule spawn_monsters false");
        }
        let dimension = if piglins {
            Java26BuiltinDimension::Overworld
        } else {
            world.command("execute in minecraft:the_nether run tp listener 2.5 70 6.5");
            Java26BuiltinDimension::Nether
        };
        world
            .world
            .set_block(dimension, below(FIRE), default_state(floor), 2 | 16);
        for dy in 0..column_height {
            world.world.set_raw_block_state_for_test(
                dimension,
                offset(FIRE, 0, dy, 0),
                default_state("minecraft:nether_portal"),
            );
        }
        let ticked = offset(FIRE, 0, column_height - 1, 0);
        // A spawn is `nextInt(2000) < difficulty` per tick, so 16 384 ticks
        // miss one at hard with odds of about e^-24.
        for _ in 0..16_384 {
            world.world.run_live_random_tick_sample_for_test(
                dimension,
                ticked,
                default_state("minecraft:nether_portal"),
            );
            if world.world.stored_entity_ids_for_test(dimension).into_iter().any(|id| {
                world.world.stored_entity_for_test(dimension, id).is_some_and(|entity| {
                    entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
                })
            }) {
                break;
            }
        }
        let found: Vec<_> = world
            .world
            .stored_entity_ids_for_test(dimension)
            .into_iter()
            .filter_map(|id| {
                let entity = world.world.stored_entity_for_test(dimension, id)?;
                let position = entity.position();
                // Generated chunks may hold their own piglins; only the
                // portal's column counts.
                (entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
                    && (position.x - 2.5).abs() < 1.0
                    && (position.z - 3.5).abs() < 1.0)
                    .then_some(position)
            })
            .collect();
        if monsters && piglins && nearby && difficulty > 0 && floor != "minecraft:ice" {
            assert!(found
                .iter()
                .any(|position| position.x == 2.5 && position.y == 70.0 && position.z == 3.5));
        } else {
            assert!(found.is_empty(), "height={column_height} monsters={monsters} piglins={piglins} nearby={nearby} difficulty={difficulty} floor={floor}: {found:?}");
        }
    }
}
