use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;

fn freeze_running(world: &ServerWorld, enderman: i32) -> bool {
    let info = world.mob_goal_debug_for_test(DIMENSION, enderman).unwrap();
    let goal = info
        .goals
        .iter()
        .find(|goal| goal.name == "EndermanFreezeWhenLookedAt")
        .expect("the production enderman installs its freeze goal");
    assert_eq!(goal.priority, 1);
    goal.is_running
}

pub(super) fn rotate(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    rotation: PlayerRotation,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(rotation),
            on_ground: true,
            horizontal_collision: false,
        }),
    );
}

fn canonical_zero(angle: f32) -> f32 {
    if angle == 0.0 {
        0.0
    } else {
        angle
    }
}

pub(super) fn stare_rotation(world: &ServerWorld, enderman: i32) -> PlayerRotation {
    let player = world.publication().players[0].position;
    let entity = world.stored_entity_for_test(DIMENSION, enderman).unwrap();
    let position = entity.position();
    let dx = position.x - player.x;
    let dz = position.z - player.z;
    let dy = position.y + f64::from(entity.eye_height()) - (player.y + 1.62);
    PlayerRotation {
        y_rot: canonical_zero((-dx).atan2(dz).to_degrees() as f32),
        x_rot: canonical_zero(-dy.atan2(dx.hypot(dz)).to_degrees() as f32),
    }
}

fn stare(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    enderman: i32,
) {
    for _ in 0..8 {
        let rotation = stare_rotation(world, enderman);
        rotate(sender, receiver, world, rotation);
    }
}

pub(super) fn teleport(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation: u64,
    position: Vec3d,
) {
    let result = console_command(
        sender,
        receiver,
        world,
        operation,
        &format!("tp alice {} {} {}", position.x, position.y, position.z),
    );
    assert_eq!(console_command_answer(&result).0, Some(1));
    let id = result
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("the command sends a teleport");
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::AcceptTeleportation { id },
    );
}

fn angry_enderman(
    mut config: ServerWorldConfig,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    config.game_type = GameType::Survival;
    config.admin_lists = operator_config(&[("alice", 1)]).admin_lists;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    teleport(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let (enderman, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    rotate(
        &sender,
        &receiver,
        &mut world,
        PlayerRotation {
            y_rot: 180.0,
            x_rot: 0.0,
        },
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity {
            entity_id: enderman,
        }),
    );
    tick_until(&mut world, 4, |world| {
        target_of(world, enderman) == Some(Uuid::from_u128(1))
    });
    (sender, receiver, world, enderman)
}

#[test]
fn an_attacked_enderman_freezes_navigation_and_looks_at_the_staring_players_eyes() {
    let (sender, receiver, mut world, enderman) = angry_enderman(datapack_config());
    tick_until(&mut world, 400, |world| {
        world.ground_path_for_test(DIMENSION, enderman).is_some()
    });
    let position = world
        .stored_entity_for_test(DIMENSION, enderman)
        .unwrap()
        .position();
    teleport(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            z: position.z - 8.0,
            ..position
        },
    );
    assert!(world.ground_path_for_test(DIMENSION, enderman).is_some());
    stare(&sender, &receiver, &mut world, enderman);
    assert!(freeze_running(&world, enderman));
    assert!(world.ground_path_for_test(DIMENSION, enderman).is_none());
    let player = world.publication().players[0].position;
    let entity = world.stored_entity_for_test(DIMENSION, enderman).unwrap();
    let position = entity.position();
    let expected = (position.y + f64::from(entity.eye_height()) - (player.y + 1.62))
        .atan2((position.x - player.x).hypot(position.z - player.z))
        .to_degrees() as f32;
    assert!(
        (entity.x_rot() - expected).abs() < 3.0,
        "pitch {} should look at player eyes {expected}",
        entity.x_rot()
    );
    rotate(
        &sender,
        &receiver,
        &mut world,
        PlayerRotation {
            y_rot: 180.0,
            x_rot: 0.0,
        },
    );
    for _ in 0..4 {
        world.tick(Vec::new());
    }
    assert!(!freeze_running(&world, enderman));
    assert_eq!(target_of(&world, enderman), Some(Uuid::from_u128(1)));
}

#[test]
fn a_solid_occluder_or_a_target_beyond_sixteen_blocks_stops_freezing_without_clearing_target() {
    for obstructed in [true, false] {
        let (sender, receiver, mut world, enderman) = angry_enderman(datapack_config());
        stare(&sender, &receiver, &mut world, enderman);
        assert!(freeze_running(&world, enderman));
        if obstructed {
            let player = world.publication().players[0].position;
            let end = world
                .stored_entity_for_test(DIMENSION, enderman)
                .unwrap()
                .position();
            let x = ((player.x + end.x) * 0.5).floor() as i32;
            let z = ((player.z + end.z) * 0.5).floor() as i32;
            for y in 65..=68 {
                assert!(world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    block_state_id("minecraft:stone"),
                    SET_BLOCK_UPDATE_ALL
                ));
            }
        } else {
            teleport(
                &sender,
                &receiver,
                &mut world,
                2,
                Vec3d {
                    x: 8.5,
                    y: 65.0,
                    z: -11.0,
                },
            );
        }
        stare(&sender, &receiver, &mut world, enderman);
        assert!(!freeze_running(&world, enderman));
        assert_eq!(target_of(&world, enderman), Some(Uuid::from_u128(1)));
    }
}

fn disguise_config() -> ServerWorldConfig {
    let root = std::env::temp_dir().join(format!("bbb-gaze-disguise-{}", Uuid::new_v4()));
    let pack = root.join("datapacks/gaze");
    std::fs::create_dir_all(pack.join("data/minecraft/tags/item")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"gaze test","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    std::fs::write(
        pack.join("data/minecraft/tags/item/gaze_disguise_equipment.json"),
        r#"{"replace":true,"values":["minecraft:carved_pumpkin","minecraft:iron_helmet"]}"#,
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    }
}

#[test]
fn live_head_equipment_uses_the_gaze_disguise_tag_and_removing_it_restarts_freezing() {
    for (config, item) in [
        (datapack_config(), "minecraft:carved_pumpkin"),
        (disguise_config(), "minecraft:iron_helmet"),
    ] {
        let (sender, receiver, mut world, enderman) = angry_enderman(config);
        stare(&sender, &receiver, &mut world, enderman);
        assert!(freeze_running(&world, enderman));
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            39,
            ItemStackSummary {
                item_id: Some(vanilla_item_type_for_resource_id(item).unwrap().id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        );
        stare(&sender, &receiver, &mut world, enderman);
        assert!(!freeze_running(&world, enderman), "{item}");
        assert_eq!(target_of(&world, enderman), Some(Uuid::from_u128(1)));
        world.set_inventory_slot_for_test(ConnectionId(1), 39, ItemStackSummary::empty());
        // The disguised player let the enderman's melee goal walk it in; a
        // stare from inside four blocks would teleport it instead
        // (`EnderMan.java:558-560`).
        let position = world
            .stored_entity_for_test(DIMENSION, enderman)
            .unwrap()
            .position();
        teleport(
            &sender,
            &receiver,
            &mut world,
            2,
            Vec3d {
                z: position.z - 8.0,
                ..position
            },
        );
        stare(&sender, &receiver, &mut world, enderman);
        assert!(freeze_running(&world, enderman), "{item}");
    }
}

#[test]
fn enderman_daylight_real_world_clears_old_target_and_emits_teleport_effects() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.admin_lists = operator_config(&[("alice", 1)]).admin_lists;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // The survival observer must remain alive throughout the daylight wait.
    let protection = console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        "effect give alice minecraft:resistance infinite 4 true",
    );
    assert_eq!(console_command_answer(&protection).0, Some(1));
    let observer_health = world.player_health_for_test(ConnectionId(1));
    install_floor(&mut world);
    teleport(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let position = Vec3d {
        x: 8.5,
        y: 65.0,
        z: 12.5,
    };
    let mut saved = crate::entity::EntityStore::new(34, crate::entity::EntityIdCounter::new());
    let id = saved
        .spawn_general_living(VANILLA_ENTITY_TYPE_ENDERMAN_ID, position)
        .unwrap();
    let NbtSummaryValue::Compound(mut record) = saved.command_data(id).unwrap() else {
        panic!("enderman record");
    };
    record.retain(|entry| entry.name != "anger_end_time" && entry.name != "angry_at");
    record.extend([
        NbtSummaryEntry {
            name: "anger_end_time".into(),
            value: NbtSummaryValue::Long(world.game_time() + 10000),
        },
        NbtSummaryEntry {
            name: "angry_at".into(),
            value: NbtSummaryValue::IntArray(vec![0, 0, 0, 1]),
        },
    ]);
    saved.load_command_data(id, &record).unwrap();
    let enderman = world
        .restore_stored_entity_for_test(
            DIMENSION,
            Uuid::from_u128(100),
            VANILLA_ENTITY_TYPE_ENDERMAN_ID,
            position,
            saved.get(id).unwrap().data().clone(),
        )
        .unwrap();
    tick_until(&mut world, 4, |world| {
        target_of(world, enderman) == Some(Uuid::from_u128(1))
    });
    for x in -2..=2 {
        for z in -2..=2 {
            world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (x, z));
        }
    }
    stare(&sender, &receiver, &mut world, enderman);
    assert!(freeze_running(&world, enderman));
    let sound = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.enderman.teleport",
    )
    .unwrap();
    for _ in 0..3000 {
        let result = world.tick(receiver.drain());
        for event in &result.events {
            if let WorldEvent::Send {
                packet: ClientboundPlayPacket::KeepAlive { id },
                ..
            } = event
            {
                enqueue(
                    &sender,
                    WorldCommand::Play {
                        connection_id: ConnectionId(1),
                        packet: PlayServerbound::KeepAlive { id: *id },
                    },
                )
                .unwrap();
            }
        }
        let has_event = result.events.iter().any(|event| matches!(event,
            WorldEvent::Send {packet:ClientboundPlayPacket::EntityEvent(event),..} if event.entity_id == enderman && event.event_id == 46));
        if !has_event {
            continue;
        }
        assert_eq!(
            world.player_health_for_test(ConnectionId(1)),
            observer_health
        );
        assert_eq!(target_of(&world, enderman), None);
        let body = world.stored_entity_for_test(DIMENSION, enderman).unwrap();
        assert!(body.tick_count() >= 600);
        let sounds:Vec<_> = result.events.iter().filter_map(|event| match event {
            WorldEvent::Send {packet:ClientboundPlayPacket::Sound(event),..}
                if matches!(event.sound, bbb_protocol::packets::SoundEventHolder::Reference {registry_id} if registry_id == sound) => Some(event),
            _ => None,
        }).collect();
        let emitted = body.old_position();
        let player = world.publication().players[0].position;
        let distance = (emitted.x - player.x).powi(2)
            + (emitted.y - player.y).powi(2)
            + (emitted.z - player.z).powi(2);
        // PlayerList.java:597-617: the destination sound is audible only within its radius.
        assert_eq!(sounds.len(), if distance < 16.0 * 16.0 { 2 } else { 0 });
        if let [first, second] = sounds.as_slice() {
            assert_eq!(first.position, second.position);
        }
        assert_ne!(body.position(), position);
        return;
    }
    let body = world.stored_entity_for_test(DIMENSION, enderman).unwrap();
    let state = (
        body.tick_count(),
        body.position(),
        target_of(&world, enderman),
    );
    let pos = BlockPos {
        x: state.1.x.floor() as i32,
        y: (state.1.y + 2.5).floor() as i32,
        z: state.1.z.floor() as i32,
    };
    let light = world.entity_level_light_for_test(DIMENSION, pos);
    panic!("the real daylight custom step teleports and clears the aged target; state {state:?}, light {light:?}, player_health {}, viewers {:?}", world.player_health_for_test(ConnectionId(1)), world.player_tracking_viewers_for_test(DIMENSION, enderman));
}
