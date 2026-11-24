mod disk;
mod record_slots;
use disk::PoiDisk;
use record_slots::{relative_key, PoiRecordSlots};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::ChunkStore;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_facts::poi_types::{
    vanilla_poi_type_for_resource_id, PoiTypeCatalogEntry, VANILLA_POI_TYPES,
};
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_identity, VANILLA_BLOCK_STATE_COUNT,
};
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::world_storage::Java26PoiChunkSaveEdits;
use bbb_server_persistence::{Java26PoiChunkData, Java26PoiRecordEdit, Java26PoiSectionEdit};
use bbb_server_types::chunk_data::{ChunkColumn, SectionBlocks};
use bbb_server_types::save::Java26LevelKey;

type Position = (i32, i32, i32);
type SectionKey = (i32, i32, i32);
type Column = (i32, i32);

#[inline]
fn section_key(chunk_x: i32, section_y: i32, chunk_z: i32) -> SectionKey {
    (chunk_x, chunk_z, section_y)
}

fn range_columns(center: [i32; 3], radius: i32) -> Option<(i32, i32, i32, i32)> {
    if radius < 0 {
        return None;
    }
    let reach = i64::from(radius).div_euclid(16) + 1;
    let min_x = (i64::from(center[0] >> 4) - reach).max(i64::from(i32::MIN >> 4)) as i32;
    let max_x = (i64::from(center[0] >> 4) + reach).min(i64::from(i32::MAX >> 4)) as i32;
    let min_z = (i64::from(center[2] >> 4) - reach).max(i64::from(i32::MIN >> 4)) as i32;
    let max_z = (i64::from(center[2] >> 4) + reach).min(i64::from(i32::MAX >> 4)) as i32;
    Some((min_x, max_x, min_z, max_z))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoiBlockChange {
    pos: [i32; 3],
    old_state: i32,
    new_state: i32,
}

impl PoiBlockChange {
    pub(crate) fn new(pos: [i32; 3], old_state: i32, new_state: i32) -> Option<Self> {
        (poi_type_for_state(old_state) != poi_type_for_state(new_state)).then_some(Self {
            pos,
            old_state,
            new_state,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoiOccupancy {
    Any,
    HasSpace,
    IsOccupied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoiRecord {
    pub pos: [i32; 3],
    pub type_id: &'static str,
    pub free_tickets: i32,
}

impl PoiRecord {
    fn new(pos: [i32; 3], kind: &'static PoiTypeCatalogEntry) -> Self {
        Self {
            pos,
            type_id: kind.resource_id,
            free_tickets: kind.max_tickets,
        }
    }

    fn matches(&self, occupancy: PoiOccupancy) -> bool {
        // PoiRecord.java:59-65: occupied means unequal, not merely below capacity.
        match occupancy {
            PoiOccupancy::Any => true,
            PoiOccupancy::HasSpace => self.free_tickets > 0,
            PoiOccupancy::IsOccupied => self.free_tickets != self.kind().max_tickets,
        }
    }

    fn in_range(&self, center: [i32; 3], radius_squared: i128, occupancy: PoiOccupancy) -> bool {
        self.matches(occupancy)
            && self
                .pos
                .iter()
                .zip(center)
                .map(|(a, b)| (i128::from(*a) - i128::from(b)).pow(2))
                .sum::<i128>()
                <= radius_squared
    }

    fn kind(&self) -> &'static PoiTypeCatalogEntry {
        vanilla_poi_type_for_resource_id(self.type_id).expect("POI type was validated on load")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PoiSection {
    valid: bool,
    records: BTreeMap<Position, PoiRecord>,
    record_slots: PoiRecordSlots,
}

impl PoiSection {
    fn new(valid: bool) -> Self {
        Self {
            valid,
            records: BTreeMap::new(),
            record_slots: PoiRecordSlots::new(),
        }
    }
    fn insert(&mut self, record: PoiRecord) {
        let [x, y, z] = record.pos;
        self.record_slots.insert(relative_key(record.pos));
        self.records.insert((x, y, z), record);
    }
    fn remove(&mut self, pos: Position) -> bool {
        let Some(record) = self.records.remove(&pos) else {
            return false;
        };
        self.record_slots.remove(relative_key(record.pos));
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoiManager {
    stored: BTreeMap<Column, Java26PoiChunkData>,
    loaded: BTreeSet<Column>,
    /// Keyed as `(chunk_x, chunk_z, section_y)` so column and `(cx, cz)` window queries
    /// scan only the target chunk columns without walking unrelated `chunk_z` ranges.
    sections: BTreeMap<SectionKey, PoiSection>,
    dirty: BTreeMap<Column, u64>,
    next_epoch: u64,
    disk: PoiDisk,
}

impl PoiManager {
    pub(super) fn open_disk(
        &mut self,
        reader: Option<bbb_server_persistence::world_storage::Java26ColumnReader>,
        dimension: Java26LevelKey,
        columns: impl IntoIterator<Item = Column>,
    ) {
        self.disk.open(reader, dimension, columns);
    }

    pub(super) fn disk_refusal(&self) -> Option<String> {
        self.disk.refusal()
    }

    /// The column's record came with its terrain, so no query reads it again.
    pub(super) fn forget_disk(&mut self, column: Column) {
        self.disk.forget(column);
    }

    pub(super) fn unload_chunk(&mut self, chunk: Column) {
        self.stored.remove(&chunk);
        self.loaded.remove(&chunk);
        let keys: Vec<_> = self
            .sections
            .range((chunk.0, chunk.1, i32::MIN)..=(chunk.0, chunk.1, i32::MAX))
            .map(|(&key, _)| key)
            .collect();
        for key in keys {
            self.sections.remove(&key);
        }
        self.dirty.remove(&chunk);
    }

    /// Evicts clean POI sections for a storage-backed unloaded column and
    /// re-registers it for lazy disk reload so future queries/ticket releases
    /// still answer from durable storage without leaking memory.
    pub(super) fn evict_clean_chunk(&mut self, chunk: Column) {
        if !self.disk.has_source() || self.dirty.contains_key(&chunk) {
            return;
        }
        let had_sections = self
            .sections
            .range((chunk.0, chunk.1, i32::MIN)..=(chunk.0, chunk.1, i32::MAX))
            .next()
            .is_some();
        let had_stored = self.stored.contains_key(&chunk);
        self.unload_chunk(chunk);
        if had_sections || had_stored {
            self.disk.mark_unread(chunk);
        }
    }

    pub(crate) fn apply_changes(&mut self, changes: impl IntoIterator<Item = PoiBlockChange>) {
        for change in changes {
            self.block_changed(change.pos, change.old_state, change.new_state);
        }
    }

    pub fn from_saved(chunks: impl IntoIterator<Item = Java26PoiChunkData>) -> Self {
        Self {
            stored: chunks
                .into_iter()
                .map(|chunk| ((chunk.chunk_x(), chunk.chunk_z()), chunk))
                .collect(),
            ..Self::default()
        }
    }

    fn load(&mut self, column: Column) {
        if !self.loaded.insert(column) {
            return;
        }
        if let Some(data) = self.disk.take(column) {
            self.stored.entry(column).or_insert(data);
        }
        if let Some(data) = self.stored.remove(&column) {
            for section in data.sections() {
                let mut live = PoiSection::new(section.valid());
                for record in section.records() {
                    let kind = vanilla_poi_type_for_resource_id(record.type_id())
                        .expect("parsed POI type");
                    live.insert(PoiRecord {
                        pos: record.pos(),
                        type_id: kind.resource_id,
                        free_tickets: record.free_tickets(),
                    });
                }
                self.sections
                    .insert(section_key(column.0, section.y(), column.1), live);
            }
        }
    }

    fn mark_dirty(&mut self, section: SectionKey) {
        self.next_epoch = self.next_epoch.wrapping_add(1);
        self.dirty.insert((section.0, section.1), self.next_epoch);
    }

    fn check_column(
        &mut self,
        column: Column,
        blocks: &ChunkColumn,
        saved_sections: &[(i32, bool)],
    ) {
        self.load(column);
        for &(section_y, has_blocks) in saved_sections {
            let index = section_y - blocks.min_y().div_euclid(16);
            if index < 0 || index as usize >= blocks.section_count() {
                continue;
            }
            let index = index as usize;
            let key = section_key(column.0, section_y, column.1);
            if self.sections.get(&key).is_some_and(|section| section.valid) {
                continue;
            }
            let air = SectionBlocks::Single(blocks.air_state_id());
            let section_blocks = if has_blocks {
                blocks.section(index)
            } else {
                &air
            };
            let may_have_poi = match section_blocks {
                SectionBlocks::Single(state) => poi_type_for_state(*state).is_some(),
                SectionBlocks::Indirect { palette, .. } => palette
                    .iter()
                    .any(|state| poi_type_for_state(*state).is_some()),
            };
            if !may_have_poi && !self.sections.contains_key(&key) {
                continue;
            }
            let mut refreshed = self
                .sections
                .remove(&key)
                .unwrap_or_else(|| PoiSection::new(true));
            let mut old = std::mem::take(&mut refreshed.records);
            refreshed.record_slots.clear_preserving_capacity();
            refreshed.valid = true;
            if may_have_poi {
                for y in 0..16 {
                    for z in 0..16 {
                        for x in 0..16 {
                            if let Some(kind) = poi_type_for_state(section_blocks.get(x, y, z)) {
                                let pos = (
                                    column.0 * 16 + x as i32,
                                    section_y * 16 + y as i32,
                                    column.1 * 16 + z as i32,
                                );
                                // PoiSection.java:126-137: refresh retains the old record by position, including type/tickets.
                                let record = old
                                    .remove(&pos)
                                    .unwrap_or_else(|| PoiRecord::new([pos.0, pos.1, pos.2], kind));
                                refreshed.insert(record);
                            }
                        }
                    }
                }
            }
            self.sections.insert(key, refreshed);
            self.mark_dirty(key);
        }
    }

    fn block_changed(&mut self, pos: [i32; 3], old_state: i32, new_state: i32) {
        let old = poi_type_for_state(old_state);
        let new = poi_type_for_state(new_state);
        if old == new {
            return;
        }
        let [x, y, z] = pos;
        let key = section_key(x >> 4, y >> 4, z >> 4);
        self.load((key.0, key.1));
        if old.is_some() {
            if let Some(section) = self.sections.get_mut(&key) {
                if section.remove((x, y, z)) {
                    self.mark_dirty(key);
                }
            }
        }
        if let Some(kind) = new {
            let section = self
                .sections
                .entry(key)
                .or_insert_with(|| PoiSection::new(true));
            if section
                .records
                .get(&(x, y, z))
                .is_none_or(|record| record.type_id != kind.resource_id)
            {
                section.insert(PoiRecord::new(pos, kind));
                self.mark_dirty(key);
            }
        }
    }

    pub fn record(&mut self, pos: [i32; 3]) -> Option<&PoiRecord> {
        let [x, y, z] = pos;
        self.load((x >> 4, z >> 4));
        self.sections
            .get(&section_key(x >> 4, y >> 4, z >> 4))?
            .records
            .get(&(x, y, z))
    }

    /// `PoiManager.getInChunk` (PoiManager.java:105-111) at `Occupancy.ANY`:
    /// every record the column's sections hold, section Y ascending.
    pub fn records_in_chunk(&mut self, column: Column) -> Vec<PoiRecord> {
        self.load(column);
        self.sections
            .range((column.0, column.1, i32::MIN)..=(column.0, column.1, i32::MAX))
            .flat_map(|(_, section)| section.records.values())
            .cloned()
            .collect()
    }

    fn range_sections(&mut self, center: [i32; 3], radius: i32) -> Vec<SectionKey> {
        let Some((min_x, max_x, min_z, max_z)) = range_columns(center, radius) else {
            return Vec::new();
        };
        // Disk POI queries do not generate terrain; they read the stored
        // columns in range (`PoiManager.java:281-290`, `SectionStorage.getOrLoad`).
        let mut stored = Vec::new();
        for cx in min_x..=max_x {
            stored.extend(
                self.stored
                    .range((cx, min_z)..=(cx, max_z))
                    .map(|(&c, _)| c),
            );
        }
        stored.extend(self.disk.columns_in_range((min_x, max_x), (min_z, max_z)));
        for column in stored {
            self.load(column);
        }
        // ChunkPos.java:202-235; PoiManager.java:87-110: X advances inside Z, then section Y.
        let mut keys = Vec::new();
        for cx in min_x..=max_x {
            keys.extend(
                self.sections
                    .range((cx, min_z, i32::MIN)..=(cx, max_z, i32::MAX))
                    .map(|(&key, _)| key),
            );
        }
        keys.sort_unstable_by_key(|&(x, z, y)| (z, x, y));
        keys
    }

    pub fn find_type_in_range(
        &mut self,
        type_id: &str,
        center: [i32; 3],
        radius: i32,
        occupancy: PoiOccupancy,
    ) -> Option<PoiRecord> {
        let keys = self.range_sections(center, radius);
        let radius_squared = i128::from(radius) * i128::from(radius);
        for key in keys {
            let section = &self.sections[&key];
            for record in section.records.values() {
                if record.type_id == type_id && record.in_range(center, radius_squared, occupancy) {
                    return Some(record.clone());
                }
            }
        }
        None
    }

    pub fn find_closest_with_type(
        &mut self,
        center: [i32; 3],
        radius: i32,
        occupancy: PoiOccupancy,
        predicate: impl Fn(&PoiRecord) -> bool,
    ) -> Option<PoiRecord> {
        let keys = self.range_sections(center, radius);
        let radius_squared = i128::from(radius) * i128::from(radius);
        let mut closest: Option<(&PoiRecord, i128)> = None;
        for key in keys {
            let section = &self.sections[&key];
            for record in section.records.values() {
                if !predicate(record) || !record.matches(occupancy) {
                    continue;
                }
                let distance_squared = record
                    .pos
                    .iter()
                    .zip(center)
                    .map(|(a, b)| (i128::from(*a) - i128::from(b)).pow(2))
                    .sum::<i128>();
                if distance_squared <= radius_squared
                    && closest.is_none_or(|(_, distance)| distance_squared < distance)
                {
                    closest = Some((record, distance_squared));
                }
            }
        }
        closest.map(|(record, _)| record.clone())
    }

    pub(crate) fn visit_records_in_range(
        &self,
        center: [i32; 3],
        radius: i32,
        occupancy: PoiOccupancy,
        mut visitor: impl FnMut(&PoiRecord),
    ) {
        let Some((min_x, max_x, min_z, max_z)) = range_columns(center, radius) else {
            return;
        };
        let radius_squared = i128::from(radius) * i128::from(radius);
        for cx in min_x..=max_x {
            for (_, section) in self
                .sections
                .range((cx, min_z, i32::MIN)..=(cx, max_z, i32::MAX))
            {
                for record in section.records.values() {
                    if record.in_range(center, radius_squared, occupancy) {
                        visitor(record);
                    }
                }
            }
        }
        let mut visit_stored = |data: &Java26PoiChunkData| {
            for stored in data.records() {
                let kind =
                    vanilla_poi_type_for_resource_id(stored.type_id()).expect("parsed POI type");
                let record = PoiRecord {
                    pos: stored.pos(),
                    type_id: kind.resource_id,
                    free_tickets: stored.free_tickets(),
                };
                if record.in_range(center, radius_squared, occupancy) {
                    visitor(&record);
                }
            }
        };
        for cx in min_x..=max_x {
            for (&column, data) in self.stored.range((cx, min_z)..=(cx, max_z)) {
                // A loaded column stays authoritative even when its live records are empty.
                if self.loaded.contains(&column) {
                    continue;
                }
                visit_stored(data);
            }
        }
        self.disk
            .visit_range((min_x, max_x), (min_z, max_z), |column, data| {
                if !self.loaded.contains(&column) && !self.stored.contains_key(&column) {
                    visit_stored(data);
                }
            });
    }

    pub fn records_in_range(
        &mut self,
        center: [i32; 3],
        radius: i32,
        occupancy: PoiOccupancy,
        predicate: impl Fn(&PoiRecord) -> bool,
    ) -> Vec<PoiRecord> {
        let keys = self.range_sections(center, radius);
        let radius_squared = i128::from(radius) * i128::from(radius);
        keys.iter()
            .filter_map(|key| self.sections.get(key))
            .flat_map(|section| section.records.values())
            .filter(|record| {
                record.in_range(center, radius_squared, occupancy) && predicate(record)
            })
            .cloned()
            .collect()
    }

    /// `PoiManager.getInSquare` (`PoiManager.java:87-95`): matches records
    /// whose X and Z distances from `center` are both within `radius`, reading
    /// stored POI sections without generating terrain chunks.
    pub fn records_in_square(
        &mut self,
        center: [i32; 3],
        radius: i32,
        occupancy: PoiOccupancy,
        predicate: impl Fn(&PoiRecord) -> bool,
    ) -> Vec<PoiRecord> {
        let keys = self.range_sections(center, radius);
        let radius = i64::from(radius);
        keys.iter()
            .filter_map(|key| self.sections.get(key))
            .flat_map(|section| section.records.values())
            .filter(|record| {
                record.matches(occupancy)
                    && (i64::from(record.pos[0]) - i64::from(center[0])).abs() <= radius
                    && (i64::from(record.pos[2]) - i64::from(center[2])).abs() <= radius
                    && predicate(record)
            })
            .cloned()
            .collect()
    }

    pub fn acquire_ticket(&mut self, pos: [i32; 3]) -> bool {
        if !self
            .record(pos)
            .is_some_and(|record| record.free_tickets > 0)
        {
            return false;
        }
        let [x, y, z] = pos;
        let key = section_key(x >> 4, y >> 4, z >> 4);
        self.sections
            .get_mut(&key)
            .unwrap()
            .records
            .get_mut(&(x, y, z))
            .unwrap()
            .free_tickets -= 1;
        self.mark_dirty(key);
        true
    }

    pub fn release_ticket(&mut self, pos: [i32; 3]) -> Option<bool> {
        let record = self.record(pos)?;
        let success = record.free_tickets < record.kind().max_tickets;
        let [x, y, z] = pos;
        let key = section_key(x >> 4, y >> 4, z >> 4);
        if success {
            self.sections
                .get_mut(&key)
                .unwrap()
                .records
                .get_mut(&(x, y, z))
                .unwrap()
                .free_tickets += 1;
        }
        // PoiSection.java:99-108: even a failed release dirties an existing record.
        self.mark_dirty(key);
        Some(success)
    }

    fn is_village_source(section: &PoiSection, tags: &RegistryTagSnapshot) -> bool {
        section.records.values().any(|record| {
            record.matches(PoiOccupancy::IsOccupied)
                && tags.contains(
                    "minecraft:point_of_interest_type",
                    "minecraft:village",
                    record.type_id,
                )
        })
    }

    fn collect_village_sources_in_radius(
        &self,
        center: [i32; 3],
        radius: i32,
        tags: &RegistryTagSnapshot,
    ) -> Vec<[i32; 3]> {
        let min_x = center[0].saturating_sub(radius);
        let max_x = center[0].saturating_add(radius);
        let min_y = center[1].saturating_sub(radius);
        let max_y = center[1].saturating_add(radius);
        let min_z = center[2].saturating_sub(radius);
        let max_z = center[2].saturating_add(radius);
        let mut sources = Vec::new();
        for cx in min_x..=max_x {
            for (&(x, z, y), value) in self.sections.range((cx, min_z, min_y)..=(cx, max_z, max_y))
            {
                if (min_y..=max_y).contains(&y) && Self::is_village_source(value, tags) {
                    sources.push([x, y, z]);
                }
            }
        }
        sources
    }

    // PoiManager.java:228-233, PoiManager.java:295-303; SectionTracker.java:12-25: occupied tagged sections are distance sources.
    pub fn sections_to_village(&self, section: [i32; 3], tags: &RegistryTagSnapshot) -> i32 {
        let min_x = section[0].saturating_sub(6);
        let max_x = section[0].saturating_add(6);
        let min_y = section[1].saturating_sub(6);
        let max_y = section[1].saturating_add(6);
        let min_z = section[2].saturating_sub(6);
        let max_z = section[2].saturating_add(6);
        let mut best = 7_i32;
        for cx in min_x..=max_x {
            for (&(x, z, y), value) in self.sections.range((cx, min_z, min_y)..=(cx, max_z, max_y))
            {
                if !(min_y..=max_y).contains(&y) {
                    continue;
                }
                let dist = (i64::from(x) - i64::from(section[0]))
                    .abs()
                    .max((i64::from(y) - i64::from(section[1])).abs())
                    .max((i64::from(z) - i64::from(section[2])).abs())
                    as i32;
                if dist < best && Self::is_village_source(value, tags) {
                    best = dist;
                    if best == 0 {
                        return 0;
                    }
                }
            }
        }
        best
    }

    pub fn save_edits(&self, dimension: &Java26LevelKey) -> Vec<Java26PoiChunkSaveEdits> {
        self.dirty
            .iter()
            .map(|(&(x, z), &epoch)| Java26PoiChunkSaveEdits {
                dimension: dimension.clone(),
                chunk_x: x,
                chunk_z: z,
                epoch,
                sections: self
                    .sections
                    .range((x, z, i32::MIN)..=(x, z, i32::MAX))
                    .map(|(&(_, _, y), section)| Java26PoiSectionEdit {
                        y,
                        valid: section.valid,
                        records: section
                            .record_slots
                            .stream_keys()
                            .map(|key| {
                                let pos = (
                                    x * 16 + i32::from((key >> 8) & 15),
                                    y * 16 + i32::from(key & 15),
                                    z * 16 + i32::from((key >> 4) & 15),
                                );
                                &section.records[&pos]
                            })
                            .map(|record| Java26PoiRecordEdit {
                                pos: record.pos,
                                type_id: record.type_id.to_string(),
                                free_tickets: record.free_tickets,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    pub fn acknowledge_save(&mut self, chunk: Column, epoch: u64) -> bool {
        if self.dirty.get(&chunk) != Some(&epoch) {
            return false;
        }
        self.dirty.remove(&chunk);
        true
    }
}

// PoiTypes.java:115-120: membership comes from the bootstrap state sets.
pub fn poi_type_for_state(state: i32) -> Option<&'static PoiTypeCatalogEntry> {
    static TYPES: OnceLock<Vec<Option<&'static PoiTypeCatalogEntry>>> = OnceLock::new();
    let types = TYPES.get_or_init(|| {
        (0..VANILLA_BLOCK_STATE_COUNT)
            .map(|id| {
                let block = vanilla_block_type(vanilla_block_state_block_type_id(id)?)?;
                VANILLA_POI_TYPES.iter().find(|kind| {
                    kind.matching_blocks.contains(&block.constant_name)
                        && (!kind.head_only
                            || vanilla_block_state_identity(id).is_some_and(|state| {
                                state
                                    .properties
                                    .get("part")
                                    .is_some_and(|part| part == "head")
                            }))
                })
            })
            .collect()
    });
    usize::try_from(state)
        .ok()
        .and_then(|index| types.get(index))
        .copied()
        .flatten()
}

impl ChunkStore {
    // `BehaviorUtils.findSectionClosestToVillage` (`BehaviorUtils.java:107-113`).
    pub(crate) fn closest_village_section(&self, pos: BlockPos) -> Option<BlockPos> {
        let tags = self.registry_tags.as_deref()?;
        let center = [pos.x >> 4, pos.y >> 4, pos.z >> 4];
        // Any village source within distance 6 of any candidate in `center ± 2`
        // lies within `center ± 8`.
        let sources = self.poi.collect_village_sources_in_radius(center, 8, tags);
        if sources.is_empty() {
            return None;
        }
        let distance_to = |section: [i32; 3]| -> i32 {
            sources
                .iter()
                .map(|&src| {
                    (i64::from(src[0]) - i64::from(section[0]))
                        .abs()
                        .max((i64::from(src[1]) - i64::from(section[1])).abs())
                        .max((i64::from(src[2]) - i64::from(section[2])).abs())
                        .min(7) as i32
                })
                .min()
                .unwrap_or(7)
        };
        let mut closest_distance = distance_to(center);
        let mut closest = None;
        for x in center[0] - 2..=center[0] + 2 {
            for y in center[1] - 2..=center[1] + 2 {
                for z in center[2] - 2..=center[2] + 2 {
                    let distance = distance_to([x, y, z]);
                    if distance < closest_distance {
                        closest_distance = distance;
                        closest = Some(BlockPos {
                            x: x * 16 + 8,
                            y: y * 16 + 8,
                            z: z * 16 + 8,
                        });
                    }
                }
            }
        }
        closest
    }

    pub fn sections_to_village(&self, section: [i32; 3]) -> Option<i32> {
        Some(
            self.poi
                .sections_to_village(section, self.registry_tags.as_deref()?),
        )
    }
    pub fn poi(&self) -> &PoiManager {
        &self.poi
    }
    pub fn poi_mut(&mut self) -> &mut PoiManager {
        &mut self.poi
    }
    pub fn restore_poi(&mut self, chunks: impl IntoIterator<Item = Java26PoiChunkData>) {
        for chunk in chunks {
            let column = (chunk.chunk_x(), chunk.chunk_z());
            // POI already live, read by a query first or kept while the
            // column was evicted, is newer than the record its terrain brought.
            if self.poi.loaded.contains(&column) {
                continue;
            }
            self.poi.forget_disk(column);
            self.poi.unload_chunk(column);
            self.poi.stored.insert(column, chunk);
        }
    }
    pub fn update_poi_on_block_change(&mut self, pos: [i32; 3], old_state: i32, new_state: i32) {
        self.poi.block_changed(pos, old_state, new_state);
    }
    pub fn check_poi_consistency(&mut self, column: Column, saved_sections: &[(i32, bool)]) {
        if let Some(blocks) = self.chunks.get(&column).map(|loaded| &loaded.column) {
            self.poi.check_column(column, blocks, saved_sections);
        }
    }
}

#[cfg(test)]
mod tests;
