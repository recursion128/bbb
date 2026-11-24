use super::*;
use crate::entity::{ArmorStandData, EntityData, MinecartData};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::save::Java26BuiltinDimension;

const POS: BlockPos = BlockPos { x: 0, y: 70, z: 0 };
const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn plate(name: &str, power: i32) -> i32 {
    let state = default_state(name);
    with_property(state, "power", &power.to_string())
        .or_else(|| with_property(state, "powered", if power > 0 { "true" } else { "false" }))
        .unwrap()
}

fn detector(powered: bool) -> i32 {
    state_of(
        "minecraft:detector_rail",
        &[
            ("powered", if powered { "true" } else { "false" }),
            ("shape", "north_south"),
            ("waterlogged", "false"),
        ],
    )
}

fn tripwire(attached: bool, powered: bool) -> i32 {
    state_of(
        "minecraft:tripwire",
        &[
            ("north", "false"),
            ("east", "true"),
            ("south", "false"),
            ("west", "true"),
            ("attached", if attached { "true" } else { "false" }),
            ("disarmed", "false"),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn hook(facing: &str, powered: bool) -> i32 {
    state_of(
        "minecraft:tripwire_hook",
        &[
            ("facing", facing),
            ("attached", "true"),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn spawn(world: &mut NativeWorld, minecart: bool, x: f64) -> InsideEntity {
    let (kind, body) = if minecart {
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            EntityData::Minecart(MinecartData::new()),
        )
    } else {
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            EntityData::ArmorStand(ArmorStandData::new().unwrap()),
        )
    };
    let (id, _) = world.world.spawn_stored_entity_for_test(
        DIMENSION,
        kind,
        Vec3d {
            x,
            y: f64::from(POS.y),
            z: 0.5,
        },
        body,
    );
    let entity = world.world.stored_entity_for_test(DIMENSION, id).unwrap();
    InsideEntity {
        source: Some(id),
        bounds: entity.bounding_box(),
        minecart: crate::entity::minecart::is_minecart(entity.data()),
        living: entity.living().is_some(),
        ignores_block_triggers: entity.is_ignoring_block_triggers(),
    }
}

fn scene(state: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    world.set(below(POS), stone());
    if block_type_of(state) == Some(bbb_protocol::block_types::VANILLA_BLOCK_DETECTOR_RAIL_ID) {
        world.set(POS, detector(false));
        // Restore a legal saved powered rail before its occupancy recheck.
        world
            .world
            .set_raw_block_state_for_test(DIMENSION, POS, state);
    } else {
        world.set(POS, state);
    }
    world
}

#[test]
fn pressure_plate_rechecks_update_power_and_click_off() {
    let old = plate("minecraft:heavy_weighted_pressure_plate", 15);
    let mut weighted = scene(old);
    for _ in 0..11 {
        spawn(&mut weighted, false, 0.5);
    }
    weighted.with_level(|level| tick_block(level, POS, old));
    assert_eq!(int_property(weighted.state(POS), "power"), Some(2));
    assert!(weighted.sounds().is_empty());

    let old = plate("minecraft:stone_pressure_plate", 15);
    let mut empty = scene(old);
    empty.with_level(|level| tick_block(level, POS, old));
    assert_eq!(bool_property(empty.state(POS), "powered"), Some(false));
    assert!(empty
        .sounds()
        .iter()
        .any(|sound| sound.resource_id == "minecraft:block.stone_pressure_plate.click_off"));
    assert!(empty
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_DEACTIVATE && event.pos == POS));
}

#[test]
fn detector_rail_rechecks_release_or_hold_the_press() {
    for occupied in [false, true] {
        let mut world = scene(detector(true));
        if occupied {
            spawn(&mut world, true, 0.5);
        }
        world.with_level(|level| tick_block(level, POS, detector(true)));
        assert_eq!(bool_property(world.state(POS), "powered"), Some(occupied));
    }
}

#[test]
fn a_detector_rails_analog_output_prefers_a_command_block_over_a_container() {
    for (powered, command, container, expected) in [
        (false, true, true, 0),
        (true, false, false, 0),
        (true, true, false, 7),
        (true, false, true, 4),
        (true, true, true, 7),
    ] {
        let mut world = scene(detector(powered));
        let position = Vec3d {
            x: 0.5,
            y: 70.0,
            z: 0.5,
        };
        if container {
            let mut body = crate::entity::MinecartContainerData::new_chest();
            for slot in 0..6 {
                body.set_item(
                    slot,
                    Some(crate::tests::native_stack(
                        bbb_protocol::packets::ItemStackSummary {
                            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                            count: 64,
                            component_patch: Default::default(),
                        },
                    )),
                )
                .unwrap();
            }
            world.world.spawn_stored_entity_for_test(
                DIMENSION,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
                position,
                EntityData::ChestMinecart(body),
            );
        }
        if command {
            let mut body = crate::entity::CommandBlockMinecartData::new();
            body.set_success_count(7);
            world.world.spawn_stored_entity_for_test(
                DIMENSION,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
                position,
                EntityData::CommandBlockMinecart(body),
            );
        }
        assert_eq!(
            world.with_level(|level| {
                level.analog_output_signal(POS, bbb_protocol::packets::Direction::North)
            }),
            expected
        );
    }
}

#[test]
fn tripwire_rechecks_its_live_attached_shape_then_updates_the_hook_line() {
    for occupied in [false, true] {
        let mut world = scene(tripwire(true, true));
        world.set(BlockPos { x: -3, ..POS }, stone());
        world.set(BlockPos { x: 3, ..POS }, stone());
        world.set(BlockPos { x: -2, ..POS }, hook("east", true));
        world.set(BlockPos { x: -1, ..POS }, tripwire(true, false));
        world.set(BlockPos { x: 1, ..POS }, tripwire(true, false));
        world.set(BlockPos { x: 2, ..POS }, hook("west", true));
        if occupied {
            spawn(&mut world, false, 0.5);
        }
        world.with_level(|level| tick_block(level, POS, tripwire(true, true)));
        assert_eq!(bool_property(world.state(POS), "powered"), Some(occupied));
        if !occupied {
            for x in [-2, 2] {
                assert_eq!(
                    bool_property(world.state(BlockPos { x, ..POS }), "powered"),
                    Some(false)
                );
            }
        }
    }
}

#[test]
fn a_placed_detector_rail_presses_against_the_minecarts_already_on_it() {
    for (before, occupied) in [(false, true), (false, false), (true, false)] {
        let mut world = scene(detector(before));
        if occupied {
            spawn(&mut world, true, 0.5);
        }
        world.with_level(|level| on_place(level, POS, detector(before), air(), false));
        assert_eq!(bool_property(world.state(POS), "powered"), Some(occupied));
    }
}

#[test]
fn a_detector_rail_presses_the_moment_a_minecart_is_inside_it() {
    for (minecart, x, occupied) in [
        (true, 0.5, false),
        (true, 2.0, false),
        (false, 0.5, true),
        (false, 0.5, false),
    ] {
        let mut world = scene(detector(false));
        if occupied {
            spawn(&mut world, true, 0.5);
        }
        let inside = spawn(&mut world, minecart, x);
        assert!(world.with_level(|level| entity_inside(level, POS, detector(false), inside)));
        assert_eq!(
            bool_property(world.state(POS), "powered"),
            Some((minecart && x == 0.5) || occupied)
        );
    }
}

#[test]
fn pressure_plate_contact_uses_actual_entity_kind_and_weighted_occupancy() {
    for (name, minecart, count, expected) in [
        ("minecraft:oak_pressure_plate", true, 1, 15),
        ("minecraft:stone_pressure_plate", true, 1, 0),
        ("minecraft:stone_pressure_plate", false, 1, 15),
        ("minecraft:stone_pressure_plate", true, 2, 15),
        ("minecraft:heavy_weighted_pressure_plate", false, 11, 2),
    ] {
        let state = plate(name, 0);
        let mut world = scene(state);
        let inside = spawn(&mut world, minecart, 0.5);
        for _ in 1..count {
            spawn(
                &mut world,
                if name == "minecraft:stone_pressure_plate" {
                    false
                } else {
                    minecart
                },
                0.5,
            );
        }
        assert!(world.with_level(|level| entity_inside(level, POS, state, inside)));
        let output = int_property(world.state(POS), "power").unwrap_or_else(|| {
            if bool_property(world.state(POS), "powered") == Some(true) {
                15
            } else {
                0
            }
        });
        assert_eq!(output, expected, "{name} minecart={minecart}");
        if expected > 0 && name == "minecraft:oak_pressure_plate" {
            assert!(
                world
                    .sounds()
                    .iter()
                    .any(|sound| sound.resource_id
                        == "minecraft:block.wooden_pressure_plate.click_on")
            );
            assert!(world
                .events()
                .iter()
                .any(|event| event.pos == POS
                    && event.game_event == VanillaGameEvent::BLOCK_ACTIVATE));
        }
    }
}

#[test]
fn a_pressed_plate_ignores_further_visitors() {
    let state = plate("minecraft:oak_pressure_plate", 15);
    let mut world = scene(state);
    let inside = spawn(&mut world, false, 0.5);
    assert!(world.with_level(|level| entity_inside(level, POS, state, inside)));
    assert_eq!(world.state(POS), state);
    assert!(world.sounds().is_empty());
}
