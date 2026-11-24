//! Live block writes and the light checks they queue.

use rustc_hash::FxHashSet;

use super::lighting::section_has_only_air;
use super::{split_world_coord, ChunkStore};
use bbb_protocol::chunk_light::light_engine::{
    has_different_light_properties, has_different_light_properties_with_registry,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::{LightUpdate, LightUpdateData};
use bbb_server_types::chunk_data::ChunkGenerationState;
use bbb_server_types::chunk_data::ChunkLightState;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::save::SaveResult;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct DirtyLightSections {
    sky: FxHashSet<usize>,
    block: FxHashSet<usize>,
}

impl ChunkStore {
    /// Applies one live block write through [`ChunkStore::set_block_state`].
    /// Noise columns accept it only at FULL, and a durable one is rewritten whole.
    pub fn set_live_block_state(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        state: i32,
    ) -> SaveResult<Option<i32>> {
        let (cx, lx) = split_world_coord(x);
        let (cz, lz) = split_world_coord(z);
        let chunk = (cx, cz);
        self.prepare_generation_admission(chunk)?;
        let Some(previous) = self
            .column(cx, cz)
            .and_then(|column| column.get_block_state(lx, y, lz))
        else {
            return Ok(None);
        };
        if previous == state {
            return Ok(Some(previous));
        }
        let durable = !self.is_ordinary_source() && {
            if !self.is_ticking_full(cx, cz) {
                return Ok(None);
            }
            self.is_storage_backed_noise_full(cx, cz) || self.has_whole_rewrite(chunk)
        };
        if durable
            && (!self.pending_light_changes.contains_key(&chunk) || !self.has_whole_rewrite(chunk))
        {
            self.validate_servable_noise_full_center(chunk)?;
        }
        self.set_block_state(x, y, z, state);
        self.loaded_mut(chunk).mark_save_dirty();
        if durable || self.has_whole_rewrite(chunk) {
            self.mark_whole_rewrite_pending(chunk)?;
        }
        Ok(Some(previous))
    }

    /// Runs the light checks queued since the previous owner barrier.
    pub fn flush_live_relights(&mut self) -> SaveResult<()> {
        if self.is_ordinary_source() {
            return self.flush_ordinary_lights(false);
        }
        if self.pending_light_changes.is_empty() {
            return Ok(());
        }
        let changes = std::mem::take(&mut self.pending_light_changes);
        self.light_storage.take_dirty();
        self.relight_changed_blocks(&changes)?;
        for &chunk in changes.keys() {
            self.refresh_noise_light_metadata(chunk)?;
        }
        let dirty = self.light_storage.take_dirty();
        for chunk in self.record_light_dirt(dirty) {
            if self.is_durable(chunk) && post_light(self.generation(chunk)) {
                self.mark_whole_rewrite_pending(chunk)?;
            }
        }
        Ok(())
    }

    /// Refreshes a relit column's sky sources and occupied sections.
    fn refresh_noise_light_metadata(&mut self, chunk: (i32, i32)) -> SaveResult<()> {
        let (Some(column), Some(&lowest)) = (
            self.column_at(chunk),
            self.light_storage.column_sources(chunk),
        ) else {
            return Ok(());
        };
        let min_section = column.min_y() >> 4;
        let non_empty_sections = (0..column.section_count())
            .filter(|&index| !section_has_only_air(column, index))
            .map(|index| min_section + index as i32)
            .collect::<Vec<_>>();
        if let Some(light) = self.generation_mut(chunk).and_then(post_light_state_mut) {
            light.is_light_correct = true;
            light.initialized.lowest_sky_source_y = lowest;
            light.initialized.non_empty_sections = non_empty_sections;
        }
        Ok(())
    }

    /// Accumulates changed sections of loaded lit columns into the live packet
    /// filters and returns those columns.
    pub(super) fn record_light_dirt(
        &mut self,
        dirty: bbb_protocol::chunk_light::light_storage::DirtyLightSections,
    ) -> Vec<(i32, i32)> {
        let mut changed = Vec::new();
        for (chunk, (sky, block)) in dirty {
            if let Some(loaded) = self.chunks.get_mut(&chunk) {
                loaded.cached_packet.take();
            }
            let Some(column) = self.column_at(chunk) else {
                continue;
            };
            if !self.light_storage.is_enabled(chunk) {
                continue;
            }
            let min_light_section = (column.min_y() >> 4) - 1;
            let count = column.section_count() + 2;
            let index = |y: &i32| {
                usize::try_from(y - min_light_section)
                    .ok()
                    .filter(|&index| index < count)
            };
            let entry = self.live_light_updates.entry(chunk).or_default();
            if self.has_skylight {
                entry.sky.extend(sky.iter().filter_map(index));
            }
            entry.block.extend(block.iter().filter_map(index));
            if entry.sky.is_empty() && entry.block.is_empty() {
                self.live_light_updates.remove(&chunk);
            }
            if (self.has_skylight && !sky.is_empty()) || !block.is_empty() {
                changed.push(chunk);
            }
        }
        changed
    }

    /// Reads the latest layer data through cumulative per-layer section filters
    /// (`ChunkHolder.java:155-187`; `ClientboundLightUpdatePacketData.java:39-47`).
    pub fn take_live_light_updates(&mut self) -> SaveResult<Vec<LightUpdate>> {
        self.flush_live_relights()?;
        self.take_live_light_updates_already_flushed()
    }

    /// Variant of [`Self::take_live_light_updates`] for callers that just
    /// flushed live relights under their own barrier.
    pub(crate) fn take_live_light_updates_already_flushed(
        &mut self,
    ) -> SaveResult<Vec<LightUpdate>> {
        if self.live_light_updates.is_empty() {
            return Ok(Vec::new());
        }
        let mut updates = self
            .live_light_updates
            .iter()
            .map(|(&chunk, dirty)| {
                let light_data = self.filtered_packet_light_data(chunk, dirty)?;
                Ok(LightUpdate {
                    chunk_x: chunk.0,
                    chunk_z: chunk.1,
                    light_data,
                })
            })
            .collect::<SaveResult<Vec<_>>>()?;
        updates.sort_unstable_by_key(|update| (update.chunk_x, update.chunk_z));
        self.live_light_updates.clear();
        Ok(updates)
    }

    fn filtered_packet_light_data(
        &self,
        chunk: (i32, i32),
        dirty: &DirtyLightSections,
    ) -> SaveResult<LightUpdateData> {
        let column =
            self.column_at(chunk)
                .ok_or_else(|| bbb_server_types::save::SaveError::InvalidNbt {
                    path: None,
                    detail: format!(
                        "clientbound chunk [{},{}]: light projection has no loaded column",
                        chunk.0, chunk.1
                    ),
                })?;
        let min_light_section = column
            .min_y()
            .div_euclid(16)
            .checked_sub(1)
            .ok_or_else(|| bbb_server_types::save::SaveError::InvalidNbt {
                path: None,
                detail: format!(
                    "clientbound chunk [{},{}]: minimum light section overflows",
                    chunk.0, chunk.1
                ),
            })?;
        let light_section_count = column.section_count().checked_add(2).ok_or_else(|| {
            bbb_server_types::save::SaveError::InvalidNbt {
                path: None,
                detail: format!(
                    "clientbound chunk [{},{}]: light section count overflows",
                    chunk.0, chunk.1
                ),
            }
        })?;
        let mut dirty_indices = Vec::with_capacity(dirty.sky.len() + dirty.block.len());
        if self.has_skylight {
            dirty_indices.extend(
                dirty
                    .sky
                    .iter()
                    .copied()
                    .filter(|&index| index < light_section_count),
            );
        }
        dirty_indices.extend(
            dirty
                .block
                .iter()
                .copied()
                .filter(|&index| index < light_section_count),
        );
        dirty_indices.sort_unstable();
        dirty_indices.dedup();
        let mut data = LightUpdateData {
            sky_y_mask: Vec::new(),
            block_y_mask: Vec::new(),
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: Vec::new(),
            block_updates: Vec::new(),
        };
        for section_index in dirty_indices {
            let offset = i32::try_from(section_index).map_err(|_| {
                bbb_server_types::save::SaveError::InvalidNbt {
                    path: None,
                    detail: format!(
                        "clientbound chunk [{},{}]: light section index overflows",
                        chunk.0, chunk.1
                    ),
                }
            })?;
            let section_y = min_light_section.checked_add(offset).ok_or_else(|| {
                bbb_server_types::save::SaveError::InvalidNbt {
                    path: None,
                    detail: format!(
                        "clientbound chunk [{},{}]: light section y overflows",
                        chunk.0, chunk.1
                    ),
                }
            })?;
            let single = self.light_storage.packet_data(chunk, section_y, 1)?;
            if self.has_skylight && dirty.sky.contains(&section_index) {
                if packet_bit(&single.sky_y_mask, 0) {
                    set_packet_bit(&mut data.sky_y_mask, section_index);
                    data.sky_updates.extend(single.sky_updates);
                } else if packet_bit(&single.empty_sky_y_mask, 0) {
                    set_packet_bit(&mut data.empty_sky_y_mask, section_index);
                }
            }
            if dirty.block.contains(&section_index) {
                if packet_bit(&single.block_y_mask, 0) {
                    set_packet_bit(&mut data.block_y_mask, section_index);
                    data.block_updates.extend(single.block_updates);
                } else if packet_bit(&single.empty_block_y_mask, 0) {
                    set_packet_bit(&mut data.empty_block_y_mask, section_index);
                }
            }
        }
        Ok(data)
    }
}

/// Marks a changed column for the next light barrier and queues
/// `LevelChunk.setBlockState`'s light check (`LevelChunk.java:289-302`);
/// section emptiness changes are checked too.
pub(super) fn record_light_change(
    pending: &mut crate::chunk::PendingLightChangeMap,
    pos: (i32, i32, i32),
    previous: i32,
    state: i32,
) {
    record_light_change_with_registry(pending, pos, previous, state, None);
}

pub(super) fn record_light_change_with_registry(
    pending: &mut crate::chunk::PendingLightChangeMap,
    pos: (i32, i32, i32),
    previous: i32,
    state: i32,
    registry: Option<&ContentRegistry>,
) {
    let checks = pending.entry((pos.0 >> 4, pos.2 >> 4)).or_default();
    let different = match registry {
        Some(registry) => has_different_light_properties_with_registry(previous, state, registry),
        None => has_different_light_properties(previous, state),
    };
    if different || block_state_is_air(previous) != block_state_is_air(state) {
        checks.insert(pos);
    }
}

fn post_light(state: Option<&ChunkGenerationState>) -> bool {
    matches!(
        state,
        Some(
            ChunkGenerationState::Light(_)
                | ChunkGenerationState::Spawn(_)
                | ChunkGenerationState::NoiseFull(_)
        )
    )
}

pub(super) fn post_light_state_mut(
    state: &mut ChunkGenerationState,
) -> Option<&mut ChunkLightState> {
    match state {
        ChunkGenerationState::Light(light) => Some(light),
        ChunkGenerationState::Spawn(spawn) => Some(spawn.light_mut()),
        ChunkGenerationState::NoiseFull(full) => Some(full.light_mut()),
        _ => None,
    }
}

fn packet_bit(mask: &[i64], index: usize) -> bool {
    mask.get(index / 64)
        .is_some_and(|word| (*word as u64) & (1_u64 << (index % 64)) != 0)
}

fn set_packet_bit(mask: &mut Vec<i64>, index: usize) {
    let word = index / 64;
    if mask.len() <= word {
        mask.resize(word + 1, 0);
    }
    mask[word] |= (1_u64 << (index % 64)) as i64;
}

#[cfg(test)]
fn filter_light_data(data: &mut LightUpdateData, dirty: &DirtyLightSections) {
    filter_layer(
        &mut data.sky_y_mask,
        &mut data.empty_sky_y_mask,
        &mut data.sky_updates,
        &dirty.sky,
    );
    filter_layer(
        &mut data.block_y_mask,
        &mut data.empty_block_y_mask,
        &mut data.block_updates,
        &dirty.block,
    );
}

#[cfg(test)]
fn filter_layer(
    mask: &mut Vec<i64>,
    empty: &mut Vec<i64>,
    arrays: &mut Vec<Vec<u8>>,
    dirty: &FxHashSet<usize>,
) {
    let mut index = 0;
    arrays.retain(|_| {
        while !packet_bit(mask, index) {
            index += 1;
        }
        let keep = dirty.contains(&index);
        index += 1;
        keep
    });
    for bits in [mask, empty] {
        for (word_index, word) in bits.iter_mut().enumerate() {
            for bit in 0..64 {
                if !dirty.contains(&(word_index * 64 + bit)) {
                    *word &= !((1_u64 << bit) as i64);
                }
            }
        }
        while bits.last() == Some(&0) {
            bits.pop();
        }
    }
}

#[cfg(test)]
mod tests;
