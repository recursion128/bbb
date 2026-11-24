use std::sync::Arc;

use rustc_hash::FxHashMap;

use bbb_protocol::packets::{
    LevelChunkBlockEntity as ProtocolLevelChunkBlockEntity,
    LevelChunkData as ProtocolLevelChunkData, LightUpdateData as ProtocolLightUpdateData,
};
use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID, VANILLA_BLOCK_ENTITY_TYPE_BRUSHABLE_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID, VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID, VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID,
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID, VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID,
        VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID, VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
        VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID,
        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID, VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID,
        VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
        VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
    },
    codec::{Decoder, ProtocolError},
    content_registry::ContentRegistry,
    packets::JigsawJoint,
};

use crate::{ChunkPos, Result, WorldDecodeError, WorldDimension};

use super::{
    light::LightData,
    palette::{packed_long_len, palette_kind, PaletteDomain, PaletteKind, PalettedContainerData},
    sign_text::decode_sign_block_entity_text,
    state::{
        BlockEntityPayload, BlockEntityRecord, ChunkColumn, ChunkSection, ChunkState,
        HeightmapData, NbtPayloadSummary,
    },
};

const MAX_CHUNK_SECTION_BUFFER: usize = 2 * 1024 * 1024;
const MAX_SKIP_NBT_DEPTH: usize = 512;
/// Vanilla horizontal world bound (`30_000_000` blocks = `1_875_000` chunks).
const MAX_WORLD_CHUNK_COORD: i32 = 1_875_000;

pub fn decode_level_chunk_with_light(
    pos: ChunkPos,
    chunk_data: ProtocolLevelChunkData,
    light_data: ProtocolLightUpdateData,
    dimension: WorldDimension,
) -> Result<ChunkColumn> {
    decode_level_chunk_with_light_with_registry(
        pos,
        chunk_data,
        light_data,
        dimension,
        ContentRegistry::vanilla(),
    )
}

pub fn decode_level_chunk_with_light_with_registry(
    pos: ChunkPos,
    chunk_data: ProtocolLevelChunkData,
    light_data: ProtocolLightUpdateData,
    dimension: WorldDimension,
    content_registry: &ContentRegistry,
) -> Result<ChunkColumn> {
    if pos.x.abs_diff(0) > MAX_WORLD_CHUNK_COORD as u32
        || pos.z.abs_diff(0) > MAX_WORLD_CHUNK_COORD as u32
    {
        return Err(ProtocolError::InvalidData(format!(
            "chunk coordinates ({}, {}) outside world bounds",
            pos.x, pos.z
        ))
        .into());
    }
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
    let sections = decode_sections(&chunk_data.section_data, content_registry)?;
    let expected_sections = dimension.section_count();
    if sections.len() != expected_sections {
        return Err(WorldDecodeError::ChunkSectionCountMismatch {
            expected: expected_sections,
            actual: sections.len(),
        });
    }
    let block_entities = decode_block_entities(chunk_data.block_entities, &sections, dimension)?;
    let light = LightData::try_from_protocol(light_data, expected_sections.saturating_add(2))?;

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
            ContentRegistry::vanilla(),
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
    let Some(&root_tag) = raw_nbt.first() else {
        return Err(ProtocolError::UnexpectedEof { what: "u8" }.into());
    };
    if root_tag == 0 {
        // Validate there are no trailing bytes on TAG_End before reporting null root.
        let _ = decode_nbt_payload_summary(&raw_nbt)?;
        return Err(ProtocolError::InvalidData(
            "block entity data nbt root must be a non-null compound tag".to_string(),
        )
        .into());
    }
    if root_tag != 10 {
        let _ = decode_nbt_payload_summary(&raw_nbt)?;
        return Err(ProtocolError::InvalidData(format!(
            "block entity data nbt root must be a compound tag, got tag id {root_tag}"
        ))
        .into());
    }
    decode_block_entity_record_single_pass(
        local_x,
        y,
        local_z,
        block_entity_type_id,
        raw_nbt,
        jigsaw_default_joint,
    )
}

fn decode_sections(
    bytes: &[u8],
    content_registry: &ContentRegistry,
) -> Result<Vec<Arc<ChunkSection>>> {
    let mut decoder = Decoder::new(bytes);
    let mut sections = Vec::new();
    while !decoder.is_empty() {
        sections.push(Arc::new(decode_section(&mut decoder, content_registry)?));
    }
    Ok(sections)
}

fn decode_section(
    decoder: &mut Decoder<'_>,
    content_registry: &ContentRegistry,
) -> Result<ChunkSection> {
    let non_empty_block_count = decoder.read_i16()?;
    let fluid_count = decoder.read_i16()?;
    let block_states =
        decode_paletted_container(decoder, PaletteDomain::BlockStates, content_registry)?;
    let biomes = decode_paletted_container(decoder, PaletteDomain::Biomes, content_registry)?;
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
    content_registry: &ContentRegistry,
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
        PaletteKind::Local => {
            let max_entries = 1usize.checked_shl(u32::from(bits_per_entry)).unwrap_or(0);
            read_var_i32_array(decoder, max_entries)?
        }
        PaletteKind::Global => Vec::new(),
    };
    let packed_data_len = packed_long_len(entry_count, bits_per_entry as usize);
    let packed_data = read_fixed_long_array(decoder, packed_data_len)?;

    let container = PalettedContainerData {
        domain,
        bits_per_entry,
        palette_kind,
        palette_global_ids,
        packed_data,
        entry_count,
    };

    if container.palette_kind == PaletteKind::Local {
        for index in 0..container.entry_count {
            if container.value_at(index).is_none() {
                return Err(ProtocolError::InvalidData(format!(
                    "chunk local palette entry at index {index} out of range (palette len {})",
                    container.palette_global_ids.len()
                ))
                .into());
            }
        }
    }

    if domain == PaletteDomain::BlockStates {
        let total_states = content_registry.total_block_state_count();
        match container.palette_kind {
            PaletteKind::SingleValue | PaletteKind::Local => {
                for &state_id in &container.palette_global_ids {
                    if !(0..total_states).contains(&state_id) {
                        return Err(ProtocolError::InvalidData(format!(
                            "chunk palette block state id {state_id} outside 0..{total_states}"
                        ))
                        .into());
                    }
                }
            }
            PaletteKind::Global => {
                for index in 0..container.entry_count {
                    let Some(value) = container.value_at(index) else {
                        return Err(ProtocolError::InvalidData(
                            "chunk global palette entry out of range".to_string(),
                        )
                        .into());
                    };
                    if !(0..total_states).contains(&value.global_id) {
                        return Err(ProtocolError::InvalidData(format!(
                            "chunk global block state id {} outside 0..{total_states}",
                            value.global_id
                        ))
                        .into());
                    }
                }
            }
        }
    }

    Ok(container)
}

// Cap unique block entities decoded from a single untrusted chunk packet to
// the physical volume of its sections (up to 98,304 for a 24-section column).
const MAX_BLOCK_ENTITIES_PER_CHUNK: usize = 98_304;

fn decode_block_entities(
    block_entities: Vec<ProtocolLevelChunkBlockEntity>,
    sections: &[Arc<ChunkSection>],
    dimension: WorldDimension,
) -> Result<Arc<Vec<BlockEntityRecord>>> {
    let max_entries = sections
        .len()
        .max(1)
        .saturating_mul(4096)
        .min(MAX_BLOCK_ENTITIES_PER_CHUNK);
    // Deduplicate by (packed_xz, y) before decoding NBT so duplicate-coordinate
    // chunk bombs cannot force millions of redundant NBT parses or records.
    // Vanilla `LevelChunk.replaceWithPacketData` stores into a BlockPos map
    // (last entry wins).
    let mut deduped: Vec<ProtocolLevelChunkBlockEntity> =
        Vec::with_capacity(block_entities.len().min(max_entries));
    let mut seen = FxHashMap::<u32, usize>::with_capacity_and_hasher(
        block_entities.len().min(max_entries),
        Default::default(),
    );
    for entity in block_entities {
        let key = (u32::from(entity.y as u16) << 8) | u32::from(entity.packed_xz);
        if let Some(&slot) = seen.get(&key) {
            deduped[slot] = entity;
        } else if deduped.len() < max_entries {
            let slot = deduped.len();
            seen.insert(key, slot);
            deduped.push(entity);
        }
    }

    let mut out = Vec::with_capacity(deduped.len());
    for entity in deduped {
        let ProtocolLevelChunkBlockEntity {
            packed_xz,
            y,
            block_entity_type_id,
            raw_nbt,
        } = entity;
        let jigsaw_default_joint =
            jigsaw_default_joint_for_chunk_block(sections, dimension, packed_xz, y);
        out.push(decode_block_entity_record_single_pass(
            packed_xz >> 4,
            y,
            packed_xz & 0x0f,
            block_entity_type_id,
            raw_nbt,
            jigsaw_default_joint,
        )?);
    }
    Ok(Arc::new(out))
}

fn nbt_summary_for_validated_bytes(raw_nbt: &[u8]) -> Option<NbtPayloadSummary> {
    let &root_type = raw_nbt.first()?;
    if root_type == 0 {
        None
    } else {
        Some(NbtPayloadSummary {
            root_type,
            byte_len: raw_nbt.len(),
        })
    }
}

fn decode_block_entity_record_single_pass(
    local_x: u8,
    y: i16,
    local_z: u8,
    block_entity_type_id: i32,
    raw_nbt: Vec<u8>,
    jigsaw_default_joint: JigsawJoint,
) -> Result<BlockEntityRecord> {
    let (nbt, payload) = match block_entity_type_id {
        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID | VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID => {
            let payload = decode_sign_block_entity_text(&raw_nbt)?
                .map(|data| BlockEntityPayload::SignText(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID => {
            let payload = crate::chunks::decode_vault_shared_data(&raw_nbt)?
                .map(|data| BlockEntityPayload::VaultSharedData(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID => {
            let payload = crate::chunks::decode_campfire_block_entity_data(&raw_nbt)?
                .map(|data| BlockEntityPayload::Campfire(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID => {
            let payload = crate::chunks::decode_shelf_block_entity_data(&raw_nbt)?
                .map(|data| BlockEntityPayload::Shelf(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_BRUSHABLE_BLOCK_ID => {
            let payload = crate::chunks::decode_brushable_block_entity_data(&raw_nbt)?
                .map(|data| BlockEntityPayload::Brushable(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_CONDUIT_ID => {
            let payload = crate::chunks::decode_conduit_destroy_target(&raw_nbt)?
                .map(BlockEntityPayload::ConduitDestroyTarget)
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID => {
            let payload = crate::chunks::decode_decorated_pot_sherds(&raw_nbt)?
                .map(|data| BlockEntityPayload::DecoratedPotSherds(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID => {
            let payload = crate::chunks::decode_banner_patterns(&raw_nbt)?
                .map(|data| BlockEntityPayload::BannerPatterns(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID => {
            let payload = crate::chunks::decode_end_gateway_block_entity_data(&raw_nbt)?
                .map(BlockEntityPayload::EndGateway)
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID => {
            let payload = crate::chunks::decode_spawner_block_entity_data(&raw_nbt)?
                .map(|data| BlockEntityPayload::Spawner(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID => {
            let payload = crate::chunks::decode_trial_spawner_block_entity_data(&raw_nbt)?
                .map(|data| BlockEntityPayload::TrialSpawner(Box::new(data)))
                .unwrap_or(BlockEntityPayload::None);
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID => {
            let payload = BlockEntityPayload::CommandBlock(Box::new(
                crate::command_blocks::decode_command_block_entity_data(&raw_nbt)?,
            ));
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID => {
            let payload = BlockEntityPayload::ComparatorOutput(
                crate::comparator_blocks::decode_comparator_output(&raw_nbt)?,
            );
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID => {
            let payload = BlockEntityPayload::StructureBlock(Box::new(
                crate::structure_blocks::decode_structure_block_entity_data(&raw_nbt)?,
            ));
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID => {
            let payload = BlockEntityPayload::JigsawBlock(Box::new(
                crate::jigsaw_blocks::decode_jigsaw_block_entity_data(
                    &raw_nbt,
                    jigsaw_default_joint,
                )?,
            ));
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID => {
            let payload = BlockEntityPayload::TestBlock(Box::new(
                crate::test_blocks::decode_test_block_entity_data(&raw_nbt)?,
            ));
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID => {
            let payload = BlockEntityPayload::TestInstanceBlock(Box::new(
                crate::test_instance_blocks::decode_test_instance_block_entity_data(&raw_nbt)?,
            ));
            (nbt_summary_for_validated_bytes(&raw_nbt), payload)
        }
        VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID => {
            let (note_block_sound, skull_profile) =
                crate::chunks::decode_skull_block_entity_payload(&raw_nbt)?;
            (
                nbt_summary_for_validated_bytes(&raw_nbt),
                BlockEntityPayload::skull(note_block_sound, skull_profile),
            )
        }
        _ => (
            decode_nbt_payload_summary(&raw_nbt)?,
            BlockEntityPayload::None,
        ),
    };
    Ok(BlockEntityRecord {
        local_x,
        y,
        local_z,
        type_id: block_entity_type_id,
        raw_nbt,
        nbt,
        payload,
    })
}

fn jigsaw_default_joint_for_chunk_block(
    sections: &[Arc<ChunkSection>],
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

fn read_var_i32_array(decoder: &mut Decoder<'_>, max_entries: usize) -> Result<Vec<i32>> {
    let count = decoder.read_len()?;
    if count > max_entries || count > decoder.remaining_len() {
        return Err(ProtocolError::InvalidData(format!(
            "chunk palette length {count} exceeds bound (max {max_entries}, remaining {})",
            decoder.remaining_len()
        ))
        .into());
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(decoder.read_var_i32()?);
    }
    Ok(out)
}

fn read_fixed_long_array(decoder: &mut Decoder<'_>, count: usize) -> Result<Vec<i64>> {
    let (words, _) = decoder.read_exact(count * 8, "i64")?.as_chunks::<8>();
    Ok(words.iter().copied().map(i64::from_be_bytes).collect())
}

fn skip_nbt_any(decoder: &mut Decoder<'_>) -> Result<Option<NbtPayloadSummary>> {
    let start = decoder.position();
    let root_type = decoder.read_u8()?;
    if root_type == 0 {
        return Ok(None);
    }
    skip_nbt_payload(decoder, root_type, 0)?;
    Ok(Some(NbtPayloadSummary {
        root_type,
        byte_len: decoder.position() - start,
    }))
}

fn skip_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Result<()> {
    if depth > MAX_SKIP_NBT_DEPTH {
        return Err(ProtocolError::InvalidData("nbt exceeded max depth".to_string()).into());
    }
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
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty nbt list has end tag element type".to_string(),
                )
                .into());
            }
            for _ in 0..len {
                skip_nbt_payload(decoder, element_type, depth + 1)?;
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
                skip_nbt_payload(decoder, nested_type, depth + 1)?;
            }
            Ok(())
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt int array")?;
            Ok(())
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt long array")?;
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
            let sections = [Arc::new(ChunkSection {
                non_empty_block_count: 4096,
                fluid_count: 0,
                block_states: single_value_palette(
                    PaletteDomain::BlockStates,
                    block_state_id,
                    4096,
                ),
                biomes: single_value_palette(PaletteDomain::Biomes, 0, 64),
            })];
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

    #[test]
    fn skip_nbt_payload_guards_depth_tag_end_list_and_array_overflow() {
        // Depth cap exceeded: 600 nested TAG_List(TAG_List, len=1)
        let mut deep = Vec::new();
        for _ in 0..600 {
            deep.push(9); // element_type = TAG_List
            deep.extend_from_slice(&1_i32.to_be_bytes());
        }
        let mut decoder = Decoder::new(&deep);
        assert!(skip_nbt_payload(&mut decoder, 9, 0).is_err());

        // TAG_List with element_type = TAG_End and non-zero length must be rejected.
        let mut bad_end_list = vec![0u8];
        bad_end_list.extend_from_slice(&3_i32.to_be_bytes());
        let mut decoder = Decoder::new(&bad_end_list);
        assert!(skip_nbt_payload(&mut decoder, 9, 0).is_err());

        // TAG_List with element_type = TAG_End and length = 0 is valid.
        let mut empty_end_list = vec![0u8];
        empty_end_list.extend_from_slice(&0_i32.to_be_bytes());
        let mut decoder = Decoder::new(&empty_end_list);
        assert!(skip_nbt_payload(&mut decoder, 9, 0).is_ok());

        // TAG_Int_Array / TAG_Long_Array huge length must fail cleanly without panicking.
        let huge = i32::MAX.to_be_bytes();
        let mut decoder = Decoder::new(&huge);
        assert!(skip_nbt_payload(&mut decoder, 11, 0).is_err());
        let mut decoder = Decoder::new(&huge);
        assert!(skip_nbt_payload(&mut decoder, 12, 0).is_err());
    }

    #[test]
    fn chunk_palette_validation_accepts_modded_and_rejects_out_of_range_state_ids() {
        use bbb_protocol::codec::Encoder;
        use bbb_protocol::content_registry::{ContentRegistryBuilder, ModBlockRegistration};

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "mod_test:chunk_block".into(),
            state_count: 1,
            light_emission: 0,
            destroy_speed: 1.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        let registry = builder.freeze_for_session().unwrap();
        let mod_state_id = registry.blocks().first().unwrap().first_state_id;

        let encode_single_value = |state_id: i32| {
            let mut enc = Encoder::new();
            enc.write_u8(0); // bits_per_entry = 0 => SingleValue
            enc.write_var_i32(state_id);
            enc.into_inner()
        };

        let valid_bytes = encode_single_value(mod_state_id);
        let mut dec = Decoder::new(&valid_bytes);
        assert!(decode_paletted_container(&mut dec, PaletteDomain::BlockStates, &registry).is_ok());
        // Vanilla-only registry rejects modded state ID.
        let mut dec = Decoder::new(&valid_bytes);
        assert!(decode_paletted_container(
            &mut dec,
            PaletteDomain::BlockStates,
            ContentRegistry::vanilla()
        )
        .is_err());

        let out_of_range_bytes = encode_single_value(mod_state_id + 99);
        let mut dec = Decoder::new(&out_of_range_bytes);
        assert!(
            decode_paletted_container(&mut dec, PaletteDomain::BlockStates, &registry).is_err()
        );

        let negative_bytes = encode_single_value(-1);
        let mut dec = Decoder::new(&negative_bytes);
        assert!(
            decode_paletted_container(&mut dec, PaletteDomain::BlockStates, &registry).is_err()
        );
    }
}
