use super::*;
use crate::{
    codec::Encoder,
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_level_chunk_envelope() {
    let mut payload = Encoder::new();
    payload.write_i32(12);
    payload.write_i32(-4);
    payload.write_bytes(&[1, 2, 3]);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::LevelChunkWithLight(LevelChunkWithLight {
            x: 12,
            z: -4,
            raw_after_position: vec![1, 2, 3],
        })
    );
}

#[test]
fn decodes_light_update_envelope() {
    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    payload.write_var_i32(-4);
    payload.write_bytes(&[9, 8, 7]);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_LIGHT_UPDATE, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::LightUpdate(LightUpdate {
            chunk_x: 12,
            chunk_z: -4,
            raw_light_data: vec![9, 8, 7],
        })
    );
}

#[test]
fn decodes_chunks_biomes_update() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_i64(pack_chunk_pos(12, -4));
    payload.write_var_i32(3);
    payload.write_bytes(&[1, 2, 3]);
    payload.write_i64(pack_chunk_pos(-8, 5));
    payload.write_var_i32(2);
    payload.write_bytes(&[4, 5]);

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_CHUNKS_BIOMES, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ChunksBiomes(ChunksBiomes {
            chunks: vec![
                ChunkBiomeData {
                    pos: ChunkPos { x: 12, z: -4 },
                    raw_biomes: vec![1, 2, 3],
                },
                ChunkBiomeData {
                    pos: ChunkPos { x: -8, z: 5 },
                    raw_biomes: vec![4, 5],
                },
            ],
        })
    );
}

#[test]
fn decodes_block_ack_and_block_updates() {
    let mut payload = Encoder::new();
    payload.write_var_i32(17);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BlockChangedAck(BlockChangedAck { sequence: 17 })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(1234);
    payload.write_i64(pack_block_pos(34, -12, -45));
    payload.write_u8(7);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_BLOCK_DESTRUCTION,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BlockDestruction(BlockDestruction {
            id: 1234,
            pos: BlockPos {
                x: 34,
                y: -12,
                z: -45,
            },
            progress: 7,
        })
    );

    let mut payload = Encoder::new();
    payload.write_i64(pack_block_pos(35, 64, -46));
    payload.write_u8(1);
    payload.write_u8(5);
    payload.write_var_i32(123);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_EVENT, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BlockEvent(BlockEvent {
            pos: BlockPos {
                x: 35,
                y: 64,
                z: -46,
            },
            b0: 1,
            b1: 5,
            block_id: 123,
        })
    );

    let mut payload = Encoder::new();
    payload.write_i64(pack_block_pos(34, -12, -45));
    payload.write_var_i32(9);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_UPDATE, &payload.into_inner())
            .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BlockUpdate(BlockUpdate {
            pos: BlockPos {
                x: 34,
                y: -12,
                z: -45,
            },
            block_state_id: 9,
        })
    );

    let mut payload = Encoder::new();
    payload.write_i64(pack_section_pos(2, -1, -3));
    payload.write_var_i32(2);
    payload.write_var_i64((9 << 12) | section_relative_pos(2, 1, 3));
    payload.write_var_i64(section_relative_pos(15, 15, 15));
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SectionBlocksUpdate(SectionBlocksUpdate {
            section_x: 2,
            section_y: -1,
            section_z: -3,
            updates: vec![
                BlockUpdate {
                    pos: BlockPos {
                        x: 34,
                        y: -15,
                        z: -45,
                    },
                    block_state_id: 9,
                },
                BlockUpdate {
                    pos: BlockPos {
                        x: 47,
                        y: -1,
                        z: -33,
                    },
                    block_state_id: 0,
                },
            ],
        })
    );
}

#[test]
fn decodes_block_entity_data_update() {
    let raw_nbt = nbt_compound_with_string("id", "minecraft:chest");
    let mut payload = Encoder::new();
    payload.write_i64(pack_block_pos(34, 64, -45));
    payload.write_var_i32(5);
    payload.write_bytes(&raw_nbt);

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BlockEntityData(BlockEntityData {
            pos: BlockPos {
                x: 34,
                y: 64,
                z: -45,
            },
            block_entity_type_id: 5,
            raw_nbt,
        })
    );
}

#[test]
fn decodes_forget_level_chunk() {
    let mut payload = Encoder::new();
    payload.write_i64(pack_chunk_pos(12, -4));

    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ForgetLevelChunk(ForgetLevelChunk {
            pos: ChunkPos { x: 12, z: -4 },
        })
    );
}

#[test]
fn decodes_chunk_cache_center_and_radius() {
    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    payload.write_var_i32(-4);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetChunkCacheCenter(SetChunkCacheCenter {
            chunk_x: 12,
            chunk_z: -4,
        })
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(10);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SetChunkCacheRadius(SetChunkCacheRadius { radius: 10 })
    );
}

#[test]
fn decodes_level_event() {
    let mut payload = Encoder::new();
    payload.write_i32(2001);
    payload.write_i64(pack_block_pos(34, 64, -45));
    payload.write_i32(9);
    payload.write_bool(true);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_LEVEL_EVENT, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::LevelEvent(LevelEvent {
            event_type: 2001,
            pos: BlockPos {
                x: 34,
                y: 64,
                z: -45,
            },
            data: 9,
            global: true,
        })
    );
}

fn pack_block_pos(x: i32, y: i32, z: i32) -> i64 {
    (((x as i64) & 0x3ffffff) << 38) | (((z as i64) & 0x3ffffff) << 12) | ((y as i64) & 0xfff)
}

fn pack_section_pos(x: i32, y: i32, z: i32) -> i64 {
    (((x as i64) & 0x3fffff) << 42) | (((z as i64) & 0x3fffff) << 20) | ((y as i64) & 0xfffff)
}

fn pack_chunk_pos(x: i32, z: i32) -> i64 {
    (((x as u32) as u64) | (((z as u32) as u64) << 32)) as i64
}

fn nbt_compound_with_string(name: &str, value: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
    payload.extend_from_slice(value.as_bytes());
    payload.push(0);
    payload
}

fn section_relative_pos(x: i64, y: i64, z: i64) -> i64 {
    (x << 8) | (z << 4) | y
}
