use super::*;

const TRAPDOOR_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn trapdoor_state(resource_id: &str, open: bool, powered: bool, waterlogged: bool) -> i32 {
    state_of(
        resource_id,
        &[
            ("facing", "north"),
            ("half", "bottom"),
            ("open", if open { "true" } else { "false" }),
            ("powered", if powered { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn trapdoor(resource_id: &str, waterlogged: bool) -> i32 {
    trapdoor_state(resource_id, false, false, waterlogged)
}

fn expected_trapdoor_sound(name: &str, opening: bool) -> &'static str {
    let family = match name {
        "minecraft:iron_trapdoor" => "iron",
        "minecraft:copper_trapdoor"
        | "minecraft:exposed_copper_trapdoor"
        | "minecraft:weathered_copper_trapdoor"
        | "minecraft:oxidized_copper_trapdoor"
        | "minecraft:waxed_copper_trapdoor"
        | "minecraft:waxed_exposed_copper_trapdoor"
        | "minecraft:waxed_weathered_copper_trapdoor"
        | "minecraft:waxed_oxidized_copper_trapdoor" => "copper",
        "minecraft:cherry_trapdoor" => "cherry_wood",
        "minecraft:bamboo_trapdoor" => "bamboo_wood",
        "minecraft:crimson_trapdoor" | "minecraft:warped_trapdoor" => "nether_wood",
        _ => "wooden",
    };
    match (family, opening) {
        ("iron", true) => "minecraft:block.iron_trapdoor.open",
        ("iron", false) => "minecraft:block.iron_trapdoor.close",
        ("copper", true) => "minecraft:block.copper_trapdoor.open",
        ("copper", false) => "minecraft:block.copper_trapdoor.close",
        ("cherry_wood", true) => "minecraft:block.cherry_wood_trapdoor.open",
        ("cherry_wood", false) => "minecraft:block.cherry_wood_trapdoor.close",
        ("bamboo_wood", true) => "minecraft:block.bamboo_wood_trapdoor.open",
        ("bamboo_wood", false) => "minecraft:block.bamboo_wood_trapdoor.close",
        ("nether_wood", true) => "minecraft:block.nether_wood_trapdoor.open",
        ("nether_wood", false) => "minecraft:block.nether_wood_trapdoor.close",
        ("wooden", true) => "minecraft:block.wooden_trapdoor.open",
        ("wooden", false) => "minecraft:block.wooden_trapdoor.close",
        _ => unreachable!(),
    }
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(TRAPDOOR_POS, state);
    world.set(relative(TRAPDOOR_POS, direction), stone());
    world.shape_update(TRAPDOOR_POS, direction);
    world.state(TRAPDOOR_POS)
}

#[test]
fn a_waterlogged_trapdoor_rearms_water_for_horizontal_and_vertical_updates() {
    let state = trapdoor("minecraft:oak_trapdoor", true);
    for direction in [Direction::East, Direction::Up] {
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, direction), state);
        assert!(world
            .saved_fluid_ticks()
            .iter()
            .any(|tick| tick.id == "minecraft:water"
                && tick.x == TRAPDOOR_POS.x
                && tick.y == TRAPDOOR_POS.y
                && tick.z == TRAPDOOR_POS.z
                && tick.delay == 5));
    }
}

#[test]
fn a_powered_trapdoor_opens_with_its_block_set_sound() {
    for name in [
        "minecraft:oak_trapdoor",
        "minecraft:iron_trapdoor",
        "minecraft:copper_trapdoor",
    ] {
        let initial = trapdoor(name, false);
        let opened = trapdoor_state(name, true, true, false);
        let mut world = NativeWorld::new();
        world.set(TRAPDOOR_POS, initial);
        world.set(
            relative(TRAPDOOR_POS, Direction::East),
            default_state("minecraft:redstone_block"),
        );

        world.neighbor_changed(
            TRAPDOOR_POS,
            block_type_of(initial).expect("trapdoor block type"),
        );

        assert_eq!(world.state(TRAPDOOR_POS), opened, "{name}");
        assert!(world.sounds().iter().any(|sound| sound.pos == TRAPDOOR_POS
            && sound.resource_id == expected_trapdoor_sound(name, true)));
        assert!(world
            .events()
            .iter()
            .any(|event| event.pos == TRAPDOOR_POS
                && event.game_event == VanillaGameEvent::BLOCK_OPEN));
    }
}

#[test]
fn a_waterlogged_open_trapdoor_closes_and_rearms_water() {
    let initial = trapdoor_state("minecraft:oak_trapdoor", true, true, true);
    let closed = trapdoor("minecraft:oak_trapdoor", true);
    let mut world = NativeWorld::new();
    world.set(TRAPDOOR_POS, initial);

    world.neighbor_changed(TRAPDOOR_POS, block_type_of(stone()).unwrap());

    assert_eq!(world.state(TRAPDOOR_POS), closed);
    assert!(world.sounds().iter().any(|sound| sound.pos == TRAPDOOR_POS
        && sound.resource_id == expected_trapdoor_sound("minecraft:oak_trapdoor", false)));
    assert!(world.events().iter().any(
        |event| event.pos == TRAPDOOR_POS && event.game_event == VanillaGameEvent::BLOCK_CLOSE
    ));
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && tick.x == TRAPDOOR_POS.x
            && tick.y == TRAPDOOR_POS.y
            && tick.z == TRAPDOOR_POS.z
            && tick.delay == 5));
}

#[test]
fn power_change_without_open_change_is_silent() {
    let initial = trapdoor_state("minecraft:oak_trapdoor", true, false, true);
    let powered = trapdoor_state("minecraft:oak_trapdoor", true, true, true);
    let mut world = NativeWorld::new();
    world.set(TRAPDOOR_POS, initial);
    world.set(
        relative(TRAPDOOR_POS, Direction::East),
        default_state("minecraft:redstone_block"),
    );

    world.neighbor_changed(TRAPDOOR_POS, block_type_of(stone()).unwrap());

    assert_eq!(world.state(TRAPDOOR_POS), powered);
    assert!(world.sounds().is_empty());
    assert!(world.events().is_empty());
}

#[test]
fn a_hand_toggles_a_trapdoor_but_not_an_iron_one() {
    for name in ["minecraft:oak_trapdoor", "minecraft:iron_trapdoor"] {
        let initial = trapdoor(name, false);
        let mut world = NativeWorld::new();
        world.set(TRAPDOOR_POS, initial);

        let consumed = world.with_level(|level| {
            use_without_item(
                level,
                TRAPDOOR_POS,
                initial,
                interaction_player(true, Direction::North),
            )
        });

        if name == "minecraft:iron_trapdoor" {
            assert!(!consumed);
            assert_eq!(world.state(TRAPDOOR_POS), initial);
            assert!(world.sounds().is_empty());
            continue;
        }
        let opened = trapdoor_state(name, true, false, false);
        assert!(consumed, "{name}");
        assert_eq!(world.state(TRAPDOOR_POS), opened, "{name}");
        let sounds: Vec<String> = world
            .sounds()
            .into_iter()
            .map(|sound| sound.resource_id)
            .collect();
        assert_eq!(sounds, [expected_trapdoor_sound(name, true)], "{name}");
        assert!(world
            .events()
            .iter()
            .any(|event| event.pos == TRAPDOOR_POS
                && event.game_event == VanillaGameEvent::BLOCK_OPEN));
    }
}

#[test]
fn a_hand_closing_a_waterlogged_trapdoor_rearms_its_water() {
    let initial = trapdoor_state("minecraft:oak_trapdoor", true, false, true);
    let closed = trapdoor_state("minecraft:oak_trapdoor", false, false, true);
    let mut world = NativeWorld::new();
    world.set(TRAPDOOR_POS, initial);

    assert!(world.with_level(|level| use_without_item(
        level,
        TRAPDOOR_POS,
        initial,
        interaction_player(true, Direction::North)
    )));

    assert_eq!(world.state(TRAPDOOR_POS), closed);
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && tick.x == TRAPDOOR_POS.x
            && tick.y == TRAPDOOR_POS.y
            && tick.z == TRAPDOOR_POS.z
            && tick.delay == 5));
    assert!(world.events().iter().any(
        |event| event.pos == TRAPDOOR_POS && event.game_event == VanillaGameEvent::BLOCK_CLOSE
    ));
}
