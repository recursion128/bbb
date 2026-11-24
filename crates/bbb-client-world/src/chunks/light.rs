use bbb_protocol::chunk_light::light_storage::LightDataLayer;
use bbb_protocol::packets::LightUpdateData;
pub use bbb_render_types::{PackedLightArray, TerrainSectionLightRef, LIGHT_ARRAY_BYTES};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    section_block_index, ChunkPos, Result, TerrainLight, WorldDecodeError, WorldDimension,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueuedLightUpdate {
    pub(crate) pos: ChunkPos,
    pub(crate) light: LightData,
    /// Vanilla queues two closure shapes: the chunk-load closure (apply light,
    /// enable the chunk, mark its 3x3 neighborhood dirty, report it ready to
    /// render) and the standalone light-update closure (apply light, mark only
    /// the changed light sections).
    pub(crate) initial_chunk_load: bool,
    pub(crate) counted_packet: bool,
}

/// Process-local pending work. Cloning a WorldStore builds a publication
/// generation, so pending closures deliberately do not cross that boundary.
#[derive(Debug, Default)]
pub(crate) struct RuntimeLightUpdateQueue(VecDeque<QueuedLightUpdate>);

impl Clone for RuntimeLightUpdateQueue {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Deref for RuntimeLightUpdateQueue {
    type Target = VecDeque<QueuedLightUpdate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RuntimeLightUpdateQueue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LightSectionLayer {
    #[default]
    Uninitialized,
    Empty,
    Packed(PackedLightArray),
}

const fn default_has_sky_light() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightData {
    pub sky_sections: Vec<LightSectionLayer>,
    pub block_sections: Vec<LightSectionLayer>,
    #[serde(default = "default_has_sky_light")]
    has_sky_light: bool,
    /// Cached vertically extruded bottom `(x, 0, z)` sky slices for Overworld
    /// `Uninitialized` sections below a non-uniform `Packed` section above
    /// (`SkyLightSectionStorage.java:30-53`).
    #[serde(skip)]
    inherited_sky_sections: Vec<Option<Arc<PackedLightArray>>>,
}

impl Default for LightData {
    fn default() -> Self {
        Self {
            sky_sections: Vec::new(),
            block_sections: Vec::new(),
            has_sky_light: true,
            inherited_sky_sections: Vec::new(),
        }
    }
}

impl PartialEq for LightData {
    fn eq(&self, other: &Self) -> bool {
        self.sky_sections == other.sky_sections
            && self.block_sections == other.block_sections
            && self.has_sky_light == other.has_sky_light
    }
}

impl Eq for LightData {}

impl LightData {
    pub fn try_from_protocol(update: LightUpdateData, section_count: usize) -> Result<Self> {
        validate_light_update_data(&update, section_count)?;
        Ok(Self::from_protocol_unchecked(update))
    }

    pub fn from_protocol_unchecked(update: LightUpdateData) -> Self {
        let mut light = Self {
            sky_sections: build_sections_from_protocol(
                &update.sky_y_mask,
                &update.empty_sky_y_mask,
                update.sky_updates,
            ),
            block_sections: build_sections_from_protocol(
                &update.block_y_mask,
                &update.empty_block_y_mask,
                update.block_updates,
            ),
            has_sky_light: true,
            inherited_sky_sections: Vec::new(),
        };
        light.rebuild_inherited_sky_cache();
        light
    }

    pub fn has_sky_light(&self) -> bool {
        self.has_sky_light
    }

    pub fn set_has_sky_light(&mut self, has_sky_light: bool) {
        if self.has_sky_light == has_sky_light {
            return;
        }
        self.has_sky_light = has_sky_light;
        self.rebuild_inherited_sky_cache();
    }

    pub fn set_sky_packed(&mut self, section_index: usize, bytes: impl Into<PackedLightArray>) {
        set_section_layer(
            &mut self.sky_sections,
            section_index,
            LightSectionLayer::Packed(bytes.into()),
        );
        self.rebuild_inherited_sky_cache();
    }

    pub fn set_block_packed(&mut self, section_index: usize, bytes: impl Into<PackedLightArray>) {
        set_section_layer(
            &mut self.block_sections,
            section_index,
            LightSectionLayer::Packed(bytes.into()),
        );
    }

    pub fn set_sky_empty(&mut self, section_index: usize) {
        set_section_layer(
            &mut self.sky_sections,
            section_index,
            LightSectionLayer::Empty,
        );
        self.rebuild_inherited_sky_cache();
    }

    pub fn set_block_empty(&mut self, section_index: usize) {
        set_section_layer(
            &mut self.block_sections,
            section_index,
            LightSectionLayer::Empty,
        );
    }

    pub fn sky_packed(&self, section_index: usize) -> Option<&[u8; LIGHT_ARRAY_BYTES]> {
        match self.sky_sections.get(section_index)? {
            LightSectionLayer::Packed(bytes) => Some(&bytes.0),
            LightSectionLayer::Uninitialized | LightSectionLayer::Empty => None,
        }
    }

    pub fn block_packed(&self, section_index: usize) -> Option<&[u8; LIGHT_ARRAY_BYTES]> {
        match self.block_sections.get(section_index)? {
            LightSectionLayer::Packed(bytes) => Some(&bytes.0),
            LightSectionLayer::Uninitialized | LightSectionLayer::Empty => None,
        }
    }

    pub fn sky_array_count(&self) -> usize {
        self.sky_sections
            .iter()
            .filter(|layer| matches!(layer, LightSectionLayer::Packed(_)))
            .count()
    }

    pub fn block_array_count(&self) -> usize {
        self.block_sections
            .iter()
            .filter(|layer| matches!(layer, LightSectionLayer::Packed(_)))
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.sky_sections.is_empty() && self.block_sections.is_empty()
    }

    fn rebuild_inherited_sky_cache(&mut self) {
        self.inherited_sky_sections.clear();
        if !self.has_sky_light || self.sky_sections.is_empty() {
            return;
        }
        let mut above_extruded: Option<Arc<PackedLightArray>> = None;
        for index in (0..self.sky_sections.len()).rev() {
            match &self.sky_sections[index] {
                LightSectionLayer::Empty => {
                    above_extruded = None;
                }
                LightSectionLayer::Packed(bytes) => {
                    above_extruded = if bottom_slice_uniform(&bytes.0).is_some() {
                        None
                    } else {
                        Some(Arc::new(extrude_bottom_slice(&bytes.0)))
                    };
                }
                LightSectionLayer::Uninitialized => {
                    if let Some(extruded) = above_extruded.as_ref() {
                        if self.inherited_sky_sections.len() <= index {
                            self.inherited_sky_sections.resize(index + 1, None);
                        }
                        self.inherited_sky_sections[index] = Some(Arc::clone(extruded));
                    }
                }
            }
        }
    }

    #[inline]
    fn inherited_sky_section_ref(&self, light_section_index: usize) -> TerrainSectionLightRef<'_> {
        let Some(above) = self
            .sky_sections
            .get(light_section_index.saturating_add(1)..)
            .and_then(|above| {
                above
                    .iter()
                    .find(|layer| !matches!(layer, LightSectionLayer::Uninitialized))
            })
        else {
            return TerrainSectionLightRef::Uniform(15);
        };
        match above {
            LightSectionLayer::Empty => TerrainSectionLightRef::Uniform(0),
            LightSectionLayer::Packed(bytes) => {
                if let Some(uniform) = bottom_slice_uniform(&bytes.0) {
                    TerrainSectionLightRef::Uniform(uniform)
                } else if let Some(cached) = self
                    .inherited_sky_sections
                    .get(light_section_index)
                    .and_then(Option::as_deref)
                {
                    TerrainSectionLightRef::Packed(&cached.0)
                } else {
                    TerrainSectionLightRef::Uniform(15)
                }
            }
            LightSectionLayer::Uninitialized => TerrainSectionLightRef::Uniform(15),
        }
    }

    #[inline]
    pub(crate) fn section_refs(
        &self,
        light_section_index: usize,
    ) -> (TerrainSectionLightRef<'_>, TerrainSectionLightRef<'_>) {
        let sky = if !self.has_sky_light {
            TerrainSectionLightRef::Uniform(0)
        } else {
            match self.sky_sections.get(light_section_index) {
                None | Some(LightSectionLayer::Uninitialized) => {
                    self.inherited_sky_section_ref(light_section_index)
                }
                Some(LightSectionLayer::Empty) => TerrainSectionLightRef::Uniform(0),
                Some(LightSectionLayer::Packed(bytes)) => TerrainSectionLightRef::Packed(&bytes.0),
            }
        };
        let block = match self.block_sections.get(light_section_index) {
            None | Some(LightSectionLayer::Uninitialized | LightSectionLayer::Empty) => {
                TerrainSectionLightRef::Uniform(0)
            }
            Some(LightSectionLayer::Packed(bytes)) => TerrainSectionLightRef::Packed(&bytes.0),
        };
        (sky, block)
    }
}

/// `section_block_index(x, 0, z)` spans indices `0..256` (bytes `0..128`).
fn bottom_slice_uniform(bytes: &[u8; LIGHT_ARRAY_BYTES]) -> Option<u8> {
    let first = bytes[0];
    let nibble = first & 15;
    if (first >> 4) != nibble {
        return None;
    }
    bytes[..128]
        .iter()
        .all(|&byte| byte == first)
        .then_some(nibble)
}

fn extrude_bottom_slice(bytes: &[u8; LIGHT_ARRAY_BYTES]) -> PackedLightArray {
    let mut extruded = [0u8; LIGHT_ARRAY_BYTES];
    let bottom_slice = &bytes[..128];
    for layer in 0..16 {
        let start = layer * 128;
        extruded[start..start + 128].copy_from_slice(bottom_slice);
    }
    PackedLightArray::from(extruded)
}

fn set_section_layer(
    sections: &mut Vec<LightSectionLayer>,
    section_index: usize,
    layer: LightSectionLayer,
) {
    if matches!(layer, LightSectionLayer::Uninitialized) {
        if let Some(slot) = sections.get_mut(section_index) {
            *slot = LightSectionLayer::Uninitialized;
            while matches!(sections.last(), Some(LightSectionLayer::Uninitialized)) {
                sections.pop();
            }
        }
        return;
    }
    if sections.len() <= section_index {
        sections.resize_with(section_index + 1, LightSectionLayer::default);
    }
    sections[section_index] = layer;
}

fn build_sections_from_protocol(
    mask: &[i64],
    empty_mask: &[i64],
    updates: Vec<Vec<u8>>,
) -> Vec<LightSectionLayer> {
    let mut sections = Vec::new();
    for (section_index, update) in set_bit_indices(mask).zip(updates) {
        if let Ok(array) = <[u8; LIGHT_ARRAY_BYTES]>::try_from(update) {
            set_section_layer(
                &mut sections,
                section_index,
                LightSectionLayer::Packed(PackedLightArray::from(array)),
            );
        }
    }
    for section_index in set_bit_indices(empty_mask) {
        set_section_layer(&mut sections, section_index, LightSectionLayer::Empty);
    }
    sections
}

pub(crate) fn validate_light_update_data(
    light: &LightUpdateData,
    section_count: usize,
) -> Result<()> {
    validate_light_mask("sky_y_mask", &light.sky_y_mask, section_count)?;
    validate_light_mask("block_y_mask", &light.block_y_mask, section_count)?;
    validate_light_mask("empty_sky_y_mask", &light.empty_sky_y_mask, section_count)?;
    validate_light_mask(
        "empty_block_y_mask",
        &light.empty_block_y_mask,
        section_count,
    )?;

    if masks_overlap(&light.sky_y_mask, &light.empty_sky_y_mask) {
        return Err(WorldDecodeError::OverlappingLightMasks { layer: "sky" });
    }
    if masks_overlap(&light.block_y_mask, &light.empty_block_y_mask) {
        return Err(WorldDecodeError::OverlappingLightMasks { layer: "block" });
    }

    validate_light_layers("sky_updates", &light.sky_y_mask, &light.sky_updates)?;
    validate_light_layers("block_updates", &light.block_y_mask, &light.block_updates)?;
    Ok(())
}

fn validate_light_mask(field: &'static str, mask: &[i64], section_count: usize) -> Result<()> {
    if let Some(section) = set_bit_indices(mask).find(|section| *section >= section_count) {
        return Err(WorldDecodeError::LightMaskOutsideDimension {
            field,
            section,
            section_count,
        });
    }
    Ok(())
}

fn masks_overlap(left: &[i64], right: &[i64]) -> bool {
    left.iter()
        .zip(right)
        .any(|(left, right)| (*left as u64 & *right as u64) != 0)
}

fn validate_light_layers(field: &'static str, mask: &[i64], layers: &[Vec<u8>]) -> Result<()> {
    let expected = mask
        .iter()
        .map(|word| (*word as u64).count_ones() as usize)
        .sum();
    if layers.len() != expected {
        return Err(WorldDecodeError::LightDataLayerCountMismatch {
            field,
            expected,
            actual: layers.len(),
        });
    }
    for (index, layer) in layers.iter().enumerate() {
        if layer.len() != LIGHT_ARRAY_BYTES {
            return Err(WorldDecodeError::LightDataLayerSizeMismatch {
                field,
                index,
                expected: LIGHT_ARRAY_BYTES,
                actual: layer.len(),
            });
        }
    }
    Ok(())
}

pub(crate) fn merge_light_data(target: &mut LightData, update: LightData) {
    merge_section_vec(&mut target.sky_sections, update.sky_sections);
    merge_section_vec(&mut target.block_sections, update.block_sections);
    target.rebuild_inherited_sky_cache();
}

fn merge_section_vec(target: &mut Vec<LightSectionLayer>, update: Vec<LightSectionLayer>) {
    for (index, layer) in update.into_iter().enumerate() {
        if !matches!(layer, LightSectionLayer::Uninitialized) {
            set_section_layer(target, index, layer);
        }
    }
}

pub(crate) fn updated_light_section_indices(update: &LightData) -> Vec<usize> {
    let len = update.sky_sections.len().max(update.block_sections.len());
    let mut indices = Vec::new();
    for index in 0..len {
        let sky_changed = update
            .sky_sections
            .get(index)
            .is_some_and(|layer| !matches!(layer, LightSectionLayer::Uninitialized));
        let block_changed = update
            .block_sections
            .get(index)
            .is_some_and(|layer| !matches!(layer, LightSectionLayer::Uninitialized));
        if sky_changed || block_changed {
            indices.push(index);
        }
    }
    indices
}

pub(crate) fn sample_terrain_light(
    light: &LightData,
    dimension: WorldDimension,
    local_x: usize,
    y: i32,
    local_z: usize,
) -> TerrainLight {
    sample_terrain_light_with_sky(light, dimension, light.has_sky_light(), local_x, y, local_z)
}

/// Samples stored block + sky light mirroring `SkyLightSectionStorage.getLightValue`
/// (`SkyLightSectionStorage.java:26-54`) and `BlockLightSectionStorage`.
pub(crate) fn sample_terrain_light_with_sky(
    light: &LightData,
    dimension: WorldDimension,
    has_sky_light: bool,
    local_x: usize,
    y: i32,
    local_z: usize,
) -> TerrainLight {
    let section_y = y.div_euclid(16);
    let light_section_index = section_y - (dimension.min_section_y() - 1);
    let Ok(light_section_index) = usize::try_from(light_section_index) else {
        return TerrainLight::DARK;
    };
    let local_y = y.rem_euclid(16) as usize;
    let nibble_index = section_block_index(local_x as u8, local_y as u8, local_z as u8);
    let sky = if !has_sky_light {
        0
    } else {
        match light.sky_sections.get(light_section_index) {
            Some(LightSectionLayer::Empty) => 0,
            Some(LightSectionLayer::Packed(bytes)) => {
                TerrainSectionLightRef::Packed(&bytes.0).get(nibble_index)
            }
            None | Some(LightSectionLayer::Uninitialized) => match light
                .sky_sections
                .get(light_section_index.saturating_add(1)..)
                .and_then(|above| {
                    above
                        .iter()
                        .find(|layer| !matches!(layer, LightSectionLayer::Uninitialized))
                }) {
                None => 15,
                Some(LightSectionLayer::Empty) => 0,
                Some(LightSectionLayer::Packed(above)) => {
                    let bottom_index = section_block_index(local_x as u8, 0, local_z as u8);
                    TerrainSectionLightRef::Packed(&above.0).get(bottom_index)
                }
                Some(LightSectionLayer::Uninitialized) => 15,
            },
        }
    };
    let block = match light.block_sections.get(light_section_index) {
        None | Some(LightSectionLayer::Uninitialized | LightSectionLayer::Empty) => 0,
        Some(LightSectionLayer::Packed(bytes)) => {
            TerrainSectionLightRef::Packed(&bytes.0).get(nibble_index)
        }
    };
    TerrainLight { sky, block }
}

fn set_bit_indices(words: &[i64]) -> impl Iterator<Item = usize> + '_ {
    words.iter().enumerate().flat_map(|(word_index, word)| {
        let mut bits = *word as u64;
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let bit = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Some(word_index * 64 + bit)
        })
    })
}

pub(super) fn light_layer(light: &LightData, section_index: usize, sky: bool) -> LightDataLayer {
    let (sky_ref, block_ref) = light.section_refs(section_index);
    let layer = if sky { sky_ref } else { block_ref };
    match layer {
        TerrainSectionLightRef::Uniform(value) => LightDataLayer::Uniform(value),
        TerrainSectionLightRef::Packed(bytes) => {
            let first = bytes[0];
            if first & 15 == first >> 4 && bytes.iter().all(|&byte| byte == first) {
                LightDataLayer::Uniform(first & 15)
            } else {
                LightDataLayer::Packed(Box::new(*bytes))
            }
        }
    }
}

pub(super) fn replace_light_layer_bytes(
    light: &mut LightData,
    section_index: usize,
    sky: bool,
    bytes: Box<[u8; LIGHT_ARRAY_BYTES]>,
) -> bool {
    let unchanged = match light_layer(light, section_index, sky) {
        LightDataLayer::Uniform(value) => bytes.iter().all(|&byte| byte == value | (value << 4)),
        LightDataLayer::Packed(previous) => *previous == *bytes,
    };
    if unchanged {
        return false;
    }
    let sections = if sky {
        &mut light.sky_sections
    } else {
        &mut light.block_sections
    };
    if bytes.iter().all(|&byte| byte == 0) {
        set_section_layer(sections, section_index, LightSectionLayer::Empty);
    } else {
        set_section_layer(
            sections,
            section_index,
            LightSectionLayer::Packed(PackedLightArray::from_boxed(bytes)),
        );
    }
    if sky {
        light.rebuild_inherited_sky_cache();
    }
    true
}
