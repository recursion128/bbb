//! POI records the `poi/` headers list that no query has read yet. Vanilla
//! reads a POI column the first time any query touches it, whether or not its
//! chunk is loaded (`SectionStorage.getOrLoad`, `SectionStorage.java:124-187`).

use std::cell::{RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet};

use bbb_server_persistence::world_storage::Java26ColumnReader;
use bbb_server_persistence::Java26PoiChunkData;
use bbb_server_types::save::{Java26LevelKey, SaveError};

type Column = (i32, i32);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DiskState {
    unread: BTreeSet<Column>,
    /// Records read for a query that only looked; a write moves them live.
    read: BTreeMap<Column, Java26PoiChunkData>,
    refusal: Option<String>,
}

/// Queries that only look at POI read through a shared reference, so the
/// columns they reach are cached under interior mutability.
#[derive(Debug, Default)]
pub(super) struct PoiDisk {
    source: Option<(Java26ColumnReader, Java26LevelKey)>,
    state: RefCell<DiskState>,
}

impl Clone for PoiDisk {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            state: RefCell::new(self.state.borrow().clone()),
        }
    }
}

impl PartialEq for PoiDisk {
    fn eq(&self, other: &Self) -> bool {
        *self.state.borrow() == *other.state.borrow()
    }
}

impl Eq for PoiDisk {}

impl PoiDisk {
    fn borrow_mut(&self) -> RefMut<'_, DiskState> {
        self.state.borrow_mut()
    }

    pub(super) fn open(
        &mut self,
        reader: Option<Java26ColumnReader>,
        dimension: Java26LevelKey,
        columns: impl IntoIterator<Item = Column>,
    ) {
        let Some(reader) = reader else {
            return;
        };
        self.source = Some((reader, dimension));
        self.state.get_mut().unread.extend(columns);
    }

    /// Reads every unread column in `x` × `z` and hands each record read so
    /// far to `visit`.
    pub(super) fn visit_range(
        &self,
        (min_x, max_x): (i32, i32),
        (min_z, max_z): (i32, i32),
        mut visit: impl FnMut(Column, &Java26PoiChunkData),
    ) {
        let mut state = self.borrow_mut();
        let mut unread = Vec::new();
        for cx in min_x..=max_x {
            unread.extend(state.unread.range((cx, min_z)..=(cx, max_z)).copied());
        }
        for column in unread {
            read_into(&self.source, &mut state, column);
        }
        for cx in min_x..=max_x {
            for (&column, data) in state.read.range((cx, min_z)..=(cx, max_z)) {
                visit(column, data);
            }
        }
    }

    /// Columns in `x` × `z` whose stored record no query has loaded yet.
    pub(super) fn columns_in_range(
        &self,
        (min_x, max_x): (i32, i32),
        (min_z, max_z): (i32, i32),
    ) -> Vec<Column> {
        let state = self.state.borrow();
        let mut columns = Vec::new();
        for cx in min_x..=max_x {
            let range = (cx, min_z)..=(cx, max_z);
            columns.extend(
                state
                    .unread
                    .range(range.clone())
                    .chain(state.read.range(range).map(|(column, _)| column))
                    .copied(),
            );
        }
        columns
    }

    pub(super) fn has_source(&self) -> bool {
        self.source.is_some()
    }

    pub(super) fn mark_unread(&mut self, column: Column) {
        if self.source.is_some() {
            let state = self.state.get_mut();
            state.read.remove(&column);
            state.unread.insert(column);
        }
    }

    /// The column's stored record, read now if no query read it yet.
    pub(super) fn take(&mut self, column: Column) -> Option<Java26PoiChunkData> {
        let Self { source, state } = self;
        let state = state.get_mut();
        read_into(source, state, column);
        state.read.remove(&column)
    }

    /// The column's record arrived with its terrain instead.
    pub(super) fn forget(&mut self, column: Column) {
        let state = self.state.get_mut();
        state.unread.remove(&column);
        state.read.remove(&column);
    }

    pub(super) fn refusal(&self) -> Option<String> {
        self.state.borrow().refusal.clone()
    }
}

fn read_into(
    source: &Option<(Java26ColumnReader, Java26LevelKey)>,
    state: &mut DiskState,
    column: Column,
) {
    if !state.unread.remove(&column) {
        return;
    }
    let Some((reader, dimension)) = source else {
        return;
    };
    match reader.read_poi(dimension, column.0, column.1) {
        Ok(Some(record)) => {
            state.read.insert(column, record.data);
        }
        Ok(None) => {}
        Err(error @ SaveError::UnsupportedDataVersion { .. }) => {
            let message = super::super::stored_columns::stored_record_refusal(
                "poi", column, dimension, &error,
            );
            tracing::error!(event = "world.stored_chunk_refused", %message);
            state.refusal.get_or_insert(message);
        }
        // `SectionStorage.java:189-208`: a record that does not read is
        // logged and the column has no POI.
        Err(error) => tracing::error!(
            event = "world.stored_poi_unreadable",
            dimension = %dimension.resource_id(),
            x = column.0,
            z = column.1,
            %error,
        ),
    }
}
