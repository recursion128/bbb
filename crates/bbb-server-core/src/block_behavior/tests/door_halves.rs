//! `DoorBlock` (`DoorBlock.java`): the two survival questions a door's halves
//! ask, and the shape update that keeps the pair consistent.

use super::*;

use crate::block_behavior::door::{self};

/// The cell the lower half stands in.
const DOOR: BlockPos = BlockPos { x: 8, y: 68, z: 5 };

fn door_state(name: &str, half: &str, facing: &str, hinge: &str, open: bool, powered: bool) -> i32 {
    state_of(
        name,
        &[
            ("facing", facing),
            ("half", half),
            ("hinge", hinge),
            ("open", if open { "true" } else { "false" }),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn lower(name: &str) -> i32 {
    door_state(name, "lower", "north", "left", false, false)
}

fn upper(name: &str) -> i32 {
    door_state(name, "upper", "north", "left", false, false)
}

/// Whether `door_state` would stand with `below_state` under it.
fn survives_over(door_state_id: i32, below_state: i32) -> Option<bool> {
    door::can_survive(door_state_id, DOOR, &mut |at| {
        if at == below(DOOR) {
            below_state
        } else {
            air()
        }
    })
}

#[test]
fn the_lower_half_needs_a_full_sturdy_face_under_it() {
    for below_name in ["minecraft:stone", "minecraft:dirt", "minecraft:glass"] {
        assert_eq!(
            survives_over(lower("minecraft:oak_door"), default_state(below_name)),
            Some(true),
            "{below_name}"
        );
    }
    // A fence is centre-sturdy but not full, which is exactly the distinction
    // between a door and a candle.
    for below_name in [
        "minecraft:oak_fence",
        "minecraft:oak_slab",
        "minecraft:farmland",
        "minecraft:air",
        "minecraft:torch",
    ] {
        assert_eq!(
            survives_over(lower("minecraft:oak_door"), default_state(below_name)),
            Some(false),
            "{below_name}"
        );
    }
}

#[test]
fn the_upper_half_needs_the_same_door_under_it() {
    assert_eq!(
        survives_over(upper("minecraft:oak_door"), lower("minecraft:oak_door")),
        Some(true)
    );
    // Any state of the same block will do; the property values are not read.
    assert_eq!(
        survives_over(
            upper("minecraft:oak_door"),
            door_state("minecraft:oak_door", "lower", "east", "right", true, true)
        ),
        Some(true)
    );
    assert_eq!(
        survives_over(upper("minecraft:oak_door"), lower("minecraft:iron_door")),
        Some(false)
    );
    assert_eq!(
        survives_over(upper("minecraft:oak_door"), stone()),
        Some(false)
    );
}

// ---------------------------------------------------------------------------
// `updateShape`
// ---------------------------------------------------------------------------

#[test]
fn a_lower_half_becomes_air_when_its_floor_stops_supporting_it() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), stone());
    world.set(at(0, 70, 0), lower("minecraft:oak_door"));
    world.set(at(0, 71, 0), upper("minecraft:oak_door"));

    world.set(at(0, 69, 0), air());
    world.shape_update(at(0, 70, 0), Direction::Down);

    assert_eq!(world.state(at(0, 70, 0)), air());
    // And the upper half follows, because losing the lower half is a change to
    // its own other-half neighbour.
    assert_eq!(world.state(at(0, 71, 0)), air());
}

#[test]
fn a_lower_half_that_lost_its_floor_survives_an_update_from_the_side() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), lower("minecraft:oak_door"));
    world.set(at(0, 71, 0), upper("minecraft:oak_door"));

    world.shape_update(at(0, 70, 0), Direction::North);

    assert_eq!(world.state(at(0, 70, 0)), lower("minecraft:oak_door"));
}

#[test]
fn an_upper_half_never_checks_the_floor_on_a_shape_update() {
    let mut world = NativeWorld::new();
    world.set(at(0, 71, 0), upper("minecraft:oak_door"));
    world.set(at(0, 70, 0), lower("minecraft:oak_door"));

    world.shape_update(at(0, 71, 0), Direction::North);

    assert_eq!(world.state(at(0, 71, 0)), upper("minecraft:oak_door"));
}

#[test]
fn a_half_whose_partner_is_gone_becomes_air() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), stone());
    world.set(at(0, 70, 0), lower("minecraft:oak_door"));
    world.set(at(0, 71, 0), stone());

    world.shape_update(at(0, 70, 0), Direction::Up);

    assert_eq!(world.state(at(0, 70, 0)), air());
}

#[test]
fn a_half_adopts_its_partners_state_and_keeps_only_its_own_half() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), stone());
    world.set(at(0, 70, 0), lower("minecraft:oak_door"));
    world.set(
        at(0, 71, 0),
        door_state("minecraft:oak_door", "upper", "east", "right", true, true),
    );

    world.shape_update(at(0, 70, 0), Direction::Up);

    assert_eq!(
        world.state(at(0, 70, 0)),
        door_state("minecraft:oak_door", "lower", "east", "right", true, true)
    );
}

#[test]
fn the_class_test_on_the_neighbour_is_the_block_class_not_the_block() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), stone());
    world.set(at(0, 70, 0), lower("minecraft:oak_door"));
    world.set(at(0, 71, 0), upper("minecraft:iron_door"));

    world.shape_update(at(0, 70, 0), Direction::Up);

    assert_eq!(world.state(at(0, 70, 0)), lower("minecraft:iron_door"));
}

#[test]
fn a_half_whose_partner_carries_the_same_half_becomes_air() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), stone());
    world.set(at(0, 70, 0), lower("minecraft:oak_door"));
    world.set(at(0, 71, 0), lower("minecraft:oak_door"));

    world.shape_update(at(0, 70, 0), Direction::Up);

    assert_eq!(world.state(at(0, 70, 0)), air());
}

// ---------------------------------------------------------------------------
// `neighborChanged`
// ---------------------------------------------------------------------------

fn expected_door_sound(name: &str, opening: bool) -> &'static str {
    let family = match name {
        "minecraft:iron_door" => "iron",
        "minecraft:copper_door"
        | "minecraft:exposed_copper_door"
        | "minecraft:weathered_copper_door"
        | "minecraft:oxidized_copper_door"
        | "minecraft:waxed_copper_door"
        | "minecraft:waxed_exposed_copper_door"
        | "minecraft:waxed_weathered_copper_door"
        | "minecraft:waxed_oxidized_copper_door" => "copper",
        "minecraft:cherry_door" => "cherry_wood",
        "minecraft:bamboo_door" => "bamboo_wood",
        "minecraft:crimson_door" | "minecraft:warped_door" => "nether_wood",
        _ => "wooden",
    };
    match (family, opening) {
        ("iron", true) => "minecraft:block.iron_door.open",
        ("iron", false) => "minecraft:block.iron_door.close",
        ("copper", true) => "minecraft:block.copper_door.open",
        ("copper", false) => "minecraft:block.copper_door.close",
        ("cherry_wood", true) => "minecraft:block.cherry_wood_door.open",
        ("cherry_wood", false) => "minecraft:block.cherry_wood_door.close",
        ("bamboo_wood", true) => "minecraft:block.bamboo_wood_door.open",
        ("bamboo_wood", false) => "minecraft:block.bamboo_wood_door.close",
        ("nether_wood", true) => "minecraft:block.nether_wood_door.open",
        ("nether_wood", false) => "minecraft:block.nether_wood_door.close",
        ("wooden", true) => "minecraft:block.wooden_door.open",
        ("wooden", false) => "minecraft:block.wooden_door.close",
        _ => unreachable!(),
    }
}

fn stone_block_type() -> i32 {
    block_type_of(stone()).expect("stone block type")
}

#[test]
fn a_powered_door_opens_with_its_block_set_sound_and_game_event() {
    for name in [
        "minecraft:oak_door",
        "minecraft:iron_door",
        "minecraft:copper_door",
    ] {
        let initial = door_state(name, "lower", "north", "left", false, false);
        let opened = door_state(name, "lower", "north", "left", true, true);
        let mut world = NativeWorld::new();
        world.set(DOOR, initial);
        world.set(
            relative(DOOR, Direction::East),
            default_state("minecraft:redstone_block"),
        );

        world.neighbor_changed(DOOR, stone_block_type());

        assert_eq!(world.state(DOOR), opened, "{name}");
        assert!(
            world
                .sounds()
                .iter()
                .any(|sound| sound.pos == DOOR
                    && sound.resource_id == expected_door_sound(name, true))
        );
        assert!(world
            .events()
            .iter()
            .any(|event| event.pos == DOOR && event.game_event == VanillaGameEvent::BLOCK_OPEN));
    }
}

#[test]
fn a_powered_open_door_closes_with_sound_and_game_event() {
    let initial = door_state("minecraft:oak_door", "lower", "north", "left", true, true);
    let closed = lower("minecraft:oak_door");
    let mut world = NativeWorld::new();
    world.set(DOOR, initial);

    world.neighbor_changed(DOOR, stone_block_type());

    assert_eq!(world.state(DOOR), closed);
    assert!(world.sounds().iter().any(|sound| sound.pos == DOOR
        && sound.resource_id == expected_door_sound("minecraft:oak_door", false)));
    assert!(world
        .events()
        .iter()
        .any(|event| event.pos == DOOR && event.game_event == VanillaGameEvent::BLOCK_CLOSE));
}

#[test]
fn a_signal_at_the_other_half_powers_the_door() {
    let initial = lower("minecraft:oak_door");
    let opened = door_state("minecraft:oak_door", "lower", "north", "left", true, true);
    let other_half = relative(DOOR, Direction::Up);
    let mut world = NativeWorld::new();
    world.set(DOOR, initial);
    world.set(
        relative(other_half, Direction::East),
        default_state("minecraft:redstone_block"),
    );

    world.neighbor_changed(DOOR, stone_block_type());

    assert_eq!(world.state(DOOR), opened);
}

#[test]
fn power_changes_without_an_open_change_are_silent() {
    let initial = door_state("minecraft:oak_door", "lower", "north", "left", true, false);
    let updated = door_state("minecraft:oak_door", "lower", "north", "left", true, true);
    let mut world = NativeWorld::new();
    world.set(DOOR, initial);
    world.set(
        relative(DOOR, Direction::East),
        default_state("minecraft:redstone_block"),
    );

    world.neighbor_changed(DOOR, stone_block_type());

    assert_eq!(world.state(DOOR), updated);
    assert!(world.sounds().is_empty());
    assert!(world.events().is_empty());
}

#[test]
fn a_hand_opens_a_door_but_not_an_iron_one() {
    for name in ["minecraft:oak_door", "minecraft:iron_door"] {
        let initial = door_state(name, "lower", "north", "left", false, false);
        let mut world = NativeWorld::new();
        world.set(DOOR, initial);

        let consumed = world.with_level(|level| {
            use_without_item(
                level,
                DOOR,
                initial,
                interaction_player(true, Direction::North),
            )
        });

        if name == "minecraft:iron_door" {
            assert!(!consumed);
            assert_eq!(world.state(DOOR), initial);
            assert!(world.sounds().is_empty());
            continue;
        }
        let opened = door_state(name, "lower", "north", "left", true, false);
        assert!(consumed, "{name}");
        assert_eq!(world.state(DOOR), opened, "{name}");
        let sounds: Vec<String> = world
            .sounds()
            .into_iter()
            .map(|sound| sound.resource_id)
            .collect();
        assert_eq!(sounds, [expected_door_sound(name, true)], "{name}");
        assert!(world
            .events()
            .iter()
            .any(|event| event.pos == DOOR && event.game_event == VanillaGameEvent::BLOCK_OPEN));
    }
}

#[test]
fn a_hand_closes_an_open_door_and_leaves_powered_alone() {
    let initial = door_state("minecraft:oak_door", "lower", "north", "left", true, true);
    let closed = door_state("minecraft:oak_door", "lower", "north", "left", false, true);
    let mut world = NativeWorld::new();
    world.set(DOOR, initial);

    assert!(world.with_level(|level| use_without_item(
        level,
        DOOR,
        initial,
        interaction_player(true, Direction::North)
    )));

    assert_eq!(world.state(DOOR), closed);
    let sounds: Vec<String> = world
        .sounds()
        .into_iter()
        .map(|sound| sound.resource_id)
        .collect();
    assert_eq!(sounds, ["minecraft:block.wooden_door.close"]);
    assert!(world
        .events()
        .iter()
        .any(|event| event.pos == DOOR && event.game_event == VanillaGameEvent::BLOCK_CLOSE));
}
