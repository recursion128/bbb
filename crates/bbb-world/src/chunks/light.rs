use bbb_protocol::packets::LightUpdateData;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, VecDeque},
    ops::{Deref, DerefMut},
};

use crate::{
    section_block_index, ChunkPos, Result, TerrainLight, WorldDecodeError, WorldDimension,
};

pub(crate) const LIGHT_ARRAY_BYTES: usize = 2048;

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
pub struct LightData {
    pub sky_y_mask: Vec<i64>,
    pub block_y_mask: Vec<i64>,
    pub empty_sky_y_mask: Vec<i64>,
    pub empty_block_y_mask: Vec<i64>,
    pub sky_updates: Vec<Vec<u8>>,
    pub block_updates: Vec<Vec<u8>>,
}

impl From<LightUpdateData> for LightData {
    fn from(update: LightUpdateData) -> Self {
        Self {
            sky_y_mask: update.sky_y_mask,
            block_y_mask: update.block_y_mask,
            empty_sky_y_mask: update.empty_sky_y_mask,
            empty_block_y_mask: update.empty_block_y_mask,
            sky_updates: update.sky_updates,
            block_updates: update.block_updates,
        }
    }
}

pub(crate) fn validate_light_data(light: &LightData, section_count: usize) -> Result<()> {
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
    if let Some(section) = set_bit_indices(mask)
        .into_iter()
        .find(|section| *section >= section_count)
    {
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
    merge_light_layer(
        &mut target.sky_y_mask,
        &mut target.empty_sky_y_mask,
        &mut target.sky_updates,
        &update.sky_y_mask,
        &update.empty_sky_y_mask,
        &update.sky_updates,
    );
    merge_light_layer(
        &mut target.block_y_mask,
        &mut target.empty_block_y_mask,
        &mut target.block_updates,
        &update.block_y_mask,
        &update.empty_block_y_mask,
        &update.block_updates,
    );
}

pub(crate) fn updated_light_section_indices(update: &LightData) -> Vec<usize> {
    let mut sections = BTreeSet::new();
    for mask in [
        &update.sky_y_mask,
        &update.block_y_mask,
        &update.empty_sky_y_mask,
        &update.empty_block_y_mask,
    ] {
        sections.extend(set_bit_indices(mask));
    }
    sections.into_iter().collect()
}

fn merge_light_layer(
    mask: &mut Vec<i64>,
    empty_mask: &mut Vec<i64>,
    updates: &mut Vec<Vec<u8>>,
    update_mask: &[i64],
    update_empty_mask: &[i64],
    update_arrays: &[Vec<u8>],
) {
    for (section_index, update_array) in set_bit_indices(update_mask).into_iter().zip(update_arrays)
    {
        set_light_layer_data(
            mask,
            empty_mask,
            updates,
            section_index,
            update_array.clone(),
        );
    }
    for section_index in set_bit_indices(update_empty_mask) {
        set_light_layer_empty(mask, empty_mask, updates, section_index);
    }
}

fn set_light_layer_data(
    mask: &mut Vec<i64>,
    empty_mask: &mut Vec<i64>,
    updates: &mut Vec<Vec<u8>>,
    section_index: usize,
    update: Vec<u8>,
) {
    clear_bit(empty_mask, section_index);
    if let Some(rank) = bitset_rank(mask, section_index) {
        if let Some(existing) = updates.get_mut(rank) {
            *existing = update;
        }
        return;
    }

    let insert_index = bitset_rank_before(mask, section_index);
    set_bit(mask, section_index);
    updates.insert(insert_index.min(updates.len()), update);
}

fn set_light_layer_empty(
    mask: &mut Vec<i64>,
    empty_mask: &mut Vec<i64>,
    updates: &mut Vec<Vec<u8>>,
    section_index: usize,
) {
    if let Some(rank) = bitset_rank(mask, section_index) {
        if rank < updates.len() {
            updates.remove(rank);
        }
        clear_bit(mask, section_index);
    }
    set_bit(empty_mask, section_index);
}

pub(crate) fn sample_terrain_light(
    light: &LightData,
    dimension: WorldDimension,
    local_x: usize,
    y: i32,
    local_z: usize,
) -> TerrainLight {
    let section_y = y.div_euclid(16);
    let light_section_index = section_y - (dimension.min_section_y() - 1);
    let Ok(light_section_index) = usize::try_from(light_section_index) else {
        return TerrainLight::FULL_BRIGHT;
    };
    let local_y = y.rem_euclid(16) as usize;
    let nibble_index = section_block_index(local_x as u8, local_y as u8, local_z as u8);
    TerrainLight {
        sky: sample_light_layer(
            &light.sky_y_mask,
            &light.empty_sky_y_mask,
            &light.sky_updates,
            light_section_index,
            nibble_index,
            15,
        ),
        block: sample_light_layer(
            &light.block_y_mask,
            &light.empty_block_y_mask,
            &light.block_updates,
            light_section_index,
            nibble_index,
            0,
        ),
    }
    .clamp()
}

fn sample_light_layer(
    mask: &[i64],
    empty_mask: &[i64],
    updates: &[Vec<u8>],
    section_index: usize,
    nibble_index: usize,
    fallback: u8,
) -> u8 {
    if bitset_get(empty_mask, section_index) {
        return 0;
    }
    if !bitset_get(mask, section_index) {
        return fallback;
    }
    let Some(update_index) = bitset_rank(mask, section_index) else {
        return fallback;
    };
    let Some(layer) = updates.get(update_index) else {
        return fallback;
    };
    read_light_nibble(layer, nibble_index).unwrap_or(fallback)
}

fn bitset_get(words: &[i64], bit: usize) -> bool {
    words
        .get(bit / 64)
        .map(|word| ((*word as u64) & (1u64 << (bit % 64))) != 0)
        .unwrap_or(false)
}

fn set_bit(words: &mut Vec<i64>, bit: usize) {
    let word_index = bit / 64;
    if words.len() <= word_index {
        words.resize(word_index + 1, 0);
    }
    let raw = words[word_index] as u64 | (1u64 << (bit % 64));
    words[word_index] = raw as i64;
}

fn clear_bit(words: &mut [i64], bit: usize) {
    if let Some(word) = words.get_mut(bit / 64) {
        let raw = *word as u64 & !(1u64 << (bit % 64));
        *word = raw as i64;
    }
}

fn bitset_rank(words: &[i64], bit: usize) -> Option<usize> {
    if !bitset_get(words, bit) {
        return None;
    }
    Some(bitset_rank_before(words, bit))
}

fn bitset_rank_before(words: &[i64], bit: usize) -> usize {
    let full_words = bit / 64;
    let mut rank = 0usize;
    for word in &words[..full_words.min(words.len())] {
        rank += (*word as u64).count_ones() as usize;
    }
    let within = bit % 64;
    let mask = if within == 0 { 0 } else { (1u64 << within) - 1 };
    rank += words
        .get(full_words)
        .map(|word| ((*word as u64) & mask).count_ones() as usize)
        .unwrap_or(0);
    rank
}

fn set_bit_indices(words: &[i64]) -> Vec<usize> {
    let mut out = Vec::new();
    for (word_index, word) in words.iter().enumerate() {
        let mut bits = *word as u64;
        while bits != 0 {
            let bit = bits.trailing_zeros() as usize;
            out.push(word_index * 64 + bit);
            bits &= bits - 1;
        }
    }
    out
}

fn read_light_nibble(layer: &[u8], nibble_index: usize) -> Option<u8> {
    let byte = *layer.get(nibble_index / 2)?;
    let shift = (nibble_index % 2) * 4;
    Some((byte >> shift) & 0x0f)
}
