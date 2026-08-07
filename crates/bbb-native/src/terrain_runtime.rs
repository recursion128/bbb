use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use bbb_renderer::terrain::{
    TerrainCardinalLighting, TerrainCell, TerrainChunkSnapshot, TerrainFluid, TerrainFluidKind,
    TerrainLight, TerrainMaterialClass, TerrainSectionMeshLayers, TerrainSectionVisibility,
    TerrainSkipRendering, TerrainTextureAtlas, TerrainTint,
};
use bbb_renderer::{
    CameraFrustum, CameraPose, TerrainChunkMeshOwner, TerrainMeshWorkerCounters,
    TerrainSectionMeshOwner, TerrainSectionRenderState,
};
use bbb_world::{ChunkPos, WorldCardinalLighting, WorldStore};

use crate::biome_tint::{block_wants_biome_blend, build_biome_blend, BiomeBlend};
use crate::camera_pose::camera_pose_from_world;

mod mesh_worker;
mod occlusion;
mod textures;
use mesh_worker::{
    TerrainMeshJob, TerrainMeshJobKey, TerrainMeshResult, TerrainMeshSubmit, TerrainMeshWorker,
};
use occlusion::{
    ChunkSectionDebugGizmos, ResidentSection, SectionPosition, TerrainCoordinateMap,
    TerrainCoordinateSet, TerrainOcclusionJob, TerrainOcclusionWorker,
    TerrainSectionOcclusionGraph,
};
pub(crate) use textures::{
    load_terrain_textures, terrain_particle_biome_blend_center, try_load_terrain_textures,
    BlockRenderPosition, TerrainParticleTintCatalog, TerrainTextureState,
};

// Snapshot extraction clones a full chunk-height block/biome input before the
// mesh worker can own it. Keep that unavoidable copy to one render-thread step
// per frame so a view-center change cannot batch several multi-section clones
// into one presentation hitch; the selected/resident target remains unchanged.
const MAX_TERRAIN_CACHE_WORK_PER_FRAME: usize = 1;
const TERRAIN_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);
const VANILLA_SECTION_FADE_DURATION: Duration = Duration::from_millis(750);
const VANILLA_SECTION_CONSUMER_VISIBILITY: f32 = 0.3;

#[derive(Debug, Clone, Copy)]
struct TerrainSectionResidency {
    uploaded_at: Instant,
    fade_duration: Duration,
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
    has_fading_sections: bool,
}

impl TerrainSectionFrameProjection {
    fn replace(
        &mut self,
        level_generation: u64,
        positions: impl IntoIterator<Item = SectionPosition>,
        residency: &TerrainCoordinateMap<SectionPosition, TerrainSectionResidency>,
        now: Instant,
    ) {
        self.revision = self.revision.wrapping_add(1);
        self.states.clear();
        self.by_position.clear();
        self.has_fading_sections = false;
        for position in positions {
            let Some(section) = residency.get(&position) else {
                continue;
            };
            let visibility = section.visibility(now).clamp(0.0, 1.0);
            self.has_fading_sections |= visibility < 1.0;
            self.states.push(TerrainSectionRenderState {
                owner: TerrainSectionMeshOwner {
                    chunk: TerrainChunkMeshOwner {
                        level_generation,
                        chunk_x: position.x,
                        chunk_z: position.z,
                    },
                    section_y: position.y,
                },
                visibility,
            });
            self.by_position.insert(position, visibility);
        }
    }

    fn refresh(
        &mut self,
        residency: &TerrainCoordinateMap<SectionPosition, TerrainSectionResidency>,
        now: Instant,
    ) -> bool {
        let previous_len = self.states.len();
        self.states.retain(|state| {
            residency.contains_key(&SectionPosition {
                x: state.owner.chunk.chunk_x,
                y: state.owner.section_y,
                z: state.owner.chunk.chunk_z,
            })
        });
        let mut changed = self.states.len() != previous_len;
        self.by_position.clear();
        self.has_fading_sections = false;
        for state in &mut self.states {
            let position = SectionPosition {
                x: state.owner.chunk.chunk_x,
                y: state.owner.section_y,
                z: state.owner.chunk.chunk_z,
            };
            let Some(section) = residency.get(&position) else {
                continue;
            };
            let visibility = section.visibility(now).clamp(0.0, 1.0);
            self.has_fading_sections |= visibility < 1.0;
            changed |= visibility.to_bits() != state.visibility.to_bits();
            state.visibility = visibility;
            self.by_position.insert(position, visibility);
        }
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }

    fn has_fading_sections(&self) -> bool {
        self.has_fading_sections
    }

    fn consumer_visible(&self, position: SectionPosition) -> bool {
        self.by_position
            .get(&position)
            .is_some_and(|visibility| *visibility >= VANILLA_SECTION_CONSUMER_VISIBILITY)
    }
}

#[derive(Debug)]
struct CachedTerrainSnapshot {
    snapshot_revision: u64,
    snapshot: Option<Arc<TerrainChunkSnapshot>>,
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
    world_revision: u64,
    resident_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerrainFrustumVisibilityKey {
    graph_key: Option<TerrainOcclusionRebuildKey>,
    graph_valid: bool,
    resident_revision: u64,
    camera_rotation_bucket: Option<[i32; 2]>,
    surface_size: [u32; 2],
    camera_fov_bits: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainCacheWork {
    RefreshSnapshot(ChunkPos),
    QueueMesh(ChunkPos),
    Idle,
}

#[derive(Debug, Default)]
pub(crate) struct TerrainUploadState {
    observed_level_generation: u64,
    observed_world_revision: u64,
    selected_chunks: Vec<ChunkPos>,
    snapshots: HashMap<ChunkPos, CachedTerrainSnapshot>,
    meshes: HashMap<ChunkPos, CachedTerrainMesh>,
    next_resident_revision: u64,
    mesh_generation: u64,
    pending_mesh_jobs: HashMap<ChunkPos, TerrainMeshJobKey>,
    mesh_worker: Option<TerrainMeshWorker>,
    mesh_atlas: Option<Arc<TerrainTextureAtlas>>,
    mesh_worker_counters: TerrainMeshWorkerCounters,
    cache_ready: bool,
    reload_all_chunks_requested: bool,
    texture_animation_tick: u64,
    last_texture_animation_at: Option<Instant>,
    section_residency: TerrainCoordinateMap<SectionPosition, TerrainSectionResidency>,
    occlusion_graph: TerrainSectionOcclusionGraph,
    occlusion_rebuild_key: Option<TerrainOcclusionRebuildKey>,
    pending_occlusion_rebuild_key: Option<TerrainOcclusionRebuildKey>,
    occlusion_input_key: Option<TerrainOcclusionInputKey>,
    occlusion_graph_input_key: Option<TerrainOcclusionInputKey>,
    pending_occlusion_input_key: Option<TerrainOcclusionInputKey>,
    occlusion_section_updates:
        TerrainCoordinateMap<SectionPosition, Option<TerrainSectionVisibility>>,
    occlusion_sections: Arc<[ResidentSection]>,
    occlusion_loaded_chunks: Arc<TerrainCoordinateSet<ChunkPos>>,
    occlusion_worker: Option<TerrainOcclusionWorker>,
    frustum_visibility_key: Option<TerrainFrustumVisibilityKey>,
    renderer_sections: TerrainSectionFrameProjection,
    reported_readiness: Option<(u64, usize, bool)>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TerrainVisibilityContext {
    pub(crate) camera_pose: Option<CameraPose>,
    pub(crate) surface_size: [u32; 2],
    pub(crate) camera_fov_degrees: f32,
    pub(crate) render_distance_chunks: u32,
    pub(crate) smart_cull: bool,
}

impl TerrainVisibilityContext {
    pub(crate) fn new(
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
        Some(CameraFrustum::from_pose(
            pose,
            aspect,
            self.camera_fov_degrees,
        ))
    }
}

impl TerrainUploadState {
    pub(crate) fn refresh_frame_visibility(&mut self, context: TerrainVisibilityContext) -> bool {
        let key = terrain_frustum_visibility_key(self, context);
        if self.frustum_visibility_key == Some(key) {
            if !self.renderer_sections.has_fading_sections() {
                return false;
            }
            return self
                .renderer_sections
                .refresh(&self.section_residency, Instant::now());
        }
        publish_terrain_frustum_visibility(self, context);
        true
    }

    pub(crate) fn renderer_sections(&self) -> &[TerrainSectionRenderState] {
        &self.renderer_sections.states
    }

    pub(crate) fn renderer_sections_revision(&self) -> u64 {
        self.renderer_sections.revision
    }

    pub(crate) fn chunk_section_debug_gizmos(
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

    pub(crate) fn has_uploaded_chunks(&self) -> bool {
        !self.meshes.is_empty()
    }

    pub(crate) fn request_reload_all_chunks(&mut self) {
        self.reload_all_chunks_requested = true;
    }

    pub(crate) fn is_player_section_compiled_and_visible(&self, world: &WorldStore) -> bool {
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
        section_residency_is_consumer_visible(&self.section_residency, position, Instant::now())
    }

    pub(crate) fn is_entity_section_compiled_and_visible(
        &self,
        world: &WorldStore,
        position: bbb_world::EntityVec3,
    ) -> bool {
        let block_y = position.y.floor() as i32;
        if !world.dimension().contains_y(block_y) {
            return true;
        }
        section_residency_is_consumer_visible(
            &self.section_residency,
            SectionPosition {
                x: (position.x.floor() as i32).div_euclid(16),
                y: block_y.div_euclid(16),
                z: (position.z.floor() as i32).div_euclid(16),
            },
            Instant::now(),
        )
    }

    pub(crate) fn is_block_section_compiled_and_visible(
        &self,
        world: &WorldStore,
        position: bbb_world::BlockPos,
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

    /// Returns a command payload only when readiness changes or a new level
    /// load becomes active. Same-dimension respawn keeps the ClientLevel but
    /// starts a fresh Vanilla LevelLoadTracker.
    pub(crate) fn take_readiness_edge(
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
    pub(crate) fn reload_all_chunks_requested(&self) -> bool {
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
    use bbb_protocol::packets::Vec3d;
    use bbb_world::{LocalPlayerPoseState, WorldDimension};

    #[test]
    fn far_section_fades_zero_threshold_to_one_at_vanilla_times() {
        let now = Instant::now();
        let far = SectionPosition { x: 8, y: 0, z: 0 };
        let section = TerrainSectionResidency {
            uploaded_at: now,
            fade_duration: section_fade_duration(far, [8.0, 8.0, 8.0]),
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
        let sections = [(0, 1)];
        let mut upload = TerrainUploadState::default();
        synchronize_chunk_section_residency(
            &mut upload,
            pos,
            &sections,
            [8.0, 8.0, 8.0],
            &HashSet::new(),
            now,
        );
        let position = SectionPosition { x: 0, y: 0, z: 0 };
        let first = upload.section_residency[&position];
        assert_eq!(first.fade_duration, Duration::ZERO);

        synchronize_chunk_section_residency(
            &mut upload,
            pos,
            &[(0, 2)],
            [256.0, 8.0, 8.0],
            &HashSet::from([0]),
            now + Duration::from_secs(1),
        );
        let recompiled = upload.section_residency[&position];
        assert_eq!(recompiled.uploaded_at, first.uploaded_at);
        assert_eq!(recompiled.fade_duration, Duration::ZERO);

        let far_pos = ChunkPos { x: 8, z: 0 };
        let far_position = SectionPosition { x: 8, y: 0, z: 0 };
        synchronize_chunk_section_residency(
            &mut upload,
            far_pos,
            &sections,
            [8.0, 8.0, 8.0],
            &HashSet::new(),
            now,
        );
        assert_eq!(
            upload.section_residency[&far_position].fade_duration,
            VANILLA_SECTION_FADE_DURATION
        );
        synchronize_chunk_section_residency(
            &mut upload,
            far_pos,
            &[(0, 2)],
            [8.0, 8.0, 8.0],
            &HashSet::from([0]),
            now + Duration::from_millis(100),
        );
        assert_eq!(
            upload.section_residency[&far_position].fade_duration,
            Duration::ZERO,
            "onSectionBecomingNonEmpty cancels an in-progress far-section fade"
        );

        synchronize_chunk_section_residency(
            &mut upload,
            pos,
            &[],
            [256.0, 8.0, 8.0],
            &HashSet::new(),
            now + Duration::from_secs(2),
        );
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
            },
        );
        let mut projection = TerrainSectionFrameProjection::default();
        projection.replace(9, [position], &residency, now + Duration::from_millis(224));
        assert_eq!(projection.revision, 1);
        assert!(projection.has_fading_sections());
        assert!(!projection.consumer_visible(position));
        assert_eq!(projection.states[0].owner.chunk.level_generation, 9);

        assert!(projection.refresh(&residency, now + Duration::from_millis(225)));
        assert_eq!(projection.revision, 2);
        assert!(projection.consumer_visible(position));
        assert_eq!(projection.states[0].visibility, 0.3);
        assert!(!projection.refresh(&residency, now + Duration::from_millis(225)));
        assert_eq!(projection.revision, 2);

        residency.clear();
        assert!(projection.refresh(&residency, now + Duration::from_millis(226)));
        assert_eq!(projection.revision, 3);
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
            },
        );
        upload
            .renderer_sections
            .replace(1, [position], &upload.section_residency, now);

        assert!(!upload.is_player_section_compiled_and_visible(&world));
        assert!(!upload.is_entity_section_compiled_and_visible(
            &world,
            bbb_world::EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
        ));
        assert!(!upload.is_block_section_compiled_and_visible(
            &world,
            bbb_world::BlockPos { x: 1, y: 64, z: 1 },
        ));

        upload
            .section_residency
            .get_mut(&position)
            .unwrap()
            .uploaded_at = now - Duration::from_millis(300);
        assert!(upload
            .renderer_sections
            .refresh(&upload.section_residency, now));
        assert!(upload.is_player_section_compiled_and_visible(&world));
        assert!(upload.is_entity_section_compiled_and_visible(
            &world,
            bbb_world::EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
        ));
        assert!(upload.is_block_section_compiled_and_visible(
            &world,
            bbb_world::BlockPos { x: 1, y: 64, z: 1 },
        ));
    }

    #[test]
    fn production_compile_order_keeps_vanilla_default_none_policy() {
        let source = include_str!("terrain_runtime.rs");
        let owner = ["fn queue_terrain_", "chunk_mesh("].concat();
        let queue_start = source.find(&owner).expect("terrain queue owner");
        let next_owner = ["\nfn drain_terrain_", "mesh_results("].concat();
        let queue_end = source[queue_start..]
            .find(&next_owner)
            .map(|offset| queue_start + offset)
            .expect("terrain queue owner end");
        let queue = &source[queue_start..queue_end];
        assert!(queue.contains("priority: (chunk_distance_key(pos, center), pos.x, pos.z)"));
        assert!(!queue.contains("local_block_predictions"));
        assert!(!include_str!("terrain_runtime/mesh_worker.rs").contains("priority: (u8,"));
    }
}

pub(crate) fn maybe_upload_terrain_texture_animation(
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
    _textures: &mut TerrainTextureState,
) {
    let Some((previous_tick, tick)) = advance_texture_animation_tick(upload, Instant::now()) else {
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

pub(crate) fn maybe_upload_decoded_terrain(
    world: &WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    visibility: TerrainVisibilityContext,
) {
    let mesh_worker_counters_before = upload.mesh_worker_counters;
    let mut renderer_dirty = drain_terrain_mesh_results(world, upload);
    let mesh_worker_counters_changed = upload.mesh_worker_counters != mesh_worker_counters_before;
    let level_generation_changed =
        upload.observed_level_generation != world.client_level_generation();
    let level_changed =
        synchronize_terrain_level_generation(upload, world.client_level_generation());
    if world.chunk_count() == 0 {
        let terrain_cleared = synchronize_empty_terrain_upload(upload, world.terrain_revision());
        if renderer_dirty || level_changed || terrain_cleared {
            upload_cached_terrain(renderer, upload, true);
        }
        refresh_terrain_visibility(world, upload, visibility);
        publish_mesh_worker_counters(world, renderer, upload);
        return;
    }

    if terrain_cache_selection_is_stable(world, upload, level_changed) {
        if renderer_dirty {
            upload_cached_terrain(renderer, upload, false);
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

    let center = terrain_upload_center(world);
    let selected_chunks = selected_terrain_chunks(world, center);
    let selected: HashSet<_> = selected_chunks.iter().copied().collect();
    let loaded: HashSet<_> = world.chunk_positions().into_iter().collect();
    let required_snapshots: HashSet<_> = selected_chunks
        .iter()
        .copied()
        .flat_map(chunk_neighborhood)
        .filter(|pos| loaded.contains(pos))
        .collect();

    renderer_dirty |= level_changed;
    let renderer_full_sync = level_generation_changed || upload.reload_all_chunks_requested;
    if upload.reload_all_chunks_requested {
        renderer_dirty |= !upload.meshes.is_empty();
        cancel_queued_terrain_mesh_jobs(upload);
        upload.mesh_generation = upload.mesh_generation.saturating_add(1);
        upload.mesh_atlas = None;
        upload.snapshots.clear();
        upload.meshes.clear();
        upload.section_residency.clear();
        upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
        upload.occlusion_rebuild_key = None;
        upload.pending_occlusion_rebuild_key = None;
        clear_terrain_occlusion_inputs(upload);
        upload.renderer_sections = TerrainSectionFrameProjection::default();
        upload.cache_ready = false;
        upload.reload_all_chunks_requested = false;
    }
    upload
        .snapshots
        .retain(|pos, _| required_snapshots.contains(pos));
    let removed_sections = upload
        .meshes
        .iter()
        .filter(|(pos, _)| !selected.contains(pos))
        .flat_map(|(pos, cached)| {
            cached
                .resident_revisions
                .iter()
                .map(move |&(section_y, _)| SectionPosition {
                    x: pos.x,
                    y: section_y,
                    z: pos.z,
                })
        })
        .collect::<Vec<_>>();
    for position in removed_sections {
        upload.occlusion_section_updates.insert(position, None);
        upload.section_residency.remove(&position);
    }
    let resident_before = upload.meshes.len();
    upload.meshes.retain(|pos, _| selected.contains(pos));
    renderer_dirty |= upload.meshes.len() != resident_before;
    upload.selected_chunks = selected_chunks;

    for _ in 0..MAX_TERRAIN_CACHE_WORK_PER_FRAME {
        match next_terrain_cache_work(world, upload, &loaded) {
            TerrainCacheWork::RefreshSnapshot(pos) => {
                refresh_terrain_snapshot(world, upload, textures, pos);
            }
            TerrainCacheWork::QueueMesh(pos) => {
                let queue_full_before = upload.mesh_worker_counters.queue_full;
                renderer_dirty |= queue_terrain_chunk_mesh(world, upload, textures, pos);
                if upload.mesh_worker_counters.queue_full != queue_full_before {
                    break;
                }
            }
            TerrainCacheWork::Idle => break,
        }
    }

    if renderer_dirty {
        upload_cached_terrain(renderer, upload, renderer_full_sync);
    }
    refresh_terrain_visibility(world, upload, visibility);
    publish_mesh_worker_counters(world, renderer, upload);
    upload.observed_world_revision = world.terrain_revision();
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
    upload.selected_chunks.clear();
    upload.snapshots.clear();
    upload.meshes.clear();
    upload.reload_all_chunks_requested = false;
    upload.section_residency.clear();
    upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
    upload.occlusion_rebuild_key = None;
    upload.pending_occlusion_rebuild_key = None;
    clear_terrain_occlusion_inputs(upload);
    upload.renderer_sections = TerrainSectionFrameProjection::default();
    upload.cache_ready = false;
    clear_renderer
}

fn synchronize_empty_terrain_upload(upload: &mut TerrainUploadState, world_revision: u64) -> bool {
    let clear_renderer = !upload.meshes.is_empty();
    cancel_queued_terrain_mesh_jobs(upload);
    upload.observed_world_revision = world_revision;
    upload.selected_chunks.clear();
    upload.snapshots.clear();
    upload.meshes.clear();
    upload.reload_all_chunks_requested = false;
    upload.section_residency.clear();
    upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
    upload.occlusion_rebuild_key = None;
    upload.pending_occlusion_rebuild_key = None;
    clear_terrain_occlusion_inputs(upload);
    upload.renderer_sections = TerrainSectionFrameProjection::default();
    upload.cache_ready = false;
    clear_renderer
}

fn selected_terrain_chunks(world: &WorldStore, center: ChunkPos) -> Vec<ChunkPos> {
    let mut positions = world.chunk_positions();
    positions.sort_by_key(|pos| (chunk_distance_key(*pos, center), pos.x, pos.z));
    positions
}

fn next_terrain_cache_work(
    world: &WorldStore,
    upload: &TerrainUploadState,
    loaded: &HashSet<ChunkPos>,
) -> TerrainCacheWork {
    for &target in &upload.selected_chunks {
        // Vanilla only reports a chunk ready to render from its queued light
        // closure: withhold the chunk's own mesh until its packet-load light
        // applies (a fresh chunk stays invisible, a re-sent chunk keeps its
        // previous mesh). The chunk still serves as neighbor snapshot context.
        if world.terrain_chunk_initial_light_pending(target) {
            continue;
        }
        let mesh_revision = world.terrain_chunk_mesh_revision(target);
        if upload
            .meshes
            .get(&target)
            .is_some_and(|cached| cached.revision == mesh_revision)
        {
            continue;
        }

        let mut neighborhood: Vec<_> = chunk_neighborhood(target)
            .filter(|pos| loaded.contains(pos))
            .collect();
        neighborhood.sort_by_key(|pos| (chunk_distance_key(*pos, target), pos.x, pos.z));
        for pos in neighborhood {
            let snapshot_revision = world.terrain_chunk_snapshot_revision(pos);
            if !upload
                .snapshots
                .get(&pos)
                .is_some_and(|cached| cached.snapshot_revision == snapshot_revision)
            {
                return TerrainCacheWork::RefreshSnapshot(pos);
            }
        }
        let key = terrain_mesh_job_key(world, upload, target);
        if upload.pending_mesh_jobs.get(&target) == Some(&key) {
            continue;
        }
        return TerrainCacheWork::QueueMesh(target);
    }
    TerrainCacheWork::Idle
}

fn refresh_terrain_snapshot(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    pos: ChunkPos,
) {
    let snapshot_revision = world.terrain_chunk_snapshot_revision(pos);
    let snapshot = world.extract_terrain_render_chunk(pos).map(|snapshot| {
        let biome_sampler = world.chunk_biome_sampler(pos);
        Arc::new(convert_terrain_snapshot(
            snapshot,
            textures,
            terrain_cardinal_lighting(world),
            &biome_sampler,
        ))
    });
    upload.snapshots.insert(
        pos,
        CachedTerrainSnapshot {
            snapshot_revision,
            snapshot,
        },
    );
}

fn terrain_section_mesh_revisions(world: &WorldStore, pos: ChunkPos) -> Vec<(i32, u64)> {
    world
        .terrain_chunk_section_mesh_revisions(pos)
        .map(|section| (section.section_y, section.revision))
        .collect()
}

fn dirty_terrain_sections(
    world: &WorldStore,
    upload: &TerrainUploadState,
    pos: ChunkPos,
) -> Vec<(i32, u64)> {
    let current = terrain_section_mesh_revisions(world, pos);
    let cached = upload.meshes.get(&pos);
    current
        .into_iter()
        .filter(|&(section_y, revision)| {
            !cached.is_some_and(|cached| {
                cached
                    .section_mesh_revisions
                    .iter()
                    .any(|&(cached_y, cached_revision)| {
                        cached_y == section_y && cached_revision == revision
                    })
            })
        })
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

fn synchronize_chunk_section_residency(
    upload: &mut TerrainUploadState,
    pos: ChunkPos,
    sections: &[(i32, u64)],
    camera_position: [f32; 3],
    newly_non_empty: &HashSet<i32>,
    now: Instant,
) {
    let desired: HashSet<_> = sections.iter().map(|&(section_y, _)| section_y).collect();
    upload.section_residency.retain(|position, _| {
        position.x != pos.x || position.z != pos.z || desired.contains(&position.y)
    });
    for &section_y in &desired {
        let position = SectionPosition {
            x: pos.x,
            y: section_y,
            z: pos.z,
        };
        let section =
            upload
                .section_residency
                .entry(position)
                .or_insert_with(|| TerrainSectionResidency {
                    uploaded_at: now,
                    fade_duration: section_fade_duration(position, camera_position),
                });
        if newly_non_empty.contains(&section_y) {
            section.fade_duration = Duration::ZERO;
        }
    }
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
    let section_count = dirty_terrain_sections(world, upload, pos).len();
    TerrainMeshJobKey {
        level_generation: world.client_level_generation(),
        mesh_generation: upload.mesh_generation,
        pos,
        mesh_revision: world.terrain_chunk_mesh_revision(pos),
        section_count,
    }
}

fn queue_terrain_chunk_mesh(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    pos: ChunkPos,
) -> bool {
    let key = terrain_mesh_job_key(world, upload, pos);
    let sections = dirty_terrain_sections(world, upload, pos);
    debug_assert_eq!(key.section_count, sections.len());
    let target = upload
        .snapshots
        .get(&pos)
        .and_then(|cached| cached.snapshot.as_ref())
        .cloned();
    let Some(target) = target else {
        upload.next_resident_revision = upload.next_resident_revision.saturating_add(1);
        let resident_revision = upload.next_resident_revision;
        let section_mesh_revisions = terrain_section_mesh_revisions(world, pos);
        for &(section_y, _) in &section_mesh_revisions {
            upload.occlusion_section_updates.insert(
                SectionPosition {
                    x: pos.x,
                    y: section_y,
                    z: pos.z,
                },
                Some(TerrainSectionVisibility::all_visible(
                    section_y, false, false,
                )),
            );
        }
        synchronize_chunk_section_residency(
            upload,
            pos,
            &section_mesh_revisions,
            terrain_camera_position(world),
            &HashSet::new(),
            Instant::now(),
        );
        upload.meshes.insert(
            pos,
            CachedTerrainMesh {
                revision: key.mesh_revision,
                resident_revisions: section_mesh_revisions
                    .iter()
                    .map(|&(section_y, _)| (section_y, resident_revision))
                    .collect(),
                section_mesh_revisions,
                camera_position: terrain_camera_position(world),
                layers: TerrainSectionMeshLayers {
                    source_sections: key.section_count,
                    ..TerrainSectionMeshLayers::default()
                },
            },
        );
        return true;
    };
    let neighborhood = chunk_neighborhood(pos)
        .filter(|neighbor| *neighbor != pos)
        .filter_map(|neighbor| {
            upload
                .snapshots
                .get(&neighbor)
                .and_then(|cached| cached.snapshot.as_ref())
                .cloned()
        })
        .collect();
    let atlas = upload
        .mesh_atlas
        .get_or_insert_with(|| Arc::new(textures.atlas.clone()))
        .clone();
    let center = terrain_upload_center(world);
    let job = TerrainMeshJob {
        key,
        // BBB has no mutable `prioritizeChunkUpdates` surface; Vanilla 26.1's
        // default is `NONE`, so visible near-to-far distance remains the sole
        // compile ordering input. Local prediction state must not silently
        // select PLAYER_AFFECTED behavior.
        priority: (chunk_distance_key(pos, center), pos.x, pos.z),
        target,
        neighborhood,
        atlas,
        camera_position: terrain_camera_position(world),
        sections,
    };
    let worker = upload
        .mesh_worker
        .get_or_insert_with(TerrainMeshWorker::spawn);
    match worker.submit(job) {
        TerrainMeshSubmit::Queued { evicted } => {
            if let Some(evicted) = evicted {
                if upload.pending_mesh_jobs.get(&evicted.pos) == Some(&evicted) {
                    upload.pending_mesh_jobs.remove(&evicted.pos);
                }
                upload.mesh_worker_counters.stale_jobs =
                    upload.mesh_worker_counters.stale_jobs.saturating_add(1);
                upload.mesh_worker_counters.stale_sections = upload
                    .mesh_worker_counters
                    .stale_sections
                    .saturating_add(evicted.section_count as u64);
            }
            upload.pending_mesh_jobs.insert(pos, key);
            upload.mesh_worker_counters.queued_jobs =
                upload.mesh_worker_counters.queued_jobs.saturating_add(1);
            upload.mesh_worker_counters.queued_sections = upload
                .mesh_worker_counters
                .queued_sections
                .saturating_add(key.section_count as u64);
        }
        TerrainMeshSubmit::AlreadyQueued => {
            upload.pending_mesh_jobs.insert(pos, key);
        }
        TerrainMeshSubmit::Full => {
            upload.mesh_worker_counters.queue_full =
                upload.mesh_worker_counters.queue_full.saturating_add(1);
        }
    }
    update_mesh_worker_queue_depth(upload);
    false
}

fn drain_terrain_mesh_results(world: &WorldStore, upload: &mut TerrainUploadState) -> bool {
    let mut renderer_dirty = false;
    loop {
        let result = match upload.mesh_worker.as_mut() {
            Some(worker) => worker.try_recv(),
            None => None,
        };
        let Some(result) = result else {
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
    result: TerrainMeshResult,
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
    let valid = result.key == current
        && upload.selected_chunks.contains(&result.key.pos)
        && upload.snapshots.contains_key(&result.key.pos);
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
    let current_sections = terrain_section_mesh_revisions(world, result.key.pos);
    let residency_sections = current_sections.clone();
    let previous_section_ys = upload
        .meshes
        .get(&result.key.pos)
        .map(|cached| {
            cached
                .resident_revisions
                .iter()
                .map(|&(section_y, _)| section_y)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let previously_empty_ys = upload
        .meshes
        .get(&result.key.pos)
        .into_iter()
        .flat_map(|cached| cached.layers.visibility.iter())
        .filter(|visibility| visibility.loaded_empty)
        .map(|visibility| visibility.section_y)
        .collect::<HashSet<_>>();
    let dirty_section_ys = result
        .sections
        .iter()
        .map(|&(section_y, _)| section_y)
        .collect::<HashSet<_>>();
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
    merge_terrain_section_layers(&mut cached.layers, result.layers, &dirty_section_ys);
    cached.layers.source_sections = current_sections.len();
    cached.revision = result.key.mesh_revision;
    cached.section_mesh_revisions = current_sections;
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
    let newly_non_empty_ys = cached
        .layers
        .visibility
        .iter()
        .filter(|visibility| {
            previously_empty_ys.contains(&visibility.section_y) && !visibility.loaded_empty
        })
        .map(|visibility| visibility.section_y)
        .collect::<HashSet<_>>();
    let mut section_updates = dirty_section_ys
        .iter()
        .filter(|section_y| {
            cached
                .resident_revisions
                .iter()
                .any(|(resident_y, _)| resident_y == *section_y)
        })
        .map(|&section_y| {
            let visibility = cached
                .layers
                .visibility
                .iter()
                .find(|visibility| visibility.section_y == section_y)
                .copied()
                .unwrap_or_else(|| TerrainSectionVisibility::all_visible(section_y, false, false));
            (
                SectionPosition {
                    x: result.key.pos.x,
                    y: section_y,
                    z: result.key.pos.z,
                },
                Some(visibility),
            )
        })
        .collect::<Vec<_>>();
    section_updates.extend(
        previous_section_ys
            .into_iter()
            .filter(|section_y| {
                !cached
                    .resident_revisions
                    .iter()
                    .any(|(resident_y, _)| resident_y == section_y)
            })
            .map(|section_y| {
                (
                    SectionPosition {
                        x: result.key.pos.x,
                        y: section_y,
                        z: result.key.pos.z,
                    },
                    None,
                )
            }),
    );
    for (position, visibility) in section_updates {
        upload
            .occlusion_section_updates
            .insert(position, visibility);
    }
    synchronize_chunk_section_residency(
        upload,
        result.key.pos,
        &residency_sections,
        mesh_camera_position,
        &newly_non_empty_ys,
        Instant::now(),
    );
    true
}

fn merge_terrain_section_layers(
    cached: &mut TerrainSectionMeshLayers,
    replacement: TerrainSectionMeshLayers,
    dirty_section_ys: &HashSet<i32>,
) {
    fn merge_layer(
        cached: &mut Vec<bbb_renderer::terrain::TerrainSectionMesh>,
        replacement: Vec<bbb_renderer::terrain::TerrainSectionMesh>,
        dirty_section_ys: &HashSet<i32>,
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
    cached
        .visibility
        .retain(|section| !dirty_section_ys.contains(&section.section_y));
    cached.visibility.extend(replacement.visibility);
    cached
        .visibility
        .sort_unstable_by_key(|section| section.section_y);
}

fn cancel_queued_terrain_mesh_jobs(upload: &mut TerrainUploadState) {
    let cancelled = upload
        .mesh_worker
        .as_mut()
        .map(TerrainMeshWorker::cancel_queued)
        .unwrap_or_default();
    for key in cancelled {
        if upload.pending_mesh_jobs.get(&key.pos) == Some(&key) {
            upload.pending_mesh_jobs.remove(&key.pos);
        }
        upload.mesh_worker_counters.stale_jobs =
            upload.mesh_worker_counters.stale_jobs.saturating_add(1);
        upload.mesh_worker_counters.stale_sections = upload
            .mesh_worker_counters
            .stale_sections
            .saturating_add(key.section_count as u64);
    }
    update_mesh_worker_queue_depth(upload);
}

fn update_mesh_worker_queue_depth(upload: &mut TerrainUploadState) {
    let depth = upload
        .mesh_worker
        .as_ref()
        .map(TerrainMeshWorker::queue_depth)
        .unwrap_or(0);
    upload.mesh_worker_counters.queue_depth = depth;
    upload.mesh_worker_counters.peak_queue_depth =
        upload.mesh_worker_counters.peak_queue_depth.max(depth);
}

fn publish_mesh_worker_counters(
    world: &WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
) {
    let counters = terrain_residency_counters(world, upload);
    upload.cache_ready = counters.cache_ready;
    renderer.set_terrain_mesh_worker_counters(counters);
}

fn terrain_residency_counters(
    world: &WorldStore,
    upload: &TerrainUploadState,
) -> TerrainMeshWorkerCounters {
    let mut counters = upload.mesh_worker_counters;
    counters.selected_chunks = upload.selected_chunks.len();
    counters.selected_sections = upload
        .selected_chunks
        .iter()
        .map(|&pos| world.terrain_chunk_section_mesh_revisions(pos).count())
        .sum();
    for &pos in &upload.selected_chunks {
        let Some(cached) = upload
            .meshes
            .get(&pos)
            .filter(|cached| cached.revision == world.terrain_chunk_mesh_revision(pos))
        else {
            continue;
        };
        counters.resident_chunks += 1;
        counters.resident_sections += cached.resident_revisions.len();
    }
    counters.pending_jobs = upload.pending_mesh_jobs.len();
    counters.cache_ready = counters.selected_chunks != 0
        && counters.resident_chunks == counters.selected_chunks
        && counters.resident_sections == counters.selected_sections
        && counters.pending_jobs == 0;
    counters
}

fn upload_cached_terrain(
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
    full_sync: bool,
) {
    let cached: Vec<_> = upload
        .selected_chunks
        .iter()
        .filter_map(|pos| upload.meshes.get(pos).map(|cached| (*pos, cached)))
        .collect();
    let source_sections = cached
        .iter()
        .map(|(_, cached)| cached.layers.source_sections)
        .sum();
    if full_sync {
        renderer.synchronize_terrain_section_mesh_layer_refs(
            source_sections,
            cached.iter().map(|(pos, cached)| {
                (
                    TerrainChunkMeshOwner {
                        level_generation: upload.observed_level_generation,
                        chunk_x: pos.x,
                        chunk_z: pos.z,
                    },
                    cached.resident_revisions.as_slice(),
                    &cached.layers,
                    cached.camera_position,
                )
            }),
        );
        return;
    }

    let mut changes = Vec::new();
    let mut removed = Vec::new();
    for (&position, &visibility) in &upload.occlusion_section_updates {
        let owner = TerrainSectionMeshOwner {
            chunk: TerrainChunkMeshOwner {
                level_generation: upload.observed_level_generation,
                chunk_x: position.x,
                chunk_z: position.z,
            },
            section_y: position.y,
        };
        if visibility.is_none() {
            removed.push(owner);
            continue;
        }
        let pos = ChunkPos {
            x: position.x,
            z: position.z,
        };
        let Some(cached) = upload.meshes.get(&pos) else {
            removed.push(owner);
            continue;
        };
        let Some((_, revision)) = cached
            .resident_revisions
            .iter()
            .find(|(section_y, _)| *section_y == position.y)
        else {
            removed.push(owner);
            continue;
        };
        changes.push((owner, *revision, &cached.layers, cached.camera_position));
    }
    renderer.update_terrain_section_mesh_layer_refs(source_sections, changes, removed);
}

fn refresh_terrain_visibility(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    context: TerrainVisibilityContext,
) -> bool {
    let Some(camera_pose) = context.camera_pose else {
        upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
        upload.occlusion_rebuild_key = None;
        upload.pending_occlusion_rebuild_key = None;
        upload.occlusion_graph_input_key = None;
        upload.pending_occlusion_input_key = None;
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
        upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
        upload.occlusion_rebuild_key = None;
        upload.pending_occlusion_rebuild_key = None;
        upload.occlusion_graph_input_key = None;
        upload.pending_occlusion_input_key = None;
        publish_terrain_frustum_visibility(upload, context);
        return true;
    };
    let current_input_key = terrain_occlusion_input_key(world, upload);

    let pending_key = upload.pending_occlusion_rebuild_key;
    let mut accepted_result = None;
    if let Some(worker) = upload.occlusion_worker.as_mut() {
        while let Some(result) = worker.try_recv() {
            if pending_key == Some(result.key) {
                accepted_result = Some(result);
            }
        }
    }
    let graph_changed = accepted_result.is_some();
    if let Some(result) = accepted_result {
        upload.occlusion_graph = result.graph;
        upload.occlusion_rebuild_key = Some(result.key);
        upload.occlusion_graph_input_key = upload.pending_occlusion_input_key.take();
        upload.pending_occlusion_rebuild_key = None;
    }
    let partial_changed = upload.occlusion_graph.is_valid()
        && upload.occlusion_graph_input_key != Some(current_input_key);
    if partial_changed {
        let loaded_chunks = world.chunk_positions().into_iter().collect();
        upload.occlusion_graph.update_sections(
            upload
                .occlusion_section_updates
                .iter()
                .map(|(&position, &visibility)| (position, visibility)),
            &loaded_chunks,
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
    upload.occlusion_input_key = None;
    upload.occlusion_graph_input_key = None;
    upload.pending_occlusion_input_key = None;
    upload.occlusion_section_updates.clear();
    upload.occlusion_sections = Arc::default();
    upload.occlusion_loaded_chunks = Arc::default();
    upload.frustum_visibility_key = None;
}

fn terrain_occlusion_input_key(
    world: &WorldStore,
    upload: &TerrainUploadState,
) -> TerrainOcclusionInputKey {
    TerrainOcclusionInputKey {
        level_generation: world.client_level_generation(),
        world_revision: world.terrain_revision(),
        resident_revision: upload.next_resident_revision,
    }
}

fn refresh_terrain_occlusion_inputs(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    input_key: TerrainOcclusionInputKey,
) {
    if upload.occlusion_input_key == Some(input_key) {
        return;
    }
    upload.occlusion_sections = upload
        .selected_chunks
        .iter()
        .filter_map(|pos| upload.meshes.get(pos).map(|cached| (*pos, cached)))
        .flat_map(|(pos, cached)| {
            cached
                .resident_revisions
                .iter()
                .map(move |&(section_y, _)| {
                    let visibility = cached
                        .layers
                        .visibility
                        .iter()
                        .find(|visibility| visibility.section_y == section_y)
                        .copied()
                        .unwrap_or_else(|| {
                            TerrainSectionVisibility::all_visible(section_y, false, false)
                        });
                    ResidentSection {
                        position: SectionPosition {
                            x: pos.x,
                            y: section_y,
                            z: pos.z,
                        },
                        visibility,
                    }
                })
        })
        .collect::<Vec<_>>()
        .into();
    upload.occlusion_loaded_chunks = Arc::new(world.chunk_positions().into_iter().collect());
    upload.occlusion_input_key = Some(input_key);
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
    let now = Instant::now();
    let Some(frustum) = context
        .frustum()
        .filter(|_| upload.occlusion_graph.is_valid())
    else {
        let visible = selected_resident_sections(upload);
        upload.renderer_sections.replace(
            upload.observed_level_generation,
            visible,
            &upload.section_residency,
            now,
        );
        upload.frustum_visibility_key = Some(visibility_key);
        return;
    };
    // Like Vanilla's stable ViewArea graph, partial propagation is monotonic until
    // the next full invalidation. Do not publish stale graph leaves whose current
    // resident mesh has already been removed or remapped.
    let visible = upload
        .occlusion_graph
        .visible_sections(frustum)
        .into_iter()
        .filter(|position| has_resident_section(upload, *position))
        .collect::<Vec<_>>();
    upload.renderer_sections.replace(
        upload.observed_level_generation,
        visible,
        &upload.section_residency,
        now,
    );
    upload.frustum_visibility_key = Some(visibility_key);
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
        resident_revision: upload.next_resident_revision,
        camera_rotation_bucket,
        surface_size: context.surface_size,
        camera_fov_bits: context.camera_fov_degrees.to_bits(),
    }
}

fn selected_resident_sections(upload: &TerrainUploadState) -> Vec<SectionPosition> {
    upload
        .selected_chunks
        .iter()
        .filter_map(|pos| upload.meshes.get(pos).map(|cached| (*pos, cached)))
        .flat_map(|(pos, cached)| {
            cached
                .resident_revisions
                .iter()
                .map(move |&(section_y, _)| SectionPosition {
                    x: pos.x,
                    y: section_y,
                    z: pos.z,
                })
        })
        .collect()
}

fn has_resident_section(upload: &TerrainUploadState, position: SectionPosition) -> bool {
    upload
        .meshes
        .get(&ChunkPos {
            x: position.x,
            z: position.z,
        })
        .is_some_and(|cached| {
            cached
                .resident_revisions
                .iter()
                .any(|&(section_y, _)| section_y == position.y)
        })
}

fn terrain_camera_position(world: &WorldStore) -> [f32; 3] {
    camera_pose_from_world(world)
        .map(|pose| {
            [
                pose.position[0],
                pose.position[1] + pose.eye_height,
                pose.position[2],
            ]
        })
        .unwrap_or([0.0, 0.0, 0.0])
}

fn chunk_neighborhood(pos: ChunkPos) -> impl Iterator<Item = ChunkPos> {
    (-1..=1).flat_map(move |dz| {
        (-1..=1).map(move |dx| ChunkPos {
            x: pos.x + dx,
            z: pos.z + dz,
        })
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

pub(crate) fn terrain_cardinal_lighting(world: &WorldStore) -> TerrainCardinalLighting {
    match world.level_info().map(|level| level.cardinal_lighting()) {
        Some(WorldCardinalLighting::Nether) => TerrainCardinalLighting::Nether,
        _ => TerrainCardinalLighting::Default,
    }
}

fn convert_terrain_snapshot(
    snapshot: bbb_world::TerrainChunkSnapshot,
    textures: &TerrainTextureState,
    cardinal_lighting: TerrainCardinalLighting,
    biome_sampler: &bbb_world::ChunkBiomeSampler,
) -> TerrainChunkSnapshot {
    let chunk_origin_x = snapshot.pos.x * 16;
    let chunk_origin_z = snapshot.pos.z * 16;
    let cutout_leaves = textures.cutout_leaves();
    let cells = snapshot
        .cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            let light = TerrainLight {
                sky: cell.light.sky,
                block: cell.light.block,
            };
            if matches!(cell.material, bbb_world::TerrainMaterialClass::Empty) {
                return TerrainCell {
                    block_state_id: cell.block_state_id,
                    light,
                    ..TerrainCell::EMPTY
                };
            }
            let local_x = (index % 16) as i32;
            let local_y = (index / (16 * 16)) as i32;
            let local_z = ((index / 16) % 16) as i32;
            let position = BlockRenderPosition {
                x: chunk_origin_x + local_x,
                y: snapshot.min_y + local_y,
                z: chunk_origin_z + local_z,
            };
            let world_material = cell.material;
            let has_water_fluid = cell
                .fluid
                .is_some_and(|fluid| matches!(fluid.kind, bbb_world::TerrainFluidKind::Water));
            // Only biome-resolver blocks (grass / foliage / dry-foliage / water)
            // read the blend window, so we skip building the 5x5 sample grid for
            // the interior stone/dirt/air majority of a chunk.
            let wants_blend = cell
                .block_name
                .as_deref()
                .map(|name| block_wants_biome_blend(name, has_water_fluid))
                .unwrap_or(has_water_fluid);
            let blend = wants_blend.then(|| build_chunk_biome_blend(biome_sampler, position));
            let mut render_data = textures.block_render_data(
                cell.block_name.as_deref(),
                &cell.block_properties,
                world_material,
                cell.biome_id,
                Some(position),
                blend.as_ref(),
            );
            textures.force_opaque_leaves(
                cell.block_name.as_deref(),
                &mut render_data.2,
                &mut render_data.3,
            );
            let (texture_indices, tint, face_transparency, render_shape, ambient_occlusion) =
                render_data;
            let fluid = cell.fluid.map(renderer_fluid);
            let (fluid_texture_indices, fluid_tint) = fluid
                .map(|fluid| {
                    textures.fluid_render_data(
                        fluid.kind,
                        cell.biome_id,
                        Some(position),
                        blend.as_ref(),
                    )
                })
                .unwrap_or(([0; 6], [TerrainTint::WHITE; 6]));
            let fluid_overlay_texture_index =
                fluid.and_then(|fluid| textures.fluid_overlay_texture_index(fluid.kind));
            TerrainCell {
                block_state_id: cell.block_state_id,
                is_air: matches!(cell.material, bbb_world::TerrainMaterialClass::Empty),
                solid_render: cell.solid_render,
                can_occlude: cell.can_occlude,
                face_occlusion_shape_indices: cell.face_occlusion_shape_indices,
                view_blocking: cell.view_blocking,
                light_dampening: cell.light_dampening,
                shade_brightness: cell.shade_brightness,
                fluid,
                fluid_texture_indices,
                fluid_overlay_texture_index,
                fluid_overlay_neighbor: cell.fluid_overlay_neighbor,
                fluid_tint,
                texture_indices,
                render_shape,
                ambient_occlusion,
                state_light_emission: cell.state_light_emission,
                emissive_rendering: cell.emissive_rendering,
                material: match cell.material {
                    bbb_world::TerrainMaterialClass::Empty => TerrainMaterialClass::Empty,
                    bbb_world::TerrainMaterialClass::Invisible => TerrainMaterialClass::Empty,
                    bbb_world::TerrainMaterialClass::Opaque => TerrainMaterialClass::Opaque,
                    bbb_world::TerrainMaterialClass::Cutout => TerrainMaterialClass::Cutout,
                    bbb_world::TerrainMaterialClass::Fluid => TerrainMaterialClass::Fluid,
                    bbb_world::TerrainMaterialClass::Translucent => {
                        TerrainMaterialClass::Translucent
                    }
                },
                light,
                tint,
                face_transparency,
                skip_rendering: renderer_skip_rendering(cell.skip_rendering, cutout_leaves),
            }
        })
        .collect();
    TerrainChunkSnapshot::new(
        snapshot.pos.x,
        snapshot.pos.z,
        snapshot.min_y,
        snapshot.height,
        cells,
    )
    .with_cardinal_lighting(cardinal_lighting)
}

/// Samples the `biomeBlendRadius` window (x/z plane, fixed y) for `center`,
/// pulling neighbour-chunk biomes through `sampler`. Columns whose chunk is not
/// loaded stay `None`, so the averaging step honestly truncates the window at
/// the render-distance edge instead of inventing biome data.
fn build_chunk_biome_blend(
    sampler: &bbb_world::ChunkBiomeSampler,
    center: BlockRenderPosition,
) -> BiomeBlend {
    build_biome_blend(center, |pos| sampler.biome_id_at(pos.x, pos.y, pos.z))
}

fn renderer_skip_rendering(
    skip_rendering: bbb_world::TerrainSkipRendering,
    cutout_leaves: bool,
) -> TerrainSkipRendering {
    TerrainSkipRendering {
        same_block_key: skip_rendering.same_block_key,
        same_block_culls_all_faces: skip_rendering.same_block_culls_all_faces,
        same_block_culls_vertical_faces: skip_rendering.same_block_culls_vertical_faces,
        culls_with_leaves: !cutout_leaves && skip_rendering.leaves_block,
        iron_bars_block: skip_rendering.iron_bars_block,
        bars_tag: skip_rendering.bars_tag,
        north: skip_rendering.north,
        south: skip_rendering.south,
        west: skip_rendering.west,
        east: skip_rendering.east,
        connection_property_mask: skip_rendering.connection_property_mask,
    }
}

fn renderer_fluid(fluid: bbb_world::TerrainFluidState) -> TerrainFluid {
    let kind = match fluid.kind {
        bbb_world::TerrainFluidKind::Water => TerrainFluidKind::Water,
        bbb_world::TerrainFluidKind::Lava => TerrainFluidKind::Lava,
    };
    TerrainFluid::new(kind, fluid.amount, fluid.falling)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        BlockPos as ProtocolBlockPos, BlockUpdate, SectionBlocksUpdate, SetChunkCacheCenter, Vec3d,
    };
    use bbb_renderer::terrain::{TerrainMesh, TerrainSectionMesh};
    use bbb_world::{
        ChunkColumn, ChunkSection, ChunkState, LightData, LocalPlayerPoseState, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };

    #[test]
    fn renderer_fluid_preserves_world_fluid_state() {
        assert_eq!(
            renderer_fluid(bbb_world::TerrainFluidState::new(
                bbb_world::TerrainFluidKind::Water,
                5,
                false,
            )),
            TerrainFluid::new(TerrainFluidKind::Water, 5, false)
        );
        assert_eq!(
            renderer_fluid(bbb_world::TerrainFluidState::new(
                bbb_world::TerrainFluidKind::Lava,
                8,
                true,
            )),
            TerrainFluid::new(TerrainFluidKind::Lava, 8, true)
        );
    }

    #[test]
    fn renderer_skip_rendering_preserves_world_flags() {
        let skip = bbb_world::TerrainSkipRendering {
            same_block_key: 42,
            same_block_culls_all_faces: true,
            same_block_culls_vertical_faces: true,
            leaves_block: true,
            iron_bars_block: true,
            bars_tag: true,
            north: true,
            south: false,
            west: true,
            east: false,
            connection_property_mask: 0b1111,
        };

        assert_eq!(
            renderer_skip_rendering(skip, false),
            TerrainSkipRendering {
                same_block_key: 42,
                same_block_culls_all_faces: true,
                same_block_culls_vertical_faces: true,
                culls_with_leaves: true,
                iron_bars_block: true,
                bars_tag: true,
                north: true,
                south: false,
                west: true,
                east: false,
                connection_property_mask: 0b1111,
            }
        );
        assert!(!renderer_skip_rendering(skip, true).culls_with_leaves);
    }

    #[test]
    fn texture_animation_tick_advances_at_client_tick_interval() {
        let mut upload = TerrainUploadState::default();
        let start = Instant::now();

        assert_eq!(advance_texture_animation_tick(&mut upload, start), None);
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(49)),
            None
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(50)),
            Some((0, 1))
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(149)),
            Some((1, 2))
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(250)),
            Some((2, 5))
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(299)),
            None
        );
    }

    #[test]
    fn occlusion_rebuild_key_matches_vanilla_eight_block_invalidation() {
        let world = WorldStore::new();
        let mut upload = TerrainUploadState {
            next_resident_revision: 7,
            ..Default::default()
        };
        let context = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [0.25, 64.0, 7.75],
                y_rot: 10.0,
                x_rot: -5.0,
                eye_height: 1.5,
            }),
            [1280, 720],
            70.0,
            12,
            true,
        );
        let key =
            terrain_occlusion_rebuild_key(&world, &upload, context, [0.25, 65.5, 7.75]).unwrap();

        let rotated_and_resized = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [7.99, 64.25, 0.01],
                y_rot: 179.0,
                x_rot: 45.0,
                eye_height: 1.25,
            }),
            [720, 1280],
            110.0,
            12,
            true,
        );
        assert_eq!(
            terrain_occlusion_rebuild_key(&world, &upload, rotated_and_resized, [7.99, 65.5, 0.01],),
            Some(key),
            "rotation, FOV and surface changes only require per-frame frustum traversal",
        );

        let crossed_cell =
            terrain_occlusion_rebuild_key(&world, &upload, context, [8.0, 65.5, 7.75]).unwrap();
        assert_ne!(crossed_cell, key);

        upload.next_resident_revision += 1;
        assert_eq!(
            terrain_occlusion_rebuild_key(&world, &upload, context, [0.25, 65.5, 7.75]),
            Some(key),
            "Vanilla schedules propagation from recompiled sections instead of invalidating the graph",
        );
        let different_view = TerrainVisibilityContext {
            render_distance_chunks: 13,
            ..context
        };
        assert_ne!(
            terrain_occlusion_rebuild_key(
                &world,
                &TerrainUploadState {
                    next_resident_revision: 7,
                    ..Default::default()
                },
                different_view,
                [0.25, 65.5, 7.75],
            ),
            Some(key),
        );
        assert!(
            terrain_occlusion_rebuild_key(&world, &upload, context, [f64::NAN, 65.5, 7.75],)
                .is_none()
        );
    }

    #[test]
    fn upload_tail_reuses_visibility_already_published_for_the_frame() {
        let world = WorldStore::new();
        let context = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [4.0, 64.0, 4.0],
                y_rot: 35.0,
                x_rot: -10.0,
                eye_height: 1.62,
            }),
            [1280, 720],
            70.0,
            12,
            true,
        );
        let mut upload = TerrainUploadState::default();
        let eye = [4.0, 65.62, 4.0];
        upload.occlusion_rebuild_key = terrain_occlusion_rebuild_key(&world, &upload, context, eye);
        upload.occlusion_graph_input_key = Some(terrain_occlusion_input_key(&world, &upload));

        assert!(!refresh_terrain_visibility(&world, &mut upload, context));

        upload.occlusion_rebuild_key = None;
        assert!(refresh_terrain_visibility(&world, &mut upload, context));
        assert!(upload.pending_occlusion_rebuild_key.is_some());

        while upload.occlusion_rebuild_key.is_none() {
            refresh_terrain_visibility(&world, &mut upload, context);
            std::thread::yield_now();
        }
        assert!(upload.pending_occlusion_rebuild_key.is_none());
        assert!(!refresh_terrain_visibility(&world, &mut upload, context));
    }

    #[test]
    fn camera_cell_rebuild_reuses_immutable_occlusion_inputs() {
        let world = WorldStore::new();
        let context = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [4.0, 64.0, 4.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            }),
            [1280, 720],
            70.0,
            12,
            true,
        );
        let mut upload = TerrainUploadState::default();
        assert!(refresh_terrain_visibility(&world, &mut upload, context));
        let sections = Arc::clone(&upload.occlusion_sections);
        let loaded_chunks = Arc::clone(&upload.occlusion_loaded_chunks);

        while upload.pending_occlusion_rebuild_key.is_some() {
            refresh_terrain_visibility(&world, &mut upload, context);
            std::thread::yield_now();
        }

        let next_cell = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [12.0, 64.0, 4.0],
                ..context.camera_pose.unwrap()
            }),
            context.surface_size,
            context.camera_fov_degrees,
            context.render_distance_chunks,
            context.smart_cull,
        );
        refresh_terrain_visibility(&world, &mut upload, next_cell);
        assert!(upload.pending_occlusion_rebuild_key.is_some());
        assert!(Arc::ptr_eq(&sections, &upload.occlusion_sections));
        assert!(Arc::ptr_eq(&loaded_chunks, &upload.occlusion_loaded_chunks));
    }

    #[test]
    fn published_partial_graph_filters_sections_without_resident_meshes() {
        let resident_pos = ChunkPos { x: 0, z: 0 };
        let mut upload = TerrainUploadState {
            observed_level_generation: 4,
            selected_chunks: vec![resident_pos],
            ..Default::default()
        };
        upload.meshes.insert(
            resident_pos,
            CachedTerrainMesh {
                revision: 1,
                section_mesh_revisions: vec![(0, 1)],
                resident_revisions: vec![(0, 1)],
                camera_position: [0.0; 3],
                layers: TerrainSectionMeshLayers::default(),
            },
        );
        upload.section_residency.insert(
            SectionPosition { x: 0, y: 0, z: 0 },
            TerrainSectionResidency {
                uploaded_at: Instant::now(),
                fade_duration: Duration::ZERO,
            },
        );
        let loaded = (-2..=2)
            .flat_map(|z| (-2..=2).map(move |x| ChunkPos { x, z }))
            .collect::<TerrainCoordinateSet<_>>();
        upload.occlusion_graph.rebuild(
            [
                ResidentSection {
                    position: SectionPosition { x: 0, y: 0, z: 0 },
                    visibility: TerrainSectionVisibility::all_visible(0, true, false),
                },
                ResidentSection {
                    position: SectionPosition { x: 0, y: 0, z: 1 },
                    visibility: TerrainSectionVisibility::all_visible(0, true, false),
                },
            ],
            &loaded,
            [8.0, 8.0, 8.0],
            2,
            0,
            1,
            false,
        );
        let context = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [8.0, 0.0, 8.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 8.0,
            }),
            [1280, 720],
            120.0,
            2,
            false,
        );

        publish_terrain_frustum_visibility(&mut upload, context);

        assert_eq!(
            upload.renderer_sections.states,
            [TerrainSectionRenderState {
                owner: TerrainSectionMeshOwner {
                    chunk: TerrainChunkMeshOwner {
                        level_generation: 4,
                        chunk_x: 0,
                        chunk_z: 0,
                    },
                    section_y: 0,
                },
                visibility: 1.0,
            }]
        );
    }

    #[test]
    fn frame_visibility_matches_vanilla_two_degree_rotation_gate() {
        let mut upload = TerrainUploadState::default();
        let context = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [4.0, 64.0, 4.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 1.62,
            }),
            [1280, 720],
            70.0,
            12,
            true,
        );
        assert!(upload.refresh_frame_visibility(context));
        assert!(!upload.refresh_frame_visibility(context));

        let within_camera_cell = TerrainVisibilityContext::new(
            Some(CameraPose {
                position: [7.5, 64.0, 7.5],
                y_rot: 1.99,
                ..context.camera_pose.unwrap()
            }),
            context.surface_size,
            context.camera_fov_degrees,
            context.render_distance_chunks,
            context.smart_cull,
        );
        assert!(!upload.refresh_frame_visibility(within_camera_cell));

        let next_rotation_bucket = TerrainVisibilityContext::new(
            Some(CameraPose {
                y_rot: 2.0,
                ..within_camera_cell.camera_pose.unwrap()
            }),
            within_camera_cell.surface_size,
            within_camera_cell.camera_fov_degrees,
            within_camera_cell.render_distance_chunks,
            within_camera_cell.smart_cull,
        );
        assert!(upload.refresh_frame_visibility(next_rotation_bucket));

        let resized = TerrainVisibilityContext::new(
            next_rotation_bucket.camera_pose,
            [1920, 1080],
            next_rotation_bucket.camera_fov_degrees,
            next_rotation_bucket.render_distance_chunks,
            next_rotation_bucket.smart_cull,
        );
        assert!(upload.refresh_frame_visibility(resized));
    }

    #[test]
    fn terrain_reload_request_marks_all_cached_chunks_for_rebuild() {
        let mut upload = TerrainUploadState::default();
        assert!(!upload.reload_all_chunks_requested());
        upload.request_reload_all_chunks();
        assert!(upload.reload_all_chunks_requested());
    }

    #[test]
    fn ready_unchanged_terrain_reuses_resident_selection() {
        let world = WorldStore::new();
        let mut upload = TerrainUploadState {
            observed_world_revision: world.terrain_revision(),
            cache_ready: true,
            ..Default::default()
        };

        assert!(terrain_cache_selection_is_stable(&world, &upload, false));
        assert!(!terrain_cache_selection_is_stable(&world, &upload, true));
        upload.reload_all_chunks_requested = true;
        assert!(!terrain_cache_selection_is_stable(&world, &upload, false));
        upload.reload_all_chunks_requested = false;
        upload.cache_ready = false;
        assert!(!terrain_cache_selection_is_stable(&world, &upload, false));
    }

    #[test]
    fn empty_world_transition_clears_incremental_caches_once() {
        let pos = ChunkPos { x: 2, z: -3 };
        let mut upload = TerrainUploadState {
            observed_world_revision: 4,
            selected_chunks: vec![pos],
            meshes: HashMap::from([(
                pos,
                CachedTerrainMesh {
                    revision: 3,
                    section_mesh_revisions: Vec::new(),
                    resident_revisions: Vec::new(),
                    camera_position: [0.0; 3],
                    layers: TerrainSectionMeshLayers::default(),
                },
            )]),
            reload_all_chunks_requested: true,
            ..Default::default()
        };

        assert!(synchronize_empty_terrain_upload(&mut upload, 9));
        assert_eq!(upload.observed_world_revision, 9);
        assert!(upload.selected_chunks.is_empty());
        assert!(upload.snapshots.is_empty());
        assert!(upload.meshes.is_empty());
        assert!(!upload.reload_all_chunks_requested);

        assert!(!synchronize_empty_terrain_upload(&mut upload, 9));
    }

    #[test]
    fn level_generation_change_drops_old_residency_without_reusing_revision_ids() {
        let pos = ChunkPos { x: 2, z: -3 };
        let mut upload = TerrainUploadState {
            observed_level_generation: 4,
            selected_chunks: vec![pos],
            meshes: HashMap::from([(
                pos,
                CachedTerrainMesh {
                    revision: 3,
                    section_mesh_revisions: Vec::new(),
                    resident_revisions: Vec::new(),
                    camera_position: [0.0; 3],
                    layers: TerrainSectionMeshLayers::default(),
                },
            )]),
            next_resident_revision: 8,
            ..Default::default()
        };
        upload.section_residency.insert(
            SectionPosition {
                x: pos.x,
                y: 4,
                z: pos.z,
            },
            TerrainSectionResidency {
                uploaded_at: Instant::now(),
                fade_duration: Duration::ZERO,
            },
        );

        assert!(!synchronize_terrain_level_generation(&mut upload, 4));
        assert!(synchronize_terrain_level_generation(&mut upload, 5));
        assert!(upload.selected_chunks.is_empty());
        assert!(upload.snapshots.is_empty());
        assert!(upload.meshes.is_empty());
        assert!(upload.section_residency.is_empty());
        assert_eq!(upload.next_resident_revision, 8);
        assert!(!synchronize_terrain_level_generation(&mut upload, 5));
    }

    #[test]
    fn selected_resident_sections_publish_exact_vertical_mesh_membership() {
        let selected = ChunkPos { x: -1, z: 2 };
        let unselected = ChunkPos { x: 8, z: 9 };
        let mesh = |resident_revisions| CachedTerrainMesh {
            revision: 1,
            section_mesh_revisions: Vec::new(),
            resident_revisions,
            camera_position: [0.0; 3],
            layers: TerrainSectionMeshLayers::default(),
        };
        let upload = TerrainUploadState {
            selected_chunks: vec![selected],
            meshes: HashMap::from([
                (selected, mesh(vec![(-4, 10), (3, 11)])),
                (unselected, mesh(vec![(7, 12)])),
            ]),
            ..Default::default()
        };

        assert_eq!(
            selected_resident_sections(&upload),
            vec![
                SectionPosition { x: -1, y: -4, z: 2 },
                SectionPosition { x: -1, y: 3, z: 2 },
            ]
        );
        assert!(has_resident_section(
            &upload,
            SectionPosition { x: -1, y: 3, z: 2 }
        ));
        assert!(!has_resident_section(
            &upload,
            SectionPosition { x: -1, y: 4, z: 2 }
        ));
        assert!(!has_resident_section(
            &upload,
            SectionPosition { x: 3, y: 3, z: 2 }
        ));
    }

    #[test]
    fn partial_worker_result_preserves_unchanged_section_mesh_and_residency() {
        let pos = ChunkPos { x: 0, z: 0 };
        let mut world = WorldStore::new();
        world.insert_decoded_chunk(test_chunk(pos));
        let current_sections = terrain_section_mesh_revisions(&world, pos);
        assert!(current_sections.len() >= 2);
        let dirty_y = current_sections[0].0;
        let stable_y = current_sections[1].0;
        let mut cached_sections = current_sections.clone();
        cached_sections[0].1 = cached_sections[0].1.saturating_add(1);
        let cached_layers = TerrainSectionMeshLayers {
            opaque: vec![
                TerrainSectionMesh {
                    section_y: dirty_y,
                    mesh: TerrainMesh {
                        source_sections: 11,
                        ..TerrainMesh::default()
                    },
                },
                TerrainSectionMesh {
                    section_y: stable_y,
                    mesh: TerrainMesh {
                        source_sections: 22,
                        ..TerrainMesh::default()
                    },
                },
            ],
            source_sections: current_sections.len(),
            ..TerrainSectionMeshLayers::default()
        };
        let mut upload = TerrainUploadState {
            observed_level_generation: world.client_level_generation(),
            mesh_generation: 3,
            selected_chunks: vec![pos],
            snapshots: HashMap::from([(pos, cached_snapshot(&world, pos))]),
            meshes: HashMap::from([(
                pos,
                CachedTerrainMesh {
                    revision: world.terrain_chunk_mesh_revision(pos).saturating_sub(1),
                    section_mesh_revisions: cached_sections,
                    resident_revisions: current_sections
                        .iter()
                        .map(|&(section_y, _)| (section_y, 10))
                        .collect(),
                    camera_position: [0.0; 3],
                    layers: cached_layers,
                },
            )]),
            next_resident_revision: 10,
            ..Default::default()
        };
        let key = terrain_mesh_job_key(&world, &upload, pos);
        assert_eq!(key.section_count, 1);
        upload.pending_mesh_jobs.insert(pos, key);

        assert!(accept_terrain_mesh_result(
            &world,
            &mut upload,
            TerrainMeshResult {
                key,
                camera_position: [4.0, 5.0, 6.0],
                build_nanos: 23,
                sections: vec![(dirty_y, current_sections[0].1)],
                layers: TerrainSectionMeshLayers {
                    opaque: vec![TerrainSectionMesh {
                        section_y: dirty_y,
                        mesh: TerrainMesh {
                            source_sections: 33,
                            ..TerrainMesh::default()
                        },
                    }],
                    source_sections: 1,
                    ..TerrainSectionMeshLayers::default()
                },
            },
        ));

        let cached = upload.meshes.get(&pos).expect("partial result installed");
        assert_eq!(cached.section_mesh_revisions, current_sections);
        assert_eq!(cached.layers.source_sections, current_sections.len());
        assert_eq!(cached.layers.opaque.len(), 2);
        assert_eq!(cached.layers.opaque[0].section_y, dirty_y);
        assert_eq!(cached.layers.opaque[0].mesh.source_sections, 33);
        assert_eq!(cached.layers.opaque[1].section_y, stable_y);
        assert_eq!(cached.layers.opaque[1].mesh.source_sections, 22);
        assert_eq!(
            cached
                .resident_revisions
                .iter()
                .find(|(section_y, _)| *section_y == dirty_y),
            Some(&(dirty_y, 11))
        );
        assert_eq!(
            cached
                .resident_revisions
                .iter()
                .find(|(section_y, _)| *section_y == stable_y),
            Some(&(stable_y, 10))
        );
        assert_eq!(upload.mesh_worker_counters.completed_sections, 1);
        assert_eq!(upload.mesh_worker_counters.recent_build_sections, 1);
        assert_eq!(upload.mesh_worker_counters.peak_build_sections, 1);
        assert_eq!(upload.occlusion_section_updates.len(), 1);
        assert_eq!(
            upload.occlusion_section_updates.get(&SectionPosition {
                x: pos.x,
                y: dirty_y,
                z: pos.z,
            }),
            Some(&Some(TerrainSectionVisibility::all_visible(
                dirty_y, false, false,
            )))
        );
    }

    #[test]
    fn block_update_packets_rebuild_dirty_sections_and_reuse_stable_layers() {
        let pos = ChunkPos { x: 0, z: 0 };
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 48,
        });
        world.insert_decoded_chunk(test_chunk_with_sections(pos, 3));
        let stone = propertyless_block_state_id("minecraft:stone");
        let dirt = propertyless_block_state_id("minecraft:dirt");
        let initial_sections = terrain_section_mesh_revisions(&world, pos);
        assert_eq!(
            initial_sections
                .iter()
                .map(|&(section_y, _)| section_y)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );

        let initial_layers = TerrainSectionMeshLayers {
            opaque: initial_sections
                .iter()
                .enumerate()
                .map(|(index, &(section_y, _))| TerrainSectionMesh {
                    section_y,
                    mesh: TerrainMesh {
                        source_sections: (index + 1) * 10,
                        ..TerrainMesh::default()
                    },
                })
                .collect(),
            source_sections: initial_sections.len(),
            ..TerrainSectionMeshLayers::default()
        };
        let mut initial_mesh = cached_mesh(&world, pos, 100);
        initial_mesh.layers = initial_layers;
        let mut upload = TerrainUploadState {
            observed_level_generation: world.client_level_generation(),
            selected_chunks: vec![pos],
            snapshots: HashMap::from([(pos, cached_snapshot(&world, pos))]),
            meshes: HashMap::from([(pos, initial_mesh)]),
            next_resident_revision: 100,
            ..Default::default()
        };

        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 5, y: 5, z: 7 },
            block_state_id: stone,
        }));
        upload.snapshots.insert(pos, cached_snapshot(&world, pos));
        let single_dirty = dirty_terrain_sections(&world, &upload, pos);
        assert_eq!(
            single_dirty
                .iter()
                .map(|&(section_y, _)| section_y)
                .collect::<Vec<_>>(),
            vec![0]
        );
        let single_key = terrain_mesh_job_key(&world, &upload, pos);
        assert_eq!(single_key.section_count, 1);
        upload.pending_mesh_jobs.insert(pos, single_key);
        assert!(accept_terrain_mesh_result(
            &world,
            &mut upload,
            TerrainMeshResult {
                key: single_key,
                camera_position: [0.0; 3],
                build_nanos: 1,
                sections: single_dirty,
                layers: TerrainSectionMeshLayers {
                    opaque: vec![TerrainSectionMesh {
                        section_y: 0,
                        mesh: TerrainMesh {
                            source_sections: 11,
                            ..TerrainMesh::default()
                        },
                    }],
                    source_sections: 1,
                    ..TerrainSectionMeshLayers::default()
                },
            },
        ));

        let cached = upload.meshes.get(&pos).expect("single update installed");
        assert_eq!(
            cached
                .layers
                .opaque
                .iter()
                .map(|section| section.mesh.source_sections)
                .collect::<Vec<_>>(),
            vec![11, 20, 30]
        );
        assert_eq!(
            cached.resident_revisions,
            vec![(0, 101), (1, 100), (2, 100)]
        );

        assert_eq!(
            world.apply_section_blocks_update(SectionBlocksUpdate {
                section_x: 0,
                section_y: 2,
                section_z: 0,
                updates: vec![BlockUpdate {
                    pos: ProtocolBlockPos { x: 6, y: 37, z: 8 },
                    block_state_id: dirt,
                }],
            }),
            1
        );
        upload.snapshots.insert(pos, cached_snapshot(&world, pos));
        let section_dirty = dirty_terrain_sections(&world, &upload, pos);
        assert_eq!(
            section_dirty
                .iter()
                .map(|&(section_y, _)| section_y)
                .collect::<Vec<_>>(),
            vec![2]
        );
        let section_key = terrain_mesh_job_key(&world, &upload, pos);
        assert_eq!(section_key.section_count, 1);
        upload.pending_mesh_jobs.insert(pos, section_key);
        assert!(accept_terrain_mesh_result(
            &world,
            &mut upload,
            TerrainMeshResult {
                key: section_key,
                camera_position: [0.0; 3],
                build_nanos: 1,
                sections: section_dirty,
                layers: TerrainSectionMeshLayers {
                    opaque: vec![TerrainSectionMesh {
                        section_y: 2,
                        mesh: TerrainMesh {
                            source_sections: 31,
                            ..TerrainMesh::default()
                        },
                    }],
                    source_sections: 1,
                    ..TerrainSectionMeshLayers::default()
                },
            },
        ));

        let cached = upload.meshes.get(&pos).expect("section update installed");
        assert_eq!(
            cached
                .layers
                .opaque
                .iter()
                .map(|section| section.mesh.source_sections)
                .collect::<Vec<_>>(),
            vec![11, 20, 31]
        );
        assert_eq!(
            cached.resident_revisions,
            vec![(0, 101), (1, 100), (2, 102)]
        );
    }

    #[test]
    fn stale_worker_result_cannot_replace_current_chunk_revision() {
        let pos = ChunkPos { x: 0, z: 0 };
        let mut world = WorldStore::new();
        world.insert_decoded_chunk(test_chunk(pos));
        let mut upload = TerrainUploadState {
            observed_level_generation: world.client_level_generation(),
            mesh_generation: 3,
            selected_chunks: vec![pos],
            snapshots: HashMap::from([(pos, cached_snapshot(&world, pos))]),
            ..Default::default()
        };
        let current = terrain_mesh_job_key(&world, &upload, pos);
        let current_sections = dirty_terrain_sections(&world, &upload, pos);
        let stale = TerrainMeshJobKey {
            mesh_generation: current.mesh_generation - 1,
            ..current
        };
        upload.pending_mesh_jobs.insert(pos, stale);

        assert!(!accept_terrain_mesh_result(
            &world,
            &mut upload,
            TerrainMeshResult {
                key: stale,
                camera_position: [1.0, 2.0, 3.0],
                build_nanos: 17,
                sections: current_sections.clone(),
                layers: TerrainSectionMeshLayers::default(),
            },
        ));
        assert!(upload.meshes.is_empty());
        assert_eq!(upload.mesh_worker_counters.completed_jobs, 1);
        assert_eq!(upload.mesh_worker_counters.stale_jobs, 1);

        upload.pending_mesh_jobs.insert(pos, current);
        assert!(accept_terrain_mesh_result(
            &world,
            &mut upload,
            TerrainMeshResult {
                key: current,
                camera_position: [4.0, 5.0, 6.0],
                build_nanos: 23,
                sections: current_sections,
                layers: TerrainSectionMeshLayers::default(),
            },
        ));
        let cached = upload.meshes.get(&pos).expect("current result installed");
        assert_eq!(cached.revision, current.mesh_revision);
        assert_eq!(cached.camera_position, [4.0, 5.0, 6.0]);
        assert_eq!(upload.mesh_worker_counters.completed_jobs, 2);
        assert_eq!(upload.mesh_worker_counters.stale_jobs, 1);
        assert_eq!(upload.mesh_worker_counters.recent_build_nanos, 23);
        assert_eq!(upload.mesh_worker_counters.peak_build_nanos, 23);
    }

    #[test]
    fn cache_scheduler_refreshes_one_neighbor_at_a_time_before_one_mesh() {
        let center = ChunkPos { x: 0, z: 0 };
        let east = ChunkPos { x: 1, z: 0 };
        let mut world = WorldStore::new();
        world.insert_decoded_chunk(test_chunk(center));
        world.insert_decoded_chunk(test_chunk(east));
        let selected_chunks = selected_terrain_chunks(&world, center);
        let loaded = world.chunk_positions().into_iter().collect();
        let mut upload = TerrainUploadState {
            selected_chunks,
            ..Default::default()
        };

        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::RefreshSnapshot(center)
        );
        upload
            .snapshots
            .insert(center, cached_snapshot(&world, center));
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::RefreshSnapshot(east)
        );
        upload.snapshots.insert(east, cached_snapshot(&world, east));
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::QueueMesh(center)
        );
        upload.meshes.insert(center, cached_mesh(&world, center, 1));
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::QueueMesh(east)
        );
        upload.meshes.insert(east, cached_mesh(&world, east, 2));
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::Idle
        );

        // Replacing the east chunk invalidates the center mesh. Even though
        // the center's canonical content revision is unchanged, its render
        // snapshot must be refreshed because biome tint blending samples the
        // neighboring column.
        world.insert_decoded_chunk(test_chunk(east));
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::RefreshSnapshot(center)
        );
    }

    #[test]
    fn terrain_selection_keeps_every_server_loaded_chunk_in_distance_order() {
        let center = ChunkPos { x: 0, z: 0 };
        let mut world = WorldStore::new();
        for x in -4..=4 {
            for z in -4..=4 {
                world.insert_decoded_chunk(test_chunk(ChunkPos { x, z }));
            }
        }

        let selected = selected_terrain_chunks(&world, center);

        assert_eq!(selected.len(), 81);
        assert_eq!(selected[0], center);
        assert_eq!(selected.iter().copied().collect::<HashSet<_>>().len(), 81);
        assert!(selected.windows(2).all(|pair| {
            chunk_distance_key(pair[0], center) <= chunk_distance_key(pair[1], center)
        }));
    }

    #[test]
    fn terrain_residency_counters_require_every_selected_section_and_no_pending_job() {
        let center = ChunkPos { x: 0, z: 0 };
        let east = ChunkPos { x: 1, z: 0 };
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 48,
        });
        world.insert_decoded_chunk(test_chunk_with_sections(center, 3));
        world.insert_decoded_chunk(test_chunk_with_sections(east, 3));
        let mut upload = TerrainUploadState {
            selected_chunks: vec![center, east],
            meshes: HashMap::from([(center, cached_mesh(&world, center, 1))]),
            ..Default::default()
        };

        let incomplete = terrain_residency_counters(&world, &upload);
        assert_eq!(incomplete.selected_chunks, 2);
        assert_eq!(incomplete.resident_chunks, 1);
        assert_eq!(incomplete.selected_sections, 6);
        assert_eq!(incomplete.resident_sections, 3);
        assert!(!incomplete.cache_ready);

        upload.meshes.insert(east, cached_mesh(&world, east, 2));
        let key = terrain_mesh_job_key(&world, &upload, east);
        upload.pending_mesh_jobs.insert(east, key);
        let pending = terrain_residency_counters(&world, &upload);
        assert_eq!(pending.resident_chunks, 2);
        assert_eq!(pending.resident_sections, 6);
        assert_eq!(pending.pending_jobs, 1);
        assert!(!pending.cache_ready);

        upload.pending_mesh_jobs.clear();
        assert!(terrain_residency_counters(&world, &upload).cache_ready);
    }

    #[test]
    fn section_ready_distinguishes_vertical_meshes_and_height_policy() {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: -64,
            height: 384,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: -0.25,
                y: 64.0,
                z: 16.0,
            },
            ..LocalPlayerPoseState::default()
        });
        let mut upload = TerrainUploadState::default();

        assert!(!upload.is_player_section_compiled_and_visible(&world));
        let position = SectionPosition { x: -1, y: 4, z: 1 };
        upload.section_residency.insert(
            position,
            TerrainSectionResidency {
                uploaded_at: Instant::now(),
                fade_duration: Duration::ZERO,
            },
        );
        upload
            .renderer_sections
            .replace(0, [position], &upload.section_residency, Instant::now());
        assert!(upload.is_player_section_compiled_and_visible(&world));
        assert!(upload.is_entity_section_compiled_and_visible(
            &world,
            bbb_world::EntityVec3 {
                x: -0.25,
                y: 64.0,
                z: 16.0,
            }
        ));
        assert!(upload.is_block_section_compiled_and_visible(
            &world,
            bbb_world::BlockPos {
                x: -1,
                y: 64,
                z: 16,
            }
        ));

        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: -0.25,
                y: 80.0,
                z: 16.0,
            },
            ..LocalPlayerPoseState::default()
        });
        assert!(
            !upload.is_player_section_compiled_and_visible(&world),
            "another vertical section in the same chunk is not compiled"
        );
        assert!(!upload.is_entity_section_compiled_and_visible(
            &world,
            bbb_world::EntityVec3 {
                x: -0.25,
                y: 80.0,
                z: 16.0,
            }
        ));
        assert!(!upload.is_block_section_compiled_and_visible(
            &world,
            bbb_world::BlockPos {
                x: -1,
                y: 80,
                z: 16,
            }
        ));

        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: -0.25,
                y: 400.0,
                z: 16.0,
            },
            ..LocalPlayerPoseState::default()
        });
        assert!(!upload.is_player_section_compiled_and_visible(&world));
        assert!(upload.is_entity_section_compiled_and_visible(
            &world,
            bbb_world::EntityVec3 {
                x: -0.25,
                y: 400.0,
                z: 16.0,
            }
        ));
        assert!(upload.is_block_section_compiled_and_visible(
            &world,
            bbb_world::BlockPos {
                x: -1,
                y: 400,
                z: 16,
            }
        ));
    }

    #[test]
    fn terrain_upload_center_is_derived_from_world_chunk_state() {
        let mut world = WorldStore::new();
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: 0, z: 0 });

        world.insert_decoded_chunk(test_chunk(ChunkPos { x: 5, z: 6 }));
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: 5, z: 6 });

        world.insert_decoded_chunk(test_chunk(ChunkPos { x: -1, z: 2 }));
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: 5, z: 6 });

        world.apply_set_chunk_cache_center(SetChunkCacheCenter {
            chunk_x: -4,
            chunk_z: 7,
        });
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: -4, z: 7 });
    }

    fn test_chunk(pos: ChunkPos) -> ChunkColumn {
        ChunkColumn {
            pos,
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: Vec::new(),
            light: LightData::default(),
        }
    }

    fn test_chunk_with_sections(pos: ChunkPos, section_count: usize) -> ChunkColumn {
        ChunkColumn {
            sections: (0..section_count)
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        bbb_world::vanilla_air_block_state_id(),
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
                .collect(),
            ..test_chunk(pos)
        }
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn propertyless_block_state_id(name: &str) -> i32 {
        bbb_world::vanilla_block_state_id(name, &std::collections::BTreeMap::new())
            .unwrap_or_else(|| panic!("missing propertyless vanilla block state {name}"))
    }

    fn cached_snapshot(world: &WorldStore, pos: ChunkPos) -> CachedTerrainSnapshot {
        CachedTerrainSnapshot {
            snapshot_revision: world.terrain_chunk_snapshot_revision(pos),
            snapshot: None,
        }
    }

    fn cached_mesh(world: &WorldStore, pos: ChunkPos, resident_revision: u64) -> CachedTerrainMesh {
        let section_mesh_revisions = terrain_section_mesh_revisions(world, pos);
        CachedTerrainMesh {
            revision: world.terrain_chunk_mesh_revision(pos),
            resident_revisions: section_mesh_revisions
                .iter()
                .map(|&(section_y, _)| (section_y, resident_revision))
                .collect(),
            section_mesh_revisions,
            camera_position: [0.0; 3],
            layers: TerrainSectionMeshLayers::default(),
        }
    }
}

#[cfg(test)]
mod initial_light_gate_tests {
    use super::*;
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
    use bbb_protocol::packets::{
        ChunkHeightmapData, LevelChunkData, LevelChunkWithLight, LightUpdateData,
    };
    use std::collections::HashSet;

    fn synthetic_chunk_packet() -> LevelChunkWithLight {
        let mut sections = Encoder::new();
        for _ in 0..bbb_world::WorldDimension::default().section_count() {
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
        world.set_local_player_pose(bbb_world::LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: f64::from(near.x) * 16.0 + 8.0,
                y: 64.0,
                z: f64::from(near.z) * 16.0 + 8.0,
            },
            ..bbb_world::LocalPlayerPoseState::default()
        });

        // Vanilla's flag-8 player-priority rebuild maps to the distance-first
        // target scan: both chunks need work, the player's chunk wins.
        let selected = selected_terrain_chunks(&world, near);
        assert_eq!(selected, vec![near, distant]);
        let loaded: HashSet<_> = world.chunk_positions().into_iter().collect();
        let mut upload = TerrainUploadState::default();
        upload.selected_chunks = selected;
        match next_terrain_cache_work(&world, &upload, &loaded) {
            TerrainCacheWork::RefreshSnapshot(pos) => {
                assert!((pos.x - near.x).abs() <= 1 && (pos.z - near.z).abs() <= 1);
            }
            work => panic!("expected near-chunk snapshot work, got {work:?}"),
        }
    }

    #[test]
    fn mesh_targets_withhold_a_chunk_until_its_initial_light_applies() {
        let mut world = WorldStore::new();
        let pos = world
            .insert_level_chunk_with_light(synthetic_chunk_packet())
            .unwrap();
        let loaded: HashSet<_> = world.chunk_positions().into_iter().collect();
        let mut upload = TerrainUploadState::default();
        upload.selected_chunks = vec![pos];

        // Vanilla only reports the chunk ready to render from its queued light
        // closure: before the poll the chunk contributes no mesh work of its own.
        assert!(world.terrain_chunk_initial_light_pending(pos));
        assert!(matches!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::Idle
        ));

        assert_eq!(world.poll_queued_light_updates(), 1);
        assert!(!world.terrain_chunk_initial_light_pending(pos));
        assert!(matches!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::RefreshSnapshot(snapshot) if snapshot == pos
        ));
    }
}
