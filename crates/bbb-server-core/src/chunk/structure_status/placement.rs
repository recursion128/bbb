//! Ordered mutable access to cached referenced starts.

use super::*;
use bbb_server_worldgen::noise::structure_starts::nbt::unpack_chunk;
use bbb_server_worldgen::noise::structure_starts::PlaceableStart;

impl NoiseStructureStatusCache {
    pub(crate) fn referenced_structure_sources(
        &self,
        target: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<Vec<[i32; 2]>> {
        if !self.references_ready.contains(&target) {
            return Err(invalid_status(format!(
                "target STRUCTURE_REFERENCES {target:?} is not published"
            )));
        }
        let data = self.chunks.get(&target).ok_or_else(|| {
            invalid_status(format!("target STRUCTURE_REFERENCES {target:?} is absent"))
        })?;
        Ok(data
            .references()
            .get(structure_id)
            .into_iter()
            .flatten()
            .map(|&packed| unpack_chunk(packed))
            .collect())
    }

    pub(crate) fn place_referenced_starts<S: PlaceableStart>(
        &mut self,
        target: [i32; 2],
        structure_id: &str,
        mut place: impl FnMut(&mut S),
    ) -> SaveResult<()> {
        let sources = self.referenced_structure_sources(target, structure_id)?;
        for source in sources {
            let data = self.chunks.get_mut(&source).ok_or_else(|| {
                invalid_status(format!(
                    "cached STRUCTURE_STARTS source {source:?} is absent"
                ))
            })?;
            if let Some(start) = S::find_mut(data, structure_id) {
                place(start);
            }
        }
        Ok(())
    }
}
