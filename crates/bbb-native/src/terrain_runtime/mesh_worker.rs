use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use bbb_renderer::terrain::{
    build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera, TerrainChunkSnapshot,
    TerrainSectionMeshLayers, TerrainTextureAtlas,
};
use bbb_world::ChunkPos;

const TERRAIN_MESH_QUEUE_CAPACITY: usize = 4;
// The render thread drains completed results before doing up to one bounded
// cache-work batch. `try_recv` can observe Empty immediately before the worker
// publishes its current job, while the four-slot input queue is still full. A
// fast worker can then finish those pre-existing jobs plus every job submitted
// by the new batch before the next render-thread drain. Keep that exact bounded
// burst; `batch + 1` is insufficient because it omits the queued jobs.
const TERRAIN_MESH_RESULT_CAPACITY: usize =
    super::MAX_TERRAIN_CACHE_WORK_PER_FRAME + TERRAIN_MESH_QUEUE_CAPACITY + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerrainMeshJobKey {
    pub(super) level_generation: u64,
    pub(super) mesh_generation: u64,
    pub(super) pos: ChunkPos,
    pub(super) mesh_revision: u64,
    pub(super) section_count: usize,
}

#[derive(Debug)]
pub(super) struct TerrainMeshJob {
    pub(super) key: TerrainMeshJobKey,
    pub(super) priority: (u8, i64, i32, i32),
    pub(super) target: Arc<TerrainChunkSnapshot>,
    pub(super) neighborhood: Vec<Arc<TerrainChunkSnapshot>>,
    pub(super) atlas: Arc<TerrainTextureAtlas>,
    pub(super) camera_position: [f32; 3],
    pub(super) sections: Vec<(i32, u64)>,
}

#[derive(Debug)]
pub(super) struct TerrainMeshResult {
    pub(super) key: TerrainMeshJobKey,
    pub(super) camera_position: [f32; 3],
    pub(super) build_nanos: u64,
    pub(super) sections: Vec<(i32, u64)>,
    pub(super) layers: TerrainSectionMeshLayers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerrainMeshSubmit {
    Queued { evicted: Option<TerrainMeshJobKey> },
    AlreadyQueued,
    Full,
}

#[derive(Debug, Default)]
struct TerrainMeshJobQueue {
    jobs: Vec<TerrainMeshJob>,
    stopping: bool,
}

impl TerrainMeshJobQueue {
    fn submit(&mut self, job: TerrainMeshJob) -> TerrainMeshSubmit {
        if self.stopping {
            return TerrainMeshSubmit::Full;
        }
        if self.jobs.iter().any(|queued| queued.key == job.key) {
            return TerrainMeshSubmit::AlreadyQueued;
        }
        if let Some(index) = self
            .jobs
            .iter()
            .position(|queued| queued.key.pos == job.key.pos)
        {
            let evicted = self.jobs.swap_remove(index).key;
            self.jobs.push(job);
            return TerrainMeshSubmit::Queued {
                evicted: Some(evicted),
            };
        }
        if self.jobs.len() < TERRAIN_MESH_QUEUE_CAPACITY {
            self.jobs.push(job);
            return TerrainMeshSubmit::Queued { evicted: None };
        }

        let (worst_index, worst_priority) = self
            .jobs
            .iter()
            .enumerate()
            .max_by_key(|(_, queued)| queued.priority)
            .map(|(index, queued)| (index, queued.priority))
            .expect("a full terrain mesh queue has a worst job");
        if job.priority >= worst_priority {
            return TerrainMeshSubmit::Full;
        }
        let evicted = self.jobs.swap_remove(worst_index).key;
        self.jobs.push(job);
        TerrainMeshSubmit::Queued {
            evicted: Some(evicted),
        }
    }

    fn pop_next(&mut self) -> Option<TerrainMeshJob> {
        let next = self
            .jobs
            .iter()
            .enumerate()
            .min_by_key(|(_, job)| job.priority)
            .map(|(index, _)| index)?;
        Some(self.jobs.swap_remove(next))
    }
}

#[derive(Debug, Default)]
struct TerrainMeshWorkerShared {
    queue: Mutex<TerrainMeshJobQueue>,
    ready: Condvar,
}

#[derive(Debug)]
pub(super) struct TerrainMeshWorker {
    shared: Arc<TerrainMeshWorkerShared>,
    result_rx: Receiver<TerrainMeshResult>,
    handle: Option<JoinHandle<()>>,
}

impl TerrainMeshWorker {
    pub(super) fn spawn() -> Self {
        let shared = Arc::new(TerrainMeshWorkerShared::default());
        let (result_tx, result_rx) = mpsc::sync_channel(TERRAIN_MESH_RESULT_CAPACITY);
        let thread_shared = Arc::clone(&shared);
        let handle = thread::Builder::new()
            .name("bbb-terrain-mesh".to_owned())
            .spawn(move || run_worker(thread_shared, result_tx))
            .expect("failed to spawn bounded terrain mesh worker");
        Self {
            shared,
            result_rx,
            handle: Some(handle),
        }
    }

    pub(super) fn submit(&mut self, job: TerrainMeshJob) -> TerrainMeshSubmit {
        self.propagate_panic();
        let outcome = self
            .shared
            .queue
            .lock()
            .expect("terrain mesh queue lock poisoned")
            .submit(job);
        if matches!(outcome, TerrainMeshSubmit::Queued { .. }) {
            self.shared.ready.notify_one();
        }
        outcome
    }

    pub(super) fn try_recv(&mut self) -> Option<TerrainMeshResult> {
        self.propagate_panic();
        match self.result_rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.propagate_panic();
                panic!("terrain mesh worker disconnected without a panic payload")
            }
        }
    }

    pub(super) fn cancel_queued(&mut self) -> Vec<TerrainMeshJobKey> {
        self.propagate_panic();
        let mut queue = self
            .shared
            .queue
            .lock()
            .expect("terrain mesh queue lock poisoned");
        queue.jobs.drain(..).map(|job| job.key).collect()
    }

    pub(super) fn queue_depth(&self) -> usize {
        self.shared
            .queue
            .lock()
            .expect("terrain mesh queue lock poisoned")
            .jobs
            .len()
    }

    fn propagate_panic(&mut self) {
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        if !handle.is_finished() {
            return;
        }
        let handle = self.handle.take().expect("finished worker handle exists");
        match handle.join() {
            Ok(()) => panic!("terrain mesh worker exited before shutdown"),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    fn stop_and_join(&mut self) {
        if self.handle.is_none() {
            return;
        }
        {
            let mut queue = self
                .shared
                .queue
                .lock()
                .expect("terrain mesh queue lock poisoned during shutdown");
            queue.stopping = true;
            queue.jobs.clear();
        }
        self.shared.ready.notify_all();
        let handle = self
            .handle
            .take()
            .expect("terrain mesh worker handle exists");
        if let Err(payload) = handle.join() {
            if !thread::panicking() {
                std::panic::resume_unwind(payload);
            }
        }
    }
}

impl Drop for TerrainMeshWorker {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

fn run_worker(shared: Arc<TerrainMeshWorkerShared>, result_tx: SyncSender<TerrainMeshResult>) {
    loop {
        let job = {
            let mut queue = shared
                .queue
                .lock()
                .expect("terrain mesh queue lock poisoned");
            while queue.jobs.is_empty() && !queue.stopping {
                queue = shared
                    .ready
                    .wait(queue)
                    .expect("terrain mesh queue lock poisoned while waiting");
            }
            if queue.stopping {
                return;
            }
            queue
                .pop_next()
                .expect("a signaled terrain mesh queue has work")
        };
        let started = Instant::now();
        let section_ys = job
            .sections
            .iter()
            .map(|&(section_y, _)| section_y)
            .collect::<Vec<_>>();
        let layers = build_terrain_chunk_section_subset_mesh_layers_with_atlas_and_camera(
            &job.target,
            job.neighborhood.iter().map(Arc::as_ref),
            &job.atlas,
            job.camera_position,
            &section_ys,
        );
        let result = TerrainMeshResult {
            key: job.key,
            camera_position: job.camera_position,
            build_nanos: started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
            sections: job.sections,
            layers,
        };
        match result_tx.try_send(result) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                panic!("bounded terrain mesh result capacity invariant violated")
            }
            Err(TrySendError::Disconnected(_)) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_renderer::terrain::TerrainCell;

    fn job(pos: ChunkPos, revision: u64, priority: i64) -> TerrainMeshJob {
        TerrainMeshJob {
            key: TerrainMeshJobKey {
                level_generation: 1,
                mesh_generation: 2,
                pos,
                mesh_revision: revision,
                section_count: 1,
            },
            priority: (1, priority, pos.x, pos.z),
            target: Arc::new(TerrainChunkSnapshot::new(
                pos.x,
                pos.z,
                0,
                16,
                vec![TerrainCell::EMPTY; 16 * 16 * 16],
            )),
            neighborhood: Vec::new(),
            atlas: Arc::new(TerrainTextureAtlas::unit()),
            camera_position: [0.0, 0.0, 0.0],
            sections: vec![(0, revision)],
        }
    }

    #[test]
    fn bounded_queue_prioritizes_near_work_and_replaces_stale_owner_revision() {
        let mut queue = TerrainMeshJobQueue::default();
        for distance in 10..14 {
            assert_eq!(
                queue.submit(job(
                    ChunkPos {
                        x: distance as i32,
                        z: 0,
                    },
                    1,
                    distance,
                )),
                TerrainMeshSubmit::Queued { evicted: None }
            );
        }
        let far = ChunkPos { x: 13, z: 0 };
        let near = ChunkPos { x: 0, z: 0 };
        assert_eq!(
            queue.submit(job(near, 1, 0)),
            TerrainMeshSubmit::Queued {
                evicted: Some(TerrainMeshJobKey {
                    level_generation: 1,
                    mesh_generation: 2,
                    pos: far,
                    mesh_revision: 1,
                    section_count: 1,
                })
            }
        );
        assert_eq!(queue.pop_next().expect("near job").key.pos, near);

        let owner = ChunkPos { x: 10, z: 0 };
        assert_eq!(
            queue.submit(job(owner, 2, 10)),
            TerrainMeshSubmit::Queued {
                evicted: Some(TerrainMeshJobKey {
                    level_generation: 1,
                    mesh_generation: 2,
                    pos: owner,
                    mesh_revision: 1,
                    section_count: 1,
                })
            }
        );

        let mut player_queue = TerrainMeshJobQueue::default();
        for distance in 1..=4 {
            let pos = ChunkPos {
                x: distance as i32,
                z: 0,
            };
            assert_eq!(
                player_queue.submit(job(pos, 1, distance)),
                TerrainMeshSubmit::Queued { evicted: None }
            );
        }
        let mut player_changed = job(ChunkPos { x: 99, z: 0 }, 1, 99);
        player_changed.priority.0 = 0;
        assert!(matches!(
            player_queue.submit(player_changed),
            TerrainMeshSubmit::Queued { evicted: Some(_) }
        ));
        assert_eq!(
            player_queue.pop_next().expect("player-changed job").key.pos,
            ChunkPos { x: 99, z: 0 }
        );
    }

    #[test]
    fn worker_builds_from_immutable_values_and_joins_on_drop() {
        let mut worker = TerrainMeshWorker::spawn();
        let key = job(ChunkPos { x: 0, z: 0 }, 7, 0).key;
        assert_eq!(
            worker.submit(job(key.pos, key.mesh_revision, 0)),
            TerrainMeshSubmit::Queued { evicted: None }
        );
        let result = loop {
            if let Some(result) = worker.try_recv() {
                break result;
            }
            thread::yield_now();
        };
        assert_eq!(result.key, key);
        assert_eq!(result.sections, [(0, 7)]);
        assert_eq!(result.layers.source_sections, 1);
    }

    #[test]
    fn result_capacity_accepts_preexisting_queue_plus_one_fast_work_batch_without_drain() {
        let mut worker = TerrainMeshWorker::spawn();
        for index in 0..TERRAIN_MESH_RESULT_CAPACITY {
            let pos = ChunkPos {
                x: i32::try_from(index).unwrap(),
                z: 0,
            };
            let mut pending = Some(job(pos, 1, index as i64));
            loop {
                match worker.submit(pending.take().expect("pending job")) {
                    TerrainMeshSubmit::Queued { .. } => break,
                    TerrainMeshSubmit::Full => {
                        pending = Some(job(pos, 1, index as i64));
                        thread::yield_now();
                    }
                    TerrainMeshSubmit::AlreadyQueued => panic!("fresh position was queued"),
                }
            }
        }

        let mut completed = 0;
        while completed < TERRAIN_MESH_RESULT_CAPACITY {
            if worker.try_recv().is_some() {
                completed += 1;
            } else {
                thread::yield_now();
            }
        }
    }

    #[test]
    fn worker_source_cannot_read_live_world_or_gpu_owners() {
        let source = include_str!("mesh_worker.rs");
        for forbidden in [
            ["World", "Store"].concat(),
            ["wg", "pu"].concat(),
            ["&mut ", "Renderer"].concat(),
        ] {
            assert!(
                !source.contains(&forbidden),
                "terrain mesh worker restored forbidden owner {forbidden}"
            );
        }
    }
}
