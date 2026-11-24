use super::native_world::NativeWorld;
use super::*;

const CAULDRON: BlockPos = BlockPos { x: 2, y: 70, z: 3 };

fn cauldron(name: &str, level: Option<i32>) -> i32 {
    match level {
        Some(level) => state_of(name, &[("level", &level.to_string())]),
        None => default_state(name),
    }
}

fn dripstone(thickness: &str, direction: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:pointed_dripstone",
        &[
            ("thickness", thickness),
            ("vertical_direction", direction),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn lava(level: i32) -> i32 {
    state_of("minecraft:lava", &[("level", &level.to_string())])
}

fn position_above(pos: BlockPos, distance: i32) -> BlockPos {
    at(pos.x, pos.y + distance, pos.z)
}

fn place_stalactite(world: &mut NativeWorld, tip_distance: i32, root_distance: i32, source: i32) {
    let tip = position_above(CAULDRON, tip_distance);
    world.set(tip, dripstone("tip", "down", false));
    for distance in 1..root_distance {
        world.set(
            position_above(tip, distance),
            dripstone("middle", "down", false),
        );
    }
    let root = position_above(tip, root_distance);
    world.set(root, default_state("minecraft:dripstone_block"));
    world.set(position_above(root, 1), source);
}

fn dispatch_tick(world: &mut NativeWorld) {
    let state = world.state(CAULDRON);
    world.with_level(|level| tick_block(level, CAULDRON, state));
}

fn assert_unchanged(world: &mut NativeWorld, expected: i32) {
    assert_eq!(world.state(CAULDRON), expected);
    assert!(world.events().is_empty());
    assert!(world.level_events().is_empty());
}

#[test]
fn tip_search_accepts_steps_one_and_ten_but_not_eleven() {
    for distance in [1, 10] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, cauldron("minecraft:cauldron", None));
        place_stalactite(&mut world, distance, 1, water(0));
        dispatch_tick(&mut world);
        assert_eq!(
            world.state(CAULDRON),
            cauldron("minecraft:water_cauldron", Some(1)),
            "distance {distance}"
        );
    }

    let mut world = NativeWorld::new();
    world.set(CAULDRON, cauldron("minecraft:cauldron", None));
    place_stalactite(&mut world, 11, 1, water(0));
    dispatch_tick(&mut world);
    assert_unchanged(&mut world, cauldron("minecraft:cauldron", None));
}

#[test]
fn tip_requires_down_tip_thickness_and_dry_state() {
    for invalid in [
        dripstone("tip", "up", false),
        dripstone("frustum", "down", false),
        dripstone("tip", "down", true),
    ] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, cauldron("minecraft:cauldron", None));
        world.set(position_above(CAULDRON, 1), invalid);
        world.set(
            position_above(CAULDRON, 2),
            default_state("minecraft:dripstone_block"),
        );
        world.set(position_above(CAULDRON, 3), water(0));
        dispatch_tick(&mut world);
        assert_unchanged(&mut world, cauldron("minecraft:cauldron", None));
    }
}

#[test]
fn the_drip_falls_through_passable_blocks_and_stops_on_solid_ones() {
    for passable in [air(), default_state("minecraft:short_grass")] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, cauldron("minecraft:cauldron", None));
        place_stalactite(&mut world, 2, 1, water(0));
        world.set(position_above(CAULDRON, 1), passable);
        dispatch_tick(&mut world);
        assert_eq!(
            world.state(CAULDRON),
            cauldron("minecraft:water_cauldron", Some(1))
        );
    }

    for obstruction in [stone(), water(0), default_state("minecraft:oak_fence")] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, cauldron("minecraft:cauldron", None));
        place_stalactite(&mut world, 2, 1, water(0));
        world.set(position_above(CAULDRON, 1), obstruction);
        dispatch_tick(&mut world);
        assert_unchanged(&mut world, cauldron("minecraft:cauldron", None));
    }
}

#[test]
fn root_search_accepts_step_ten_and_rejects_step_eleven_or_wrong_direction() {
    let mut world = NativeWorld::new();
    world.set(CAULDRON, cauldron("minecraft:cauldron", None));
    place_stalactite(&mut world, 1, 10, water(0));
    dispatch_tick(&mut world);
    assert_eq!(
        world.state(CAULDRON),
        cauldron("minecraft:water_cauldron", Some(1))
    );

    for setup in ["too_far", "wrong_direction"] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, cauldron("minecraft:cauldron", None));
        place_stalactite(&mut world, 1, 11, water(0));
        if setup == "wrong_direction" {
            world.set(
                position_above(CAULDRON, 5),
                dripstone("middle", "up", false),
            );
        }
        dispatch_tick(&mut world);
        assert_unchanged(&mut world, cauldron("minecraft:cauldron", None));
    }

    let mut broken = NativeWorld::new();
    broken.set(CAULDRON, cauldron("minecraft:cauldron", None));
    place_stalactite(&mut broken, 1, 5, water(0));
    broken.set(position_above(CAULDRON, 3), stone());
    dispatch_tick(&mut broken);
    assert_unchanged(&mut broken, cauldron("minecraft:cauldron", None));
}

#[test]
fn exact_source_fluid_identity_rejects_flowing_water_and_lava() {
    for (source, expected) in [
        (water(0), Some("minecraft:water_cauldron")),
        (lava(0), Some("minecraft:lava_cauldron")),
        (water(1), None),
        (lava(1), None),
    ] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, cauldron("minecraft:cauldron", None));
        place_stalactite(&mut world, 1, 1, source);
        dispatch_tick(&mut world);
        match expected {
            Some(name) => assert_eq!(world.state(CAULDRON), cauldron(name, None)),
            None => assert_unchanged(&mut world, cauldron("minecraft:cauldron", None)),
        }
    }
}

#[test]
fn mud_supplies_water_to_the_live_cauldron() {
    let mut overworld = NativeWorld::new();
    overworld.set(CAULDRON, cauldron("minecraft:cauldron", None));
    place_stalactite(&mut overworld, 1, 1, default_state("minecraft:mud"));
    dispatch_tick(&mut overworld);
    assert_eq!(
        overworld.state(CAULDRON),
        cauldron("minecraft:water_cauldron", Some(1))
    );
}

#[test]
fn a_drip_fills_the_cauldron_one_step_with_its_level_event() {
    for (current, source, expected, event) in [
        (
            cauldron("minecraft:cauldron", None),
            water(0),
            cauldron("minecraft:water_cauldron", Some(1)),
            1047,
        ),
        (
            cauldron("minecraft:cauldron", None),
            lava(0),
            cauldron("minecraft:lava_cauldron", None),
            1046,
        ),
        (
            cauldron("minecraft:water_cauldron", Some(1)),
            water(0),
            cauldron("minecraft:water_cauldron", Some(2)),
            1047,
        ),
    ] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, current);
        place_stalactite(&mut world, 1, 1, source);
        dispatch_tick(&mut world);
        assert_eq!(world.state(CAULDRON), expected);
        assert!(world
            .level_events()
            .iter()
            .any(|entry| entry.event_type == event && entry.pos == CAULDRON && entry.data == 0));
    }
}

#[test]
fn full_water_lava_and_powder_snow_cauldron_refuse_drips() {
    for (current, source) in [
        (cauldron("minecraft:water_cauldron", Some(3)), water(0)),
        (cauldron("minecraft:water_cauldron", Some(1)), lava(0)),
        (cauldron("minecraft:lava_cauldron", None), water(0)),
        (cauldron("minecraft:lava_cauldron", None), lava(0)),
        (
            cauldron("minecraft:powder_snow_cauldron", Some(1)),
            water(0),
        ),
    ] {
        let mut world = NativeWorld::new();
        world.set(CAULDRON, current);
        place_stalactite(&mut world, 1, 1, source);
        dispatch_tick(&mut world);
        assert_unchanged(&mut world, current);
    }
}
