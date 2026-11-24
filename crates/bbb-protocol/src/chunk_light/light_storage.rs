//! Block and sky light layers, one section array per column.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

pub use super::{LightError, LightResult};
use crate::packets::LightUpdateData;

#[derive(Default)]
struct FxHasher(u64);

impl Hasher for FxHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0.rotate_left(5) ^ u64::from(byte)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }

    #[inline]
    fn write_i32(&mut self, value: i32) {
        self.0 =
            (self.0.rotate_left(5) ^ u64::from(value as u32)).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

type FxHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

const SECTION_VOLUME: usize = 16 * 16 * 16;
pub const LIGHT_ARRAY_BYTES: usize = SECTION_VOLUME / 2;

pub type LightSectionPos = (i32, i32, i32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LightDataLayer {
    Uniform(u8),
    Packed(Box<[u8; LIGHT_ARRAY_BYTES]>),
}

impl Default for LightDataLayer {
    fn default() -> Self {
        Self::Uniform(0)
    }
}

impl LightDataLayer {
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> u8 {
        self.get_index(light_index(x, y, z))
    }

    #[inline]
    fn get_index(&self, index: usize) -> u8 {
        match self {
            Self::Uniform(value) => *value,
            Self::Packed(bytes) => (bytes[index / 2] >> ((index & 1) * 4)) & 15,
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, value: u8) {
        self.set_index(light_index(x, y, z), value);
    }

    #[inline]
    fn set_index(&mut self, index: usize, value: u8) {
        debug_assert!(value <= 15);
        let bytes = self.materialize();
        let shift = (index & 1) * 4;
        bytes[index / 2] = (bytes[index / 2] & !(15 << shift)) | ((value & 15) << shift);
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Uniform(0))
    }

    /// Drops a materialized all-zero array so relit and freshly lit sections
    /// project the same empty layer.
    fn normalize_zero(&mut self) {
        if let Self::Packed(bytes) = self {
            if bytes.iter().all(|&byte| byte == 0) {
                *self = Self::Uniform(0);
            }
        }
    }

    fn fill(&mut self, value: u8) {
        debug_assert!(value <= 15);
        *self = Self::Uniform(value);
    }

    fn wire_bytes(&self) -> Box<[u8; LIGHT_ARRAY_BYTES]> {
        match self {
            Self::Uniform(value) => Box::new([value | (value << 4); LIGHT_ARRAY_BYTES]),
            Self::Packed(bytes) => bytes.clone(),
        }
    }

    fn materialize(&mut self) -> &mut [u8; LIGHT_ARRAY_BYTES] {
        if let Self::Uniform(value) = self {
            let packed = *value | (*value << 4);
            *self = Self::Packed(Box::new([packed; LIGHT_ARRAY_BYTES]));
        }
        let Self::Packed(bytes) = self else {
            unreachable!()
        };
        bytes
    }

    fn wire_vec(&self) -> Vec<u8> {
        match self {
            Self::Uniform(value) => vec![*value | (*value << 4); LIGHT_ARRAY_BYTES],
            Self::Packed(bytes) => bytes.to_vec(),
        }
    }

    fn saved_bytes(&self) -> Option<Vec<i8>> {
        match self {
            Self::Uniform(0) => None,
            Self::Uniform(value) => Some(vec![(*value | (*value << 4)) as i8; LIGHT_ARRAY_BYTES]),
            Self::Packed(bytes) => Some(bytes.iter().map(|byte| *byte as i8).collect()),
        }
    }

    fn from_saved(bytes: &[i8]) -> Self {
        if bytes.len() == LIGHT_ARRAY_BYTES {
            let first = bytes[0] as u8;
            // Collapse uniform non-zero nibble layers (e.g. all-15 sky sections)
            // without altering explicit all-zero saved arrays, which keep
            // `is_empty() == false` for packet mask parity.
            if first != 0
                && (first & 0x0f) == (first >> 4)
                && bytes.iter().all(|&b| b as u8 == first)
            {
                return Self::Uniform(first & 0x0f);
            }
        }
        let mut packed = Box::new([0; LIGHT_ARRAY_BYTES]);
        for (to, from) in packed.iter_mut().zip(bytes) {
            *to = *from as u8;
        }
        Self::Packed(packed)
    }

    fn repeat_first_layer(&self) -> Self {
        match self {
            Self::Uniform(value) => Self::Uniform(*value),
            Self::Packed(bytes) => {
                let mut repeated = Box::new([0; LIGHT_ARRAY_BYTES]);
                for layer in 0..16 {
                    let start = layer * 128;
                    repeated[start..start + 128].copy_from_slice(&bytes[..128]);
                }
                Self::Packed(repeated)
            }
        }
    }
}

#[inline]
fn light_index(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < 16 && y < 16 && z < 16);
    (y << 8) | (z << 4) | x
}

#[inline]
fn pos_index(pos: (i32, i32, i32)) -> usize {
    light_index(local(pos.0), local(pos.1), local(pos.2))
}

/// One present section: `LayerLightSectionStorage.SectionState` plus both layers.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LightSection {
    has_data: bool,
    neighbor_count: u8,
    block: LightDataLayer,
    sky: LightDataLayer,
    dirty_block: bool,
    dirty_sky: bool,
}

impl LightSection {
    #[inline]
    fn storing(&self) -> bool {
        self.has_data || self.neighbor_count != 0
    }

    #[inline]
    fn layer(&self, sky: bool) -> &LightDataLayer {
        if sky {
            &self.sky
        } else {
            &self.block
        }
    }
}

#[derive(Clone, Debug)]
struct LightColumn {
    chunk: (i32, i32),
    enabled: bool,
    sources: Option<Box<[i32; 256]>>,
    base_y: i32,
    sections: Vec<Option<LightSection>>,
    dirty_block: BTreeSet<i32>,
    dirty_sky: BTreeSet<i32>,
    dirty_listed: bool,
}

impl LightColumn {
    #[inline]
    fn section(&self, section_y: i32) -> Option<&LightSection> {
        let index = usize::try_from(section_y.checked_sub(self.base_y)?).ok()?;
        self.sections.get(index)?.as_ref()
    }

    #[inline]
    fn section_mut(&mut self, section_y: i32) -> Option<&mut LightSection> {
        let index = usize::try_from(section_y.checked_sub(self.base_y)?).ok()?;
        self.sections.get_mut(index)?.as_mut()
    }

    fn slot(&mut self, section_y: i32) -> &mut Option<LightSection> {
        if self.sections.is_empty() {
            self.base_y = section_y;
        } else if section_y < self.base_y {
            let grow = (self.base_y - section_y) as usize;
            self.sections.splice(0..0, std::iter::repeat_n(None, grow));
            self.base_y = section_y;
        }
        let index = (section_y - self.base_y) as usize;
        if index >= self.sections.len() {
            self.sections.resize(index + 1, None);
        }
        &mut self.sections[index]
    }

    fn present(&self) -> impl DoubleEndedIterator<Item = (i32, &LightSection)> {
        let base = self.base_y;
        self.sections
            .iter()
            .enumerate()
            .filter_map(move |(index, section)| Some((base + index as i32, section.as_ref()?)))
    }

    fn top(&self) -> Option<i32> {
        self.present().next_back().map(|(y, _)| y + 1)
    }

    fn mark_dirty(&mut self, section_y: i32) {
        self.dirty_block.insert(section_y);
        self.dirty_sky.insert(section_y);
    }
}

pub type DirtyLightSections = BTreeMap<(i32, i32), (BTreeSet<i32>, BTreeSet<i32>)>;

#[derive(Clone, Debug, Default)]
pub struct ChunkLightStorage {
    index: FxHashMap<(i32, i32), usize>,
    columns: Vec<LightColumn>,
    dirty_columns: Vec<(i32, i32)>,
    y_counts: BTreeMap<i32, u32>,
    queued_block: BTreeMap<LightSectionPos, LightDataLayer>,
    queued_sky: BTreeMap<LightSectionPos, LightDataLayer>,
}

impl PartialEq for ChunkLightStorage {
    fn eq(&self, other: &Self) -> bool {
        let column = |storage: &Self, chunk: &(i32, i32)| {
            let column = &storage.columns[storage.index[chunk]];
            (
                column.enabled,
                column.sources.clone(),
                column
                    .present()
                    .map(|(y, section)| {
                        (
                            y,
                            section.has_data,
                            section.neighbor_count,
                            section.block.clone(),
                            section.sky.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        };
        self.index.len() == other.index.len()
            && self.index.keys().all(|chunk| {
                other.index.contains_key(chunk) && column(self, chunk) == column(other, chunk)
            })
            && self.queued_block == other.queued_block
            && self.queued_sky == other.queued_sky
    }
}

impl Eq for ChunkLightStorage {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedLightSection {
    pub y: i32,
    pub block_light: Option<Vec<i8>>,
    pub sky_light: Option<Vec<i8>>,
}

impl ChunkLightStorage {
    fn column(&self, chunk: (i32, i32)) -> Option<&LightColumn> {
        self.index.get(&chunk).map(|&slot| &self.columns[slot])
    }

    fn column_mut(&mut self, chunk: (i32, i32)) -> Option<&mut LightColumn> {
        self.index.get(&chunk).map(|&slot| &mut self.columns[slot])
    }

    fn column_or_insert(&mut self, chunk: (i32, i32)) -> &mut LightColumn {
        let slot = *self.index.entry(chunk).or_insert_with(|| {
            self.columns.push(LightColumn {
                chunk,
                enabled: false,
                sources: None,
                base_y: 0,
                sections: Vec::new(),
                dirty_block: BTreeSet::new(),
                dirty_sky: BTreeSet::new(),
                dirty_listed: false,
            });
            self.columns.len() - 1
        });
        &mut self.columns[slot]
    }

    fn record_dirty_column(dirty_columns: &mut Vec<(i32, i32)>, column: &mut LightColumn) {
        if !column.dirty_listed {
            column.dirty_listed = true;
            dirty_columns.push(column.chunk);
        }
    }

    fn section(&self, section: LightSectionPos) -> Option<&LightSection> {
        self.column((section.0, section.2))?.section(section.1)
    }

    /// Creates a present section with `LayerLightSectionStorage.createDataLayer`
    /// layers (`SkyLightSectionStorage.java:93-126`).
    fn ensure_section(&mut self, section: LightSectionPos) -> &mut LightSection {
        let chunk = (section.0, section.2);
        if self
            .column(chunk)
            .and_then(|column| column.section(section.1))
            .is_none()
        {
            let block = self.queued_block.remove(&section).unwrap_or_default();
            let sky = match self.queued_sky.remove(&section) {
                Some(layer) => layer,
                None => self.create_sky_layer(section),
            };
            *self.y_counts.entry(section.1).or_default() += 1;
            let slot = *self.index.entry(chunk).or_insert_with(|| {
                self.columns.push(LightColumn {
                    chunk,
                    enabled: false,
                    sources: None,
                    base_y: 0,
                    sections: Vec::new(),
                    dirty_block: BTreeSet::new(),
                    dirty_sky: BTreeSet::new(),
                    dirty_listed: false,
                });
                self.columns.len() - 1
            });
            let column = &mut self.columns[slot];
            *column.slot(section.1) = Some(LightSection {
                has_data: false,
                neighbor_count: 0,
                block,
                sky,
                dirty_block: true,
                dirty_sky: true,
            });
            column.mark_dirty(section.1);
            Self::record_dirty_column(&mut self.dirty_columns, column);
        }
        self.column_mut(chunk)
            .and_then(|column| column.section_mut(section.1))
            .expect("the section was just ensured")
    }

    fn create_sky_layer(&self, section: LightSectionPos) -> LightDataLayer {
        let column = self.column((section.0, section.2));
        let above = column.and_then(|column| {
            column
                .present()
                .find(|&(y, _)| y > section.1)
                .map(|(_, above)| &above.sky)
        });
        if let Some(above) = above {
            above.repeat_first_layer()
        } else if column.is_some_and(|column| column.enabled) {
            LightDataLayer::Uniform(15)
        } else {
            LightDataLayer::Uniform(0)
        }
    }

    fn remove_section(&mut self, section: LightSectionPos) {
        let Some(&col_slot) = self.index.get(&(section.0, section.2)) else {
            return;
        };
        let column = &mut self.columns[col_slot];
        let Some(slot) = usize::try_from(section.1 - column.base_y)
            .ok()
            .and_then(|index| column.sections.get_mut(index))
        else {
            return;
        };
        if slot.take().is_some() {
            column.mark_dirty(section.1);
            Self::record_dirty_column(&mut self.dirty_columns, column);
            let count = self
                .y_counts
                .get_mut(&section.1)
                .expect("indexed section y");
            *count -= 1;
            if *count == 0 {
                self.y_counts.remove(&section.1);
            }
        }
    }

    fn prune_empty_border_column(&mut self, chunk: (i32, i32)) {
        let Some(&slot) = self.index.get(&chunk) else {
            return;
        };
        let column = &self.columns[slot];
        if column.enabled || column.sources.is_some() || column.present().next().is_some() {
            return;
        }
        self.index.remove(&chunk);
        self.columns.swap_remove(slot);
        if let Some(moved) = self.columns.get(slot) {
            self.index.insert(moved.chunk, slot);
        }
    }

    /// Mirrors `ThreadedLevelLightEngine.updateChunkStatus` (`ThreadedLevelLightEngine.java:69-83`)
    /// and `LayerLightSectionStorage.updateSectionStatus` (`LayerLightSectionStorage.java:212-261`):
    /// disables the column, drops queued data, and unregisters each `has_data` section while keeping
    /// any sections still needed as the 26-neighbor border shell of adjacent loaded columns.
    pub fn unload_column(&mut self, chunk: (i32, i32)) {
        self.queued_block.retain(|&(x, _, z), _| (x, z) != chunk);
        self.queued_sky.retain(|&(x, _, z), _| (x, z) != chunk);
        let Some(&slot) = self.index.get(&chunk) else {
            return;
        };
        let column = &mut self.columns[slot];
        column.enabled = false;
        column.sources = None;
        let has_data_ys: Vec<i32> = column
            .present()
            .filter_map(|(y, section)| section.has_data.then_some(y))
            .collect();
        for y in has_data_ys {
            self.unregister_non_empty_section((chunk.0, y, chunk.1));
        }
        if let Some(column) = self.column(chunk) {
            let orphan_ys: Vec<i32> = column
                .present()
                .filter_map(|(y, section)| (!section.storing()).then_some(y))
                .collect();
            for y in orphan_ys {
                self.remove_section((chunk.0, y, chunk.1));
            }
        }
        for dx in -1..=1 {
            for dz in -1..=1 {
                self.prune_empty_border_column((chunk.0 + dx, chunk.1 + dz));
            }
        }
    }

    /// `LayerLightSectionStorage.updateSectionStatus` and its 26-neighbour shell
    /// (`LayerLightSectionStorage.java:212-261`).
    pub fn register_non_empty_section(&mut self, section: LightSectionPos) {
        let state = self.ensure_section(section);
        if state.has_data {
            return;
        }
        state.has_data = true;
        for (dx, dy, dz) in NEIGHBOR_OFFSETS {
            let state = self.ensure_section((section.0 + dx, section.1 + dy, section.2 + dz));
            state.neighbor_count = state
                .neighbor_count
                .checked_add(1)
                .expect("a light section has at most 26 neighbours");
        }
    }

    /// The emptied half of `LayerLightSectionStorage.updateSectionStatus`: sections
    /// whose state drops to zero lose their layers (`:243-261`).
    pub fn unregister_non_empty_section(&mut self, section: LightSectionPos) {
        let Some(state) = self
            .column_mut((section.0, section.2))
            .and_then(|column| column.section_mut(section.1))
        else {
            return;
        };
        if !state.has_data {
            return;
        }
        state.has_data = false;
        if !state.storing() {
            self.remove_section(section);
        }

        for (dx, dy, dz) in NEIGHBOR_OFFSETS {
            let neighbor = (section.0 + dx, section.1 + dy, section.2 + dz);
            let Some(state) = self
                .column_mut((neighbor.0, neighbor.2))
                .and_then(|column| column.section_mut(neighbor.1))
            else {
                continue;
            };
            state.neighbor_count = state.neighbor_count.saturating_sub(1);
            if !state.storing() {
                self.remove_section(neighbor);
            }
        }
    }

    pub fn enable_column(&mut self, chunk: (i32, i32)) {
        self.column_or_insert(chunk).enabled = true;
    }

    pub fn is_enabled(&self, chunk: (i32, i32)) -> bool {
        self.column(chunk).is_some_and(|column| column.enabled)
    }

    /// Restores a light-only section without claiming that it has block data
    /// (`LevelLightEngine.java:157-166`, `LayerLightSectionStorage.java:212-253`).
    pub fn ensure_empty_section_for_restore(&mut self, section: LightSectionPos) {
        self.ensure_section(section);
    }

    /// `SkyLightEngine.setLightEnabled` fills newly allocated, fully sourced
    /// restore layers after queued saved arrays have been installed
    /// (`SkyLightEngine.java:281-297`).
    pub fn enable_restored_column(
        &mut self,
        chunk: (i32, i32),
        lowest_sky_source_y: &[i32; 16 * 16],
    ) {
        self.enable_column(chunk);
        let highest_non_source_y = lowest_sky_source_y
            .iter()
            .copied()
            .max()
            .expect("a light source witness has 256 columns")
            .saturating_sub(1);
        let lowest_fully_sourced_section_y = highest_non_source_y.div_euclid(16) + 1;
        let (Some(top_section_y), Some(global_bottom_section_y)) =
            (self.top_section_y(chunk), self.bottom_section_y())
        else {
            return;
        };
        let bottom_section_y = global_bottom_section_y.max(lowest_fully_sourced_section_y);
        let column = self.column_mut(chunk).expect("the column was enabled");
        for section_y in (bottom_section_y..top_section_y).rev() {
            if let Some(section) = column.section_mut(section_y) {
                if section.sky.is_empty() {
                    section.sky.fill(15);
                }
            }
        }
    }

    pub fn set_column_sources(&mut self, chunk: (i32, i32), sources: &[i32; 256]) {
        let column = self.column_or_insert(chunk);
        match &mut column.sources {
            Some(current) => **current = *sources,
            None => column.sources = Some(Box::new(*sources)),
        }
    }

    pub fn column_sources(&self, chunk: (i32, i32)) -> Option<&[i32; 256]> {
        self.column(chunk)?.sources.as_deref()
    }

    pub fn column_sources_mut(&mut self, chunk: (i32, i32)) -> Option<&mut [i32; 256]> {
        self.column_mut(chunk)?.sources.as_deref_mut()
    }

    /// `ChunkSkyLightSources.getLowestSourceY` for a column with known sources.
    #[inline]
    pub fn lowest_sky_source(&self, x: i32, z: i32) -> Option<i32> {
        let sources = self
            .column((x.div_euclid(16), z.div_euclid(16)))?
            .sources
            .as_ref()?;
        Some(sources[local(x) + local(z) * 16])
    }

    pub fn storing(&self, section: LightSectionPos) -> bool {
        self.section(section).is_some_and(LightSection::storing)
    }

    pub fn bottom_section_y(&self) -> Option<i32> {
        self.y_counts.keys().next().copied()
    }

    pub fn top_section_y(&self, chunk: (i32, i32)) -> Option<i32> {
        self.column(chunk)?.top()
    }

    pub fn has_light_data_at_or_below(&self, section_y: i32) -> bool {
        self.bottom_section_y()
            .is_some_and(|bottom| section_y >= bottom)
    }

    pub fn block_level(&self, pos: (i32, i32, i32)) -> u8 {
        self.section(section_pos(pos))
            .map_or(0, |section| section.block.get_index(pos_index(pos)))
    }

    pub fn sky_level(&self, pos: (i32, i32, i32)) -> u8 {
        let section = section_pos(pos);
        let Some(column) = self.column((section.0, section.2)) else {
            return 0;
        };
        let Some(top) = column.top() else {
            return if column.enabled { 15 } else { 0 };
        };
        if section.1 >= top {
            return if column.enabled { 15 } else { 0 };
        }
        if let Some(layer) = column.section(section.1) {
            return layer.sky.get_index(pos_index(pos));
        }
        // SkyLightSectionStorage.java:26-50: missing layers inherit the upper bottom plane.
        column
            .present()
            .find(|&(y, _)| y > section.1)
            .map_or(15, |(_, above)| {
                above.sky.get(local(pos.0), 0, local(pos.2))
            })
    }

    pub fn stored_block_level(&self, pos: (i32, i32, i32)) -> u8 {
        self.section(section_pos(pos))
            .expect("stored light read requires a present section")
            .block
            .get_index(pos_index(pos))
    }

    pub fn stored_sky_level(&self, pos: (i32, i32, i32)) -> u8 {
        self.section(section_pos(pos))
            .expect("stored light read requires a present section")
            .sky
            .get_index(pos_index(pos))
    }

    /// The column slot for repeated level access; valid until a column is added
    /// or removed.
    #[inline]
    pub fn slot(&self, chunk: (i32, i32)) -> Option<usize> {
        self.index.get(&chunk).copied()
    }

    /// The stored level when `pos` lies in a storing section of the slot's column.
    #[inline]
    pub fn level_at(&self, slot: usize, pos: (i32, i32, i32), sky: bool) -> Option<u8> {
        let section = self.columns[slot].section(pos.1 >> 4)?;
        section
            .storing()
            .then(|| section.layer(sky).get_index(pos_index(pos)))
    }

    #[inline]
    pub fn enabled_at(&self, slot: usize) -> bool {
        self.columns[slot].enabled
    }

    #[inline]
    pub fn section_storing_at(&self, slot: usize, section_y: i32) -> bool {
        self.columns[slot]
            .section(section_y)
            .is_some_and(LightSection::storing)
    }

    #[inline]
    pub fn set_level_at(&mut self, slot: usize, pos: (i32, i32, i32), sky: bool, value: u8) {
        let column = &mut self.columns[slot];
        let section_y = pos.1 >> 4;
        let section = column
            .section_mut(section_y)
            .expect("light writes target a present section");
        let index = pos_index(pos);
        let (layer, dirty) = if sky {
            (&mut section.sky, &mut section.dirty_sky)
        } else {
            (&mut section.block, &mut section.dirty_block)
        };
        if layer.get_index(index) == value {
            return;
        }
        layer.set_index(index, value);
        if !*dirty {
            *dirty = true;
            if sky {
                column.dirty_sky.insert(section_y);
            } else {
                column.dirty_block.insert(section_y);
            }
            Self::record_dirty_column(&mut self.dirty_columns, column);
        }
    }

    pub fn set_block_level(&mut self, pos: (i32, i32, i32), value: u8) {
        let slot = self.slot((pos.0 >> 4, pos.2 >> 4)).expect("present column");
        self.set_level_at(slot, pos, false, value);
    }

    pub fn set_sky_level(&mut self, pos: (i32, i32, i32), value: u8) {
        let slot = self.slot((pos.0 >> 4, pos.2 >> 4)).expect("present column");
        self.set_level_at(slot, pos, true, value);
    }

    /// Section ys whose sky or block layer changed since the last call, per column.
    pub fn take_dirty(&mut self) -> DirtyLightSections {
        let mut dirty = BTreeMap::new();
        for chunk in self.dirty_columns.drain(..) {
            let Some(&slot) = self.index.get(&chunk) else {
                continue;
            };
            let column = &mut self.columns[slot];
            column.dirty_listed = false;
            if column.dirty_sky.is_empty() && column.dirty_block.is_empty() {
                continue;
            }
            let sky = std::mem::take(&mut column.dirty_sky);
            let block = std::mem::take(&mut column.dirty_block);
            for &y in sky.iter().chain(&block) {
                if let Some(section) = column.section_mut(y) {
                    section.dirty_sky = false;
                    section.dirty_block = false;
                    section.sky.normalize_zero();
                    section.block.normalize_zero();
                }
            }
            dirty.insert(chunk, (sky, block));
        }
        dirty
    }

    pub fn saved_sections(&self, chunk: (i32, i32)) -> Vec<SavedLightSection> {
        let column = self.column(chunk);
        let queued = |map: &BTreeMap<LightSectionPos, LightDataLayer>| {
            map.range((chunk.0, i32::MIN, i32::MIN)..=(chunk.0, i32::MAX, i32::MAX))
                .filter(|(section, _)| section.2 == chunk.1)
                .map(|(section, _)| section.1)
                .collect::<Vec<_>>()
        };
        let mut ys = column
            .map(|column| column.present().map(|(y, _)| y).collect::<BTreeSet<_>>())
            .unwrap_or_default();
        ys.extend(queued(&self.queued_block));
        ys.extend(queued(&self.queued_sky));
        ys.into_iter()
            .filter_map(|y| {
                let present = column.and_then(|column| column.section(y));
                let section = (chunk.0, y, chunk.1);
                let block_light = present
                    .map(|section| &section.block)
                    .or_else(|| self.queued_block.get(&section))
                    .and_then(LightDataLayer::saved_bytes);
                let sky_light = present
                    .map(|section| &section.sky)
                    .or_else(|| self.queued_sky.get(&section))
                    .and_then(LightDataLayer::saved_bytes);
                (block_light.is_some() || sky_light.is_some()).then_some(SavedLightSection {
                    y,
                    block_light,
                    sky_light,
                })
            })
            .collect()
    }

    /// `ClientboundLightUpdatePacketData.prepareSectionData`
    /// (`ClientboundLightUpdatePacketData.java:39-47,68-86`) reads queued data before
    /// stored layers (`LayerLightSectionStorage.java:70-72`); BitSets trim on the wire
    /// (`FriendlyByteBuf.java:648-654`).
    pub fn packet_data(
        &self,
        chunk: (i32, i32),
        min_light_section: i32,
        light_section_count: usize,
    ) -> LightResult<LightUpdateData> {
        let mut data = LightUpdateData {
            sky_y_mask: Vec::new(),
            block_y_mask: Vec::new(),
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: Vec::new(),
            block_updates: Vec::new(),
        };
        let column = self.column(chunk);
        for section_index in 0..light_section_count {
            let offset = i32::try_from(section_index)
                .map_err(|_| invalid_packet_light(chunk, "light section index does not fit i32"))?;
            let section_y = min_light_section.checked_add(offset).ok_or_else(|| {
                invalid_packet_light(chunk, "light section coordinate overflows i32")
            })?;
            let pos = (chunk.0, section_y, chunk.1);
            let present = column.and_then(|column| column.section(section_y));
            append_packet_layer(
                self.queued_sky
                    .get(&pos)
                    .or(present.map(|section| &section.sky)),
                section_index,
                &mut data.sky_y_mask,
                &mut data.empty_sky_y_mask,
                &mut data.sky_updates,
            );
            append_packet_layer(
                self.queued_block
                    .get(&pos)
                    .or(present.map(|section| &section.block)),
                section_index,
                &mut data.block_y_mask,
                &mut data.empty_block_y_mask,
                &mut data.block_updates,
            );
        }
        Ok(data)
    }

    pub fn queue_saved_sections(&mut self, chunk: (i32, i32), sections: &[SavedLightSection]) {
        for section in sections {
            let pos = (chunk.0, section.y, chunk.1);
            for (bytes, sky) in [(&section.block_light, false), (&section.sky_light, true)] {
                if let Some(bytes) = bytes {
                    self.queue_layer_data(pos, sky, LightDataLayer::from_saved(bytes));
                }
            }
        }
    }

    pub fn queue_layer_data(&mut self, section: LightSectionPos, sky: bool, layer: LightDataLayer) {
        if self.storing(section) {
            let column = self
                .column_mut((section.0, section.2))
                .expect("storing column");
            let state = column.section_mut(section.1).expect("storing section");
            *if sky {
                &mut state.sky
            } else {
                &mut state.block
            } = layer;
        } else if sky {
            self.queued_sky.insert(section, layer);
        } else {
            self.queued_block.insert(section, layer);
        }
    }

    /// Materializes implicit layers for hosts that keep dense canonical section arrays.
    pub fn resolved_layer_bytes(
        &self,
        section: LightSectionPos,
        sky: bool,
    ) -> Box<[u8; LIGHT_ARRAY_BYTES]> {
        let queued = if sky {
            &self.queued_sky
        } else {
            &self.queued_block
        };
        if let Some(layer) = queued
            .get(&section)
            .or_else(|| self.section(section).map(|section| section.layer(sky)))
        {
            return layer.wire_bytes();
        }
        if sky {
            self.create_sky_layer(section).wire_bytes()
        } else {
            Box::new([0; LIGHT_ARRAY_BYTES])
        }
    }
}

const NEIGHBOR_OFFSETS: [(i32, i32, i32); 26] = {
    let mut offsets = [(0, 0, 0); 26];
    let mut index = 0;
    let mut dx = -1;
    while dx <= 1 {
        let mut dy = -1;
        while dy <= 1 {
            let mut dz = -1;
            while dz <= 1 {
                if dx != 0 || dy != 0 || dz != 0 {
                    offsets[index] = (dx, dy, dz);
                    index += 1;
                }
                dz += 1;
            }
            dy += 1;
        }
        dx += 1;
    }
    offsets
};

fn append_packet_layer(
    layer: Option<&LightDataLayer>,
    section_index: usize,
    mask: &mut Vec<i64>,
    empty_mask: &mut Vec<i64>,
    updates: &mut Vec<Vec<u8>>,
) {
    let Some(layer) = layer else {
        return;
    };
    if layer.is_empty() {
        set_packet_bit(empty_mask, section_index);
    } else {
        set_packet_bit(mask, section_index);
        updates.push(layer.wire_vec());
    }
}

fn set_packet_bit(mask: &mut Vec<i64>, index: usize) {
    let word = index / 64;
    if mask.len() <= word {
        mask.resize(word + 1, 0);
    }
    mask[word] |= (1_u64 << (index % 64)) as i64;
}

fn invalid_packet_light(chunk: (i32, i32), detail: impl Into<String>) -> LightError {
    LightError::PacketLight {
        chunk_x: chunk.0,
        chunk_z: chunk.1,
        detail: detail.into(),
    }
}

pub fn section_pos(pos: (i32, i32, i32)) -> LightSectionPos {
    (pos.0 >> 4, pos.1 >> 4, pos.2 >> 4)
}

#[inline]
fn local(value: i32) -> usize {
    (value & 15) as usize
}

#[cfg(test)]
mod tests;
