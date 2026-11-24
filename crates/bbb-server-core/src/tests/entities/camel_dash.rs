//! Camel riding-jump presentation and cooldown through real client packets.

use super::feeding::interact_entity_packet;
use super::horse_interaction::{hold, horse_world, item_id, mount_screen, played, worn};
use super::saddled_mounts::{drive, mount, spawn_beside};
use super::*;
use bbb_protocol::entity_synched_data::VANILLA_CAMEL_DASH_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{EntityDataValueKind, PlayerCommand, PlayerCommandAction};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation = world.game_time() as u64 + 10000;
    let tick = console_command(sender, receiver, world, operation, text);
    assert!(console_command_answer(&tick).0.is_some(), "{text}");
    tick
}

fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    let tick = world.tick(receiver.drain());
    for event in &tick.events {
        if let WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::KeepAlive { id },
            ..
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: ConnectionId(1),
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
    tick
}

fn fixture(
    kind: i32,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver, mut world) = horse_world();
    command(&sender, &receiver, &mut world, "gamerule spawn_mobs false");
    command(&sender, &receiver, &mut world, "time set midnight");
    for _ in 0..160 {
        tick(&sender, &receiver, &mut world);
    }
    let camel = spawn_beside(&mut world, kind);
    let uuid = world.stored_entity_for_test(DIM, camel).unwrap().uuid();
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(camel),
    );
    assert_eq!(
        worn(&world, camel, EquipmentSlot::Saddle),
        Some(item_id("minecraft:saddle")),
        "the saddle click equips the real camel saddle slot"
    );
    let opened = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: camel,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }),
    );
    let screen = mount_screen(&opened.events).expect("the camel family inventory opens");
    assert_eq!(screen.entity_id, camel);
    assert_eq!(screen.inventory_columns, 0);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(bbb_protocol::packets::ContainerCloseRequest {
            container_id: screen.container_id,
        }),
    );
    hold(&mut world, "minecraft:air", 0);
    mount(&sender, &receiver, &mut world, camel);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let opened = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::Interact(InteractEntity {
            entity_id: camel,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }),
    );
    let screen = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::MountScreenOpen(screen),
            } => Some(*screen),
            _ => None,
        })
        .expect("a non-passenger opens the occupied camel inventory");
    assert_eq!(screen.entity_id, camel);
    assert_eq!(screen.inventory_columns, 0);
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ContainerClose(bbb_protocol::packets::ContainerCloseRequest {
            container_id: screen.container_id,
        }),
    );
    (sender, receiver, world, camel, uuid)
}

fn jump(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    amount: i32,
) -> TickResult {
    let entity_id = world.player_entity_id_for_test(ConnectionId(1));
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id,
            action: PlayerCommandAction::StartRidingJump,
            data: amount,
        }),
    )
}

fn dash_metadata(events: &[WorldEvent], id: i32, dashing: bool) -> bool {
    events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(1)),
        Some(ClientboundPlayPacket::SetEntityData(data))
            if data.id == id && data.values.iter().any(|value|
                value.data_id == VANILLA_CAMEL_DASH_DATA_ID && value.value == EntityDataValueKind::Boolean(dashing))
    ))
}

fn ready_sound(events: &[WorldEvent], resource: &str, position: Vec3d) -> bool {
    events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(1)),
        Some(ClientboundPlayPacket::Sound(sound))
            if matches!(sound.sound, SoundEventHolder::Reference { registry_id } if vanilla_sound_event_resource_id(registry_id) == Some(resource))
                && sound.source == bbb_protocol::packets::SoundSource::Neutral && sound.volume == 1.0 && sound.pitch == 1.0
                && sound.position == Vec3d { x: position.x.floor() + 0.5, y: position.y.floor() + 0.5, z: position.z.floor() + 0.5 }
    ))
}

#[test]
fn camel_family_dash_tracks_the_riders_packets_and_sounds_ready_even_after_loading_silent() {
    for (kind, family) in [
        (VANILLA_ENTITY_TYPE_CAMEL_ID, "camel"),
        (VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, "camel_husk"),
    ] {
        let (sender, receiver, mut world, camel, uuid) = fixture(kind);
        let dash = format!("minecraft:entity.{family}.dash");
        let ready = format!("minecraft:entity.{family}.dash_ready");
        let rejected = jump(&sender, &receiver, &mut world, 0);
        assert!(!dash_metadata(&rejected.events, camel, true) && !played(&rejected.events, &dash));
        let pos = world
            .stored_entity_for_test(DIM, camel)
            .unwrap()
            .block_position();
        let sensor = BlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z + 3,
        };
        let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:sculk_sensor",
            &BTreeMap::from([
                ("sculk_sensor_phase".to_owned(), "inactive".to_owned()),
                ("power".to_owned(), "0".to_owned()),
                ("waterlogged".to_owned(), "false".to_owned()),
            ]),
        )
        .unwrap();
        assert!(world.set_block(DIM, sensor, state, SET_BLOCK_UPDATE_ALL));
        let dashed = jump(&sender, &receiver, &mut world, 60);
        let started = world.game_time();
        let mut published_dash = dash_metadata(&dashed.events, camel, true);
        assert!(played(&dashed.events, &dash));
        assert!(dashed.events.iter().any(|event| matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::Sound(sound))
                if matches!(sound.sound, SoundEventHolder::Reference { registry_id } if vanilla_sound_event_resource_id(registry_id) == Some(dash.as_str()))
                    && sound.volume == 0.8 && (0.8..=1.2).contains(&sound.pitch)
        )));
        let loaded = command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{Silent:1b}}"),
        );
        published_dash |= dash_metadata(&loaded.events, camel, true);
        assert!(
            !dash_metadata(&loaded.events, camel, false),
            "loading Silent preserves the running dash"
        );
        // The external rider supplies movement; START_RIDING_JUMP supplies no server impulse.
        let (expected, actual) = drive(&sender, &receiver, &mut world, camel);
        assert_eq!(actual, expected);
        let mut cleared = false;
        let mut heard_ready = false;
        let mut sensor_responded = false;
        for _ in 0..60 {
            let output = tick(&sender, &receiver, &mut world);
            published_dash |= dash_metadata(&output.events, camel, true);
            let clear = dash_metadata(&output.events, camel, false);
            if world.game_time() - started < 5 {
                assert!(
                    !clear,
                    "the loaded dash remains active before its landing window"
                );
            }
            cleared |= clear;
            if ready_sound(
                &output.events,
                &ready,
                world.stored_entity_for_test(DIM, camel).unwrap().position(),
            ) {
                assert!((50..=60).contains(&(world.game_time() - started)));
                heard_ready = true;
            }
            if let Some(state) = world.block_state(DIM, sensor) {
                sensor_responded |= bbb_server_types::block_queries::int_property(state, "power")
                    .is_some_and(|power| power > 0)
                    && world
                        .level_block_entity_for_test(DIM, sensor)
                        .and_then(|entity| entity.sculk_sensor())
                        .is_some_and(|sensor| sensor.last_vibration_frequency() == 4);
            }
        }
        assert!(
            published_dash && cleared,
            "dash metadata starts and clears on the grounded tick path"
        );
        assert!(
            heard_ready,
            "ready bypasses Silent and plays at the block center"
        );
        assert!(
            sensor_responded,
            "the dash's entity action activates a real sculk sensor"
        );
    }
}

#[test]
fn a_saddled_camel_rejects_riding_jumps_while_sitting_and_during_its_stand_transition() {
    let (sender, receiver, mut world, camel, uuid) = fixture(VANILLA_ENTITY_TYPE_CAMEL_ID);
    let sitting_since = world.game_time() - 100;
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{LastPoseTick:-{sitting_since}L}}"),
    );
    let seated = jump(&sender, &receiver, &mut world, 60);
    assert!(!dash_metadata(&seated.events, camel, true));
    assert!(!played(&seated.events, "minecraft:entity.camel.dash"));
    let stand_started = world.game_time();
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{LastPoseTick:{stand_started}L}}"),
    );
    let standing = jump(&sender, &receiver, &mut world, 60);
    assert!(!dash_metadata(&standing.events, camel, true));
    assert!(!played(&standing.events, "minecraft:entity.camel.dash"));
    while world.game_time() - stand_started < 53 {
        tick(&sender, &receiver, &mut world);
    }
    let dashed = jump(&sender, &receiver, &mut world, 60);
    let mut published = dash_metadata(&dashed.events, camel, true);
    assert!(played(&dashed.events, "minecraft:entity.camel.dash"));
    for _ in 0..3 {
        let output = tick(&sender, &receiver, &mut world);
        published |= dash_metadata(&output.events, camel, true);
    }
    assert!(published, "a fully standing camel accepts its rider's jump");
}
