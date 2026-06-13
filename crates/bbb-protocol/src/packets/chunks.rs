use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};

use super::BlockHitResult;

const MAX_CHUNKS_BIOMES_BUFFER: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockUpdate {
    pub pos: BlockPos,
    pub block_state_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEntityData {
    pub pos: BlockPos,
    pub block_entity_type_id: i32,
    pub raw_nbt: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockChangedAck {
    pub sequence: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestruction {
    pub id: i32,
    pub pos: BlockPos,
    pub progress: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEvent {
    pub pos: BlockPos,
    pub b0: u8,
    pub b1: u8,
    pub block_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionBlocksUpdate {
    pub section_x: i32,
    pub section_y: i32,
    pub section_z: i32,
    pub updates: Vec<BlockUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunksBiomes {
    pub chunks: Vec<ChunkBiomeData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkBiomeData {
    pub pos: ChunkPos,
    pub raw_biomes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetLevelChunk {
    pub pos: ChunkPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelEvent {
    pub event_type: i32,
    pub pos: BlockPos,
    pub data: i32,
    pub global: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetChunkCacheCenter {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetChunkCacheRadius {
    pub radius: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelChunkWithLight {
    pub x: i32,
    pub z: i32,
    pub raw_after_position: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightUpdate {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub raw_light_data: Vec<u8>,
}

pub(crate) fn decode_block_changed_ack(decoder: &mut Decoder<'_>) -> Result<BlockChangedAck> {
    Ok(BlockChangedAck {
        sequence: decoder.read_var_i32()?,
    })
}

pub(crate) fn decode_block_destruction(decoder: &mut Decoder<'_>) -> Result<BlockDestruction> {
    Ok(BlockDestruction {
        id: decoder.read_var_i32()?,
        pos: decode_block_pos(decoder.read_i64()?),
        progress: decoder.read_u8()?,
    })
}

pub(crate) fn decode_block_entity_data(decoder: &mut Decoder<'_>) -> Result<BlockEntityData> {
    Ok(BlockEntityData {
        pos: decode_block_pos(decoder.read_i64()?),
        block_entity_type_id: decoder.read_var_i32()?,
        raw_nbt: decoder.remaining().to_vec(),
    })
}

pub(crate) fn decode_block_event(decoder: &mut Decoder<'_>) -> Result<BlockEvent> {
    Ok(BlockEvent {
        pos: decode_block_pos(decoder.read_i64()?),
        b0: decoder.read_u8()?,
        b1: decoder.read_u8()?,
        block_id: decoder.read_var_i32()?,
    })
}

pub(crate) fn decode_block_update(decoder: &mut Decoder<'_>) -> Result<BlockUpdate> {
    Ok(BlockUpdate {
        pos: decode_block_pos(decoder.read_i64()?),
        block_state_id: decoder.read_var_i32()?,
    })
}

pub(crate) fn decode_chunks_biomes(decoder: &mut Decoder<'_>) -> Result<ChunksBiomes> {
    let count = decoder.read_len()?;
    let mut chunks = Vec::with_capacity(count);
    for _ in 0..count {
        let pos = decode_chunk_pos(decoder.read_i64()?);
        let len = decoder.read_len()?;
        if len > MAX_CHUNKS_BIOMES_BUFFER {
            return Err(ProtocolError::PacketTooLarge(len, MAX_CHUNKS_BIOMES_BUFFER));
        }
        chunks.push(ChunkBiomeData {
            pos,
            raw_biomes: decoder.read_exact(len, "chunk biome data")?.to_vec(),
        });
    }
    Ok(ChunksBiomes { chunks })
}

pub(crate) fn decode_forget_level_chunk(decoder: &mut Decoder<'_>) -> Result<ForgetLevelChunk> {
    Ok(ForgetLevelChunk {
        pos: decode_chunk_pos(decoder.read_i64()?),
    })
}

pub(crate) fn decode_level_event(decoder: &mut Decoder<'_>) -> Result<LevelEvent> {
    Ok(LevelEvent {
        event_type: decoder.read_i32()?,
        pos: decode_block_pos(decoder.read_i64()?),
        data: decoder.read_i32()?,
        global: decoder.read_bool()?,
    })
}

pub(crate) fn decode_level_chunk_with_light(
    decoder: &mut Decoder<'_>,
) -> Result<LevelChunkWithLight> {
    let x = decoder.read_i32()?;
    let z = decoder.read_i32()?;
    Ok(LevelChunkWithLight {
        x,
        z,
        raw_after_position: decoder.remaining().to_vec(),
    })
}

pub(crate) fn decode_light_update(decoder: &mut Decoder<'_>) -> Result<LightUpdate> {
    Ok(LightUpdate {
        chunk_x: decoder.read_var_i32()?,
        chunk_z: decoder.read_var_i32()?,
        raw_light_data: decoder.remaining().to_vec(),
    })
}

pub(crate) fn decode_section_blocks_update(
    decoder: &mut Decoder<'_>,
) -> Result<SectionBlocksUpdate> {
    let (section_x, section_y, section_z) = decode_section_pos(decoder.read_i64()?);
    let count = decoder.read_len()?;
    let mut updates = Vec::with_capacity(count);
    for _ in 0..count {
        let packed_change = decoder.read_var_i64()? as u64;
        let packed_pos = (packed_change & 0x0fff) as u16;
        let block_state_id = (packed_change >> 12) as i32;
        updates.push(BlockUpdate {
            pos: BlockPos {
                x: section_x * 16 + i32::from((packed_pos >> 8) & 0x0f),
                y: section_y * 16 + i32::from(packed_pos & 0x0f),
                z: section_z * 16 + i32::from((packed_pos >> 4) & 0x0f),
            },
            block_state_id,
        });
    }
    Ok(SectionBlocksUpdate {
        section_x,
        section_y,
        section_z,
        updates,
    })
}

pub(crate) fn decode_set_chunk_cache_center(
    decoder: &mut Decoder<'_>,
) -> Result<SetChunkCacheCenter> {
    Ok(SetChunkCacheCenter {
        chunk_x: decoder.read_var_i32()?,
        chunk_z: decoder.read_var_i32()?,
    })
}

pub(crate) fn decode_set_chunk_cache_radius(
    decoder: &mut Decoder<'_>,
) -> Result<SetChunkCacheRadius> {
    Ok(SetChunkCacheRadius {
        radius: decoder.read_var_i32()?,
    })
}

pub(crate) fn decode_block_pos(packed: i64) -> BlockPos {
    BlockPos {
        x: (packed >> 38) as i32,
        y: ((packed << 52) >> 52) as i32,
        z: ((packed << 26) >> 38) as i32,
    }
}

pub(crate) fn encode_block_pos(pos: BlockPos) -> i64 {
    (((pos.x as i64) & 0x3ffffff) << 38)
        | (((pos.z as i64) & 0x3ffffff) << 12)
        | ((pos.y as i64) & 0xfff)
}

pub(crate) fn encode_block_hit_result(out: &mut Encoder, hit: BlockHitResult) {
    out.write_i64(encode_block_pos(hit.pos));
    out.write_var_i32(i32::from(hit.direction.id()));
    out.write_f32(hit.cursor_x);
    out.write_f32(hit.cursor_y);
    out.write_f32(hit.cursor_z);
    out.write_bool(hit.inside);
    out.write_bool(hit.world_border_hit);
}

fn decode_section_pos(packed: i64) -> (i32, i32, i32) {
    (
        (packed >> 42) as i32,
        ((packed << 44) >> 44) as i32,
        ((packed << 22) >> 42) as i32,
    )
}

fn decode_chunk_pos(packed: i64) -> ChunkPos {
    ChunkPos {
        x: packed as i32,
        z: (packed >> 32) as i32,
    }
}

#[cfg(test)]
mod tests {
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
            decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_EVENT, &payload.into_inner())
                .unwrap();
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
            decode_play_clientbound(ids::play::CLIENTBOUND_LEVEL_EVENT, &payload.into_inner())
                .unwrap();
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
}
