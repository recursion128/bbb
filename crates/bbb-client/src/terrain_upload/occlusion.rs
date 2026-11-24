//! Vanilla-compatible chunk-section visibility propagation and octree traversal.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{BuildHasher, Hasher},
    sync::Arc,
};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError, TrySendError},
    thread::{self, JoinHandle},
};

#[cfg(target_arch = "wasm32")]
use super::wasm_dispatch::{
    lock_free_channel, spawn_from_main_thread, LockFreeReceiver, LockFreeSender,
};

use bbb_client_render::{
    terrain::{TerrainFace, TerrainSectionVisibility},
    AabbFrustumIntersection, CameraFrustum,
};
use bbb_client_world::ChunkPos;

use super::TerrainOcclusionRebuildKey;

mod debug;
pub use debug::ChunkSectionDebugGizmos;

const MINIMUM_ADVANCED_CULLING_SECTION_DISTANCE: i32 = 60_i32.div_euclid(16);
const CEILED_SECTION_DIAGONAL: f64 = 28.0;

/// Vanilla's occlusion graph uses packed-long fastutil maps/sets. These keys
/// are renderer-owned coordinates rather than attacker-controlled strings, so
/// a compact deterministic hasher avoids paying SipHash for every graph edge
/// while retaining the same collision-safe map semantics.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TerrainCoordinateBuildHasher;

#[derive(Debug, Clone, Copy)]
pub(super) struct TerrainCoordinateHasher(u64);

impl BuildHasher for TerrainCoordinateBuildHasher {
    type Hasher = TerrainCoordinateHasher;

    fn build_hasher(&self) -> Self::Hasher {
        TerrainCoordinateHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for TerrainCoordinateHasher {
    fn finish(&self) -> u64 {
        let mut hash = self.0;
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        hash ^ (hash >> 33)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 ^= value;
        self.0 = self.0.wrapping_mul(0x1000_0000_01b3);
    }

    fn write_i32(&mut self, value: i32) {
        self.write_u64(u64::from(value as u32));
    }
}

pub(super) type TerrainCoordinateMap<K, V> = HashMap<K, V, TerrainCoordinateBuildHasher>;
pub(super) type TerrainCoordinateSet<K> = HashSet<K, TerrainCoordinateBuildHasher>;

#[derive(Debug)]
pub(super) struct TerrainOcclusionJob {
    pub(super) key: TerrainOcclusionRebuildKey,
    pub(super) sections: Arc<Vec<ResidentSection>>,
    pub(super) loaded_chunks: Arc<TerrainCoordinateSet<ChunkPos>>,
    pub(super) camera_position: [f64; 3],
    pub(super) view_distance: i32,
    pub(super) min_section_y: i32,
    pub(super) section_count: usize,
    pub(super) smart_cull: bool,
}

#[derive(Debug)]
pub(super) struct TerrainOcclusionResult {
    pub(super) key: TerrainOcclusionRebuildKey,
    pub(super) graph: TerrainSectionOcclusionGraph,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(super) struct TerrainOcclusionWorker {
    job_tx: Option<SyncSender<TerrainOcclusionJob>>,
    result_rx: Receiver<TerrainOcclusionResult>,
    handle: Option<JoinHandle<()>>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub(super) struct TerrainOcclusionWorker {
    result_tx: LockFreeSender<TerrainOcclusionResult>,
    result_rx: LockFreeReceiver<TerrainOcclusionResult>,
    in_flight: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl TerrainOcclusionWorker {
    pub(super) fn spawn() -> Self {
        let (job_tx, job_rx) = mpsc::sync_channel(1);
        let (result_tx, result_rx) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("bbb-terrain-occlusion".to_owned())
            .spawn(move || run_occlusion_worker(job_rx, result_tx))
            .expect("failed to spawn terrain occlusion worker");
        Self {
            job_tx: Some(job_tx),
            result_rx,
            handle: Some(handle),
        }
    }

    pub(super) fn submit(&mut self, job: TerrainOcclusionJob) {
        self.propagate_panic();
        match self
            .job_tx
            .as_ref()
            .expect("live terrain occlusion worker has a sender")
            .try_send(job)
        {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                panic!("terrain occlusion worker accepted more than one pending rebuild")
            }
            Err(TrySendError::Disconnected(_)) => {
                self.propagate_panic();
                panic!("terrain occlusion worker disconnected without a panic payload")
            }
        }
    }

    pub(super) fn try_recv(&mut self) -> Option<TerrainOcclusionResult> {
        self.propagate_panic();
        match self.result_rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.propagate_panic();
                panic!("terrain occlusion worker disconnected without a panic payload")
            }
        }
    }

    fn propagate_panic(&mut self) {
        let Some(handle) = self.handle.as_ref() else {
            return;
        };
        if !handle.is_finished() {
            return;
        }
        let handle = self
            .handle
            .take()
            .expect("finished occlusion worker handle exists");
        match handle.join() {
            Ok(()) => panic!("terrain occlusion worker exited before shutdown"),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for TerrainOcclusionWorker {
    fn drop(&mut self) {
        self.job_tx.take();
        let Some(handle) = self.handle.take() else {
            return;
        };
        if let Err(payload) = handle.join() {
            if !thread::panicking() {
                std::panic::resume_unwind(payload);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl TerrainOcclusionWorker {
    pub(super) fn spawn() -> Self {
        let (result_tx, result_rx) = lock_free_channel();
        Self {
            result_tx,
            result_rx,
            in_flight: false,
        }
    }

    pub(super) fn submit(&mut self, job: TerrainOcclusionJob) {
        assert!(
            !self.in_flight,
            "terrain occlusion worker accepted more than one pending rebuild"
        );
        self.in_flight = true;
        let result_tx = self.result_tx.clone();
        spawn_from_main_thread(move || {
            let mut graph = TerrainSectionOcclusionGraph::default();
            graph.rebuild(
                job.sections.iter().copied(),
                job.loaded_chunks.as_ref(),
                job.camera_position,
                job.view_distance,
                job.min_section_y,
                job.section_count,
                job.smart_cull,
            );
            let _ = result_tx.send(TerrainOcclusionResult {
                key: job.key,
                graph,
            });
        });
    }

    pub(super) fn try_recv(&mut self) -> Option<TerrainOcclusionResult> {
        match self.result_rx.try_recv() {
            Ok(result) => {
                self.in_flight = false;
                Some(result)
            }
            Err(bbb_render_types::wasm_sync::TryRecvEmpty) => None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_occlusion_worker(
    job_rx: Receiver<TerrainOcclusionJob>,
    result_tx: Sender<TerrainOcclusionResult>,
) {
    while let Ok(job) = job_rx.recv() {
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(
            job.sections.iter().copied(),
            job.loaded_chunks.as_ref(),
            job.camera_position,
            job.view_distance,
            job.min_section_y,
            job.section_count,
            job.smart_cull,
        );
        if result_tx
            .send(TerrainOcclusionResult {
                key: job.key,
                graph,
            })
            .is_err()
        {
            return;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct SectionPosition {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) z: i32,
}

impl SectionPosition {
    pub(super) fn from_block_position(position: [f64; 3]) -> Option<Self> {
        position
            .iter()
            .all(|coordinate| coordinate.is_finite())
            .then(|| Self {
                x: (position[0].floor() as i32).div_euclid(16),
                y: (position[1].floor() as i32).div_euclid(16),
                z: (position[2].floor() as i32).div_euclid(16),
            })
    }

    pub(super) fn origin(self) -> [i32; 3] {
        [
            self.x.saturating_mul(16),
            self.y.saturating_mul(16),
            self.z.saturating_mul(16),
        ]
    }

    fn center(self) -> [i32; 3] {
        self.origin().map(|coordinate| coordinate.saturating_add(8))
    }

    fn relative(self, direction: TerrainFace) -> Self {
        let [x, y, z] = direction_step(direction);
        Self {
            x: self.x.saturating_add(x),
            y: self.y.saturating_add(y),
            z: self.z.saturating_add(z),
        }
    }

    pub(super) fn chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x,
            z: self.z,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResidentSection {
    pub(super) position: SectionPosition,
    pub(super) visibility: TerrainSectionVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OcclusionNode {
    pub(super) position: SectionPosition,
    pub(super) source_directions: u8,
    directions: u8,
    pub(super) step: u32,
}

impl OcclusionNode {
    fn new(position: SectionPosition, source: Option<TerrainFace>, step: u32) -> Self {
        let mut node = Self {
            position,
            source_directions: 0,
            directions: 0,
            step,
        };
        if let Some(source) = source {
            node.add_source_direction(source);
        }
        node
    }

    pub(super) fn has_source_direction(self, direction: TerrainFace) -> bool {
        self.source_directions & direction_bit(direction) != 0
    }

    fn has_source_directions(self) -> bool {
        self.source_directions != 0
    }

    fn add_source_direction(&mut self, direction: TerrainFace) {
        self.source_directions |= direction_bit(direction);
    }

    fn has_direction(self, direction: TerrainFace) -> bool {
        self.directions & direction_bit(direction) != 0
    }

    fn inherit_direction(&mut self, old_directions: u8, direction: TerrainFace) {
        self.directions |= old_directions | direction_bit(direction);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OctreeVisit {
    pub(super) min: [i32; 3],
    pub(super) max_exclusive: [i32; 3],
    pub(super) section: Option<SectionPosition>,
    pub(super) depth: u32,
    pub(super) fully_visible: bool,
    pub(super) is_close: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VisibleSection {
    pub(super) position: SectionPosition,
    pub(super) nearby: bool,
}

impl From<SectionPosition> for VisibleSection {
    fn from(position: SectionPosition) -> Self {
        Self {
            position,
            nearby: false,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct TerrainSectionOcclusionGraph {
    nodes: TerrainCoordinateMap<SectionPosition, OcclusionNode>,
    section_visibility: TerrainCoordinateMap<SectionPosition, TerrainSectionVisibility>,
    waiting_sections: TerrainCoordinateSet<SectionPosition>,
    // One word per column and 64 section heights, merged only when replacing the graph.
    visited_empty: TerrainCoordinateMap<SectionPosition, u64>,
    // Distant rays can depend on a section outside the local update neighborhood.
    ray_waiters: TerrainCoordinateMap<SectionPosition, Vec<SectionPosition>>,
    #[cfg(test)]
    reachable_sections: TerrainCoordinateSet<SectionPosition>,
    octree: Option<SectionOctree>,
    valid: bool,
}

impl TerrainSectionOcclusionGraph {
    fn empty_bit(position: SectionPosition) -> (SectionPosition, u64) {
        (
            SectionPosition {
                y: position.y.div_euclid(64),
                ..position
            },
            1 << position.y.rem_euclid(64),
        )
    }

    pub(super) fn visited_empty(&self, position: SectionPosition) -> bool {
        let (word, bit) = Self::empty_bit(position);
        self.visited_empty
            .get(&word)
            .is_some_and(|mask| mask & bit != 0)
    }

    fn record_empty_visit(&mut self, position: SectionPosition) {
        let (word, bit) = Self::empty_bit(position);
        *self.visited_empty.entry(word).or_default() |= bit;
    }

    pub(super) fn forget_empty(&mut self, position: SectionPosition) {
        let (word, bit) = Self::empty_bit(position);
        if let Some(mask) = self.visited_empty.get_mut(&word) {
            *mask &= !bit;
            if *mask == 0 {
                self.visited_empty.remove(&word);
            }
        }
    }

    pub(super) fn replace_preserving_visits(&mut self, mut next: Self) {
        if self.visited_empty.len() > next.visited_empty.len() {
            std::mem::swap(&mut self.visited_empty, &mut next.visited_empty);
        }
        for (word, bits) in self.visited_empty.drain() {
            *next.visited_empty.entry(word).or_default() |= bits;
        }
        *self = next;
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn rebuild(
        &mut self,
        sections: impl IntoIterator<Item = ResidentSection>,
        loaded_chunks: &TerrainCoordinateSet<ChunkPos>,
        camera_position: [f64; 3],
        view_distance: i32,
        min_section_y: i32,
        section_count: usize,
        smart_cull: bool,
    ) {
        self.nodes.clear();
        self.section_visibility.clear();
        self.waiting_sections.clear();
        self.ray_waiters.clear();
        #[cfg(test)]
        self.reachable_sections.clear();
        self.octree = None;
        self.valid = false;

        self.section_visibility.extend(
            sections
                .into_iter()
                .map(|section| (section.position, section.visibility)),
        );
        let sections = &self.section_visibility;
        let Some(camera_section) = SectionPosition::from_block_position(camera_position) else {
            return;
        };
        if sections.is_empty() || section_count == 0 {
            return;
        }
        let view_distance = view_distance.max(0);
        let max_section_y = min_section_y
            .saturating_add(i32::try_from(section_count).unwrap_or(i32::MAX))
            .saturating_sub(1);
        let mut queue = VecDeque::new();
        if sections.contains_key(&camera_section) {
            self.nodes
                .insert(camera_section, OcclusionNode::new(camera_section, None, 0));
            queue.push_back(camera_section);
        } else if camera_section.y < min_section_y || camera_section.y > max_section_y {
            let below_world = camera_section.y < min_section_y;
            let section_y = if below_world {
                min_section_y
            } else {
                max_section_y
            };
            let source = if below_world {
                TerrainFace::Up
            } else {
                TerrainFace::Down
            };
            let camera_center = camera_section.center();
            let mut initial = Vec::new();
            for dx in -view_distance..=view_distance {
                for dz in -view_distance..=view_distance {
                    let position = SectionPosition {
                        x: camera_section.x.saturating_add(dx),
                        y: section_y,
                        z: camera_section.z.saturating_add(dz),
                    };
                    if !sections.contains_key(&position)
                        || !is_in_view_distance(camera_section, position, view_distance)
                    {
                        continue;
                    }
                    let mut node = OcclusionNode::new(position, Some(source), 0);
                    node.inherit_direction(0, source);
                    if dx > 0 {
                        node.inherit_direction(node.directions, TerrainFace::East);
                    } else if dx < 0 {
                        node.inherit_direction(node.directions, TerrainFace::West);
                    }
                    if dz > 0 {
                        node.inherit_direction(node.directions, TerrainFace::South);
                    } else if dz < 0 {
                        node.inherit_direction(node.directions, TerrainFace::North);
                    }
                    let center = position.center();
                    let distance_squared = center
                        .into_iter()
                        .zip(camera_center)
                        .map(|(value, camera)| i64::from(value - camera).pow(2))
                        .sum::<i64>();
                    initial.push((distance_squared, position, node));
                }
            }
            initial.sort_unstable_by_key(|(distance, position, _)| (*distance, *position));
            for (_, position, node) in initial {
                self.nodes.insert(position, node);
                queue.push_back(position);
            }
        } else {
            // The immutable resident set is incomplete around the camera.
            // Keep the renderer's conservative all-visible fallback.
            return;
        }

        self.octree = Some(SectionOctree::new(
            camera_section,
            view_distance,
            section_count,
            min_section_y.saturating_mul(16),
        ));
        self.propagate(
            queue,
            loaded_chunks,
            camera_position,
            view_distance,
            min_section_y,
            section_count,
            smart_cull,
        );
        self.valid = true;
    }

    /// Mirrors Vanilla's `schedulePropagationFrom` + `runPartialUpdate`: a newly
    /// compiled section updates the current mesh visibility in place, then the
    /// existing graph nodes continue propagating without rebuilding the graph.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_sections(
        &mut self,
        updates: impl IntoIterator<Item = (SectionPosition, Option<TerrainSectionVisibility>)>,
        loaded_chunks: &TerrainCoordinateSet<ChunkPos>,
        camera_position: [f64; 3],
        view_distance: i32,
        min_section_y: i32,
        section_count: usize,
        smart_cull: bool,
    ) {
        if !self.valid {
            return;
        }

        let mut changed = Vec::new();
        let mut scheduled = Vec::new();
        for (position, visibility) in updates {
            let previous = match visibility {
                Some(visibility) => self.section_visibility.insert(position, visibility),
                None => {
                    self.forget_empty(position);
                    self.waiting_sections.remove(&position);
                    self.section_visibility.remove(&position)
                }
            };
            if previous != visibility {
                changed.push(position);
            } else if visibility.is_some_and(|section| section.loaded_empty)
                && self.nodes.contains_key(&position)
                && !self.waiting_sections.contains(&position)
                && !self.visited_empty(position)
            {
                // A coalesced reinstall needs a fresh visit even with the same empty input.
                scheduled.push(position);
            }
        }
        changed.sort_unstable();
        changed.dedup();

        scheduled.reserve(changed.len().saturating_mul(7));
        for position in changed {
            if self.nodes.contains_key(&position) {
                scheduled.push(position);
            }
            for direction in TerrainFace::ALL {
                let neighbor = position.relative(direction);
                if self.nodes.contains_key(&neighbor) {
                    scheduled.push(neighbor);
                }
            }
        }
        // Only the propagation frontier waits for the eight horizontal neighbors.
        scheduled.extend(
            self.waiting_sections
                .iter()
                .copied()
                .filter(|position| has_all_neighbors(position.chunk(), loaded_chunks)),
        );
        scheduled.sort_unstable();
        scheduled.dedup();
        let queue = scheduled.into_iter().collect();
        self.propagate(
            queue,
            loaded_chunks,
            camera_position,
            view_distance,
            min_section_y,
            section_count,
            smart_cull,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn propagate(
        &mut self,
        mut queue: VecDeque<SectionPosition>,
        loaded_chunks: &TerrainCoordinateSet<ChunkPos>,
        camera_position: [f64; 3],
        view_distance: i32,
        min_section_y: i32,
        section_count: usize,
        smart_cull: bool,
    ) {
        let Some(camera_section) = SectionPosition::from_block_position(camera_position) else {
            return;
        };
        let view_distance = view_distance.max(0);
        let max_section_y = min_section_y
            .saturating_add(i32::try_from(section_count).unwrap_or(i32::MAX))
            .saturating_sub(1);
        while let Some(position) = queue.pop_front() {
            let node = self.nodes[&position];
            self.waiting_sections.remove(&position);
            let Some(section_visibility) = self.section_visibility.get(&node.position).copied()
            else {
                continue;
            };
            #[cfg(test)]
            self.reachable_sections.insert(node.position);
            if !section_visibility.loaded_empty {
                if let Some(octree) = self.octree.as_mut() {
                    octree.add(node.position);
                }
            } else {
                self.record_empty_visit(position);
            }
            let distant_from_camera = (node.position.x - camera_section.x).abs()
                > MINIMUM_ADVANCED_CULLING_SECTION_DISTANCE
                || (node.position.y - camera_section.y).abs()
                    > MINIMUM_ADVANCED_CULLING_SECTION_DISTANCE
                || (node.position.z - camera_section.z).abs()
                    > MINIMUM_ADVANCED_CULLING_SECTION_DISTANCE;
            for direction in TerrainFace::ALL {
                let relative = node.position.relative(direction);
                if !self.section_visibility.contains_key(&relative)
                    || !is_in_view_distance(camera_section, relative, view_distance)
                    || (relative.y - camera_section.y).abs() > view_distance
                    || (smart_cull && node.has_direction(direction.opposite()))
                {
                    continue;
                }
                if smart_cull && node.has_source_directions() {
                    let visible = TerrainFace::ALL.into_iter().any(|source| {
                        node.has_source_direction(source)
                            && section_visibility
                                .faces_can_see_each_other(source.opposite(), direction)
                    });
                    if !visible {
                        continue;
                    }
                }
                if smart_cull && distant_from_camera {
                    if self
                        .nodes
                        .get(&relative)
                        .is_some_and(|next| next.has_source_direction(direction))
                    {
                        continue;
                    }
                    if let Some(missing) = first_missing_ray_section(
                        node.position,
                        direction,
                        camera_section,
                        camera_position,
                        min_section_y,
                        max_section_y,
                        &self.nodes,
                    ) {
                        let waiters = self.ray_waiters.entry(missing).or_default();
                        if !waiters.contains(&position) {
                            waiters.push(position);
                        }
                        continue;
                    }
                }
                if let Some(existing) = self.nodes.get_mut(&relative) {
                    if !existing.has_source_direction(direction) {
                        existing.add_source_direction(direction);
                        // A new entrance can expose a previously blocked exit.
                        if smart_cull {
                            if has_all_neighbors(relative.chunk(), loaded_chunks) {
                                queue.push_back(relative);
                            } else {
                                self.waiting_sections.insert(relative);
                            }
                        }
                    }
                    continue;
                }
                let mut next =
                    OcclusionNode::new(relative, Some(direction), node.step.saturating_add(1));
                next.inherit_direction(node.directions, direction);
                self.nodes.insert(relative, next);
                if let Some(waiters) = self.ray_waiters.remove(&relative) {
                    queue.extend(waiters);
                }
                if has_all_neighbors(relative.chunk(), loaded_chunks) {
                    queue.push_back(relative);
                } else {
                    self.waiting_sections.insert(relative);
                }
            }
        }
    }

    pub(super) fn is_valid(&self) -> bool {
        self.valid
    }

    pub(super) fn is_rooted_at(&self, eye: [f32; 3]) -> bool {
        self.octree.as_ref().is_some_and(|tree| {
            SectionPosition::from_block_position(eye.map(f64::from))
                .is_some_and(|section| section.center() == tree.camera_section_center)
        })
    }

    #[cfg(test)]
    pub(super) fn reachable_sections(&self) -> &TerrainCoordinateSet<SectionPosition> {
        &self.reachable_sections
    }

    #[cfg(test)]
    pub(super) fn node(&self, position: SectionPosition) -> Option<OcclusionNode> {
        self.nodes.get(&position).copied()
    }

    pub(super) fn octree_visits(
        &self,
        frustum: CameraFrustum,
        close_distance: i32,
    ) -> Vec<OctreeVisit> {
        let mut visits = Vec::new();
        if let Some(octree) = &self.octree {
            octree.visit(frustum, close_distance, |visit| visits.push(visit));
        }
        visits
    }

    pub(super) fn visible_sections(&self, frustum: CameraFrustum) -> Vec<VisibleSection> {
        let mut visible = Vec::new();
        self.fill_visible_sections(frustum, &mut visible);
        visible
    }

    pub(super) fn fill_visible_sections(
        &self,
        frustum: CameraFrustum,
        visible: &mut Vec<VisibleSection>,
    ) {
        visible.clear();
        if let Some(octree) = &self.octree {
            // Vanilla `SectionOcclusionGraph.addSectionsInFrustum` consumes
            // leaves directly from one octree visitor. Production needs only
            // the section and its nearby flag; branch bounds/depth snapshots
            // remain exclusive to `octree_visits` and the debug renderer.
            octree.visit(frustum, 32, |visit| {
                if let Some(position) = visit.section {
                    visible.push(VisibleSection {
                        position,
                        nearby: visit.is_close,
                    });
                }
            });
        }
    }
}

fn is_in_view_distance(
    camera: SectionPosition,
    section: SectionPosition,
    view_distance: i32,
) -> bool {
    let delta_x = (section.x - camera.x).abs().saturating_sub(1).max(0);
    let delta_z = (section.z - camera.z).abs().saturating_sub(1).max(0);
    i64::from(delta_x).pow(2) + i64::from(delta_z).pow(2) < i64::from(view_distance).pow(2)
}

fn has_all_neighbors(chunk: ChunkPos, loaded_chunks: &TerrainCoordinateSet<ChunkPos>) -> bool {
    (-1..=1).all(|dz| {
        (-1..=1).all(|dx| {
            (dx == 0 && dz == 0)
                || loaded_chunks.contains(&ChunkPos {
                    x: chunk.x.saturating_add(dx),
                    z: chunk.z.saturating_add(dz),
                })
        })
    })
}

#[allow(clippy::too_many_arguments)]
fn first_missing_ray_section(
    section: SectionPosition,
    direction: TerrainFace,
    camera_section: SectionPosition,
    camera_position: [f64; 3],
    min_section_y: i32,
    max_section_y: i32,
    nodes: &TerrainCoordinateMap<SectionPosition, OcclusionNode>,
) -> Option<SectionPosition> {
    let origin = section.origin();
    let camera_center = camera_section.center();
    let axis = face_axis(direction);
    let max_x = if axis == 0 {
        camera_center[0] > origin[0]
    } else {
        camera_center[0] < origin[0]
    };
    let max_y = if axis == 1 {
        camera_center[1] > origin[1]
    } else {
        camera_center[1] < origin[1]
    };
    let max_z = if axis == 2 {
        camera_center[2] > origin[2]
    } else {
        camera_center[2] < origin[2]
    };
    let mut check = [
        f64::from(origin[0] + if max_x { 16 } else { 0 }),
        f64::from(origin[1] + if max_y { 16 } else { 0 }),
        f64::from(origin[2] + if max_z { 16 } else { 0 }),
    ];
    let delta = [
        camera_position[0] - check[0],
        camera_position[1] - check[1],
        camera_position[2] - check[2],
    ];
    let length = delta
        .into_iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !length.is_finite() || length <= f64::EPSILON {
        return None;
    }
    let step = delta.map(|value| value / length * CEILED_SECTION_DIAGONAL);
    while squared_distance(check, camera_position) > 3600.0 {
        for axis in 0..3 {
            check[axis] += step[axis];
        }
        let check_section = SectionPosition::from_block_position(check)?;
        if check_section.y > max_section_y || check_section.y < min_section_y {
            break;
        }
        if !nodes.contains_key(&check_section) {
            return Some(check_section);
        }
    }
    None
}

fn squared_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum()
}

fn direction_bit(direction: TerrainFace) -> u8 {
    1_u8 << direction.index()
}

fn direction_step(direction: TerrainFace) -> [i32; 3] {
    match direction {
        TerrainFace::Down => [0, -1, 0],
        TerrainFace::Up => [0, 1, 0],
        TerrainFace::North => [0, 0, -1],
        TerrainFace::South => [0, 0, 1],
        TerrainFace::West => [-1, 0, 0],
        TerrainFace::East => [1, 0, 0],
    }
}

fn face_axis(direction: TerrainFace) -> usize {
    match direction {
        TerrainFace::West | TerrainFace::East => 0,
        TerrainFace::Down | TerrainFace::Up => 1,
        TerrainFace::North | TerrainFace::South => 2,
    }
}

#[derive(Debug)]
struct SectionOctree {
    branches: Vec<OctreeBranch>,
    camera_section_center: [i32; 3],
}

impl SectionOctree {
    fn new(
        camera_section: SectionPosition,
        render_distance: i32,
        sections_per_chunk: usize,
        min_block_y: i32,
    ) -> Self {
        let diameter = render_distance.saturating_mul(2).saturating_add(1).max(1);
        let size_sections = u32::try_from(diameter)
            .ok()
            .and_then(u32::checked_next_power_of_two)
            .and_then(|size| i32::try_from(size).ok())
            .unwrap_or(i32::MAX / 16);
        let distance_blocks = render_distance.saturating_mul(16);
        let origin = camera_section.origin();
        let min_x = origin[0].saturating_sub(distance_blocks);
        let min_y = if usize::try_from(size_sections)
            .ok()
            .is_some_and(|size| size >= sections_per_chunk)
        {
            min_block_y
        } else {
            origin[1].saturating_sub(distance_blocks)
        };
        let min_z = origin[2].saturating_sub(distance_blocks);
        let size_blocks = size_sections.saturating_mul(16);
        let bounds = IntBounds {
            min: [min_x, min_y, min_z],
            max: [
                min_x.saturating_add(size_blocks).saturating_sub(1),
                min_y.saturating_add(size_blocks).saturating_sub(1),
                min_z.saturating_add(size_blocks).saturating_sub(1),
            ],
        };
        let camera_section_center = camera_section.center();
        Self {
            branches: vec![OctreeBranch::new(bounds, camera_section_center)],
            camera_section_center,
        }
    }

    fn add(&mut self, section: SectionPosition) {
        let origin = section.origin();
        let mut current = 0;
        loop {
            let branch = &self.branches[current];
            let negative = std::array::from_fn(|axis| origin[axis] < branch.center[axis]);
            let opposite =
                std::array::from_fn(|axis| negative[axis] != branch.camera_negative[axis]);
            let index = branch.sorting.node_index(opposite);
            if branch.bounds.span_x() <= 32 {
                self.branches[current].children[index] = Some(OctreeNode::Leaf(section));
                return;
            }
            current = match branch.children[index] {
                Some(OctreeNode::Branch(next)) => next,
                Some(OctreeNode::Leaf(_)) => unreachable!("leaves belong to terminal branches"),
                None => {
                    let child = OctreeBranch::new(
                        branch.bounds.child(negative),
                        self.camera_section_center,
                    );
                    let next = self.branches.len();
                    self.branches.push(child);
                    self.branches[current].children[index] = Some(OctreeNode::Branch(next));
                    next
                }
            };
        }
    }

    fn visit(
        &self,
        frustum: CameraFrustum,
        close_distance: i32,
        mut visitor: impl FnMut(OctreeVisit),
    ) {
        self.branches[0].visit(
            &self.branches,
            self.camera_section_center,
            frustum,
            close_distance,
            false,
            0,
            true,
            &mut visitor,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntBounds {
    min: [i32; 3],
    max: [i32; 3],
}

impl IntBounds {
    fn center(self) -> [i32; 3] {
        std::array::from_fn(|axis| {
            self.min[axis].saturating_add(
                self.max[axis]
                    .saturating_sub(self.min[axis])
                    .saturating_add(1)
                    / 2,
            )
        })
    }

    fn span_x(self) -> i32 {
        self.max[0].saturating_sub(self.min[0]).saturating_add(1)
    }

    fn max_exclusive(self) -> [i32; 3] {
        self.max.map(|coordinate| coordinate.saturating_add(1))
    }

    fn as_f64(self) -> ([f64; 3], [f64; 3]) {
        (self.min.map(f64::from), self.max_exclusive().map(f64::from))
    }

    fn child(self, negative: [bool; 3]) -> Self {
        let center = self.center();
        let min = std::array::from_fn(|axis| {
            if negative[axis] {
                self.min[axis]
            } else {
                center[axis]
            }
        });
        let max = std::array::from_fn(|axis| {
            if negative[axis] {
                center[axis].saturating_sub(1)
            } else {
                self.max[axis]
            }
        });
        Self { min, max }
    }
}

#[derive(Debug)]
struct OctreeBranch {
    children: [Option<OctreeNode>; 8],
    bounds: IntBounds,
    center: [i32; 3],
    sorting: AxisSorting,
    camera_negative: [bool; 3],
}

impl OctreeBranch {
    fn new(bounds: IntBounds, camera_center: [i32; 3]) -> Self {
        let center = bounds.center();
        let camera_diff = std::array::from_fn(|axis| camera_center[axis] - center[axis]);
        Self {
            children: [None; 8],
            bounds,
            center,
            sorting: AxisSorting::for_differences(camera_diff.map(i32::abs)),
            camera_negative: camera_diff.map(|difference| difference < 0),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn visit(
        &self,
        branches: &[OctreeBranch],
        camera_center: [i32; 3],
        frustum: CameraFrustum,
        close_distance: i32,
        mut skip_frustum_check: bool,
        depth: u32,
        mut is_close: bool,
        visitor: &mut impl FnMut(OctreeVisit),
    ) {
        let fully_visible = if skip_frustum_check {
            true
        } else {
            let (min, max) = self.bounds.as_f64();
            match frustum.classify_aabb(min, max) {
                AabbFrustumIntersection::Outside => return,
                AabbFrustumIntersection::Intersecting => false,
                AabbFrustumIntersection::Inside => {
                    skip_frustum_check = true;
                    true
                }
            }
        };
        is_close &= inclusive_bounds_are_close(camera_center, self.bounds, close_distance);
        visitor(OctreeVisit {
            min: self.bounds.min,
            max_exclusive: self.bounds.max_exclusive(),
            section: None,
            depth,
            fully_visible,
            is_close,
        });
        for child in self.children.iter().flatten() {
            child.visit(
                branches,
                camera_center,
                frustum,
                close_distance,
                skip_frustum_check,
                depth.saturating_add(1),
                is_close,
                visitor,
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OctreeNode {
    Branch(usize),
    Leaf(SectionPosition),
}

impl OctreeNode {
    #[allow(clippy::too_many_arguments)]
    fn visit(
        &self,
        branches: &[OctreeBranch],
        camera_center: [i32; 3],
        frustum: CameraFrustum,
        close_distance: i32,
        skip_frustum_check: bool,
        depth: u32,
        is_close: bool,
        visitor: &mut impl FnMut(OctreeVisit),
    ) {
        match self {
            Self::Branch(index) => branches[*index].visit(
                branches,
                camera_center,
                frustum,
                close_distance,
                skip_frustum_check,
                depth,
                is_close,
                visitor,
            ),
            Self::Leaf(section) => {
                let origin = section.origin();
                let bounds = IntBounds {
                    min: origin,
                    max: origin.map(|coordinate| coordinate.saturating_add(15)),
                };
                let (min, max) = bounds.as_f64();
                if !skip_frustum_check && !frustum.intersects_aabb(min, max) {
                    return;
                }
                let is_close =
                    is_close && exclusive_bounds_are_close(camera_center, bounds, close_distance);
                visitor(OctreeVisit {
                    min: bounds.min,
                    max_exclusive: bounds.max_exclusive(),
                    section: Some(*section),
                    depth,
                    fully_visible: skip_frustum_check,
                    is_close,
                });
            }
        }
    }
}

fn inclusive_bounds_are_close(camera_center: [i32; 3], bounds: IntBounds, distance: i32) -> bool {
    (0..3).all(|axis| {
        camera_center[axis] > bounds.min[axis].saturating_sub(distance)
            && camera_center[axis] < bounds.max[axis].saturating_add(distance)
    })
}

fn exclusive_bounds_are_close(camera_center: [i32; 3], bounds: IntBounds, distance: i32) -> bool {
    (0..3).all(|axis| {
        camera_center[axis] > bounds.min[axis].saturating_sub(distance)
            && camera_center[axis] < bounds.max[axis].saturating_add(1).saturating_add(distance)
    })
}

#[derive(Debug, Clone, Copy)]
enum AxisSorting {
    Xyz,
    Xzy,
    Yxz,
    Yzx,
    Zxy,
    Zyx,
}

impl AxisSorting {
    fn for_differences([x, y, z]: [i32; 3]) -> Self {
        if x > y && x > z {
            if y > z {
                Self::Xyz
            } else {
                Self::Xzy
            }
        } else if y > x && y > z {
            if x > z {
                Self::Yxz
            } else {
                Self::Yzx
            }
        } else if x > y {
            Self::Zxy
        } else {
            Self::Zyx
        }
    }

    fn shifts(self) -> [usize; 3] {
        match self {
            Self::Xyz => [4, 2, 1],
            Self::Xzy => [4, 1, 2],
            Self::Yxz => [2, 4, 1],
            Self::Yzx => [1, 4, 2],
            Self::Zxy => [2, 1, 4],
            Self::Zyx => [1, 2, 4],
        }
    }

    fn node_index(self, opposite: [bool; 3]) -> usize {
        self.shifts()
            .into_iter()
            .zip(opposite)
            .filter_map(|(shift, opposite)| opposite.then_some(shift))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_render::{terrain::TerrainSectionVisibility, CameraPose};

    fn section(x: i32, y: i32, z: i32, loaded_empty: bool) -> ResidentSection {
        ResidentSection {
            position: SectionPosition { x, y, z },
            visibility: TerrainSectionVisibility::all_visible(y, !loaded_empty, loaded_empty),
        }
    }

    fn blocked_section(x: i32, y: i32, z: i32) -> ResidentSection {
        ResidentSection {
            position: SectionPosition { x, y, z },
            visibility: TerrainSectionVisibility::none_visible(y, true, false),
        }
    }

    fn loaded_square(radius: i32) -> TerrainCoordinateSet<ChunkPos> {
        (-radius..=radius)
            .flat_map(|z| (-radius..=radius).map(move |x| ChunkPos { x, z }))
            .collect()
    }

    fn test_frustum() -> CameraFrustum {
        CameraFrustum::from_pose(
            CameraPose {
                position: [8.0, 0.0, 8.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 8.0,
            },
            1.0,
            120.0,
        )
    }

    #[test]
    fn terrain_runtime_occlusion_propagates_fifo_sources_without_backtracking() {
        let sections = [
            section(0, 0, 0, false),
            section(0, 0, 1, false),
            section(1, 0, 1, false),
            section(1, 0, 0, false),
        ];
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, true);

        assert!(graph.is_valid());
        assert_eq!(
            graph
                .node(SectionPosition { x: 0, y: 0, z: 0 })
                .unwrap()
                .step,
            0
        );
        let diagonal = graph.node(SectionPosition { x: 1, y: 0, z: 1 }).unwrap();
        assert_eq!(diagonal.step, 2);
        assert!(diagonal.has_source_direction(TerrainFace::East));
        assert!(diagonal.has_source_direction(TerrainFace::South));
        assert_eq!(graph.reachable_sections().len(), 4);
    }

    #[test]
    fn terrain_runtime_occlusion_loaded_empty_sections_propagate_but_skip_octree() {
        let sections = [section(0, 0, 0, true), section(0, 0, 1, false)];
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, true);

        assert_eq!(graph.reachable_sections().len(), 2);
        let origin = SectionPosition { x: 0, y: 0, z: 0 };
        assert!(graph.visited_empty(origin));
        let visible = graph.visible_sections(test_frustum());
        assert_eq!(
            visible,
            vec![VisibleSection {
                position: SectionPosition { x: 0, y: 0, z: 1 },
                nearby: true,
            }]
        );
        for y in [-64, 64] {
            let mut next = TerrainSectionOcclusionGraph::default();
            next.rebuild(
                [section(0, y, 0, true)],
                &loaded_square(3),
                [8.0, f64::from(y * 16 + 8), 8.0],
                3,
                -80,
                160,
                true,
            );
            graph.replace_preserving_visits(next);
        }
        graph.forget_empty(origin);
        assert!(!graph.visited_empty(origin));
        assert!(graph.visited_empty(SectionPosition { y: -64, ..origin }));
        assert!(graph.visited_empty(SectionPosition { y: 64, ..origin }));
    }

    #[test]
    fn terrain_runtime_occlusion_smart_cull_obeys_compiled_face_visibility() {
        let sections = [
            section(0, 0, 0, false),
            blocked_section(0, 0, 1),
            section(0, 0, 2, false),
        ];
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, true);
        assert!(graph
            .reachable_sections()
            .contains(&SectionPosition { x: 0, y: 0, z: 1 }));
        assert!(!graph
            .reachable_sections()
            .contains(&SectionPosition { x: 0, y: 0, z: 2 }));

        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, false);
        assert!(graph
            .reachable_sections()
            .contains(&SectionPosition { x: 0, y: 0, z: 2 }));
    }

    #[test]
    fn distant_occlusion_ray_resumes_when_a_nonadjacent_section_arrives() {
        let mut graph = TerrainSectionOcclusionGraph::default();
        let loaded = loaded_square(7);
        let initial = [
            (0, 0),
            (1, 0),
            (1, 1),
            (1, 2),
            (2, 2),
            (3, 2),
            (4, 2),
            (5, 2),
        ];
        graph.rebuild(
            initial.map(|(x, z)| section(x, 0, z, false)),
            &loaded,
            [8.0; 3],
            8,
            0,
            1,
            true,
        );
        let frustum = CameraFrustum::from_pose(
            CameraPose {
                position: [8.0; 3],
                y_rot: -70.0,
                x_rot: 0.0,
                eye_height: 0.0,
            },
            1.0,
            70.0,
        );
        let far_visible = |graph: &TerrainSectionOcclusionGraph| {
            graph
                .visible_sections(frustum)
                .iter()
                .any(|section| section.position == SectionPosition { x: 5, y: 0, z: 2 })
        };
        assert!(!far_visible(&graph));
        // The ray from (4, 2) towards the camera crosses (2, 1).
        let late = section(2, 0, 1, false);
        graph.update_sections(
            [(late.position, Some(late.visibility))],
            &loaded,
            [8.0; 3],
            8,
            0,
            1,
            true,
        );
        assert!(far_visible(&graph));
    }

    #[test]
    fn terrain_runtime_occlusion_partial_compile_propagates_without_full_rebuild() {
        let initial = [
            section(0, 0, 0, false),
            blocked_section(0, 0, 1),
            section(0, 0, 2, false),
            blocked_section(1, 0, 0),
            section(2, 0, 0, false),
        ];
        let mut graph = TerrainSectionOcclusionGraph::default();
        let loaded = loaded_square(3);
        graph.rebuild(initial, &loaded, [8.0, 8.0, 8.0], 3, 0, 1, true);
        assert!(!graph
            .reachable_sections()
            .contains(&SectionPosition { x: 0, y: 0, z: 2 }));
        assert!(!graph
            .reachable_sections()
            .contains(&SectionPosition { x: 2, y: 0, z: 0 }));

        graph.update_sections(
            [(
                SectionPosition { x: 0, y: 0, z: 1 },
                Some(section(0, 0, 1, false).visibility),
            )],
            &loaded,
            [8.0, 8.0, 8.0],
            3,
            0,
            1,
            true,
        );

        assert!(graph
            .reachable_sections()
            .contains(&SectionPosition { x: 0, y: 0, z: 2 }));
        assert!(!graph
            .reachable_sections()
            .contains(&SectionPosition { x: 2, y: 0, z: 0 }));
    }

    #[test]
    fn late_horizontal_neighbors_resume_the_waiting_visibility_path() {
        let entrance = section(0, 0, 1, true);
        let target = SectionPosition { x: 0, y: 0, z: 2 };
        let initial = [section(0, 0, 0, false), entrance, section(0, 0, 2, false)];
        let loaded = loaded_square(3);
        let mut partial_loaded = loaded.clone();
        partial_loaded.remove(&ChunkPos { x: 1, z: 2 });
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(initial, &partial_loaded, [8.0; 3], 3, 0, 1, true);
        graph.update_sections([], &partial_loaded, [8.0; 3], 3, 0, 1, true);
        assert!(!graph.visited_empty(entrance.position));
        assert!(!graph
            .visible_sections(test_frustum())
            .iter()
            .any(|visible| visible.position == entrance.position || visible.position == target));

        graph.update_sections([], &loaded, [8.0; 3], 3, 0, 1, true);
        assert!(graph.visited_empty(entrance.position));
        graph.forget_empty(entrance.position);
        graph.update_sections(
            [(entrance.position, Some(entrance.visibility))],
            &loaded,
            [8.0; 3],
            3,
            0,
            1,
            true,
        );
        assert!(graph.visited_empty(entrance.position));
        assert!(graph
            .visible_sections(test_frustum())
            .iter()
            .any(|visible| visible.position == target));
    }

    #[test]
    fn newly_compiled_entrance_exposes_the_far_side_of_a_section_wall() {
        use bbb_client_render::terrain::{
            fixture::{TerrainCell, TerrainChunkFixture, TerrainFixtureWorld},
            TerrainMaterialClass, TerrainTextureAtlas,
        };

        let mut cells = vec![TerrainCell::EMPTY; 16 * 16 * 16];
        for y in 0..16 {
            for z in 0..16 {
                cells[(y * 16 + z) * 16 + 8] =
                    TerrainCell::with_texture(TerrainMaterialClass::Opaque, 0);
            }
        }
        let wall = TerrainFixtureWorld::new(
            &[TerrainChunkFixture::new(1, 1, 0, 16, cells)],
            TerrainTextureAtlas::unit(),
        )
        .build_all(0, [8.0; 3])
        .visibility[0];
        assert!(!wall.faces_can_see_each_other(TerrainFace::West, TerrainFace::East));
        assert!(wall.faces_can_see_each_other(TerrainFace::North, TerrainFace::East));
        let initial = [
            section(0, 0, 0, false),
            section(0, 0, 1, false),
            ResidentSection {
                position: SectionPosition { x: 1, y: 0, z: 1 },
                visibility: wall,
            },
            section(2, 0, 1, false),
        ];
        let entrance = section(1, 0, 0, false);
        let target = SectionPosition { x: 2, y: 0, z: 1 };
        let loaded = loaded_square(3);
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(initial, &loaded, [8.0; 3], 3, 0, 1, true);
        assert!(!graph.reachable_sections().contains(&target));

        let mut partial_loaded = loaded.clone();
        partial_loaded.remove(&ChunkPos { x: 2, z: 2 });
        graph.update_sections(
            [(entrance.position, Some(entrance.visibility))],
            &partial_loaded,
            [8.0; 3],
            3,
            0,
            1,
            true,
        );
        assert!(!graph
            .visible_sections(test_frustum())
            .iter()
            .any(|visible| visible.position == target));

        graph.update_sections([], &loaded, [8.0; 3], 3, 0, 1, true);
        assert!(graph
            .visible_sections(test_frustum())
            .iter()
            .any(|visible| visible.position == target));

        let mut full = TerrainSectionOcclusionGraph::default();
        full.rebuild(
            initial.into_iter().chain([entrance]),
            &loaded,
            [8.0; 3],
            3,
            0,
            1,
            true,
        );
        assert!(full.reachable_sections().contains(&target));
        assert_eq!(graph.reachable_sections(), full.reachable_sections());
    }

    #[test]
    fn terrain_runtime_occlusion_octree_visits_camera_ordered_branches_and_leaves() {
        let sections = [
            section(0, 0, 0, false),
            section(0, 0, 1, false),
            section(1, 0, 1, false),
        ];
        let mut graph = TerrainSectionOcclusionGraph::default();
        graph.rebuild(sections, &loaded_square(3), [8.0, 8.0, 8.0], 3, 0, 1, false);
        let visits = graph.octree_visits(test_frustum(), 32);
        assert!(visits.first().is_some_and(|visit| visit.section.is_none()));
        assert!(visits.iter().any(|visit| visit.depth > 0));
        let leaves = visits
            .iter()
            .filter_map(|visit| visit.section)
            .collect::<Vec<_>>();
        assert_eq!(
            leaves,
            vec![
                SectionPosition { x: 0, y: 0, z: 0 },
                SectionPosition { x: 0, y: 0, z: 1 },
                SectionPosition { x: 1, y: 0, z: 1 },
            ]
        );
        assert_eq!(
            graph
                .visible_sections(test_frustum())
                .into_iter()
                .map(|visible| (visible.position, visible.nearby))
                .collect::<Vec<_>>(),
            visits
                .iter()
                .filter_map(|visit| visit.section.map(|section| (section, visit.is_close)))
                .collect::<Vec<_>>(),
            "production visibility must preserve the debug visitor's leaf order and nearby flags"
        );
    }

    #[test]
    fn terrain_runtime_occlusion_octree_close_bounds_match_vanilla_bound_kinds() {
        let bounds = IntBounds {
            min: [0; 3],
            max: [15; 3],
        };

        assert!(!inclusive_bounds_are_close([-32, 8, 8], bounds, 32));
        assert!(inclusive_bounds_are_close([-31, 8, 8], bounds, 32));
        assert!(!inclusive_bounds_are_close([47, 8, 8], bounds, 32));
        assert!(inclusive_bounds_are_close([46, 8, 8], bounds, 32));

        assert!(!exclusive_bounds_are_close([-32, 8, 8], bounds, 32));
        assert!(exclusive_bounds_are_close([-31, 8, 8], bounds, 32));
        assert!(!exclusive_bounds_are_close([48, 8, 8], bounds, 32));
        assert!(exclusive_bounds_are_close([47, 8, 8], bounds, 32));
    }
}
