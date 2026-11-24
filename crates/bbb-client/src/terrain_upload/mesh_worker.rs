#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver as ResultReceiver, Sender as ResultSender};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[cfg(target_arch = "wasm32")]
use super::wasm_dispatch::{
    lock_free_channel, spawn_from_main_thread, LockFreeReceiver as ResultReceiver,
    LockFreeSender as ResultSender,
};

use bbb_client_render::terrain::{
    build_terrain_section_meshes_with_index_order, TerrainBlockRenderTable, TerrainChunkSnapshot,
    TerrainSectionMeshLayers, TerrainSectionVisibility,
};
use bbb_client_world::ChunkPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerrainMeshJobKey {
    pub(super) level_generation: u64,
    pub(super) mesh_generation: u64,
    pub(super) pos: ChunkPos,
    pub(super) mesh_revision: u64,
}

#[derive(Debug)]
pub(super) struct TerrainMeshJob {
    pub(super) key: TerrainMeshJobKey,
    pub(super) target: Arc<TerrainChunkSnapshot>,
    pub(super) neighborhood: Vec<Arc<TerrainChunkSnapshot>>,
    pub(super) table: Arc<TerrainBlockRenderTable>,
    pub(super) camera_position: [f32; 3],
    pub(super) sections: Vec<(i32, u64)>,
    pub(super) shared_quad_indices: bool,
}

#[derive(Debug)]
pub(super) struct TerrainMeshResult {
    pub(super) key: TerrainMeshJobKey,
    pub(super) camera_position: [f32; 3],
    pub(super) build_nanos: u64,
    pub(super) sections: Vec<(i32, u64)>,
    pub(super) layers: TerrainSectionMeshLayers,
}

#[derive(Debug)]
pub(super) struct TerrainMeshPool {
    #[cfg(not(target_arch = "wasm32"))]
    pool: rayon::ThreadPool,
    result_tx: ResultSender<TerrainMeshResult>,
    result_rx: ResultReceiver<TerrainMeshResult>,
    next_submit: u64,
    outstanding: usize,
    /// Jobs submitted before the latest cancellation skip their build.
    cancelled_before: Arc<AtomicU64>,
    capacity: usize,
}

impl TerrainMeshPool {
    fn spawn_task(&self, task: impl FnOnce() + Send + 'static) {
        #[cfg(not(target_arch = "wasm32"))]
        self.pool.spawn(task);
        #[cfg(target_arch = "wasm32")]
        spawn_from_main_thread(task);
    }

    pub(super) fn spawn() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (threads, pool, (result_tx, result_rx)) = {
            let threads = std::thread::available_parallelism()
                .map_or(1, |threads| threads.get().saturating_sub(1))
                .clamp(1, 8);
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .thread_name(|index| format!("bbb-terrain-mesh-{index}"))
                .build()
                .expect("failed to spawn the terrain mesh pool");
            (threads, pool, mpsc::channel())
        };
        #[cfg(target_arch = "wasm32")]
        let (threads, (result_tx, result_rx)) = (
            rayon::current_num_threads().clamp(1, 8),
            lock_free_channel(),
        );
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pool,
            result_tx,
            result_rx,
            next_submit: 0,
            outstanding: 0,
            cancelled_before: Arc::new(AtomicU64::new(0)),
            capacity: threads * 2,
        }
    }

    /// Submitted jobs whose results have not been taken yet.
    pub(super) fn outstanding(&self) -> usize {
        self.outstanding
    }

    pub(super) fn is_full(&self) -> bool {
        self.outstanding() >= self.capacity
    }

    pub(super) fn submit(&mut self, job: TerrainMeshJob) {
        let sequence = self.next_submit;
        self.next_submit += 1;
        self.outstanding += 1;
        let result_tx = self.result_tx.clone();
        let cancelled_before = Arc::clone(&self.cancelled_before);
        self.spawn_task(move || {
            let result = if sequence < cancelled_before.load(Ordering::Acquire) {
                cancelled_result(job)
            } else {
                build_terrain_mesh_job(job)
            };
            // The receiver lives as long as the pool; a send after shutdown is moot.
            let _ = result_tx.send(result);
        });
    }

    // Accept completed chunks without waiting for an earlier, slower build.
    pub(super) fn try_take(&mut self) -> Option<TerrainMeshResult> {
        let result = self.result_rx.try_recv().ok()?;
        self.outstanding -= 1;
        Some(result)
    }

    #[cfg(test)]
    pub(super) fn with_pending_result() -> (Self, impl FnOnce(TerrainMeshResult)) {
        let mut pool = Self::spawn();
        pool.next_submit = 1;
        pool.outstanding = 1;
        let sender = pool.result_tx.clone();
        (pool, move |result| sender.send(result).unwrap())
    }

    /// Jobs that have not started yet return empty results instead of meshing.
    pub(super) fn cancel_queued(&self) {
        self.cancelled_before
            .store(self.next_submit, Ordering::Release);
    }
}

fn cancelled_result(job: TerrainMeshJob) -> TerrainMeshResult {
    TerrainMeshResult {
        key: job.key,
        camera_position: job.camera_position,
        build_nanos: 0,
        sections: job.sections,
        layers: TerrainSectionMeshLayers::default(),
    }
}

pub(super) fn build_terrain_mesh_job(job: TerrainMeshJob) -> TerrainMeshResult {
    let started = bbb_client_clock::Instant::now();
    let section_ys = job
        .sections
        .iter()
        .map(|&(section_y, _)| section_y)
        .collect::<Vec<_>>();
    let mut layers = build_terrain_section_meshes_with_index_order(
        &job.target,
        job.neighborhood.iter().map(Arc::as_ref),
        &job.table,
        job.camera_position,
        &section_ys,
        job.shared_quad_indices,
    );
    // Compact snapshots omit empty height bands, but those completed sections
    // still carry their open visibility path through the full dimension.
    layers.visibility.extend(
        section_ys
            .into_iter()
            .filter(|section_y| {
                let min_y = section_y * 16;
                min_y < job.target.min_y || min_y >= job.target.min_y + job.target.height as i32
            })
            .map(|section_y| TerrainSectionVisibility::all_visible(section_y, false, true)),
    );
    TerrainMeshResult {
        key: job.key,
        camera_position: job.camera_position,
        build_nanos: started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
        sections: job.sections,
        layers,
    }
}
