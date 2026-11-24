//! Columns the region headers list and nothing has read yet.

use super::*;
use bbb_server_persistence::world_storage::{Java26ColumnReader, Java26RuntimeColumn};
use bbb_server_types::chunk_data::{SectionBlocks, SECTION_VOLUME};
use bbb_server_types::save::Java26LevelKey;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
use rustc_hash::FxHashSet;
use std::cell::RefCell;

const MAX_STORED_STRUCTURE_CACHE: usize = 4096;
const MAX_STORED_BLOCK_CACHE: usize = 16;

#[derive(Clone, Debug, Default)]
pub(crate) struct StoredColumns {
    unread: FxHashSet<(i32, i32)>,
    reader: Option<Java26ColumnReader>,
    /// Unreadable columns given up for regeneration, for the level owner.
    regenerated: Vec<(i32, i32)>,
    /// Records from another data version; never read again nor replaced.
    refused: FxHashSet<(i32, i32)>,
    refusal: Option<String>,
    structures: StoredStructures,
    blocks: StoredBlocks,
}

type StoredStructureEntry = (
    bbb_server_persistence::Java26ChunkStatus,
    Option<Arc<NoiseStructureChunkData>>,
);
type StoredStructureMap = BTreeMap<(i32, i32), Option<StoredStructureEntry>>;

/// Structure data read out of unread columns for a lookup that only looks.
#[derive(Debug, Default)]
struct StoredStructures(RefCell<StoredStructureMap>);

impl Clone for StoredStructures {
    fn clone(&self) -> Self {
        Self(RefCell::new(self.0.borrow().clone()))
    }
}

type StoredBlockSections = Vec<(i8, SectionBlocks)>;
type StoredBlockCacheEntry = ((i32, i32), StoredBlockSections);

/// Small bounded MRU cache of decoded block sections for unloaded stored columns
/// so repeated border block queries do not repeatedly read and parse `.mca` files.
#[derive(Debug, Default)]
struct StoredBlocks(RefCell<Vec<StoredBlockCacheEntry>>);

impl Clone for StoredBlocks {
    fn clone(&self) -> Self {
        Self(RefCell::new(self.0.borrow().clone()))
    }
}

impl ChunkStore {
    pub(crate) fn open_stored_columns(
        &mut self,
        reader: Option<Java26ColumnReader>,
        dimension: &Java26LevelKey,
        chunks: impl IntoIterator<Item = (i32, i32)>,
        poi_chunks: impl IntoIterator<Item = (i32, i32)>,
    ) {
        self.poi
            .open_disk(reader.clone(), dimension.clone(), poi_chunks);
        let stored = &mut self.saved_column_restoration.stored;
        stored.reader = reader;
        stored.unread.extend(chunks);
    }

    /// A stored column that has to be read before anything uses its position.
    pub fn is_stored_unread(&self, chunk: (i32, i32)) -> bool {
        self.saved_column_restoration.stored.unread.contains(&chunk)
    }

    pub(crate) fn has_stored_unread(&self) -> bool {
        !self.saved_column_restoration.stored.unread.is_empty()
    }

    /// A stored column the next read may still answer.
    pub(crate) fn is_stored_readable(&self, chunk: (i32, i32)) -> bool {
        let stored = &self.saved_column_restoration.stored;
        stored.unread.contains(&chunk) && !stored.refused.contains(&chunk)
    }

    pub(crate) fn stored_column_reader(&self) -> Option<&Java26ColumnReader> {
        self.saved_column_restoration.stored.reader.as_ref()
    }

    /// The structure starts and references a stored column nothing has read
    /// holds, read from its record: `StructureManager.fillStartsForStructure`
    /// brings a start chunk to `STRUCTURE_STARTS` to answer
    /// (`StructureManager.java:71-80`), without loading it for play.
    pub fn stored_structure_component(
        &self,
        cx: i32,
        cz: i32,
    ) -> Option<Arc<NoiseStructureChunkData>> {
        self.stored_structure_component_with_status(cx, cz)
            .and_then(|(_, data)| data)
    }

    pub(crate) fn stored_structure_component_with_status(
        &self,
        cx: i32,
        cz: i32,
    ) -> Option<(
        bbb_server_persistence::Java26ChunkStatus,
        Option<Arc<NoiseStructureChunkData>>,
    )> {
        let chunk = (cx, cz);
        if !self.is_stored_readable(chunk) {
            return None;
        }
        let stored = &self.saved_column_restoration.stored;
        let reader = stored.reader.as_ref()?;
        let resources = self.saved_column_restoration.resources.as_ref()?;
        if let Some(cached) = stored.structures.0.borrow().get(&chunk) {
            return cached.clone();
        }
        let value = reader
            .read_chunk(&resources.dimension, cx, cz)
            .ok()
            .flatten()
            .map(|record| {
                let status = record.data.status();
                let component = self.saved_structure_component(&record.data).map(Arc::new);
                (status, component)
            });
        let mut cache = stored.structures.0.borrow_mut();
        if cache.len() >= MAX_STORED_STRUCTURE_CACHE && !cache.contains_key(&chunk) {
            cache.pop_first();
        }
        cache.insert(chunk, value.clone());
        value
    }

    /// The first record this level refused to read, as it was reported.
    pub(crate) fn storage_refusal(&self) -> Option<String> {
        self.saved_column_restoration
            .stored
            .refusal
            .clone()
            .or_else(|| self.poi.disk_refusal())
    }

    pub(crate) fn take_regenerated_stored_columns(&mut self) -> Vec<(i32, i32)> {
        std::mem::take(&mut self.saved_column_restoration.stored.regenerated)
    }

    pub(crate) fn forget_stored_column(&mut self, chunk: (i32, i32)) {
        let stored = &mut self.saved_column_restoration.stored;
        stored.unread.remove(&chunk);
        stored.structures.0.get_mut().remove(&chunk);
        stored.blocks.0.get_mut().retain(|(key, _)| *key != chunk);
    }

    pub(crate) fn mark_stored_unread(&mut self, chunk: (i32, i32)) {
        let stored = &mut self.saved_column_restoration.stored;
        stored.unread.insert(chunk);
        stored.structures.0.get_mut().remove(&chunk);
        stored.blocks.0.get_mut().retain(|(key, _)| *key != chunk);
    }

    /// Reads one block state on demand from an unloaded stored column in region
    /// storage through a small bounded section cache so repeated queries do not
    /// re-read and re-parse `.mca` records on every block lookup.
    pub(super) fn read_stored_block_state(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let (cx, lx) = split_world_coord(x);
        let (cz, lz) = split_world_coord(z);
        let chunk = (cx, cz);
        if !self.is_stored_readable(chunk) {
            return None;
        }
        let stored = &self.saved_column_restoration.stored;
        let reader = stored.reader.as_ref()?;
        let resources = self.saved_column_restoration.resources.as_ref()?;
        self.note_cold_column_read(chunk);
        let section_y = i8::try_from(y.div_euclid(16)).ok()?;
        let ly = y.rem_euclid(16) as usize;
        {
            let mut cache = stored.blocks.0.borrow_mut();
            if let Some(pos) = cache.iter().position(|(key, _)| *key == chunk) {
                let last = cache.len() - 1;
                if pos != last {
                    let entry = cache.remove(pos);
                    cache.push(entry);
                }
                let (_, sections) = cache.last()?;
                let idx = sections
                    .binary_search_by_key(&section_y, |(y, _)| *y)
                    .ok()?;
                return Some(sections[idx].1.get(lx as usize, ly, lz as usize));
            }
        }
        let record = reader.read_chunk(&resources.dimension, cx, cz).ok()??;
        let mut sections = Vec::with_capacity(record.data.sections().len());
        let content_registry = self.content_registry();
        for section in record.data.sections() {
            let Some(block_states) = section.block_states.as_ref() else {
                continue;
            };
            let Some(palette) = block_states
                .palette
                .iter()
                .map(|saved| {
                    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                        &saved.name,
                        &saved.properties,
                    )
                    .or_else(|| {
                        let spec = content_registry.block_by_resource_id(&saved.name)?;
                        let offset = saved
                            .properties
                            .get(bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY)
                            .map_or(Some(0_u16), |raw| raw.parse::<u16>().ok())?;
                        (offset < spec.state_count)
                            .then(|| spec.first_state_id.saturating_add(i32::from(offset)))
                    })
                })
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let blocks = if palette.len() == 1 {
                SectionBlocks::Single(palette[0])
            } else {
                let mut indices = [0_u16; SECTION_VOLUME];
                let mut valid = true;
                for (cell, slot) in indices.iter_mut().enumerate() {
                    if let Some(entry) = block_states
                        .palette_index(cell)
                        .filter(|&entry| entry < palette.len())
                        .and_then(|entry| u16::try_from(entry).ok())
                    {
                        *slot = entry;
                    } else {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    continue;
                }
                SectionBlocks::from_palette_indices(palette, &indices)
            };
            sections.push((section.y, blocks));
        }
        sections.sort_unstable_by_key(|(y, _)| *y);
        let result = sections
            .binary_search_by_key(&section_y, |(y, _)| *y)
            .ok()
            .map(|idx| sections[idx].1.get(lx as usize, ly, lz as usize));
        let mut cache = stored.blocks.0.borrow_mut();
        if let Some(pos) = cache.iter().position(|(key, _)| *key == chunk) {
            cache.remove(pos);
        } else if cache.len() >= MAX_STORED_BLOCK_CACHE {
            cache.remove(0);
        }
        cache.push((chunk, sections));
        result
    }

    /// `ChunkMap.scheduleChunkLoad` for a position something needs right now:
    /// the stored column is read and published before generation may run
    /// there.
    pub(crate) fn read_stored_column(&mut self, chunk: (i32, i32)) -> SaveResult<()> {
        if !self.is_stored_readable(chunk) {
            return Ok(());
        }
        let (Some(reader), Some(resources)) = (
            self.saved_column_restoration.stored.reader.clone(),
            self.saved_column_restoration.resources.clone(),
        ) else {
            return Ok(());
        };
        let record = reader.read_column(&resources.dimension, chunk.0, chunk.1);
        self.publish_stored_read(chunk, record)
    }

    /// Publishes what a read of a stored column produced.
    fn publish_stored_read(
        &mut self,
        chunk: (i32, i32),
        record: SaveResult<Java26RuntimeColumn>,
    ) -> SaveResult<()> {
        let record = match record {
            Ok(record) if record.chunk.is_some() => record,
            Ok(_) => {
                self.forget_stored_column(chunk);
                return Ok(());
            }
            Err(error) => {
                self.stored_read_failed(chunk, "chunk", &error);
                return Ok(());
            }
        };
        self.forget_stored_column(chunk);
        if let Err(error) = self.restore_runtime_column_record(chunk, record) {
            self.discard_unrestorable_column(chunk.0, chunk.1);
            self.stored_read_failed(chunk, "chunk", &error);
        }
        Ok(())
    }

    /// Settles a stored record that did not read. Vanilla logs a chunk that
    /// fails to load and generates it afresh (`ChunkMap.java:577-603`); a
    /// record of another data version is refused instead, so the column is
    /// neither regenerated nor written over. Answers whether it regenerates.
    pub(crate) fn stored_read_failed(
        &mut self,
        chunk: (i32, i32),
        kind: &'static str,
        error: &SaveError,
    ) -> bool {
        let dimension = self
            .saved_column_restoration
            .resources
            .as_ref()
            .map(|resources| resources.dimension.resource_id().to_string())
            .unwrap_or_default();
        let stored = &mut self.saved_column_restoration.stored;
        if matches!(error, SaveError::UnsupportedDataVersion { .. }) {
            let message = refusal_message(kind, chunk, &dimension, error);
            tracing::error!(event = "world.stored_chunk_refused", %message);
            stored.refused.insert(chunk);
            stored.refusal.get_or_insert(message);
            return false;
        }
        tracing::error!(
            event = "world.stored_chunk_unreadable",
            dimension,
            x = chunk.0,
            z = chunk.1,
            kind,
            %error,
            "the record is skipped and the chunk generated afresh"
        );
        if kind == "chunk" {
            stored.unread.remove(&chunk);
            stored.regenerated.push(chunk);
        }
        true
    }
}

/// The one report of a record this server refuses to read.
pub(super) fn stored_record_refusal(
    kind: &str,
    chunk: (i32, i32),
    dimension: &Java26LevelKey,
    error: &SaveError,
) -> String {
    refusal_message(kind, chunk, dimension.resource_id(), error)
}

fn refusal_message(kind: &str, chunk: (i32, i32), dimension: &str, error: &SaveError) -> String {
    format!(
        "{kind} record of chunk [{}, {}] in {dimension}: {error}",
        chunk.0, chunk.1
    )
}

#[cfg(test)]
impl ChunkStore {
    pub(crate) fn stored_unread_columns_for_test(&self) -> Vec<(i32, i32)> {
        let mut chunks = self
            .saved_column_restoration
            .stored
            .unread
            .iter()
            .copied()
            .collect::<Vec<_>>();
        chunks.sort_unstable();
        chunks
    }
}
