use super::native_world::NativeWorld;

use super::*;

const AT: BlockPos = BlockPos { x: 6, y: 80, z: -4 };

fn survives_with(state: i32, cell: BlockPos, neighbour: i32) -> Option<bool> {
    can_survive(state, AT, vanilla_tags(), &mut |pos| {
        if pos == cell {
            neighbour
        } else {
            air()
        }
    })
}

#[test]
fn a_candle_cake_wants_a_solid_cell_under_it() {
    let cake = default_state("minecraft:candle_cake");
    assert_eq!(survives_with(cake, below(AT), stone()), Some(true));
    assert_eq!(
        survives_with(cake, below(AT), default_state("minecraft:oak_fence")),
        Some(true)
    );
    assert_eq!(survives_with(cake, below(AT), air()), Some(false));

    let mut world = NativeWorld::new();
    world.set(AT, cake);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), cake);
    world.shape_update(AT, Direction::Down);
    assert_eq!(world.state(AT), air());
}

#[test]
fn a_sea_pickle_takes_any_non_empty_top_face() {
    let pickle = state_of(
        "minecraft:sea_pickle",
        &[("pickles", "1"), ("waterlogged", "true")],
    );
    for (name, expected) in [
        ("minecraft:stone", true),
        ("minecraft:oak_fence", true),
        ("minecraft:torch", false),
        ("minecraft:air", false),
    ] {
        assert_eq!(
            survives_with(pickle, below(AT), default_state(name)),
            Some(expected),
            "{name}"
        );
    }

    let mut world = NativeWorld::new();
    world.set(AT, pickle);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), water(0));
}

#[test]
fn a_snow_layer_reads_two_tags_before_it_reads_a_shape() {
    let snow = |layers: i32| state_of("minecraft:snow", &[("layers", &layers.to_string())]);
    for (name, expected) in [
        ("minecraft:ice", false),
        ("minecraft:soul_sand", true),
        ("minecraft:stone", true),
        ("minecraft:air", false),
    ] {
        assert_eq!(
            survives_with(snow(1), below(AT), default_state(name)),
            Some(expected),
            "{name}"
        );
    }
    assert_eq!(survives_with(snow(1), below(AT), snow(8)), Some(true));
    assert_eq!(survives_with(snow(1), below(AT), snow(7)), Some(false));

    let mut world = NativeWorld::new();
    world.set(below(AT), default_state("minecraft:bamboo"));
    world.set(AT, snow(1));
    world.shape_update(AT, Direction::Down);
    assert_eq!(world.state(AT), air());

    let mut world = NativeWorld::new();
    world.set(below(AT), default_state("minecraft:scaffolding"));
    world.set(AT, snow(1));
    world.shape_update(AT, Direction::Down);
    assert_eq!(world.state(AT), snow(1));
}

#[test]
fn a_spore_blossom_hangs_from_a_centre_and_refuses_water_in_its_own_cell() {
    let blossom = default_state("minecraft:spore_blossom");
    assert_eq!(survives_with(blossom, above(AT), stone()), Some(true));
    assert_eq!(survives_with(blossom, above(AT), air()), Some(false));
    assert_eq!(
        can_survive(
            blossom,
            AT,
            vanilla_tags(),
            &mut |pos| if pos == above(AT) { stone() } else { water(0) }
        ),
        Some(false)
    );

    let mut world = NativeWorld::new();
    world.set(AT, blossom);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), blossom);
    world.shape_update(AT, Direction::Up);
    assert_eq!(world.state(AT), air());
}

#[test]
fn hanging_roots_want_a_full_face_above_and_re_arm_their_water_after_the_gate() {
    let roots = |waterlogged: bool| {
        state_of(
            "minecraft:hanging_roots",
            &[("waterlogged", if waterlogged { "true" } else { "false" })],
        )
    };
    assert_eq!(survives_with(roots(false), above(AT), stone()), Some(true));
    assert_eq!(
        survives_with(
            roots(false),
            above(AT),
            default_state("minecraft:oak_fence")
        ),
        Some(false)
    );

    let mut world = NativeWorld::new();
    world.set(AT, roots(true));
    world.shape_update(AT, Direction::Up);
    assert_eq!(world.state(AT), water(0));

    let mut world = NativeWorld::new();
    world.set(above(AT), stone());
    world.set(AT, roots(true));
    world.shape_update(AT, Direction::Up);
    assert_eq!(world.state(AT), roots(true));
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && tick.x == AT.x
            && tick.y == AT.y
            && tick.z == AT.z
            && tick.delay == 5));
}

#[test]
fn a_mangrove_propagule_swaps_tag_and_cell_when_it_hangs() {
    let propagule = |hanging: bool| {
        state_of(
            "minecraft:mangrove_propagule",
            &[
                ("age", "0"),
                ("hanging", if hanging { "true" } else { "false" }),
                ("stage", "0"),
                ("waterlogged", "false"),
            ],
        )
    };
    let leaves = default_state("minecraft:mangrove_leaves");
    assert_eq!(
        survives_with(propagule(true), above(AT), leaves),
        Some(true)
    );
    assert_eq!(
        survives_with(propagule(true), below(AT), dirt()),
        Some(false)
    );
    assert_eq!(
        survives_with(propagule(false), below(AT), dirt()),
        Some(true)
    );
    assert_eq!(
        survives_with(propagule(false), above(AT), leaves),
        Some(false)
    );
}

#[test]
fn leaf_litter_asks_for_the_face_under_it_rather_than_the_vegetation_tag() {
    let litter = state_of(
        "minecraft:leaf_litter",
        &[("facing", "north"), ("segment_amount", "1")],
    );
    assert_eq!(survives_with(litter, below(AT), stone()), Some(true));
    assert_eq!(survives_with(litter, below(AT), air()), Some(false));

    let mut world = NativeWorld::new();
    world.set(AT, litter);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), air());
}

#[test]
fn a_dirt_path_reverts_through_a_scheduled_tick() {
    let path = default_state("minecraft:dirt_path");
    assert_eq!(survives_with(path, above(AT), air()), Some(true));
    assert_eq!(survives_with(path, above(AT), stone()), Some(false));
    assert_eq!(
        survives_with(path, above(AT), default_state("minecraft:oak_fence_gate")),
        Some(true)
    );

    let mut world = NativeWorld::new();
    world.set(above(AT), stone());
    world.set(AT, path);

    world.shape_update(AT, Direction::North);

    world.shape_update(AT, Direction::Up);

    {
        let state = world.state(AT);
        world.with_level(|level| tick_block(level, AT, state));
    }
    assert_eq!(world.state(AT), dirt());
}

#[test]
fn farmland_answers_the_same_question_through_the_table() {
    let farmland = default_state("minecraft:farmland");
    assert_eq!(survives_with(farmland, above(AT), air()), Some(true));
    assert_eq!(survives_with(farmland, above(AT), stone()), Some(false));
    assert_eq!(
        survives_with(farmland, above(AT), default_state("minecraft:wheat")),
        Some(true)
    );
}

#[test]
fn a_grindstone_says_yes_where_its_parent_would_have_said_no() {
    let grindstone = state_of(
        "minecraft:grindstone",
        &[("face", "wall"), ("facing", "north")],
    );
    assert_eq!(
        survives_with(grindstone, relative(AT, Direction::South), air()),
        Some(true)
    );
    assert_eq!(survives_with(grindstone, below(AT), air()), Some(true));
}

#[test]
fn a_pressure_plate_takes_either_a_rigid_floor_or_a_centre() {
    let plate = default_state("minecraft:stone_pressure_plate");
    for (name, expected) in [
        ("minecraft:stone", true),
        ("minecraft:oak_fence", true),
        ("minecraft:hopper", true),
        ("minecraft:oak_slab", false),
        ("minecraft:air", false),
    ] {
        assert_eq!(
            survives_with(plate, below(AT), default_state(name)),
            Some(expected),
            "{name}"
        );
    }

    let mut world = NativeWorld::new();
    world.set(AT, plate);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), plate);
    world.shape_update(AT, Direction::Down);
    assert_eq!(world.state(AT), air());
}

#[test]
fn a_lever_reads_whichever_cell_face_and_facing_between_them_name() {
    let lever = |face: &str, facing: &str| {
        state_of(
            "minecraft:lever",
            &[("face", face), ("facing", facing), ("powered", "false")],
        )
    };
    for (face, facing, cell) in [
        ("floor", "north", below(AT)),
        ("ceiling", "north", above(AT)),
        ("wall", "north", relative(AT, Direction::South)),
        ("wall", "east", relative(AT, Direction::West)),
    ] {
        let state = lever(face, facing);
        assert_eq!(
            survives_with(state, cell, stone()),
            Some(true),
            "{face}/{facing}"
        );
        assert_eq!(
            survives_with(state, cell, air()),
            Some(false),
            "{face}/{facing}"
        );
        assert_eq!(
            survives_with(state, cell, default_state("minecraft:oak_fence")),
            Some(false),
            "{face}/{facing}"
        );
    }

    let mut world = NativeWorld::new();
    let wall_lever = lever("wall", "north");
    world.set(AT, wall_lever);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), wall_lever);
    world.shape_update(AT, Direction::South);
    assert_eq!(world.state(AT), air());
}

#[test]
fn a_tripwire_hook_holds_on_to_the_cell_behind_it() {
    let hook = |facing: &str| {
        state_of(
            "minecraft:tripwire_hook",
            &[
                ("attached", "false"),
                ("facing", facing),
                ("powered", "false"),
            ],
        )
    };
    assert_eq!(
        survives_with(hook("north"), relative(AT, Direction::South), stone()),
        Some(true)
    );
    assert_eq!(
        survives_with(hook("north"), relative(AT, Direction::South), air()),
        Some(false)
    );

    let mut world = NativeWorld::new();
    world.set(AT, hook("north"));
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), hook("north"));
    world.shape_update(AT, Direction::South);
    assert_eq!(world.state(AT), air());
}

#[test]
fn frogspawn_floats_on_the_surface_and_never_under_it() {
    let spawn = default_state("minecraft:frogspawn");
    let held = |below_state: i32, own_cell: i32| {
        can_survive(spawn, AT, vanilla_tags(), &mut |pos| {
            if pos == below(AT) {
                below_state
            } else if pos == AT {
                own_cell
            } else {
                air()
            }
        })
    };
    assert_eq!(held(water(0), air()), Some(true));
    assert_eq!(held(water(0), water(0)), Some(false));
    assert_eq!(held(stone(), air()), Some(false));

    let mut world = NativeWorld::new();
    world.set(AT, spawn);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), air());
}

#[test]
fn frogspawn_hatches_into_live_tadpoles() {
    let spawn = default_state("minecraft:frogspawn");
    let mut world = NativeWorld::new();
    world.set(below(AT), water(0));
    world.set(AT, spawn);
    assert!(world
        .saved_block_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:frogspawn"
            && tick.x == AT.x
            && tick.y == AT.y
            && tick.z == AT.z
            && (crate::block_behavior::footing::MIN_HATCH_TICK_DELAY
                ..=crate::block_behavior::footing::MAX_HATCH_TICK_DELAY)
                .contains(&tick.delay)));
    world.tick_block(AT);
    assert_eq!(world.state(AT), air());
    let tadpoles: Vec<_> = world
        .entities()
        .into_iter()
        .filter(|entity| {
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TADPOLE_ID
        })
        .collect();
    assert!((2..=5).contains(&tadpoles.len()));
    for tadpole in tadpoles {
        assert_eq!(tadpole.position().y, f64::from(AT.y) - 0.5);
    }
    assert!(world.items().is_empty());
    assert!(world.sounds().iter().any(|sound| sound.pos == AT
        && sound.resource_id == "minecraft:block.frogspawn.hatch"
        && sound.volume == 1.0
        && sound.pitch == 1.0));
}

#[test]
fn soul_fire_is_one_tag_under_it() {
    let fire = default_state("minecraft:soul_fire");
    for (name, expected) in [
        ("minecraft:soul_sand", true),
        ("minecraft:soul_soil", true),
        ("minecraft:stone", false),
        ("minecraft:air", false),
    ] {
        assert_eq!(
            survives_with(fire, below(AT), default_state(name)),
            Some(expected),
            "{name}"
        );
    }

    let mut world = NativeWorld::new();
    world.set(below(AT), default_state("minecraft:soul_sand"));
    world.set(AT, fire);
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), fire);
    world.set(below(AT), stone());
    world.shape_update(AT, Direction::North);
    assert_eq!(world.state(AT), air());
}
