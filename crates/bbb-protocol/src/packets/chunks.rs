use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};

use super::BlockHitResult;

const MAX_CHUNKS_BIOMES_BUFFER: usize = 2 * 1024 * 1024;
const MAX_LEVEL_CHUNK_BUFFER: usize = 2 * 1024 * 1024;
const LIGHT_ARRAY_BYTES: usize = 2048;
const MAX_NBT_DEPTH: usize = 64;
const MAX_NBT_LIST_ITEMS: usize = 4096;

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
    pub chunk_data: LevelChunkData,
    pub light_data: LightUpdateData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelChunkData {
    pub heightmaps: Vec<ChunkHeightmapData>,
    pub section_data: Vec<u8>,
    pub block_entities: Vec<LevelChunkBlockEntity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkHeightmapData {
    pub kind_id: i32,
    pub data: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelChunkBlockEntity {
    pub packed_xz: u8,
    pub y: i16,
    pub block_entity_type_id: i32,
    pub raw_nbt: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightUpdate {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub light_data: LightUpdateData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightUpdateData {
    pub sky_y_mask: Vec<i64>,
    pub block_y_mask: Vec<i64>,
    pub empty_sky_y_mask: Vec<i64>,
    pub empty_block_y_mask: Vec<i64>,
    pub sky_updates: Vec<Vec<u8>>,
    pub block_updates: Vec<Vec<u8>>,
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
    let packet = LevelChunkWithLight {
        x,
        z,
        chunk_data: decode_level_chunk_data(decoder)?,
        light_data: decode_light_update_data(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing {} bytes after level chunk with light packet",
            decoder.remaining_len()
        )));
    }
    Ok(packet)
}

pub(crate) fn decode_light_update(decoder: &mut Decoder<'_>) -> Result<LightUpdate> {
    let packet = LightUpdate {
        chunk_x: decoder.read_var_i32()?,
        chunk_z: decoder.read_var_i32()?,
        light_data: decode_light_update_data(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing {} bytes after light update packet",
            decoder.remaining_len()
        )));
    }
    Ok(packet)
}

fn decode_level_chunk_data(decoder: &mut Decoder<'_>) -> Result<LevelChunkData> {
    let heightmaps = decode_heightmaps(decoder)?;
    let section_len = decoder.read_len()?;
    if section_len > MAX_LEVEL_CHUNK_BUFFER {
        return Err(ProtocolError::PacketTooLarge(
            section_len,
            MAX_LEVEL_CHUNK_BUFFER,
        ));
    }
    let section_data = decoder
        .read_exact(section_len, "level chunk section data")?
        .to_vec();
    let block_entities = decode_level_chunk_block_entities(decoder)?;
    Ok(LevelChunkData {
        heightmaps,
        section_data,
        block_entities,
    })
}

fn decode_heightmaps(decoder: &mut Decoder<'_>) -> Result<Vec<ChunkHeightmapData>> {
    let count = decoder.read_len()?;
    let mut heightmaps = Vec::with_capacity(count);
    for _ in 0..count {
        heightmaps.push(ChunkHeightmapData {
            kind_id: decoder.read_var_i32()?,
            data: read_long_array(decoder)?,
        });
    }
    Ok(heightmaps)
}

fn decode_level_chunk_block_entities(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<LevelChunkBlockEntity>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(LevelChunkBlockEntity {
            packed_xz: decoder.read_u8()?,
            y: decoder.read_i16()?,
            block_entity_type_id: decoder.read_var_i32()?,
            raw_nbt: read_raw_nbt(decoder)?,
        });
    }
    Ok(out)
}

fn decode_light_update_data(decoder: &mut Decoder<'_>) -> Result<LightUpdateData> {
    Ok(LightUpdateData {
        sky_y_mask: read_long_array(decoder)?,
        block_y_mask: read_long_array(decoder)?,
        empty_sky_y_mask: read_long_array(decoder)?,
        empty_block_y_mask: read_long_array(decoder)?,
        sky_updates: read_light_array_list(decoder)?,
        block_updates: read_light_array_list(decoder)?,
    })
}

fn read_long_array(decoder: &mut Decoder<'_>) -> Result<Vec<i64>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(decoder.read_i64()?);
    }
    Ok(out)
}

fn read_light_array_list(decoder: &mut Decoder<'_>) -> Result<Vec<Vec<u8>>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = decoder.read_len()?;
        if len != LIGHT_ARRAY_BYTES {
            return Err(ProtocolError::InvalidData(format!(
                "light update array has {len} bytes, expected {LIGHT_ARRAY_BYTES}"
            )));
        }
        out.push(decoder.read_exact(len, "light update array")?.to_vec());
    }
    Ok(out)
}

fn read_raw_nbt(decoder: &mut Decoder<'_>) -> Result<Vec<u8>> {
    let start = decoder.position();
    let tag_id = decoder.read_u8()?;
    if tag_id != 0 {
        skip_nbt_payload(decoder, tag_id, 0)?;
    }
    Ok(decoder.bytes_from(start).to_vec())
}

fn skip_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Result<()> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "chunk nbt exceeded max depth".to_string(),
        ));
    }

    match tag_id {
        1 => {
            decoder.read_exact(1, "nbt byte")?;
        }
        2 => {
            decoder.read_exact(2, "nbt short")?;
        }
        3 | 5 => {
            decoder.read_exact(4, "nbt int/float")?;
        }
        4 | 6 => {
            decoder.read_exact(8, "nbt long/double")?;
        }
        7 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len, "nbt byte array")?;
        }
        8 => {
            read_nbt_string(decoder)?;
        }
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_LIST_ITEMS));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty nbt list has end tag element type".to_string(),
                ));
            }
            for _ in 0..len {
                skip_nbt_payload(decoder, element_type, depth + 1)?;
            }
        }
        10 => loop {
            let nested_type = decoder.read_u8()?;
            if nested_type == 0 {
                break;
            }
            read_nbt_string(decoder)?;
            skip_nbt_payload(decoder, nested_type, depth + 1)?;
        },
        11 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt int array")?;
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt long array")?;
        }
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid chunk nbt tag id {other}"
            )));
        }
    }
    Ok(())
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(ProtocolError::NegativeLength(len));
    }
    Ok(len as usize)
}

fn read_nbt_string(decoder: &mut Decoder<'_>) -> Result<()> {
    let len = decoder.read_u16()? as usize;
    decoder.read_exact(len, "nbt string")?;
    Ok(())
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

pub(crate) fn decode_chunk_pos(packed: i64) -> ChunkPos {
    ChunkPos {
        x: packed as i32,
        z: (packed >> 32) as i32,
    }
}

#[cfg(test)]
mod tests;
