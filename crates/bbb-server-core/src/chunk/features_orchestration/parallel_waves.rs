//! Deterministic DAG-levelized rayon execution for non-overlapping FEATURES targets.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bbb_server_types::save::{SaveError, SaveResult};
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use super::feature_target_chunks;
use super::registered_sequence::RegisteredStructureSteps;
use crate::chunk::blending::BlendingBorderTickWitness;
use crate::chunk::block_entities::{ChunkBlockEntitySnapshot, LevelBlockEntities};
use crate::chunk::noise_generator::features::FeatureRegionData;
use crate::chunk::noise_generator::NoiseFinalDensityProgram;
use crate::chunk::poi::PoiBlockChange;
use crate::chunk::structure_status::NoiseStructureStatusCache;
use crate::chunk::{Chunk, ChunkStore};
use crate::registry_tags::RegistryTagSnapshot;

type ChunkKey = (i32, i32);

struct TargetTaskSpec {
    target: ChunkKey,
    region_chunks: [ChunkKey; 9],
    region_chunk_set: BTreeSet<ChunkKey>,
    structure_sources: Vec<[i32; 2]>,
    steps: RegisteredStructureSteps,
    blending_witness: Option<BlendingBorderTickWitness>,
}

struct IsolatedFeatureTask {
    target: ChunkKey,
    region_chunks: [ChunkKey; 9],
    region_chunk_set: BTreeSet<ChunkKey>,
    chunks: FxHashMap<ChunkKey, Chunk>,
    structure_status: NoiseStructureStatusCache,
    steps: RegisteredStructureSteps,
    be_snapshot: ChunkBlockEntitySnapshot,
    block_entities: LevelBlockEntities,
    blending_witness: Option<BlendingBorderTickWitness>,
}

struct IsolatedFeatureOutput {
    region_chunks: [ChunkKey; 9],
    chunks: FxHashMap<ChunkKey, Chunk>,
    structure_status: NoiseStructureStatusCache,
    be_snapshot: ChunkBlockEntitySnapshot,
    block_entities: LevelBlockEntities,
    pending_light_changes: FxHashMap<ChunkKey, FxHashSet<(i32, i32, i32)>>,
    poi_changes: Vec<PoiBlockChange>,
    dirty: BTreeSet<ChunkKey>,
    result: SaveResult<()>,
}

pub(super) struct ParallelFeaturesOutcome {
    pub(super) changed_chunks: BTreeSet<ChunkKey>,
    pub(super) result: SaveResult<()>,
}

fn footprints_overlap(
    center_a: ChunkKey,
    sources_a: &[[i32; 2]],
    center_b: ChunkKey,
    sources_b: &[[i32; 2]],
) -> bool {
    if center_a.0.abs_diff(center_b.0) <= 2 && center_a.1.abs_diff(center_b.1) <= 2 {
        return true;
    }
    if sources_a.is_empty() && sources_b.is_empty() {
        return false;
    }
    let contains = |sources: &[[i32; 2]], center: ChunkKey, coord: [i32; 2]| {
        coord == [center.0, center.1] || sources.contains(&coord)
    };
    sources_b
        .iter()
        .any(|&coord| contains(sources_a, center_a, coord))
        || sources_a
            .iter()
            .any(|&coord| contains(sources_b, center_b, coord))
}

fn compute_waves(specs: &[TargetTaskSpec]) -> Vec<Vec<usize>> {
    if specs.is_empty() {
        return Vec::new();
    }
    let mut wave_of = vec![0_usize; specs.len()];
    let mut num_waves = 1_usize;
    for i in 0..specs.len() {
        let mut wave = 0_usize;
        for j in 0..i {
            if wave_of[j] >= wave
                && footprints_overlap(
                    specs[i].target,
                    &specs[i].structure_sources,
                    specs[j].target,
                    &specs[j].structure_sources,
                )
            {
                wave = wave_of[j] + 1;
            }
        }
        wave_of[i] = wave;
        num_waves = num_waves.max(wave + 1);
    }
    let mut waves = vec![Vec::new(); num_waves];
    for (index, &wave) in wave_of.iter().enumerate() {
        waves[wave].push(index);
    }
    waves
}

fn run_single_in_place(
    store: &mut ChunkStore,
    program: &Arc<NoiseFinalDensityProgram>,
    spec: TargetTaskSpec,
    changed_chunks: &mut BTreeSet<ChunkKey>,
) -> SaveResult<()> {
    let context = program.structure_generation_context()?;
    let tags = store.registry_tags.as_deref();
    let mut data = FeatureRegionData::new(
        &mut store.chunks,
        &mut store.block_entities,
        &mut store.poi,
        &mut store.pending_light_changes,
        &spec.region_chunk_set,
    );
    let source = program.feature_placement_source([spec.target.0, spec.target.1])?;
    let mut steps = spec.steps;
    let mut region = data.region(&source, [spec.target.0, spec.target.1])?;
    let mut placement = Ok(());
    for step in 0..source.step_count() {
        if let Err(err) = steps.place_step(&context, &mut store.structure_status, step, &mut region)
        {
            placement = Err(err);
            break;
        }
        if let Err(err) = region.place_feature_step(&source, step) {
            placement = Err(err);
            break;
        }
    }
    if placement.is_ok() {
        placement = region.generate_blending_border_ticks(spec.blending_witness.as_ref(), tags);
    }
    let _ = region;
    if placement.is_ok() {
        data.complete_target(spec.target);
    }
    changed_chunks.extend(std::mem::take(&mut data.dirty));
    placement
}

fn run_isolated_task(
    mut task: IsolatedFeatureTask,
    program: &NoiseFinalDensityProgram,
    tags: Option<&RegistryTagSnapshot>,
) -> IsolatedFeatureOutput {
    let mut pending_light_changes = FxHashMap::default();
    let mut dirty = BTreeSet::new();
    let mut poi_changes = Vec::new();
    let result = (|| -> SaveResult<()> {
        let context = program.structure_generation_context()?;
        let source = program.feature_placement_source([task.target.0, task.target.1])?;
        let mut data = FeatureRegionData::new_isolated(
            &mut task.chunks,
            &mut task.block_entities,
            &mut pending_light_changes,
            &task.region_chunk_set,
        );
        let mut region = data.region(&source, [task.target.0, task.target.1])?;
        for step in 0..source.step_count() {
            task.steps
                .place_step(&context, &mut task.structure_status, step, &mut region)?;
            region.place_feature_step(&source, step)?;
        }
        region.generate_blending_border_ticks(task.blending_witness.as_ref(), tags)?;
        let _ = region;
        data.complete_target(task.target);
        dirty = std::mem::take(&mut data.dirty);
        poi_changes = std::mem::take(&mut data.poi_changes);
        Ok(())
    })();
    IsolatedFeatureOutput {
        region_chunks: task.region_chunks,
        chunks: task.chunks,
        structure_status: task.structure_status,
        be_snapshot: task.be_snapshot,
        block_entities: task.block_entities,
        pending_light_changes,
        poi_changes,
        dirty,
        result,
    }
}

pub(super) fn execute_feature_targets_parallel(
    store: &mut ChunkStore,
    program: &Arc<NoiseFinalDensityProgram>,
    ordered_targets: &[ChunkKey],
    structure_steps: &mut BTreeMap<ChunkKey, RegisteredStructureSteps>,
    blending_border_ticks: &BTreeMap<ChunkKey, Option<BlendingBorderTickWitness>>,
) -> SaveResult<ParallelFeaturesOutcome> {
    let generate_structures = store.structure_status.generate_structures();
    let mut specs = Vec::with_capacity(ordered_targets.len());
    for &target in ordered_targets {
        let region_chunks = feature_target_chunks(target)?;
        let region_chunk_set = region_chunks.iter().copied().collect::<BTreeSet<_>>();
        let structure_sources = if generate_structures {
            store
                .structure_status
                .target_reference_source_chunks([target.0, target.1])?
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        let steps = structure_steps.remove(&target).unwrap_or_default();
        let blending_witness = blending_border_ticks.get(&target).and_then(Clone::clone);
        specs.push(TargetTaskSpec {
            target,
            region_chunks,
            region_chunk_set,
            structure_sources,
            steps,
            blending_witness,
        });
    }

    let waves = compute_waves(&specs);
    let mut spec_slots = specs.into_iter().map(Some).collect::<Vec<_>>();
    let mut changed_chunks = BTreeSet::new();
    let mut first_error: Option<SaveError> = None;

    for wave_indices in waves {
        if wave_indices.len() == 1 {
            let spec = spec_slots[wave_indices[0]].take().expect("wave task slot");
            if let Err(err) = run_single_in_place(store, program, spec, &mut changed_chunks) {
                first_error = Some(err);
                break;
            }
            continue;
        }

        let mut tasks = Vec::with_capacity(wave_indices.len());
        for index in wave_indices {
            let spec = spec_slots[index].take().expect("wave task slot");
            let be_snapshot = store
                .block_entities
                .snapshot_for_chunks(&store.chunks, &spec.region_chunks);
            let block_entities = LevelBlockEntities::from_snapshot_on_loan(&be_snapshot);
            let mut chunks =
                FxHashMap::with_capacity_and_hasher(spec.region_chunks.len(), Default::default());
            for chunk in spec.region_chunks {
                if let Some(loaded) = store.chunks.remove(&chunk) {
                    chunks.insert(chunk, loaded);
                }
            }
            let structure_status = store
                .structure_status
                .take_isolated_target([spec.target.0, spec.target.1], &spec.structure_sources);
            tasks.push(IsolatedFeatureTask {
                target: spec.target,
                region_chunks: spec.region_chunks,
                region_chunk_set: spec.region_chunk_set,
                chunks,
                structure_status,
                steps: spec.steps,
                be_snapshot,
                block_entities,
                blending_witness: spec.blending_witness,
            });
        }

        let tags = store.registry_tags.as_deref();
        let program_ref = program.as_ref();
        let outputs: Vec<IsolatedFeatureOutput> = tasks
            .into_par_iter()
            .map(|task| run_isolated_task(task, program_ref, tags))
            .collect();

        for output in outputs {
            for (coord, chunk) in output.chunks {
                store.chunks.insert(coord, chunk);
            }
            store
                .structure_status
                .restore_isolated_target(output.structure_status);
            store.block_entities.reconcile_isolated(
                &store.chunks,
                &output.region_chunks,
                output.be_snapshot,
                output.block_entities,
            );
            changed_chunks.extend(output.dirty);
            for (chunk, positions) in output.pending_light_changes {
                store
                    .pending_light_changes
                    .entry(chunk)
                    .or_default()
                    .extend(positions);
            }
            if !output.poi_changes.is_empty() {
                store.poi.apply_changes(output.poi_changes);
            }
            if first_error.is_none() {
                if let Err(err) = output.result {
                    first_error = Some(err);
                }
            }
        }
        if first_error.is_some() {
            break;
        }
    }

    Ok(ParallelFeaturesOutcome {
        changed_chunks,
        result: match first_error {
            Some(err) => Err(err),
            None => Ok(()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::noise_generator::overworld_program;

    #[test]
    fn batched_parallel_features_matches_serial_single_column_generation() {
        let program = overworld_program(true, 1337);
        let mut batched =
            ChunkStore::new_overworld_carvers_with_structure_generation(program.clone(), true)
                .unwrap();
        let mut serial =
            ChunkStore::new_overworld_carvers_with_structure_generation(program, true).unwrap();

        let region = (0..6)
            .flat_map(|x| (0..6).map(move |z| (x, z)))
            .collect::<Vec<_>>();

        batched.try_ensure_noise_full_columns(&region).unwrap();
        for &(x, z) in &region {
            serial.try_ensure_noise_full_column(x, z).unwrap();
        }

        for (x, z) in region {
            let a = batched.column(x, z).expect("batched column");
            let b = serial.column(x, z).expect("serial column");
            assert_eq!(a, b, "column [{x},{z}] diverged between batched and serial");
            assert_eq!(
                batched.proto_entity_records(x, z),
                serial.proto_entity_records(x, z),
                "proto entities diverged at [{x},{z}]"
            );
        }
    }
}
