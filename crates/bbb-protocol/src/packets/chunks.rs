use serde::{Deserialize, Serialize};

use crate::{
    block_entity_types::vanilla_block_entity_type,
    block_state_types::vanilla_block_state_id,
    block_types::vanilla_block_type,
    codec::{Decoder, Encoder, ProtocolError, Result},
    content_registry::ContentRegistry,
    heightmap_types::{vanilla_heightmap_type, VANILLA_HEIGHTMAP_TYPES},
    ids,
};

use super::BlockHitResult;

const MAX_CHUNKS_BIOMES_BUFFER: usize = 2 * 1024 * 1024;
const MAX_CHUNKS_BIOMES_UPDATES: usize = 65_536;
const MAX_LEVEL_CHUNK_BUFFER: usize = 2 * 1024 * 1024;
// DimensionType.Y_SIZE is 4,064 blocks, so a 16x16 column cannot contain more
// distinct block-entity positions than this even in the tallest legal level.
const MAX_LEVEL_CHUNK_BLOCK_ENTITIES: usize = 16 * 16 * 4064;
const MAX_HEIGHTMAP_LONGS: usize = 16 * 16;
const MAX_SECTION_BLOCK_UPDATES: usize = 4096;
const LIGHT_ARRAY_BYTES: usize = 2048;
// A maximum-height dimension has 254 chunk sections plus the two light-engine
// boundary sections, so its four BitSet masks occupy at most four longs.
const MAX_LIGHT_SECTIONS: usize = 256;
const MAX_LIGHT_MASK_LONGS: usize = MAX_LIGHT_SECTIONS / 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x.div_euclid(16),
            z: self.z.div_euclid(16),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

pub fn encode_play_clientbound_set_chunk_cache_center(
    update: SetChunkCacheCenter,
) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.chunk_x);
    encoder.write_var_i32(update.chunk_z);
    (
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER,
        encoder.into_inner(),
    )
}

/// `ClientboundSetChunkCacheRadiusPacket.write`
/// (`ClientboundSetChunkCacheRadiusPacket.java:22-24`).
/// `PlayerList.setViewDistance` (`PlayerList.java:801-803`) broadcasts it
/// whenever the server view distance changes, so the client can resize its own
/// chunk cache before the columns for the new radius arrive.
pub fn encode_play_clientbound_set_chunk_cache_radius(
    update: SetChunkCacheRadius,
) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.radius);
    (
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS,
        encoder.into_inner(),
    )
}

/// `ClientboundChunksBiomesPacket.write` (`ClientboundChunksBiomesPacket.java:28-30`)
/// is `writeCollection` of `ChunkBiomeData.write` (`ClientboundChunksBiomesPacket.java:81-84`):
/// packed `ChunkPos`, then `writeByteArray` (`FriendlyByteBuf.java:289-292`).
pub fn encode_play_clientbound_chunks_biomes(update: &ChunksBiomes) -> Result<(i32, Vec<u8>)> {
    if update.chunks.len() > MAX_CHUNKS_BIOMES_UPDATES {
        return Err(ProtocolError::PacketTooLarge(
            update.chunks.len(),
            MAX_CHUNKS_BIOMES_UPDATES,
        ));
    }
    let mut encoder = Encoder::new();
    write_len(&mut encoder, update.chunks.len(), "chunks biomes")?;
    for chunk in &update.chunks {
        if chunk.raw_biomes.len() > MAX_CHUNKS_BIOMES_BUFFER {
            return Err(ProtocolError::PacketTooLarge(
                chunk.raw_biomes.len(),
                MAX_CHUNKS_BIOMES_BUFFER,
            ));
        }
        encoder.write_i64(encode_chunk_pos(chunk.pos));
        write_len(&mut encoder, chunk.raw_biomes.len(), "chunk biome data")?;
        encoder.write_bytes(&chunk.raw_biomes);
    }
    Ok((ids::play::CLIENTBOUND_CHUNKS_BIOMES, encoder.into_inner()))
}

/// `ClientboundForgetLevelChunkPacket.write`
/// (`ClientboundForgetLevelChunkPacket.java:18-20`) writes the position through
/// `FriendlyByteBuf.writeChunkPos` (`FriendlyByteBuf.java:406-409`), i.e. the
/// single packed long produced by `ChunkPos.pack`.
///
/// The server only reaches this packet from `PlayerChunkSender.dropChunk`
/// (`PlayerChunkSender.java:44-48`), and only for chunks that already left the
/// pending-send queue: a column the client was never sent must not be
/// unloaded, or the client drops a chunk it is still waiting for.
pub fn encode_play_clientbound_forget_level_chunk(update: ForgetLevelChunk) -> (i32, Vec<u8>) {
    let mut encoder = Encoder::new();
    encoder.write_i64(encode_chunk_pos(update.pos));
    (
        ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK,
        encoder.into_inner(),
    )
}

pub fn encode_play_clientbound_level_chunk_with_light(
    packet: &LevelChunkWithLight,
) -> Result<(i32, Vec<u8>)> {
    validate_level_chunk_with_light(packet)?;
    let mut encoder = Encoder::new();
    encoder.write_i32(packet.x);
    encoder.write_i32(packet.z);
    write_len(
        &mut encoder,
        packet.chunk_data.heightmaps.len(),
        "heightmaps",
    )?;
    for heightmap in &packet.chunk_data.heightmaps {
        encoder.write_var_i32(heightmap.kind_id);
        write_long_array(&mut encoder, &heightmap.data, "heightmap data")?;
    }
    write_len(
        &mut encoder,
        packet.chunk_data.section_data.len(),
        "chunk section data",
    )?;
    encoder.write_bytes(&packet.chunk_data.section_data);
    write_len(
        &mut encoder,
        packet.chunk_data.block_entities.len(),
        "chunk block entities",
    )?;
    for block_entity in &packet.chunk_data.block_entities {
        encoder.write_u8(block_entity.packed_xz);
        encoder.write_i16(block_entity.y);
        encoder.write_var_i32(block_entity.block_entity_type_id);
        encoder.write_bytes(&block_entity.raw_nbt);
    }
    encode_light_update_data_validated(&mut encoder, &packet.light_data)?;
    Ok((
        ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
        encoder.into_inner(),
    ))
}

/// `ClientboundLightUpdatePacket.write` (`ClientboundLightUpdatePacket.java:37-41`).
pub fn encode_play_clientbound_light_update(packet: &LightUpdate) -> Result<(i32, Vec<u8>)> {
    validate_light_update_data(&packet.light_data)?;
    let mut encoder = Encoder::new();
    encoder.write_var_i32(packet.chunk_x);
    encoder.write_var_i32(packet.chunk_z);
    encode_light_update_data_validated(&mut encoder, &packet.light_data)?;
    Ok((ids::play::CLIENTBOUND_LIGHT_UPDATE, encoder.into_inner()))
}

fn validate_level_chunk_with_light(packet: &LevelChunkWithLight) -> Result<()> {
    if packet.chunk_data.heightmaps.len() > VANILLA_HEIGHTMAP_TYPES.len() {
        return Err(ProtocolError::PacketTooLarge(
            packet.chunk_data.heightmaps.len(),
            VANILLA_HEIGHTMAP_TYPES.len(),
        ));
    }
    for heightmap in &packet.chunk_data.heightmaps {
        if vanilla_heightmap_type(heightmap.kind_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "unknown heightmap type id {}",
                heightmap.kind_id
            )));
        }
        if heightmap.data.len() > MAX_HEIGHTMAP_LONGS {
            return Err(ProtocolError::PacketTooLarge(
                heightmap.data.len(),
                MAX_HEIGHTMAP_LONGS,
            ));
        }
    }
    if packet.chunk_data.section_data.len() > MAX_LEVEL_CHUNK_BUFFER {
        return Err(ProtocolError::PacketTooLarge(
            packet.chunk_data.section_data.len(),
            MAX_LEVEL_CHUNK_BUFFER,
        ));
    }
    if packet.chunk_data.block_entities.len() > MAX_LEVEL_CHUNK_BLOCK_ENTITIES {
        return Err(ProtocolError::PacketTooLarge(
            packet.chunk_data.block_entities.len(),
            MAX_LEVEL_CHUNK_BLOCK_ENTITIES,
        ));
    }
    for block_entity in &packet.chunk_data.block_entities {
        if vanilla_block_entity_type(block_entity.block_entity_type_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "unknown block entity type id {}",
                block_entity.block_entity_type_id
            )));
        }
        let mut decoder = Decoder::new(&block_entity.raw_nbt);
        skip_raw_optional_compound_nbt(&mut decoder)?;
        if !decoder.is_empty() {
            return Err(ProtocolError::InvalidData(
                "trailing bytes after chunk block entity nbt".to_string(),
            ));
        }
    }
    validate_light_update_data(&packet.light_data)
}

fn encode_light_update_data_validated(
    encoder: &mut Encoder,
    light: &LightUpdateData,
) -> Result<()> {
    for (values, label) in [
        (&light.sky_y_mask, "sky light mask"),
        (&light.block_y_mask, "block light mask"),
        (&light.empty_sky_y_mask, "empty sky light mask"),
        (&light.empty_block_y_mask, "empty block light mask"),
    ] {
        if values.len() > MAX_LIGHT_MASK_LONGS {
            return Err(ProtocolError::PacketTooLarge(
                values.len(),
                MAX_LIGHT_MASK_LONGS,
            ));
        }
        write_long_array(encoder, values, label)?;
    }
    for updates in [&light.sky_updates, &light.block_updates] {
        if updates.len() > MAX_LIGHT_SECTIONS {
            return Err(ProtocolError::PacketTooLarge(
                updates.len(),
                MAX_LIGHT_SECTIONS,
            ));
        }
        write_len(encoder, updates.len(), "light update arrays")?;
        for update in updates {
            if update.len() != LIGHT_ARRAY_BYTES {
                return Err(ProtocolError::InvalidData(format!(
                    "light update array has {} bytes, expected {LIGHT_ARRAY_BYTES}",
                    update.len()
                )));
            }
            write_len(encoder, update.len(), "light update array")?;
            encoder.write_bytes(update);
        }
    }
    Ok(())
}

fn write_long_array(encoder: &mut Encoder, values: &[i64], what: &'static str) -> Result<()> {
    write_len(encoder, values.len(), what)?;
    for value in values {
        encoder.write_i64(*value);
    }
    Ok(())
}

fn write_len(encoder: &mut Encoder, len: usize, what: &'static str) -> Result<()> {
    let len = i32::try_from(len)
        .map_err(|_| ProtocolError::InvalidData(format!("{what} length exceeds i32")))?;
    encoder.write_var_i32(len);
    Ok(())
}

pub(crate) fn decode_block_changed_ack(decoder: &mut Decoder<'_>) -> Result<BlockChangedAck> {
    let packet = BlockChangedAck {
        sequence: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block changed ack packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_block_destruction(decoder: &mut Decoder<'_>) -> Result<BlockDestruction> {
    let packet = BlockDestruction {
        id: decoder.read_var_i32()?,
        pos: decode_block_pos(decoder.read_i64()?),
        progress: decoder.read_u8()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block destruction packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_block_entity_data(decoder: &mut Decoder<'_>) -> Result<BlockEntityData> {
    let pos = decode_block_pos(decoder.read_i64()?);
    let block_entity_type_id = decoder.read_var_i32()?;
    if vanilla_block_entity_type(block_entity_type_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "unknown block entity type id {block_entity_type_id}"
        )));
    }
    let packet = BlockEntityData {
        pos,
        block_entity_type_id,
        raw_nbt: read_raw_required_compound_nbt(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block entity data packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_block_event_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<BlockEvent> {
    let pos = decode_block_pos(decoder.read_i64()?);
    let b0 = decoder.read_u8()?;
    let b1 = decoder.read_u8()?;
    let block_id = decoder.read_var_i32()?;
    if validate_block_id(block_id, registry).is_err() {
        return Err(ProtocolError::InvalidData(format!(
            "unknown block registry id {block_id} in block event packet"
        )));
    }
    let packet = BlockEvent {
        pos,
        b0,
        b1,
        block_id,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block event packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_block_update_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<BlockUpdate> {
    let packet = BlockUpdate {
        pos: decode_block_pos(decoder.read_i64()?),
        block_state_id: validate_block_state_id(decoder.read_var_i32()?, registry)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block update packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_chunks_biomes(decoder: &mut Decoder<'_>) -> Result<ChunksBiomes> {
    let count = decoder.read_len()?;
    if count > MAX_CHUNKS_BIOMES_UPDATES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_CHUNKS_BIOMES_UPDATES,
        ));
    }
    let mut chunks = Vec::with_capacity(decoder.bounded_capacity(count, 9));
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
    let packet = ChunksBiomes { chunks };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after chunks biomes packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_forget_level_chunk(decoder: &mut Decoder<'_>) -> Result<ForgetLevelChunk> {
    let packet = ForgetLevelChunk {
        pos: decode_chunk_pos(decoder.read_i64()?),
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after forget level chunk packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(crate) fn decode_level_event(decoder: &mut Decoder<'_>) -> Result<LevelEvent> {
    let packet = LevelEvent {
        event_type: decoder.read_i32()?,
        pos: decode_block_pos(decoder.read_i64()?),
        data: decoder.read_i32()?,
        global: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after level event packet".to_string(),
        ));
    }
    Ok(packet)
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
    if count > VANILLA_HEIGHTMAP_TYPES.len() {
        return Err(ProtocolError::PacketTooLarge(
            count,
            VANILLA_HEIGHTMAP_TYPES.len(),
        ));
    }
    let mut heightmaps = Vec::with_capacity(decoder.bounded_capacity(count, 2));
    for _ in 0..count {
        let kind_id = decoder.read_var_i32()?;
        if vanilla_heightmap_type(kind_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "unknown heightmap type id {kind_id}"
            )));
        }
        heightmaps.push(ChunkHeightmapData {
            kind_id,
            data: read_bounded_long_array(decoder, MAX_HEIGHTMAP_LONGS, "heightmap data")?,
        });
    }
    Ok(heightmaps)
}

fn decode_level_chunk_block_entities(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<LevelChunkBlockEntity>> {
    let count = decoder.read_len()?;
    if count > MAX_LEVEL_CHUNK_BLOCK_ENTITIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_LEVEL_CHUNK_BLOCK_ENTITIES,
        ));
    }
    const MIN_BLOCK_ENTITY_BYTES: usize = 5;
    if count > decoder.remaining_len() / MIN_BLOCK_ENTITY_BYTES {
        return Err(ProtocolError::InvalidData(format!(
            "level chunk block entity count {count} exceeds remaining packet bytes"
        )));
    }
    let mut out = Vec::with_capacity(decoder.bounded_capacity(count, MIN_BLOCK_ENTITY_BYTES));
    for _ in 0..count {
        let packed_xz = decoder.read_u8()?;
        let y = decoder.read_i16()?;
        let block_entity_type_id = decoder.read_var_i32()?;
        if vanilla_block_entity_type(block_entity_type_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "unknown block entity type id {block_entity_type_id}"
            )));
        }
        out.push(LevelChunkBlockEntity {
            packed_xz,
            y,
            block_entity_type_id,
            raw_nbt: read_raw_optional_compound_nbt(decoder)?,
        });
    }
    Ok(out)
}

fn decode_light_update_data(decoder: &mut Decoder<'_>) -> Result<LightUpdateData> {
    let light = LightUpdateData {
        sky_y_mask: read_bounded_long_array(decoder, MAX_LIGHT_MASK_LONGS, "sky light mask")?,
        block_y_mask: read_bounded_long_array(decoder, MAX_LIGHT_MASK_LONGS, "block light mask")?,
        empty_sky_y_mask: read_bounded_long_array(
            decoder,
            MAX_LIGHT_MASK_LONGS,
            "empty sky light mask",
        )?,
        empty_block_y_mask: read_bounded_long_array(
            decoder,
            MAX_LIGHT_MASK_LONGS,
            "empty block light mask",
        )?,
        sky_updates: read_light_array_list(decoder)?,
        block_updates: read_light_array_list(decoder)?,
    };
    validate_light_update_data(&light)?;
    Ok(light)
}

fn read_bounded_long_array(decoder: &mut Decoder<'_>, max: usize, label: &str) -> Result<Vec<i64>> {
    let count = decoder.read_len()?;
    if count > max {
        return Err(ProtocolError::PacketTooLarge(count, max));
    }
    if count > decoder.remaining_len() / std::mem::size_of::<i64>() {
        return Err(ProtocolError::InvalidData(format!(
            "{label} count {count} exceeds remaining packet bytes"
        )));
    }
    let mut out = Vec::with_capacity(decoder.bounded_capacity(count, std::mem::size_of::<i64>()));
    for _ in 0..count {
        out.push(decoder.read_i64()?);
    }
    Ok(out)
}

fn read_light_array_list(decoder: &mut Decoder<'_>) -> Result<Vec<Vec<u8>>> {
    let count = decoder.read_len()?;
    if count > MAX_LIGHT_SECTIONS {
        return Err(ProtocolError::PacketTooLarge(count, MAX_LIGHT_SECTIONS));
    }
    let mut out = Vec::with_capacity(decoder.bounded_capacity(count, LIGHT_ARRAY_BYTES + 2));
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

fn validate_light_update_data(light: &LightUpdateData) -> Result<()> {
    for (layer, data_mask, empty_mask, updates) in [
        (
            "sky",
            &light.sky_y_mask,
            &light.empty_sky_y_mask,
            &light.sky_updates,
        ),
        (
            "block",
            &light.block_y_mask,
            &light.empty_block_y_mask,
            &light.block_updates,
        ),
    ] {
        let data_count = bit_count(data_mask);
        if data_count != updates.len() {
            return Err(ProtocolError::InvalidData(format!(
                "{layer} light mask has {data_count} data sections but {} arrays",
                updates.len()
            )));
        }
        if data_mask
            .iter()
            .zip(empty_mask)
            .any(|(data, empty)| data & empty != 0)
        {
            return Err(ProtocolError::InvalidData(format!(
                "{layer} light data and empty masks overlap"
            )));
        }
    }
    Ok(())
}

fn bit_count(mask: &[i64]) -> usize {
    mask.iter()
        .map(|word| (*word as u64).count_ones() as usize)
        .sum()
}

fn skip_raw_optional_compound_nbt(decoder: &mut Decoder<'_>) -> Result<()> {
    let tag_id = decoder.read_u8()?;
    match tag_id {
        0 => Ok(()),
        10 => skip_nbt_payload(decoder, tag_id, 0),
        other => Err(ProtocolError::InvalidData(format!(
            "block entity nbt root must be a compound tag or null, got tag id {other}"
        ))),
    }
}

fn read_raw_optional_compound_nbt(decoder: &mut Decoder<'_>) -> Result<Vec<u8>> {
    let start = decoder.position();
    skip_raw_optional_compound_nbt(decoder)?;
    Ok(decoder.bytes_from(start).to_vec())
}

fn read_raw_required_compound_nbt(decoder: &mut Decoder<'_>) -> Result<Vec<u8>> {
    let start = decoder.position();
    let tag_id = decoder.read_u8()?;
    if tag_id != 10 {
        return Err(ProtocolError::InvalidData(format!(
            "block entity data nbt root must be a non-null compound tag, got tag id {tag_id}"
        )));
    }
    skip_nbt_payload(decoder, tag_id, 0)?;
    Ok(decoder.bytes_from(start).to_vec())
}

fn skip_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, _depth: usize) -> Result<()> {
    // ClientboundLevelChunkPacketData.java:143,150; NbtAccounter.java:6-8,23-25:
    // default network quota (2 MiB, depth 512, no 4096 collection entry cap).
    crate::nbt_summary::skip_network_nbt_payload(
        decoder,
        tag_id,
        crate::nbt_summary::SkipNbtLimits {
            max_depth: crate::nbt_summary::VANILLA_DEFAULT_NBT_MAX_DEPTH,
            max_list_items: usize::MAX,
            max_array_items: usize::MAX,
            max_compound_entries: usize::MAX,
            max_total_bytes: Some(crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize),
            validate_mutf8: true,
            label: "chunk nbt",
        },
    )
}

pub(crate) fn decode_section_blocks_update_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<SectionBlocksUpdate> {
    let (section_x, section_y, section_z) = decode_section_pos(decoder.read_i64()?);
    let count = decoder.read_len()?;
    if count > MAX_SECTION_BLOCK_UPDATES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_SECTION_BLOCK_UPDATES,
        ));
    }
    let mut updates = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        let packed_change = decoder.read_var_i64()?;
        let packed_pos = (packed_change & 0x0fff) as u16;
        let raw_state_id = i32::try_from(packed_change >> 12).map_err(|_| {
            ProtocolError::InvalidData(format!(
                "section block update state id {} overflows i32",
                packed_change >> 12
            ))
        })?;
        let block_state_id = validate_block_state_id(raw_state_id, registry)?;
        updates.push(BlockUpdate {
            pos: BlockPos {
                x: section_x * 16 + i32::from((packed_pos >> 8) & 0x0f),
                y: section_y * 16 + i32::from(packed_pos & 0x0f),
                z: section_z * 16 + i32::from((packed_pos >> 4) & 0x0f),
            },
            block_state_id,
        });
    }
    let update = SectionBlocksUpdate {
        section_x,
        section_y,
        section_z,
        updates,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after section blocks update packet".to_string(),
        ));
    }
    Ok(update)
}

pub fn validate_block_id(block_id: i32, registry: &ContentRegistry) -> Result<i32> {
    if vanilla_block_type(block_id).is_some() || registry.block_by_id(block_id).is_some() {
        return Ok(block_id);
    }
    Err(ProtocolError::InvalidData(format!(
        "unknown block registry id {block_id}"
    )))
}

pub fn validate_block_state_id(block_state_id: i32, registry: &ContentRegistry) -> Result<i32> {
    if let Some(id) = vanilla_block_state_id(block_state_id) {
        return Ok(id);
    }
    if registry.block_by_state_id(block_state_id).is_some() {
        return Ok(block_state_id);
    }
    Err(ProtocolError::InvalidData(if registry.is_vanilla_only() {
        format!("unknown vanilla 26.1 block state id {block_state_id}")
    } else {
        format!("unknown block state id {block_state_id}")
    }))
}

pub(crate) fn decode_set_chunk_cache_center(
    decoder: &mut Decoder<'_>,
) -> Result<SetChunkCacheCenter> {
    let update = SetChunkCacheCenter {
        chunk_x: decoder.read_var_i32()?,
        chunk_z: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set chunk cache center packet".to_string(),
        ));
    }
    Ok(update)
}

pub(crate) fn decode_set_chunk_cache_radius(
    decoder: &mut Decoder<'_>,
) -> Result<SetChunkCacheRadius> {
    let update = SetChunkCacheRadius {
        radius: decoder.read_var_i32()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set chunk cache radius packet".to_string(),
        ));
    }
    Ok(update)
}

pub(crate) fn decode_block_pos(packed: i64) -> BlockPos {
    BlockPos {
        x: (packed >> 38) as i32,
        y: ((packed << 52) >> 52) as i32,
        z: ((packed << 26) >> 38) as i32,
    }
}

/// `BlockPos.asLong` (`BlockPos.java:111-116`).
#[inline]
pub fn encode_block_pos(pos: BlockPos) -> i64 {
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

/// `ChunkPos.pack` (`ChunkPos.java:76-78`).
#[inline]
pub fn encode_chunk_pos(pos: ChunkPos) -> i64 {
    ((pos.x as u32) as i64) | (((pos.z as u32) as i64) << 32)
}

#[cfg(test)]
mod tests;
