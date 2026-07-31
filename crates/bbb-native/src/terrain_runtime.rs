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
    TerrainSectionMeshOwner,
};
use bbb_world::{
    block_name_is_half_transparent, block_name_is_leaves, ChunkPos, WorldCardinalLighting,
    WorldStore,
};

use crate::biome_tint::{block_wants_biome_blend, build_biome_blend, BiomeBlend};
use crate::camera_pose::camera_pose_from_world;

mod mesh_worker;
mod occlusion;
mod textures;
use mesh_worker::{
    TerrainMeshJob, TerrainMeshJobKey, TerrainMeshResult, TerrainMeshSubmit, TerrainMeshWorker,
};
use occlusion::{
    ChunkSectionDebugGizmos, ResidentSection, SectionPosition, TerrainSectionOcclusionGraph,
};
pub(crate) use textures::{
    load_terrain_textures, terrain_particle_biome_blend_center, try_load_terrain_textures,
    BlockRenderPosition, TerrainParticleTintCatalog, TerrainTextureState,
};

const MAX_TERRAIN_CACHE_WORK_PER_FRAME: usize = 8;
const TERRAIN_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);

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
    reload_all_chunks_requested: bool,
    texture_animation_tick: u64,
    last_texture_animation_at: Option<Instant>,
    compiled_and_visible_sections: HashSet<(i32, i32, i32)>,
    occlusion_graph: TerrainSectionOcclusionGraph,
    renderer_visible_sections: Option<Vec<TerrainSectionMeshOwner>>,
    reported_readiness: Option<(u64, bool)>,
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
    pub(crate) fn refresh_frame_visibility(&mut self, context: TerrainVisibilityContext) {
        publish_terrain_frustum_visibility(self, context);
    }

    pub(crate) fn renderer_visible_sections(&self) -> Option<&[TerrainSectionMeshOwner]> {
        self.renderer_visible_sections.as_deref()
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
        if self.occlusion_graph.is_valid() {
            self.occlusion_graph
                .reachable_sections()
                .contains(&position)
        } else {
            self.compiled_and_visible_sections
                .contains(&(position.x, position.y, position.z))
        }
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
        self.compiled_and_visible_sections.contains(&(
            (position.x.floor() as i32).div_euclid(16),
            block_y.div_euclid(16),
            (position.z.floor() as i32).div_euclid(16),
        ))
    }

    pub(crate) fn is_block_section_compiled_and_visible(
        &self,
        world: &WorldStore,
        position: bbb_world::BlockPos,
    ) -> bool {
        if !world.dimension().contains_y(position.y) {
            return true;
        }
        self.compiled_and_visible_sections.contains(&(
            position.x.div_euclid(16),
            position.y.div_euclid(16),
            position.z.div_euclid(16),
        ))
    }

    /// Returns a command payload only when readiness changes or a new client
    /// level generation becomes active.
    pub(crate) fn take_readiness_edge(
        &mut self,
        level_generation: u64,
        ready: bool,
    ) -> Option<(u64, bool)> {
        let next = (level_generation, ready);
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
        assert_eq!(upload.take_readiness_edge(4, false), Some((4, false)));
        assert_eq!(upload.take_readiness_edge(4, false), None);
        assert_eq!(upload.take_readiness_edge(4, true), Some((4, true)));
        assert_eq!(upload.take_readiness_edge(4, true), None);
        assert_eq!(upload.take_readiness_edge(5, true), Some((5, true)));
    }
}

pub(crate) fn maybe_upload_terrain_texture_animation(
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
) {
    if !textures.has_texture_animation() {
        return;
    }
    let Some(tick) = advance_texture_animation_tick(upload, Instant::now()) else {
        return;
    };
    match textures.animation_atlas_frame(tick) {
        Ok(Some(atlas)) => {
            let mip_rgba = atlas.rgba_slices();
            if let Err(err) = renderer.update_terrain_texture_atlas_mips(&mip_rgba) {
                tracing::warn!(?err, "failed to update animated terrain texture atlas");
            }
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(
                ?err,
                "failed to stitch animated terrain texture atlas frame"
            );
        }
    }
}

fn advance_texture_animation_tick(upload: &mut TerrainUploadState, now: Instant) -> Option<u64> {
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
    upload.texture_animation_tick = upload.texture_animation_tick.saturating_add(ticks);
    let advanced = Duration::from_millis(
        ticks.saturating_mul(TERRAIN_TEXTURE_ANIMATION_INTERVAL.as_millis() as u64),
    );
    upload.last_texture_animation_at = last.checked_add(advanced).or(Some(now));
    Some(upload.texture_animation_tick)
}

pub(crate) fn maybe_upload_decoded_terrain(
    world: &WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    visibility: TerrainVisibilityContext,
) {
    let mut renderer_dirty = drain_terrain_mesh_results(world, upload);
    let level_changed =
        synchronize_terrain_level_generation(upload, world.client_level_generation());
    if world.chunk_count() == 0 {
        let terrain_cleared = synchronize_empty_terrain_upload(upload, world.terrain_revision());
        if renderer_dirty || level_changed || terrain_cleared {
            upload_cached_terrain(renderer, upload);
        }
        refresh_terrain_visibility(world, upload, visibility);
        publish_mesh_worker_counters(world, renderer, upload);
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
    if upload.reload_all_chunks_requested {
        renderer_dirty |= !upload.meshes.is_empty();
        cancel_queued_terrain_mesh_jobs(upload);
        upload.mesh_generation = upload.mesh_generation.saturating_add(1);
        upload.mesh_atlas = None;
        upload.snapshots.clear();
        upload.meshes.clear();
        upload.compiled_and_visible_sections.clear();
        upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
        upload.renderer_visible_sections = None;
        upload.reload_all_chunks_requested = false;
    }
    upload
        .snapshots
        .retain(|pos, _| required_snapshots.contains(pos));
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
        upload_cached_terrain(renderer, upload);
    }
    refresh_terrain_visibility(world, upload, visibility);
    publish_mesh_worker_counters(world, renderer, upload);
    upload.observed_world_revision = world.terrain_revision();
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
    upload.compiled_and_visible_sections.clear();
    upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
    upload.renderer_visible_sections = None;
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
    upload.compiled_and_visible_sections.clear();
    upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
    upload.renderer_visible_sections = None;
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
    let player_changed = world.local_block_predictions().iter().any(|prediction| {
        prediction.pos.x.div_euclid(16) == pos.x && prediction.pos.z.div_euclid(16) == pos.z
    });
    let job = TerrainMeshJob {
        key,
        priority: (
            u8::from(!player_changed),
            chunk_distance_key(pos, center),
            pos.x,
            pos.z,
        ),
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
    upload: &TerrainUploadState,
) {
    renderer.set_terrain_mesh_worker_counters(terrain_residency_counters(world, upload));
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

fn upload_cached_terrain(renderer: &mut bbb_renderer::Renderer, upload: &mut TerrainUploadState) {
    let cached: Vec<_> = upload
        .selected_chunks
        .iter()
        .filter_map(|pos| upload.meshes.get(pos).map(|cached| (*pos, cached)))
        .collect();
    let source_sections = cached
        .iter()
        .map(|(_, cached)| cached.layers.source_sections)
        .sum();
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
}

fn refresh_terrain_visibility(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    context: TerrainVisibilityContext,
) {
    let Some(camera_pose) = context.camera_pose else {
        upload.occlusion_graph = TerrainSectionOcclusionGraph::default();
        publish_terrain_frustum_visibility(upload, context);
        return;
    };
    let camera_position = [
        f64::from(camera_pose.position[0]),
        f64::from(camera_pose.position[1] + camera_pose.eye_height),
        f64::from(camera_pose.position[2]),
    ];
    let resident_sections = upload
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
        .collect::<Vec<_>>();
    let loaded_chunks = world.chunk_positions().into_iter().collect();
    upload.occlusion_graph.rebuild(
        resident_sections,
        &loaded_chunks,
        camera_position,
        i32::try_from(context.render_distance_chunks).unwrap_or(i32::MAX),
        world.dimension().min_section_y(),
        world.dimension().section_count(),
        context.smart_cull,
    );
    publish_terrain_frustum_visibility(upload, context);
}

fn publish_terrain_frustum_visibility(
    upload: &mut TerrainUploadState,
    context: TerrainVisibilityContext,
) {
    let Some(frustum) = context
        .frustum()
        .filter(|_| upload.occlusion_graph.is_valid())
    else {
        upload.compiled_and_visible_sections = selected_resident_sections(upload);
        upload.renderer_visible_sections = None;
        return;
    };
    let visible = upload.occlusion_graph.visible_sections(frustum);
    upload.compiled_and_visible_sections = visible
        .iter()
        .map(|position| (position.x, position.y, position.z))
        .collect();
    upload.renderer_visible_sections = Some(
        visible
            .into_iter()
            .map(|position| TerrainSectionMeshOwner {
                chunk: TerrainChunkMeshOwner {
                    level_generation: upload.observed_level_generation,
                    chunk_x: position.x,
                    chunk_z: position.z,
                },
                section_y: position.y,
            })
            .collect(),
    );
}

fn selected_resident_sections(upload: &TerrainUploadState) -> HashSet<(i32, i32, i32)> {
    upload
        .selected_chunks
        .iter()
        .filter_map(|pos| upload.meshes.get(pos).map(|cached| (*pos, cached)))
        .flat_map(|(pos, cached)| {
            cached
                .resident_revisions
                .iter()
                .map(move |&(section_y, _)| (pos.x, section_y, pos.z))
        })
        .collect()
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
                fluid,
                fluid_texture_indices,
                fluid_overlay_texture_index,
                fluid_overlay_neighbor: block_uses_fluid_water_overlay(cell.block_name.as_deref()),
                fluid_tint,
                texture_indices,
                render_shape,
                ambient_occlusion,
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
    }
}

fn renderer_fluid(fluid: bbb_world::TerrainFluidState) -> TerrainFluid {
    let kind = match fluid.kind {
        bbb_world::TerrainFluidKind::Water => TerrainFluidKind::Water,
        bbb_world::TerrainFluidKind::Lava => TerrainFluidKind::Lava,
    };
    TerrainFluid::new(kind, fluid.amount, fluid.falling)
}

fn block_uses_fluid_water_overlay(block_name: Option<&str>) -> bool {
    let Some(block_name) = block_name else {
        return false;
    };
    block_name_is_half_transparent(block_name) || block_name_is_leaves(block_name)
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
            }
        );
        assert!(!renderer_skip_rendering(skip, true).culls_with_leaves);
    }

    #[test]
    fn fluid_water_overlay_neighbors_follow_vanilla_half_transparent_and_leaves_blocks() {
        for block_name in [
            "minecraft:glass",
            "minecraft:tinted_glass",
            "minecraft:white_stained_glass",
            "minecraft:ice",
            "minecraft:frosted_ice",
            "minecraft:blue_ice",
            "minecraft:slime_block",
            "minecraft:honey_block",
            "minecraft:copper_grate",
            "minecraft:weathered_copper_grate",
            "minecraft:waxed_oxidized_copper_grate",
            "minecraft:oak_leaves",
            "minecraft:flowering_azalea_leaves",
        ] {
            assert!(
                block_uses_fluid_water_overlay(Some(block_name)),
                "{block_name} should use water overlay"
            );
        }

        for block_name in [
            None,
            Some("stone"),
            Some("minecraft:stone"),
            Some("minecraft:packed_ice"),
            Some("minecraft:glass_pane"),
            Some("minecraft:white_stained_glass_pane"),
        ] {
            assert!(
                !block_uses_fluid_water_overlay(block_name),
                "{block_name:?} should not use water overlay"
            );
        }
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
            Some(1)
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(149)),
            Some(2)
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(250)),
            Some(5)
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(299)),
            None
        );
    }

    #[test]
    fn terrain_reload_request_marks_all_cached_chunks_for_rebuild() {
        let mut upload = TerrainUploadState::default();
        assert!(!upload.reload_all_chunks_requested());
        upload.request_reload_all_chunks();
        assert!(upload.reload_all_chunks_requested());
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
            compiled_and_visible_sections: HashSet::from([(pos.x, 4, pos.z)]),
            ..Default::default()
        };

        assert!(!synchronize_terrain_level_generation(&mut upload, 4));
        assert!(synchronize_terrain_level_generation(&mut upload, 5));
        assert!(upload.selected_chunks.is_empty());
        assert!(upload.snapshots.is_empty());
        assert!(upload.meshes.is_empty());
        assert!(upload.compiled_and_visible_sections.is_empty());
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
            HashSet::from([(-1, -4, 2), (-1, 3, 2)])
        );
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
        upload.compiled_and_visible_sections.insert((-1, 4, 1));
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
