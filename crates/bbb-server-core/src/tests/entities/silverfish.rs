use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SILVERFISH_ID;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const NEAR: BlockPos = BlockPos {
    x: 8,
    y: 100,
    z: 11,
};
const FAR: BlockPos = BlockPos { x: 8, y: 106, z: 8 };

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation_id = world.publication().tick;
    let tick = console_command(sender, receiver, world, operation_id, text);
    let outcome = admin_outcome(&tick);
    assert!(!outcome.replayed, "{text}: fixture commands must execute");
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    tick
}

fn with_world(test: impl FnOnce(&WorldCommandSender, &WorldCommandReceiver, &mut ServerWorld)) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 100_000;
    let root = std::env::temp_dir().join(format!("bbb-silverfish-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Silverfish behavior");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config, &candidate);
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
    for text in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "gamerule random_tick_speed 0",
        "gamerule mob_griefing false",
        "time set day",
        "weather clear",
        "gamemode spectator observer",
        "tp observer 10.5 105 8.5",
        "fill 3 99 3 13 106 13 minecraft:stone",
        "fill 3 100 3 13 106 13 minecraft:air",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    test(&sender, &receiver, &mut world);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn axis_state(resource: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        resource,
        &BTreeMap::from([("axis".into(), "x".into())]),
    )
    .unwrap()
}

fn silverfish_ids(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(DIM, id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SILVERFISH_ID)
        })
        .collect()
}

fn summon(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> (i32, Uuid) {
    command(
        sender,
        receiver,
        world,
        "summon minecraft:silverfish 8.5 100 8.5 {PersistenceRequired:1b}",
    );
    let ids = silverfish_ids(world);
    assert_eq!(ids.len(), 1);
    let id = ids[0];
    let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
    command(
        sender,
        receiver,
        world,
        &format!("attribute {uuid} minecraft:movement_speed base set 0"),
    );
    let entity = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 8.5,
            y: 100.0,
            z: 8.5
        }
    );
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        0.0
    );
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        panic!("silverfish has living data");
    };
    assert!(!data.is_no_ai());
    assert!(world
        .player_tracking_viewers_for_test(DIM, id)
        .contains(&ConnectionId(1)));
    (id, uuid)
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

fn assert_no_items(world: &ServerWorld) {
    assert!(
        world.stored_entity_ids_for_test(DIM).into_iter().all(|id| {
            !matches!(
                world.stored_entity_for_test(DIM, id).unwrap().data(),
                crate::entity::EntityData::Item(_)
            )
        }),
        "silverfish block behavior creates no item drops"
    );
}

fn animation(tick: &TickResult, id: i32) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::EntityEvent(packet) }
        if packet.entity_id == id && packet.event_id == 20
    ))
}

#[test]
fn silverfish_magic_wakes_real_infested_blocks_with_current_griefing_and_drop_rules() {
    for (griefing, drops) in [(true, true), (false, true), (true, false)] {
        with_world(|sender, receiver, world| {
            for text in [
                "tp observer 10.5 100 8.5",
                "gamemode survival observer",
                "effect give observer minecraft:resistance infinite 4 true",
                "setblock 8 100 11 minecraft:infested_deepslate[axis=x]",
                "setblock 8 106 8 minecraft:infested_deepslate[axis=x]",
            ] {
                command(sender, receiver, world, text);
            }
            let (id, uuid) = summon(sender, receiver, world);
            let infested = axis_state("minecraft:infested_deepslate");
            assert_eq!(world.block_state(DIM, NEAR), Some(infested));
            assert_eq!(world.block_state(DIM, FAR), Some(infested));
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..60 {
                world.tick(receiver.drain());
            }
            world.tick_rate_mut().set_frozen(true);
            for text in [
                format!("gamerule mob_griefing {griefing}"),
                format!("gamerule block_drops {drops}"),
                format!("damage {uuid} 1 minecraft:generic"),
            ] {
                command(sender, receiver, world, &text);
            }
            assert_eq!(
                world.game_rule_for_test("minecraft:mob_griefing"),
                Some(GameRuleValue::Boolean(griefing))
            );
            assert_eq!(
                world.game_rule_for_test("minecraft:block_drops"),
                Some(GameRuleValue::Boolean(drops))
            );
            let after_generic = health(world, id);
            assert_eq!(after_generic, 7.0);
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..60 {
                world.tick(receiver.drain());
                assert_eq!(world.block_state(DIM, NEAR), Some(infested));
                assert_eq!(world.block_state(DIM, FAR), Some(infested));
                assert_eq!(silverfish_ids(world), vec![id]);
                assert_eq!(health(world, id), after_generic);
            }
            world.tick_rate_mut().set_frozen(true);
            command(
                sender,
                receiver,
                world,
                &format!("damage {uuid} 1 minecraft:magic"),
            );
            let after_magic = health(world, id);
            assert_eq!(after_magic, 6.0);
            let expected = if griefing {
                block_state_id("minecraft:air")
            } else {
                axis_state("minecraft:deepslate")
            };
            let mut added = Vec::new();
            let mut animated = Vec::new();
            let mut destroyed = false;
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..100 {
                let tick = world.tick(receiver.drain());
                for event in &tick.events {
                    match event {
                        WorldEvent::Send {
                            connection_id: ConnectionId(1),
                            packet: ClientboundPlayPacket::AddEntity(packet),
                        } if packet.entity_type_id == VANILLA_ENTITY_TYPE_SILVERFISH_ID => {
                            added.push(packet.id)
                        }
                        WorldEvent::Send {
                            connection_id: ConnectionId(1),
                            packet: ClientboundPlayPacket::EntityEvent(packet),
                        } if packet.event_id == 20 => animated.push(packet.entity_id),
                        WorldEvent::Send {
                            connection_id: ConnectionId(1),
                            packet: ClientboundPlayPacket::LevelEvent(packet),
                        } if packet.event_type == 2001 && packet.pos == NEAR => destroyed = true,
                        _ => {}
                    }
                }
                assert_eq!(world.block_state(DIM, FAR), Some(infested));
                assert_eq!(world.stored_entity_for_test(DIM, id).unwrap().uuid(), uuid);
                assert_eq!(health(world, id), after_magic);
                assert_no_items(world);
                if world.block_state(DIM, NEAR) == Some(expected) {
                    break;
                }
            }
            assert_eq!(world.block_state(DIM, NEAR), Some(expected));
            assert_eq!(destroyed, griefing);
            let ids = silverfish_ids(world);
            if griefing && drops {
                assert_eq!(ids.len(), 2);
                let new_id = *ids.iter().find(|&&other| other != id).unwrap();
                assert_eq!(added, vec![new_id]);
                assert!(animated.contains(&new_id));
            } else {
                assert_eq!(ids, vec![id]);
                assert!(added.is_empty());
                assert!(animated.is_empty());
            }
            assert_eq!(world.player_health_for_test(ConnectionId(1)), 20.0);
        });
    }
}

#[test]
fn silverfish_merges_into_one_real_axis_preserving_host_only_when_griefing_is_enabled() {
    with_world(|sender, receiver, world| {
        for text in [
            "fill 7 99 7 9 101 9 minecraft:deepslate[axis=x]",
            "setblock 8 100 8 minecraft:air",
        ] {
            command(sender, receiver, world, text);
        }
        let neighbors = [
            BlockPos { x: 7, y: 100, z: 8 },
            BlockPos { x: 9, y: 100, z: 8 },
            BlockPos { x: 8, y: 99, z: 8 },
            BlockPos { x: 8, y: 101, z: 8 },
            BlockPos { x: 8, y: 100, z: 7 },
            BlockPos { x: 8, y: 100, z: 9 },
        ];
        let host = axis_state("minecraft:deepslate");
        let infested = axis_state("minecraft:infested_deepslate");
        for pos in neighbors {
            assert_eq!(world.block_state(DIM, pos), Some(host));
        }
        let (id, uuid) = summon(sender, receiver, world);
        let initial_health = health(world, id);
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..150 {
            world.tick(receiver.drain());
            assert_eq!(world.stored_entity_for_test(DIM, id).unwrap().uuid(), uuid);
            assert_eq!(health(world, id), initial_health);
            for pos in neighbors {
                assert_eq!(world.block_state(DIM, pos), Some(host));
            }
        }
        world.tick_rate_mut().set_frozen(true);
        command(sender, receiver, world, "gamerule mob_griefing true");
        assert_eq!(
            world.game_rule_for_test("minecraft:mob_griefing"),
            Some(GameRuleValue::Boolean(true))
        );
        let mut animated = false;
        world.tick_rate_mut().set_frozen(false);
        for _ in 0..1000 {
            let tick = world.tick(receiver.drain());
            animated |= animation(&tick, id);
            assert_no_items(world);
            if world.live_entity_id_for_uuid_for_test(DIM, uuid).is_none() {
                break;
            }
        }
        assert!(
            world.live_entity_id_for_uuid_for_test(DIM, uuid).is_none(),
            "a targetless live silverfish enters its host"
        );
        assert!(silverfish_ids(world).is_empty());
        assert!(
            animated,
            "the tracking viewer receives the source's merge animation"
        );
        let mut changed = 0;
        for pos in neighbors {
            let state = world.block_state(DIM, pos).unwrap();
            assert!(state == host || state == infested);
            changed += usize::from(state == infested);
        }
        assert_eq!(changed, 1, "one host becomes infested and retains axis=x");
        assert_no_items(world);
    });
}
