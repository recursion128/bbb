//! STRUCTURE_STARTS, BIOMES and the noise/surface/carver pass depend only on
//! the seed, the frozen program, the start window and the column itself, so a
//! generation batch computes them on the rayon pool and the serial pipeline
//! adopts each result exactly where it would have generated it. FEATURES,
//! LIGHT and FULL stay serial in request order, so the output does not depend
//! on thread timing.

use std::collections::BTreeSet;
use std::panic::AssertUnwindSafe;
use std::sync::{mpsc, Arc};

use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::chunk_data::{ChunkColumn, ChunkGenerationState};
use bbb_server_types::save::SaveResult;
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use super::blending::BlendingResolver;
use super::noise_generator::{
    NoiseFinalDensityProgram, NoiseGeneratedChunk, NoiseStructureRingPositions,
};
use super::restored_generation::pre_carvers_java_status;
use super::structure_status::structure_starts_window;
use super::{ChunkSource, ChunkStore};

/// A column stage computed ahead of the serial pipeline, with the input it
/// was computed from so adoption can prove nothing changed since.
#[derive(Clone, Debug)]
pub(crate) enum Prefetched {
    /// A fresh CARVERS column and its STRUCTURE_REFERENCES.
    Fresh(NoiseGeneratedChunk, NoiseStructureChunkData),
    Resumed {
        input: ResumeInput,
        generated: NoiseGeneratedChunk,
    },
    Biomes {
        input: ChunkColumn,
        output: ChunkColumn,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResumeInput {
    pub(crate) status: Java26ChunkStatus,
    pub(crate) column: ChunkColumn,
    pub(crate) postprocessing: Vec<(i32, i32, i32)>,
}

enum TaskKind {
    Fresh,
    Resume(ResumeInput),
    Biomes(ChunkColumn),
}

struct ColumnTask {
    chunk: [i32; 2],
    references_published: bool,
    kind: TaskKind,
}

/// The owned inputs of one batch.
pub struct GenerationBatch {
    program: Arc<NoiseFinalDensityProgram>,
    generate_structures: bool,
    rings: NoiseStructureRingPositions,
    ready_starts: FxHashMap<[i32; 2], NoiseStructureChunkData>,
    missing_starts: Vec<[i32; 2]>,
    tasks: Vec<ColumnTask>,
}

pub struct GenerationOutput {
    starts: Vec<NoiseStructureChunkData>,
    columns: Vec<([i32; 2], Prefetched)>,
}

/// A batch running on the rayon pool.
pub struct PendingGeneration {
    columns: Vec<(i32, i32)>,
    receiver: mpsc::Receiver<Option<GenerationOutput>>,
}

impl PendingGeneration {
    pub fn columns(&self) -> &[(i32, i32)] {
        &self.columns
    }

    /// `None` while the batch runs; `Some(None)` when it panicked, which leaves
    /// every stage to the serial pipeline.
    pub fn try_take(&self) -> Option<Option<GenerationOutput>> {
        match self.receiver.try_recv() {
            Ok(output) => Some(output),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(None),
        }
    }

    pub fn wait(&self) -> Option<GenerationOutput> {
        self.receiver.recv().ok().flatten()
    }
}

impl GenerationBatch {
    pub fn spawn(self) -> PendingGeneration {
        let columns = self
            .tasks
            .iter()
            .map(|task| (task.chunk[0], task.chunk[1]))
            .collect();
        let (sender, receiver) = mpsc::channel();
        rayon::spawn(move || {
            // A panic must not abort the pool; the serial pipeline redoes the work.
            let output = std::panic::catch_unwind(AssertUnwindSafe(|| self.run())).ok();
            let _ = sender.send(output);
        });
        PendingGeneration { columns, receiver }
    }

    /// Failed stages are dropped: the serial pipeline repeats them and reports
    /// the error from its own call site.
    pub fn run(self) -> GenerationOutput {
        let program = &*self.program;
        let computed = self
            .missing_starts
            .par_iter()
            .filter_map(|&[x, z]| {
                program
                    .generate_structure_starts_status_task(
                        x,
                        z,
                        self.generate_structures,
                        &self.rings,
                    )
                    .ok()
            })
            .collect::<Vec<_>>();
        let mut starts = self
            .ready_starts
            .iter()
            .map(|(&chunk, data)| (chunk, data))
            .collect::<FxHashMap<_, _>>();
        starts.extend(computed.iter().map(|data| (data.chunk(), data)));
        let columns = self
            .tasks
            .into_par_iter()
            .filter_map(|task| {
                let chunk = task.chunk;
                run_column_task(program, &starts, task).map(|done| (chunk, done))
            })
            .collect();
        GenerationOutput {
            starts: computed,
            columns,
        }
    }
}

fn run_column_task(
    program: &NoiseFinalDensityProgram,
    starts: &FxHashMap<[i32; 2], &NoiseStructureChunkData>,
    task: ColumnTask,
) -> Option<Prefetched> {
    const ABSENT: &str = "the batch lacks a STRUCTURE_STARTS source";
    let source_at = |x: i32, z: i32| starts.get(&[x, z]).copied().ok_or(ABSENT);
    let target = *starts.get(&task.chunk)?;
    let blender = BlendingResolver::empty();
    let [x, z] = task.chunk;
    if matches!(task.kind, TaskKind::Fresh) {
        return if task.references_published {
            program
                .generate_fresh_carved_chunk_from_jigsaw_structure_references_borrowed(
                    target,
                    source_at,
                    Some(&blender),
                )
                .ok()
                .map(|generated| Prefetched::Fresh(generated, target.clone()))
        } else {
            program
                .generate_fresh_carved_chunk_from_complete_jigsaw_source_window_borrowed(
                    target,
                    source_at,
                    Some(&blender),
                )
                .ok()
                .map(|(generated, references)| Prefetched::Fresh(generated, references))
        };
    }
    let references = if task.references_published {
        target.clone()
    } else {
        NoiseStructureChunkData::references_from_complete_source_window_borrowed(target, source_at)
            .ok()?
    };
    // The serial owner publishes the target References before reading sources.
    let beardifier = program
        .beardifier_from_structure_references_borrowed(&references, |sx, sz| {
            if [sx, sz] == task.chunk {
                Ok(&references)
            } else {
                source_at(sx, sz)
            }
        })
        .ok()?;
    match task.kind {
        TaskKind::Fresh => unreachable!("handled above"),
        TaskKind::Resume(input) => program
            .resume_saved_chunk_to_carvers(
                crate::chunk::noise_generator::SavedChunkResume {
                    chunk: [x, z],
                    saved_status: input.status,
                    saved_column: &input.column,
                    saved_postprocessing: input.postprocessing.clone(),
                },
                &beardifier,
                &blender,
            )
            .ok()
            .map(|generated| Prefetched::Resumed { input, generated }),
        TaskKind::Biomes(input) => {
            let mut output = input.clone();
            program
                .fill_loading_biomes((x, z), &mut output, &beardifier, &blender)
                .ok()?;
            Some(Prefetched::Biomes { input, output })
        }
    }
}

impl ChunkStore {
    /// Computes a batch's parallel stages on the calling thread before the
    /// serial pipeline runs.
    pub(crate) fn pregenerate(
        &mut self,
        full: &[(i32, i32)],
        loading: &[((i32, i32), Java26ChunkStatus)],
    ) -> SaveResult<()> {
        if let Some(batch) = self.plan_generation(full, loading, &FxHashSet::default())? {
            let output = batch.run();
            self.install_generation(output);
        }
        Ok(())
    }

    /// Collects what the serial pipeline would generate for these targets,
    /// skipping `in_flight` columns another batch already computes. Saved
    /// columns the work reads are restored now, as the pipeline would.
    pub fn plan_generation(
        &mut self,
        full: &[(i32, i32)],
        loading: &[((i32, i32), Java26ChunkStatus)],
        in_flight: &FxHashSet<(i32, i32)>,
    ) -> SaveResult<Option<GenerationBatch>> {
        let ChunkSource::NoiseCarvers(program) = &self.source else {
            return Ok(None);
        };
        let program = Arc::clone(program);
        let mut carvers = BTreeSet::new();
        let mut biomes = BTreeSet::new();
        let mut starts_only = BTreeSet::new();
        let around = |center: (i32, i32), radius: i32, into: &mut BTreeSet<(i32, i32)>| {
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    if let (Some(x), Some(z)) = (center.0.checked_add(dx), center.1.checked_add(dz))
                    {
                        into.insert((x, z));
                    }
                }
            }
        };
        for &center in full {
            if !matches!(
                self.generation(center),
                Some(ChunkGenerationState::NoiseFull(_))
            ) && !self.is_storage_backed_noise_full(center.0, center.1)
            {
                // FEATURES of the radius-one LIGHT window write radius two.
                around(center, 2, &mut carvers);
            }
        }
        for &(chunk, status) in loading {
            if status >= Java26ChunkStatus::Features {
                around(chunk, 1, &mut carvers);
            } else if status >= Java26ChunkStatus::Carvers {
                carvers.insert(chunk);
            } else if status >= Java26ChunkStatus::Biomes {
                biomes.insert(chunk);
            } else {
                starts_only.insert(chunk);
            }
        }
        biomes.retain(|chunk| !carvers.contains(chunk));

        let mut tasks = Vec::new();
        for chunk in carvers {
            if let Some(kind) = self.carvers_task(chunk, in_flight) {
                tasks.push((chunk, kind));
            }
        }
        for chunk in biomes {
            if let Some(kind) = self.biomes_task(chunk, in_flight) {
                tasks.push((chunk, kind));
            }
        }
        let mut sources = BTreeSet::new();
        for &((x, z), _) in &tasks {
            sources.extend(structure_starts_window(x, z)?);
        }
        let windows = sources.clone();
        for chunk in starts_only {
            if self.prepare_generation_admission(chunk).is_ok() {
                sources.insert(chunk);
            }
        }
        let missing_starts = sources
            .iter()
            .map(|&(x, z)| [x, z])
            .filter(|&source| self.structure_status.starts_generatable(source))
            .collect::<Vec<_>>();
        if tasks.is_empty() && missing_starts.is_empty() {
            return Ok(None);
        }
        let rings = self.structure_status.rings(&program)?;
        let ready_starts = windows
            .iter()
            .filter_map(|&(x, z)| {
                self.structure_status
                    .ready_starts([x, z])
                    .map(|data| ([x, z], data.clone()))
            })
            .collect();
        let tasks: Vec<_> = tasks
            .into_iter()
            .map(|((x, z), kind)| ColumnTask {
                chunk: [x, z],
                references_published: self.structure_status.references_published([x, z]),
                kind,
            })
            .collect();
        Ok(Some(GenerationBatch {
            program,
            generate_structures: self.structure_status.generate_structures(),
            rings,
            ready_starts,
            missing_starts,
            tasks,
        }))
    }

    fn admitted(&mut self, chunk: (i32, i32), in_flight: &FxHashSet<(i32, i32)>) -> bool {
        !in_flight.contains(&chunk)
            && !self.structure_status.is_prefetched([chunk.0, chunk.1])
            && self.prepare_generation_admission(chunk).is_ok()
    }

    /// The start window is restored only for work the pipeline would do, and
    /// a column with an old-generation neighbour blends serially.
    fn window_admitted(&mut self, chunk: (i32, i32)) -> bool {
        self.prepare_structure_window_admission(chunk).is_ok()
            && self
                .blending_resolver(chunk)
                .is_ok_and(|blender| blender.is_empty())
    }

    fn carvers_task(
        &mut self,
        chunk: (i32, i32),
        in_flight: &FxHashSet<(i32, i32)>,
    ) -> Option<TaskKind> {
        if !self.admitted(chunk, in_flight) {
            return None;
        }
        let kind = match self.chunk(chunk) {
            None => TaskKind::Fresh,
            Some(loaded) => {
                let state = loaded.pre_carvers.as_ref()?;
                if loaded
                    .legacy_work
                    .as_ref()
                    .is_some_and(|work| work.below_zero_retrogen().is_some())
                {
                    return None;
                }
                TaskKind::Resume(ResumeInput {
                    status: pre_carvers_java_status(*state.status()),
                    column: loaded.column.clone(),
                    postprocessing: state.postprocessing_positions().to_vec(),
                })
            }
        };
        self.window_admitted(chunk).then_some(kind)
    }

    fn biomes_task(
        &mut self,
        chunk: (i32, i32),
        in_flight: &FxHashSet<(i32, i32)>,
    ) -> Option<TaskKind> {
        if !self.admitted(chunk, in_flight) {
            return None;
        }
        let column = match self.chunk(chunk) {
            None => self.empty_proto_column(),
            Some(loaded) => {
                let status = pre_carvers_java_status(*loaded.pre_carvers.as_ref()?.status());
                if status >= Java26ChunkStatus::Biomes {
                    return None;
                }
                loaded.column.clone()
            }
        };
        self.window_admitted(chunk)
            .then_some(TaskKind::Biomes(column))
    }

    /// Columns a noise FULL center wants at INITIALIZE_LIGHT before
    /// its own LIGHT, SPAWN and FULL step: its radius-one window, then the
    /// `wanted` columns at radius two whose terrain is ready. FEATURES there
    /// would otherwise demote and relight the lit center afterwards.
    pub(crate) fn collect_full_dependencies(
        &self,
        (x, z): (i32, i32),
        wanted: impl Fn((i32, i32)) -> bool,
    ) -> Vec<(i32, i32)> {
        if !matches!(self.source, ChunkSource::NoiseCarvers(_))
            || self.is_ticking_full(x, z)
            || self.is_storage_backed_noise_full(x, z)
        {
            return Vec::new();
        }
        let short = |(cx, cz): (i32, i32)| {
            self.generation_status(cx, cz)
                .is_none_or(|status| status < Java26ChunkStatus::InitializeLight)
        };
        let ring = |radius: i32| {
            (-radius..=radius)
                .flat_map(move |dx| {
                    (-radius..=radius).map(move |dz| Some((x.checked_add(dx)?, z.checked_add(dz)?)))
                })
                .flatten()
        };
        let mut deps = ring(1).filter(|&chunk| short(chunk)).collect::<Vec<_>>();
        deps.extend(ring(2).filter(|&(cx, cz)| {
            (cx.abs_diff(x) == 2 || cz.abs_diff(z) == 2)
                && short((cx, cz))
                && wanted((cx, cz))
                && (-1..=1).all(|dx| {
                    (-1..=1).all(|dz| {
                        let (nx, nz) = (cx + dx, cz + dz);
                        self.structure_status.has_prefetched_carvers([nx, nz])
                            || self
                                .generation_status(nx, nz)
                                .is_some_and(|status| status >= Java26ChunkStatus::Carvers)
                    })
                })
        }));
        deps
    }

    pub fn install_generation(&mut self, output: GenerationOutput) {
        self.structure_status
            .publish_batch(output.starts, output.columns);
    }

    /// Drops batch results the pipeline did not consume.
    pub fn clear_prefetched(&mut self) {
        self.structure_status.clear_prefetched();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::noise_generator::overworld_program;

    fn store() -> ChunkStore {
        ChunkStore::new_overworld_carvers_with_structure_generation(
            overworld_program(true, 0),
            true,
        )
        .unwrap()
    }

    fn area() -> Vec<(i32, i32)> {
        (-1..=1)
            .flat_map(|x| (-1..=1).map(move |z| (x, z)))
            .collect()
    }

    fn at(status: Java26ChunkStatus) -> Vec<((i32, i32), Java26ChunkStatus)> {
        area().into_iter().map(|chunk| (chunk, status)).collect()
    }

    fn columns(store: &mut ChunkStore) -> Vec<ChunkColumn> {
        area()
            .into_iter()
            .map(|(x, z)| store.try_load_column(x, z).unwrap().clone())
            .collect()
    }

    fn adopted_columns(store: &mut ChunkStore) -> Vec<ChunkColumn> {
        store
            .pregenerate(&[], &at(Java26ChunkStatus::Carvers))
            .unwrap();
        assert!(area()
            .iter()
            .all(|&(x, z)| store.structure_status.is_prefetched([x, z])));
        let columns = columns(store);
        assert!(!area()
            .iter()
            .any(|&(x, z)| store.structure_status.is_prefetched([x, z])));
        columns
    }

    #[test]
    fn batch_columns_equal_the_serial_owner_for_fresh_and_resumed_chunks() {
        let fresh = columns(&mut store());
        assert_eq!(adopted_columns(&mut store()), fresh);

        let mut serial = store();
        serial
            .ensure_loading_generation(&at(Java26ChunkStatus::StructureStarts))
            .unwrap();
        let mut interleaved = store();
        interleaved
            .pregenerate(&[], &at(Java26ChunkStatus::Carvers))
            .unwrap();
        interleaved
            .ensure_loading_generation(&at(Java26ChunkStatus::StructureStarts))
            .unwrap();
        let serial = columns(&mut serial);
        let interleaved = columns(&mut interleaved);
        for (adopted, expected) in interleaved.iter().zip(&serial) {
            assert_eq!(
                adopted.encode_section_data(),
                expected.encode_section_data()
            );
        }
        assert_eq!(interleaved, serial);

        let mut resumed = store();
        resumed
            .ensure_loading_generation(&at(Java26ChunkStatus::Biomes))
            .unwrap();
        let mut adopted = store();
        adopted
            .ensure_loading_generation(&at(Java26ChunkStatus::Biomes))
            .unwrap();
        assert_eq!(adopted_columns(&mut adopted), columns(&mut resumed));
        assert_eq!(columns(&mut resumed), fresh);
    }
}
