//! Camel sitting through live AI, tracking, saves, water and a player's lead.

use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_poses::{VANILLA_ENTITY_POSE_SITTING_ID, VANILLA_ENTITY_POSE_STANDING_ID};
use bbb_protocol::entity_synched_data::{
    VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID, VANILLA_ENTITY_POSE_DATA_ID,
};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID};
use bbb_protocol::item_types::{VANILLA_ITEM_CACTUS_ID, VANILLA_ITEM_LEAD_ID};
use bbb_protocol::packets::{EntityDataValueKind, ItemStackSummary, SoundEventHolder};

const ALICE: ConnectionId = ConnectionId(1);

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation_id = world.game_time() as u64 + 10000;
    let tick = console_command(sender, receiver, world, operation_id, text);
    assert!(console_command_answer(&tick).0.is_some(), "{text}");
    tick
}

fn last_pose(world: &ServerWorld, id: i32) -> i64 {
    let Some(NbtSummaryValue::Compound(record)) =
        world.stored_entity_command_data_for_test(DIMENSION, id)
    else {
        panic!("camel record");
    };
    match record
        .iter()
        .find(|field| field.name == "LastPoseTick")
        .unwrap()
        .value
    {
        NbtSummaryValue::Long(tick) => tick,
        _ => panic!("LastPoseTick is a long"),
    }
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn horizontal_distance(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn metadata(tick: &TickResult, id: i32, data_id: u8, expected: EntityDataValueKind) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetEntityData(update), .. }
        if update.id == id && update.values.iter().any(|value|
            value.data_id == data_id && value.value == expected))
    })
}

fn sound(tick: &TickResult, name: &str) -> bool {
    let expected = SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(name)
            .unwrap(),
    };
    tick.events.iter().any(|event| match event {
        WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } => sound.sound == expected,
        WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::SoundEntity(sound),
            ..
        } => sound.sound == expected,
        _ => false,
    })
}

fn hold(world: &mut ServerWorld, item_id: i32) {
    world.set_inventory_slot_for_test(
        ALICE,
        0,
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        },
    );
}

#[test]
fn camel_idle_sitting_survives_a_real_restart_and_food_wakes_it_before_it_walks() {
    let root = std::env::temp_dir().join(format!("bbb-camel-sitting-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-camel-sitting");
    let paths = WorldSavePaths::new(&root).unwrap();
    let mut candidate = storage.load_candidate().unwrap();
    install_forced_ticket(&mut candidate);
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    // Keep wandering on the saved, ticking chunk until the random idle pose runs.
    for edge in 0..=15 {
        for y in 65..=67 {
            for (x, z) in [(0, edge), (15, edge), (edge, 0), (edge, 15)] {
                world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    block_state_id("minecraft:stone"),
                    SET_BLOCK_UPDATE_ALL,
                );
            }
        }
    }
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 2.5,
            y: 65.0,
            z: 2.5,
        },
    );
    command(&sender, &receiver, &mut world, "gamerule spawn_mobs false");
    let (camel, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let standing_height = world
        .stored_entity_for_test(DIMENSION, camel)
        .unwrap()
        .bounding_box()
        .max_y
        - 65.0;
    let mut sat = false;
    let mut pose_packet = false;
    let mut signed_packet = false;
    let mut sit_sound = false;
    for _ in 0..4000 {
        let tick = world.tick(receiver.drain());
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                enqueue(
                    &sender,
                    WorldCommand::Play {
                        connection_id: ALICE,
                        packet: PlayServerbound::KeepAlive { id: *id },
                    },
                )
                .unwrap();
            }
        }
        let signed = last_pose(&world, camel);
        if signed < 0 {
            assert_eq!(-signed, world.game_time());
            signed_packet |= metadata(
                &tick,
                camel,
                VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
                EntityDataValueKind::Long(signed),
            );
            pose_packet |= metadata(
                &tick,
                camel,
                VANILLA_ENTITY_POSE_DATA_ID,
                EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_SITTING_ID),
            );
            sit_sound |= sound(&tick, "minecraft:entity.camel.sit");
            sat = true;
            break;
        }
    }
    assert!(sat, "an idle camel eventually chooses to sit");
    let sitting_at = position(&world, camel);
    let sitting_tick = last_pose(&world, camel);
    let body = world.stored_entity_for_test(DIMENSION, camel).unwrap();
    assert!(body.bounding_box().max_y - body.position().y < standing_height - 1.0);
    for _ in 0..45 {
        let tick = world.tick(receiver.drain());
        signed_packet |= metadata(
            &tick,
            camel,
            VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
            EntityDataValueKind::Long(sitting_tick),
        );
        pose_packet |= metadata(
            &tick,
            camel,
            VANILLA_ENTITY_POSE_DATA_ID,
            EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_SITTING_ID),
        );
        sit_sound |= sound(&tick, "minecraft:entity.camel.sit");
        assert!(
            horizontal_distance(position(&world, camel), sitting_at) < 1.0e-6,
            "sitting on the ground cannot slide"
        );
    }
    assert!(
        signed_packet && pose_packet && sit_sound,
        "the observer receives the sitting pose, canonical signed time and sound"
    );
    let request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    drop(world);
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let mut reloaded = storage.load_candidate().unwrap();
    install_forced_ticket(&mut reloaded);
    let mut world = spawn_pinned_restore(config, &reloaded);
    let camel = world
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .expect("the same camel reloads");
    assert_eq!(last_pose(&world, camel), sitting_tick);
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, camel)
            .unwrap()
            .pose_id(),
        VANILLA_ENTITY_POSE_SITTING_ID
    );
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let food_at = Vec3d {
        x: if sitting_at.x < 8.0 { 12.5 } else { 2.5 },
        ..sitting_at
    };
    world.set_player_position_for_test(ALICE, food_at);
    hold(&mut world, VANILLA_ITEM_CACTUS_ID);
    let mut stood = false;
    for _ in 0..30 {
        let tick = world.tick(receiver.drain());
        if last_pose(&world, camel) >= 0 {
            assert!(sound(&tick, "minecraft:entity.camel.stand"));
            assert_eq!(last_pose(&world, camel), world.game_time());
            stood = true;
            break;
        }
    }
    assert!(stood, "food navigation wakes a settled sitting camel");
    let stand_tick = last_pose(&world, camel);
    while world.game_time() - stand_tick < 51 {
        world.tick(receiver.drain());
        assert!(
            horizontal_distance(position(&world, camel), sitting_at) < 1.0e-6,
            "standing animation holds the grounded camel still"
        );
    }
    let before = horizontal_distance(sitting_at, food_at);
    tick_until(&mut world, 160, |world| {
        horizontal_distance(position(world, camel), food_at) < before - 2.0
    });
    drop(world);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn camel_and_camel_husk_stand_when_hurt_or_in_water_even_with_no_ai() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    for _ in 0..160 {
        world.tick(receiver.drain());
    }
    for (kind, x, stand_sound) in [
        (
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            4,
            "minecraft:entity.camel.stand",
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            11,
            "minecraft:entity.camel_husk.stand",
        ),
    ] {
        let (id, uuid) = spawn(
            &mut world,
            kind,
            Vec3d {
                x: f64::from(x) + 0.5,
                y: 65.0,
                z: 5.5,
            },
        );
        let rider_uuid = Uuid::from_u128(1000 + x as u128);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                "summon armor_stand {} 65 5.5 {{UUID:[I;0,0,0,{}],Small:1b,NoGravity:1b,Invulnerable:1b}}",
                f64::from(x) + 0.5,
                1000 + x
            ),
        );
        let rider = world
            .live_entity_id_for_uuid_for_test(DIMENSION, rider_uuid)
            .unwrap();
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("ride {rider_uuid} mount {uuid}"),
        );
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{NoAI:1b,LastPoseTick:-1L}}"),
        );
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .pose_id(),
            VANILLA_ENTITY_POSE_SITTING_ID
        );
        assert_eq!(last_pose(&world, id), -1);
        let health = world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .living()
            .unwrap()
            .health();
        let tick = command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 1 minecraft:generic"),
        );
        let body = world.stored_entity_for_test(DIMENSION, id).unwrap();
        assert!(matches!(body.data(), EntityData::GeneralLiving(data) if data.is_no_ai()));
        assert_eq!(body.living().unwrap().health(), health - 1.0);
        assert_eq!(body.pose_id(), VANILLA_ENTITY_POSE_STANDING_ID);
        let standing_tick = last_pose(&world, id);
        assert!(standing_tick >= 0 && world.game_time() - standing_tick >= 53);
        assert!(metadata(
            &tick,
            id,
            VANILLA_ENTITY_POSE_DATA_ID,
            EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_STANDING_ID),
        ));
        assert!(metadata(
            &tick,
            id,
            VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
            EntityDataValueKind::Long(standing_tick),
        ));
        assert_eq!(world.stored_passengers_for_test(DIMENSION, id), &[rider]);
        let passenger = world.stored_entity_for_test(DIMENSION, rider).unwrap();
        let scale = body
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
            as f32;
        let [_, height] = bbb_protocol::entity_types::vanilla_entity_type_dimensions(kind).unwrap();
        let (sin, cos) = (-f64::from(body.y_rot()).to_radians()).sin_cos();
        let [rider_x, rider_y, rider_z] =
            bbb_protocol::entity_types::vanilla_entity_type_vehicle_attachment(
                passenger.entity_type_id(),
            )
            .unwrap();
        let (rider_sin, rider_cos) = (-f64::from(passenger.y_rot()).to_radians()).sin_cos();
        let expected = Vec3d {
            x: body.position().x + f64::from(0.5 * scale) * sin
                - (f64::from(rider_x) * rider_cos + f64::from(rider_z) * rider_sin),
            y: body.position().y + f64::from(height * scale - 0.375 * scale) - f64::from(rider_y),
            z: body.position().z + f64::from(0.5 * scale) * cos
                - (f64::from(rider_z) * rider_cos - f64::from(rider_x) * rider_sin),
        };
        for (actual, expected) in [
            (passenger.position().x, expected.x),
            (passenger.position().y, expected.y),
            (passenger.position().z, expected.z),
        ] {
            assert!((actual - expected).abs() < 1.0e-5);
        }
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{LastPoseTick:-1L}}"),
        );
        assert_eq!(last_pose(&world, id), -1);
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .pose_id(),
            VANILLA_ENTITY_POSE_SITTING_ID
        );
        for dx in -1..=1 {
            for z in 4..=6 {
                world.set_block(
                    DIMENSION,
                    BlockPos {
                        x: x + dx,
                        y: 65,
                        z,
                    },
                    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                        "minecraft:water",
                        &BTreeMap::from([("level".to_string(), "0".to_string())]),
                    )
                    .expect("built-in Java 26.1 water source state"),
                    SET_BLOCK_UPDATE_ALL,
                );
            }
        }
        let tick = world.tick(receiver.drain());
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, id)
                .unwrap()
                .pose_id(),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
        assert_eq!(last_pose(&world, id), (world.game_time() - 53).max(0));
        assert!(
            !sound(&tick, stand_sound),
            "water uses the instant pose change"
        );
    }
}

#[test]
fn a_player_lead_elastic_pull_normally_wakes_a_settled_camel() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 5.5,
            y: 65.0,
            z: 8.5,
        },
    );
    for _ in 0..160 {
        world.tick(receiver.drain());
    }
    let (camel, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
    );
    hold(&mut world, VANILLA_ITEM_LEAD_ID);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: camel,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert_eq!(
        crate::entity::leash::leash_holder(world.stored_entity_for_test(DIMENSION, camel).unwrap()),
        Some(world.player_entity_id_for_test(ALICE))
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,LastPoseTick:-1L}}"),
    );
    assert!(last_pose(&world, camel) < 0);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 11.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let tick = world.tick(receiver.drain());
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, camel)
            .unwrap()
            .pose_id(),
        VANILLA_ENTITY_POSE_STANDING_ID
    );
    assert_eq!(last_pose(&world, camel), world.game_time());
    assert!(sound(&tick, "minecraft:entity.camel.stand"));
}
