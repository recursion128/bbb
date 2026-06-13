use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_debug_value_packets_with_raw_subscription_payloads() {
    let mut block_payload = Encoder::new();
    block_payload.write_i64(chunks::encode_block_pos(BlockPos { x: 1, y: 64, z: -2 }));
    block_payload.write_var_i32(5);
    block_payload.write_bool(true);
    block_payload.write_u8(0xaa);
    let block_payload = block_payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE, &block_payload).unwrap(),
        PlayClientbound::DebugBlockValue(DebugBlockValue {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload: vec![5, 1, 0xaa],
        })
    );

    let mut chunk_payload = Encoder::new();
    chunk_payload.write_i64(3i64 | ((-4i64) << 32));
    chunk_payload.write_var_i32(7);
    chunk_payload.write_bool(false);
    let chunk_payload = chunk_payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_CHUNK_VALUE, &chunk_payload).unwrap(),
        PlayClientbound::DebugChunkValue(DebugChunkValue {
            pos: ChunkPos { x: 3, z: -4 },
            raw_update_payload: vec![7, 0],
        })
    );

    let mut entity_payload = Encoder::new();
    entity_payload.write_var_i32(123);
    entity_payload.write_var_i32(9);
    entity_payload.write_bool(true);
    entity_payload.write_u8(0xbb);
    let entity_payload = entity_payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_ENTITY_VALUE, &entity_payload)
            .unwrap(),
        PlayClientbound::DebugEntityValue(DebugEntityValue {
            entity_id: 123,
            raw_update_payload: vec![9, 1, 0xbb],
        })
    );
}

#[test]
fn decodes_debug_event_raw_payload() {
    let mut payload = Encoder::new();
    payload.write_var_i32(4);
    payload.write_u8(0xcc);
    let payload = payload.into_inner();

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_DEBUG_EVENT, &payload).unwrap(),
        PlayClientbound::DebugEvent(DebugEvent {
            raw_event_payload: vec![4, 0xcc],
        })
    );
}

#[test]
fn decodes_debug_sample_packet_wire_order() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_i64(100);
    payload.write_i64(-50);
    payload.write_var_i32(0);
    let payload = payload.into_inner();

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
    let payload = payload.into_inner();

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
    let payload = payload.into_inner();

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
}
