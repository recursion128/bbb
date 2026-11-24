//! Immutable saved terrain retained while the level owner reloads its typed state.

use super::*;
use bbb_server_types::chunk_data::ChunkColumn;

#[derive(Debug, Default)]
pub(super) struct ColdColumnReads(std::cell::RefCell<BTreeSet<(i32, i32)>>);

impl ChunkStore {
    pub(super) fn note_cold_column_read(&self, chunk: (i32, i32)) {
        self.cold_column_reads.0.borrow_mut().insert(chunk);
    }

    pub fn cold_column(&self, cx: i32, cz: i32) -> Option<&ChunkColumn> {
        let column = self.cold_columns.get(&(cx, cz))?;
        self.note_cold_column_read((cx, cz));
        Some(column)
    }

    pub fn take_cold_column_reads(&mut self) -> BTreeSet<(i32, i32)> {
        std::mem::take(self.cold_column_reads.0.get_mut())
    }

    pub(super) fn authorize_saved_column_hydration(&mut self, chunk: (i32, i32)) {
        self.cold_columns.remove(&chunk);
        self.forget_stored_column(chunk);
        self.discard_saved_column_input(chunk);
        self.cold_column_reads.0.get_mut().remove(&chunk);
    }

    pub(super) fn require_generation_admission(&self, chunk: (i32, i32)) -> SaveResult<()> {
        if self.cold_columns.contains_key(&chunk) || self.is_stored_unread(chunk) {
            return Err(invalid_carvers_region(
                [chunk.0, chunk.1],
                "saved column is unloaded; waiting for saved-column hydration before generation or live write",
            ));
        }
        Ok(())
    }

    pub(super) fn require_structure_window_admission(&self, chunk: (i32, i32)) -> SaveResult<()> {
        for source in structure_status::structure_starts_window(chunk.0, chunk.1)? {
            self.require_generation_admission(source)?;
        }
        Ok(())
    }

    pub(super) fn retain_cold_column(&mut self, chunk: (i32, i32), column: &ChunkColumn) {
        let mut snapshot = column.clone();
        *snapshot.block_ticks_mut() = Default::default();
        *snapshot.fluid_ticks_mut() = Default::default();
        self.cold_columns.insert(chunk, snapshot);
    }
}
