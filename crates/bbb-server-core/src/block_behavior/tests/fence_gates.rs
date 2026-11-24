use bbb_server_types::directions::{opposite, relative};

use super::*;

const GATE_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn gate_state(resource_id: &str, facing: &str, in_wall: bool, open: bool, powered: bool) -> i32 {
    state_of(
        resource_id,
        &[
            ("facing", facing),
            ("in_wall", if in_wall { "true" } else { "false" }),
            ("open", if open { "true" } else { "false" }),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn gate(resource_id: &str, facing: &str, in_wall: bool) -> i32 {
    gate_state(resource_id, facing, in_wall, false, false)
}

fn expected_gate_sound(name: &str, opening: bool) -> &'static str {
    let family = match name {
        "minecraft:cherry_fence_gate" => "cherry_wood",
        "minecraft:bamboo_fence_gate" => "bamboo_wood",
        "minecraft:crimson_fence_gate" | "minecraft:warped_fence_gate" => "nether_wood",
        _ => "default",
    };
    match (family, opening) {
        ("cherry_wood", true) => "minecraft:block.cherry_wood_fence_gate.open",
        ("cherry_wood", false) => "minecraft:block.cherry_wood_fence_gate.close",
        ("bamboo_wood", true) => "minecraft:block.bamboo_wood_fence_gate.open",
        ("bamboo_wood", false) => "minecraft:block.bamboo_wood_fence_gate.close",
        ("nether_wood", true) => "minecraft:block.nether_wood_fence_gate.open",
        ("nether_wood", false) => "minecraft:block.nether_wood_fence_gate.close",
        ("default", true) => "minecraft:block.fence_gate.open",
        ("default", false) => "minecraft:block.fence_gate.close",
        _ => unreachable!(),
    }
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction, neighbour_state: i32) -> i32 {
    world.set(GATE_POS, state);
    world.set(relative(GATE_POS, direction), neighbour_state);
    world.shape_update(GATE_POS, direction);
    world.state(GATE_POS)
}

#[test]
fn a_wall_on_either_lateral_side_sets_and_retains_in_wall() {
    let wall = default_state("minecraft:cobblestone_wall");
    for (facing, direction) in [("north", Direction::East), ("east", Direction::North)] {
        let mut changed_side = NativeWorld::new();
        assert_eq!(
            update(
                &mut changed_side,
                gate("minecraft:oak_fence_gate", facing, false),
                direction,
                wall,
            ),
            gate("minecraft:oak_fence_gate", facing, true)
        );

        let mut opposite_side = NativeWorld::new();
        opposite_side.set(relative(GATE_POS, opposite(direction)), wall);
        assert_eq!(
            update(
                &mut opposite_side,
                gate("minecraft:oak_fence_gate", facing, true),
                direction,
                air(),
            ),
            gate("minecraft:oak_fence_gate", facing, true)
        );
    }
}

#[test]
fn removing_both_lateral_walls_clears_in_wall() {
    for (facing, direction) in [("south", Direction::West), ("west", Direction::South)] {
        assert_eq!(
            update(
                &mut NativeWorld::new(),
                gate("minecraft:warped_fence_gate", facing, true),
                direction,
                stone(),
            ),
            gate("minecraft:warped_fence_gate", facing, false)
        );
    }
}

#[test]
fn facing_axis_and_vertical_updates_are_unchanged() {
    let gate = gate("minecraft:bamboo_fence_gate", "north", true);
    for direction in [
        Direction::North,
        Direction::South,
        Direction::Up,
        Direction::Down,
    ] {
        assert_eq!(
            update(&mut NativeWorld::new(), gate, direction, air()),
            gate
        );
    }
}

#[test]
fn a_powered_gate_opens_and_closes_with_its_wood_type_sound() {
    for name in ["minecraft:oak_fence_gate", "minecraft:cherry_fence_gate"] {
        let closed = gate(name, "north", false);
        let opened = gate_state(name, "north", false, true, true);
        let gate_type = block_type_of(closed).expect("fence gate block type");

        let mut opening = NativeWorld::new();
        opening.set(GATE_POS, closed);
        opening.set(
            relative(GATE_POS, Direction::East),
            default_state("minecraft:redstone_block"),
        );
        opening.neighbor_changed(GATE_POS, gate_type);

        assert_eq!(opening.state(GATE_POS), opened, "{name}");
        assert!(opening
            .sounds()
            .iter()
            .any(|sound| sound.pos == GATE_POS
                && sound.resource_id == expected_gate_sound(name, true)));
        assert!(
            opening
                .events()
                .iter()
                .any(|event| event.game_event == VanillaGameEvent::BLOCK_OPEN
                    && event.pos == GATE_POS)
        );

        let mut closing = NativeWorld::new();
        closing.set(GATE_POS, opened);
        closing.neighbor_changed(GATE_POS, gate_type);

        assert_eq!(closing.state(GATE_POS), closed, "{name}");
        assert!(closing
            .sounds()
            .iter()
            .any(|sound| sound.pos == GATE_POS
                && sound.resource_id == expected_gate_sound(name, false)));
        assert!(closing.events().iter().any(|event| event.game_event
            == VanillaGameEvent::BLOCK_CLOSE
            && event.pos == GATE_POS));
    }
}

#[test]
fn a_power_change_without_an_open_change_is_silent() {
    let initial = gate_state("minecraft:oak_fence_gate", "north", false, true, false);
    let powered = gate_state("minecraft:oak_fence_gate", "north", false, true, true);
    let mut world = NativeWorld::new();
    world.set(GATE_POS, initial);
    world.set(
        relative(GATE_POS, Direction::East),
        default_state("minecraft:redstone_block"),
    );

    world.neighbor_changed(GATE_POS, block_type_of(stone()).unwrap());

    assert_eq!(world.state(GATE_POS), powered);
    assert!(world.sounds().is_empty());
    assert!(world.events().is_empty());
}

#[test]
fn a_hand_swings_a_closed_gate_facing_away_around_to_meet_the_player() {
    let initial = gate("minecraft:oak_fence_gate", "south", false);
    let opened = gate_state("minecraft:oak_fence_gate", "north", false, true, false);
    let mut world = NativeWorld::new();
    world.set(GATE_POS, initial);

    assert!(world.with_level(|level| use_without_item(
        level,
        GATE_POS,
        initial,
        interaction_player(true, Direction::North)
    )));

    assert_eq!(world.state(GATE_POS), opened);

    assert!(world.sounds().iter().any(
        |sound| sound.pos == GATE_POS && sound.resource_id == "minecraft:block.fence_gate.open"
    ));
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_OPEN && event.pos == GATE_POS));
}

#[test]
fn a_hand_opens_a_gate_already_facing_the_player_without_turning_it() {
    let initial = gate("minecraft:oak_fence_gate", "north", false);
    let opened = gate_state("minecraft:oak_fence_gate", "north", false, true, false);
    let mut world = NativeWorld::new();
    world.set(GATE_POS, initial);

    assert!(world.with_level(|level| use_without_item(
        level,
        GATE_POS,
        initial,
        interaction_player(true, Direction::East)
    )));

    assert_eq!(world.state(GATE_POS), opened);
}

#[test]
fn a_hand_closes_an_open_gate_keeping_its_facing() {
    let initial = gate_state("minecraft:warped_fence_gate", "east", false, true, false);
    let closed = gate("minecraft:warped_fence_gate", "east", false);
    let mut world = NativeWorld::new();
    world.set(GATE_POS, initial);

    assert!(world.with_level(|level| use_without_item(
        level,
        GATE_POS,
        initial,
        interaction_player(true, Direction::West)
    )));

    assert_eq!(world.state(GATE_POS), closed);

    assert!(world.sounds().iter().any(|sound| sound.pos == GATE_POS
        && sound.resource_id == "minecraft:block.nether_wood_fence_gate.close"));
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_CLOSE && event.pos == GATE_POS));
}
