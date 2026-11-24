use std::collections::{BTreeMap, BTreeSet};

use super::{invalid_carvers_region, ChunkSource, ChunkStore};
use crate::chunk::noise_generator::NoiseFinalDensityProgram;
use bbb_server_types::chunk_data::ChunkGenerationState;
use bbb_server_types::save::SaveResult;

const WINDOW_SIDE: usize = 5;
const WINDOW_LEN: usize = WINDOW_SIDE * WINDOW_SIDE;
const TARGET_SIDE: usize = 3;
const TARGET_LEN: usize = TARGET_SIDE * TARGET_SIDE;

pub(crate) mod parallel_waves;
pub(crate) mod registered_sequence;
#[cfg(test)]
mod registered_sequence_tests;

use registered_sequence::prepare_registered_structure_targets;

impl ChunkStore {
    pub fn generate_fresh_initialize_light_window(&mut self, center: (i32, i32)) -> SaveResult<()> {
        self.generate_initialize_light_window(center, false, None)
    }

    pub(super) fn ensure_initialize_light_windows_for_relight(
        &mut self,
        centers: &[(i32, i32)],
    ) -> SaveResult<()> {
        if centers.is_empty() {
            return Ok(());
        }
        self.generate_initialize_light_windows(centers, true, None, true)
    }

    pub(super) fn generate_loading_features_batch(
        &mut self,
        targets: &BTreeSet<(i32, i32)>,
    ) -> SaveResult<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let mut remaining = targets.clone();
        let mut selected_centers = Vec::new();
        while let Some(&center) = remaining.first() {
            selected_centers.push(center);
            remaining.retain(|chunk| {
                !(chunk.0.abs_diff(center.0) <= 1 && chunk.1.abs_diff(center.1) <= 1)
            });
        }
        self.generate_initialize_light_windows(&selected_centers, true, Some(targets), false)
    }

    pub(super) fn generate_initialize_light_window(
        &mut self,
        center: (i32, i32),
        allow_existing: bool,
        loading_targets: Option<&BTreeSet<(i32, i32)>>,
    ) -> SaveResult<()> {
        self.generate_initialize_light_windows(
            &[center],
            allow_existing,
            loading_targets,
            loading_targets.is_none(),
        )
    }

    fn generate_initialize_light_windows(
        &mut self,
        centers: &[(i32, i32)],
        allow_existing: bool,
        loading_targets: Option<&BTreeSet<(i32, i32)>>,
        initialize_light: bool,
    ) -> SaveResult<()> {
        let mut targets = Vec::new();
        let mut seen_targets = BTreeSet::new();
        for &center in centers {
            for target in feature_window_targets(center)? {
                if loading_targets.is_none_or(|selected| selected.contains(&target))
                    && seen_targets.insert(target)
                {
                    targets.push(target);
                }
            }
        }
        let allowed_chunks = targets
            .iter()
            .try_fold(BTreeSet::new(), |mut chunks, &target| {
                chunks.extend(feature_target_chunks(target)?);
                Ok::<_, bbb_server_types::save::SaveError>(chunks)
            })?;
        let durable_chunks = self.durable_chunks();
        let program = match &self.source {
            ChunkSource::NoiseCarvers(program) => program.clone(),
            ChunkSource::Flat(_) | ChunkSource::Debug(_) => {
                return Err(invalid_carvers_region(
                    [centers[0].0, centers[0].1],
                    "fresh FEATURES windows require a noise CARVERS store",
                ))
            }
        };
        for &center in centers {
            for (chunk_x, chunk_z) in feature_window_schedule_coordinates(center)? {
                if allowed_chunks.contains(&(chunk_x, chunk_z)) {
                    self.try_load_column_unlit(chunk_x, chunk_z)?;
                }
            }
        }
        let mut new_targets = BTreeSet::new();
        let mut ordered_new_targets = Vec::new();
        for &target in &targets {
            let completed = self.features_completed(target);
            if !allow_existing && completed {
                return Err(invalid_carvers_region(
                    [target.0, target.1],
                    "the target already completed bounded FEATURES decoration",
                ));
            }
            if !completed {
                new_targets.insert(target);
                ordered_new_targets.push(target);
            }
        }
        if !allow_existing {
            for target in &new_targets {
                for chunk in feature_target_chunks(*target)? {
                    if matches!(
                        self.generation(chunk),
                        Some(ChunkGenerationState::NoiseFull(_))
                    ) {
                        return Err(invalid_carvers_region(
                            [target.0, target.1],
                            format!(
                                "an unfinished FEATURES target would rewrite FULL chunk [{},{}]",
                                chunk.0, chunk.1
                            ),
                        ));
                    }
                }
            }
        }
        let blending_border_ticks = new_targets
            .iter()
            .map(|&target| {
                self.blending_border_tick_witness(target)
                    .map(|witness| (target, witness))
            })
            .collect::<SaveResult<BTreeMap<_, _>>>()?;

        let (mut structure_steps, dynamic_sources) = prepare_registered_structure_targets(
            self,
            &program,
            &new_targets,
            loading_targets.is_some(),
        )?;
        let outcome = parallel_waves::execute_feature_targets_parallel(
            self,
            &program,
            &ordered_new_targets,
            &mut structure_steps,
            &blending_border_ticks,
        )?;
        let durable_generation_changes = outcome
            .changed_chunks
            .intersection(&durable_chunks)
            .copied()
            .collect();
        let current_durable_chunks = self.durable_chunks();
        for chunk in durable_feature_rewrite_chunks(
            durable_generation_changes,
            &dynamic_sources,
            &current_durable_chunks,
        ) {
            self.mark_whole_rewrite_pending(chunk)?;
        }

        for &chunk in &targets {
            if self.is_storage_backed_generation(chunk)
                && matches!(
                    self.generation(chunk),
                    Some(ChunkGenerationState::Features(_))
                )
            {
                self.mark_whole_rewrite_pending(chunk)?;
            }
        }
        outcome.result?;
        if initialize_light {
            self.initialize_light_windows(centers)?;
        }
        Ok(())
    }
}

fn durable_feature_rewrite_chunks(
    mut generation_changes: BTreeSet<(i32, i32)>,
    dynamic_sources: &BTreeSet<(i32, i32)>,
    durable_chunks: &BTreeSet<(i32, i32)>,
) -> BTreeSet<(i32, i32)> {
    generation_changes.extend(dynamic_sources.intersection(durable_chunks).copied());
    generation_changes
}

impl ChunkStore {
    pub(super) fn feature_relight_centers(
        &self,
        center: (i32, i32),
        selected: Option<&BTreeSet<(i32, i32)>>,
    ) -> SaveResult<BTreeSet<(i32, i32)>> {
        let targets = feature_window_targets(center)?
            .into_iter()
            .filter(|target| {
                selected.is_none_or(|selected| selected.contains(target))
                    && !self.features_completed(*target)
            })
            .collect::<Vec<_>>();
        let mut relight = BTreeSet::new();
        for target in targets {
            for dx in -2..=2 {
                let Some(cx) = target.0.checked_add(dx) else {
                    continue;
                };
                for dz in -2..=2 {
                    let Some(cz) = target.1.checked_add(dz) else {
                        continue;
                    };
                    let chunk = (cx, cz);
                    if relight.contains(&chunk) {
                        continue;
                    }
                    if let Some(loaded) = self.chunks.get(&chunk) {
                        if matches!(
                            loaded.generation,
                            Some(
                                ChunkGenerationState::Light(_)
                                    | ChunkGenerationState::Spawn(_)
                                    | ChunkGenerationState::NoiseFull(_)
                            )
                        ) {
                            relight.insert(chunk);
                        }
                    }
                }
            }
        }
        Ok(relight)
    }

    pub(super) fn features_completed(&self, chunk: (i32, i32)) -> bool {
        self.generation(chunk)
            .is_some_and(|state| !matches!(state, ChunkGenerationState::Carvers { .. }))
    }
}

fn feature_window_coordinates(center: (i32, i32)) -> SaveResult<[(i32, i32); WINDOW_LEN]> {
    let mut coordinates = [(0, 0); WINDOW_LEN];
    for (index, coordinate) in coordinates.iter_mut().enumerate() {
        let offset_x = i32::try_from(index % WINDOW_SIDE).expect("window x index fits i32") - 2;
        let offset_z = i32::try_from(index / WINDOW_SIDE).expect("window z index fits i32") - 2;
        coordinate.0 = center.0.checked_add(offset_x).ok_or_else(|| {
            invalid_carvers_region([center.0, center.1], "FEATURES window x overflows")
        })?;
        coordinate.1 = center.1.checked_add(offset_z).ok_or_else(|| {
            invalid_carvers_region([center.0, center.1], "FEATURES window z overflows")
        })?;
    }
    Ok(coordinates)
}

fn feature_window_schedule_coordinates(center: (i32, i32)) -> SaveResult<[(i32, i32); WINDOW_LEN]> {
    // This is the per-task x-outer/z-inner order; independent FULL tasks are
    // not commutative (`ChunkGenerationTask.java:115-128`).
    let mut coordinates = feature_window_coordinates(center)?;
    coordinates.sort_unstable();
    Ok(coordinates)
}

fn feature_target_chunks(target: (i32, i32)) -> SaveResult<[(i32, i32); TARGET_LEN]> {
    feature_window_targets(target)
}

fn feature_window_targets(center: (i32, i32)) -> SaveResult<[(i32, i32); TARGET_LEN]> {
    let mut targets = [(0, 0); TARGET_LEN];
    for (index, target) in targets.iter_mut().enumerate() {
        let offset_x = i32::try_from(index / TARGET_SIDE).expect("target x index fits i32") - 1;
        let offset_z = i32::try_from(index % TARGET_SIDE).expect("target z index fits i32") - 1;
        target.0 = center.0.checked_add(offset_x).ok_or_else(|| {
            invalid_carvers_region([center.0, center.1], "FEATURES target x overflows")
        })?;
        target.1 = center.1.checked_add(offset_z).ok_or_else(|| {
            invalid_carvers_region([center.0, center.1], "FEATURES target z overflows")
        })?;
    }
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::chunk::noise_generator::overworld_program;

    const CENTER: (i32, i32) = (0, 0);

    fn program() -> Arc<NoiseFinalDensityProgram> {
        overworld_program(true, 0)
    }

    fn fresh_window_store() -> ChunkStore {
        let mut store =
            ChunkStore::new_overworld_carvers_with_structure_generation(program(), false).unwrap();
        for (x, z) in feature_window_coordinates(CENTER).unwrap() {
            store.try_load_column(x, z).unwrap();
        }
        store
    }

    #[test]
    fn features_window_structure_disabled_publishes_exact_initialize_light_window() {
        let mut store = fresh_window_store();
        let coordinates = feature_window_coordinates(CENTER).unwrap();

        store
            .generate_fresh_initialize_light_window(CENTER)
            .unwrap();

        let mut postprocessing_count = 0;
        for chunk in coordinates {
            assert!(store.chunk_flag(chunk, |chunk| chunk.pending_initial_persistence));
            if (chunk.0 - CENTER.0).abs() <= 1 && (chunk.1 - CENTER.1).abs() <= 1 {
                let Some(ChunkGenerationState::InitializeLight(initialized)) =
                    store.generation(chunk)
                else {
                    panic!("inner FEATURES target {chunk:?} was not initialized")
                };
                postprocessing_count += initialized.features().postprocessing_positions().len();
            } else {
                let Some(ChunkGenerationState::Carvers {
                    postprocessing_positions,
                    ..
                }) = store.generation(chunk)
                else {
                    panic!("outer FEATURES dependency {chunk:?} did not remain CARVERS")
                };
                postprocessing_count += postprocessing_positions.len();
            }
        }
        assert!(postprocessing_count > 0);
    }

    #[test]
    fn durable_structure_sources_join_generation_rewrite_set() {
        let column_change = (0, 0);
        let durable_source = (1, 0);
        let fresh_source = (2, 0);
        assert_eq!(
            durable_feature_rewrite_chunks(
                BTreeSet::from([column_change]),
                &BTreeSet::from([durable_source, fresh_source]),
                &BTreeSet::from([column_change, durable_source]),
            ),
            BTreeSet::from([column_change, durable_source])
        );
    }
}
