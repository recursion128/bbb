use super::{Java26SaveRequest, Java26WorldStorage};
use crate::world_storage::dimensions::Java26DimensionGeometries;
use crate::{Java26PoiChunkData, Java26PoiSectionEdit};
use bbb_server_types::save::{Java26LevelKey, NamedCompound, SaveError, SaveResult};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26PoiChunkSaveEdits {
    pub dimension: Java26LevelKey,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub epoch: u64,
    pub sections: Vec<Java26PoiSectionEdit>,
}

type PreparedPoi = Vec<(Java26LevelKey, i32, i32, NamedCompound)>;

impl Java26WorldStorage {
    pub(super) fn prepare_poi_edits(
        &self,
        request: &Java26SaveRequest,
        geometries: &Java26DimensionGeometries,
    ) -> SaveResult<PreparedPoi> {
        let mut seen = HashSet::new();
        let mut prepared = Vec::with_capacity(request.poi_edits.len());
        for edit in &request.poi_edits {
            if !seen.insert((&edit.dimension, edit.chunk_x, edit.chunk_z)) {
                return Err(SaveError::InvalidNbt {
                    path: None,
                    detail: "duplicate POI chunk edit".to_string(),
                });
            }
            let geometry = geometries
                .get(&edit.dimension)
                .ok_or_else(|| super::missing_geometry(&edit.dimension))?
                .sections();
            let region_dir = self.paths.dimension_dir(&edit.dimension).join("poi");
            let path =
                region_dir.join(format!("r.{}.{}.mca", edit.chunk_x >> 5, edit.chunk_z >> 5));
            let store = self.region_store(&path, &region_dir)?;
            // Validate the `.mca` container header up front so a corrupt 8 KiB
            // region header or symlink aborts save preflight identically to
            // `region/` and `entities/` before any in-place region write runs.
            store.validate_existing_header()?;
            let parsed = store
                .read_chunk(edit.chunk_x, edit.chunk_z)
                .and_then(|raw| {
                    raw.map(|raw| {
                        Java26PoiChunkData::parse(
                            raw,
                            edit.chunk_x,
                            edit.chunk_z,
                            geometry.min_section_y,
                            geometry.section_count,
                        )
                    })
                    .transpose()
                });
            // Repeated saves must patch durable state, not the startup
            // candidate. A malformed chunk record was loaded as empty, so the
            // live sections replace it; one of another data version is never
            // written over, and a failed read fails the save, since the edit
            // carries only the column's live sections.
            let current = match parsed {
                Ok(Some(current)) => current,
                Err(SaveError::UnsupportedDataVersion { .. }) => continue,
                Err(error) if !malformed_poi_record(&error) => return Err(error),
                Ok(None) | Err(_) => Java26PoiChunkData::empty(
                    edit.chunk_x,
                    edit.chunk_z,
                    geometry.min_section_y,
                    geometry.section_count,
                )?,
            };
            let raw = current.raw_with_section_edits(
                &edit.sections,
                geometry.min_section_y,
                geometry.section_count,
            )?;
            prepared.push((edit.dimension.clone(), edit.chunk_x, edit.chunk_z, raw));
        }
        Ok(prepared)
    }
}

/// A chunk-slot record whose bytes are present inside a valid `.mca` container
/// but do not decode, as opposed to an `.mca` container/path/OS read failure.
pub(super) fn malformed_poi_record(error: &SaveError) -> bool {
    match error {
        SaveError::InvalidNbt { .. }
        | SaveError::InvalidRegion { .. }
        | SaveError::UnsupportedCompression { .. } => true,
        SaveError::BudgetExceeded { resource, .. } => *resource != "region file",
        SaveError::Io { operation, .. } => *operation == crate::region_file::DECOMPRESSING_CHUNK,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io(operation: &'static str) -> SaveError {
        SaveError::Io {
            operation,
            path: "poi/r.0.0.mca".into(),
            source: std::io::Error::other("EIO"),
        }
    }

    /// Only a record that does not decode is replaced; a read that failed
    /// fails the save, so the sections it did not touch survive on disk.
    #[test]
    fn only_a_record_that_does_not_decode_is_replaced() {
        assert!(malformed_poi_record(&io(
            crate::region_file::DECOMPRESSING_CHUNK
        )));
        assert!(malformed_poi_record(&SaveError::InvalidNbt {
            path: None,
            detail: "truncated".into(),
        }));
        assert!(malformed_poi_record(&SaveError::BudgetExceeded {
            resource: "decompressed region chunk",
            actual: 9,
            limit: 8,
        }));
        assert!(!malformed_poi_record(&SaveError::BudgetExceeded {
            resource: "region file",
            actual: 9,
            limit: 8,
        }));
        assert!(!malformed_poi_record(&io("reading region chunk")));
        assert!(!malformed_poi_record(&SaveError::InvalidPath {
            path: "poi".into(),
            detail: "symlink",
        }));
    }
}
