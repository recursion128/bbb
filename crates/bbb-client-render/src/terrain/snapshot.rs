//! Compact paletted chunk views sampled directly during terrain meshing.

use std::sync::Arc;

use bbb_render_types::{
    PackedLightArray, PalettedContainerData, TerrainChunkSource, TerrainLight,
    TerrainSectionLightRef, TerrainSectionView, LIGHT_ARRAY_BYTES,
};

use super::TerrainCardinalLighting;

#[derive(Debug, Clone)]
enum FixtureSectionLight {
    Uniform(u8),
    Packed(PackedLightArray),
}

impl FixtureSectionLight {
    fn from_nibbles(bytes: [u8; LIGHT_ARRAY_BYTES], default_nibble: u8) -> Self {
        let first = bytes[0];
        if (first & 0x0f) == (first >> 4) && bytes.iter().all(|&byte| byte == first) {
            return Self::Uniform(first & 0x0f);
        }
        let _ = default_nibble;
        Self::Packed(PackedLightArray::from(bytes))
    }

    #[inline]
    fn as_ref(&self) -> TerrainSectionLightRef<'_> {
        match self {
            Self::Uniform(value) => TerrainSectionLightRef::Uniform(*value),
            Self::Packed(bytes) => TerrainSectionLightRef::Packed(&bytes.0),
        }
    }
}

#[derive(Debug)]
struct FixturePalettedSection {
    block_states: PalettedContainerData,
    sky_light: FixtureSectionLight,
    block_light: FixtureSectionLight,
}

#[derive(Debug)]
struct FixturePalettedChunk {
    sections: Box<[FixturePalettedSection]>,
}

impl TerrainChunkSource for FixturePalettedChunk {
    #[inline]
    fn section(&self, section_index: usize) -> Option<TerrainSectionView<'_>> {
        let section = self.sections.get(section_index)?;
        Some(TerrainSectionView {
            block_states: &section.block_states,
            biomes: None,
            sky_light: section.sky_light.as_ref(),
            block_light: section.block_light.as_ref(),
        })
    }
}

#[inline]
fn section_block_index(x: usize, y: usize, z: usize) -> usize {
    (y << 8) | (z << 4) | x
}

#[inline]
fn write_nibble(bytes: &mut [u8; LIGHT_ARRAY_BYTES], block_index: usize, value: u8) {
    let shift = (block_index & 1) << 2;
    let slot = &mut bytes[block_index >> 1];
    *slot = (*slot & !(0x0f << shift)) | ((value.min(15)) << shift);
}

/// One chunk's compact paletted block, biome and light view for meshing.
#[derive(Debug, Clone)]
pub struct TerrainChunkSnapshot {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub min_y: i32,
    pub height: usize,
    source_min_section_y: i32,
    source: Arc<dyn TerrainChunkSource>,
    pub cardinal_lighting: TerrainCardinalLighting,
}

impl TerrainChunkSnapshot {
    pub fn from_source(
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: usize,
        source_min_section_y: i32,
        source: Arc<dyn TerrainChunkSource>,
        cardinal_lighting: TerrainCardinalLighting,
    ) -> Self {
        Self {
            chunk_x,
            chunk_z,
            min_y,
            height,
            source_min_section_y,
            source,
            cardinal_lighting,
        }
    }

    pub fn from_cells(
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: usize,
        cells: impl IntoIterator<Item = (u32, TerrainLight)>,
    ) -> Self {
        let cells: Vec<(u32, TerrainLight)> = cells.into_iter().collect();
        assert_eq!(cells.len(), 16 * height * 16);
        let source_min_section_y = min_y.div_euclid(16);
        let section_count = if height == 0 {
            0
        } else {
            let max_world_y = min_y + height as i32 - 1;
            (max_world_y.div_euclid(16) - source_min_section_y + 1) as usize
        };
        let mut sections = Vec::with_capacity(section_count);
        for section_offset in 0..section_count {
            let section_y = source_min_section_y + section_offset as i32;
            let section_min_y = section_y * 16;
            let mut ids = [0u32; 4096];
            let mut sky = [0xffu8; LIGHT_ARRAY_BYTES];
            let mut block = [0u8; LIGHT_ARRAY_BYTES];
            for local_y in 0..16_usize {
                let world_y = section_min_y + local_y as i32;
                if world_y < min_y || i64::from(world_y) >= i64::from(min_y) + height as i64 {
                    continue;
                }
                let y_in_chunk = (world_y - min_y) as usize;
                for z in 0..16_usize {
                    for x in 0..16_usize {
                        let (state_id, light) =
                            cells[super::mesh::cell_index(x, y_in_chunk, z, height)];
                        let block_index = section_block_index(x, local_y, z);
                        ids[block_index] = state_id;
                        write_nibble(&mut sky, block_index, light.sky);
                        write_nibble(&mut block, block_index, light.block);
                    }
                }
            }
            sections.push(FixturePalettedSection {
                block_states: PalettedContainerData::from_block_state_ids(&ids),
                sky_light: FixtureSectionLight::from_nibbles(sky, 15),
                block_light: FixtureSectionLight::from_nibbles(block, 0),
            });
        }
        Self {
            chunk_x,
            chunk_z,
            min_y,
            height,
            source_min_section_y,
            source: Arc::new(FixturePalettedChunk {
                sections: sections.into_boxed_slice(),
            }),
            cardinal_lighting: TerrainCardinalLighting::Default,
        }
    }

    pub fn with_cardinal_lighting(mut self, cardinal_lighting: TerrainCardinalLighting) -> Self {
        self.cardinal_lighting = cardinal_lighting;
        self
    }

    #[inline]
    pub(crate) fn contains_world_y(&self, world_y: i32) -> bool {
        world_y >= self.min_y && i64::from(world_y) < i64::from(self.min_y) + self.height as i64
    }

    #[inline]
    pub(crate) fn section_view(&self, section_y: i32) -> Option<TerrainSectionView<'_>> {
        let index = usize::try_from(section_y.checked_sub(self.source_min_section_y)?).ok()?;
        self.source.section(index)
    }

    #[inline]
    pub(crate) fn uniform_block_state_id(&self, section_y: i32) -> Option<u32> {
        let section_min_y = section_y.checked_mul(16)?;
        if !self.contains_world_y(section_min_y)
            || !self.contains_world_y(section_min_y.checked_add(15)?)
        {
            return None;
        }
        self.section_view(section_y)?.uniform_block_state_id()
    }
}
