use bbb_protocol::packets::{
    LevelChunkBlockEntity as ProtocolLevelChunkBlockEntity,
    LevelChunkData as ProtocolLevelChunkData, LightUpdateData as ProtocolLightUpdateData,
};
use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID, VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID,
        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
        VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID, VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
        VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID, VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
        VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID, VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
        VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID, VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
    },
    codec::{Decoder, ProtocolError},
    packets::JigsawJoint,
};

use crate::{ChunkPos, Result, WorldDecodeError, WorldDimension};

use super::{
    light::validate_light_data,
    palette::{packed_long_len, palette_kind, PaletteDomain, PaletteKind, PalettedContainerData},
    sign_text::decode_sign_block_entity_text,
    state::{
        BlockEntityRecord, ChunkColumn, ChunkSection, ChunkState, HeightmapData, NbtPayloadSummary,
    },
};

const MAX_CHUNK_SECTION_BUFFER: usize = 2 * 1024 * 1024;

pub fn decode_level_chunk_with_light(
    pos: ChunkPos,
    chunk_data: ProtocolLevelChunkData,
    light_data: ProtocolLightUpdateData,
    dimension: WorldDimension,
) -> Result<ChunkColumn> {
    if chunk_data.section_data.len() > MAX_CHUNK_SECTION_BUFFER {
        return Err(WorldDecodeError::ChunkSectionBufferTooLarge {
            actual: chunk_data.section_data.len(),
            max: MAX_CHUNK_SECTION_BUFFER,
        });
    }
    let heightmaps = chunk_data
        .heightmaps
        .into_iter()
        .map(|heightmap| HeightmapData {
            kind_id: heightmap.kind_id,
            data: heightmap.data,
        })
        .collect();
    let sections = decode_sections(&chunk_data.section_data)?;
    let expected_sections = dimension.section_count();
    if sections.len() != expected_sections {
        return Err(WorldDecodeError::ChunkSectionCountMismatch {
            expected: expected_sections,
            actual: sections.len(),
        });
    }
    let block_entities = decode_block_entities(chunk_data.block_entities, &sections, dimension)?;
    let light = light_data.into();
    validate_light_data(&light, expected_sections.saturating_add(2))?;

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

pub(crate) fn decode_block_entity_data_record(
    local_x: u8,
    y: i16,
    local_z: u8,
    block_entity_type_id: i32,
    raw_nbt: Vec<u8>,
    jigsaw_default_joint: JigsawJoint,
) -> Result<BlockEntityRecord> {
    let Some(nbt) = decode_nbt_payload_summary(&raw_nbt)? else {
        return Err(ProtocolError::InvalidData(
            "block entity data nbt root must be a non-null compound tag".to_string(),
        )
        .into());
    };
    if nbt.root_type != 10 {
        return Err(ProtocolError::InvalidData(format!(
            "block entity data nbt root must be a compound tag, got tag id {}",
            nbt.root_type
        ))
        .into());
    }
    decode_block_entity_record(
        local_x,
        y,
        local_z,
        block_entity_type_id,
        raw_nbt,
        Some(nbt),
        jigsaw_default_joint,
    )
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

fn decode_block_entities(
    block_entities: Vec<ProtocolLevelChunkBlockEntity>,
    sections: &[ChunkSection],
    dimension: WorldDimension,
) -> Result<Vec<BlockEntityRecord>> {
    let mut out = Vec::with_capacity(block_entities.len());
    for entity in block_entities {
        let ProtocolLevelChunkBlockEntity {
            packed_xz,
            y,
            block_entity_type_id,
            raw_nbt,
        } = entity;
        let mut decoder = Decoder::new(&raw_nbt);
        let nbt = skip_nbt_any(&mut decoder)?;
        if !decoder.is_empty() {
            return Err(WorldDecodeError::TrailingBlockEntityData {
                remaining: decoder.remaining_len(),
            });
        }
        let jigsaw_default_joint =
            jigsaw_default_joint_for_chunk_block(sections, dimension, packed_xz, y);
        out.push(decode_block_entity_record(
            packed_xz >> 4,
            y,
            packed_xz & 0x0f,
            block_entity_type_id,
            raw_nbt,
            nbt,
            jigsaw_default_joint,
        )?);
    }
    Ok(out)
}

fn decode_block_entity_record(
    local_x: u8,
    y: i16,
    local_z: u8,
    block_entity_type_id: i32,
    raw_nbt: Vec<u8>,
    nbt: Option<NbtPayloadSummary>,
    jigsaw_default_joint: JigsawJoint,
) -> Result<BlockEntityRecord> {
    let sign_text = if matches!(
        block_entity_type_id,
        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID | VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
    ) {
        decode_sign_block_entity_text(&raw_nbt)?
    } else {
        None
    };
    let vault_shared_data = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID {
        crate::chunks::decode_vault_shared_data(&raw_nbt)?
    } else {
        None
    };
    let campfire = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID {
        crate::chunks::decode_campfire_block_entity_data(&raw_nbt)?
    } else {
        None
    };
    let shelf = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID {
        crate::chunks::decode_shelf_block_entity_data(&raw_nbt)?
    } else {
        None
    };
    let conduit_destroy_target = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID {
        crate::chunks::decode_conduit_destroy_target(&raw_nbt)?
    } else {
        None
    };
    let decorated_pot_sherds = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID
    {
        crate::chunks::decode_decorated_pot_sherds(&raw_nbt)?
    } else {
        None
    };
    let banner_patterns = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID {
        crate::chunks::decode_banner_patterns(&raw_nbt)?
    } else {
        None
    };
    let end_gateway = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID {
        crate::chunks::decode_end_gateway_block_entity_data(&raw_nbt)?
    } else {
        None
    };
    let spawner = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID {
        crate::chunks::decode_spawner_block_entity_data(&raw_nbt)?
    } else {
        None
    };
    let trial_spawner = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID {
        crate::chunks::decode_trial_spawner_block_entity_data(&raw_nbt)?
    } else {
        None
    };
    let command_block = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID {
        Some(crate::command_blocks::decode_command_block_entity_data(
            &raw_nbt,
        )?)
    } else {
        None
    };
    let comparator_output = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID {
        Some(crate::comparator_blocks::decode_comparator_output(
            &raw_nbt,
        )?)
    } else {
        None
    };
    let structure_block = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID {
        Some(crate::structure_blocks::decode_structure_block_entity_data(
            &raw_nbt,
        )?)
    } else {
        None
    };
    let jigsaw_block = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID {
        Some(crate::jigsaw_blocks::decode_jigsaw_block_entity_data(
            &raw_nbt,
            jigsaw_default_joint,
        )?)
    } else {
        None
    };
    let test_block = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID {
        Some(crate::test_blocks::decode_test_block_entity_data(&raw_nbt)?)
    } else {
        None
    };
    let test_instance_block =
        if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID {
            Some(crate::test_instance_blocks::decode_test_instance_block_entity_data(&raw_nbt)?)
        } else {
            None
        };
    let note_block_sound = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID {
        crate::chunks::decode_skull_note_block_sound(&raw_nbt)?
    } else {
        None
    };
    let skull_profile = if block_entity_type_id == VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID {
        crate::chunks::decode_skull_profile(&raw_nbt)?
    } else {
        None
    };
    Ok(BlockEntityRecord {
        local_x,
        y,
        local_z,
        type_id: block_entity_type_id,
        raw_nbt,
        nbt,
        sign_text,
        vault_shared_data,
        campfire,
        shelf,
        conduit_destroy_target,
        decorated_pot_sherds,
        banner_patterns,
        end_gateway,
        spawner,
        trial_spawner,
        command_block,
        comparator_output,
        structure_block,
        jigsaw_block,
        test_block,
        test_instance_block,
        note_block_sound,
        skull_profile,
    })
}

fn jigsaw_default_joint_for_chunk_block(
    sections: &[ChunkSection],
    dimension: WorldDimension,
    packed_xz: u8,
    y: i16,
) -> bbb_protocol::packets::JigsawJoint {
    let y = i32::from(y);
    let section_y = y.div_euclid(16);
    let Some(section_index) = usize::try_from(section_y - dimension.min_section_y()).ok() else {
        return bbb_protocol::packets::JigsawJoint::Rollable;
    };
    let Some(section) = sections.get(section_index) else {
        return bbb_protocol::packets::JigsawJoint::Rollable;
    };
    let local_x = packed_xz >> 4;
    let local_z = packed_xz & 0x0f;
    let block_index = crate::section_block_index(local_x, y.rem_euclid(16) as u8, local_z);
    let block_state_registry = crate::BlockStateRegistry::vanilla_26_1();
    let orientation = section
        .block_states
        .value_at(block_index)
        .and_then(|value| block_state_registry.by_id(value.global_id))
        .and_then(|state| state.properties.get("orientation"))
        .map(String::as_str);
    crate::jigsaw_blocks::jigsaw_default_joint_for_orientation(orientation)
}

fn read_var_i32_array(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(decoder.read_var_i32()?);
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::JigsawJoint;

    use super::*;

    #[test]
    fn jigsaw_chunk_default_joint_uses_decoded_block_orientation() {
        let dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        for (orientation, expected) in [
            ("north_up", JigsawJoint::Aligned),
            ("up_north", JigsawJoint::Rollable),
        ] {
            let block_state_id = crate::vanilla_block_state_id(
                "minecraft:jigsaw",
                &BTreeMap::from([("orientation".to_string(), orientation.to_string())]),
            )
            .unwrap();
            let sections = [ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 0,
                block_states: single_value_palette(
                    PaletteDomain::BlockStates,
                    block_state_id,
                    4096,
                ),
                biomes: single_value_palette(PaletteDomain::Biomes, 0, 64),
            }];
            assert_eq!(
                jigsaw_default_joint_for_chunk_block(&sections, dimension, 0x23, 1),
                expected
            );
        }
    }

    fn single_value_palette(
        domain: PaletteDomain,
        global_id: i32,
        entry_count: usize,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }
}
