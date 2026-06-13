use bbb_protocol::codec::Decoder;

use crate::{
    BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState, HeightmapData,
    NbtPayloadSummary, Result, WorldDecodeError,
};

use super::{
    light::decode_light_data,
    palette::{packed_long_len, palette_kind, PaletteDomain, PaletteKind, PalettedContainerData},
};

const MAX_CHUNK_SECTION_BUFFER: usize = 2 * 1024 * 1024;

pub fn decode_level_chunk_with_light(pos: ChunkPos, payload: &[u8]) -> Result<ChunkColumn> {
    let mut decoder = Decoder::new(payload);
    let heightmaps = decode_heightmaps(&mut decoder)?;

    let section_buffer_len = decoder.read_len()?;
    if section_buffer_len > MAX_CHUNK_SECTION_BUFFER {
        return Err(WorldDecodeError::ChunkSectionBufferTooLarge {
            actual: section_buffer_len,
            max: MAX_CHUNK_SECTION_BUFFER,
        });
    }
    let section_buffer = decoder.read_exact(section_buffer_len, "chunk section buffer")?;
    let sections = decode_sections(section_buffer)?;
    let block_entities = decode_block_entities(&mut decoder)?;
    let light = decode_light_data(&mut decoder)?;

    Ok(ChunkColumn {
        pos,
        state: ChunkState::Decoded,
        heightmaps,
        sections,
        block_entities,
        light,
    })
}

pub(crate) fn decode_biome_sections(
    bytes: &[u8],
    expected_sections: usize,
) -> Result<Vec<PalettedContainerData>> {
    if bytes.len() > MAX_CHUNK_SECTION_BUFFER {
        return Err(WorldDecodeError::ByteArrayTooLarge {
            actual: bytes.len(),
            max: MAX_CHUNK_SECTION_BUFFER,
        });
    }

    let mut decoder = Decoder::new(bytes);
    let mut biomes = Vec::with_capacity(expected_sections);
    for _ in 0..expected_sections {
        biomes.push(decode_paletted_container(
            &mut decoder,
            PaletteDomain::Biomes,
        )?);
    }
    if !decoder.is_empty() {
        return Err(WorldDecodeError::TrailingBiomeData {
            remaining: decoder.remaining_len(),
        });
    }
    Ok(biomes)
}

pub(crate) fn decode_nbt_payload_summary(bytes: &[u8]) -> Result<Option<NbtPayloadSummary>> {
    let mut decoder = Decoder::new(bytes);
    let nbt = skip_nbt_any(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(WorldDecodeError::TrailingBlockEntityData {
            remaining: decoder.remaining_len(),
        });
    }
    Ok(nbt)
}

fn decode_heightmaps(decoder: &mut Decoder<'_>) -> Result<Vec<HeightmapData>> {
    let count = decoder.read_len()?;
    let mut heightmaps = Vec::with_capacity(count);
    for _ in 0..count {
        let kind_id = decoder.read_var_i32()?;
        let data = read_long_array(decoder)?;
        heightmaps.push(HeightmapData { kind_id, data });
    }
    Ok(heightmaps)
}

fn decode_sections(bytes: &[u8]) -> Result<Vec<ChunkSection>> {
    let mut decoder = Decoder::new(bytes);
    let mut sections = Vec::new();
    while !decoder.is_empty() {
        sections.push(decode_section(&mut decoder)?);
    }
    Ok(sections)
}

fn decode_section(decoder: &mut Decoder<'_>) -> Result<ChunkSection> {
    let non_empty_block_count = decoder.read_i16()?;
    let fluid_count = decoder.read_i16()?;
    let block_states = decode_paletted_container(decoder, PaletteDomain::BlockStates)?;
    let biomes = decode_paletted_container(decoder, PaletteDomain::Biomes)?;
    Ok(ChunkSection {
        non_empty_block_count,
        fluid_count,
        block_states,
        biomes,
    })
}

fn decode_paletted_container(
    decoder: &mut Decoder<'_>,
    domain: PaletteDomain,
) -> Result<PalettedContainerData> {
    let bits_per_entry = decoder.read_u8()?;
    if bits_per_entry > 64 {
        return Err(WorldDecodeError::InvalidPalettedBits(bits_per_entry));
    }
    let entry_count = match domain {
        PaletteDomain::BlockStates => 16 * 16 * 16,
        PaletteDomain::Biomes => 4 * 4 * 4,
    };
    let palette_kind = palette_kind(domain, bits_per_entry);
    let palette_global_ids = match palette_kind {
        PaletteKind::SingleValue => vec![decoder.read_var_i32()?],
        PaletteKind::Local => read_var_i32_array(decoder)?,
        PaletteKind::Global => Vec::new(),
    };
    let packed_data_len = packed_long_len(entry_count, bits_per_entry as usize);
    let packed_data = read_fixed_long_array(decoder, packed_data_len)?;

    Ok(PalettedContainerData {
        domain,
        bits_per_entry,
        palette_kind,
        palette_global_ids,
        packed_data,
        entry_count,
    })
}

fn decode_block_entities(decoder: &mut Decoder<'_>) -> Result<Vec<BlockEntityRecord>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let packed_xz = decoder.read_u8()?;
        let y = decoder.read_i16()?;
        let type_id = decoder.read_var_i32()?;
        let nbt = skip_nbt_any(decoder)?;
        out.push(BlockEntityRecord {
            local_x: packed_xz >> 4,
            y,
            local_z: packed_xz & 0x0f,
            type_id,
            nbt,
        });
    }
    Ok(out)
}

fn read_var_i32_array(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(decoder.read_var_i32()?);
    }
    Ok(out)
}

fn read_long_array(decoder: &mut Decoder<'_>) -> Result<Vec<i64>> {
    let count = decoder.read_len()?;
    read_fixed_long_array(decoder, count)
}

fn read_fixed_long_array(decoder: &mut Decoder<'_>, count: usize) -> Result<Vec<i64>> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(decoder.read_i64()?);
    }
    Ok(out)
}

fn skip_nbt_any(decoder: &mut Decoder<'_>) -> Result<Option<NbtPayloadSummary>> {
    let start = decoder.position();
    let root_type = decoder.read_u8()?;
    if root_type == 0 {
        return Ok(None);
    }
    skip_nbt_payload(decoder, root_type)?;
    Ok(Some(NbtPayloadSummary {
        root_type,
        byte_len: decoder.position() - start,
    }))
}

fn skip_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8) -> Result<()> {
    match tag_id {
        0 => Ok(()),
        1 => {
            decoder.read_exact(1, "nbt byte")?;
            Ok(())
        }
        2 => {
            decoder.read_exact(2, "nbt short")?;
            Ok(())
        }
        3 | 5 => {
            decoder.read_exact(4, "nbt int/float")?;
            Ok(())
        }
        4 | 6 => {
            decoder.read_exact(8, "nbt long/double")?;
            Ok(())
        }
        7 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len, "nbt byte array")?;
            Ok(())
        }
        8 => {
            let len = decoder.read_u16()? as usize;
            decoder.read_exact(len, "nbt string")?;
            Ok(())
        }
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            for _ in 0..len {
                skip_nbt_payload(decoder, element_type)?;
            }
            Ok(())
        }
        10 => {
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    break;
                }
                let name_len = decoder.read_u16()? as usize;
                decoder.read_exact(name_len, "nbt compound name")?;
                skip_nbt_payload(decoder, nested_type)?;
            }
            Ok(())
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len * 4, "nbt int array")?;
            Ok(())
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len * 8, "nbt long array")?;
            Ok(())
        }
        other => Err(WorldDecodeError::InvalidNbtTag(other)),
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(WorldDecodeError::NegativeNbtArrayLength(len));
    }
    Ok(len as usize)
}
