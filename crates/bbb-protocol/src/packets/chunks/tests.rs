use super::*;
use crate::{
    codec::{Encoder, ProtocolError},
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_level_chunk_envelope() {
    let mut payload = Encoder::new();
    payload.write_i32(12);
    payload.write_i32(-4);
    write_level_chunk_data(&mut payload, &[1, 2, 3]);
    write_light_data(&mut payload, &[0b10], &[0b100], &[], &[], 1, 1);

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
            chunk_data: LevelChunkData {
                heightmaps: vec![ChunkHeightmapData {
                    kind_id: 1,
                    data: vec![7, 8],
                }],
                section_data: vec![1, 2, 3],
                block_entities: vec![LevelChunkBlockEntity {
                    packed_xz: 0xab,
                    y: 64,
                    block_entity_type_id: 5,
                    raw_nbt: vec![0],
                }],
            },
            light_data: LightUpdateData {
                sky_y_mask: vec![0b10],
                block_y_mask: vec![0b100],
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: vec![vec![1; 2048]],
                block_updates: vec![vec![2; 2048]],
            },
        })
    );
}

#[test]
fn decodes_light_update_envelope() {
    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    payload.write_var_i32(-4);
    write_light_data(&mut payload, &[0b10], &[0b100], &[0b1000], &[], 1, 1);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_LIGHT_UPDATE, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::LightUpdate(LightUpdate {
            chunk_x: 12,
            chunk_z: -4,
            light_data: LightUpdateData {
                sky_y_mask: vec![0b10],
                block_y_mask: vec![0b100],
                empty_sky_y_mask: vec![0b1000],
                empty_block_y_mask: Vec::new(),
                sky_updates: vec![vec![1; 2048]],
                block_updates: vec![vec![2; 2048]],
            },
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_LIGHT_UPDATE, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing 1 bytes after light update packet")
    );
}

#[test]
fn rejects_oversized_level_chunk_section_buffer() {
    let mut payload = Encoder::new();
    payload.write_i32(12);
    payload.write_i32(-4);
    payload.write_var_i32(0);
    payload.write_var_i32(2 * 1024 * 1024 + 1);

    let err = decode_play_clientbound(
        ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(err, ProtocolError::PacketTooLarge(_, 2_097_152)));
}

#[test]
fn rejects_light_update_arrays_that_are_not_data_layers() {
    let mut payload = Encoder::new();
    payload.write_var_i32(12);
    payload.write_var_i32(-4);
    write_long_array(&mut payload, &[0b10]);
    write_long_array(&mut payload, &[]);
    write_long_array(&mut payload, &[]);
    write_long_array(&mut payload, &[]);
    payload.write_var_i32(1);
    payload.write_var_i32(2);
    payload.write_bytes(&[1, 2]);
    payload.write_var_i32(0);

    let err = decode_play_clientbound(ids::play::CLIENTBOUND_LIGHT_UPDATE, &payload.into_inner())
        .unwrap_err();
    assert!(
        matches!(err, ProtocolError::InvalidData(message) if message.contains("expected 2048"))
    );
}

#[test]
fn rejects_impossible_level_chunk_catalog_and_collection_domains() {
    let mut too_many_heightmaps = Encoder::new();
    too_many_heightmaps.write_i32(0);
    too_many_heightmaps.write_i32(0);
    too_many_heightmaps
        .write_var_i32((crate::heightmap_types::VANILLA_HEIGHTMAP_TYPES.len() + 1) as i32);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
            &too_many_heightmaps.into_inner()
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == crate::heightmap_types::VANILLA_HEIGHTMAP_TYPES.len() + 1
                && max == crate::heightmap_types::VANILLA_HEIGHTMAP_TYPES.len()
    ));

    let mut unknown_heightmap = Encoder::new();
    unknown_heightmap.write_i32(0);
    unknown_heightmap.write_i32(0);
    unknown_heightmap.write_var_i32(1);
    unknown_heightmap.write_var_i32(crate::heightmap_types::VANILLA_HEIGHTMAP_TYPES.len() as i32);
    unknown_heightmap.write_var_i32(0);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
            &unknown_heightmap.into_inner()
        ),
        Err(ProtocolError::InvalidData(message))
            if message.contains("unknown heightmap type id")
    ));

    let mut too_many_block_entities = Encoder::new();
    too_many_block_entities.write_i32(0);
    too_many_block_entities.write_i32(0);
    too_many_block_entities.write_var_i32(0);
    too_many_block_entities.write_var_i32(0);
    too_many_block_entities.write_var_i32((MAX_LEVEL_CHUNK_BLOCK_ENTITIES + 1) as i32);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
            &too_many_block_entities.into_inner()
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_LEVEL_CHUNK_BLOCK_ENTITIES + 1
                && max == MAX_LEVEL_CHUNK_BLOCK_ENTITIES
    ));

    let mut unknown_block_entity_type = Encoder::new();
    unknown_block_entity_type.write_i32(0);
    unknown_block_entity_type.write_i32(0);
    unknown_block_entity_type.write_var_i32(0);
    unknown_block_entity_type.write_var_i32(0);
    unknown_block_entity_type.write_var_i32(1);
    unknown_block_entity_type.write_u8(0);
    unknown_block_entity_type.write_i16(0);
    unknown_block_entity_type
        .write_var_i32(crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES.len() as i32);
    unknown_block_entity_type.write_u8(0);
    write_light_data(&mut unknown_block_entity_type, &[], &[], &[], &[], 0, 0);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
            &unknown_block_entity_type.into_inner()
        ),
        Err(ProtocolError::InvalidData(message))
            if message.contains("unknown block entity type id")
    ));
}

#[test]
fn rejects_impossible_light_mask_and_data_layer_cardinality() {
    let mut oversized_mask = Encoder::new();
    oversized_mask.write_var_i32(0);
    oversized_mask.write_var_i32(0);
    write_long_array(&mut oversized_mask, &[0; MAX_LIGHT_MASK_LONGS + 1]);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LIGHT_UPDATE,
            &oversized_mask.into_inner()
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_LIGHT_MASK_LONGS + 1 && max == MAX_LIGHT_MASK_LONGS
    ));

    let mut too_many_layers = Encoder::new();
    too_many_layers.write_var_i32(0);
    too_many_layers.write_var_i32(0);
    for _ in 0..4 {
        write_long_array(&mut too_many_layers, &[]);
    }
    too_many_layers.write_var_i32((MAX_LIGHT_SECTIONS + 1) as i32);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LIGHT_UPDATE,
            &too_many_layers.into_inner()
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_LIGHT_SECTIONS + 1 && max == MAX_LIGHT_SECTIONS
    ));

    let mut missing_layer = Encoder::new();
    missing_layer.write_var_i32(0);
    missing_layer.write_var_i32(0);
    write_light_data(&mut missing_layer, &[1], &[], &[], &[], 0, 0);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LIGHT_UPDATE,
            &missing_layer.into_inner()
        ),
        Err(ProtocolError::InvalidData(message))
            if message.contains("sky light mask has 1 data sections but 0 arrays")
    ));

    let mut overlapping_masks = Encoder::new();
    overlapping_masks.write_var_i32(0);
    overlapping_masks.write_var_i32(0);
    write_light_data(&mut overlapping_masks, &[1], &[], &[1], &[], 1, 0);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_LIGHT_UPDATE,
            &overlapping_masks.into_inner()
        ),
        Err(ProtocolError::InvalidData(message))
            if message.contains("sky light data and empty masks overlap")
    ));
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

    let mut bytes = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_CHUNKS_BIOMES, &bytes).unwrap();
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

    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CHUNKS_BIOMES, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after chunks biomes packet")
    );

    let mut oversized = Encoder::new();
    oversized.write_var_i32((MAX_CHUNKS_BIOMES_UPDATES + 1) as i32);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_CHUNKS_BIOMES,
            &oversized.into_inner()
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_CHUNKS_BIOMES_UPDATES + 1 && max == MAX_CHUNKS_BIOMES_UPDATES
    ));
}

#[test]
fn decodes_block_ack_and_block_updates() {
    let mut payload = Encoder::new();
    payload.write_var_i32(17);
    let mut bytes = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK, &bytes).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::BlockChangedAck(BlockChangedAck { sequence: 17 })
    );
    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after block changed ack packet")
    );

    let mut payload = Encoder::new();
    payload.write_var_i32(1234);
    payload.write_i64(pack_block_pos(34, -12, -45));
    payload.write_u8(7);
    let mut bytes = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_DESTRUCTION, &bytes).unwrap();
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
    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_DESTRUCTION, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after block destruction packet")
    );

    let mut payload = Encoder::new();
    payload.write_i64(pack_block_pos(35, 64, -46));
    payload.write_u8(1);
    payload.write_u8(5);
    payload.write_var_i32(123);
    let mut bytes = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_EVENT, &bytes).unwrap();
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
    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_EVENT, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after block event packet")
    );
    for invalid_block_id in [
        -1,
        i32::try_from(crate::block_types::VANILLA_BLOCK_TYPES.len()).unwrap(),
    ] {
        let mut payload = Encoder::new();
        payload.write_i64(pack_block_pos(35, 64, -46));
        payload.write_u8(1);
        payload.write_u8(5);
        payload.write_var_i32(invalid_block_id);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_EVENT, &payload.into_inner())
                .unwrap_err()
                .to_string()
                .contains(&format!(
                    "unknown block registry id {invalid_block_id} in block event packet"
                ))
        );
    }

    let mut payload = Encoder::new();
    payload.write_i64(pack_block_pos(34, -12, -45));
    payload.write_var_i32(9);
    let mut bytes = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_UPDATE, &bytes).unwrap();
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
    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_UPDATE, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after block update packet")
    );

    for invalid_state_id in [-1, crate::block_state_types::VANILLA_BLOCK_STATE_COUNT] {
        let mut invalid = Encoder::new();
        invalid.write_i64(pack_block_pos(34, -12, -45));
        invalid.write_var_i32(invalid_state_id);
        assert!(decode_play_clientbound(
            ids::play::CLIENTBOUND_BLOCK_UPDATE,
            &invalid.into_inner(),
        )
        .unwrap_err()
        .to_string()
        .contains("unknown vanilla 26.1 block state id"));
    }

    let mut payload = Encoder::new();
    payload.write_i64(pack_section_pos(2, -1, -3));
    payload.write_var_i32(2);
    payload.write_var_i64((9 << 12) | section_relative_pos(2, 1, 3));
    payload.write_var_i64(section_relative_pos(15, 15, 15));
    let mut bytes = payload.into_inner();
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE, &bytes).unwrap();
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

    bytes.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE, &bytes)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after section blocks update packet")
    );

    let mut oversized = Encoder::new();
    oversized.write_i64(pack_section_pos(2, -1, -3));
    oversized.write_var_i32((MAX_SECTION_BLOCK_UPDATES + 1) as i32);
    assert!(matches!(
        decode_play_clientbound(
            ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE,
            &oversized.into_inner()
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_SECTION_BLOCK_UPDATES + 1 && max == MAX_SECTION_BLOCK_UPDATES
    ));

    for invalid_state_id in [
        -1_i64,
        i64::from(crate::block_state_types::VANILLA_BLOCK_STATE_COUNT),
    ] {
        let mut invalid = Encoder::new();
        invalid.write_i64(pack_section_pos(2, -1, -3));
        invalid.write_var_i32(1);
        invalid.write_var_i64((invalid_state_id << 12) | i64::from(section_relative_pos(2, 1, 3)));
        assert!(decode_play_clientbound(
            ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE,
            &invalid.into_inner(),
        )
        .unwrap_err()
        .to_string()
        .contains("unknown vanilla 26.1 block state id"));
    }
}

#[test]
fn decodes_block_entity_data_update() {
    let raw_nbt = nbt_compound_with_string("id", "minecraft:chest");
    let mut payload = Encoder::new();
    payload.write_i64(pack_block_pos(34, 64, -45));
    payload.write_var_i32(crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID);
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
            block_entity_type_id: crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
            raw_nbt,
        })
    );
}

#[test]
fn block_entity_data_requires_known_type_and_terminal_compound_nbt() {
    for block_entity_type in crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES {
        let payload = block_entity_data_payload(block_entity_type.id, &[10, 0]);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA, &payload).unwrap();
        let PlayClientbound::BlockEntityData(packet) = packet else {
            panic!("expected block entity data");
        };
        assert_eq!(packet.block_entity_type_id, block_entity_type.id);
        assert_eq!(packet.raw_nbt, vec![10, 0]);
    }

    for invalid_type_id in [
        -1,
        i32::try_from(crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES.len()).unwrap(),
    ] {
        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
            &block_entity_data_payload(invalid_type_id, &[10, 0]),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&format!("unknown block entity type id {invalid_type_id}")),
            "{error}"
        );
    }

    for invalid_root in [vec![0], vec![1, 0], vec![9, 0, 0, 0, 0, 0]] {
        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
            &block_entity_data_payload(
                crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
                &invalid_root,
            ),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must be a non-null compound tag"),
            "{error}"
        );
    }

    let error = decode_play_clientbound(
        ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
        &block_entity_data_payload(
            crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
            &[10, 0, 0],
        ),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("trailing bytes after block entity data packet"),
        "{error}"
    );
}

#[test]
fn block_entity_nbt_rejects_malformed_and_oversized_structures() {
    let decode = |nbt: &[u8]| {
        decode_play_clientbound(
            ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
            &block_entity_data_payload(
                crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
                nbt,
            ),
        )
    };

    let mut legal_depth = nested_compound_nbt(MAX_NBT_DEPTH);
    decode(&legal_depth).unwrap();
    legal_depth = nested_compound_nbt(MAX_NBT_DEPTH + 1);
    assert!(decode(&legal_depth)
        .unwrap_err()
        .to_string()
        .contains("exceeded max depth"));

    let legal_entries = compound_with_byte_entries(MAX_NBT_COMPOUND_ENTRIES);
    decode(&legal_entries).unwrap();
    let oversized_entries = compound_with_byte_entries(MAX_NBT_COMPOUND_ENTRIES + 1);
    assert!(matches!(
        decode(&oversized_entries),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_NBT_COMPOUND_ENTRIES + 1 && max == MAX_NBT_COMPOUND_ENTRIES
    ));

    for tag_id in [7, 11, 12] {
        let oversized_array = compound_with_sized_collection(tag_id, MAX_NBT_ARRAY_ITEMS + 1);
        assert!(matches!(
            decode(&oversized_array),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_NBT_ARRAY_ITEMS + 1 && max == MAX_NBT_ARRAY_ITEMS
        ));
    }
    let oversized_list = compound_with_sized_collection(9, MAX_NBT_LIST_ITEMS + 1);
    assert!(matches!(
        decode(&oversized_list),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_NBT_LIST_ITEMS + 1 && max == MAX_NBT_LIST_ITEMS
    ));

    assert!(matches!(
        decode(&[10, 1, 0, 0]),
        Err(ProtocolError::UnexpectedEof { .. })
    ));
    assert!(decode(&[10, 1, 0, 1, 0xff, 0, 0])
        .unwrap_err()
        .to_string()
        .contains("invalid modified utf-8 leading byte"));
    assert!(decode(&[10, 8, 0, 0, 0, 1, 0xff, 0])
        .unwrap_err()
        .to_string()
        .contains("invalid modified utf-8 leading byte"));
}

#[test]
fn level_chunk_block_entity_nbt_is_nullable_but_other_roots_are_invalid() {
    let mut payload = Encoder::new();
    payload.write_i32(12);
    payload.write_i32(-4);
    write_level_chunk_data_with_nbt(&mut payload, &[1, 2, 3], &[1, 0]);
    write_light_data(&mut payload, &[0b10], &[0b100], &[], &[], 1, 1);
    let error = decode_play_clientbound(
        ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("must be a compound tag or null"),
        "{error}"
    );
}

#[test]
fn decodes_forget_level_chunk() {
    let mut payload = Encoder::new();
    payload.write_i64(pack_chunk_pos(12, -4));

    let payload = payload.into_inner();
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ForgetLevelChunk(ForgetLevelChunk {
            pos: ChunkPos { x: 12, z: -4 },
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after forget level chunk packet")
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

    let mut trailing = Encoder::new();
    trailing.write_var_i32(12);
    trailing.write_var_i32(-4);
    trailing.write_u8(0);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER,
        &trailing.into_inner()
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after set chunk cache center packet"));

    let mut trailing = Encoder::new();
    trailing.write_var_i32(10);
    trailing.write_u8(0);
    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS,
        &trailing.into_inner()
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after set chunk cache radius packet"));
}

#[test]
fn decodes_level_event() {
    let mut payload = Encoder::new();
    payload.write_i32(2001);
    payload.write_i64(pack_block_pos(34, 64, -45));
    payload.write_i32(9);
    payload.write_bool(true);
    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_LEVEL_EVENT, &payload).unwrap();
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
    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_LEVEL_EVENT, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after level event packet")
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

fn block_entity_data_payload(block_entity_type_id: i32, raw_nbt: &[u8]) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_i64(pack_block_pos(34, 64, -45));
    payload.write_var_i32(block_entity_type_id);
    payload.write_bytes(raw_nbt);
    payload.into_inner()
}

fn nested_compound_nbt(nested_count: usize) -> Vec<u8> {
    let mut nbt = Vec::with_capacity(1 + nested_count * 3 + nested_count + 1);
    nbt.push(10);
    for _ in 0..nested_count {
        nbt.extend_from_slice(&[10, 0, 0]);
    }
    nbt.resize(nbt.len() + nested_count + 1, 0);
    nbt
}

fn compound_with_byte_entries(count: usize) -> Vec<u8> {
    let mut nbt = Vec::with_capacity(2 + count * 4);
    nbt.push(10);
    for _ in 0..count {
        nbt.extend_from_slice(&[1, 0, 0, 0]);
    }
    nbt.push(0);
    nbt
}

fn compound_with_sized_collection(tag_id: u8, count: usize) -> Vec<u8> {
    let mut nbt = vec![10, tag_id, 0, 0];
    if tag_id == 9 {
        nbt.push(1);
    }
    nbt.extend_from_slice(&(count as i32).to_be_bytes());
    nbt.push(0);
    nbt
}

fn section_relative_pos(x: i64, y: i64, z: i64) -> i64 {
    (x << 8) | (z << 4) | y
}

fn write_level_chunk_data(out: &mut Encoder, section_data: &[u8]) {
    write_level_chunk_data_with_nbt(out, section_data, &[0]);
}

fn write_level_chunk_data_with_nbt(out: &mut Encoder, section_data: &[u8], raw_nbt: &[u8]) {
    out.write_var_i32(1);
    out.write_var_i32(1);
    write_long_array(out, &[7, 8]);
    out.write_var_i32(section_data.len() as i32);
    out.write_bytes(section_data);
    out.write_var_i32(1);
    out.write_u8(0xab);
    out.write_i16(64);
    out.write_var_i32(5);
    out.write_bytes(raw_nbt);
}

fn write_light_data(
    out: &mut Encoder,
    sky_y_mask: &[i64],
    block_y_mask: &[i64],
    empty_sky_y_mask: &[i64],
    empty_block_y_mask: &[i64],
    sky_updates: usize,
    block_updates: usize,
) {
    write_long_array(out, sky_y_mask);
    write_long_array(out, block_y_mask);
    write_long_array(out, empty_sky_y_mask);
    write_long_array(out, empty_block_y_mask);
    write_light_arrays(out, sky_updates, 1);
    write_light_arrays(out, block_updates, 2);
}

fn write_long_array(out: &mut Encoder, values: &[i64]) {
    out.write_var_i32(values.len() as i32);
    for value in values {
        out.write_i64(*value);
    }
}

fn write_light_arrays(out: &mut Encoder, count: usize, fill: u8) {
    out.write_var_i32(count as i32);
    for _ in 0..count {
        out.write_var_i32(2048);
        out.write_bytes(&vec![fill; 2048]);
    }
}
