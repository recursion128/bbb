use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use bbb_renderer::terrain::{
    build_terrain_chunk_mesh_layers_with_atlas_and_camera, TerrainCardinalLighting, TerrainCell,
    TerrainChunkSnapshot, TerrainFluid, TerrainFluidKind, TerrainLight, TerrainMaterialClass,
    TerrainMeshLayers, TerrainSkipRendering, TerrainTint,
};
use bbb_world::{
    block_name_is_half_transparent, block_name_is_leaves, ChunkPos, WorldCardinalLighting,
    WorldStore,
};

use crate::biome_tint::{
    block_wants_biome_blend, BiomeBlend, BIOME_BLEND_DIAMETER, BIOME_BLEND_RADIUS,
    BIOME_BLEND_SAMPLES,
};
use crate::camera_pose::camera_pose_from_world;

mod textures;
pub(crate) use textures::{
    load_terrain_textures, try_load_terrain_textures, BlockRenderPosition,
    TerrainParticleTintCatalog, TerrainTextureState,
};

const MAX_TERRAIN_UPLOAD_CHUNKS: usize = 49;
const TERRAIN_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug)]
struct CachedTerrainSnapshot {
    content_revision: u64,
    mesh_revision: u64,
    snapshot: Option<TerrainChunkSnapshot>,
}

#[derive(Debug)]
struct CachedTerrainMesh {
    revision: u64,
    layers: TerrainMeshLayers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainCacheWork {
    RefreshSnapshot(ChunkPos),
    RebuildMesh(ChunkPos),
    Idle,
}

#[derive(Debug, Default)]
pub(crate) struct TerrainUploadState {
    observed_world_revision: u64,
    selected_chunks: Vec<ChunkPos>,
    snapshots: HashMap<ChunkPos, CachedTerrainSnapshot>,
    meshes: HashMap<ChunkPos, CachedTerrainMesh>,
    reload_all_chunks_requested: bool,
    texture_animation_tick: u64,
    last_texture_animation_at: Option<Instant>,
    compiled_and_visible_chunks: HashSet<(i32, i32)>,
    reported_readiness: Option<(u64, bool)>,
}

impl TerrainUploadState {
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
        self.compiled_and_visible_chunks.contains(&(
            (pose.position.x.floor() as i32).div_euclid(16),
            (pose.position.z.floor() as i32).div_euclid(16),
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
) {
    if world.chunk_count() == 0 {
        if synchronize_empty_terrain_upload(upload, world.terrain_revision()) {
            renderer.upload_terrain_mesh_layers(TerrainMeshLayers::default());
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

    let mut renderer_dirty = false;
    if upload.reload_all_chunks_requested {
        renderer_dirty = !upload.meshes.is_empty();
        upload.snapshots.clear();
        upload.meshes.clear();
        upload.compiled_and_visible_chunks.clear();
        upload.reload_all_chunks_requested = false;
    }
    upload
        .snapshots
        .retain(|pos, _| required_snapshots.contains(pos));
    let resident_before = upload.meshes.len();
    upload.meshes.retain(|pos, _| selected.contains(pos));
    renderer_dirty |= upload.meshes.len() != resident_before;
    upload.selected_chunks = selected_chunks;

    match next_terrain_cache_work(world, upload, &loaded) {
        TerrainCacheWork::RefreshSnapshot(pos) => {
            refresh_terrain_snapshot(world, upload, textures, pos);
        }
        TerrainCacheWork::RebuildMesh(pos) => {
            rebuild_terrain_chunk_mesh(world, upload, textures, pos);
            renderer_dirty = true;
        }
        TerrainCacheWork::Idle => {}
    }

    if renderer_dirty {
        upload_cached_terrain(renderer, upload);
    }
    upload.observed_world_revision = world.terrain_revision();
}

fn synchronize_empty_terrain_upload(upload: &mut TerrainUploadState, world_revision: u64) -> bool {
    let clear_renderer = !upload.meshes.is_empty();
    upload.observed_world_revision = world_revision;
    upload.selected_chunks.clear();
    upload.snapshots.clear();
    upload.meshes.clear();
    upload.reload_all_chunks_requested = false;
    upload.compiled_and_visible_chunks.clear();
    clear_renderer
}

fn selected_terrain_chunks(world: &WorldStore, center: ChunkPos) -> Vec<ChunkPos> {
    let mut positions = world.chunk_positions();
    positions.sort_by_key(|pos| (chunk_distance_key(*pos, center), pos.x, pos.z));
    positions.truncate(MAX_TERRAIN_UPLOAD_CHUNKS);
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
            let content_revision = world.terrain_chunk_content_revision(pos);
            if !upload.snapshots.get(&pos).is_some_and(|cached| {
                cached.content_revision == content_revision
                    && (pos != target || cached.mesh_revision == mesh_revision)
            }) {
                return TerrainCacheWork::RefreshSnapshot(pos);
            }
        }
        return TerrainCacheWork::RebuildMesh(target);
    }
    TerrainCacheWork::Idle
}

fn refresh_terrain_snapshot(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    pos: ChunkPos,
) {
    let content_revision = world.terrain_chunk_content_revision(pos);
    let mesh_revision = world.terrain_chunk_mesh_revision(pos);
    let snapshot = world.extract_terrain_chunk(pos).map(|snapshot| {
        let biome_sampler = world.chunk_biome_sampler(pos);
        convert_terrain_snapshot(
            snapshot,
            textures,
            terrain_cardinal_lighting(world),
            &biome_sampler,
        )
    });
    upload.snapshots.insert(
        pos,
        CachedTerrainSnapshot {
            content_revision,
            mesh_revision,
            snapshot,
        },
    );
}

fn rebuild_terrain_chunk_mesh(
    world: &WorldStore,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
    pos: ChunkPos,
) {
    let revision = world.terrain_chunk_mesh_revision(pos);
    let layers = upload
        .snapshots
        .get(&pos)
        .and_then(|cached| cached.snapshot.as_ref())
        .map(|target| {
            let neighborhood = chunk_neighborhood(pos).filter_map(|neighbor| {
                upload
                    .snapshots
                    .get(&neighbor)
                    .and_then(|cached| cached.snapshot.as_ref())
            });
            build_terrain_chunk_mesh_layers_with_atlas_and_camera(
                target,
                neighborhood,
                &textures.atlas,
                terrain_camera_position(world),
            )
        })
        .unwrap_or_default();
    upload
        .meshes
        .insert(pos, CachedTerrainMesh { revision, layers });
}

fn upload_cached_terrain(renderer: &mut bbb_renderer::Renderer, upload: &mut TerrainUploadState) {
    let compiled_and_visible_chunks = upload
        .selected_chunks
        .iter()
        .filter(|pos| upload.meshes.contains_key(pos))
        .map(|pos| (pos.x, pos.z))
        .collect();
    let cached: Vec<_> = upload
        .selected_chunks
        .iter()
        .filter_map(|pos| upload.meshes.get(pos))
        .collect();
    let source_sections = cached
        .iter()
        .map(|cached| cached.layers.source_sections)
        .sum();
    renderer.upload_terrain_mesh_layer_refs(
        source_sections,
        cached.iter().flat_map(|cached| cached.layers.opaque.iter()),
        cached.iter().flat_map(|cached| cached.layers.cutout.iter()),
        cached
            .iter()
            .flat_map(|cached| cached.layers.translucent.iter()),
    );
    upload.compiled_and_visible_chunks = compiled_and_visible_chunks;
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

fn terrain_cardinal_lighting(world: &WorldStore) -> TerrainCardinalLighting {
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
    let cells = snapshot
        .cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
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
            let blend = wants_blend.then(|| build_biome_blend(biome_sampler, position));
            let (texture_indices, tint, face_transparency, render_shape, ambient_occlusion) =
                textures.block_render_data(
                    cell.block_name.as_deref(),
                    &cell.block_properties,
                    world_material,
                    cell.biome_id,
                    Some(position),
                    blend.as_ref(),
                );
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
                light: TerrainLight {
                    sky: cell.light.sky,
                    block: cell.light.block,
                },
                tint,
                face_transparency,
                skip_rendering: renderer_skip_rendering(cell.skip_rendering),
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
fn build_biome_blend(
    sampler: &bbb_world::ChunkBiomeSampler,
    center: BlockRenderPosition,
) -> BiomeBlend {
    let mut samples = [None; BIOME_BLEND_SAMPLES];
    for row in 0..BIOME_BLEND_DIAMETER {
        for col in 0..BIOME_BLEND_DIAMETER {
            let index = (row * BIOME_BLEND_DIAMETER + col) as usize;
            samples[index] = sampler.biome_id_at(
                center.x - BIOME_BLEND_RADIUS + col,
                center.y,
                center.z - BIOME_BLEND_RADIUS + row,
            );
        }
    }
    BiomeBlend::new(center, samples)
}

fn renderer_skip_rendering(
    skip_rendering: bbb_world::TerrainSkipRendering,
) -> TerrainSkipRendering {
    TerrainSkipRendering {
        same_block_key: skip_rendering.same_block_key,
        same_block_culls_all_faces: skip_rendering.same_block_culls_all_faces,
        same_block_culls_vertical_faces: skip_rendering.same_block_culls_vertical_faces,
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
    use bbb_protocol::packets::{SetChunkCacheCenter, Vec3d};
    use bbb_world::{ChunkColumn, ChunkState, LightData, LocalPlayerPoseState, WorldDimension};

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
            iron_bars_block: true,
            bars_tag: true,
            north: true,
            south: false,
            west: true,
            east: false,
        };

        assert_eq!(
            renderer_skip_rendering(skip),
            TerrainSkipRendering {
                same_block_key: 42,
                same_block_culls_all_faces: true,
                same_block_culls_vertical_faces: true,
                iron_bars_block: true,
                bars_tag: true,
                north: true,
                south: false,
                west: true,
                east: false,
            }
        );
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
                    layers: TerrainMeshLayers::default(),
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
        upload.snapshots.insert(
            center,
            CachedTerrainSnapshot {
                content_revision: world.terrain_chunk_content_revision(center),
                mesh_revision: world.terrain_chunk_mesh_revision(center),
                snapshot: None,
            },
        );
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::RefreshSnapshot(east)
        );
        upload.snapshots.insert(
            east,
            CachedTerrainSnapshot {
                content_revision: world.terrain_chunk_content_revision(east),
                mesh_revision: world.terrain_chunk_mesh_revision(east),
                snapshot: None,
            },
        );
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::RebuildMesh(center)
        );
        upload.meshes.insert(
            center,
            CachedTerrainMesh {
                revision: world.terrain_chunk_mesh_revision(center),
                layers: TerrainMeshLayers::default(),
            },
        );
        assert_eq!(
            next_terrain_cache_work(&world, &upload, &loaded),
            TerrainCacheWork::RebuildMesh(east)
        );
        upload.meshes.insert(
            east,
            CachedTerrainMesh {
                revision: world.terrain_chunk_mesh_revision(east),
                layers: TerrainMeshLayers::default(),
            },
        );
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
    fn player_section_ready_requires_an_uploaded_in_height_chunk() {
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
        upload.compiled_and_visible_chunks.insert((-1, 1));
        assert!(upload.is_player_section_compiled_and_visible(&world));

        world.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: -0.25,
                y: 400.0,
                z: 16.0,
            },
            ..LocalPlayerPoseState::default()
        });
        assert!(!upload.is_player_section_compiled_and_visible(&world));
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
}
