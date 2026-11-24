use bbb_client_clock::Instant;
use std::{collections::hash_map::Entry, sync::Arc, time::Duration};

use bbb_client_assets::{BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier};
use bbb_client_render::{
    scene::terrain_textures::{TerrainResourceSource, TerrainTextureState},
    terrain::{
        TerrainBlockRenderTable, TerrainCardinalLighting, TerrainChunkSnapshot, TerrainSectionMesh,
        TerrainSectionMeshLayers, TerrainSectionVisibility,
    },
    CameraFrustum, CameraPose, Renderer, TerrainChunkMeshOwner, TerrainMeshWorkerCounters,
    TerrainSectionMeshOwner, TerrainSectionRenderState,
};
use bbb_client_world::{ChunkPos, FxHashMap, FxHashSet, WorldCardinalLighting, WorldStore};

mod mesh_worker;
mod occlusion;
#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm_dispatch;
use mesh_worker::{
    build_terrain_mesh_job, TerrainMeshJob, TerrainMeshJobKey, TerrainMeshPool, TerrainMeshResult,
};
pub use occlusion::ChunkSectionDebugGizmos;
use occlusion::{
    ResidentSection, SectionPosition, TerrainCoordinateMap, TerrainCoordinateSet,
    TerrainOcclusionJob, TerrainOcclusionWorker, TerrainSectionOcclusionGraph, VisibleSection,
};

// Result merging and snapshot extraction share the render-thread work budget
// so completed worker batches cannot monopolize a frame.
const TERRAIN_CACHE_WORK_BUDGET_PER_FRAME: Duration = Duration::from_millis(3);
const TERRAIN_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);
const VANILLA_SECTION_FADE_DURATION: Duration = Duration::from_millis(750);
const VANILLA_SECTION_CONSUMER_VISIBILITY: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainPrioritizeChunkUpdates {
    None,
    PlayerAffected,
    Nearby,
}

#[derive(Debug, Clone, Copy)]
struct TerrainSectionResidency {
    uploaded_at: Instant,
    fade_duration: Duration,
    compiled_visibility: TerrainSectionVisibility,
}

#[derive(Debug, Clone, Copy)]
enum EntitySectionVisit {
    /// The next visible compile has no fade; the section still needs its first visit.
    PreviouslyEmpty,
    VisibleAt(Instant),
}

impl TerrainSectionResidency {
    fn visibility(self, now: Instant) -> f32 {
        if self.fade_duration.is_zero() {
            return 1.0;
        }
        now.saturating_duration_since(self.uploaded_at).as_millis() as f32
            / self.fade_duration.as_millis() as f32
    }
}

#[derive(Debug, Default)]
struct TerrainSectionFrameProjection {
    states: Vec<TerrainSectionRenderState>,
    by_position: TerrainCoordinateMap<SectionPosition, f32>,
    revision: u64,
    layout_revision: u64,
    fading_indices: Vec<usize>,
    fade_tracked_indices: Vec<usize>,
    fade_states: Vec<(TerrainSectionMeshOwner, f32)>,
}

impl TerrainSectionFrameProjection {
    fn clear(&mut self) {
        self.states.clear();
        self.by_position.clear();
        self.fading_indices.clear();
        self.fade_tracked_indices.clear();
        self.fade_states.clear();
        self.revision = self.revision.wrapping_add(1);
        self.layout_revision = self.layout_revision.wrapping_add(1);
    }

    fn replace<I, V>(
        &mut self,
        level_generation: u64,
        visible_sections: I,
        residency: &TerrainCoordinateMap<SectionPosition, TerrainSectionResidency>,
        now: Instant,
    ) where
        I: IntoIterator<Item = V>,
        V: Into<VisibleSection>,
    {
        self.revision = self.revision.wrapping_add(1);
        self.layout_revision = self.layout_revision.wrapping_add(1);
        self.states.clear();
        self.by_position.clear();
        self.fading_indices.clear();
        self.fade_tracked_indices.clear();
        self.fade_states.clear();
        for visible in visible_sections.into_iter().map(Into::into) {
            let position = visible.position;
            let Some(section) = residency.get(&position) else {
                continue;
            };
            let visibility = section.visibility(now).clamp(0.0, 1.0);
            let owner = TerrainSectionMeshOwner {
                chunk: TerrainChunkMeshOwner {
                    level_generation,
                    chunk_x: position.x,
                    chunk_z: position.z,
                },
                section_y: position.y,
            };
            if visibility < 1.0 {
                let index = self.states.len();
                self.fading_indices.push(index);
                self.fade_tracked_indices.push(index);
                self.fade_states.push((owner, visibility));
            }
            self.states.push(TerrainSectionRenderState {
                owner,
                visibility,
                nearby: visible.nearby,
            });
            self.by_position.insert(position, visibility);
        }
    }

    fn refresh(
        &mut self,
        residency: &TerrainCoordinateMap<SectionPosition, TerrainSectionResidency>,
        now: Instant,
    ) -> bool {
        let mut changed = false;
        let states = &mut self.states;
        let by_position = &mut self.by_position;
        self.fading_indices.retain(|&index| {
            let state = &mut states[index];
            let position = SectionPosition {
                x: state.owner.chunk.chunk_x,
                y: state.owner.section_y,
                z: state.owner.chunk.chunk_z,
            };
            // Membership changes replace the projection before this fade-only path.
            let visibility = residency[&position].visibility(now).clamp(0.0, 1.0);
            if visibility.to_bits() != state.visibility.to_bits() {
                changed = true;
                state.visibility = visibility;
                *by_position.get_mut(&position).unwrap() = visibility;
            }
            visibility < 1.0
        });
        if changed {
            self.revision = self.revision.wrapping_add(1);
            self.fade_states.clear();
            for &index in &self.fade_tracked_indices {
                let state = self.states[index];
                self.fade_states.push((state.owner, state.visibility));
            }
        }
        changed
    }

    fn has_fading_sections(&self) -> bool {
        !self.fading_indices.is_empty()
    }

    fn consumer_visible(&self, position: SectionPosition) -> bool {
        self.by_position
            .get(&position)
            .is_some_and(|visibility| *visibility >= VANILLA_SECTION_CONSUMER_VISIBILITY)
    }
}

#[derive(Debug)]
struct CachedTerrainMesh {
    revision: u64,
    section_mesh_revisions: Vec<(i32, u64)>,
    resident_revisions: Vec<(i32, u64)>,
    camera_position: [f32; 3],
    layers: TerrainSectionMeshLayers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerrainOcclusionRebuildKey {
    level_generation: u64,
    camera_cell: [i32; 3],
    render_distance_chunks: u32,
    min_section_y: i32,
    section_count: usize,
    smart_cull: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerrainOcclusionInputKey {
    level_generation: u64,
    membership_revision: u64,
    visibility_revision: u64,
}

#[derive(Debug)]
struct TerrainOcclusionSectionUpdate {
    visibility: Option<TerrainSectionVisibility>,
    removed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerrainFrustumVisibilityKey {
    graph_key: Option<TerrainOcclusionRebuildKey>,
    graph_valid: bool,
    admits_entity_sections: bool,
    visibility_revision: u64,
    camera_rotation_bucket: Option<[i32; 2]>,
    surface_size: [u32; 2],
    camera_fov_bits: u32,
}

#[derive(Debug, PartialEq, Eq)]
enum TerrainCacheWork {
    QueueMesh(TerrainMeshWork),
    Idle,
}

#[derive(Debug, PartialEq, Eq)]
struct TerrainMeshWork {
    key: TerrainMeshJobKey,
    dirty_sections: Vec<DirtyTerrainSection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainMeshQueueOutcome {
    SynchronouslyCompiled { renderer_dirty: bool },
    Async,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirtyTerrainSection {
    section_y: i32,
    revision: u64,
    dirty_from_player: bool,
}

fn section_is_dirty_from_player(player_changed_revision: u64, compiled_revision: u64) -> bool {
    player_changed_revision > compiled_revision
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerrainSelectionKey {
    membership_revision: u64,
    center: ChunkPos,
}

impl TerrainSelectionKey {
    fn of(world: &WorldStore) -> Option<Self> {
        (world.chunk_count() != 0).then(|| Self {
            membership_revision: world.chunk_membership_revision(),
            center: terrain_upload_center(world),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerrainResidencyKey {
    membership_revision: u64,
    world_revision: u64,
    mesh_generation: u64,
}

impl TerrainResidencyKey {
    fn of(world: &WorldStore, upload: &TerrainUploadState) -> Self {
        Self {
            membership_revision: world.chunk_membership_revision(),
            world_revision: world.terrain_revision(),
            mesh_generation: upload.mesh_generation,
        }
    }
}

#[derive(Debug, Default)]
pub struct TerrainUploadState {
    observed_level_generation: u64,
    observed_world_revision: u64,
    selection_key: Option<TerrainSelectionKey>,
    selected_chunks: Vec<ChunkPos>,
    selected_cursor: usize,
    meshes: FxHashMap<ChunkPos, CachedTerrainMesh>,
    /// Remember consumed player changes across resource-driven mesh clears.
    compiled_player_sections: TerrainCoordinateMap<SectionPosition, u64>,
    next_resident_revision: u64,
    occlusion_revision: u64,
    mesh_section_updates: TerrainCoordinateSet<SectionPosition>,
    mesh_generation: u64,
    pending_mesh_jobs: FxHashMap<ChunkPos, TerrainMeshJobKey>,
    mesh_worker: Option<TerrainMeshPool>,
    block_table: Option<Arc<TerrainBlockRenderTable>>,
    mesh_worker_counters: TerrainMeshWorkerCounters,
    residency_key: Option<TerrainResidencyKey>,
    cache_ready: bool,
    reload_all_chunks_requested: bool,
    texture_animation_tick: u64,
    last_texture_animation_at: Option<Instant>,
    section_residency: TerrainCoordinateMap<SectionPosition, TerrainSectionResidency>,
    // Eager terrain upload can precede the first visible section compile.
    entity_visits: TerrainCoordinateMap<SectionPosition, EntitySectionVisit>,
    occlusion_graph: TerrainSectionOcclusionGraph,
    occlusion_rebuild_key: Option<TerrainOcclusionRebuildKey>,
    pending_occlusion_rebuild_key: Option<TerrainOcclusionRebuildKey>,
    occlusion_sections_revision: Option<u64>,
    occlusion_loaded_key: Option<(u64, u64)>,
    occlusion_graph_input_key: Option<TerrainOcclusionInputKey>,
    pending_occlusion_input_key: Option<TerrainOcclusionInputKey>,
    occlusion_section_updates: TerrainCoordinateMap<SectionPosition, TerrainOcclusionSectionUpdate>,
    occlusion_sections: Arc<Vec<ResidentSection>>,
    occlusion_loaded_chunks: Arc<TerrainCoordinateSet<ChunkPos>>,
    occlusion_worker: Option<TerrainOcclusionWorker>,
    frustum_visibility_key: Option<TerrainFrustumVisibilityKey>,
    frustum_visible_sections: Vec<VisibleSection>,
    renderer_sections: TerrainSectionFrameProjection,
    reported_readiness: Option<(u64, usize, bool)>,
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainVisibilityContext {
    pub(crate) camera_pose: Option<CameraPose>,
    pub(crate) surface_size: [u32; 2],
    pub(crate) camera_fov_degrees: f32,
    pub(crate) render_distance_chunks: u32,
    pub(crate) smart_cull: bool,
}

impl TerrainVisibilityContext {
    pub fn new(
        camera_pose: Option<CameraPose>,
        surface_size: [u32; 2],
        camera_fov_degrees: f32,
        render_distance_chunks: u32,
        smart_cull: bool,
    ) -> Self {
        Self {
            camera_pose,
            surface_size,
            camera_fov_degrees,
            render_distance_chunks,
            smart_cull,
        }
    }

    fn frustum(self) -> Option<CameraFrustum> {
        let pose = self.camera_pose?;
        let aspect = self.surface_size[0].max(1) as f32 / self.surface_size[1].max(1) as f32;
        Some(
            CameraFrustum::from_pose(pose, aspect, self.camera_fov_degrees).including_camera_cube(
                [
                    pose.position[0],
                    pose.position[1] + pose.eye_height,
                    pose.position[2],
                ],
            ),
        )
    }
}

impl TerrainUploadState {
    pub fn refresh_frame_visibility(&mut self, context: TerrainVisibilityContext) -> bool {
        let key = terrain_frustum_visibility_key(self, context);
        if self.frustum_visibility_key == Some(key) {
            if !self.renderer_sections.has_fading_sections() {
                return false;
            }
            return self
                .renderer_sections
                .refresh(&self.section_residency, bbb_client_clock::now());
        }
        publish_terrain_frustum_visibility(self, context);
        true
    }

    pub fn renderer_sections(&self) -> &[TerrainSectionRenderState] {
        &self.renderer_sections.states
    }

    pub fn renderer_sections_revision(&self) -> u64 {
        self.renderer_sections.revision
    }

    pub fn renderer_sections_layout_revision(&self) -> u64 {
        self.renderer_sections.layout_revision
    }

    pub fn renderer_fading_sections(&self) -> &[(TerrainSectionMeshOwner, f32)] {
        &self.renderer_sections.fade_states
    }

    pub fn chunk_section_debug_gizmos(
        &self,
        context: TerrainVisibilityContext,
        show_paths: bool,
        show_octree: bool,
        show_visibility: bool,
    ) -> ChunkSectionDebugGizmos {
        context
            .frustum()
            .map_or_else(ChunkSectionDebugGizmos::default, |frustum| {
                self.occlusion_graph
                    .debug_gizmos(frustum, show_paths, show_octree, show_visibility)
            })
    }

    /// Whether queued, budgeted or in-flight terrain work remains for `world`:
    /// a mesh job on the worker, an occlusion rebuild, a selection refresh, or
    /// a chunk the per-frame budget has not reached yet.
    pub fn has_pending_work(&self, world: &WorldStore) -> bool {
        !self.pending_mesh_jobs.is_empty()
            || self
                .mesh_worker
                .as_ref()
                .is_some_and(|pool| pool.outstanding() != 0)
            || self.pending_occlusion_rebuild_key.is_some()
            || self.pending_occlusion_input_key.is_some()
            || self.observed_world_revision != world.terrain_revision()
            || self.selection_key != TerrainSelectionKey::of(world)
            || !matches!(
                next_terrain_cache_work(world, self, &mut 0),
                TerrainCacheWork::Idle
            )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_uploaded_chunks(&self) -> bool {
        !self.meshes.is_empty()
    }

    pub fn request_reload_all_chunks(&mut self) {
        self.reload_all_chunks_requested = true;
    }

    pub(crate) fn replace_mesh_resources(&mut self, rebuild: bool) {
        self.block_table = None;
        self.reload_all_chunks_requested |= rebuild;
    }

    pub(crate) fn ensure_block_table(
        &mut self,
        world: &WorldStore,
        textures: &TerrainTextureState,
    ) -> Arc<TerrainBlockRenderTable> {
        let context = world.terrain_block_render_context();
        if self
            .block_table
            .as_ref()
            .is_some_and(|table| !table.is_current(textures, &context))
        {
            self.block_table = None;
        }
        self.block_table
            .get_or_insert_with(|| {
                Arc::new(TerrainBlockRenderTable::from_resources(
                    TerrainResourceSource::new(textures, context),
                ))
            })
            .clone()
    }

    pub fn is_player_section_compiled_and_visible(&self, world: &WorldStore) -> bool {
        let Some(pose) = world.local_player_pose() else {
            return false;
        };
        let block_y = pose.position.y.floor() as i32;
        if !world.dimension().contains_y(block_y) {
            return false;
        }
        let position = SectionPosition {
            x: (pose.position.x.floor() as i32).div_euclid(16),
            y: block_y.div_euclid(16),
            z: (pose.position.z.floor() as i32).div_euclid(16),
        };
        section_residency_is_consumer_visible(
            &self.section_residency,
            position,
            bbb_client_clock::now(),
        )
    }

    pub fn is_entity_section_compiled_and_visible(
        &self,
        world: &WorldStore,
        position: bbb_client_world::EntityVec3,
    ) -> bool {
        let block_y = position.y.floor() as i32;
        if !world.dimension().contains_y(block_y) {
            return true;
        }
        let position = SectionPosition {
            x: (position.x.floor() as i32).div_euclid(16),
            y: block_y.div_euclid(16),
            z: (position.z.floor() as i32).div_euclid(16),
        };
        matches!(self.entity_visits.get(&position), Some(EntitySectionVisit::VisibleAt(at)) if bbb_client_clock::now() >= *at)
            || (self.occlusion_graph.visited_empty(position)
                && self.section_residency.contains_key(&position))
    }

    pub fn is_block_section_compiled_and_visible(
        &self,
        world: &WorldStore,
        position: bbb_client_world::BlockPos,
    ) -> bool {
        if !world.dimension().contains_y(position.y) {
            return true;
        }
        self.renderer_sections.consumer_visible(SectionPosition {
            x: position.x.div_euclid(16),
            y: position.y.div_euclid(16),
            z: position.z.div_euclid(16),
        })
    }

    pub(crate) fn is_section_occluder_ready(
        &self,
        world: &WorldStore,
        section_x: i32,
        section_y: i32,
        section_z: i32,
    ) -> bool {
        let section = SectionPosition {
            x: section_x,
            y: section_y,
            z: section_z,
        };
        if !self
            .renderer_sections
            .by_position
            .get(&section)
            .is_some_and(|&visibility| visibility >= 1.0)
        {
            return false;
        }
        let chunk = ChunkPos {
            x: section_x,
            z: section_z,
        };
        self.meshes
            .get(&chunk)
            .is_some_and(|mesh| mesh.revision == world.terrain_chunk_mesh_revision(chunk))
    }

    #[cfg(test)]
    pub(crate) fn mark_section_occluder_ready_for_test(
        &mut self,
        world: &WorldStore,
        section_x: i32,
        section_y: i32,
        section_z: i32,
        visibility: f32,
    ) {
        let section = SectionPosition {
            x: section_x,
            y: section_y,
            z: section_z,
        };
        self.renderer_sections
            .by_position
            .insert(section, visibility);
        self.renderer_sections.revision = self.renderer_sections.revision.wrapping_add(1);
        if visibility >= VANILLA_SECTION_CONSUMER_VISIBILITY {
            self.entity_visits.insert(
                section,
                EntitySectionVisit::VisibleAt(bbb_client_clock::now()),
            );
        }
        let chunk = ChunkPos {
            x: section_x,
            z: section_z,
        };
        let revision = world.terrain_chunk_mesh_revision(chunk);
        self.meshes.insert(
            chunk,
            CachedTerrainMesh {
                revision,
                section_mesh_revisions: vec![(section_y, revision)],
                resident_revisions: vec![(section_y, 1)],
                camera_position: [0.0; 3],
                layers: TerrainSectionMeshLayers::default(),
            },
        );
    }

    /// Returns a command payload only when readiness changes or a new level
    /// load becomes active. Same-dimension respawn keeps the ClientLevel but
    /// starts a fresh Vanilla LevelLoadTracker.
    pub fn take_readiness_edge(
        &mut self,
        level_generation: u64,
        level_load_start: usize,
        ready: bool,
    ) -> Option<(u64, usize, bool)> {
        let next = (level_generation, level_load_start, ready);
        if self.reported_readiness == Some(next) {
            return None;
        }
        self.reported_readiness = Some(next);
        Some(next)
    }

    #[cfg(test)]
    pub fn reload_all_chunks_requested(&self) -> bool {
        self.reload_all_chunks_requested
    }
}

#[cfg(test)]
mod readiness_tests {
    use super::TerrainUploadState;

    #[test]
    fn readiness_reports_only_edges_and_new_level_generations() {
        let mut upload = TerrainUploadState::default();
        assert_eq!(upload.take_readiness_edge(4, 1, false), Some((4, 1, false)));
        assert_eq!(upload.take_readiness_edge(4, 1, false), None);
        assert_eq!(upload.take_readiness_edge(4, 1, true), Some((4, 1, true)));
        assert_eq!(upload.take_readiness_edge(4, 1, true), None);
        assert_eq!(upload.take_readiness_edge(4, 2, true), Some((4, 2, true)));
        assert_eq!(upload.take_readiness_edge(5, 1, true), Some((5, 1, true)));
    }
}

#[cfg(test)]
mod section_fade_tests {
    use super::*;
    use bbb_client_world::{LocalPlayerPoseState, WorldDimension};
    use bbb_protocol::packets::Vec3d;

    #[test]
    fn far_section_fades_zero_threshold_to_one_at_vanilla_times() {
        let now = Instant::now();
        let far = SectionPosition { x: 8, y: 0, z: 0 };
        let section = TerrainSectionResidency {
            uploaded_at: now,
            fade_duration: section_fade_duration(far, [8.0, 8.0, 8.0]),
            compiled_visibility: TerrainSectionVisibility::all_visible(far.y, true, false),
        };

        assert_eq!(section.fade_duration, VANILLA_SECTION_FADE_DURATION);
        assert_eq!(section.visibility(now), 0.0);
        assert!(!section_residency_is_consumer_visible(
            &TerrainCoordinateMap::from_iter([(far, section)]),
            far,
            now + Duration::from_millis(224),
        ));
        assert!(section_residency_is_consumer_visible(
            &TerrainCoordinateMap::from_iter([(far, section)]),
            far,
            now + Duration::from_millis(225),
        ));
        assert_eq!(section.visibility(now + VANILLA_SECTION_FADE_DURATION), 1.0);
        assert_eq!(
            section_fade_duration(
                SectionPosition {
                    x: i32::MAX,
                    y: i32::MAX,
                    z: i32::MAX,
                },
                [f32::MIN; 3],
            ),
            VANILLA_SECTION_FADE_DURATION,
            "malformed extreme coordinates remain bounded instead of overflowing"
        );
    }

    #[test]
    fn nearby_and_previously_empty_sections_do_not_restart_fade_on_recompile() {
        let now = Instant::now();
        let pos = ChunkPos { x: 0, z: 0 };
        let position = SectionPosition { x: 0, y: 0, z: 0 };
        let empty = TerrainSectionVisibility::all_visible(0, false, true);
        let filled = TerrainSectionVisibility::all_visible(0, true, false);
        let mut upload = TerrainUploadState::default();
        install_resident_section(&mut upload, pos, empty, [8.0; 3], now);
        let first = upload.section_residency[&position];
        assert_eq!(first.fade_duration, Duration::ZERO);
        install_resident_section(
            &mut upload,
            pos,
            filled,
            [256.0, 8.0, 8.0],
            now + Duration::from_secs(1),
        );
        let recompiled = upload.section_residency[&position];
        assert_eq!(recompiled.uploaded_at, first.uploaded_at);
        assert_eq!(recompiled.fade_duration, Duration::ZERO);
        let far_pos = ChunkPos { x: 8, z: 0 };
        let far_position = SectionPosition { x: 8, y: 0, z: 0 };
        install_resident_section(&mut upload, far_pos, empty, [8.0; 3], now);
        assert_eq!(
            upload.section_residency[&far_position].fade_duration,
            VANILLA_SECTION_FADE_DURATION
        );
        install_resident_section(
            &mut upload,
            far_pos,
            filled,
            [8.0; 3],
            now + Duration::from_millis(100),
        );
        assert_eq!(
            upload.section_residency[&far_position].fade_duration,
            Duration::ZERO
        );
        remove_resident_section(&mut upload, position);
        assert!(!upload.section_residency.contains_key(&position));
        assert!(upload.section_residency.contains_key(&far_position));
    }

    #[test]
    fn ordered_frame_projection_refreshes_fade_and_consumer_gate_together() {
        let now = Instant::now();
        let position = SectionPosition { x: 4, y: 2, z: -3 };
        let mut residency = TerrainCoordinateMap::default();
        residency.insert(
            position,
            TerrainSectionResidency {
                uploaded_at: now,
                fade_duration: VANILLA_SECTION_FADE_DURATION,
                compiled_visibility: TerrainSectionVisibility::default(),
            },
        );
        let mut projection = TerrainSectionFrameProjection::default();
        projection.replace(9, [position], &residency, now + Duration::from_millis(224));
        assert!(projection.has_fading_sections());
        assert!(!projection.consumer_visible(position));
        assert_eq!(projection.states[0].owner.chunk.level_generation, 9);

        assert!(projection.refresh(&residency, now + Duration::from_millis(225)));
        assert!(projection.consumer_visible(position));
        assert_eq!(projection.states[0].visibility, 0.3);
        assert!(!projection.refresh(&residency, now + Duration::from_millis(225)));

        residency.clear();
        projection.replace(
            9,
            std::iter::empty::<SectionPosition>(),
            &residency,
            now + Duration::from_millis(226),
        );
        assert!(projection.states.is_empty());
        assert!(!projection.consumer_visible(position));
    }

    #[test]
    fn player_entity_block_entity_and_readiness_share_the_point_three_gate() {
        let now = Instant::now();
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: -64,
            height: 384,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
            ..LocalPlayerPoseState::default()
        });
        let position = SectionPosition { x: 0, y: 4, z: 0 };
        let mut upload = TerrainUploadState::default();
        upload.section_residency.insert(
            position,
            TerrainSectionResidency {
                uploaded_at: now - Duration::from_millis(100),
                fade_duration: VANILLA_SECTION_FADE_DURATION,
                compiled_visibility: TerrainSectionVisibility::default(),
            },
        );
        upload
            .renderer_sections
            .replace(1, [position], &upload.section_residency, now);

        // These three read their own `Instant::now()`, so the residency has to
        // be aged against a clock taken here rather than against the one the
        // projection was built from: whatever the scheduler spends between the
        // two would otherwise be added to the 100 ms and cross the 225 ms the
        // 0.3 gate sits at.
        let sample = Instant::now();
        upload
            .section_residency
            .get_mut(&position)
            .unwrap()
            .uploaded_at = sample - Duration::from_millis(100);
        upload.entity_visits.insert(
            position,
            EntitySectionVisit::VisibleAt(sample + Duration::from_millis(125)),
        );
        assert!(!upload.is_player_section_compiled_and_visible(&world));
        assert!(!upload.is_entity_section_compiled_and_visible(
            &world,
            bbb_client_world::EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
        ));
        assert!(!upload.is_block_section_compiled_and_visible(
            &world,
            bbb_client_world::BlockPos { x: 1, y: 64, z: 1 },
        ));

        upload
            .section_residency
            .get_mut(&position)
            .unwrap()
            .uploaded_at = now - Duration::from_millis(300);
        upload
            .entity_visits
            .insert(position, EntitySectionVisit::VisibleAt(now));
        assert!(upload
            .renderer_sections
            .refresh(&upload.section_residency, now));
        // Past the gate the drift only helps, so this side can keep the clock
        // the projection was refreshed against.
        assert!(upload.is_player_section_compiled_and_visible(&world));
        assert!(upload.is_entity_section_compiled_and_visible(
            &world,
            bbb_client_world::EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
        ));
        assert!(upload.is_block_section_compiled_and_visible(
            &world,
            bbb_client_world::BlockPos { x: 1, y: 64, z: 1 },
        ));
    }

    #[test]
    fn prioritize_chunk_updates_matches_vanilla_sync_decision() {
        let pos = ChunkPos { x: 0, z: 0 };
        let server_dirty = DirtyTerrainSection {
            section_y: 0,
            revision: 2,
            dirty_from_player: false,
        };
        let player_dirty = DirtyTerrainSection {
            dirty_from_player: true,
            ..server_dirty
        };

        assert!(!section_rebuilds_synchronously(
            TerrainPrioritizeChunkUpdates::None,
            pos,
            player_dirty,
            [8.0, 8.0, 8.0],
        ));
        assert!(!section_rebuilds_synchronously(
            TerrainPrioritizeChunkUpdates::PlayerAffected,
            pos,
            server_dirty,
            [8.0, 8.0, 8.0],
        ));
        assert!(section_rebuilds_synchronously(
            TerrainPrioritizeChunkUpdates::PlayerAffected,
            pos,
            player_dirty,
            [128.0, 8.0, 8.0],
        ));
        assert!(section_rebuilds_synchronously(
            TerrainPrioritizeChunkUpdates::Nearby,
            pos,
            server_dirty,
            [35.0, 8.0, 8.0],
        ));
        assert!(!section_rebuilds_synchronously(
            TerrainPrioritizeChunkUpdates::Nearby,
            pos,
            server_dirty,
            [36.0, 8.0, 8.0],
        ));
        assert!(section_is_dirty_from_player(7, 6));
        assert!(!section_is_dirty_from_player(7, 7));
        assert!(!section_is_dirty_from_player(7, 9));
    }
}

pub fn synchronize_terrain_biome_registry(
    textures: &mut TerrainTextureState,
    world: &WorldStore,
) -> bool {
    textures.synchronize_biome_color_profiles(
        world.registry_resource_ids_in_holder_order("minecraft:worldgen/biome"),
        |holder_id, resource_id| {
            world
                .registry_biome_client_projection(holder_id)
                .map(|projection| {
                    biome_profile_from_projection(holder_id, resource_id.to_string(), projection)
                })
        },
    )
}

fn biome_profile_from_projection(
    id: i32,
    name: String,
    projection: &bbb_client_world::BiomeClientProjection,
) -> BiomeColorProfile {
    BiomeColorProfile {
        id,
        name,
        temperature: projection.temperature,
        temperature_modifier: match projection.temperature_modifier {
            bbb_client_world::BiomeTemperatureModifier::None => BiomeTemperatureModifier::None,
            bbb_client_world::BiomeTemperatureModifier::Frozen => BiomeTemperatureModifier::Frozen,
        },
        downfall: projection.downfall,
        has_precipitation: projection.has_precipitation,
        grass_color: projection.grass_color,
        foliage_color: projection.foliage_color,
        dry_foliage_color: projection.dry_foliage_color,
        water_color: Some(projection.water_color),
        grass_color_modifier: match projection.grass_color_modifier {
            bbb_client_world::BiomeGrassColorModifier::None => GrassColorModifier::None,
            bbb_client_world::BiomeGrassColorModifier::DarkForest => GrassColorModifier::DarkForest,
            bbb_client_world::BiomeGrassColorModifier::Swamp => GrassColorModifier::Swamp,
        },
    }
}

pub fn maybe_upload_terrain_texture_animation(
    renderer: &mut Renderer,
    upload: &mut TerrainUploadState,
    _textures: &mut TerrainTextureState,
) {
    let Some((previous_tick, tick)) =
        advance_texture_animation_tick(upload, bbb_client_clock::now())
    else {
        return;
    };
    renderer.advance_terrain_texture_animation(previous_tick, tick);
}

fn advance_texture_animation_tick(
    upload: &mut TerrainUploadState,
    now: Instant,
) -> Option<(u64, u64)> {
    let Some(last) = upload.last_texture_animation_at else {
        upload.last_texture_animation_at = Some(now);
        return None;
    };
    let elapsed = now.saturating_duration_since(last);
    let ticks = elapsed.as_millis() / TERRAIN_TEXTURE_ANIMATION_INTERVAL.as_millis();
    if ticks == 0 {
        return None;
    }

    let ticks = u64::try_from(ticks).unwrap_or(u64::MAX);
    let previous_tick = upload.texture_animation_tick;
    upload.texture_animation_tick = upload.texture_animation_tick.saturating_add(ticks);
    let advanced = Duration::from_millis(
        ticks.saturating_mul(TERRAIN_TEXTURE_ANIMATION_INTERVAL.as_millis() as u64),
    );
    upload.last_texture_animation_at = last.checked_add(advanced).or(Some(now));
    Some((previous_tick, upload.texture_animation_tick))
}

pub fn maybe_upload_decoded_terrain(
    world: &WorldStore,
    renderer: &mut Renderer,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    visibility: TerrainVisibilityContext,
    prioritize_chunk_updates: TerrainPrioritizeChunkUpdates,
    camera_position: [f32; 3],
) {
    let cache_work_started = Instant::now();
    let sections_per_column = world.dimension().section_count();
    let mesh_worker_counters_before = upload
        .mesh_worker
        .as_ref()
        .is_some_and(|pool| pool.outstanding() != 0)
        .then_some(upload.mesh_worker_counters);
    let mut renderer_dirty = drain_terrain_mesh_results(world, upload, cache_work_started);
    let mesh_worker_counters_changed =
        mesh_worker_counters_before.is_some_and(|before| upload.mesh_worker_counters != before);
    let level_generation_changed =
        upload.observed_level_generation != world.client_level_generation();
    let level_changed =
        synchronize_terrain_level_generation(upload, world.client_level_generation());
    if world.chunk_count() == 0 {
        let terrain_cleared = synchronize_empty_terrain_upload(upload, world.terrain_revision());
        if renderer_dirty || level_changed || terrain_cleared {
            upload_cached_terrain(renderer, upload, sections_per_column, true);
        }
        refresh_terrain_visibility(world, upload, visibility);
        publish_mesh_worker_counters(world, renderer, upload);
        return;
    }

    renderer_dirty |= !renderer.terrain_retry_owners().is_empty();
    if terrain_cache_selection_is_stable(world, upload, level_changed) {
        if renderer_dirty {
            upload_cached_terrain(renderer, upload, sections_per_column, false);
        }
        refresh_terrain_visibility(world, upload, visibility);
        // `cache_ready` is established only after the selected/resident/pending scan below.
        // With the same level generation and terrain revision, no reload request and no
        // mesh-worker counter change, those inputs are unchanged. Preserve the already-
        // published counters instead of walking every selected chunk and resident section
        // per frame. A late stale result still republishes its diagnostic counters.
        if mesh_worker_counters_changed {
            publish_mesh_worker_counters(world, renderer, upload);
        }
        return;
    }

    renderer_dirty |= level_changed;
    let renderer_full_sync = level_generation_changed || upload.reload_all_chunks_requested;
    let selection_needs_refresh =
        terrain_cache_selection_needs_refresh(world, upload, level_changed);
    if selection_needs_refresh || upload.observed_world_revision != world.terrain_revision() {
        upload.selected_cursor = 0;
    }
    if selection_needs_refresh {
        if upload.reload_all_chunks_requested {
            renderer_dirty |= !upload.meshes.is_empty();
            cancel_queued_terrain_mesh_jobs(upload);
            upload.mesh_generation = upload.mesh_generation.saturating_add(1);
            upload.block_table = None;
            upload.meshes.clear();
            upload.mesh_section_updates.clear();
            upload.section_residency.clear();
            upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
            upload.occlusion_rebuild_key = None;
            clear_terrain_occlusion_inputs(upload);
            upload.renderer_sections.clear();
            upload.cache_ready = false;
            upload.reload_all_chunks_requested = false;
        }
        if upload.section_residency.is_empty() {
            // Reserve before the first meshes arrive so growth does not move installed sections.
            let side = 2 * (visibility.render_distance_chunks as usize + 1);
            upload
                .section_residency
                .reserve(side * side * sections_per_column);
        }
        renderer_dirty |= refresh_terrain_selection(world, upload);
    }
    upload.observed_world_revision = world.terrain_revision();

    // Charge completed worker uploads to this frame before preparing more jobs.
    // A full replacement or a synchronous player edit still publishes below.
    if renderer_dirty && !renderer_full_sync {
        upload_cached_terrain(renderer, upload, sections_per_column, false);
        renderer_dirty = false;
    }
    refresh_terrain_residency(world, upload);
    let mut selected_cursor = upload.selected_cursor;
    while cache_work_started.elapsed() < TERRAIN_CACHE_WORK_BUDGET_PER_FRAME {
        match next_terrain_cache_work(world, upload, &mut selected_cursor) {
            TerrainCacheWork::QueueMesh(work) => {
                match queue_terrain_chunk_mesh(
                    world,
                    upload,
                    textures,
                    work,
                    prioritize_chunk_updates,
                    camera_position,
                    renderer.uses_embedded_terrain_shader(),
                ) {
                    TerrainMeshQueueOutcome::SynchronouslyCompiled {
                        renderer_dirty: changed,
                    } => {
                        renderer_dirty |= changed;
                        // Priority work can leave other dirty sections in this column.
                        selected_cursor -= 1;
                    }
                    TerrainMeshQueueOutcome::Async => {}
                    TerrainMeshQueueOutcome::Full => {
                        selected_cursor -= 1;
                        upload.mesh_worker_counters.queue_full =
                            upload.mesh_worker_counters.queue_full.saturating_add(1);
                        break;
                    }
                }
            }
            TerrainCacheWork::Idle => break,
        }
    }

    upload.selected_cursor = selected_cursor;
    if renderer_dirty {
        upload_cached_terrain(renderer, upload, sections_per_column, renderer_full_sync);
    }
    refresh_terrain_visibility(world, upload, visibility);
    publish_mesh_worker_counters(world, renderer, upload);
}

fn terrain_cache_selection_is_stable(
    world: &WorldStore,
    upload: &TerrainUploadState,
    level_changed: bool,
) -> bool {
    !level_changed
        && !upload.reload_all_chunks_requested
        && upload.cache_ready
        && upload.observed_world_revision == world.terrain_revision()
        && upload.selection_key == TerrainSelectionKey::of(world)
}

fn terrain_cache_selection_needs_refresh(
    world: &WorldStore,
    upload: &TerrainUploadState,
    level_changed: bool,
) -> bool {
    level_changed
        || upload.reload_all_chunks_requested
        || upload.selection_key != TerrainSelectionKey::of(world)
}

fn synchronize_terrain_level_generation(
    upload: &mut TerrainUploadState,
    level_generation: u64,
) -> bool {
    if upload.observed_level_generation == level_generation {
        return false;
    }
    let clear_renderer = !upload.meshes.is_empty();
    cancel_queued_terrain_mesh_jobs(upload);
    upload.mesh_generation = upload.mesh_generation.saturating_add(1);
    upload.observed_level_generation = level_generation;
    upload.selection_key = None;
    upload.block_table = None;
    upload.selected_chunks.clear();
    upload.selected_cursor = 0;
    upload.meshes.clear();
    upload.mesh_section_updates.clear();
    upload.compiled_player_sections.clear();
    upload.reload_all_chunks_requested = false;
    upload.section_residency.clear();
    upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
    upload.occlusion_rebuild_key = None;
    clear_terrain_occlusion_inputs(upload);
    upload.renderer_sections.clear();
    upload.cache_ready = false;
    clear_renderer
}

fn synchronize_empty_terrain_upload(upload: &mut TerrainUploadState, world_revision: u64) -> bool {
    let clear_renderer = !upload.meshes.is_empty();
    cancel_queued_terrain_mesh_jobs(upload);
    upload.observed_world_revision = world_revision;
    upload.selection_key = None;
    upload.selected_chunks.clear();
    upload.selected_cursor = 0;
    upload.meshes.clear();
    upload.mesh_section_updates.clear();
    upload.compiled_player_sections.clear();
    upload.reload_all_chunks_requested = false;
    upload.section_residency.clear();
    upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
    upload.occlusion_rebuild_key = None;
    clear_terrain_occlusion_inputs(upload);
    upload.renderer_sections.clear();
    upload.cache_ready = false;
    clear_renderer
}

fn refresh_terrain_selection(world: &WorldStore, upload: &mut TerrainUploadState) -> bool {
    let next = TerrainSelectionKey::of(world);
    if upload.selection_key == next {
        return false;
    }
    let mut removed_mesh = false;
    if upload.selection_key.map(|key| key.membership_revision)
        != Some(world.chunk_membership_revision())
    {
        upload
            .compiled_player_sections
            .retain(|position, _| world.probe_chunk(position.chunk()).is_some());
        let old = std::mem::replace(&mut upload.selected_chunks, world.chunk_positions());
        for pos in old {
            if world.probe_chunk(pos).is_some() {
                continue;
            }
            if let Some(mesh) = upload.meshes.remove(&pos) {
                for (y, _) in mesh.resident_revisions {
                    remove_resident_section(
                        upload,
                        SectionPosition {
                            x: pos.x,
                            y,
                            z: pos.z,
                        },
                    );
                }
                removed_mesh = true;
            }
        }
    }
    sort_terrain_chunks(&mut upload.selected_chunks, terrain_upload_center(world));
    upload.selection_key = next;
    removed_mesh
}

fn sort_terrain_chunks(positions: &mut [ChunkPos], center: ChunkPos) {
    positions.sort_unstable_by_key(|pos| (chunk_distance_key(*pos, center), pos.x, pos.z));
}

fn next_terrain_cache_work(
    world: &WorldStore,
    upload: &TerrainUploadState,
    selected_cursor: &mut usize,
) -> TerrainCacheWork {
    while let Some(&target) = upload.selected_chunks.get(*selected_cursor) {
        let key = terrain_mesh_job_key(world, upload, target);
        if world.terrain_chunk_initial_light_pending(target)
            || upload
                .meshes
                .get(&target)
                .is_some_and(|cached| cached.revision == key.mesh_revision)
            || upload.pending_mesh_jobs.get(&target) == Some(&key)
        {
            *selected_cursor += 1;
            continue;
        }
        let work = terrain_mesh_work(world, upload, target);
        *selected_cursor += 1;
        return TerrainCacheWork::QueueMesh(work);
    }
    TerrainCacheWork::Idle
}

fn snapshot_chunk_from_world(
    world: &WorldStore,
    pos: ChunkPos,
    cardinal_lighting: TerrainCardinalLighting,
) -> Option<Arc<TerrainChunkSnapshot>> {
    let (min_y, height, source_min_section_y, source) = world.extract_terrain_chunk_source(pos)?;
    Some(Arc::new(TerrainChunkSnapshot::from_source(
        pos.x,
        pos.z,
        min_y,
        height,
        source_min_section_y,
        source,
        cardinal_lighting,
    )))
}

#[cfg(test)]
fn terrain_section_mesh_revisions(world: &WorldStore, pos: ChunkPos) -> Vec<(i32, u64)> {
    world
        .terrain_chunk_section_mesh_revisions(pos)
        .map(|section| (section.section_y, section.revision))
        .collect()
}

fn dirty_terrain_section_states(
    world: &WorldStore,
    upload: &TerrainUploadState,
    pos: ChunkPos,
) -> Vec<DirtyTerrainSection> {
    let cached = upload.meshes.get(&pos);
    world
        .terrain_chunk_section_mesh_revisions(pos)
        .filter_map(|section| {
            let current = cached.and_then(|mesh| {
                mesh.section_mesh_revisions
                    .iter()
                    .find(|(y, _)| *y == section.section_y)
                    .map(|&(_, revision)| revision)
            });
            if current == Some(section.revision) {
                return None;
            }
            let dirty_from_player = section.player_changed_revision != 0
                && section_is_dirty_from_player(
                    section.player_changed_revision,
                    current.unwrap_or_else(|| {
                        upload
                            .compiled_player_sections
                            .get(&SectionPosition {
                                x: pos.x,
                                y: section.section_y,
                                z: pos.z,
                            })
                            .copied()
                            .unwrap_or(0)
                    }),
                );
            Some(DirtyTerrainSection {
                section_y: section.section_y,
                revision: section.revision,
                dirty_from_player,
            })
        })
        .collect()
}

#[cfg(test)]
fn dirty_terrain_sections(
    world: &WorldStore,
    upload: &TerrainUploadState,
    pos: ChunkPos,
) -> Vec<(i32, u64)> {
    dirty_terrain_section_states(world, upload, pos)
        .into_iter()
        .map(|section| (section.section_y, section.revision))
        .collect()
}

fn section_fade_duration(position: SectionPosition, camera_position: [f32; 3]) -> Duration {
    let camera_block = camera_position.map(|coordinate| coordinate.floor() as i32);
    let center = [position.x, position.y, position.z]
        .map(|coordinate| coordinate.saturating_mul(16).saturating_add(8));
    let distance_squared = center
        .into_iter()
        .zip(camera_block)
        .map(|(coordinate, camera)| (i128::from(coordinate) - i128::from(camera)).pow(2))
        .sum::<i128>();
    if distance_squared < 768 {
        Duration::ZERO
    } else {
        VANILLA_SECTION_FADE_DURATION
    }
}

fn install_resident_section(
    upload: &mut TerrainUploadState,
    pos: ChunkPos,
    visibility: TerrainSectionVisibility,
    camera_position: [f32; 3],
    now: Instant,
) {
    let position = SectionPosition {
        x: pos.x,
        y: visibility.section_y,
        z: pos.z,
    };
    let changed = match upload.section_residency.entry(position) {
        Entry::Vacant(entry) => {
            entry.insert(TerrainSectionResidency {
                uploaded_at: now,
                fade_duration: section_fade_duration(position, camera_position),
                compiled_visibility: visibility,
            });
            true
        }
        Entry::Occupied(mut entry) => {
            let section = entry.get_mut();
            let changed = section.compiled_visibility != visibility;
            if section.compiled_visibility.loaded_empty && !visibility.loaded_empty {
                section.fade_duration = Duration::ZERO;
                upload
                    .entity_visits
                    .entry(position)
                    .and_modify(|visit| {
                        if let EntitySectionVisit::VisibleAt(at) = visit {
                            *at = (*at).min(now);
                        }
                    })
                    .or_insert(EntitySectionVisit::PreviouslyEmpty);
            }
            section.compiled_visibility = visibility;
            changed
        }
    };
    upload.mesh_section_updates.insert(position);
    if changed {
        upload.occlusion_revision += 1;
        upload
            .occlusion_section_updates
            .entry(position)
            .and_modify(|update| update.visibility = Some(visibility))
            .or_insert(TerrainOcclusionSectionUpdate {
                visibility: Some(visibility),
                removed: false,
            });
    }
}

fn remove_resident_section(upload: &mut TerrainUploadState, position: SectionPosition) {
    upload.section_residency.remove(&position);
    upload.entity_visits.remove(&position);
    upload.occlusion_graph.forget_empty(position);
    upload.mesh_section_updates.insert(position);
    upload.occlusion_revision += 1;
    upload.occlusion_section_updates.insert(
        position,
        TerrainOcclusionSectionUpdate {
            visibility: None,
            removed: true,
        },
    );
}

fn section_residency_is_consumer_visible(
    residency: &TerrainCoordinateMap<SectionPosition, TerrainSectionResidency>,
    position: SectionPosition,
    now: Instant,
) -> bool {
    residency
        .get(&position)
        .is_some_and(|section| section.visibility(now) >= VANILLA_SECTION_CONSUMER_VISIBILITY)
}

fn terrain_mesh_job_key(
    world: &WorldStore,
    upload: &TerrainUploadState,
    pos: ChunkPos,
) -> TerrainMeshJobKey {
    TerrainMeshJobKey {
        level_generation: world.client_level_generation(),
        mesh_generation: upload.mesh_generation,
        pos,
        mesh_revision: world.terrain_chunk_mesh_revision(pos),
    }
}

fn terrain_section_is_nearby(pos: ChunkPos, section_y: i32, camera_position: [f32; 3]) -> bool {
    let center = [
        pos.x.saturating_mul(16).saturating_add(8),
        section_y.saturating_mul(16).saturating_add(8),
        pos.z.saturating_mul(16).saturating_add(8),
    ];
    let camera_block = camera_position.map(|coordinate| coordinate.floor() as i32);
    center
        .into_iter()
        .zip(camera_block)
        .map(|(coordinate, camera)| (i128::from(coordinate) - i128::from(camera)).pow(2))
        .sum::<i128>()
        < 768
}

fn section_rebuilds_synchronously(
    policy: TerrainPrioritizeChunkUpdates,
    pos: ChunkPos,
    section: DirtyTerrainSection,
    camera_position: [f32; 3],
) -> bool {
    match policy {
        TerrainPrioritizeChunkUpdates::None => false,
        TerrainPrioritizeChunkUpdates::PlayerAffected => section.dirty_from_player,
        TerrainPrioritizeChunkUpdates::Nearby => {
            section.dirty_from_player
                || terrain_section_is_nearby(pos, section.section_y, camera_position)
        }
    }
}

fn terrain_mesh_work(
    world: &WorldStore,
    upload: &TerrainUploadState,
    pos: ChunkPos,
) -> TerrainMeshWork {
    let dirty_sections = dirty_terrain_section_states(world, upload, pos);
    let key = terrain_mesh_job_key(world, upload, pos);
    TerrainMeshWork {
        key,
        dirty_sections,
    }
}

fn queue_terrain_chunk_mesh(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    work: TerrainMeshWork,
    policy: TerrainPrioritizeChunkUpdates,
    camera_position: [f32; 3],
    shared_quad_indices: bool,
) -> TerrainMeshQueueOutcome {
    let TerrainMeshWork {
        key,
        dirty_sections,
    } = work;
    let pos = key.pos;
    let synchronous_sections = dirty_sections
        .iter()
        .copied()
        .filter(|section| section_rebuilds_synchronously(policy, pos, *section, camera_position))
        .map(|section| (section.section_y, section.revision))
        .collect::<Vec<_>>();
    let rebuild_synchronously = !synchronous_sections.is_empty();
    if !rebuild_synchronously
        && upload
            .mesh_worker
            .as_ref()
            .is_some_and(TerrainMeshPool::is_full)
    {
        return TerrainMeshQueueOutcome::Full;
    }
    let sections = if rebuild_synchronously {
        synchronous_sections
    } else {
        dirty_sections
            .into_iter()
            .map(|section| (section.section_y, section.revision))
            .collect()
    };
    let cardinal_lighting = terrain_cardinal_lighting(world);
    let target = snapshot_chunk_from_world(world, pos, cardinal_lighting)
        .expect("resident terrain mesh target");
    let neighborhood = chunk_neighborhood(pos)
        .filter(|neighbor| *neighbor != pos)
        .filter_map(|neighbor| snapshot_chunk_from_world(world, neighbor, cardinal_lighting))
        .collect();
    let context = world.terrain_block_render_context();
    if upload
        .block_table
        .as_ref()
        .is_some_and(|table| !table.is_current(textures, &context))
    {
        upload.block_table = None;
    }
    let table = upload
        .block_table
        .get_or_insert_with(|| {
            Arc::new(TerrainBlockRenderTable::from_resources(
                TerrainResourceSource::new(textures, context),
            ))
        })
        .clone();
    let queued_sections = sections.len();
    let job = TerrainMeshJob {
        key,
        target,
        neighborhood,
        table,
        camera_position,
        sections,
        shared_quad_indices,
    };
    if rebuild_synchronously {
        let result = build_terrain_mesh_job(job);
        return TerrainMeshQueueOutcome::SynchronouslyCompiled {
            renderer_dirty: accept_terrain_mesh_result(world, upload, result),
        };
    }
    upload
        .mesh_worker
        .get_or_insert_with(TerrainMeshPool::spawn)
        .submit(job);
    upload.pending_mesh_jobs.insert(pos, key);
    upload.mesh_worker_counters.queued_jobs =
        upload.mesh_worker_counters.queued_jobs.saturating_add(1);
    upload.mesh_worker_counters.queued_sections = upload
        .mesh_worker_counters
        .queued_sections
        .saturating_add(queued_sections as u64);
    update_mesh_worker_queue_depth(upload);
    TerrainMeshQueueOutcome::Async
}

fn drain_terrain_mesh_results(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    cache_work_started: Instant,
) -> bool {
    let mut renderer_dirty = false;
    while cache_work_started.elapsed() < TERRAIN_CACHE_WORK_BUDGET_PER_FRAME {
        let Some(result) = upload
            .mesh_worker
            .as_mut()
            .and_then(TerrainMeshPool::try_take)
        else {
            break;
        };
        renderer_dirty |= accept_terrain_mesh_result(world, upload, result);
    }
    update_mesh_worker_queue_depth(upload);
    renderer_dirty
}

fn accept_terrain_mesh_result(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    mut result: TerrainMeshResult,
) -> bool {
    let mesh_camera_position = result.camera_position;
    upload.mesh_worker_counters.completed_jobs =
        upload.mesh_worker_counters.completed_jobs.saturating_add(1);
    upload.mesh_worker_counters.completed_sections = upload
        .mesh_worker_counters
        .completed_sections
        .saturating_add(result.sections.len() as u64);
    upload.mesh_worker_counters.recent_build_nanos = result.build_nanos;
    upload.mesh_worker_counters.peak_build_nanos = upload
        .mesh_worker_counters
        .peak_build_nanos
        .max(result.build_nanos);
    upload.mesh_worker_counters.recent_build_sections = result.sections.len();
    upload.mesh_worker_counters.peak_build_sections = upload
        .mesh_worker_counters
        .peak_build_sections
        .max(result.sections.len());
    if upload.pending_mesh_jobs.get(&result.key.pos) == Some(&result.key) {
        upload.pending_mesh_jobs.remove(&result.key.pos);
    }
    let current = terrain_mesh_job_key(world, upload, result.key.pos);
    let valid = result.key == current && world.probe_chunk(result.key.pos).is_some();
    if !valid {
        upload.mesh_worker_counters.stale_jobs =
            upload.mesh_worker_counters.stale_jobs.saturating_add(1);
        upload.mesh_worker_counters.stale_sections = upload
            .mesh_worker_counters
            .stale_sections
            .saturating_add(result.sections.len() as u64);
        return false;
    }
    upload.next_resident_revision = upload.next_resident_revision.saturating_add(1);
    let resident_revision = upload.next_resident_revision;
    let track_residency = upload.residency_key == Some(TerrainResidencyKey::of(world, upload));
    let previous = upload.meshes.get(&result.key.pos);
    let was_current = previous.is_some_and(|mesh| mesh.revision == current.mesh_revision);
    let previous_section_ys = previous
        .map(|cached| {
            cached
                .resident_revisions
                .iter()
                .map(|&(section_y, _)| section_y)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let dirty_section_ys = result
        .sections
        .iter()
        .map(|&(section_y, _)| section_y)
        .collect::<FxHashSet<_>>();
    let sections = world.terrain_chunk_section_mesh_revisions(result.key.pos);
    let mut current_sections = Vec::with_capacity(sections.len());
    for section in sections {
        current_sections.push((section.section_y, section.revision));
        if section.player_changed_revision != 0 && dirty_section_ys.contains(&section.section_y) {
            upload.compiled_player_sections.insert(
                SectionPosition {
                    x: result.key.pos.x,
                    y: section.section_y,
                    z: result.key.pos.z,
                },
                section.revision,
            );
        }
    }
    let cached = upload
        .meshes
        .entry(result.key.pos)
        .or_insert_with(|| CachedTerrainMesh {
            revision: 0,
            section_mesh_revisions: Vec::new(),
            resident_revisions: Vec::new(),
            camera_position: result.camera_position,
            layers: TerrainSectionMeshLayers::default(),
        });
    let visibility_updates = std::mem::take(&mut result.layers.visibility);
    merge_terrain_section_layers(&mut cached.layers, result.layers, &dirty_section_ys);
    cached.layers.source_sections = current_sections.len();
    cached.section_mesh_revisions.retain(|(section_y, _)| {
        current_sections
            .iter()
            .any(|(current_y, _)| current_y == section_y)
    });
    for &(section_y, revision) in &result.sections {
        if let Some((_, cached_revision)) = cached
            .section_mesh_revisions
            .iter_mut()
            .find(|(cached_y, _)| *cached_y == section_y)
        {
            *cached_revision = revision;
        } else {
            cached.section_mesh_revisions.push((section_y, revision));
        }
    }
    cached
        .section_mesh_revisions
        .sort_unstable_by_key(|(section_y, _)| *section_y);
    cached.revision = if cached.section_mesh_revisions == current_sections {
        result.key.mesh_revision
    } else {
        0
    };
    cached.resident_revisions.retain(|(section_y, _)| {
        cached
            .section_mesh_revisions
            .iter()
            .any(|(current_y, _)| current_y == section_y)
    });
    for &(section_y, _) in &cached.section_mesh_revisions {
        if let Some((_, revision)) = cached
            .resident_revisions
            .iter_mut()
            .find(|(resident_y, _)| *resident_y == section_y)
        {
            if dirty_section_ys.contains(&section_y) {
                *revision = resident_revision;
            }
        } else {
            cached
                .resident_revisions
                .push((section_y, resident_revision));
        }
    }
    cached
        .resident_revisions
        .sort_unstable_by_key(|(section_y, _)| *section_y);
    cached.camera_position = result.camera_position;
    if track_residency {
        let counters = &mut upload.mesh_worker_counters;
        if was_current {
            counters.resident_chunks -= 1;
            counters.resident_sections -= previous_section_ys.len();
        }
        if cached.revision == current.mesh_revision {
            counters.resident_chunks += 1;
            counters.resident_sections += cached.resident_revisions.len();
        }
    }
    let removed_sections = previous_section_ys
        .into_iter()
        .filter(|section_y| {
            !cached
                .resident_revisions
                .iter()
                .any(|(current_y, _)| current_y == section_y)
        })
        .collect::<Vec<_>>();
    let now = bbb_client_clock::now();
    for visibility in visibility_updates {
        install_resident_section(
            upload,
            result.key.pos,
            visibility,
            mesh_camera_position,
            now,
        );
    }
    for section_y in removed_sections {
        remove_resident_section(
            upload,
            SectionPosition {
                x: result.key.pos.x,
                y: section_y,
                z: result.key.pos.z,
            },
        );
    }
    true
}

fn merge_terrain_section_layers(
    cached: &mut TerrainSectionMeshLayers,
    replacement: TerrainSectionMeshLayers,
    dirty_section_ys: &FxHashSet<i32>,
) {
    fn merge_layer(
        cached: &mut Vec<TerrainSectionMesh>,
        replacement: Vec<TerrainSectionMesh>,
        dirty_section_ys: &FxHashSet<i32>,
    ) {
        cached.retain(|section| !dirty_section_ys.contains(&section.section_y));
        cached.extend(replacement);
        cached.sort_unstable_by_key(|section| section.section_y);
    }

    merge_layer(&mut cached.opaque, replacement.opaque, dirty_section_ys);
    merge_layer(&mut cached.cutout, replacement.cutout, dirty_section_ys);
    merge_layer(
        &mut cached.translucent,
        replacement.translucent,
        dirty_section_ys,
    );
}

fn cancel_queued_terrain_mesh_jobs(upload: &mut TerrainUploadState) {
    if let Some(pool) = upload.mesh_worker.as_ref() {
        pool.cancel_queued();
    }
}

fn update_mesh_worker_queue_depth(upload: &mut TerrainUploadState) {
    let depth = upload
        .mesh_worker
        .as_ref()
        .map_or(0, TerrainMeshPool::outstanding);
    upload.mesh_worker_counters.queue_depth = depth;
    upload.mesh_worker_counters.peak_queue_depth =
        upload.mesh_worker_counters.peak_queue_depth.max(depth);
}

fn publish_mesh_worker_counters(
    world: &WorldStore,
    renderer: &mut Renderer,
    upload: &mut TerrainUploadState,
) {
    let counters = terrain_residency_counters(world, upload);
    upload.cache_ready = counters.cache_ready;
    renderer.set_terrain_mesh_worker_counters(counters);
}

fn refresh_terrain_residency(world: &WorldStore, upload: &mut TerrainUploadState) {
    let key = TerrainResidencyKey::of(world, upload);
    if upload.residency_key == Some(key) {
        return;
    }
    // This revision scan also locates the nearest dirty column in scheduling order.
    // Accepted meshes update the totals until the next revision.
    let mut chunks = 0;
    let mut sections = 0;
    let mut first_dirty = upload.selected_chunks.len();
    for (index, &pos) in upload.selected_chunks.iter().enumerate() {
        if let Some(mesh) = upload
            .meshes
            .get(&pos)
            .filter(|mesh| mesh.revision == world.terrain_chunk_mesh_revision(pos))
        {
            chunks += 1;
            sections += mesh.resident_revisions.len();
        } else {
            first_dirty = first_dirty.min(index);
        }
    }
    upload.selected_cursor = first_dirty;
    upload.mesh_worker_counters.resident_chunks = chunks;
    upload.mesh_worker_counters.resident_sections = sections;
    upload.residency_key = Some(key);
}

fn terrain_residency_counters(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
) -> TerrainMeshWorkerCounters {
    refresh_terrain_residency(world, upload);
    let mut counters = upload.mesh_worker_counters;
    counters.selected_chunks = upload.selected_chunks.len();
    counters.selected_sections = counters.selected_chunks * world.dimension().section_count();
    counters.pending_jobs = upload.pending_mesh_jobs.len();
    counters.cache_ready = counters.selected_chunks != 0
        && counters.resident_chunks == counters.selected_chunks
        && counters.resident_sections == counters.selected_sections
        && counters.pending_jobs == 0;
    counters
}

fn upload_cached_terrain(
    renderer: &mut Renderer,
    upload: &mut TerrainUploadState,
    sections_per_column: usize,
    full_sync: bool,
) {
    let level_generation = upload.observed_level_generation;
    let source_sections = upload.meshes.len() * sections_per_column;
    let touched_columns: Vec<_> = if full_sync {
        let cached = upload
            .selected_chunks
            .iter()
            .filter_map(|pos| upload.meshes.get(pos).map(|cached| (*pos, cached)))
            .collect::<Vec<_>>();
        renderer.synchronize_terrain_section_mesh_layer_refs(
            source_sections,
            cached.iter().map(|(pos, cached)| {
                (
                    TerrainChunkMeshOwner {
                        level_generation,
                        chunk_x: pos.x,
                        chunk_z: pos.z,
                    },
                    cached.resident_revisions.as_slice(),
                    &cached.layers,
                    cached.camera_position,
                )
            }),
        );
        cached.into_iter().map(|(pos, _)| pos).collect()
    } else {
        let mut changes = Vec::new();
        let mut removed = Vec::new();
        let retries = renderer.terrain_retry_owners();
        let updates = upload
            .mesh_section_updates
            .iter()
            .map(|position| TerrainSectionMeshOwner {
                chunk: TerrainChunkMeshOwner {
                    level_generation,
                    chunk_x: position.x,
                    chunk_z: position.z,
                },
                section_y: position.y,
            })
            .chain(retries);
        for owner in updates {
            let pos = ChunkPos {
                x: owner.chunk.chunk_x,
                z: owner.chunk.chunk_z,
            };
            let current = owner.chunk.level_generation == upload.observed_level_generation;
            let cached = upload.meshes.get(&pos).filter(|_| current);
            let Some((cached, (_, revision))) = cached.and_then(|cached| {
                cached
                    .resident_revisions
                    .iter()
                    .find(|(section_y, _)| *section_y == owner.section_y)
                    .map(|revision| (cached, revision))
            }) else {
                removed.push(owner);
                continue;
            };
            changes.push((owner, *revision, &cached.layers, cached.camera_position));
        }
        changes.sort_unstable_by_key(|(owner, _, _, _)| *owner);
        changes.dedup_by_key(|(owner, _, _, _)| *owner);
        renderer.update_terrain_section_mesh_layer_refs(
            source_sections,
            changes.iter().copied(),
            removed,
        );
        changes
            .chunk_by(|a, b| a.0.chunk == b.0.chunk)
            .map(|sections| ChunkPos {
                x: sections[0].0.chunk.chunk_x,
                z: sections[0].0.chunk.chunk_z,
            })
            .collect()
    };

    for pos in touched_columns {
        let cached = upload.meshes.get_mut(&pos).unwrap();
        let chunk = TerrainChunkMeshOwner {
            level_generation,
            chunk_x: pos.x,
            chunk_z: pos.z,
        };
        let revisions = &cached.resident_revisions;
        let layers = &mut cached.layers;
        // Failed uploads retain their CPU payload for retry. Successful uploads already
        // keep the translucent sort inputs in GPU residency, so only visibility stays here.
        for layer in [
            &mut layers.opaque,
            &mut layers.cutout,
            &mut layers.translucent,
        ] {
            layer.retain(|section| {
                let section_y = section.section_y;
                let uploaded = revisions
                    .binary_search_by_key(&section_y, |&(y, _)| y)
                    .is_ok_and(|i| {
                        renderer.terrain_section_mesh_is_uploaded(
                            &TerrainSectionMeshOwner { chunk, section_y },
                            revisions[i].1,
                        )
                    });
                !uploaded
            });
        }
    }
    upload.mesh_section_updates.clear();
}

fn refresh_terrain_visibility(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    context: TerrainVisibilityContext,
) -> bool {
    let Some(camera_pose) = context.camera_pose else {
        upload
            .occlusion_graph
            .replace_preserving_visits(TerrainSectionOcclusionGraph::default());
        upload.occlusion_rebuild_key = None;
        upload.occlusion_graph_input_key = None;
        publish_terrain_frustum_visibility(upload, context);
        return true;
    };
    let camera_position = [
        f64::from(camera_pose.position[0]),
        f64::from(camera_pose.position[1] + camera_pose.eye_height),
        f64::from(camera_pose.position[2]),
    ];
    let Some(rebuild_key) = terrain_occlusion_rebuild_key(world, upload, context, camera_position)
    else {
        upload
            .occlusion_graph
            .replace_preserving_visits(TerrainSectionOcclusionGraph::default());
        upload.occlusion_rebuild_key = None;
        upload.occlusion_graph_input_key = None;
        publish_terrain_frustum_visibility(upload, context);
        return true;
    };
    let current_input_key = terrain_occlusion_input_key(world, upload);

    let pending_key = upload.pending_occlusion_rebuild_key;
    let mut accepted_result = None;
    if pending_key.is_some() {
        let worker = upload
            .occlusion_worker
            .as_mut()
            .expect("pending terrain occlusion rebuild has a worker");
        while let Some(result) = worker.try_recv() {
            if pending_key == Some(result.key) {
                upload.pending_occlusion_rebuild_key = None;
                let pending_input_key = upload.pending_occlusion_input_key.take();
                if result.key == rebuild_key && pending_input_key.is_some() {
                    accepted_result = Some((result, pending_input_key));
                }
            }
        }
    }
    let graph_changed = accepted_result.is_some();
    if let Some((mut result, pending_input_key)) = accepted_result {
        if pending_input_key != Some(current_input_key) {
            // A reinstalled section can have a new mesh while the worker still
            // carries an empty visit from the section that occupied its coordinates.
            for (&position, update) in &upload.occlusion_section_updates {
                if update.removed {
                    result.graph.forget_empty(position);
                }
            }
        }
        upload
            .occlusion_graph
            .replace_preserving_visits(result.graph);
        upload.occlusion_rebuild_key = Some(result.key);
        upload.occlusion_graph_input_key = pending_input_key;
    }
    let partial_changed = upload.occlusion_graph.is_valid()
        && upload.occlusion_graph_input_key != Some(current_input_key);
    if partial_changed {
        refresh_terrain_occlusion_loaded_chunks(world, upload);
        upload.occlusion_graph.update_sections(
            upload
                .occlusion_section_updates
                .iter()
                .map(|(&position, update)| (position, update.visibility)),
            upload.occlusion_loaded_chunks.as_ref(),
            camera_position,
            i32::try_from(context.render_distance_chunks).unwrap_or(i32::MAX),
            world.dimension().min_section_y(),
            world.dimension().section_count(),
            context.smart_cull,
        );
        upload.occlusion_graph_input_key = Some(current_input_key);
        if upload.pending_occlusion_rebuild_key.is_none() {
            upload.occlusion_section_updates.clear();
        }
    }
    let graph_changed = graph_changed || partial_changed;
    if upload.occlusion_rebuild_key == Some(rebuild_key)
        && (upload.occlusion_graph.is_valid()
            || upload.occlusion_graph_input_key == Some(current_input_key))
    {
        // `update_renderer_from_publication` already traversed the current graph with this
        // frame's frustum before extracting entities and block entities. The upload tail only
        // needs to republish when it rebuilt or partially propagated that graph.
        if graph_changed {
            publish_terrain_frustum_visibility(upload, context);
        }
        return graph_changed;
    }
    if upload.pending_occlusion_rebuild_key.is_none() {
        refresh_terrain_occlusion_inputs(world, upload, current_input_key);
        upload
            .occlusion_worker
            .get_or_insert_with(TerrainOcclusionWorker::spawn)
            .submit(TerrainOcclusionJob {
                key: rebuild_key,
                sections: Arc::clone(&upload.occlusion_sections),
                loaded_chunks: Arc::clone(&upload.occlusion_loaded_chunks),
                camera_position,
                view_distance: i32::try_from(context.render_distance_chunks).unwrap_or(i32::MAX),
                min_section_y: world.dimension().min_section_y(),
                section_count: world.dimension().section_count(),
                smart_cull: context.smart_cull,
            });
        // The immutable full-build snapshot contains every update through
        // `current_input_key`; retain only changes arriving while it runs.
        upload.occlusion_section_updates.clear();
        upload.pending_occlusion_rebuild_key = Some(rebuild_key);
        upload.pending_occlusion_input_key = Some(current_input_key);
    }

    if graph_changed {
        publish_terrain_frustum_visibility(upload, context);
        true
    } else if upload.occlusion_graph.is_valid() {
        false
    } else {
        publish_terrain_frustum_visibility(upload, context);
        true
    }
}

fn clear_terrain_occlusion_inputs(upload: &mut TerrainUploadState) {
    upload.entity_visits.clear();
    upload.occlusion_sections_revision = None;
    upload.occlusion_loaded_key = None;
    upload.occlusion_graph_input_key = None;
    // The pending rebuild key owns the worker's exact in-flight job. A level,
    // empty-world, or resource-reload transition cannot cancel a job already
    // executing or queued in the bounded worker, so keep that ownership until
    // `refresh_terrain_visibility` drains its result. Clear the input key to mark
    // the result invalid even when its rebuild key (for example, a resource reload
    // in the same camera cell) would otherwise still match.
    upload.pending_occlusion_input_key = None;
    upload.occlusion_section_updates.clear();
    upload.occlusion_sections = Arc::default();
    upload.occlusion_loaded_chunks = Arc::default();
    upload.frustum_visibility_key = None;
    upload.frustum_visible_sections.clear();
}

fn terrain_occlusion_input_key(
    world: &WorldStore,
    upload: &TerrainUploadState,
) -> TerrainOcclusionInputKey {
    TerrainOcclusionInputKey {
        level_generation: world.client_level_generation(),
        membership_revision: world.chunk_membership_revision(),
        visibility_revision: upload.occlusion_revision,
    }
}

fn refresh_terrain_occlusion_loaded_chunks(world: &WorldStore, upload: &mut TerrainUploadState) {
    let key = (
        world.client_level_generation(),
        world.chunk_membership_revision(),
    );
    if upload.occlusion_loaded_key != Some(key) {
        upload.occlusion_loaded_chunks = Arc::new(world.chunk_positions().into_iter().collect());
        upload.occlusion_loaded_key = Some(key);
    }
}

fn refresh_terrain_occlusion_inputs(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    input_key: TerrainOcclusionInputKey,
) {
    refresh_terrain_occlusion_loaded_chunks(world, upload);
    if upload.occlusion_sections_revision == Some(input_key.visibility_revision) {
        return;
    }
    upload.occlusion_sections = Arc::new(
        upload
            .section_residency
            .iter()
            .map(|(&position, section)| ResidentSection {
                position,
                visibility: section.compiled_visibility,
            })
            .collect(),
    );
    upload.occlusion_sections_revision = Some(input_key.visibility_revision);
}

fn terrain_occlusion_rebuild_key(
    world: &WorldStore,
    _upload: &TerrainUploadState,
    context: TerrainVisibilityContext,
    camera_position: [f64; 3],
) -> Option<TerrainOcclusionRebuildKey> {
    let camera_cell = camera_position.map(|coordinate| {
        coordinate
            .is_finite()
            .then(|| (coordinate / 8.0).floor() as i32)
    });
    let [Some(camera_x), Some(camera_y), Some(camera_z)] = camera_cell else {
        return None;
    };
    Some(TerrainOcclusionRebuildKey {
        level_generation: world.client_level_generation(),
        camera_cell: [camera_x, camera_y, camera_z],
        render_distance_chunks: context.render_distance_chunks,
        min_section_y: world.dimension().min_section_y(),
        section_count: world.dimension().section_count(),
        smart_cull: context.smart_cull,
    })
}

fn publish_terrain_frustum_visibility(
    upload: &mut TerrainUploadState,
    context: TerrainVisibilityContext,
) {
    let visibility_key = terrain_frustum_visibility_key(upload, context);
    let now = bbb_client_clock::now();
    let Some(frustum) = context
        .frustum()
        .filter(|_| upload.occlusion_graph.is_valid())
    else {
        let visible = selected_resident_sections(upload, context);
        upload.renderer_sections.replace(
            upload.observed_level_generation,
            visible,
            &upload.section_residency,
            now,
        );
        upload.frustum_visibility_key = Some(visibility_key);
        return;
    };
    upload
        .occlusion_graph
        .fill_visible_sections(frustum, &mut upload.frustum_visible_sections);
    upload.renderer_sections.replace(
        upload.observed_level_generation,
        upload.frustum_visible_sections.iter().copied(),
        &upload.section_residency,
        now,
    );
    upload.frustum_visibility_key = Some(visibility_key);
    let pose = context.camera_pose.unwrap();
    let camera = [
        pose.position[0],
        pose.position[1] + pose.eye_height,
        pose.position[2],
    ];
    // The previous camera's graph can draw terrain while its replacement is built.
    // Reprojecting it at the destination must not permanently admit new entities.
    if !visibility_key.admits_entity_sections {
        return;
    }
    for visible in &upload.frustum_visible_sections {
        let Some(section) = upload.section_residency.get(&visible.position) else {
            continue;
        };
        if section.compiled_visibility.loaded_empty {
            continue;
        }
        match upload.entity_visits.entry(visible.position) {
            Entry::Occupied(mut entry) => {
                if matches!(entry.get(), EntitySectionVisit::PreviouslyEmpty) {
                    entry.insert(EntitySectionVisit::VisibleAt(now));
                }
            }
            Entry::Vacant(entry) => {
                let fade = section_fade_duration(visible.position, camera);
                let delay_ms =
                    (fade.as_millis() as f32 * VANILLA_SECTION_CONSUMER_VISIBILITY).round() as u64;
                entry.insert(EntitySectionVisit::VisibleAt(
                    now + Duration::from_millis(delay_ms),
                ));
            }
        }
    }
}

fn terrain_frustum_visibility_key(
    upload: &TerrainUploadState,
    context: TerrainVisibilityContext,
) -> TerrainFrustumVisibilityKey {
    let camera_rotation_bucket = context.camera_pose.map(|pose| {
        [
            (pose.x_rot / 2.0).floor() as i32,
            (pose.y_rot / 2.0).floor() as i32,
        ]
    });
    TerrainFrustumVisibilityKey {
        graph_key: upload.occlusion_rebuild_key,
        graph_valid: upload.occlusion_graph.is_valid(),
        admits_entity_sections: context.camera_pose.is_some_and(|pose| {
            upload.occlusion_graph.is_rooted_at([
                pose.position[0],
                pose.position[1] + pose.eye_height,
                pose.position[2],
            ])
        }),
        visibility_revision: upload.occlusion_revision,
        camera_rotation_bucket,
        surface_size: context.surface_size,
        camera_fov_bits: context.camera_fov_degrees.to_bits(),
    }
}

fn selected_resident_sections(
    upload: &TerrainUploadState,
    context: TerrainVisibilityContext,
) -> Vec<VisibleSection> {
    let camera_section = context.camera_pose.and_then(|pose| {
        SectionPosition::from_block_position([
            f64::from(pose.position[0]),
            f64::from(pose.position[1] + pose.eye_height),
            f64::from(pose.position[2]),
        ])
    });
    upload
        .selected_chunks
        .iter()
        .filter_map(|pos| upload.meshes.get(pos).map(|cached| (*pos, cached)))
        .flat_map(|(pos, cached)| {
            cached
                .resident_revisions
                .iter()
                .map(move |&(section_y, _)| {
                    let position = SectionPosition {
                        x: pos.x,
                        y: section_y,
                        z: pos.z,
                    };
                    VisibleSection {
                        position,
                        nearby: camera_section.is_some_and(|camera| {
                            (i64::from(camera.x) - i64::from(position.x)).abs() <= 2
                                && (i64::from(camera.y) - i64::from(position.y)).abs() <= 2
                                && (i64::from(camera.z) - i64::from(position.z)).abs() <= 2
                        }),
                    }
                })
        })
        .collect()
}

fn chunk_neighborhood(pos: ChunkPos) -> impl Iterator<Item = ChunkPos> {
    [
        (0, 0),
        (-1, 0),
        (0, -1),
        (0, 1),
        (1, 0),
        (-1, -1),
        (-1, 1),
        (1, -1),
        (1, 1),
    ]
    .into_iter()
    .map(move |(dx, dz)| ChunkPos {
        x: pos.x + dx,
        z: pos.z + dz,
    })
}

fn terrain_upload_center(world: &WorldStore) -> ChunkPos {
    world
        .chunk_cache_center()
        .or(world.first_chunk())
        .unwrap_or_else(|| {
            world
                .chunk_positions()
                .into_iter()
                .next()
                .unwrap_or(ChunkPos { x: 0, z: 0 })
        })
}

fn chunk_distance_key(pos: ChunkPos, center: ChunkPos) -> i64 {
    let dx = i64::from(pos.x - center.x);
    let dz = i64::from(pos.z - center.z);
    dx * dx + dz * dz
}

pub fn terrain_cardinal_lighting(world: &WorldStore) -> TerrainCardinalLighting {
    match world.level_info().map(|level| level.cardinal_lighting()) {
        Some(WorldCardinalLighting::Nether) => TerrainCardinalLighting::Nether,
        _ => TerrainCardinalLighting::Default,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod initial_light_gate_tests {
    use super::*;
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
    use bbb_protocol::packets::{
        ChunkHeightmapData, LevelChunkData, LevelChunkWithLight, LightUpdateData,
    };

    fn synthetic_chunk_packet() -> LevelChunkWithLight {
        let mut sections = Encoder::new();
        for _ in 0..bbb_client_world::WorldDimension::default().section_count() {
            sections.write_i16(0);
            sections.write_i16(0);
            sections.write_u8(0);
            sections.write_var_i32(0);
            sections.write_u8(0);
            sections.write_var_i32(0);
        }
        LevelChunkWithLight {
            x: 1,
            z: -2,
            chunk_data: LevelChunkData {
                heightmaps: vec![ChunkHeightmapData {
                    kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                    data: vec![42],
                }],
                section_data: sections.into_inner(),
                block_entities: Vec::new(),
            },
            light_data: LightUpdateData {
                sky_y_mask: Vec::new(),
                block_y_mask: Vec::new(),
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: Vec::new(),
                block_updates: Vec::new(),
            },
        }
    }

    #[test]
    fn player_adjacent_dirty_chunk_schedules_before_a_distant_dirty_chunk() {
        let mut world = WorldStore::new();
        let near = world
            .insert_level_chunk_with_light(synthetic_chunk_packet())
            .unwrap();
        let mut distant_packet = synthetic_chunk_packet();
        distant_packet.x = near.x + 8;
        let distant = world.insert_level_chunk_with_light(distant_packet).unwrap();
        assert_eq!(world.poll_queued_light_updates(), 2);
        world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: f64::from(near.x) * 16.0 + 8.0,
                y: 64.0,
                z: f64::from(near.z) * 16.0 + 8.0,
            },
            ..bbb_client_world::LocalPlayerPoseState::default()
        });

        // Vanilla's flag-8 player-priority rebuild maps to the distance-first
        // target scan: both chunks need work, the player's chunk wins.
        let mut selected = world.chunk_positions();
        sort_terrain_chunks(&mut selected, near);
        assert_eq!(selected, vec![near, distant]);
        let upload = TerrainUploadState {
            selected_chunks: selected,
            ..TerrainUploadState::default()
        };
        match next_terrain_cache_work(&world, &upload, &mut 0) {
            TerrainCacheWork::QueueMesh(work) => {
                assert_eq!(work.key.pos, near);
            }
            work => panic!("expected near-chunk mesh work, got {work:?}"),
        }
    }

    #[test]
    fn mesh_targets_withhold_a_chunk_until_its_initial_light_applies() {
        let mut world = WorldStore::new();
        let pos = world
            .insert_level_chunk_with_light(synthetic_chunk_packet())
            .unwrap();
        let upload = TerrainUploadState {
            selected_chunks: vec![pos],
            ..TerrainUploadState::default()
        };

        // Vanilla only reports the chunk ready to render from its queued light
        // closure: before the poll the chunk contributes no mesh work of its own.
        assert!(world.terrain_chunk_initial_light_pending(pos));
        assert!(matches!(
            next_terrain_cache_work(&world, &upload, &mut 0),
            TerrainCacheWork::Idle
        ));

        assert_eq!(world.poll_queued_light_updates(), 1);
        assert!(!world.terrain_chunk_initial_light_pending(pos));
        assert!(matches!(
            next_terrain_cache_work(&world, &upload, &mut 0),
            TerrainCacheWork::QueueMesh(work) if work.key.pos == pos
        ));
    }
}
