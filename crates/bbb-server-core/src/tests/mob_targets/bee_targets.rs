use super::enderman_freeze::teleport;
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_synched_data::VANILLA_BEE_ANGER_END_TIME_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_COW_ID};

fn bee(world: &mut ServerWorld, position: Vec3d) -> (i32, Uuid) {
    spawn(world, VANILLA_ENTITY_TYPE_BEE_ID, position)
}

fn session() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
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
    (sender, receiver, world)
}

fn anger_updates(tick: &TickResult, updates: &mut BTreeMap<i32, i64>) {
    for event in &tick.events {
        if let WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityData(update),
            ..
        } = event
        {
            for value in &update.values {
                if value.data_id == VANILLA_BEE_ANGER_END_TIME_DATA_ID {
                    if let EntityDataValueKind::Long(end_time) = value.value {
                        updates.insert(update.id, end_time);
                    }
                }
            }
        }
    }
}

fn attack(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    target: i32,
) -> TickResult {
    let before = world
        .stored_entity_for_test(DIMENSION, target)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let tick = play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: target }),
    );
    assert!(
        world
            .stored_entity_for_test(DIMENSION, target)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < before
    );
    tick
}

#[test]
fn an_attacked_bee_alerts_visible_kin_and_publishes_one_stable_typed_anger_deadline() {
    let (sender, receiver, mut world) = session();
    let (source, _) = bee(
        &mut world,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 7.0,
        },
    );
    let (kin, _) = bee(
        &mut world,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 6.0,
        },
    );
    world.tick(Vec::new());
    let mut updates = BTreeMap::new();
    anger_updates(
        &attack(&sender, &receiver, &mut world, source),
        &mut updates,
    );
    for _ in 0..16 {
        let tick = world.tick(Vec::new());
        anger_updates(&tick, &mut updates);
        if [source, kin].iter().all(|id| {
            target_of(&world, *id) == Some(Uuid::from_u128(1))
                && saved_anger(&world, *id).1 == Some(Uuid::from_u128(1))
        }) {
            break;
        }
    }
    let saved = [saved_anger(&world, source), saved_anger(&world, kin)];
    for (id, anger) in [source, kin].into_iter().zip(saved) {
        assert_eq!(
            target_of(&world, id),
            Some(Uuid::from_u128(1)),
            "bee {id} (source {source}, kin {kin}), saved {saved:?}, metadata {updates:?}"
        );
        assert_eq!(anger.1, Some(Uuid::from_u128(1)));
        let remaining = anger.0.unwrap() - world.game_time();
        assert!((390..=780).contains(&remaining), "{remaining}");
        assert_eq!(
            updates.get(&id),
            anger.0.as_ref(),
            "the Long metadata uses the saved typed deadline"
        );
    }
    for _ in 0..10 {
        world.tick(Vec::new());
        for (id, anger) in [source, kin].into_iter().zip(saved) {
            assert_eq!(target_of(&world, id), Some(Uuid::from_u128(1)));
            assert_eq!(
                saved_anger(&world, id),
                anger,
                "holding the player does not restart the timer"
            );
        }
    }
}

#[test]
fn source_line_of_sight_blocks_alerting_a_kin_that_can_see_the_attacker() {
    let (sender, receiver, mut world) = session();
    let (source, _) = bee(
        &mut world,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 7.0,
        },
    );
    let (kin, _) = bee(
        &mut world,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 6.0,
        },
    );
    for y in 65..=67 {
        assert!(world.set_block(
            DIMENSION,
            BlockPos { x: 8, y, z: 5 },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
    }
    world.tick(Vec::new());
    attack(&sender, &receiver, &mut world, source);
    tick_until(&mut world, 4, |world| {
        target_of(world, source) == Some(Uuid::from_u128(1))
    });
    for _ in 0..12 {
        world.tick(Vec::new());
        assert_eq!(target_of(&world, kin), None);
        assert_eq!(saved_anger(&world, kin).1, None);
        assert!(saved_anger(&world, kin).0.unwrap() <= world.game_time());
    }
}

#[test]
fn has_stung_blocks_nearest_player_even_during_universal_anger_and_clearing_it_allows_acquisition()
{
    let (sender, receiver, mut world) = session();
    let (bee, uuid) = bee(
        &mut world,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 9.5,
        },
    );
    let rule = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "gamerule universal_anger true",
    );
    assert_eq!(console_command_answer(&rule).0, Some(1));
    let deadline = world.game_time() + 1000;
    let changed = console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        &format!("data merge entity {uuid} {{anger_end_time:{deadline}L,HasStung:true}}"),
    );
    assert_eq!(console_command_answer(&changed).0, Some(1));
    assert_eq!(saved_anger(&world, bee), (Some(deadline), None));
    for _ in 0..30 {
        world.tick(Vec::new());
        assert_eq!(target_of(&world, bee), None);
        assert_eq!(saved_anger(&world, bee), (Some(deadline), None));
    }
    let changed = console_command(
        &sender,
        &receiver,
        &mut world,
        4,
        &format!("data merge entity {uuid} {{HasStung:false}}"),
    );
    assert_eq!(console_command_answer(&changed).0, Some(1));
    tick_until(&mut world, 100, |world| {
        target_of(world, bee) == Some(Uuid::from_u128(1))
    });
    world.tick(Vec::new());
    assert_eq!(saved_anger(&world, bee).1, Some(Uuid::from_u128(1)));
}

#[test]
fn typed_bee_anger_at_a_stored_attacker_survives_storage_restart_and_resolves_its_uuid() {
    let root = std::env::temp_dir().join(format!("bbb-bee-target-restart-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-bee-target-restart");
    let paths = storage.paths().clone();
    let mut candidate = storage.load_candidate().unwrap();
    install_forced_ticket(&mut candidate);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    install_floor(&mut world);
    let (bee, uuid) = bee(
        &mut world,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let (attacker, attacker_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 7.5,
            y: 65.0,
            z: 4.5,
        },
    );
    world.tick(Vec::new());
    let hurt = world
        .hurt_stored_entity_by_command_for_test(
            DIMENSION,
            bee,
            "minecraft:generic",
            1.0,
            Some(crate::commands::CommandDamageSourceEntity::Stored {
                entity_id: attacker,
                uuid: Some(attacker_uuid),
                position: Some(Vec3d {
                    x: 7.5,
                    y: 65.0,
                    z: 4.5,
                }),
                entity_resource_id: "minecraft:cow".into(),
                is_mob: true,
                is_living_non_player: true,
                display_name: NbtSummaryValue::String("minecraft:cow".into()),
            }),
        )
        .unwrap();
    assert!(hurt.applied && hurt.succeeded);
    tick_until(&mut world, 8, |world| {
        saved_anger(world, bee).1 == Some(attacker_uuid)
    });
    assert_eq!(target_of(&world, bee), Some(attacker_uuid));
    let saved = saved_anger(&world, bee);
    let request = world
        .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    drop(world);
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let mut candidate = storage.load_candidate().unwrap();
    install_forced_ticket(&mut candidate);
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    let bee = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    assert_eq!(saved_anger(&restored, bee), saved);
    assert_eq!(target_of(&restored, bee), None);
    tick_until(&mut restored, 5, |world| {
        target_of(world, bee) == Some(attacker_uuid)
    });
    assert_eq!(saved_anger(&restored, bee), saved);
    drop(restored);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn command_load_changes_a_frozen_no_ai_bee_target_before_any_entity_tick() {
    let (sender, receiver, mut world) = session();
    let (bee, uuid) = bee(
        &mut world,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 7.0,
        },
    );
    let (_, cow_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 11.5,
            y: 65.0,
            z: 7.0,
        },
    );
    world.tick(Vec::new());
    let frozen = console_command(&sender, &receiver, &mut world, 10, "tick freeze");
    assert_eq!(console_command_answer(&frozen).0, Some(1));
    let before = world
        .stored_entity_for_test(DIMENSION, bee)
        .unwrap()
        .tick_count();
    let bits = cow_uuid.as_u128();
    let changed = console_command(
        &sender,
        &receiver,
        &mut world,
        11,
        &format!(
            "data merge entity {uuid} {{NoAI:true,angry_at:[I;{},{},{},{}]}}",
            (bits >> 96) as i32,
            (bits >> 64) as i32,
            (bits >> 32) as i32,
            bits as i32,
        ),
    );
    assert_eq!(console_command_answer(&changed).0, Some(1));
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, bee)
            .unwrap()
            .tick_count(),
        before
    );
    assert_eq!(target_of(&world, bee), Some(cow_uuid));
    assert_eq!(saved_anger(&world, bee).1, Some(cow_uuid));
    let removed = console_command(
        &sender,
        &receiver,
        &mut world,
        12,
        &format!("data remove entity {uuid} angry_at"),
    );
    assert_eq!(console_command_answer(&removed).0, Some(1));
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, bee)
            .unwrap()
            .tick_count(),
        before
    );
    assert_eq!(target_of(&world, bee), None);
    assert_eq!(saved_anger(&world, bee).1, None);
}
