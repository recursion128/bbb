//! Saved-column retirement, after the level owner transfers ticks and entities.

use super::*;
use bbb_server_types::chunk_data::ChunkColumn;

impl ChunkStore {
    pub fn unload_saved_column(&mut self, cx: i32, cz: i32) -> Option<ChunkColumn> {
        let chunk = (cx, cz);
        let column = self.chunks.remove(&chunk)?.column;
        if self.stored_column_reader().is_some() {
            // Storage-backed columns reload on demand from region files rather
            // than retaining duplicate copies in memory.
            self.cold_columns.remove(&chunk);
            self.discard_saved_column_input(chunk);
            self.mark_stored_unread(chunk);
            self.poi.evict_clean_chunk(chunk);
        } else {
            self.retain_cold_column(chunk, &column);
        }
        self.release_column_state(chunk);
        Some(column)
    }

    /// Drops whatever a failed saved-column restore left behind, so the chunk
    /// is generated afresh (`ChunkMap.java:580-603`).
    pub(crate) fn discard_unrestorable_column(&mut self, cx: i32, cz: i32) {
        let chunk = (cx, cz);
        self.chunks.remove(&chunk);
        self.authorize_saved_column_hydration(chunk);
        self.release_column_state(chunk);
        self.poi.unload_chunk(chunk);
        self.ordinary_light_restoring = false;
    }

    fn release_column_state(&mut self, chunk: (i32, i32)) {
        self.block_entities.unload_chunk(chunk);
        self.light_storage.unload_column(chunk);
        self.structure_status.unload_chunk([chunk.0, chunk.1]);
        self.pending_light_changes.remove(&chunk);
        self.live_light_updates.remove(&chunk);
        // An evicted column's POI stays answerable and keeps its changes, the
        // way vanilla's POI storage outlives the chunk.
        self.path_navigation.unload_chunk(chunk);
    }
}
