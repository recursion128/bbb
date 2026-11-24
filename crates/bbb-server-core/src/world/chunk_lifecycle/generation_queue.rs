//! Ticket-driven generation off the world thread. Each refresh files the
//! targets it newly wants as small spatial batches and starts each batch's
//! pure stages on the rayon pool; the world thread commits batches strictly in
//! filing order once their stages are back. A batch commits one target at a
//! time, so a tick spends a bounded slice on the serial pipeline and resumes
//! the same batch on the next tick.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::save::SaveResult;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::chunk::parallel_generation::{GenerationBatch, PendingGeneration};
use crate::chunk::ChunkStore;
use crate::chunk_sender::ChunkKey;
use crate::world::{LevelId, ServerWorld};

/// Batches whose FEATURES/LIGHT reach one another: two radius-four footprints.
const BATCH_REACH: i32 = 8;

/// The side of the chunk tile one batch covers. Size 6 gives four pairwise
/// non-overlapping (`abs_diff > 2`) targets per residue class for parallel
/// FEATURES waves while keeping batch staging bounded.
const BATCH_TILE: i32 = 6;

/// Upper bound on non-overlapping targets committed in one budgeted parallel wave.
fn parallel_wave_limit() -> usize {
    rayon::current_num_threads().clamp(1, 8)
}

fn targets_non_overlapping(a: ChunkKey, b: ChunkKey) -> bool {
    a.0.abs_diff(b.0) > 2 || a.1.abs_diff(b.1) > 2
}

/// Stop between serial generation targets once this much world-thread time has elapsed.
pub(in crate::world) const COMMIT_BUDGET: Duration = Duration::from_millis(10);

struct Batch {
    requests: Vec<(ChunkKey, Java26ChunkStatus)>,
    full: VecDeque<ChunkKey>,
    loading: VecDeque<(ChunkKey, Java26ChunkStatus)>,
    stages: Option<PendingGeneration>,
    /// The stage output is installed and the serial targets are being drained.
    installed: bool,
    dependency_stages: Option<PendingGeneration>,
    /// Dependencies already committed, so ones that do not advance are left to
    /// the FULL owner instead of being retried.
    attempted_dependencies: FxHashSet<ChunkKey>,
}

enum Step {
    /// Missing radius-1/radius-2 columns of wanted FULL targets, to INITIALIZE_LIGHT.
    Dependencies(Vec<(ChunkKey, Java26ChunkStatus)>),
    FullBatch(Vec<ChunkKey>),
    LoadingBatch(Vec<(ChunkKey, Java26ChunkStatus)>),
}

impl Batch {
    /// The next batched step, dropping targets whose ticket no longer wants
    /// them: a dropped ticket stops its generation the way vanilla's
    /// ChunkHolder stops scheduling once its level falls.
    fn next_step(
        &mut self,
        chunks: &ChunkStore,
        requested: &FxHashMap<ChunkKey, u16>,
        target_limit: usize,
        wanted: impl Fn(ChunkKey, Java26ChunkStatus) -> bool,
    ) -> Option<Step> {
        self.full
            .retain(|&target| wanted(target, Java26ChunkStatus::Full));
        if !self.full.is_empty() {
            let mut deps = Vec::new();
            let mut seen = FxHashSet::default();
            for &target in &self.full {
                for dependency in chunks.collect_full_dependencies(target, |chunk| {
                    requested.get(&chunk).is_some_and(|&status| {
                        status >= status_bit(Java26ChunkStatus::InitializeLight)
                    })
                }) {
                    if !self.attempted_dependencies.contains(&dependency)
                        && seen.insert(dependency)
                        && (target_limit == usize::MAX
                            || deps
                                .iter()
                                .all(|&(chosen, _)| targets_non_overlapping(chosen, dependency)))
                    {
                        self.attempted_dependencies.insert(dependency);
                        deps.push((dependency, Java26ChunkStatus::InitializeLight));
                        if deps.len() == target_limit {
                            return Some(Step::Dependencies(deps));
                        }
                    }
                }
            }
            if !deps.is_empty() {
                return Some(Step::Dependencies(deps));
            }
            let full = if target_limit == usize::MAX {
                self.full.drain(..).collect()
            } else {
                let mut selected = Vec::with_capacity(target_limit.min(self.full.len()));
                let mut remaining = VecDeque::with_capacity(self.full.len());
                while let Some(chunk) = self.full.pop_front() {
                    if selected.len() < target_limit
                        && selected
                            .iter()
                            .all(|&chosen| targets_non_overlapping(chosen, chunk))
                    {
                        selected.push(chunk);
                    } else {
                        remaining.push_back(chunk);
                    }
                }
                self.full = remaining;
                selected
            };
            return Some(Step::FullBatch(full));
        }
        let mut loading = Vec::new();
        if target_limit == usize::MAX {
            while let Some(target) = self.loading.pop_front() {
                if wanted(target.0, target.1) {
                    loading.push(target);
                }
            }
        } else {
            let mut remaining = VecDeque::with_capacity(self.loading.len());
            while let Some(target) = self.loading.pop_front() {
                if !wanted(target.0, target.1) {
                    continue;
                }
                if loading.len() < target_limit
                    && loading
                        .iter()
                        .all(|&(chosen, _)| targets_non_overlapping(chosen, target.0))
                {
                    loading.push(target);
                } else {
                    remaining.push_back(target);
                }
            }
            self.loading = remaining;
        }
        (!loading.is_empty()).then_some(Step::LoadingBatch(loading))
    }

    /// The chunk box the batch's targets span.
    fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        self.full
            .iter()
            .chain(self.loading.iter().map(|(chunk, _)| chunk))
            .fold(None, |bounds, &(x, z)| {
                Some(match bounds {
                    None => (x, z, x, z),
                    Some((min_x, min_z, max_x, max_z)) => {
                        (min_x.min(x), min_z.min(z), max_x.max(x), max_z.max(z))
                    }
                })
            })
    }
}

/// How far one commit call goes.
#[derive(Clone, Copy)]
pub(in crate::world) enum Commit {
    /// The first `n` batches, waiting for their stages.
    Wait(usize),
    /// Whatever is ready, until the deadline passes.
    Until(Instant),
}

impl Commit {
    fn install(self, stages: Option<&PendingGeneration>, chunks: &mut ChunkStore) -> bool {
        let Some(stages) = stages else {
            return true;
        };
        let output = match self {
            Self::Wait(_) => stages.wait(),
            Self::Until(_) => match stages.try_take() {
                Some(output) => output,
                None => return false,
            },
        };
        if let Some(output) = output {
            chunks.install_generation(output);
        }
        true
    }
}

#[derive(Default)]
pub(in crate::world) struct GenerationQueue {
    batches: VecDeque<Batch>,
    /// Filing adds a status only when it exceeds every queued request for the chunk.
    requested: FxHashMap<ChunkKey, u16>,
    in_flight: FxHashSet<ChunkKey>,
    failed: FxHashSet<ChunkKey>,
    /// A running mean of one serial step, so a budgeted commit does not start
    /// a step it cannot finish in time.
    step_estimate: Duration,
}

fn status_bit(status: Java26ChunkStatus) -> u16 {
    1 << status as u32
}

impl GenerationQueue {
    pub(in crate::world) fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }

    pub(in crate::world) fn queued(&self, chunk: ChunkKey, status: Java26ChunkStatus) -> bool {
        self.requested
            .get(&chunk)
            .is_some_and(|&queued| queued >= status_bit(status))
    }

    fn push(&mut self, batch: Batch) {
        for &(chunk, status) in &batch.requests {
            *self.requested.entry(chunk).or_default() |= status_bit(status);
        }
        if let Some(stages) = &batch.stages {
            self.in_flight.extend(stages.columns());
        }
        self.batches.push_back(batch);
    }

    fn pop(&mut self) {
        let batch = self.batches.pop_front().unwrap();
        for (chunk, status) in batch.requests {
            let bits = self.requested.get_mut(&chunk).unwrap();
            *bits &= !status_bit(status);
            if *bits == 0 {
                self.requested.remove(&chunk);
            }
        }
        if let Some(stages) = batch.stages {
            for chunk in stages.columns() {
                self.in_flight.remove(chunk);
            }
        }
    }

    /// Moves the batches work on `chunks` could interleave with to the
    /// front, with every earlier batch those reach in turn, and returns how
    /// many there are. A batch it jumps reaches none of them, so committing
    /// out of filing order leaves the terrain as filing order would.
    fn front_reaching(&mut self, chunks: &[ChunkKey]) -> usize {
        let mut reach = chunks
            .iter()
            .map(|&(x, z)| (x, z, x, z))
            .collect::<Vec<_>>();
        let mut reaching = vec![false; self.batches.len()];
        for (index, batch) in self.batches.iter().enumerate().rev() {
            let Some(bounds) = batch.bounds() else {
                continue;
            };
            let near = |other: &(i32, i32, i32, i32)| {
                bounds.0 - BATCH_REACH <= other.2
                    && other.0 <= bounds.2 + BATCH_REACH
                    && bounds.1 - BATCH_REACH <= other.3
                    && other.1 <= bounds.3 + BATCH_REACH
            };
            if reach.iter().any(near) {
                reaching[index] = true;
                reach.push(bounds);
            }
        }
        let count = reaching.iter().filter(|&&reaching| reaching).count();
        let mut flags = reaching.into_iter();
        let (front, rest): (VecDeque<_>, VecDeque<_>) = std::mem::take(&mut self.batches)
            .into_iter()
            .partition(|_| flags.next().unwrap());
        self.batches = front;
        self.batches.extend(rest);
        count
    }
}

/// Drops stage results nothing consumed once no batch is queued: a later
/// batch skipped the columns an earlier one computed and adopts them from here.
pub(in crate::world) fn clear_unclaimed_prefetch(chunks: &mut ChunkStore, queue: &GenerationQueue) {
    if queue.batches.is_empty() {
        chunks.clear_prefetched();
    }
}

fn generation_met(chunks: &ChunkStore, (x, z): ChunkKey, status: Java26ChunkStatus) -> bool {
    if status == Java26ChunkStatus::Full {
        chunks.validate_served_full_column(x, z).is_ok()
            && chunks.published_worldgen_entity_records(x, z).is_empty()
    } else {
        chunks
            .generation_status(x, z)
            .is_some_and(|current| current >= status)
    }
}

impl ServerWorld {
    /// Files the targets no earlier batch covers, one batch per tile, the
    /// tiles holding the most advanced targets first, and starts their stages.
    pub(super) fn queue_generation(
        &mut self,
        dimension: &LevelId,
        targets: Vec<(ChunkKey, Java26ChunkStatus)>,
    ) {
        let level = self.level_mut_by_id(dimension);
        let queue = &level.chunk_lifecycle.generation;
        let mut tiles = BTreeMap::<ChunkKey, Vec<(ChunkKey, Java26ChunkStatus)>>::new();
        for (chunk, status) in targets {
            if !queue.queued(chunk, status)
                && !queue.failed.contains(&chunk)
                && !generation_met(&level.chunks, chunk, status)
            {
                let tile = (
                    chunk.0.div_euclid(BATCH_TILE),
                    chunk.1.div_euclid(BATCH_TILE),
                );
                tiles.entry(tile).or_default().push((chunk, status));
            }
        }
        if tiles.is_empty() {
            return;
        }
        let mut tiles = tiles.into_iter().collect::<Vec<_>>();
        tiles.sort_by_key(|(tile, targets)| {
            let best = targets.iter().map(|&(_, status)| status).max();
            (std::cmp::Reverse(best), *tile)
        });
        for (_, mut targets) in tiles {
            targets.sort_unstable();
            let (full, loading): (Vec<_>, Vec<_>) = targets
                .iter()
                .copied()
                .partition(|&(_, status)| status == Java26ChunkStatus::Full);
            let full = full.into_iter().map(|(chunk, _)| chunk).collect::<Vec<_>>();
            // A planning failure leaves every stage to the commit, which reports it.
            let stages = level
                .chunks
                .plan_generation(&full, &loading, &level.chunk_lifecycle.generation.in_flight)
                .ok()
                .flatten()
                .map(GenerationBatch::spawn);
            level.chunk_lifecycle.generation.push(Batch {
                requests: targets,
                full: full.into(),
                loading: loading.into(),
                stages,
                installed: false,
                dependency_stages: None,
                attempted_dependencies: FxHashSet::default(),
            });
        }
        // Planning restores the saved columns the batches read.
        self.reconcile_saved_column_accesses(dimension);
    }

    /// Commits filed batches in order, one serial target at a time.
    pub(in crate::world) fn commit_generation(
        &mut self,
        dimension: &LevelId,
        commit: Commit,
    ) -> SaveResult<()> {
        let mut batches = match commit {
            Commit::Wait(batches) => batches,
            Commit::Until(_) => usize::MAX,
        };
        let mut stepped = false;
        while batches > 0 {
            let level = self.level_mut_by_id(dimension);
            let queue = &mut level.chunk_lifecycle.generation;
            if let Commit::Until(deadline) = commit {
                let estimate = if stepped {
                    queue.step_estimate
                } else {
                    Duration::ZERO
                };
                if Instant::now() + estimate >= deadline {
                    return Ok(());
                }
            }
            let Some(front) = queue.batches.front_mut() else {
                return Ok(());
            };
            if !front.installed {
                if !commit.install(front.stages.as_ref(), &mut level.chunks) {
                    return Ok(());
                }
                front.installed = true;
            }
            let resumed_dependencies = front.dependency_stages.is_some();
            if resumed_dependencies {
                if !commit.install(front.dependency_stages.as_ref(), &mut level.chunks) {
                    return Ok(());
                }
                front.dependency_stages = None;
            }
            let levels = &level.chunk_lifecycle.loading_levels;
            let wanted = |chunk, status| {
                levels
                    .get(&chunk)
                    .and_then(|&level| super::ChunkLifecycle::generation_status(level))
                    .is_some_and(|wanted| wanted >= status)
            };
            let target_limit = match commit {
                Commit::Until(_) => parallel_wave_limit(),
                Commit::Wait(_) => usize::MAX,
            };
            let Some(step) = front.next_step(&level.chunks, &queue.requested, target_limit, wanted)
            else {
                queue.pop();
                clear_unclaimed_prefetch(&mut level.chunks, queue);
                batches -= 1;
                continue;
            };
            if matches!(commit, Commit::Until(_)) && !resumed_dependencies {
                if let Step::Dependencies(targets) = &step {
                    // Earlier commits may have advanced these columns since filing.
                    // Stage the current inputs before running their serial FEATURES.
                    if let Ok(Some(batch)) =
                        level
                            .chunks
                            .plan_generation(&[], targets, &FxHashSet::default())
                    {
                        front.dependency_stages = Some(batch.spawn());
                        for (chunk, _) in targets {
                            front.attempted_dependencies.remove(chunk);
                        }
                        return Ok(());
                    }
                }
            }
            let started = Instant::now();
            // Live writes since filing leave lit columns uncorrected; relighting
            // them first lets the batch demote and relight its neighbours.
            let committed = level
                .chunks
                .flush_live_relights()
                .and_then(|()| match &step {
                    // A refused dependency is the FULL owner's to report.
                    Step::Dependencies(targets) => {
                        if self
                            .level_mut_by_id(dimension)
                            .chunks
                            .ensure_loading_generation(targets)
                            .is_err()
                            && targets.len() > 1
                        {
                            for target in targets {
                                let _ = self
                                    .level_mut_by_id(dimension)
                                    .chunks
                                    .ensure_loading_generation(std::slice::from_ref(target));
                            }
                        }
                        Ok(())
                    }
                    Step::FullBatch(chunks) => self.prepare_full_chunk_columns(dimension, chunks),
                    Step::LoadingBatch(targets) => self
                        .level_mut_by_id(dimension)
                        .chunks
                        .ensure_loading_generation(targets),
                });
            let elapsed = started.elapsed();
            stepped = true;
            let queue = &mut self.level_mut_by_id(dimension).chunk_lifecycle.generation;
            queue.step_estimate = (queue.step_estimate * 7 + elapsed) / 8;
            // A column the pipeline refuses is not filed again; a budgeted
            // commit leaves it unloaded instead of stopping the level's tick.
            if let Err(error) = committed {
                match step {
                    Step::Dependencies(_) => {}
                    Step::FullBatch(chunks) => {
                        if chunks.len() <= 1 {
                            self.fail_generation_target(dimension, commit, chunks[0], error)?;
                        } else {
                            let mut any_failed = false;
                            for &chunk in &chunks {
                                if let Err(single_err) =
                                    self.prepare_full_chunk_columns(dimension, &[chunk])
                                {
                                    any_failed = true;
                                    self.fail_generation_target(
                                        dimension, commit, chunk, single_err,
                                    )?;
                                }
                            }
                            if !any_failed {
                                self.fail_generation_target(dimension, commit, chunks[0], error)?;
                            }
                        }
                    }
                    Step::LoadingBatch(targets) => {
                        if targets.len() <= 1 {
                            self.fail_generation_target(dimension, commit, targets[0].0, error)?;
                        } else {
                            let mut any_failed = false;
                            for &target in &targets {
                                if let Err(single_err) = self
                                    .level_mut_by_id(dimension)
                                    .chunks
                                    .ensure_loading_generation(&[target])
                                {
                                    any_failed = true;
                                    self.fail_generation_target(
                                        dimension, commit, target.0, single_err,
                                    )?;
                                }
                            }
                            if !any_failed {
                                self.fail_generation_target(
                                    dimension,
                                    commit,
                                    targets[0].0,
                                    error,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn fail_generation_target(
        &mut self,
        dimension: &LevelId,
        commit: Commit,
        target: ChunkKey,
        error: bbb_server_types::save::SaveError,
    ) -> SaveResult<()> {
        let level = self.level_mut_by_id(dimension);
        level.chunk_lifecycle.generation.failed.insert(target);
        if let Commit::Wait(_) = commit {
            level.chunk_lifecycle.generation.pop();
            clear_unclaimed_prefetch(&mut level.chunks, &level.chunk_lifecycle.generation);
            return Err(error);
        }
        tracing::warn!(event = "world.chunk_generation_failed", ?target, %error);
        Ok(())
    }

    /// Generation outside the queue commits the batches it could interleave
    /// with first, so it never overtakes them; batches out of its reach wait.
    pub(in crate::world) fn commit_generation_near(
        &mut self,
        dimension: &LevelId,
        chunks: &[ChunkKey],
    ) -> SaveResult<()> {
        let reaching = self
            .level_mut_by_id(dimension)
            .chunk_lifecycle
            .generation
            .front_reaching(chunks);
        self.commit_generation(dimension, Commit::Wait(reaching))
    }

    #[cfg(test)]
    pub(crate) fn queue_full_generation_for_test(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        chunk: ChunkKey,
    ) {
        self.queue_generation(&dimension.into(), vec![(chunk, Java26ChunkStatus::Full)]);
    }

    #[cfg(test)]
    pub(crate) fn finish_generation_for_test(&mut self) {
        for dimension in self.levels.keys().collect::<Vec<_>>() {
            self.commit_generation(&dimension, Commit::Wait(usize::MAX))
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promoted_requests_remain_queued_when_an_earlier_batch_retires() {
        let mut queue = GenerationQueue::default();
        for (chunk, status) in [
            ((-100, -100), Java26ChunkStatus::Full),
            ((2, 2), Java26ChunkStatus::Biomes),
            ((2, 2), Java26ChunkStatus::Full),
        ] {
            queue.push(Batch {
                requests: vec![(chunk, status)],
                full: if status == Java26ChunkStatus::Full {
                    [chunk].into()
                } else {
                    VecDeque::new()
                },
                loading: if status == Java26ChunkStatus::Full {
                    VecDeque::new()
                } else {
                    [(chunk, status)].into()
                },
                stages: None,
                installed: false,
                dependency_stages: None,
                attempted_dependencies: FxHashSet::default(),
            });
        }
        assert_eq!(queue.front_reaching(&[(2, 2)]), 2);
        queue.pop();
        assert!(queue.queued((2, 2), Java26ChunkStatus::Full));
        queue.pop();
        assert!(!queue.queued((2, 2), Java26ChunkStatus::Biomes));
        assert!(queue.queued((-100, -100), Java26ChunkStatus::Full));
    }

    #[test]
    fn budgeted_next_step_batches_pairwise_non_overlapping_targets() {
        let world = ServerWorld::new(crate::world::ServerWorldConfig::default()).unwrap();
        let store =
            world.chunk_store_for_test(bbb_server_types::save::Java26BuiltinDimension::Overworld);
        let requested = FxHashMap::default();
        let mut batch = Batch {
            requests: Vec::new(),
            full: [(0, 0), (1, 0), (3, 0), (0, 3), (3, 3), (4, 0)].into(),
            loading: VecDeque::new(),
            stages: None,
            installed: true,
            dependency_stages: None,
            attempted_dependencies: FxHashSet::default(),
        };
        let Some(Step::FullBatch(wave_one)) = batch.next_step(store, &requested, 4, |_, _| true)
        else {
            panic!("expected FullBatch");
        };
        assert_eq!(wave_one, vec![(0, 0), (3, 0), (0, 3), (3, 3)]);
        let Some(Step::FullBatch(wave_two)) = batch.next_step(store, &requested, 4, |_, _| true)
        else {
            panic!("expected second FullBatch");
        };
        assert_eq!(wave_two, vec![(1, 0), (4, 0)]);
        assert!(batch.next_step(store, &requested, 4, |_, _| true).is_none());
    }
}
