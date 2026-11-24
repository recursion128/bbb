//! `Mob.finalizeSpawn(STRUCTURE)` for the mobs structure pieces leave in
//! generated chunks (`StructureTemplate.java:500-502`, `SwampHutPiece.java:105`,
//! `WoodlandMansionPieces.java:1295`, `OceanRuinPieces.java:334`).
//!
//! Worldgen marks the records; this pass finalizes them in a generation-region
//! level before they are saved or join the entity store, so the marker never
//! reaches disk.

use super::*;
use crate::entity::save::{entity_chunk_records, read_entity_chunk_with_context};
use crate::world::chunk_lifecycle::full_preparation::worldgen_entity_document_with_registry;
use crate::world::registry_runtime::EntityNbtReadSnapshot;
use bbb_server_worldgen::noise::proto_chunk_entity::{
    has_structure_finalize_marker, take_structure_finalize_marker,
};

type Records = Vec<Vec<NbtSummaryEntry>>;

impl OriginalMobInputs<'_> {
    /// Finalizes every marked worldgen entity record of the loaded chunks.
    pub(in crate::world) fn finalize_structure_mobs(
        &self,
        chunks: &mut ChunkStore,
        input: &EntityNbtReadSnapshot,
    ) {
        let loaded: Vec<(i32, i32)> = chunks.loaded_chunks().collect();
        self.finalize_structure_mobs_in(chunks, &loaded, input);
    }

    /// Finalizes every marked worldgen entity record within `target_chunks`.
    pub(in crate::world) fn finalize_structure_mobs_in(
        &self,
        chunks: &mut ChunkStore,
        target_chunks: &[(i32, i32)],
        input: &EntityNbtReadSnapshot,
    ) {
        let marked: Vec<(i32, i32)> = target_chunks
            .iter()
            .copied()
            .filter(|&(x, z)| {
                chunks
                    .proto_entity_records(x, z)
                    .iter()
                    .chain(chunks.published_worldgen_entity_records(x, z))
                    .any(|record| has_structure_finalize_marker(record))
            })
            .collect();
        for chunk in marked {
            let Some(lists) = chunks.worldgen_entity_records_mut(chunk) else {
                continue;
            };
            let pending = lists.map(|list| {
                let (mut marked, kept): (Records, Records) = std::mem::take(list)
                    .into_iter()
                    .partition(|record| has_structure_finalize_marker(record));
                *list = kept;
                for record in &mut marked {
                    take_structure_finalize_marker(record);
                }
                marked
            });
            let finalized = pending.map(|records| {
                if records.is_empty() {
                    return records;
                }
                self.finalized_records(chunks, chunk, &records, input)
                    .unwrap_or(records)
            });
            let lists = chunks
                .worldgen_entity_records_mut(chunk)
                .expect("the chunk stays loaded");
            for (list, records) in lists.into_iter().zip(finalized) {
                list.extend(records);
            }
        }
    }

    /// The records after `finalizeSpawn`; `None` leaves them unfinalized.
    fn finalized_records(
        &self,
        chunks: &ChunkStore,
        chunk: (i32, i32),
        records: &[Vec<NbtSummaryEntry>],
        input: &EntityNbtReadSnapshot,
    ) -> Option<Records> {
        let document = worldgen_entity_document_with_registry(
            chunk,
            records.to_vec(),
            input.content_registry(),
        )
        .ok()?;
        let random = chunk_random(
            self.world.seed.wrapping_add(1),
            self.dimension.as_str(),
            chunk,
        );
        let mut runtime = self.original_runtime(chunks, random).ok()?;
        let ids =
            read_entity_chunk_with_context(&mut runtime.store, &document, input.context()).ok()?;
        for id in ids {
            if runtime.store.vehicle(id).is_some() {
                continue;
            }
            let Some(mut mob) = runtime.store.take_fresh(id) else {
                continue;
            };
            let _ = crate::entity::finalize_spawn(
                &mut NaturalSpawnLevel {
                    environment: &runtime.environment,
                    store: &mut runtime.store,
                },
                &mut mob,
                crate::entity::SpawnReason::Structure,
                None,
                &runtime.random,
            );
            runtime.store.add_fresh(mob);
        }
        let saved =
            entity_chunk_records(&runtime.store, chunk, &rustc_hash::FxHashSet::default()).ok()?;
        Some(
            saved
                .into_iter()
                .map(bbb_server_persistence::Java26EntityRecord::into_raw)
                .collect(),
        )
    }
}

impl ServerWorld {
    /// Runs [`OriginalMobInputs::finalize_structure_mobs`] over every level.
    pub(in crate::world) fn finalize_structure_mobs(&mut self) {
        let input = self.entity_nbt_read_snapshot();
        for dimension in self.level_ids() {
            let (chunks, inputs) = self.original_mob_inputs(&dimension);
            inputs.finalize_structure_mobs(chunks, &input);
        }
    }
}

#[cfg(test)]
mod tests;
