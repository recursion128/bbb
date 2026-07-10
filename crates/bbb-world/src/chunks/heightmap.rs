use crate::{
    section_block_index,
    terrain::{classify_terrain_material, terrain_fluid_state},
    ChunkColumn, ChunkPos, HeightmapData, RegistrySet, TerrainMaterialClass, WorldDimension,
    WorldStore,
};

use super::palette::packed_long_len;

pub(super) const VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID: i32 = 4;

const VANILLA_HEIGHTMAP_KIND_COUNT: usize = 6;
const VANILLA_HEIGHTMAP_ENTRY_COUNT: usize = 16 * 16;

/// A pre-resolved view of one loaded chunk's vanilla heightmaps.
///
/// Packet entries are indexed once when the sampler is built. Duplicate kind
/// ids use the final packet entry, matching vanilla's `Map.put` behavior.
#[derive(Debug, Clone, Copy)]
pub struct ChunkHeightmapSampler<'a> {
    heightmaps: [Option<&'a HeightmapData>; VANILLA_HEIGHTMAP_KIND_COUNT],
    dimension: WorldDimension,
}

impl<'a> ChunkHeightmapSampler<'a> {
    fn new(chunk: &'a ChunkColumn, dimension: WorldDimension) -> Self {
        let mut heightmaps = [None; VANILLA_HEIGHTMAP_KIND_COUNT];
        for heightmap in &chunk.heightmaps {
            let Ok(kind_index) = usize::try_from(heightmap.kind_id) else {
                continue;
            };
            let Some(entry) = heightmaps.get_mut(kind_index) else {
                continue;
            };
            *entry = Some(heightmap);
        }
        Self {
            heightmaps,
            dimension,
        }
    }

    /// Vanilla heightmap kind ids that effectively exist on this full chunk,
    /// in `Heightmap.Types` ordinal order.
    ///
    /// The four final heightmaps exist as zero-filled maps even when omitted
    /// from the packet. World-generation maps are included only when present.
    pub fn effective_kind_ids(&self) -> impl Iterator<Item = i32> + '_ {
        (0..VANILLA_HEIGHTMAP_KIND_COUNT)
            .filter(|&kind_index| {
                self.heightmaps[kind_index].is_some()
                    || is_vanilla_final_heightmap(kind_index as i32)
            })
            .map(|kind_index| kind_index as i32)
    }

    /// Samples the first available block y for a local chunk column.
    ///
    /// Missing final maps behave like vanilla's zero-filled maps. An actual
    /// malformed packet map returns `None` and is never replaced by that
    /// fallback because bbb cannot reconstruct its contents here.
    pub fn first_available(&self, kind_id: i32, local_x: u8, local_z: u8) -> Option<i32> {
        if local_x >= 16 || local_z >= 16 {
            return None;
        }
        let kind_index = usize::try_from(kind_id).ok()?;
        let heightmap = *self.heightmaps.get(kind_index)?;
        let index = heightmap_index(local_x, local_z);
        match heightmap {
            Some(heightmap) => heightmap_first_available(heightmap, self.dimension, index),
            None if is_vanilla_final_heightmap(kind_id) => {
                heightmap_bits(self.dimension)?;
                Some(self.dimension.min_y)
            }
            None => None,
        }
    }
}

impl WorldStore {
    /// Builds a borrowed heightmap sampler after locating the chunk once.
    pub fn chunk_heightmap_sampler(&self, pos: ChunkPos) -> Option<ChunkHeightmapSampler<'_>> {
        Some(ChunkHeightmapSampler::new(
            self.probe_chunk(pos)?,
            self.dimension,
        ))
    }
}

pub(super) fn chunk_heightmap_first_available(
    chunk: &ChunkColumn,
    kind_id: i32,
    dimension: WorldDimension,
    index: usize,
) -> Option<i32> {
    let heightmap = chunk
        .heightmaps
        .iter()
        .rfind(|map| map.kind_id == kind_id)?;
    heightmap_first_available(heightmap, dimension, index)
}

pub(super) fn update_motion_blocking_heightmap_for_block(
    chunk: &mut ChunkColumn,
    dimension: WorldDimension,
    registries: &RegistrySet,
    local_x: u8,
    y: i32,
    local_z: u8,
    block_state_id: i32,
) {
    let index = heightmap_index(local_x, local_z);
    let Some(bits) = heightmap_bits(dimension) else {
        return;
    };
    let Some(heightmap_index) = chunk
        .heightmaps
        .iter()
        .rposition(|map| map.kind_id == VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID)
    else {
        return;
    };
    if !valid_heightmap_data_len(&chunk.heightmaps[heightmap_index], bits) {
        return;
    }

    let Some(first_available) = chunk_heightmap_first_available(
        chunk,
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        dimension,
        index,
    ) else {
        return;
    };
    if y <= first_available - 2 {
        return;
    }

    if block_state_id_blocks_motion_or_fluid(registries, block_state_id) {
        if y >= first_available {
            set_heightmap_first_available(
                &mut chunk.heightmaps[heightmap_index],
                dimension,
                bits,
                index,
                y + 1,
            );
        }
    } else if first_available - 1 == y {
        let next_height = ((dimension.min_y)..y)
            .rev()
            .find(|candidate_y| {
                chunk_block_at_blocks_motion_or_fluid(
                    chunk,
                    dimension,
                    registries,
                    local_x,
                    *candidate_y,
                    local_z,
                )
            })
            .map(|candidate_y| candidate_y + 1)
            .unwrap_or(dimension.min_y);
        set_heightmap_first_available(
            &mut chunk.heightmaps[heightmap_index],
            dimension,
            bits,
            index,
            next_height,
        );
    }
}

fn heightmap_first_available(
    heightmap: &HeightmapData,
    dimension: WorldDimension,
    index: usize,
) -> Option<i32> {
    let bits = heightmap_bits(dimension)?;
    if !valid_heightmap_data_len(heightmap, bits) {
        return None;
    }
    let raw = read_heightmap_value(&heightmap.data, bits, index)?;
    let raw = i32::try_from(raw).ok()?;
    dimension.min_y.checked_add(raw)
}

fn chunk_block_at_blocks_motion_or_fluid(
    chunk: &ChunkColumn,
    dimension: WorldDimension,
    registries: &RegistrySet,
    local_x: u8,
    y: i32,
    local_z: u8,
) -> bool {
    if !dimension.contains_y(y) {
        return false;
    }
    let section_y = y.div_euclid(16);
    let Ok(section_index) = usize::try_from(section_y - dimension.min_section_y()) else {
        return false;
    };
    let Some(section) = chunk.sections.get(section_index) else {
        return false;
    };
    let local_y = y.rem_euclid(16) as u8;
    let block_index = section_block_index(local_x, local_y, local_z);
    let Some(block_value) = section.block_states.value_at(block_index) else {
        return false;
    };
    block_state_id_blocks_motion_or_fluid(registries, block_value.global_id)
}

fn block_state_id_blocks_motion_or_fluid(registries: &RegistrySet, block_state_id: i32) -> bool {
    let Some(block_state) = registries.block_state(block_state_id) else {
        return true;
    };
    let material = classify_terrain_material(Some(block_state.name.as_str()));
    let fluid = terrain_fluid_state(Some(block_state.name.as_str()), &block_state.properties);
    !matches!(
        material,
        TerrainMaterialClass::Empty | TerrainMaterialClass::Invisible
    ) || fluid.is_some()
}

fn set_heightmap_first_available(
    heightmap: &mut HeightmapData,
    dimension: WorldDimension,
    bits: u8,
    index: usize,
    first_available: i32,
) -> bool {
    let raw = first_available - dimension.min_y;
    let Ok(raw) = u64::try_from(raw) else {
        return false;
    };
    set_heightmap_value(&mut heightmap.data, bits, index, raw)
}

fn valid_heightmap_data_len(heightmap: &HeightmapData, bits: u8) -> bool {
    heightmap.data.len() == packed_long_len(VANILLA_HEIGHTMAP_ENTRY_COUNT, usize::from(bits))
}

pub(super) fn heightmap_index(local_x: u8, local_z: u8) -> usize {
    usize::from(local_x) + usize::from(local_z) * 16
}

fn heightmap_bits(dimension: WorldDimension) -> Option<u8> {
    if dimension.height <= 0 {
        return None;
    }
    let value = u64::try_from(dimension.height).ok()?.checked_add(1)?;
    Some(ceil_log2(value).max(1))
}

fn ceil_log2(value: u64) -> u8 {
    if value <= 1 {
        0
    } else {
        u8::try_from(u64::BITS - (value - 1).leading_zeros()).unwrap_or(u8::MAX)
    }
}

fn read_heightmap_value(packed_data: &[i64], bits_per_entry: u8, index: usize) -> Option<u64> {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return None;
    }
    let bits = usize::from(bits_per_entry);
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return None;
    }
    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let cell = *packed_data.get(cell_index)? as u64;
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    Some((cell >> bit_index) & mask)
}

fn set_heightmap_value(
    packed_data: &mut [i64],
    bits_per_entry: u8,
    index: usize,
    value: u64,
) -> bool {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return false;
    }
    let bits = usize::from(bits_per_entry);
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return false;
    }
    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let Some(cell) = packed_data.get_mut(cell_index) else {
        return false;
    };
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if value & !mask != 0 {
        return false;
    }
    let raw = *cell as u64;
    *cell = ((raw & !(mask << bit_index)) | (value << bit_index)) as i64;
    true
}

fn is_vanilla_final_heightmap(kind_id: i32) -> bool {
    matches!(kind_id, 1 | 3 | 4 | 5)
}
