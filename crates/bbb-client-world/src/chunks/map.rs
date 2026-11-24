//! Loaded chunk columns in arrival order with an O(1) position index and
//! lazily derived per-chunk block-entity lookups.

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, OnceLock};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, LightData, PaletteKind, PublicationShared,
};

/// Iteration keeps arrival order: block-entity, translucent and debug
/// consumers walk the slice and their output order must not depend on hashing.
#[derive(Debug, Clone, Default)]
pub(crate) struct ChunkMap {
    columns: Vec<PublicationShared<ChunkColumn>>,
    index: FxHashMap<ChunkPos, usize>,
    membership_revision: u64,
    /// Parallel to `columns`; reset by every mutable column access and
    /// recomputed on the next read.
    derived: Vec<ChunkDerived>,
    family_groups: Vec<OnceLock<FamilyGroup>>,
    resident_families: OnceLock<u32>,
    record_groups: Vec<OnceLock<RecordGroup>>,
}

#[derive(Debug, Clone, Default)]
struct RecordGroup {
    by_type: FxHashMap<i32, Vec<(usize, usize)>>,
}

impl RecordGroup {
    fn from_columns(columns: &[PublicationShared<ChunkColumn>]) -> Self {
        let mut by_type = FxHashMap::<i32, Vec<(usize, usize)>>::default();
        for (offset, column) in columns.iter().enumerate() {
            for (record_index, record) in column.block_entities.iter().enumerate() {
                by_type
                    .entry(record.type_id)
                    .or_default()
                    .push((offset, record_index));
            }
        }
        Self { by_type }
    }
}

// One bit per arrival-order column; a mutable column invalidates only its group.
const FAMILY_GROUP_SIZE: usize = u64::BITS as usize;

#[derive(Debug, Clone, Default)]
struct FamilyGroup {
    mask: u32,
    columns: [u64; BlockFamily::ALL.len()],
}

impl FamilyGroup {
    fn set_column(&mut self, offset: usize, old: u32, new: u32) {
        self.mask |= new;
        for family in set_bits(u64::from(old ^ new)) {
            self.columns[family] ^= 1 << offset;
            if self.columns[family] == 0 {
                self.mask &= !(1 << family);
            }
        }
    }
}

fn set_bits(mut bits: u64) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if bits == 0 {
            return None;
        }
        let index = bits.trailing_zeros() as usize;
        bits &= bits - 1;
        Some(index)
    })
}

#[derive(Debug, Clone, Default)]
struct ChunkDerived {
    families: OnceLock<u32>,
    family_blocks: OnceLock<Arc<[FamilyBlock]>>,
    /// Maps packed `(local_x, y, local_z)` key to its `block_entities` slot index.
    block_entities: OnceLock<Arc<FxHashMap<u32, u32>>>,
    /// Inclusive first..=last section indices containing any blocks or fluids.
    non_empty_sections: OnceLock<Option<(usize, usize)>>,
}

#[derive(Debug, Clone, Copy)]
struct FamilyBlock {
    column_index: u32,
    state_id: i32,
    families: u32,
}

impl ChunkMap {
    pub(crate) fn membership_revision(&self) -> u64 {
        self.membership_revision
    }

    pub(crate) fn get(&self, pos: ChunkPos) -> Option<&ChunkColumn> {
        self.shared_column(pos).map(|column| &**column)
    }

    pub(crate) fn shared_column(&self, pos: ChunkPos) -> Option<&PublicationShared<ChunkColumn>> {
        let index = *self.index.get(&pos)?;
        Some(&self.columns[index])
    }

    pub(crate) fn get_mut(&mut self, pos: ChunkPos) -> Option<&mut ChunkColumn> {
        let index = *self.index.get(&pos)?;
        self.derived[index] = ChunkDerived::default();
        self.family_groups[index / FAMILY_GROUP_SIZE].take();
        self.resident_families.take();
        self.record_groups[index / FAMILY_GROUP_SIZE].take();
        Some(&mut self.columns[index])
    }

    /// Mutable access for block-state updates that do not directly mutate
    /// `block_entities`, preserving the derived block-entity position and type
    /// indices so ordinary block edits stay O(1) even in dense block-entity chunks.
    pub(crate) fn get_mut_for_block_state_change(
        &mut self,
        pos: ChunkPos,
    ) -> Option<&mut ChunkColumn> {
        let index = *self.index.get(&pos)?;
        let derived = &mut self.derived[index];
        derived.families.take();
        derived.family_blocks.take();
        derived.non_empty_sections.take();
        self.family_groups[index / FAMILY_GROUP_SIZE].take();
        self.resident_families.take();
        Some(&mut self.columns[index])
    }

    /// Replaces a single block-entity record in place without wiping derived
    /// chunk indices when its local coordinates and type ID remain unchanged.
    pub(crate) fn replace_block_entity_in_place(
        &mut self,
        pos: ChunkPos,
        record_index: usize,
        record: BlockEntityRecord,
    ) -> bool {
        let Some(&index) = self.index.get(&pos) else {
            return false;
        };
        let column = &mut self.columns[index];
        let Some(existing) = column.block_entities.get(record_index) else {
            return false;
        };
        let same_key = record_key(existing) == record_key(&record);
        let same_type = existing.type_id == record.type_id;
        Arc::make_mut(&mut column.block_entities)[record_index] = record;
        if !same_key {
            self.derived[index].block_entities.take();
        }
        if !same_type {
            self.record_groups[index / FAMILY_GROUP_SIZE].take();
        }
        true
    }

    /// Removes the block-entity record at `pos` in O(1) while maintaining the
    /// per-chunk block-entity index incrementally.
    pub(crate) fn remove_block_entity_at(&mut self, pos: BlockPos) -> bool {
        let Some(record_index) = self.block_entity_index(pos) else {
            return false;
        };
        let Some(&index) = self.index.get(&pos.chunk()) else {
            return false;
        };
        let column = &mut self.columns[index];
        let entities = Arc::make_mut(&mut column.block_entities);
        if record_index >= entities.len() {
            return false;
        }
        let removed = entities.swap_remove(record_index);
        if let Some(map_arc) = self.derived[index].block_entities.get_mut() {
            let map = Arc::make_mut(map_arc);
            map.remove(&record_key(&removed));
            if let Some(moved) = entities.get(record_index) {
                map.insert(record_key(moved), record_index as u32);
            }
        }
        self.record_groups[index / FAMILY_GROUP_SIZE].take();
        true
    }

    /// Reconciles the block-entity record at `pos` after a block-state change
    /// in O(1), preserving the chunk's derived block-entity position map.
    pub(crate) fn reconcile_block_entity_at(
        &mut self,
        pos: BlockPos,
        remove_existing: bool,
        expected_type_id: Option<i32>,
    ) {
        let Ok(y) = i16::try_from(pos.y) else {
            return;
        };
        let chunk_pos = pos.chunk();
        let Some(&index) = self.index.get(&chunk_pos) else {
            return;
        };
        if expected_type_id.is_none() && self.columns[index].block_entities.is_empty() {
            return;
        }
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let key = block_entity_key(local_x, y, local_z);
        let mut existing_index = self.block_entity_index(pos);
        if remove_existing || expected_type_id.is_none() {
            if let Some(record_index) = existing_index {
                let entities = Arc::make_mut(&mut self.columns[index].block_entities);
                if record_index < entities.len() {
                    let removed = entities.swap_remove(record_index);
                    if let Some(map_arc) = self.derived[index].block_entities.get_mut() {
                        let map = Arc::make_mut(map_arc);
                        map.remove(&record_key(&removed));
                        if let Some(moved) = entities.get(record_index) {
                            map.insert(record_key(moved), record_index as u32);
                        }
                    }
                    self.record_groups[index / FAMILY_GROUP_SIZE].take();
                }
                existing_index = None;
            }
        }
        let Some(expected_type_id) = expected_type_id else {
            return;
        };
        if let Some(record_index) = existing_index {
            if self.columns[index].block_entities[record_index].type_id != expected_type_id {
                Arc::make_mut(&mut self.columns[index].block_entities)[record_index] =
                    BlockEntityRecord::empty(local_x, y, local_z, expected_type_id);
                self.record_groups[index / FAMILY_GROUP_SIZE].take();
            }
        } else {
            let entities = Arc::make_mut(&mut self.columns[index].block_entities);
            let new_index = entities.len();
            entities.push(BlockEntityRecord::empty(
                local_x,
                y,
                local_z,
                expected_type_id,
            ));
            if let Some(map_arc) = self.derived[index].block_entities.get_mut() {
                Arc::make_mut(map_arc).insert(key, new_index as u32);
            }
            self.record_groups[index / FAMILY_GROUP_SIZE].take();
        }
    }

    pub(crate) fn light_mut(&mut self, pos: ChunkPos) -> Option<&mut LightData> {
        let index = *self.index.get(&pos)?;
        Some(&mut self.columns[index].light)
    }

    /// Replaces a resident column in place or appends a new one.
    pub(crate) fn push(&mut self, column: PublicationShared<ChunkColumn>) {
        let previous = self.index.get(&column.pos).copied();
        let index = previous.unwrap_or(self.columns.len());
        if index == self.family_groups.len() * FAMILY_GROUP_SIZE {
            self.family_groups.push(OnceLock::new());
            self.record_groups.push(OnceLock::new());
        }
        if self.record_groups[index / FAMILY_GROUP_SIZE]
            .get()
            .is_some()
        {
            // The group index stores record slots. Positions and payloads come from the live column.
            let old = previous.map_or(&[][..], |i| self.columns[i].block_entities.as_slice());
            if !old
                .iter()
                .map(|r| r.type_id)
                .eq(column.block_entities.iter().map(|r| r.type_id))
            {
                self.record_groups[index / FAMILY_GROUP_SIZE].take();
            }
        }
        let mut derived = ChunkDerived::default();
        if let Some(group) = self.family_groups[index / FAMILY_GROUP_SIZE].get_mut() {
            let old = previous.map_or(0, |i| {
                *self.derived[i]
                    .families
                    .get_or_init(|| column_families(&self.columns[i]))
            });
            let new = column_families(&column);
            let old_mask = group.mask;
            group.set_column(index % FAMILY_GROUP_SIZE, old, new);
            if old_mask != group.mask {
                self.resident_families.take();
            }
            derived.families = OnceLock::from(new);
        } else {
            self.resident_families.take();
        }
        match previous {
            Some(index) => {
                self.columns[index] = column;
                self.derived[index] = derived;
            }
            None => {
                self.membership_revision += 1;
                self.index.insert(column.pos, index);
                self.columns.push(column);
                self.derived.push(derived);
            }
        }
    }

    pub(crate) fn remove(&mut self, pos: ChunkPos) -> bool {
        let Some(index) = self.index.remove(&pos) else {
            return false;
        };
        self.membership_revision += 1;
        let last_index = self.columns.len() - 1;
        self.columns.swap_remove(index);
        self.derived.swap_remove(index);
        if index < self.columns.len() {
            let moved_pos = self.columns[index].pos;
            self.index.insert(moved_pos, index);
        }
        self.family_groups[index / FAMILY_GROUP_SIZE].take();
        self.record_groups[index / FAMILY_GROUP_SIZE].take();
        let last_group = last_index / FAMILY_GROUP_SIZE;
        if last_group < self.family_groups.len() {
            self.family_groups[last_group].take();
            self.record_groups[last_group].take();
        }
        let group_count = self.columns.len().div_ceil(FAMILY_GROUP_SIZE);
        self.family_groups.truncate(group_count);
        self.record_groups.truncate(group_count);
        self.resident_families.take();
        true
    }

    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&PublicationShared<ChunkColumn>) -> bool) {
        let kept: Vec<bool> = self.columns.iter().map(&mut keep).collect();
        let Some(first_removed) = kept.iter().position(|kept| !kept) else {
            return;
        };
        let mut flags = kept.iter();
        self.columns.retain(|_| *flags.next().unwrap_or(&true));
        let mut flags = kept.iter();
        self.derived.retain(|_| *flags.next().unwrap_or(&true));
        self.reindex_groups_from(first_removed);
        self.membership_revision += 1;
        self.reindex();
    }

    pub(crate) fn clear(&mut self) {
        if !self.columns.is_empty() {
            self.membership_revision += 1;
        }
        self.family_groups.clear();
        self.record_groups.clear();
        self.resident_families.take();
        self.columns.clear();
        self.index.clear();
        self.derived.clear();
    }

    /// Resident columns that may hold a block of `family`, in arrival order.
    pub(crate) fn with_family(&self, family: BlockFamily) -> impl Iterator<Item = &ChunkColumn> {
        self.indexed_family_groups()
            .flat_map(move |(columns, group)| {
                set_bits(group.columns[family as usize]).map(move |offset| &*columns[offset])
            })
    }

    /// Block-state candidates, including blocks with no block-entity NBT.
    pub(crate) fn family_blocks(
        &self,
        pos: ChunkPos,
        family: BlockFamily,
        min_section_y: i32,
    ) -> impl Iterator<Item = (BlockPos, i32)> + '_ {
        let index = self.index[&pos];
        self.derived[index]
            .family_blocks
            .get_or_init(|| column_family_blocks(&self.columns[index]))
            .iter()
            .filter(move |block| block.families & family.bit() != 0)
            .map(move |block| {
                let index = block.column_index;
                (
                    BlockPos {
                        x: pos
                            .x
                            .saturating_mul(16)
                            .saturating_add((index & 0xF) as i32),
                        y: min_section_y
                            .saturating_mul(16)
                            .saturating_add((index >> 8) as i32),
                        z: pos
                            .z
                            .saturating_mul(16)
                            .saturating_add(((index >> 4) & 0xF) as i32),
                    },
                    block.state_id,
                )
            })
    }

    /// Union of the block families present in any resident column.
    pub(crate) fn families(&self) -> u32 {
        *self.resident_families.get_or_init(|| {
            self.indexed_family_groups()
                .fold(0, |families, (_, group)| families | group.mask)
        })
    }

    pub(crate) fn families_and_records_in_columns(
        &self,
        min: ChunkPos,
        max: ChunkPos,
        mut visit: impl FnMut(ChunkPos, u32),
    ) -> (u32, bool) {
        let mut families = 0;
        let mut has_records = false;
        for z in min.z..=max.z {
            for x in min.x..=max.x {
                if let Some(&i) = self.index.get(&ChunkPos { x, z }) {
                    let column = &self.columns[i];
                    let mask = *self.derived[i]
                        .families
                        .get_or_init(|| column_families(column));
                    visit(column.pos, mask);
                    families |= mask;
                    has_records |= !column.block_entities.is_empty();
                }
            }
        }
        (families, has_records)
    }

    fn indexed_family_groups(
        &self,
    ) -> impl Iterator<Item = (&[PublicationShared<ChunkColumn>], &FamilyGroup)> {
        self.columns
            .chunks(FAMILY_GROUP_SIZE)
            .zip(self.derived.chunks(FAMILY_GROUP_SIZE))
            .zip(&self.family_groups)
            .map(|((columns, derived), cached)| {
                let group = cached.get_or_init(|| {
                    let mut group = FamilyGroup::default();
                    for (offset, (column, derived)) in columns.iter().zip(derived).enumerate() {
                        let families = *derived.families.get_or_init(|| column_families(column));
                        group.set_column(offset, 0, families);
                    }
                    group
                });
                (columns, group)
            })
    }

    fn reindex_groups_from(&mut self, index: usize) {
        let keep_groups = index / FAMILY_GROUP_SIZE;
        let total_groups = self.columns.len().div_ceil(FAMILY_GROUP_SIZE);
        self.family_groups.truncate(keep_groups);
        self.family_groups.resize_with(total_groups, OnceLock::new);
        self.record_groups.truncate(keep_groups);
        self.record_groups.resize_with(total_groups, OnceLock::new);
        self.resident_families.take();
    }

    pub(crate) fn block_entities_of_type(
        &self,
        type_id: i32,
    ) -> impl Iterator<Item = (BlockPos, &BlockEntityRecord)> {
        self.columns
            .chunks(FAMILY_GROUP_SIZE)
            .zip(&self.record_groups)
            .flat_map(move |(columns, cached)| {
                let group = cached.get_or_init(|| RecordGroup::from_columns(columns));
                group.by_type.get(&type_id).into_iter().flatten().map(
                    move |&(offset, record_index)| {
                        let column = &columns[offset];
                        let record = &column.block_entities[record_index];
                        let pos = BlockPos {
                            x: column
                                .pos
                                .x
                                .saturating_mul(16)
                                .saturating_add(i32::from(record.local_x)),
                            y: i32::from(record.y),
                            z: column
                                .pos
                                .z
                                .saturating_mul(16)
                                .saturating_add(i32::from(record.local_z)),
                        };
                        (pos, record)
                    },
                )
            })
    }

    pub(crate) fn block_entity_index(&self, pos: BlockPos) -> Option<usize> {
        let index = *self.index.get(&pos.chunk())?;
        let column = &self.columns[index];
        let key = block_entity_key(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).ok()?,
            pos.z.rem_euclid(16) as u8,
        );
        let map = self.derived[index].block_entities.get_or_init(|| {
            let mut map = FxHashMap::with_capacity_and_hasher(
                column.block_entities.len(),
                Default::default(),
            );
            for (record_index, record) in column.block_entities.iter().enumerate() {
                map.entry(record_key(record)).or_insert(record_index as u32);
            }
            Arc::new(map)
        });
        map.get(&key).map(|&record| record as usize)
    }

    pub(crate) fn block_entity(&self, pos: BlockPos) -> Option<&BlockEntityRecord> {
        let index = self.block_entity_index(pos)?;
        self.get(pos.chunk())?.block_entities.get(index)
    }

    pub(crate) fn non_empty_section_span(&self, pos: ChunkPos) -> Option<Option<(usize, usize)>> {
        let index = *self.index.get(&pos)?;
        Some(*self.derived[index].non_empty_sections.get_or_init(|| {
            let mut span = None::<(usize, usize)>;
            for (section_index, section) in self.columns[index].sections.iter().enumerate() {
                if section.non_empty_block_count == 0 && section.fluid_count == 0 {
                    continue;
                }
                span = Some(
                    span.map_or((section_index, section_index), |(first, last)| {
                        (first.min(section_index), last.max(section_index))
                    }),
                );
            }
            span
        }))
    }

    fn reindex(&mut self) {
        self.index.clear();
        for (index, column) in self.columns.iter().enumerate() {
            self.index.insert(column.pos, index);
        }
    }
}

fn block_entity_key(local_x: u8, y: i16, local_z: u8) -> u32 {
    (u32::from(y as u16) << 8) | (u32::from(local_z & 0xF) << 4) | u32::from(local_x & 0xF)
}

fn record_key(record: &BlockEntityRecord) -> u32 {
    block_entity_key(record.local_x, record.y, record.local_z)
}

impl Deref for ChunkMap {
    type Target = [PublicationShared<ChunkColumn>];

    fn deref(&self) -> &Self::Target {
        &self.columns
    }
}

/// Slice mutation must not change a column's `pos`.
impl DerefMut for ChunkMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reindex_groups_from(0);
        self.derived.fill(ChunkDerived::default());
        &mut self.columns
    }
}

impl<'a> IntoIterator for &'a ChunkMap {
    type Item = &'a PublicationShared<ChunkColumn>;
    type IntoIter = std::slice::Iter<'a, PublicationShared<ChunkColumn>>;

    fn into_iter(self) -> Self::IntoIter {
        self.columns.iter()
    }
}

impl<'a> IntoIterator for &'a mut ChunkMap {
    type Item = &'a mut PublicationShared<ChunkColumn>;
    type IntoIter = std::slice::IterMut<'a, PublicationShared<ChunkColumn>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl Serialize for ChunkMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.columns.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChunkMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let columns: Vec<PublicationShared<ChunkColumn>> = Vec::deserialize(deserializer)?;
        let group_count = columns.len().div_ceil(FAMILY_GROUP_SIZE);
        let mut map = Self {
            derived: vec![ChunkDerived::default(); columns.len()],
            family_groups: vec![OnceLock::new(); group_count],
            resident_families: OnceLock::new(),
            record_groups: vec![OnceLock::new(); group_count],
            columns,
            index: FxHashMap::default(),
            membership_revision: 0,
        };
        map.reindex();
        Ok(map)
    }
}

/// Block groups whose render or ticker owners scan loaded block states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockFamily {
    Banner,
    Beacon,
    Bed,
    Bell,
    Brushable,
    Campfire,
    Chest,
    Conduit,
    CopperGolemStatue,
    DecoratedPot,
    EnchantingTable,
    EndGateway,
    EndPortal,
    Lectern,
    Shelf,
    ShulkerBox,
    Sign,
    Skull,
    Spawner,
    TrialSpawner,
    Vault,
}

impl BlockFamily {
    const ALL: [Self; 21] = [
        Self::Banner,
        Self::Beacon,
        Self::Bed,
        Self::Bell,
        Self::Brushable,
        Self::Campfire,
        Self::Chest,
        Self::Conduit,
        Self::CopperGolemStatue,
        Self::DecoratedPot,
        Self::EnchantingTable,
        Self::EndGateway,
        Self::EndPortal,
        Self::Lectern,
        Self::Shelf,
        Self::ShulkerBox,
        Self::Sign,
        Self::Skull,
        Self::Spawner,
        Self::TrialSpawner,
        Self::Vault,
    ];

    pub(crate) const fn bit(self) -> u32 {
        1 << self as u32
    }

    fn contains_block(self, name: &str) -> bool {
        match self {
            Self::Banner => {
                crate::banner_blocks::banner_color_and_form_for_block_name(name).is_some()
            }
            Self::Beacon => crate::beacon_blocks::is_beacon_block_name(name),
            Self::Bed => crate::bed_blocks::bed_color_for_block_name(name).is_some(),
            Self::Bell => crate::bell_blocks::is_bell_block_name(name),
            Self::Brushable => crate::block_name_is_brushable(name),
            Self::Campfire => matches!(name, "minecraft:campfire" | "minecraft:soul_campfire"),
            Self::Chest => crate::chest_lids::chest_model_kind_for_block_name(name).is_some(),
            Self::Conduit => crate::conduit_blocks::is_conduit_block_name(name),
            Self::CopperGolemStatue => crate::copper_golem_statue_weather_state(name).is_some(),
            Self::DecoratedPot => crate::decorated_pot_blocks::is_decorated_pot_block_name(name),
            Self::EnchantingTable => {
                crate::enchanting_table_books::is_enchanting_table_block_name(name)
            }
            Self::EndGateway => name == "minecraft:end_gateway",
            Self::EndPortal => crate::end_portal_blocks::is_end_portal_block_name(name),
            Self::Lectern => crate::lectern_books::is_lectern_block_name(name),
            Self::Shelf => crate::block_name_is_shelf(name),
            Self::ShulkerBox => {
                crate::shulker_box_blocks::shulker_box_color_for_block_name(name).is_some()
            }
            Self::Sign => crate::sign_blocks::sign_wood_and_form_for_block_name(name).is_some(),
            Self::Skull => crate::skull_blocks::is_skull_block_name(name),
            Self::Spawner => crate::spawner_blocks::is_spawner_block_name(name),
            Self::TrialSpawner => name == "minecraft:trial_spawner",
            Self::Vault => name == "minecraft:vault",
        }
    }
}

/// Family bits of every built-in block state.
fn state_families(state_id: i32) -> u32 {
    static TABLE: OnceLock<Box<[u32]>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        crate::BlockStateRegistry::vanilla_26_1()
            .iter()
            .map(|state| {
                BlockFamily::ALL
                    .iter()
                    .filter(|family| family.contains_block(&state.name))
                    .fold(0, |bits, family| bits | family.bit())
            })
            .collect()
    });
    usize::try_from(state_id)
        .ok()
        .and_then(|index| table.get(index).copied())
        .unwrap_or(0)
}

// Local palettes can retain unused entries, so this mask conservatively includes their families.
fn column_families(column: &ChunkColumn) -> u32 {
    let mut families = 0;
    for section in &column.sections {
        let block_states = &section.block_states;
        match block_states.palette_kind {
            PaletteKind::SingleValue | PaletteKind::Local => {
                for &id in &block_states.palette_global_ids {
                    families |= state_families(id);
                }
            }
            PaletteKind::Global => {
                for index in 0..block_states.entry_count {
                    if let Some(value) = block_states.value_at(index) {
                        families |= state_families(value.global_id);
                    }
                }
            }
        }
    }
    families
}

fn column_family_blocks(column: &ChunkColumn) -> Arc<[FamilyBlock]> {
    let mut blocks = Vec::new();
    for (section_index, section) in column.sections.iter().enumerate() {
        let states = &section.block_states;
        if states.palette_kind != PaletteKind::Global
            && states
                .palette_global_ids
                .iter()
                .all(|&id| state_families(id) == 0)
        {
            continue;
        }
        for index in 0..states.entry_count {
            let Some(value) = states.value_at(index) else {
                continue;
            };
            let families = state_families(value.global_id);
            if families != 0 {
                blocks.push(FamilyBlock {
                    column_index: (section_index * 4096 + index) as u32,
                    state_id: value.global_id,
                    families,
                });
            }
        }
    }
    blocks.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkState;

    fn test_column(
        x: i32,
        block_entities: Vec<BlockEntityRecord>,
    ) -> PublicationShared<ChunkColumn> {
        ChunkColumn {
            pos: ChunkPos { x, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: Arc::new(block_entities),
            light: LightData::default(),
        }
        .into()
    }

    #[test]
    fn get_mut_invalidates_only_the_mutated_record_group_and_keeps_arrival_order() {
        let mut map = ChunkMap::default();
        for x in 0..70 {
            let mut records = Vec::new();
            if x == 1 || x == 65 {
                records.push(BlockEntityRecord::empty(x as u8 & 15, 64, 0, 7));
            }
            map.push(test_column(x, records));
        }

        let initial: Vec<_> = map
            .block_entities_of_type(7)
            .map(|(pos, _)| pos.x)
            .collect();
        assert_eq!(initial, vec![16 + 1, 65 * 16 + (65 & 15)]);
        assert!(map.record_groups[0].get().is_some());
        assert!(map.record_groups[1].get().is_some());

        let column = map.get_mut(ChunkPos { x: 2, z: 0 }).unwrap();
        Arc::make_mut(&mut column.block_entities).push(BlockEntityRecord::empty(3, 70, 0, 7));

        assert!(map.record_groups[0].get().is_none());
        assert!(
            map.record_groups[1].get().is_some(),
            "unrelated chunk group must stay cached"
        );

        let after_edit: Vec<_> = map
            .block_entities_of_type(7)
            .map(|(pos, _)| pos.x)
            .collect();
        assert_eq!(after_edit, vec![16 + 1, 2 * 16 + 3, 65 * 16 + (65 & 15)]);
    }

    #[test]
    fn non_empty_section_span_caches_and_invalidates_per_column() {
        let dummy_states = crate::PalettedContainerData::from_block_state_ids(&[0; 4096]);
        let empty_section = crate::ChunkSection {
            non_empty_block_count: 0,
            fluid_count: 0,
            block_states: dummy_states.clone(),
            biomes: dummy_states.clone(),
        };
        let populated_section = crate::ChunkSection {
            non_empty_block_count: 1,
            fluid_count: 0,
            block_states: dummy_states.clone(),
            biomes: dummy_states,
        };
        let mut map = ChunkMap::default();
        let mut column = (*test_column(0, Vec::new())).clone();
        column.sections = vec![
            Arc::new(empty_section.clone()),
            Arc::new(populated_section),
            Arc::new(empty_section.clone()),
            Arc::new(empty_section),
        ];
        map.push(column.into());
        assert_eq!(
            map.non_empty_section_span(ChunkPos { x: 0, z: 0 }),
            Some(Some((1, 1)))
        );
        let mutated = map.get_mut(ChunkPos { x: 0, z: 0 }).unwrap();
        Arc::make_mut(&mut mutated.sections[3]).fluid_count = 2;
        assert_eq!(
            map.non_empty_section_span(ChunkPos { x: 0, z: 0 }),
            Some(Some((1, 3)))
        );
    }
}
