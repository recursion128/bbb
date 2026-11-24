//! World-level STRUCTURE_STARTS cache and STRUCTURE_REFERENCES publication.

use std::collections::{BTreeMap, BTreeSet};

use crate::chunk::blending::BlendingResolver;
use crate::chunk::noise_generator::{
    NoiseFinalDensityProgram, NoiseGeneratedChunk, NoiseStructureRingPositions,
};
use crate::chunk::parallel_generation::Prefetched;
use bbb_server_types::save::{SaveError, SaveResult};
use bbb_server_worldgen::noise::beardifier::JigsawBeardifier;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

mod placement;
pub mod spawn_overrides;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseStructureTaskStatus {
    Empty,
    StructureStarts,
    StructureReferences,
}

#[derive(Clone, Debug)]
pub struct NoiseStructureStatusCache {
    generate_structures: bool,
    rings: Option<NoiseStructureRingPositions>,
    chunks: BTreeMap<[i32; 2], NoiseStructureChunkData>,
    starts_ready: BTreeSet<[i32; 2]>,
    retrogen_empty_starts: BTreeSet<[i32; 2]>,
    references_ready: BTreeSet<[i32; 2]>,
    /// Column stages a parallel batch computed ahead of the serial pipeline;
    /// see [`crate::chunk::parallel_generation`].
    prefetched: BTreeMap<[i32; 2], Prefetched>,
    /// Structure data of unloaded columns, which `StructureManager` would read
    /// back from storage for a piece that references them.
    unloaded: BTreeMap<[i32; 2], NoiseStructureChunkData>,
}

impl Default for NoiseStructureStatusCache {
    fn default() -> Self {
        Self::new(true)
    }
}

const MAX_UNLOADED_STRUCTURES: usize = 4096;

impl NoiseStructureStatusCache {
    pub(super) fn unload_chunk(&mut self, chunk: [i32; 2]) {
        if let Some(data) = self.chunks.remove(&chunk) {
            if !data.is_empty() {
                if self.unloaded.len() >= MAX_UNLOADED_STRUCTURES
                    && !self.unloaded.contains_key(&chunk)
                {
                    self.unloaded.pop_first();
                }
                self.unloaded.insert(chunk, data);
            } else {
                self.unloaded.remove(&chunk);
            }
        }
        self.starts_ready.remove(&chunk);
        self.retrogen_empty_starts.remove(&chunk);
        self.references_ready.remove(&chunk);
        self.prefetched.remove(&chunk);
    }

    pub fn new(generate_structures: bool) -> Self {
        Self {
            generate_structures,
            rings: None,
            chunks: BTreeMap::new(),
            starts_ready: BTreeSet::new(),
            retrogen_empty_starts: BTreeSet::new(),
            references_ready: BTreeSet::new(),
            prefetched: BTreeMap::new(),
            unloaded: BTreeMap::new(),
        }
    }

    pub const fn generate_structures(&self) -> bool {
        self.generate_structures
    }

    pub fn status(&self, chunk_x: i32, chunk_z: i32) -> NoiseStructureTaskStatus {
        let chunk = [chunk_x, chunk_z];
        if self.references_ready.contains(&chunk) {
            NoiseStructureTaskStatus::StructureReferences
        } else if self.starts_ready.contains(&chunk) || self.chunks.contains_key(&chunk) {
            NoiseStructureTaskStatus::StructureStarts
        } else {
            NoiseStructureTaskStatus::Empty
        }
    }

    #[cfg(test)]
    pub fn starts_ready_count(&self) -> usize {
        self.chunks.len()
    }

    #[cfg(test)]
    pub fn references_ready_count(&self) -> usize {
        self.references_ready.len()
    }

    pub fn component(&self, chunk_x: i32, chunk_z: i32) -> Option<&NoiseStructureChunkData> {
        self.chunks.get(&[chunk_x, chunk_z])
    }

    /// The loaded component, else the one its column held when it unloaded.
    pub fn component_or_unloaded(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Option<&NoiseStructureChunkData> {
        let chunk = [chunk_x, chunk_z];
        self.chunks
            .get(&chunk)
            .or_else(|| self.unloaded.get(&chunk))
    }

    pub(crate) fn try_add_start_reference(
        &mut self,
        chunk: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<bool> {
        Ok(self
            .chunks
            .get_mut(&chunk)
            .is_some_and(|component| component.try_add_reference(structure_id)))
    }

    pub fn restore_chunk_component(
        &mut self,
        component: NoiseStructureChunkData,
    ) -> SaveResult<()> {
        self.restore_chunk_component_at_status(component, true, true)
    }

    pub(crate) fn restore_chunk_component_at_status(
        &mut self,
        component: NoiseStructureChunkData,
        starts_ready: bool,
        references_ready: bool,
    ) -> SaveResult<()> {
        if references_ready && !starts_ready {
            return Err(invalid_status(
                "STRUCTURE_REFERENCES cannot precede STRUCTURE_STARTS",
            ));
        }
        let chunk = component.chunk();
        self.unloaded.remove(&chunk);
        self.chunks.insert(chunk, component);
        if starts_ready {
            self.starts_ready.insert(chunk);
            self.retrogen_empty_starts.remove(&chunk);
        } else {
            self.starts_ready.remove(&chunk);
            self.retrogen_empty_starts.insert(chunk);
        }
        if references_ready {
            self.references_ready.insert(chunk);
        } else {
            self.references_ready.remove(&chunk);
        }
        Ok(())
    }

    pub fn require_only_target_references(
        &self,
        target_chunk: [i32; 2],
        owned_structure: &str,
    ) -> SaveResult<()> {
        if !self.references_ready.contains(&target_chunk) {
            return Err(invalid_status(format!(
                "target STRUCTURE_REFERENCES {:?} is not published",
                target_chunk
            )));
        }
        let target = self.chunks.get(&target_chunk).ok_or_else(|| {
            invalid_status(format!(
                "target STRUCTURE_REFERENCES {:?} is absent",
                target_chunk
            ))
        })?;
        if let Some((structure_id, _)) =
            target
                .references()
                .iter()
                .find(|(structure_id, positions)| {
                    !positions.is_empty() && structure_id.as_str() != owned_structure
                })
        {
            return Err(invalid_status(format!(
                "target {:?} references unowned decoration structure {structure_id}",
                target_chunk
            )));
        }
        Ok(())
    }

    pub fn exclusive_target_reference(&self, target_chunk: [i32; 2]) -> SaveResult<Option<String>> {
        if !self.references_ready.contains(&target_chunk) {
            return Err(invalid_status(format!(
                "target STRUCTURE_REFERENCES {:?} is not published",
                target_chunk
            )));
        }
        let target = self.chunks.get(&target_chunk).ok_or_else(|| {
            invalid_status(format!(
                "target STRUCTURE_REFERENCES {:?} is absent",
                target_chunk
            ))
        })?;
        Ok(target
            .exclusive_nonempty_referenced_structure()
            .map(str::to_string))
    }

    pub fn target_reference_ids(&self, target_chunk: [i32; 2]) -> SaveResult<Vec<String>> {
        if !self.references_ready.contains(&target_chunk) {
            return Err(invalid_status(format!(
                "target STRUCTURE_REFERENCES {target_chunk:?} is not published"
            )));
        }
        let target = self.chunks.get(&target_chunk).ok_or_else(|| {
            invalid_status(format!(
                "target STRUCTURE_REFERENCES {target_chunk:?} is absent"
            ))
        })?;
        Ok(target
            .references()
            .iter()
            .filter(|(_, positions)| !positions.is_empty())
            .map(|(id, _)| id.clone())
            .collect())
    }

    pub fn target_reference_source_chunks(
        &self,
        target_chunk: [i32; 2],
    ) -> SaveResult<BTreeSet<[i32; 2]>> {
        if !self.references_ready.contains(&target_chunk) {
            return Err(invalid_status(format!(
                "target STRUCTURE_REFERENCES {target_chunk:?} is not published"
            )));
        }
        self.chunks
            .get(&target_chunk)
            .map(NoiseStructureChunkData::referenced_source_chunks)
            .ok_or_else(|| {
                invalid_status(format!(
                    "target STRUCTURE_REFERENCES {target_chunk:?} is absent"
                ))
            })
    }

    /// Completes the 289 source tasks before publishing target References and
    /// entering NOISE/CARVERS (`ChunkPyramid.java:9-28`). Source tasks never
    /// recurse into column generation.
    pub fn generate_carvers(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<(NoiseGeneratedChunk, NoiseStructureChunkData)> {
        self.generate_carvers_internal(program, chunk_x, chunk_z, None)
    }

    pub(crate) fn generate_carvers_with_biome_blender(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk_x: i32,
        chunk_z: i32,
        biome_blender: &BlendingResolver,
    ) -> SaveResult<(NoiseGeneratedChunk, NoiseStructureChunkData)> {
        self.generate_carvers_internal(program, chunk_x, chunk_z, Some(biome_blender))
    }

    fn generate_carvers_internal(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk_x: i32,
        chunk_z: i32,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<(NoiseGeneratedChunk, NoiseStructureChunkData)> {
        self.ensure_starts_window(program, chunk_x, chunk_z)?;
        let target = [chunk_x, chunk_z];
        // A batch only prefetched columns that had no old-generation neighbours.
        let prefetched = self
            .prefetched
            .remove(&target)
            .filter(|_| biome_blender.is_none_or(BlendingResolver::is_empty));
        if let Some(Prefetched::Fresh(generated, references)) = prefetched {
            if !self.references_ready.contains(&target) {
                self.chunks.insert(target, references.clone());
                self.references_ready.insert(target);
            }
            return Ok((generated, references));
        }
        let target_data = self.chunks.get(&target).ok_or_else(|| {
            invalid_status(format!(
                "target STRUCTURE_STARTS [{chunk_x},{chunk_z}] disappeared after its dependency window completed"
            ))
        })?;
        if self.references_ready.contains(&target) {
            let generated = program
                .generate_fresh_carved_chunk_from_jigsaw_structure_references_borrowed(
                    target_data,
                    |source_x, source_z| {
                        self.chunks.get(&[source_x, source_z]).ok_or_else(|| {
                            format!(
                                "cached STRUCTURE_STARTS source [{source_x},{source_z}] is absent"
                            )
                        })
                    },
                    biome_blender,
                )?;
            return Ok((generated, target_data.clone()));
        }

        let (generated, references) = program
            .generate_fresh_carved_chunk_from_complete_jigsaw_source_window_borrowed(
                target_data,
                |source_x, source_z| {
                    self.chunks.get(&[source_x, source_z]).ok_or_else(|| {
                        format!("cached STRUCTURE_STARTS source [{source_x},{source_z}] is absent")
                    })
                },
                biome_blender,
            )?;
        self.chunks.insert(target, references.clone());
        self.references_ready.insert(target);
        Ok((generated, references))
    }

    pub(crate) fn ensure_starts_chunk(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<()> {
        let source = [chunk_x, chunk_z];
        if self.starts_ready.contains(&source) {
            return Ok(());
        }
        let starts = self.compute_starts_chunk_ephemeral(program, chunk_x, chunk_z)?;
        self.publish_generated_starts(starts);
        Ok(())
    }

    /// Computes structure starts for one chunk without inserting the result into
    /// `self.chunks` or `self.starts_ready`. Used by `/locate` and explorer-map
    /// searches so non-matching probe columns never pollute chunk storage.
    pub(crate) fn compute_starts_chunk_ephemeral(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<NoiseStructureChunkData> {
        let source = [chunk_x, chunk_z];
        if self.generate_structures && self.rings.is_none() {
            self.rings = Some(program.structure_start_ring_positions()?);
        }
        let empty_rings = BTreeMap::new();
        let rings = self.rings.as_ref().unwrap_or(&empty_rings);
        let retrogen = self.retrogen_empty_starts.contains(&source);
        let starts = program.generate_structure_starts_status_task(
            chunk_x,
            chunk_z,
            self.generate_structures,
            rings,
        )?;
        if retrogen {
            program.filter_retrogen_structure_starts(starts)
        } else {
            Ok(starts)
        }
    }

    pub(crate) fn has_ready_starts(&self, chunk: [i32; 2]) -> bool {
        self.starts_ready.contains(&chunk)
    }

    pub(crate) fn ensure_starts_chunk_with(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        generate: impl FnOnce(bool) -> SaveResult<NoiseStructureChunkData>,
    ) -> SaveResult<()> {
        if self.starts_ready.contains(&[chunk_x, chunk_z]) {
            return Ok(());
        }
        let starts = generate(self.generate_structures)?;
        self.publish_generated_starts(starts);
        Ok(())
    }

    pub(crate) fn rings(
        &mut self,
        program: &NoiseFinalDensityProgram,
    ) -> SaveResult<NoiseStructureRingPositions> {
        if self.generate_structures && self.rings.is_none() {
            self.rings = Some(program.structure_start_ring_positions()?);
        }
        Ok(self.rings.clone().unwrap_or_default())
    }

    pub(crate) fn ready_starts(&self, source: [i32; 2]) -> Option<&NoiseStructureChunkData> {
        self.starts_ready
            .contains(&source)
            .then(|| self.chunks.get(&source))
            .flatten()
    }

    /// Retrogen starts are filtered against the saved chunk, so only the
    /// serial owner generates them.
    pub(crate) fn starts_generatable(&self, source: [i32; 2]) -> bool {
        !self.starts_ready.contains(&source) && !self.retrogen_empty_starts.contains(&source)
    }

    pub(crate) fn references_published(&self, target: [i32; 2]) -> bool {
        self.references_ready.contains(&target)
    }

    pub(crate) fn is_prefetched(&self, target: [i32; 2]) -> bool {
        self.prefetched.contains_key(&target)
    }

    pub(crate) fn has_prefetched_carvers(&self, target: [i32; 2]) -> bool {
        matches!(
            self.prefetched.get(&target),
            Some(Prefetched::Fresh(..) | Prefetched::Resumed { .. })
        )
    }

    pub(crate) fn publish_batch(
        &mut self,
        starts: Vec<NoiseStructureChunkData>,
        columns: Vec<([i32; 2], Prefetched)>,
    ) {
        for starts in starts {
            if self.starts_generatable(starts.chunk()) {
                self.publish_generated_starts(starts);
            }
        }
        self.prefetched.extend(columns);
    }

    pub(crate) fn take_prefetched(&mut self, target: [i32; 2]) -> Option<Prefetched> {
        self.prefetched.remove(&target)
    }

    pub(crate) fn clear_prefetched(&mut self) {
        self.prefetched.clear();
    }

    pub(crate) fn publish_generated_starts(&mut self, starts: NoiseStructureChunkData) {
        let source = starts.chunk();
        self.retrogen_empty_starts.remove(&source);
        self.unloaded.remove(&source);
        self.chunks.insert(source, starts);

        self.starts_ready.insert(source);
    }

    fn ensure_starts_window(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<()> {
        self.ensure_starts_window_with(chunk_x, chunk_z, |cache, x, z| {
            cache.ensure_starts_chunk(program, x, z)
        })
    }

    fn ensure_starts_window_with(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        mut ensure: impl FnMut(&mut Self, i32, i32) -> SaveResult<()>,
    ) -> SaveResult<()> {
        for (source_x, source_z) in structure_starts_window(chunk_x, chunk_z)? {
            ensure(self, source_x, source_z)?;
        }
        Ok(())
    }

    pub(crate) fn ensure_references_chunk_with(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        mut generate: impl FnMut(bool, i32, i32) -> SaveResult<NoiseStructureChunkData>,
    ) -> SaveResult<()> {
        let target = [chunk_x, chunk_z];
        if self.references_ready.contains(&target) {
            return Ok(());
        }
        self.ensure_starts_window_with(chunk_x, chunk_z, |cache, x, z| {
            cache.ensure_starts_chunk_with(x, z, |enabled| generate(enabled, x, z))
        })?;
        self.publish_references_from_cached_window(target)
    }

    fn publish_references_from_cached_window(&mut self, target: [i32; 2]) -> SaveResult<()> {
        if !self.references_ready.contains(&target) {
            let target_starts = self.chunks.get(&target).ok_or_else(|| {
                invalid_status("target lost STRUCTURE_STARTS after its window completed")
            })?;
            let references =
                NoiseStructureChunkData::references_from_complete_source_window_borrowed(
                    target_starts,
                    |source_x, source_z| {
                        self.chunks.get(&[source_x, source_z]).ok_or_else(|| {
                            format!(
                                "cached STRUCTURE_STARTS source [{source_x},{source_z}] is absent"
                            )
                        })
                    },
                )
                .map_err(|error| invalid_status(error.to_string()))?;
            self.chunks.insert(target, references);
            self.references_ready.insert(target);
        }
        Ok(())
    }

    pub(crate) fn take_isolated_target(&mut self, target: [i32; 2], sources: &[[i32; 2]]) -> Self {
        let mut isolated = Self::new(self.generate_structures);
        let mut coords = std::collections::BTreeSet::new();
        coords.insert(target);
        coords.extend(sources.iter().copied());
        for coord in coords {
            if let Some(data) = self.chunks.remove(&coord) {
                isolated.chunks.insert(coord, data);
            }
            if self.starts_ready.contains(&coord) {
                isolated.starts_ready.insert(coord);
            }
            if self.references_ready.contains(&coord) {
                isolated.references_ready.insert(coord);
            }
        }
        isolated
    }

    pub(crate) fn restore_isolated_target(&mut self, isolated: Self) {
        for (coord, data) in isolated.chunks {
            self.chunks.insert(coord, data);
        }
    }

    pub(crate) fn prepare_structure_references(
        &mut self,
        program: &NoiseFinalDensityProgram,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<(NoiseStructureChunkData, JigsawBeardifier)> {
        self.ensure_starts_window(program, chunk_x, chunk_z)?;
        let target = [chunk_x, chunk_z];
        self.publish_references_from_cached_window(target)?;
        let references = self
            .chunks
            .get(&target)
            .cloned()
            .ok_or_else(|| invalid_status("target lost STRUCTURE_REFERENCES before NOISE"))?;
        let beardifier = program.beardifier_from_structure_references_borrowed(
            &references,
            |source_x, source_z| {
                self.chunks.get(&[source_x, source_z]).ok_or_else(|| {
                    format!("cached STRUCTURE_STARTS source [{source_x},{source_z}] is absent")
                })
            },
        )?;
        Ok((references, beardifier))
    }
}

pub(crate) fn structure_starts_window(chunk_x: i32, chunk_z: i32) -> SaveResult<Vec<(i32, i32)>> {
    let min_x = chunk_x
        .checked_sub(8)
        .ok_or_else(|| invalid_status("target chunk x underflows the structure window"))?;
    let max_x = chunk_x
        .checked_add(8)
        .ok_or_else(|| invalid_status("target chunk x overflows the structure window"))?;
    let min_z = chunk_z
        .checked_sub(8)
        .ok_or_else(|| invalid_status("target chunk z underflows the structure window"))?;
    let max_z = chunk_z
        .checked_add(8)
        .ok_or_else(|| invalid_status("target chunk z overflows the structure window"))?;
    Ok((min_x..=max_x)
        .flat_map(|x| (min_z..=max_z).map(move |z| (x, z)))
        .collect())
}

fn invalid_status(detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("structure status: {}", detail.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_task_status_distinguishes_starts_from_published_references() {
        let mut cache = NoiseStructureStatusCache::new(true);
        assert_eq!(cache.status(-3, 5), NoiseStructureTaskStatus::Empty);

        let component = NoiseStructureChunkData::from_supplied_starts(-3, 5, [], []);
        cache.chunks.insert([-3, 5], component);
        assert_eq!(
            cache.status(-3, 5),
            NoiseStructureTaskStatus::StructureStarts
        );
        assert_eq!(cache.starts_ready_count(), 1);
        assert_eq!(cache.references_ready_count(), 0);

        cache.references_ready.insert([-3, 5]);
        assert_eq!(
            cache.status(-3, 5),
            NoiseStructureTaskStatus::StructureReferences
        );
        assert_eq!(cache.references_ready_count(), 1);
    }
}
