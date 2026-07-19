use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_debug_value_packets_with_typed_subscription_updates() {
    let mut block_payload = Encoder::new();
    block_payload.write_i64(chunks::encode_block_pos(BlockPos { x: 1, y: 64, z: -2 }));
    block_payload.write_var_i32(6);
    block_payload.write_bool(true);
    block_payload.write_var_i32(1);
    let block_payload = block_payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE, &block_payload).unwrap(),
        PlayClientbound::DebugBlockValue(DebugBlockValue {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            update: DebugSubscriptionUpdate {
                subscription_id: 6,
                value: Some(DebugSubscriptionValue::EntityBlockIntersection(
                    DebugEntityBlockIntersection::InFluid,
                )),
            },
        })
    );

    let mut chunk_payload = Encoder::new();
    chunk_payload.write_i64(3i64 | ((-4i64) << 32));
    chunk_payload.write_var_i32(11);
    chunk_payload.write_bool(false);
    let chunk_payload = chunk_payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_CHUNK_VALUE, &chunk_payload).unwrap(),
        PlayClientbound::DebugChunkValue(DebugChunkValue {
            pos: ChunkPos { x: 3, z: -4 },
            update: DebugSubscriptionUpdate {
                subscription_id: 11,
                value: None,
            },
        })
    );

    let mut entity_payload = Encoder::new();
    entity_payload.write_var_i32(123);
    entity_payload.write_var_i32(3);
    entity_payload.write_bool(true);
    entity_payload.write_bool(true);
    entity_payload.write_var_i32(77);
    entity_payload.write_bool(true);
    entity_payload.write_i64(chunks::encode_block_pos(BlockPos { x: 5, y: 80, z: 9 }));
    let entity_payload = entity_payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_ENTITY_VALUE, &entity_payload)
            .unwrap(),
        PlayClientbound::DebugEntityValue(DebugEntityValue {
            entity_id: 123,
            update: DebugSubscriptionUpdate {
                subscription_id: 3,
                value: Some(DebugSubscriptionValue::Breeze(DebugBreezeInfo {
                    attack_target: Some(77),
                    jump_target: Some(BlockPos { x: 5, y: 80, z: 9 }),
                })),
            },
        })
    );
}

#[test]
fn decodes_debug_event_typed_payload() {
    let mut payload = Encoder::new();
    payload.write_var_i32(15);
    payload.write_var_i32(2);
    payload.write_f64(1.5);
    payload.write_f64(-2.0);
    payload.write_f64(3.25);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_EVENT, &payload).unwrap(),
        PlayClientbound::DebugEvent(DebugEvent {
            event: DebugSubscriptionEvent {
                subscription_id: 15,
                value: DebugSubscriptionValue::GameEvent(DebugGameEventInfo {
                    game_event_holder_id: 2,
                    pos: Vec3d {
                        x: 1.5,
                        y: -2.0,
                        z: 3.25,
                    },
                }),
            },
        })
    );
}

#[test]
fn decodes_bee_brain_goal_and_poi_values() {
    let mut bee = Encoder::new();
    bee.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
    bee.write_var_i32(1);
    bee.write_bool(true);
    bee.write_bool(true);
    bee.write_i64(chunks::encode_block_pos(BlockPos { x: 1, y: 2, z: 3 }));
    bee.write_bool(false);
    bee.write_var_i32(9);
    bee.write_var_i32(1);
    bee.write_i64(chunks::encode_block_pos(BlockPos { x: 4, y: 5, z: 6 }));
    assert!(matches!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE, &bee.into_inner())
            .unwrap(),
        PlayClientbound::DebugBlockValue(DebugBlockValue {
            update: DebugSubscriptionUpdate {
                value: Some(DebugSubscriptionValue::Bee(DebugBeeInfo {
                    hive_pos: Some(BlockPos { x: 1, y: 2, z: 3 }),
                    flower_pos: None,
                    travel_ticks: 9,
                    blacklisted_hives,
                })),
                ..
            },
            ..
        }) if blacklisted_hives == [BlockPos { x: 4, y: 5, z: 6 }]
    ));

    let mut brain = Encoder::new();
    brain.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
    brain.write_var_i32(2);
    brain.write_bool(true);
    brain.write_string("Bee");
    brain.write_string("minecraft:farmer");
    brain.write_i32(12);
    brain.write_f32(8.0);
    brain.write_f32(10.0);
    brain.write_string("[]");
    brain.write_bool(true);
    brain.write_i32(4);
    for text in ["work", "walk", "home", "friend"] {
        brain.write_var_i32(1);
        brain.write_string(text);
    }
    brain.write_var_i32(1);
    brain.write_i64(chunks::encode_block_pos(BlockPos { x: 7, y: 8, z: 9 }));
    brain.write_var_i32(0);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE,
        &brain.into_inner(),
    )
    .unwrap();
    let PlayClientbound::DebugBlockValue(DebugBlockValue {
        update:
            DebugSubscriptionUpdate {
                value: Some(DebugSubscriptionValue::Brain(brain)),
                ..
            },
        ..
    }) = packet
    else {
        panic!("expected typed brain update");
    };
    assert_eq!(brain.name, "Bee");
    assert_eq!(brain.activities, ["work"]);
    assert_eq!(brain.pois, [BlockPos { x: 7, y: 8, z: 9 }]);

    let mut goals = Encoder::new();
    goals.write_var_i32(10);
    goals.write_var_i32(4);
    goals.write_bool(true);
    goals.write_var_i32(1);
    goals.write_var_i32(3);
    goals.write_bool(true);
    goals.write_string("SwimGoal");
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_DEBUG_ENTITY_VALUE,
        &goals.into_inner(),
    )
    .unwrap();
    assert!(matches!(
        packet,
        PlayClientbound::DebugEntityValue(DebugEntityValue {
            update: DebugSubscriptionUpdate {
                value: Some(DebugSubscriptionValue::Goals(DebugGoalInfo { goals })),
                ..
            },
            ..
        }) if goals == [DebugGoal { priority: 3, is_running: true, name: "SwimGoal".to_string() }]
    ));

    let mut poi = Encoder::new();
    poi.write_i64(chunks::encode_block_pos(BlockPos { x: 2, y: 3, z: 4 }));
    poi.write_var_i32(8);
    poi.write_bool(true);
    poi.write_i64(chunks::encode_block_pos(BlockPos { x: 5, y: 6, z: 7 }));
    poi.write_var_i32(11);
    poi.write_var_i32(2);
    assert!(matches!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE, &poi.into_inner())
            .unwrap(),
        PlayClientbound::DebugBlockValue(DebugBlockValue {
            update: DebugSubscriptionUpdate {
                value: Some(DebugSubscriptionValue::Poi(DebugPoiInfo {
                    pos: BlockPos { x: 5, y: 6, z: 7 },
                    poi_type_holder_id: 11,
                    free_ticket_count: 2,
                })),
                ..
            },
            ..
        })
    ));
}

#[test]
fn decodes_path_hive_orientation_and_collection_values() {
    let path = decode_block_update_value(5, |payload| {
        payload.write_bool(true);
        payload.write_i32(1);
        payload.write_i64(chunks::encode_block_pos(BlockPos { x: 9, y: 70, z: -3 }));
        payload.write_var_i32(1);
        encode_path_node(payload, 1, 2, 3, 2);
        payload.write_var_i32(1);
        encode_path_node(payload, 4, 5, 6, 9);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        encode_path_node(payload, 7, 8, 9, 0);
        payload.write_f32(0.75);
    });
    let DebugSubscriptionValue::Path(path) = path else {
        panic!("expected path value");
    };
    assert!(path.path.reached);
    assert_eq!(path.path.next_node_index, 1);
    assert_eq!(path.path.nodes[0].path_type, DebugPathType::Walkable);
    assert_eq!(path.path.target_nodes[0].path_type, DebugPathType::Water);
    assert_eq!(path.path.closed_set[0].path_type, DebugPathType::Blocked);
    assert_eq!(path.max_node_distance, 0.75);

    assert_eq!(
        decode_block_update_value(7, |payload| {
            payload.write_var_i32(12);
            payload.write_var_i32(3);
            payload.write_var_i32(5);
            payload.write_bool(true);
        }),
        DebugSubscriptionValue::Hive(DebugHiveInfo {
            block_registry_id: 12,
            occupant_count: 3,
            honey_level: 5,
            sedated: true,
        })
    );
    assert_eq!(
        decode_block_update_value(9, |payload| payload.write_var_i32(47)),
        DebugSubscriptionValue::RedstoneOrientation(DebugRedstoneOrientation { index: 47 })
    );
    assert_eq!(
        decode_block_update_value(10, |_| {}),
        DebugSubscriptionValue::VillageSection
    );
    assert_eq!(
        decode_block_update_value(11, |payload| {
            payload.write_var_i32(2);
            payload.write_i64(chunks::encode_block_pos(BlockPos { x: 1, y: 2, z: 3 }));
            payload.write_i64(chunks::encode_block_pos(BlockPos { x: 4, y: 5, z: 6 }));
        }),
        DebugSubscriptionValue::Raids(vec![
            BlockPos { x: 1, y: 2, z: 3 },
            BlockPos { x: 4, y: 5, z: 6 },
        ])
    );
    assert_eq!(
        decode_block_update_value(13, |payload| payload.write_var_i32(16)),
        DebugSubscriptionValue::GameEventListener(DebugGameEventListenerInfo {
            listener_radius: 16,
        })
    );
}

#[test]
fn decodes_structure_and_neighbor_event_values() {
    let structures = decode_block_update_value(12, |payload| {
        payload.write_var_i32(1);
        encode_bounding_box(
            payload,
            BlockPos {
                x: 10,
                y: 20,
                z: 30,
            },
            BlockPos { x: 1, y: 2, z: 3 },
        );
        payload.write_var_i32(1);
        encode_bounding_box(
            payload,
            BlockPos { x: 4, y: 5, z: 6 },
            BlockPos { x: 7, y: 8, z: 9 },
        );
        payload.write_bool(true);
    });
    assert_eq!(
        structures,
        DebugSubscriptionValue::Structures(vec![DebugStructureInfo {
            bounding_box: DebugBoundingBox {
                min: BlockPos { x: 1, y: 2, z: 3 },
                max: BlockPos {
                    x: 10,
                    y: 20,
                    z: 30
                },
            },
            pieces: vec![DebugStructurePiece {
                bounding_box: DebugBoundingBox {
                    min: BlockPos { x: 4, y: 5, z: 6 },
                    max: BlockPos { x: 7, y: 8, z: 9 },
                },
                is_start: true,
            }],
        }])
    );

    let mut event = Encoder::new();
    event.write_var_i32(14);
    event.write_i64(chunks::encode_block_pos(BlockPos { x: -4, y: 63, z: 8 }));
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_EVENT, &event.into_inner()).unwrap(),
        PlayClientbound::DebugEvent(DebugEvent {
            event: DebugSubscriptionEvent {
                subscription_id: 14,
                value: DebugSubscriptionValue::NeighborUpdate(BlockPos { x: -4, y: 63, z: 8 }),
            },
        })
    );
}

#[test]
fn rejects_invalid_debug_subscription_dispatch_and_nested_values() {
    for (id, expected) in [
        (0, "has no value codec"),
        (16, "invalid debug subscription id"),
        (-1, "invalid debug subscription id"),
    ] {
        let mut payload = Encoder::new();
        payload.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
        payload.write_var_i32(id);
        payload.write_bool(false);
        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE,
            &payload.into_inner(),
        )
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }

    let mut orientation = Encoder::new();
    orientation.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
    orientation.write_var_i32(9);
    orientation.write_bool(true);
    orientation.write_var_i32(48);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE,
        &orientation.into_inner()
    )
    .unwrap_err()
    .to_string()
    .contains("invalid redstone orientation index"));

    let mut poi = Encoder::new();
    poi.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
    poi.write_var_i32(8);
    poi.write_bool(true);
    poi.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
    poi.write_var_i32(21);
    poi.write_var_i32(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE, &poi.into_inner())
            .unwrap_err()
            .to_string()
            .contains("unknown point of interest type registry id 21")
    );

    let mut game_event = Encoder::new();
    game_event.write_var_i32(15);
    game_event.write_var_i32(60);
    game_event.write_f64(0.0);
    game_event.write_f64(0.0);
    game_event.write_f64(0.0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_EVENT, &game_event.into_inner())
            .unwrap_err()
            .to_string()
            .contains("unknown game event registry id 60")
    );

    let mut oversized = Encoder::new();
    oversized.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
    oversized.write_var_i32(11);
    oversized.write_bool(true);
    oversized.write_var_i32(65_537);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE,
            &oversized.into_inner()
        ),
        Err(crate::codec::ProtocolError::PacketTooLarge(65_537, 65_536))
    ));

    let oversized_packet = vec![0; MAX_DEBUG_PAYLOAD + 1];
    let mut decoder = Decoder::new(&oversized_packet);
    assert!(matches!(
        decode_debug_block_value(&mut decoder),
        Err(crate::codec::ProtocolError::PacketTooLarge(
            size,
            MAX_DEBUG_PAYLOAD
        )) if size == MAX_DEBUG_PAYLOAD + 1
    ));
}

fn decode_block_update_value(
    subscription_id: i32,
    encode_value: impl FnOnce(&mut Encoder),
) -> DebugSubscriptionValue {
    let mut payload = Encoder::new();
    payload.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
    payload.write_var_i32(subscription_id);
    payload.write_bool(true);
    encode_value(&mut payload);
    let PlayClientbound::DebugBlockValue(DebugBlockValue {
        update: DebugSubscriptionUpdate {
            value: Some(value), ..
        },
        ..
    }) = decode_play_clientbound(
        ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE,
        &payload.into_inner(),
    )
    .unwrap()
    else {
        panic!("expected typed block debug value");
    };
    value
}

fn encode_path_node(payload: &mut Encoder, x: i32, y: i32, z: i32, path_type_ordinal: i32) {
    payload.write_i32(x);
    payload.write_i32(y);
    payload.write_i32(z);
    payload.write_f32(1.25);
    payload.write_f32(2.5);
    payload.write_bool(false);
    payload.write_var_i32(path_type_ordinal);
    payload.write_f32(3.75);
}

fn encode_bounding_box(payload: &mut Encoder, first: BlockPos, second: BlockPos) {
    payload.write_i64(chunks::encode_block_pos(first));
    payload.write_i64(chunks::encode_block_pos(second));
}

#[test]
fn decodes_debug_sample_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_i64(100);
    payload.write_i64(-50);
    payload.write_var_i32(0);
    let mut payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_SAMPLE, &payload).unwrap(),
        PlayClientbound::DebugSample(DebugSample {
            sample: vec![100, -50],
            sample_type: RemoteDebugSampleType::TickTime,
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_i64().unwrap(), 100);
    assert_eq!(decoder.read_i64().unwrap(), -50);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    payload.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_SAMPLE, &payload)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after debug sample packet")
    );
}

#[test]
fn rejects_unknown_debug_sample_type() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_var_i32(1);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_SAMPLE, &payload.into_inner())
        .unwrap_err();
    assert!(err
        .to_string()
        .contains("invalid remote debug sample type ordinal 1"));
}

#[test]
fn decodes_game_rule_values_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_string("minecraft:do_daylight_cycle");
    payload.write_string("false");
    payload.write_string("minecraft:random_tick_speed");
    payload.write_string("3");
    let mut payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_GAME_RULE_VALUES, &payload).unwrap(),
        PlayClientbound::GameRuleValues(GameRuleValues {
            values: vec![
                GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        })
    );

    payload.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_GAME_RULE_VALUES, &payload)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after game rule values packet")
    );
}

#[test]
fn decodes_game_rule_values_with_default_namespace() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_string("do_daylight_cycle");
    payload.write_string("false");
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_GAME_RULE_VALUES, &payload).unwrap(),
        PlayClientbound::GameRuleValues(GameRuleValues {
            values: vec![GameRuleValue {
                rule: "minecraft:do_daylight_cycle".to_string(),
                value: "false".to_string(),
            }],
        })
    );
}

#[test]
fn rejects_invalid_game_rule_id() {
    let mut payload = Encoder::new();
    payload.write_var_i32(1);
    payload.write_string("minecraft:DoDaylightCycle");
    payload.write_string("false");
    let payload = payload.into_inner();

    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_GAME_RULE_VALUES, &payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_game_test_highlight_positions() {
    let mut payload = Encoder::new();
    payload.write_i64(chunks::encode_block_pos(BlockPos {
        x: -10,
        y: 70,
        z: 22,
    }));
    payload.write_i64(chunks::encode_block_pos(BlockPos { x: 1, y: 2, z: 3 }));
    let mut payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS, &payload).unwrap(),
        PlayClientbound::GameTestHighlightPos(GameTestHighlightPos {
            absolute_pos: BlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: BlockPos { x: 1, y: 2, z: 3 },
        })
    );

    payload.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS, &payload)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after game test highlight pos packet")
    );
}

#[test]
fn decodes_test_instance_block_status_with_optional_size() {
    let mut payload = Encoder::new();
    payload.write_u8(8);
    payload.write_u16(5);
    payload.write_bytes(b"Ready");
    payload.write_bool(true);
    payload.write_var_i32(3);
    payload.write_var_i32(4);
    payload.write_var_i32(5);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS, &payload)
            .unwrap(),
        PlayClientbound::TestInstanceBlockStatus(TestInstanceBlockStatus {
            status: "Ready".to_string(),
            size: Some(Vec3i { x: 3, y: 4, z: 5 }),
        })
    );

    let mut no_size = Encoder::new();
    no_size.write_u8(8);
    no_size.write_u16(4);
    no_size.write_bytes(b"Idle");
    no_size.write_bool(false);

    assert_eq!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS,
            &no_size.into_inner()
        )
        .unwrap(),
        PlayClientbound::TestInstanceBlockStatus(TestInstanceBlockStatus {
            status: "Idle".to_string(),
            size: None,
        })
    );

    let mut trailing = Encoder::new();
    trailing.write_u8(8);
    trailing.write_u16(4);
    trailing.write_bytes(b"Idle");
    trailing.write_bool(false);
    trailing.write_u8(0);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS,
        &trailing.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after test instance block status packet"));
}
