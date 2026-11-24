//! `SculkSensorBlockEntity`'s runtime through a real level tick: the vibration
//! a delivered game event schedules, the travel clock, the `activate` at the
//! arrival, the deactivate tick, and the chain a sensor-produced
//! `sculk_sensor_tendrils_clicking` closes into a shrieker.

use super::*;

use crate::block_entity::vibration::VibrationData;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, PendingExplosion,
};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;

use super::helpers::{sensor_state, shrieker_state};

const SENSOR: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const SHRIEKER: BlockPos = BlockPos { x: 8, y: 64, z: 0 };
const SOURCE: BlockPos = BlockPos { x: 4, y: 64, z: 0 };
const TENDRILS: &str = "minecraft:sculk_sensor_tendrils_clicking";

/// A survival session beside a sensor, with the listener state the join and
/// the placement left behind cleared so a scenario starts from no vibration.
fn sensor_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = helpers::joined_stand_world(
        datapack_config(),
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    helpers::place(&mut world, SENSOR, sensor_state("inactive", 0, false));
    // The join and the placement leave their own events in the sink; one tick
    // flushes them before a scenario records its own.
    world.tick(receiver.drain());
    clear_listener(&mut world, SENSOR);
    (sender, receiver, world)
}

fn clear_listener(world: &mut ServerWorld, pos: BlockPos) {
    let sensor = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|entity| entity.sculk_sensor_mut())
        .expect("the sensor's block entity");
    *sensor.vibration_data_mut_for_test() = VibrationData::default();
}

fn sensor_frequency(world: &mut ServerWorld) -> i32 {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, SENSOR)
        .and_then(|entity| entity.sculk_sensor())
        .expect("the sensor's block entity")
        .last_vibration_frequency()
}

fn sensor_vibration(world: &mut ServerWorld) -> VibrationData {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, SENSOR)
        .and_then(|entity| entity.sculk_sensor())
        .expect("the sensor's block entity")
        .vibration_data()
        .clone()
}

#[test]
fn an_explosion_vibration_uses_the_exact_center_for_delivery_and_travel() {
    let (_sender, receiver, mut world) = sensor_world();
    let center = Vec3d {
        x: 8.1,
        y: 64.5,
        z: 0.5,
    };
    world.explode(
        Java26BuiltinDimension::Overworld,
        PendingExplosion {
            source: None,
            center,
            radius: 0.1,
            fire: false,
            interaction: ExplosionInteraction::None,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut Vec::new(),
    );
    let event = world
        .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|event| event.recorded.game_event == VanillaGameEvent::EXPLODE)
        .expect("the explosion raised its event");
    assert_eq!(event.position, center);
    assert_eq!(event.recorded.pos, BlockPos { x: 8, y: 64, z: 0 });
    let data = sensor_vibration(&mut world);
    let (candidate, _) = data
        .selection_strategy()
        .current()
        .expect("the explosion scheduled a vibration");
    assert_eq!(candidate.pos, center);
    assert!((candidate.distance - 7.6).abs() < 0.01);
    world.tick(receiver.drain());
    assert_eq!(sensor_vibration(&mut world).travel_time_in_ticks(), 6);
}

#[test]
fn a_delivered_vibration_activates_the_sensor() {
    for (waterlogged, wall) in [
        (false, None),
        (true, None),
        (false, Some("minecraft:stone")),
        (true, Some("minecraft:stone")),
        (false, Some("minecraft:white_wool")),
        (true, Some("minecraft:white_wool")),
    ] {
        let (_sender, receiver, mut world) = sensor_world();
        if waterlogged {
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                SENSOR,
                sensor_state("inactive", 0, true),
                SET_BLOCK_UPDATE_ALL,
            ));
            world.tick(receiver.drain());
        }
        if let Some(wall) = wall {
            let state = bbb_protocol::block_types::vanilla_block_type_for_resource_id(wall)
                .and_then(|block| vanilla_block_default_state_id(block.id))
                .unwrap();
            for y in SENSOR.y - 1..=SENSOR.y + 1 {
                for z in SENSOR.z - 1..=SENSOR.z + 1 {
                    world.set_block(
                        Java26BuiltinDimension::Overworld,
                        BlockPos {
                            x: SENSOR.x + 2,
                            y,
                            z,
                        },
                        state,
                        SET_BLOCK_UPDATE_ALL,
                    );
                }
            }
            world.tick(receiver.drain());
            clear_listener(&mut world, SENSOR);
        }
        let player = world.player_entity_id_for_test(ConnectionId(1));
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            VanillaGameEvent::BLOCK_CHANGE,
            SOURCE,
            Some(player),
            None,
        );
        world.tick(receiver.drain());

        if wall == Some("minecraft:white_wool") {
            for _ in 0..8 {
                world.tick(receiver.drain());
            }
            assert_eq!(
                world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
                Some(sensor_state("inactive", 0, waterlogged))
            );
            assert_eq!(sensor_frequency(&mut world), 0);
            continue;
        }
        let (candidate, _) = sensor_vibration(&mut world)
            .selection_strategy()
            .current()
            .expect("the event scheduled a candidate");
        assert_eq!(candidate.game_event, "minecraft:block_change");
        assert_eq!(candidate.distance, 4.0);
        assert_eq!(candidate.source, Some(Uuid::from_u128(1)));

        let mut clicking = false;
        for _ in 0..6 {
            let tick = world.tick(receiver.drain());
            clicking |= tick.events.iter().any(|event| {
            matches!(event, WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(sound), ..
            } if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                if *registry_id == vanilla_sound_event_registry_id("minecraft:block.sculk_sensor.clicking").expect("sensor clicking sound exists")))
        });
            if world.block_state(Java26BuiltinDimension::Overworld, SENSOR)
                == Some(sensor_state("active", 8, waterlogged))
            {
                break;
            }
        }
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
            Some(sensor_state("active", 8, waterlogged)),
            "a four-block arrival carries power 8"
        );
        assert_eq!(clicking, !waterlogged);
        assert_eq!(sensor_frequency(&mut world), 11);
        let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
        assert!(
            game_events.iter().any(|event| {
                event.game_event.resource_id() == TENDRILS
                    && event.pos == SENSOR
                    && event.source_entity_id == Some(player)
            }),
            "activate raised the tendrils event with the source entity: {game_events:?}"
        );

        // later: ACTIVE becomes COOLDOWN with power 0, and ten ticks after that the
        // COOLDOWN becomes INACTIVE.
        for _ in 0..29 {
            world.tick(receiver.drain());
        }
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
            Some(sensor_state("active", 8, waterlogged)),
            "the deactivate tick is thirty ticks out, not sooner"
        );
        world.tick(receiver.drain());
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
            Some(sensor_state("cooldown", 0, waterlogged))
        );
        for _ in 0..10 {
            world.tick(receiver.drain());
        }
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
            Some(sensor_state("inactive", 0, waterlogged))
        );
    }
}

#[test]
fn the_sensor_refuses_its_own_placement_and_a_frequency_zero_event() {
    let (_sender, receiver, mut world) = sensor_world();
    let player = world.player_entity_id_for_test(ConnectionId(1));

    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_PLACE,
        SENSOR,
        Some(player),
        None,
    );
    world.tick(receiver.drain());
    assert!(
        sensor_vibration(&mut world)
            .selection_strategy()
            .current()
            .is_none(),
        "the sensor's own placement is refused"
    );

    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::ITEM_INTERACT_START,
        SOURCE,
        Some(player),
        None,
    );
    world.tick(receiver.drain());
    assert!(
        sensor_vibration(&mut world)
            .selection_strategy()
            .current()
            .is_none(),
        "frequency zero is refused"
    );

    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_CHANGE,
        SOURCE,
        Some(player),
        None,
    );
    world.tick(receiver.drain());
    assert_eq!(
        sensor_vibration(&mut world)
            .selection_strategy()
            .current()
            .expect("the block change schedules")
            .0
            .distance,
        4.0
    );

    for refusal in ["spectator", "sneaking", "affected_wool"] {
        let (sender, receiver, mut world) = sensor_world();
        if refusal == "spectator" {
            world.set_player_mode_and_abilities_for_test(
                ConnectionId(1),
                GameType::Spectator,
                PlayerAbilities::for_game_type(GameType::Spectator),
            );
        } else if refusal == "sneaking" {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::PlayerInput(bbb_protocol::packets::PlayerInput {
                    shift: true,
                    ..Default::default()
                }),
            );
        }
        clear_listener(&mut world, SENSOR);
        let player = world.player_entity_id_for_test(ConnectionId(1));
        let wool =
            vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_WHITE_WOOL_ID)
                .unwrap();
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            if refusal == "sneaking" {
                VanillaGameEvent::STEP
            } else {
                VanillaGameEvent::BLOCK_CHANGE
            },
            SOURCE,
            Some(player),
            (refusal == "affected_wool").then_some(wool),
        );
        for _ in 0..8 {
            world.tick(receiver.drain());
        }
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
            Some(sensor_state("inactive", 0, false)),
            "{refusal}"
        );
        assert_eq!(sensor_frequency(&mut world), 0, "{refusal}");
        if refusal == "spectator" {
            world.set_player_mode_and_abilities_for_test(
                ConnectionId(1),
                GameType::Survival,
                PlayerAbilities::for_game_type(GameType::Survival),
            );
        }
        let stone =
            vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID)
                .unwrap();
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            VanillaGameEvent::BLOCK_CHANGE,
            SOURCE,
            Some(player),
            (refusal == "affected_wool").then_some(stone),
        );
        for _ in 0..8 {
            world.tick(receiver.drain());
        }
        assert_eq!(
            sensor_frequency(&mut world),
            11,
            "{refusal}: the allowed event arrives"
        );
        assert_ne!(
            world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
            Some(sensor_state("inactive", 0, false)),
            "{refusal}: the allowed event activates the sensor"
        );
    }
}

/// The end-to-end chain: a sensor-produced `sculk_sensor_tendrils_clicking` is
/// the one row `shrieker_can_listen` carries, so it travels to the shrieker and
/// drives `tryShriek` without any test-only listenable override.
#[test]
fn a_sensor_produced_tendrils_event_reaches_the_shrieker_and_shrieks() {
    let (_sender, receiver, mut world) = sensor_world();
    let player = world.player_entity_id_for_test(ConnectionId(1));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        SHRIEKER,
        shrieker_state(false, true),
        SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, SHRIEKER)
        .and_then(|entity| entity.sculk_shrieker_mut())
        .expect("the write created the shrieker's block entity")
        .vibration_data_mut_for_test()
        .selection_strategy_mut()
        .start_over();

    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_CHANGE,
        SOURCE,
        Some(player),
        None,
    );
    for _ in 0..8 {
        world.tick(receiver.drain());
        if world.block_state(Java26BuiltinDimension::Overworld, SENSOR)
            == Some(sensor_state("active", 8, false))
        {
            break;
        }
    }
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, SENSOR),
        Some(sensor_state("active", 8, false)),
        "the sensor activated and raised the tendrils event"
    );

    // The raise is delivered at the same tick's flush; the shrieker then
    // travels the eight blocks between them.
    for _ in 0..10 {
        world.tick(receiver.drain());
        if world.block_state(Java26BuiltinDimension::Overworld, SHRIEKER)
            == Some(shrieker_state(true, true))
        {
            break;
        }
    }
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, SHRIEKER),
        Some(shrieker_state(true, true)),
        "the tendrils event drove tryShriek"
    );
    let warning = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, SHRIEKER)
        .and_then(|entity| entity.sculk_shrieker())
        .expect("the shrieker's block entity")
        .warning_level();
    assert_eq!(warning, 1, "tryWarn escalated");
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        game_events.iter().any(|event| {
            event.game_event.resource_id() == "minecraft:shriek"
                && event.pos == SHRIEKER
                && event.source_entity_id == Some(player)
        }),
        "the shrieker raised GameEvent.SHRIEK: {game_events:?}"
    );
}

#[test]
fn calibrated_sculk_sensor_registered_radius_and_arrival_power_cover_twelve_blocks() {
    for calibrated in [false, true] {
        let (_sender, receiver, mut world) = sensor_world();
        if calibrated {
            let state = vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID,
            )
            .unwrap();
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                SENSOR,
                state,
                SET_BLOCK_UPDATE_ALL
            ));
            world.tick(receiver.drain());
            clear_listener(&mut world, SENSOR);
        }
        let player = world.player_entity_id_for_test(ConnectionId(1));
        world.record_game_event_for_test(
            Java26BuiltinDimension::Overworld,
            VanillaGameEvent::BLOCK_CHANGE,
            BlockPos { x: 12, ..SENSOR },
            Some(player),
            None,
        );
        world.tick(receiver.drain());
        let candidate = sensor_vibration(&mut world).selection_strategy().current();
        assert_eq!(candidate.is_some(), calibrated);
        if calibrated {
            assert_eq!(candidate.unwrap().0.distance, 12.0);
            for _ in 0..14 {
                world.tick(receiver.drain());
            }
            let state = world
                .block_state(Java26BuiltinDimension::Overworld, SENSOR)
                .unwrap();
            assert_eq!(
                bbb_server_types::block_queries::int_property(state, "power"),
                Some(4)
            );
            assert_eq!(sensor_frequency(&mut world), 11);
        }
    }
}

#[test]
fn an_activated_sensor_resonates_its_amethyst_to_a_calibrated_sensor_out_of_the_sources_reach() {
    const AMETHYST: BlockPos = BlockPos { x: 1, y: 64, z: 0 };
    const CALIBRATED: BlockPos = BlockPos { x: 16, y: 64, z: 0 };
    let (_sender, receiver, mut world) = sensor_world();
    for (pos, block) in [
        (
            AMETHYST,
            bbb_protocol::block_types::VANILLA_BLOCK_AMETHYST_BLOCK_ID,
        ),
        (
            CALIBRATED,
            bbb_protocol::block_types::VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID,
        ),
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            vanilla_block_default_state_id(block).unwrap(),
            SET_BLOCK_UPDATE_ALL,
        ));
    }
    world.tick(receiver.drain());
    clear_listener(&mut world, SENSOR);
    clear_listener(&mut world, CALIBRATED);
    // Six blocks from the plain sensor, twenty-two from the calibrated one.
    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_CHANGE,
        BlockPos { x: -6, ..SENSOR },
        None,
        None,
    );
    let mut resonated = false;
    for _ in 0..30 {
        world.tick(receiver.drain());
        let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
        resonated |= game_events.iter().any(|event| {
            event.game_event.resource_id() == "minecraft:resonate_11" && event.pos == AMETHYST
        });
    }
    assert!(
        resonated,
        "SculkSensorBlock.java:236 raised resonate_11 at the amethyst"
    );
    let calibrated = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, CALIBRATED)
        .and_then(|entity| entity.sculk_sensor())
        .expect("the calibrated sensor's block entity")
        .last_vibration_frequency();
    assert_eq!(
        calibrated, 11,
        "the resonance carries block_change's frequency"
    );
}
